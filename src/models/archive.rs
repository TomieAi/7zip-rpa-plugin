use crate::models::entry::FileEntry;
use crate::models::version::RpaVersion;

#[derive(Debug, Clone)]
pub struct RpaArchive {
    pub version: RpaVersion,
    pub key: u32,
    pub index_offset: u64,
    pub entries: Vec<FileEntry>,
}

impl RpaArchive {
    pub fn new(version: RpaVersion, key: u32, index_offset: u64, entries: Vec<FileEntry>) -> Self {
        Self {
            version,
            key,
            index_offset,
            entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&FileEntry> {
        self.entries.get(index)
    }

    pub fn find_by_path(&self, path: &str) -> Option<&FileEntry> {
        let norm = path.replace('\\', "/").trim_start_matches('/').to_string();
        self.entries.iter().find(|e| e.path == norm)
    }
}
