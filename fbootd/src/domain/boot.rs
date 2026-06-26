use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// Deserialize a nullable, optional field into `Option<Option<T>>` distinguishing
/// "field absent" (`None`, no change) from "field present and null" (`Some(None)`,
/// set to null). A plain `Option<Option<T>>` collapses JSON `null` to the outer
/// `None`, so an explicit null would be read as "no change" and never clear the value.
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    pub server_id: Uuid,
    pub boot_pxe: bool,
    pub pxe_bootable_id: Option<Uuid>,
    pub linux_bootable_id: Option<Uuid>,
    /// Replaces the linux bootable's base kernel command line for this server.
    pub cmdline_override: Option<String>,
    /// Appended to the effective kernel command line (after any override/base) for this server.
    pub cmdline_append: Option<String>,
    /// Custom iPXE script served verbatim instead of the generated one. `None` =
    /// use the script generated from the assigned linux bootable.
    pub ipxe_script: Option<String>,
}

impl BootConfig {
    pub fn default_for(server_id: Uuid) -> Self {
        BootConfig {
            server_id,
            boot_pxe: false,
            pxe_bootable_id: None,
            linux_bootable_id: None,
            cmdline_override: None,
            cmdline_append: None,
            ipxe_script: None,
        }
    }
}

/// Fallback bootables served to PXE clients whose MAC is not registered as a server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootDefaults {
    pub pxe_bootable_id: Option<Uuid>,
    pub linux_bootable_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateBootDefaults {
    #[serde(default, deserialize_with = "double_option")]
    pub pxe_bootable_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "double_option")]
    pub linux_bootable_id: Option<Option<Uuid>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateBootConfig {
    #[serde(default)]
    pub boot_pxe: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub pxe_bootable_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "double_option")]
    pub linux_bootable_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "double_option")]
    pub cmdline_override: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub cmdline_append: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub ipxe_script: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_clears_absent_leaves_unchanged() {
        // Absent field -> None (no change).
        let absent: UpdateBootConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.ipxe_script, None);

        // Present null -> Some(None) (clear to NULL).
        let cleared: UpdateBootConfig =
            serde_json::from_str(r#"{"ipxe_script": null}"#).unwrap();
        assert_eq!(cleared.ipxe_script, Some(None));

        // Present value -> Some(Some(..)) (set).
        let set: UpdateBootConfig =
            serde_json::from_str(r##"{"ipxe_script": "#!ipxe\nboot\n"}"##).unwrap();
        assert_eq!(set.ipxe_script, Some(Some("#!ipxe\nboot\n".to_string())));
    }
}
