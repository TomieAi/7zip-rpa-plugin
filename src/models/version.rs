use crate::models::error::RpaError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpaVersion {
    Rpa1,
    Rpa2,
    Rpa3,
    Rpa3_2,
}

impl RpaVersion {
    pub const RPA2_MAGIC: &'static str = "RPA-2.0 ";
    pub const RPA3_MAGIC: &'static str = "RPA-3.0 ";
    pub const RPA3_2_MAGIC: &'static str = "RPA-3.2 ";

    pub fn as_str(&self) -> &'static str {
        match self {
            RpaVersion::Rpa1 => "1.0",
            RpaVersion::Rpa2 => "2.0",
            RpaVersion::Rpa3 => "3.0",
            RpaVersion::Rpa3_2 => "3.2",
        }
    }

    /// Parse header line to extract RPA version, index offset, and XOR obfuscation key.
    pub fn parse_header(header_line: &str) -> Result<(RpaVersion, u64, u32), RpaError> {
        let trimmed = header_line.trim_end();
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(RpaError::InvalidHeader("Empty header line".into()));
        }

        let magic = tokens[0];
        if magic.starts_with("RPA-3.2") {
            if tokens.len() < 3 {
                return Err(RpaError::InvalidHeader(format!(
                    "RPA-3.2 requires at least 3 tokens, got {}",
                    tokens.len()
                )));
            }
            let offset = u64::from_str_radix(tokens[1], 16)
                .map_err(|e| RpaError::InvalidHeader(format!("Bad offset {}: {}", tokens[1], e)))?;
            let mut key = 0u32;
            for subkey in &tokens[3..] {
                let k = u32::from_str_radix(subkey, 16)
                    .map_err(|e| RpaError::InvalidHeader(format!("Bad subkey {}: {}", subkey, e)))?;
                key ^= k;
            }
            Ok((RpaVersion::Rpa3_2, offset, key))
        } else if magic.starts_with("RPA-3.0") {
            if tokens.len() < 3 {
                return Err(RpaError::InvalidHeader(format!(
                    "RPA-3.0 requires at least 3 tokens, got {}",
                    tokens.len()
                )));
            }
            let offset = u64::from_str_radix(tokens[1], 16)
                .map_err(|e| RpaError::InvalidHeader(format!("Bad offset {}: {}", tokens[1], e)))?;
            let mut key = 0u32;
            for subkey in &tokens[2..] {
                let k = u32::from_str_radix(subkey, 16)
                    .map_err(|e| RpaError::InvalidHeader(format!("Bad subkey {}: {}", subkey, e)))?;
                key ^= k;
            }
            Ok((RpaVersion::Rpa3, offset, key))
        } else if magic.starts_with("RPA-2.0") {
            if tokens.len() < 2 {
                return Err(RpaError::InvalidHeader(format!(
                    "RPA-2.0 requires at least 2 tokens, got {}",
                    tokens.len()
                )));
            }
            let offset = u64::from_str_radix(tokens[1], 16)
                .map_err(|e| RpaError::InvalidHeader(format!("Bad offset {}: {}", tokens[1], e)))?;
            Ok((RpaVersion::Rpa2, offset, 0))
        } else {
            Err(RpaError::UnsupportedVersion(format!("Unknown header magic: {}", magic)))
        }
    }
}
