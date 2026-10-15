#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIndex {
    pub offset: u64,
    pub length: u64,
    pub prefix: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub index: FileIndex,
    pub is_dir: bool,
}

impl FileEntry {
    pub fn new(path: String, offset: u64, length: u64, prefix: Vec<u8>) -> Self {
        let normalized = path.replace('\\', "/").trim_start_matches('/').to_string();
        let name = normalized
            .rsplit('/')
            .next()
            .unwrap_or(&normalized)
            .to_string();
        Self {
            path: normalized,
            name,
            index: FileIndex {
                offset,
                length,
                prefix,
            },
            is_dir: false,
        }
    }

    pub fn offset(&self) -> u64 {
        self.index.offset
    }

    pub fn length(&self) -> u64 {
        self.index.length
    }

    pub fn prefix(&self) -> &[u8] {
        &self.index.prefix
    }
}
