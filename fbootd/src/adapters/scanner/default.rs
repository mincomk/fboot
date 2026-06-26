use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use futures::{SinkExt, StreamExt};
use hickory_resolver::proto::rr::RData;
use hickory_resolver::TokioResolver;
use tokio::net::TcpStream;

use crate::domain::{ScanEvent, ScanOptions, ScanProgress, ScanResult};
use crate::error::{AppError, Result};
use crate::ports::{ArpTable, NetworkScanner};

const HOST_CONCURRENCY: usize = 64;
const PORT_CONCURRENCY: usize = 16;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(800);
const MAX_HOSTS: u64 = 65_536;
const IPMI_PORT: u16 = 623;
const SSH_PORT: u16 = 22;

pub struct DefaultScanner {
    arp: Arc<dyn ArpTable>,
}

impl DefaultScanner {
    pub fn new(arp: Arc<dyn ArpTable>) -> Self {
        DefaultScanner { arp }
    }
}

#[async_trait]
impl NetworkScanner for DefaultScanner {
    async fn scan(&self, opts: ScanOptions) -> Result<BoxStream<'static, ScanEvent>> {
        let hosts = enumerate_hosts(&opts.cidr)?;
        let ports = build_port_list(&opts);
        let arp = self.arp.clone();

        let (mut tx, rx) = futures::channel::mpsc::channel::<ScanEvent>(64);

        tokio::spawn(async move {
            let resolver = TokioResolver::builder_tokio()
                .ok()
                .and_then(|b| b.build().ok())
                .map(Arc::new);

            let total = hosts.len();
            let probes = hosts.into_iter().map(|ip| {
                let ports = ports.clone();
                async move { probe_host(ip, ports).await }
            });
            let mut buffered = stream::iter(probes).buffer_unordered(HOST_CONCURRENCY);

            let mut scanned = 0usize;
            while let Some(probe) = buffered.next().await {
                scanned += 1;
                if !probe.open_ports.is_empty() {
                    let mac = arp.mac_for_ip(probe.ip).await.ok().flatten();
                    let hostname = resolve_hostname(resolver.as_deref(), probe.ip).await;
                    let result = ScanResult {
                        ip: probe.ip,
                        mac,
                        hostname,
                        board_info: None,
                        ipmi: opts.probe_ipmi && probe.open_ports.contains(&IPMI_PORT),
                        ssh: opts.probe_ssh && probe.open_ports.contains(&SSH_PORT),
                        open_ports: probe.open_ports,
                    };
                    if tx.send(ScanEvent::Result(result)).await.is_err() {
                        return;
                    }
                }
                if tx
                    .send(ScanEvent::Progress(ScanProgress { scanned, total }))
                    .await
                    .is_err()
                {
                    return;
                }
            }

            let _ = tx.send(ScanEvent::Done).await;
        });

        Ok(Box::pin(rx))
    }
}

struct HostProbe {
    ip: IpAddr,
    open_ports: Vec<u16>,
}

async fn probe_host(ip: IpAddr, ports: Vec<u16>) -> HostProbe {
    let checks = ports
        .into_iter()
        .map(|port| async move { (port, tcp_open(ip, port).await) });
    let mut open: Vec<u16> = stream::iter(checks)
        .buffer_unordered(PORT_CONCURRENCY)
        .filter_map(|(port, is_open)| async move { is_open.then_some(port) })
        .collect()
        .await;
    open.sort_unstable();
    HostProbe { ip, open_ports: open }
}

async fn tcp_open(ip: IpAddr, port: u16) -> bool {
    matches!(
        tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((ip, port))).await,
        Ok(Ok(_))
    )
}

async fn resolve_hostname(resolver: Option<&TokioResolver>, ip: IpAddr) -> Option<String> {
    let resolver = resolver?;
    let lookup = resolver.reverse_lookup(ip).await.ok()?;
    for record in lookup.answers() {
        if let RData::PTR(ptr) = &record.data {
            return Some(ptr.to_string().trim_end_matches('.').to_string());
        }
    }
    None
}

fn build_port_list(opts: &ScanOptions) -> Vec<u16> {
    let mut ports = Vec::new();
    if opts.probe_ipmi {
        ports.push(IPMI_PORT);
    }
    if opts.probe_ssh {
        ports.push(SSH_PORT);
    }
    ports.extend(opts.custom_ports.iter().copied());
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn enumerate_hosts(cidr: &str) -> Result<Vec<IpAddr>> {
    let bad = || AppError::BadRequest(format!("invalid CIDR: {cidr}"));

    let (addr_str, prefix) = match cidr.split_once('/') {
        Some((a, p)) => (a.trim(), p.trim().parse::<u8>().map_err(|_| bad())?),
        None => (cidr.trim(), 32u8),
    };

    let addr = Ipv4Addr::from_str(addr_str).map_err(|_| bad())?;
    if prefix > 32 {
        return Err(bad());
    }

    let base = u32::from(addr);
    let mask: u32 = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let network = base & mask;
    let broadcast = network | !mask;

    let total = u64::from(broadcast) - u64::from(network) + 1;
    if total > MAX_HOSTS {
        return Err(AppError::BadRequest(format!(
            "CIDR range too large ({total} addresses)"
        )));
    }

    let hosts: Vec<IpAddr> = if prefix >= 31 {
        (network..=broadcast)
            .map(|n| IpAddr::V4(Ipv4Addr::from(n)))
            .collect()
    } else {
        ((network + 1)..broadcast)
            .map(|n| IpAddr::V4(Ipv4Addr::from(n)))
            .collect()
    };

    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::arp::null::NullArpTable;

    #[test]
    fn enumerates_slash24_excludes_network_and_broadcast() {
        let hosts = enumerate_hosts("192.168.1.0/24").unwrap();
        assert_eq!(hosts.len(), 254);
        assert_eq!(hosts[0].to_string(), "192.168.1.1");
        assert_eq!(hosts[253].to_string(), "192.168.1.254");
    }

    #[test]
    fn enumerates_slash30() {
        let hosts = enumerate_hosts("10.0.0.0/30").unwrap();
        let got: Vec<String> = hosts.iter().map(|h| h.to_string()).collect();
        assert_eq!(got, vec!["10.0.0.1", "10.0.0.2"]);
    }

    #[test]
    fn slash31_and_slash32() {
        let p31 = enumerate_hosts("10.0.0.0/31").unwrap();
        assert_eq!(p31.len(), 2);
        let p32 = enumerate_hosts("10.0.0.5/32").unwrap();
        assert_eq!(p32.len(), 1);
        assert_eq!(p32[0].to_string(), "10.0.0.5");
        let bare = enumerate_hosts("10.0.0.9").unwrap();
        assert_eq!(bare, vec![IpAddr::from_str("10.0.0.9").unwrap()]);
    }

    #[test]
    fn rejects_garbage_and_oversize() {
        assert!(enumerate_hosts("not-an-ip").is_err());
        assert!(enumerate_hosts("10.0.0.0/99").is_err());
        assert!(enumerate_hosts("10.0.0.0/8").is_err());
    }

    #[test]
    fn builds_port_list() {
        let opts = ScanOptions {
            cidr: "10.0.0.0/30".into(),
            probe_ipmi: true,
            probe_ssh: true,
            custom_ports: vec![80, 22, 443],
        };
        assert_eq!(build_port_list(&opts), vec![22, 80, 443, 623]);
    }

    #[test]
    fn constructs() {
        let _ = DefaultScanner::new(Arc::new(NullArpTable));
    }
}
