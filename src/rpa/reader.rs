use std::io::{Read, Seek, SeekFrom, Write};

use crate::models::entry::FileEntry;
use crate::models::error::RpaError;

pub struct RpaReader;

impl RpaReader {
    /// Read an entire file entry into memory.
    pub fn read_entry<R: Read + Seek>(
        stream: &mut R,
        entry: &FileEntry,
    ) -> Result<Vec<u8>, RpaError> {
        let mut out = Vec::with_capacity(entry.length() as usize);
        Self::stream_entry(stream, entry, &mut out)?;
        Ok(out)
    }

    /// Stream a file entry's contents to any Write destination in chunks.
    /// Handles any prepended prefix bytes transparently.
    pub fn stream_entry<R: Read + Seek, W: Write>(
        stream: &mut R,
        entry: &FileEntry,
        writer: &mut W,
    ) -> Result<u64, RpaError> {
        let mut total_written = 0u64;

        // 1. Write the prefix if present
        if !entry.prefix().is_empty() {
            writer.write_all(entry.prefix())?;
            total_written += entry.prefix().len() as u64;
        }

        // 2. Read the remaining payload from the archive stream
        let prefix_len = entry.prefix().len() as u64;
        if entry.length() > prefix_len {
            let to_read = entry.length() - prefix_len;
            stream.seek(SeekFrom::Start(entry.offset()))?;

            let mut remaining = to_read;
            let mut buf = [0u8; 65536];

            while remaining > 0 {
                let chunk_size = std::cmp::min(remaining, buf.len() as u64) as usize;
                let bytes_read = stream.read(&mut buf[..chunk_size])?;
                if bytes_read == 0 {
                    return Err(RpaError::StreamError(format!(
                        "Unexpected EOF reading file '{}': expected {} more bytes",
                        entry.path, remaining
                    )));
                }
                writer.write_all(&buf[..bytes_read])?;
                total_written += bytes_read as u64;
                remaining -= bytes_read as u64;
            }
        }

        Ok(total_written)
    }
}
