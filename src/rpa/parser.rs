use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use flate2::read::ZlibDecoder;
use serde_pickle::{DeOptions, HashableValue, Value};

use crate::models::archive::RpaArchive;
use crate::models::entry::FileEntry;
use crate::models::error::RpaError;
use crate::models::version::RpaVersion;

pub struct RpaParser;

impl RpaParser {
    /// Parse an RPA archive from any Read + Seek stream.
    pub fn parse<R: Read + Seek>(stream: &mut R) -> Result<RpaArchive, RpaError> {
        stream.seek(SeekFrom::Start(0))?;

        // 1. Read first line to determine RPA version and index location
        let (version, index_offset, key) = {
            let mut buf_reader = BufReader::new(&mut *stream);
            let mut line = String::new();
            buf_reader.read_line(&mut line)?;
            RpaVersion::parse_header(&line)?
        };

        // 2. Seek to the compressed index location
        stream.seek(SeekFrom::Start(index_offset))?;

        // 3. Decompress the zlib-compressed pickle data
        let mut decoder = ZlibDecoder::new(stream);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| RpaError::Decompress(format!("Failed to decompress index: {}", e)))?;

        // 4. Parse Python pickle index
        let pickle_value: Value = serde_pickle::from_slice(
            &decompressed,
            DeOptions::new().decode_strings(),
        )
        .map_err(|e| RpaError::Pickle(format!("Failed to deserialize index pickle: {}", e)))?;

        let dict = match pickle_value {
            Value::Dict(d) => d,
            other => {
                return Err(RpaError::CorruptedIndex(format!(
                    "Expected pickle dict root, found {:?}",
                    other
                )));
            }
        };

        // 5. Extract and deobfuscate file entries
        let mut entries = Vec::with_capacity(dict.len());

        for (k, v) in dict {
            let path = match k {
                HashableValue::String(s) => s,
                HashableValue::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                other => format!("{:?}", other),
            };

            // Value is a list of index tuples, usually 1 tuple per file
            let tuple_list = match v {
                Value::List(l) => l,
                Value::Tuple(t) => t,
                other => {
                    return Err(RpaError::CorruptedIndex(format!(
                        "Expected list/tuple for '{}', found {:?}",
                        path, other
                    )));
                }
            };

            if let Some(first_entry) = tuple_list.first() {
                let items = match first_entry {
                    Value::List(l) => l.as_slice(),
                    Value::Tuple(t) => t.as_slice(),
                    _ => continue,
                };

                if items.len() < 2 {
                    continue;
                }

                let raw_offset = Self::value_to_u64(&items[0])?;
                let raw_length = Self::value_to_u64(&items[1])?;

                let prefix = if items.len() >= 3 {
                    match &items[2] {
                        Value::Bytes(b) => b.clone(),
                        Value::String(s) => s.as_bytes().to_vec(),
                        Value::List(l) => l
                            .iter()
                            .filter_map(|x| match x {
                                Value::I64(n) => Some(*n as u8),
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };

                let (offset, length) = match version {
                    RpaVersion::Rpa3 | RpaVersion::Rpa3_2 => {
                        let k = key as u64;
                        (raw_offset ^ k, raw_length ^ k)
                    }
                    _ => (raw_offset, raw_length),
                };

                entries.push(FileEntry::new(path, offset, length, prefix));
            }
        }

        // Sort entries by path for deterministic order
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(RpaArchive::new(version, key, index_offset, entries))
    }

    fn value_to_u64(val: &Value) -> Result<u64, RpaError> {
        match val {
            Value::I64(n) => Ok(*n as u64),
            Value::Int(n) => {
                // If it fits in u64:
                use num_traits::ToPrimitive;
                n.to_u64()
                    .ok_or_else(|| RpaError::CorruptedIndex(format!("BigInt out of range: {}", n)))
            }
            other => Err(RpaError::CorruptedIndex(format!(
                "Expected integer offset/length, got {:?}",
                other
            ))),
        }
    }
}
