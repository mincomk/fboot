use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mac([u8; 6]);

#[derive(Debug, thiserror::Error)]
#[error("invalid MAC address: {0}")]
pub struct MacParseError(String);

impl Mac {
    pub fn new(bytes: [u8; 6]) -> Self {
        Mac(bytes)
    }

    pub fn octets(&self) -> [u8; 6] {
        self.0
    }
}

impl FromStr for Mac {
    type Err = MacParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleaned: String = s
            .chars()
            .filter(|c| !matches!(c, ':' | '-' | '.' | ' '))
            .collect();
        if cleaned.len() != 12 {
            return Err(MacParseError(s.to_string()));
        }
        let mut bytes = [0u8; 6];
        for (i, byte) in bytes.iter_mut().enumerate() {
            let hi = cleaned.as_bytes()[i * 2] as char;
            let lo = cleaned.as_bytes()[i * 2 + 1] as char;
            let h = hi.to_digit(16).ok_or_else(|| MacParseError(s.to_string()))?;
            let l = lo.to_digit(16).ok_or_else(|| MacParseError(s.to_string()))?;
            *byte = (h * 16 + l) as u8;
        }
        Ok(Mac(bytes))
    }
}

impl fmt::Display for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    }
}

impl Serialize for Mac {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Mac {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes() {
        let forms = [
            "AA:BB:CC:DD:EE:FF",
            "aa-bb-cc-dd-ee-ff",
            "aabb.ccdd.eeff",
            "aabbccddeeff",
        ];
        for f in forms {
            assert_eq!(f.parse::<Mac>().unwrap().to_string(), "aa:bb:cc:dd:ee:ff");
        }
    }

    #[test]
    fn rejects_bad() {
        assert!("zz:zz".parse::<Mac>().is_err());
        assert!("aabbccddee".parse::<Mac>().is_err());
    }
}
