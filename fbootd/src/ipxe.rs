use crate::domain::{Bootable, BootableRole, BootableSource};

/// Build an iPXE script that boots the given linux bootable (kernel + initrd) for a server.
/// `base_url` is the HTTP boot server origin (e.g. http://10.0.0.1:8081) serving the blobs.
///
/// The kernel command line is resolved as `base = override ?? bootable.cmdline`, with
/// `append` always layered on top when present.
pub fn linux_script(
    base_url: &str,
    bootable: &Bootable,
    cmdline_override: Option<&str>,
    cmdline_append: Option<&str>,
) -> String {
    let kernel = file_url(base_url, bootable, BootableRole::Kernel);
    let initrd = file_url(base_url, bootable, BootableRole::Initrd);
    let cmdline = effective_cmdline(bootable, cmdline_override, cmdline_append);

    let mut script = String::from("#!ipxe\n");
    if let Some(kernel) = kernel {
        let line = format!("kernel {kernel} initrd=initrd.img {cmdline}");
        script.push_str(line.trim_end());
        script.push('\n');
    }
    if let Some(initrd) = initrd {
        script.push_str(&format!("initrd {initrd}\n"));
    }
    script.push_str("boot\n");
    script
}

/// Resolve the kernel command line: a per-server override replaces the bootable's base
/// param, and a per-server append is always added on top. Returns a trimmed string
/// (possibly empty when nothing is configured).
fn effective_cmdline(
    bootable: &Bootable,
    cmdline_override: Option<&str>,
    cmdline_append: Option<&str>,
) -> String {
    let base = cmdline_override
        .or(bootable.cmdline.as_deref())
        .unwrap_or("");
    let combined = match cmdline_append {
        Some(a) if !a.is_empty() => format!("{base} {a}"),
        _ => base.to_string(),
    };
    combined.trim().to_string()
}

fn file_url(base_url: &str, bootable: &Bootable, role: BootableRole) -> Option<String> {
    let file = bootable.file(role)?;
    match &file.source {
        BootableSource::Url { url } => Some(url.clone()),
        BootableSource::File { .. } => Some(format!(
            "{base_url}/bootables/{}/{}",
            bootable.id,
            role.as_str()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BootableKind;
    use std::collections::BTreeMap;

    fn linux_bootable(cmdline: Option<&str>) -> Bootable {
        Bootable {
            id: uuid::Uuid::nil(),
            kind: BootableKind::Linux,
            name: "debian".into(),
            description: None,
            cmdline: cmdline.map(str::to_string),
            files: vec![],
            metadata: BTreeMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    fn cmdline(base: Option<&str>, override_: Option<&str>, append: Option<&str>) -> String {
        effective_cmdline(&linux_bootable(base), override_, append)
    }

    #[test]
    fn cmdline_resolution_matrix() {
        // Base only.
        assert_eq!(cmdline(Some("console=tty0 quiet"), None, None), "console=tty0 quiet");
        // Base + append.
        assert_eq!(
            cmdline(Some("console=tty0 quiet"), None, Some("ip=dhcp")),
            "console=tty0 quiet ip=dhcp"
        );
        // Override replaces base, append still added.
        assert_eq!(
            cmdline(Some("console=tty0 quiet"), Some("console=ttyS0"), Some("ip=dhcp")),
            "console=ttyS0 ip=dhcp"
        );
        // Override replaces base, no append.
        assert_eq!(
            cmdline(Some("console=tty0 quiet"), Some("console=ttyS0"), None),
            "console=ttyS0"
        );
        // Nothing anywhere -> empty.
        assert_eq!(cmdline(None, None, None), "");
        // Append only, no base.
        assert_eq!(cmdline(None, None, Some("ip=dhcp")), "ip=dhcp");
    }
}
