use std::net::{IpAddr, SocketAddr};
use std::path::Path;

use async_tftp::packet;
use async_tftp::server::TftpServerBuilder;
use futures::io::Cursor;

use crate::app_state::AppState;
use crate::domain::{Bootable, BootableRole, BootableSource};
use crate::error::Result;

pub async fn spawn(state: AppState) -> Result<()> {
    let addr = state.config.tftp_addr;
    let handler = BootHandler { state };

    let server = match TftpServerBuilder::with_handler(handler)
        .bind(addr)
        .build()
        .await
    {
        Ok(server) => server,
        Err(e) => {
            tracing::warn!(%addr, error = %e, "tftp: bind failed, service disabled");
            return Ok(());
        }
    };

    tracing::info!(%addr, "tftp: serving PXE boot files");
    tokio::spawn(async move {
        if let Err(e) = server.serve().await {
            tracing::error!(error = %e, "tftp: server stopped");
        }
    });
    Ok(())
}

struct BootHandler {
    state: AppState,
}

impl async_tftp::server::Handler for BootHandler {
    type Reader = Cursor<Vec<u8>>;
    type Writer = futures::io::Sink;

    async fn read_req_open(
        &mut self,
        client: &SocketAddr,
        path: &Path,
    ) -> std::result::Result<(Self::Reader, Option<u64>), packet::Error> {
        let requested = path.to_string_lossy().to_string();
        tracing::info!(%client, file = %requested, "tftp: read request received");
        // A running iPXE asks for a `.ipxe` script over TFTP (e.g. autoexec.ipxe). We don't
        // serve scripts over TFTP — return an empty file so iPXE proceeds (to its HTTP boot
        // URL) instead of being handed the boot program binary again, which would loop.
        if is_ipxe_script_request(path) {
            tracing::info!(%client, file = %requested, "tftp: serving empty ipxe script");
            return Ok((Cursor::new(Vec::new()), Some(0)));
        }
        match resolve_pxe_image(&self.state, client.ip()).await {
            Some(bytes) => {
                let len = bytes.len() as u64;
                tracing::info!(%client, file = %requested, bytes = len, "tftp: serving pxe image");
                Ok((Cursor::new(bytes), Some(len)))
            }
            None => {
                tracing::warn!(%client, file = %requested, "tftp: denying (see preceding reason)");
                Err(packet::Error::FileNotFound)
            }
        }
    }

    async fn write_req_open(
        &mut self,
        _client: &SocketAddr,
        _path: &Path,
        _size: Option<u64>,
    ) -> std::result::Result<Self::Writer, packet::Error> {
        Err(packet::Error::IllegalOperation)
    }
}

/// True if the requested file is an iPXE script (`.ipxe`), i.e. it comes from a
/// running iPXE rather than the firmware asking for the boot program.
fn is_ipxe_script_request(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ipxe"))
}

/// Resolve the PXE network boot program for the client at `client_ip`:
/// IP -> MAC (ARP) -> effective PXE bootable -> image bytes. The effective bootable is
/// the registered server's assignment, or the default PXE bootable for unregistered MACs.
/// The requested filename is intentionally ignored: the boot program is chosen by
/// the client's identity, matching the bootfile name advertised over ProxyDHCP.
async fn resolve_pxe_image(state: &AppState, client_ip: IpAddr) -> Option<Vec<u8>> {
    let mac = match state.arp.mac_for_ip(client_ip).await {
        Ok(Some(mac)) => mac,
        Ok(None) => {
            tracing::warn!(%client_ip, "tftp: no ARP entry for client IP, cannot map to MAC (client not yet in /proc/net/arp)");
            return None;
        }
        Err(e) => {
            tracing::warn!(%client_ip, error = %e, "tftp: ARP lookup failed");
            return None;
        }
    };

    let bootable_id = match state.effective_pxe_bootable(&mac).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!(%client_ip, %mac, "tftp: no PXE bootable assigned for this MAC (unregistered with no default, or PXE disabled)");
            return None;
        }
        Err(e) => {
            tracing::warn!(%client_ip, %mac, error = %e, "tftp: PXE bootable resolution failed");
            return None;
        }
    };

    let bootable = match state.bootables.get(bootable_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            tracing::warn!(%client_ip, %mac, %bootable_id, "tftp: assigned PXE bootable not found in DB");
            return None;
        }
        Err(e) => {
            tracing::warn!(%client_ip, %mac, %bootable_id, error = %e, "tftp: bootable lookup failed");
            return None;
        }
    };

    let Some(source) = pxe_image_source(&bootable) else {
        tracing::warn!(
            %client_ip, %mac, %bootable_id, bootable = %bootable.name,
            "tftp: PXE bootable has no `image` file (upload the network boot program, e.g. undionly.kpxe)"
        );
        return None;
    };

    match source {
        BootableSource::File { key } => match state.blob.get(key).await {
            Ok(bytes) => Some(bytes.to_vec()),
            Err(e) => {
                tracing::warn!(%client_ip, %mac, %bootable_id, key = %key, error = %e, "tftp: blob fetch failed for PXE image");
                None
            }
        },
        BootableSource::Url { url } => match fetch_url(url).await {
            Some(bytes) => Some(bytes),
            None => {
                tracing::warn!(%client_ip, %mac, %bootable_id, url = %url, "tftp: failed to fetch PXE image from url");
                None
            }
        },
    }
}

/// The image file backing a PXE bootable. The network boot program is stored
/// under the `image` role.
fn pxe_image_source(bootable: &Bootable) -> Option<&BootableSource> {
    bootable.file(BootableRole::Image).map(|f| &f.source)
}

async fn fetch_url(url: &str) -> Option<Vec<u8>> {
    let resp = reqwest::get(url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.bytes().await.ok().map(|b| b.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    use crate::domain::{BootableFile, BootableKind};

    fn bootable_with(files: Vec<BootableFile>) -> Bootable {
        Bootable {
            id: Uuid::new_v4(),
            kind: BootableKind::Pxe,
            name: "ipxe".into(),
            description: None,
            cmdline: None,
            files,
            metadata: BTreeMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn resolves_image_role_only() {
        let b = bootable_with(vec![
            BootableFile {
                role: BootableRole::Kernel,
                source: BootableSource::File { key: "k".into() },
                size: None,
            },
            BootableFile {
                role: BootableRole::Image,
                source: BootableSource::File {
                    key: "undionly".into(),
                },
                size: None,
            },
        ]);
        match pxe_image_source(&b) {
            Some(BootableSource::File { key }) => assert_eq!(key, "undionly"),
            other => panic!("expected image file source, got {other:?}"),
        }
    }

    #[test]
    fn detects_ipxe_script_request() {
        assert!(is_ipxe_script_request(Path::new("pxe/autoexec.ipxe")));
        assert!(is_ipxe_script_request(Path::new("aa:bb:cc:dd:ee:ff.ipxe")));
        assert!(!is_ipxe_script_request(Path::new("pxe/deadbeef0001.kpxe")));
        assert!(!is_ipxe_script_request(Path::new("undionly.kpxe")));
    }

    #[test]
    fn no_image_role_is_none() {
        let b = bootable_with(vec![BootableFile {
            role: BootableRole::Kernel,
            source: BootableSource::File { key: "k".into() },
            size: None,
        }]);
        assert!(pxe_image_source(&b).is_none());
    }
}
