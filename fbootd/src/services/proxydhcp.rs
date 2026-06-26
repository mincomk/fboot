use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use dhcproto::v4::{DhcpOption, HType, Message, MessageType, Opcode, OptionCode};
use dhcproto::{Decodable, Decoder, Encodable};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

use crate::app_state::AppState;
use crate::domain::Mac;
use crate::error::Result;

const PXE_VENDOR_CLASS: &[u8] = b"PXEClient";
const IPXE_USER_CLASS: &[u8] = b"iPXE";

pub async fn spawn(state: AppState) -> Result<()> {
    serve_on(state.clone(), state.config.dhcp_addr);
    let proxy_addr = state.config.dhcp_proxy_addr;
    if proxy_addr != state.config.dhcp_addr {
        serve_on(state, proxy_addr);
    }
    Ok(())
}

fn serve_on(state: AppState, addr: SocketAddr) {
    let socket = match bind_udp(addr) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(%addr, error = %e, "proxydhcp: bind failed, listener disabled");
            return;
        }
    };
    tracing::info!(%addr, "proxydhcp: listening for PXE clients");
    tokio::spawn(async move {
        if let Err(e) = run(state, socket).await {
            tracing::error!(error = %e, "proxydhcp: listener stopped");
        }
    });
}

fn bind_udp(addr: SocketAddr) -> std::io::Result<UdpSocket> {
    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let sock = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    sock.set_reuse_address(true)?;
    sock.set_broadcast(true)?;
    sock.set_nonblocking(true)?;
    sock.bind(&addr.into())?;
    UdpSocket::from_std(std::net::UdpSocket::from(sock))
}

async fn run(state: AppState, socket: UdpSocket) -> Result<()> {
    let server_ip = ipv4_of(state.config.tftp_host);
    let mut buf = vec![0u8; 2048];
    loop {
        let (len, peer) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "proxydhcp: recv error");
                continue;
            }
        };

        let msg = match Message::decode(&mut Decoder::new(&buf[..len])) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(%peer, bytes = len, error = %e, "proxydhcp: failed to decode DHCP packet");
                continue;
            }
        };

        if let Some((reply, target)) = handle(&state, &msg, server_ip, peer).await {
            match reply.to_vec() {
                Ok(bytes) => {
                    if let Err(e) = socket.send_to(&bytes, target).await {
                        tracing::warn!(%peer, error = %e, "proxydhcp: send failed");
                    }
                }
                Err(e) => tracing::warn!(error = %e, "proxydhcp: encode failed"),
            }
        }
    }
}

async fn handle(
    state: &AppState,
    msg: &Message,
    server_ip: Ipv4Addr,
    peer: SocketAddr,
) -> Option<(Message, SocketAddr)> {
    if msg.opcode() != Opcode::BootRequest {
        return None;
    }
    if !is_pxe_request(msg) {
        tracing::trace!("proxydhcp: ignoring non-PXE request (no PXEClient vendor class)");
        return None;
    }
    // PXE proxyDHCP follows the DORA handshake: a DISCOVER is answered with an
    // OFFER, a REQUEST with an ACK. A PXE client will not act on boot info from a
    // second OFFER — after it sends its REQUEST it waits for an ACK, and without
    // one it never downloads the boot program ("no media present").
    let msg_type = msg.opts().msg_type();
    let reply_type = match msg_type {
        Some(MessageType::Discover) => MessageType::Offer,
        Some(MessageType::Request) => MessageType::Ack,
        _ => return None,
    };

    let mac = match mac_from_chaddr(msg.chaddr()) {
        Some(m) => m,
        None => {
            tracing::debug!(?msg_type, "proxydhcp: PXE request has no usable chaddr, ignoring");
            return None;
        }
    };

    // An already-running iPXE re-DHCPs with user class `iPXE`; hand it the HTTP boot
    // script URL so it chainloads into our `/boot/{mac}.ipxe` endpoint. Firmware PXE
    // (no iPXE user class) instead gets the TFTP bootfile that delivers the iPXE binary.
    let bootfile = if is_ipxe_request(msg) {
        match state.effective_linux_bootable(&mac).await {
            Ok((Some(_), ..)) => {}
            Ok((None, ..)) => {
                tracing::debug!(%mac, ?msg_type, "proxydhcp: no offer, iPXE client but no linux bootable for this MAC");
                return None;
            }
            Err(e) => {
                tracing::warn!(%mac, error = %e, "proxydhcp: linux bootable resolution failed");
                return None;
            }
        }
        if state.config.http_boot_host.is_unspecified() {
            tracing::warn!(
                %mac,
                "proxydhcp: advertising iPXE script URL with host 0.0.0.0; set FBOOTD_ADVERTISE_IP so clients can reach the HTTP boot server"
            );
        }
        ipxe_boot_url(state, &mac)
    } else {
        match state.effective_pxe_bootable(&mac).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                tracing::debug!(%mac, ?msg_type, "proxydhcp: no offer, no PXE bootable for this MAC (unregistered with no default, or PXE disabled)");
                return None;
            }
            Err(e) => {
                tracing::warn!(%mac, error = %e, "proxydhcp: bootable resolution failed");
                return None;
            }
        }
        if server_ip.is_unspecified() {
            tracing::warn!(
                %mac,
                "proxydhcp: offering with next-server 0.0.0.0; set FBOOTD_ADVERTISE_IP so clients can reach TFTP"
            );
        }
        pxe_bootfile_name(&mac)
    };

    let target = reply_target(msg, peer);
    tracing::info!(%mac, %target, bootfile, ?reply_type, "proxydhcp: sending reply");
    let reply = build_offer(msg, server_ip, &bootfile, reply_type);
    Some((reply, target))
}

/// True if the request advertises the `PXEClient` vendor class (option 60).
fn is_pxe_request(msg: &Message) -> bool {
    match msg.opts().get(OptionCode::ClassIdentifier) {
        Some(DhcpOption::ClassIdentifier(data)) => {
            data.windows(PXE_VENDOR_CLASS.len()).any(|w| w == PXE_VENDOR_CLASS)
        }
        _ => false,
    }
}

/// True if the request advertises the `iPXE` user class (option 77), i.e. it comes
/// from an already-running iPXE rather than the firmware PXE ROM.
fn is_ipxe_request(msg: &Message) -> bool {
    match msg.opts().get(OptionCode::UserClass) {
        Some(DhcpOption::UserClass(data)) => {
            data.windows(IPXE_USER_CLASS.len()).any(|w| w == IPXE_USER_CLASS)
        }
        _ => false,
    }
}

/// HTTP URL of the iPXE boot script for `mac`, served by `http_boot::boot_script`
/// at `/boot/{mac}.ipxe`. The MAC uses colon-lowercase form, which that handler
/// parses back via `Mac::from_str`.
fn ipxe_boot_url(state: &AppState, mac: &Mac) -> String {
    format!("{}/boot/{}.ipxe", state.http_boot_base_url(), mac)
}

/// Bootfile name advertised to a PXE client. The TFTP server resolves the actual
/// boot program by client identity and ignores this path, so it is purely a
/// stable, human-readable label keyed by MAC.
pub(crate) fn pxe_bootfile_name(mac: &Mac) -> String {
    let o = mac.octets();
    format!(
        "pxe/{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}.kpxe",
        o[0], o[1], o[2], o[3], o[4], o[5]
    )
}

/// Construct a ProxyDHCP reply: BOOTREPLY pointing the client at our TFTP server
/// (next-server / siaddr) and the bootfile name, with the PXE vendor class and
/// server identifier options set. `msg_type` is OFFER in response to a DISCOVER
/// and ACK in response to a REQUEST.
fn build_offer(
    req: &Message,
    server_ip: Ipv4Addr,
    bootfile: &str,
    msg_type: MessageType,
) -> Message {
    let mut reply = Message::new(
        Ipv4Addr::UNSPECIFIED,
        Ipv4Addr::UNSPECIFIED,
        server_ip,
        req.giaddr(),
        req.chaddr(),
    );
    reply.set_opcode(Opcode::BootReply);
    reply.set_htype(HType::Eth);
    reply.set_xid(req.xid());
    reply.set_flags(req.flags());
    reply.set_siaddr(server_ip);
    reply.set_sname_str(server_ip.to_string());
    reply.set_fname_str(bootfile);

    let opts = reply.opts_mut();
    opts.insert(DhcpOption::MessageType(msg_type));
    opts.insert(DhcpOption::ServerIdentifier(server_ip));
    opts.insert(DhcpOption::ClassIdentifier(PXE_VENDOR_CLASS.to_vec()));
    // NUL-terminate the bootfile name: PXE ROMs read option 67 as a C string, and
    // some (AMI/ASRock) copy one byte past a non-terminated value — the 0xFF End
    // marker — into the TFTP filename. That makes the RRQ filename invalid UTF-8,
    // which strict TFTP servers silently drop. The trailing NUL stops the copy cleanly.
    let mut bootfile_opt = bootfile.as_bytes().to_vec();
    bootfile_opt.push(0);
    opts.insert(DhcpOption::BootfileName(bootfile_opt));
    reply
}

/// Where to send the reply. A relayed request (giaddr set) goes back through the
/// relay on port 67. Otherwise, if the client already has an address — the PXE Boot
/// Server Discovery REQUEST arrives unicast from the client's assigned IP (often on
/// port 4011) — we must answer it on that same address and port; a broadcast to :68
/// is never picked up by a client waiting on :4011. Only the initial DISCOVER, which
/// the client sends from 0.0.0.0 before it has an address, gets a broadcast.
fn reply_target(req: &Message, peer: SocketAddr) -> SocketAddr {
    let giaddr = req.giaddr();
    if !giaddr.is_unspecified() {
        return SocketAddr::V4(SocketAddrV4::new(giaddr, 67));
    }
    if !peer.ip().is_unspecified() {
        return peer;
    }
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, 68))
}

fn mac_from_chaddr(chaddr: &[u8]) -> Option<Mac> {
    if chaddr.len() < 6 {
        return None;
    }
    let mut bytes = [0u8; 6];
    bytes.copy_from_slice(&chaddr[..6]);
    Some(Mac::new(bytes))
}

fn ipv4_of(ip: std::net::IpAddr) -> Ipv4Addr {
    match ip {
        std::net::IpAddr::V4(v4) => v4,
        std::net::IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_packet(mac: [u8; 6], pxe: bool) -> Message {
        request_packet_with(mac, pxe, false)
    }

    fn request_packet_with(mac: [u8; 6], pxe: bool, ipxe: bool) -> Message {
        let mut msg = Message::new(
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::UNSPECIFIED,
            &mac,
        );
        msg.set_opcode(Opcode::BootRequest);
        msg.set_htype(HType::Eth);
        let opts = msg.opts_mut();
        opts.insert(DhcpOption::MessageType(MessageType::Discover));
        if pxe {
            opts.insert(DhcpOption::ClassIdentifier(b"PXEClient:Arch:00000".to_vec()));
        }
        if ipxe {
            opts.insert(DhcpOption::UserClass(b"iPXE".to_vec()));
        }
        msg
    }

    #[test]
    fn detects_pxe_vendor_class() {
        assert!(is_pxe_request(&request_packet([1, 2, 3, 4, 5, 6], true)));
        assert!(!is_pxe_request(&request_packet([1, 2, 3, 4, 5, 6], false)));
    }

    #[test]
    fn detects_ipxe_user_class() {
        assert!(is_ipxe_request(&request_packet_with([1, 2, 3, 4, 5, 6], true, true)));
        assert!(!is_ipxe_request(&request_packet_with([1, 2, 3, 4, 5, 6], true, false)));
    }

    #[test]
    fn ipxe_boot_url_round_trips_mac() {
        let mac = Mac::new([0xde, 0xad, 0xbe, 0xef, 0x00, 0x04]);
        let url = format!("http://10.0.0.1:8081/boot/{mac}.ipxe");
        assert_eq!(url, "http://10.0.0.1:8081/boot/de:ad:be:ef:00:04.ipxe");
        // http_boot::boot_script strips `.ipxe` then parses the MAC back.
        let path = url.rsplit('/').next().unwrap();
        let parsed: Mac = path.strip_suffix(".ipxe").unwrap().parse().unwrap();
        assert_eq!(parsed, mac);
    }

    #[test]
    fn offer_sets_next_server_and_bootfile() {
        let mac = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x01];
        let req = request_packet(mac, true);
        let server_ip = Ipv4Addr::new(10, 0, 0, 1);
        let bootfile = pxe_bootfile_name(&Mac::new(mac));

        let offer = build_offer(&req, server_ip, &bootfile, MessageType::Offer);

        assert_eq!(offer.opcode(), Opcode::BootReply);
        assert_eq!(offer.siaddr(), server_ip);
        assert_eq!(offer.xid(), req.xid());
        assert_eq!(offer.chaddr(), &mac);
        assert_eq!(offer.fname_str().unwrap().unwrap(), bootfile);
        assert_eq!(offer.opts().msg_type(), Some(MessageType::Offer));

        match offer.opts().get(OptionCode::BootfileName) {
            Some(DhcpOption::BootfileName(b)) => {
                // NUL-terminated so PXE ROMs don't read past the value into the 0xFF End marker.
                let mut expected = bootfile.as_bytes().to_vec();
                expected.push(0);
                assert_eq!(b, &expected);
            }
            other => panic!("missing bootfile option: {other:?}"),
        }
        match offer.opts().get(OptionCode::ServerIdentifier) {
            Some(DhcpOption::ServerIdentifier(ip)) => assert_eq!(*ip, server_ip),
            other => panic!("missing server identifier: {other:?}"),
        }
    }

    #[test]
    fn request_is_answered_with_ack_not_offer() {
        let mac = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x03];
        let req = request_packet(mac, true);
        let server_ip = Ipv4Addr::new(10, 0, 0, 1);
        let ack = build_offer(&req, server_ip, "pxe/deadbeef0003.kpxe", MessageType::Ack);
        assert_eq!(ack.opts().msg_type(), Some(MessageType::Ack));
    }

    #[test]
    fn offer_survives_roundtrip_encode() {
        let mac = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x02];
        let req = request_packet(mac, true);
        let server_ip = Ipv4Addr::new(192, 168, 1, 1);
        let offer = build_offer(&req, server_ip, "pxe/deadbeef0002.kpxe", MessageType::Offer);
        let bytes = offer.to_vec().unwrap();
        let decoded = Message::decode(&mut Decoder::new(&bytes)).unwrap();
        assert_eq!(decoded.siaddr(), server_ip);
        assert_eq!(decoded.opts().msg_type(), Some(MessageType::Offer));
    }
}
