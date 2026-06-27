use std::net::IpAddr;
use std::str::FromStr;

use crate::domain::{ArpEntry, Mac};

pub const PROC_NET_ARP: &str = "/proc/net/arp";
const ZERO_MAC: &str = "00:00:00:00:00:00";

/// Parse the contents of `/proc/net/arp` into complete, non-zero entries.
pub fn parse_arp_table(contents: &str) -> Vec<ArpEntry> {
    contents
        .lines()
        .skip(1)
        .filter_map(parse_arp_line)
        .collect()
}

fn parse_arp_line(line: &str) -> Option<ArpEntry> {
    let mut fields = line.split_whitespace();
    let ip = fields.next()?;
    let _hw_type = fields.next()?;
    let flags = fields.next()?;
    let mac = fields.next()?;

    if flags == "0x0" || mac == ZERO_MAC {
        return None;
    }

    let ip = IpAddr::from_str(ip).ok()?;
    let mac = Mac::from_str(mac).ok()?;
    Some(ArpEntry { ip, mac })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "IP address       HW type     Flags       HW address            Mask     Device
192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:01     *        eth0
192.168.1.50     0x1         0x2         aa:bb:cc:dd:ee:50     *        eth0
192.168.1.99     0x1         0x0         00:00:00:00:00:00     *        eth0
10.0.0.7         0x1         0x2         00:00:00:00:00:00     *        eth0
";

    #[test]
    fn parses_complete_entries_only() {
        let entries = parse_arp_table(SAMPLE);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ip.to_string(), "192.168.1.1");
        assert_eq!(entries[0].mac.to_string(), "aa:bb:cc:dd:ee:01");
        assert_eq!(entries[1].ip.to_string(), "192.168.1.50");
    }

    #[test]
    fn skips_incomplete_and_zero_mac() {
        let entries = parse_arp_table(SAMPLE);
        assert!(!entries.iter().any(|e| e.ip.to_string() == "192.168.1.99"));
        assert!(!entries.iter().any(|e| e.ip.to_string() == "10.0.0.7"));
    }

    #[test]
    fn handles_empty_and_header_only() {
        assert!(parse_arp_table("").is_empty());
        assert!(parse_arp_table("IP address HW type Flags HW address Mask Device").is_empty());
    }
}
