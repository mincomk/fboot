use std::sync::Arc;

use uuid::Uuid;

use crate::config::Config;
use crate::console::ConsoleHub;
use crate::domain::{IpmiCreds, Mac, Server};
use crate::error::{AppError, Result};
use crate::events::EventBus;
use crate::ports::{
    ArpTable, BlobStore, BootConfigRepo, BootDefaultsRepo, BootableRepo, IpmiController,
    NetworkScanner, ServerRepo, StatsRepo,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub servers: Arc<dyn ServerRepo>,
    pub bootables: Arc<dyn BootableRepo>,
    pub boot_config: Arc<dyn BootConfigRepo>,
    pub boot_defaults: Arc<dyn BootDefaultsRepo>,
    pub stats: Arc<dyn StatsRepo>,
    pub ipmi: Arc<dyn IpmiController>,
    pub blob: Arc<dyn BlobStore>,
    pub arp: Arc<dyn ArpTable>,
    pub scanner: Arc<dyn NetworkScanner>,
    pub events: EventBus,
    pub console: Arc<ConsoleHub>,
}

impl AppState {
    /// Resolve effective IPMI credentials for a server: per-server overrides on top of
    /// the configured defaults (admin/admin, cipher 3). The host falls back to the BMC
    /// IP resolved from the server's IPMI MAC via ARP, then its hostname.
    pub async fn ipmi_creds(&self, server: &Server) -> Result<IpmiCreds> {
        let override_creds = self.servers.get_ipmi_creds(server.id).await?;

        let host = match &override_creds {
            Some(c) if !c.host.is_empty() => c.host.clone(),
            _ => {
                self.arp
                    .ip_for_mac(&server.ipmi_mac)
                    .await?
                    .map(|ip| ip.to_string())
                    .or_else(|| server.hostname.clone())
                    .ok_or_else(|| {
                        AppError::BadRequest("no IPMI host known for server".to_string())
                    })?
            }
        };

        let (username, password, cipher) = match override_creds {
            Some(c) => (
                if c.username.is_empty() {
                    self.config.ipmi_default_user.clone()
                } else {
                    c.username
                },
                if c.password.is_empty() {
                    self.config.ipmi_default_pass.clone()
                } else {
                    c.password
                },
                if c.cipher == 0 {
                    self.config.ipmi_default_cipher
                } else {
                    c.cipher
                },
            ),
            None => (
                self.config.ipmi_default_user.clone(),
                self.config.ipmi_default_pass.clone(),
                self.config.ipmi_default_cipher,
            ),
        };

        Ok(IpmiCreds {
            host,
            username,
            password,
            cipher,
        })
    }

    /// Resolve the PXE bootable to serve to the client identified by `mac`.
    /// A registered server uses its own boot config (PXE must be enabled with a
    /// bootable assigned); an unregistered MAC falls back to the default PXE bootable.
    pub async fn effective_pxe_bootable(&self, mac: &Mac) -> Result<Option<Uuid>> {
        match self.servers.get_by_primary_mac(mac).await? {
            Some(server) => {
                let cfg = self.boot_config.get(server.id).await?;
                Ok(if cfg.boot_pxe { cfg.pxe_bootable_id } else { None })
            }
            None => Ok(self.boot_defaults.get().await?.pxe_bootable_id),
        }
    }

    /// Resolve the Linux bootable and per-server kernel command line parts for the client
    /// identified by `mac`, as `(bootable_id, cmdline_override, cmdline_append)`. A registered
    /// server uses its assigned Linux bootable plus its override/append; an unregistered MAC
    /// falls back to the default Linux bootable with no per-server params.
    pub async fn effective_linux_bootable(
        &self,
        mac: &Mac,
    ) -> Result<(Option<Uuid>, Option<String>, Option<String>)> {
        match self.servers.get_by_primary_mac(mac).await? {
            Some(server) => {
                let cfg = self.boot_config.get(server.id).await?;
                Ok((cfg.linux_bootable_id, cfg.cmdline_override, cfg.cmdline_append))
            }
            None => Ok((
                self.boot_defaults.get().await?.linux_bootable_id,
                None,
                None,
            )),
        }
    }

    /// The custom iPXE script override for the client identified by `mac`, if a
    /// registered server has one set. `None` falls back to the generated script.
    /// Unregistered MACs (served via defaults) never have an override.
    pub async fn effective_ipxe_script(&self, mac: &Mac) -> Result<Option<String>> {
        match self.servers.get_by_primary_mac(mac).await? {
            Some(server) => Ok(self.boot_config.get(server.id).await?.ipxe_script),
            None => Ok(None),
        }
    }

    /// The HTTP boot server origin (e.g. `http://10.0.0.1:8081`) used to build blob
    /// URLs embedded in generated iPXE scripts.
    pub fn http_boot_base_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.config.http_boot_host,
            self.config.http_boot_addr.port()
        )
    }
}
