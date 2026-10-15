use std::collections::BTreeMap;
use std::io::{Seek, SeekFrom, Write};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde_pickle::{HashableValue, Value};

use crate::models::error::RpaError;
use crate::models::version::RpaVersion;

/// Abstraction for an item to be written into an RPA archive.
pub trait RpaItemSource {
    fn path(&self) -> &str;
    fn size_hint(&self) -> Option<u64>;
    fn write_to<W: Write>(&mut self, writer: &mut W) -> std::io::Result<u64>;
}

/// Simple in-memory byte source for testing and small files.
pub struct BytesSource {
    pub path: String,
    pub data: Vec<u8>,
}

impl BytesSource {
    pub fn new(path: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            data,
        }
    }
}

impl RpaItemSource for BytesSource {
    fn path(&self) -> &str {
        &self.path
    }

    fn size_hint(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }

    fn write_to<W: Write>(&mut self, writer: &mut W) -> std::io::Result<u64> {
        writer.write_all(&self.data)?;
        Ok(self.data.len() as u64)
    }
}

pub struct RpaWriter;

impl RpaWriter {
    /// Writes items into an RPA archive using the specified version and key.
    pub fn write_archive<W, S, F>(
        writer: &mut W,
        items: &mut [S],
        version: RpaVersion,
        key: u32,
        mut on_progress: F,
    ) -> Result<(), RpaError>
    where
        W: Write + Seek,
        S: RpaItemSource,
        F: FnMut(u64),
    {
        // 1. Write placeholder header
        let placeholder = match version {
            RpaVersion::Rpa2 => b"RPA-2.0 0000000000000000\n".to_vec(),
            RpaVersion::Rpa3_2 => {
                format!("RPA-3.2 0000000000000000 00000000 {:08x}\n", key).into_bytes()
            }
            _ => format!("RPA-3.0 0000000000000000 {:08x}\n", key).into_bytes(),
        };
        let header_len = placeholder.len();
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&placeholder)?;

        let mut current_offset = header_len as u64;
        let mut recorded_entries = Vec::with_capacity(items.len());

        // 2. Stream all items
        for item in items.iter_mut() {
            let path = normalize_rpa_path(item.path());
            if path.is_empty() {
                continue;
            }

            let start_offset = current_offset;
            let bytes_written = item.write_to(writer)?;
            current_offset += bytes_written;

            on_progress(bytes_written);

            recorded_entries.push((path, start_offset, bytes_written, Vec::<u8>::new()));
        }

        // 3. Current position is the index offset
        let index_offset = current_offset;

        // 4. Construct Python pickle dictionary
        let mut dict: BTreeMap<HashableValue, Value> = BTreeMap::new();
        for (path, offset, length, prefix) in recorded_entries {
            let (raw_offset, raw_length) = match version {
                RpaVersion::Rpa3 | RpaVersion::Rpa3_2 => {
                    (offset ^ (key as u64), length ^ (key as u64))
                }
                _ => (offset, length),
            };

            let tuple_items = vec![
                Value::I64(raw_offset as i64),
                Value::I64(raw_length as i64),
                Value::Bytes(prefix),
            ];

            dict.insert(
                HashableValue::Bytes(path.into_bytes()),
                Value::List(vec![Value::Tuple(tuple_items)]),
            );
        }

        // 5. Serialize pickle
        let pickle_bytes = serde_pickle::value_to_vec(&Value::Dict(dict), serde_pickle::SerOptions::new())
            .map_err(|e| RpaError::Pickle(format!("Failed to serialize pickle index: {}", e)))?;

        // 6. Compress pickle with zlib
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&pickle_bytes)?;
        let compressed_index = encoder
            .finish()
            .map_err(|e| RpaError::Decompress(format!("Failed to compress index: {}", e)))?;

        // 7. Write index at the end of the archive
        writer.write_all(&compressed_index)?;

        // 8. Seek back to start and write final header with actual index offset
        writer.seek(SeekFrom::Start(0))?;
        let final_header = match version {
            RpaVersion::Rpa2 => format!("RPA-2.0 {:016x}\n", index_offset).into_bytes(),
            RpaVersion::Rpa3_2 => {
                format!("RPA-3.2 {:016x} 00000000 {:08x}\n", index_offset, key).into_bytes()
            }
            _ => format!("RPA-3.0 {:016x} {:08x}\n", index_offset, key).into_bytes(),
        };

        if final_header.len() != header_len {
            return Err(RpaError::InvalidHeader(format!(
                "Header length mismatch: expected {}, got {}",
                header_len,
                final_header.len()
            )));
        }

        writer.write_all(&final_header)?;

        // Seek back to end
        writer.seek(SeekFrom::End(0))?;
        writer.flush()?;

        Ok(())
    }
}

/// Normalizes path to forward slashes with no leading slash or `./`.
pub fn normalize_rpa_path(path: &str) -> String {
    let s = path.replace('\\', "/");
    let trimmed = s.trim_start_matches('/');
    let clean = trimmed.strip_prefix("./").unwrap_or(trimmed);
    clean.trim_start_matches('/').to_string()
}
