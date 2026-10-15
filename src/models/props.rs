pub type PROPID = u32;

// 7-Zip Item Property IDs (PropID.h)
pub const KPID_NO_PROPERTY: PROPID = 0;
pub const KPID_PATH: PROPID = 3;
pub const KPID_NAME: PROPID = 4;
pub const KPID_EXTENSION: PROPID = 5;
pub const KPID_IS_DIR: PROPID = 6;
pub const KPID_SIZE: PROPID = 7;
pub const KPID_PACK_SIZE: PROPID = 8;
pub const KPID_ATTRIB: PROPID = 9;
pub const KPID_CTIME: PROPID = 10;
pub const KPID_ATIME: PROPID = 11;
pub const KPID_MTIME: PROPID = 12;
pub const KPID_SOLID: PROPID = 13;
pub const KPID_ENCRYPTED: PROPID = 15;
pub const KPID_CRC: PROPID = 19;
pub const KPID_METHOD: PROPID = 22;
pub const KPID_POSITION: PROPID = 30;
pub const KPID_PREFIX: PROPID = 31;
pub const KPID_NUM_SUB_DIRS: PROPID = 32;
pub const KPID_NUM_SUB_FILES: PROPID = 33;
pub const KPID_OFFSET: PROPID = 37;
pub const KPID_PHY_SIZE: PROPID = 54;
pub const KPID_HEADERS_SIZE: PROPID = 55;
pub const KPID_READ_ONLY: PROPID = 61;

// 7-Zip Format Handler Property IDs (IArchive.h)
pub mod handler_prop {
    use super::PROPID;
    pub const K_NAME: PROPID = 0;
    pub const K_CLASS_ID: PROPID = 1;
    pub const K_EXTENSION: PROPID = 2;
    pub const K_ADD_EXTENSION: PROPID = 3;
    pub const K_UPDATE: PROPID = 4;
    pub const K_KEEP_NAME: PROPID = 5;
    pub const K_SIGNATURE: PROPID = 6;
    pub const K_MULTI_SIGNATURE: PROPID = 7;
    pub const K_SIGNATURE_OFFSET: PROPID = 8;
    pub const K_ALT_STREAMS: PROPID = 9;
    pub const K_NT_SECURE: PROPID = 10;
    pub const K_FLAGS: PROPID = 11;
    pub const K_TIME_FLAGS: PROPID = 12;
}

// 7-Zip Archive Info Flags (NArcInfoFlags)
pub mod arc_info_flags {
    pub const K_KEEP_NAME: u32 = 1 << 0;
    pub const K_ALT_STREAMS: u32 = 1 << 1;
    pub const K_NT_SECURE: u32 = 1 << 2;
    pub const K_FIND_SIGNATURE: u32 = 1 << 3;
    pub const K_MULTI_SIGNATURE: u32 = 1 << 4;
    pub const K_USE_GLOBAL_OFFSET: u32 = 1 << 5;
    pub const K_START_OPEN: u32 = 1 << 6;
    pub const K_PURE_START_OPEN: u32 = 1 << 7;
    pub const K_BACKWARD_OPEN: u32 = 1 << 8;
    pub const K_PRE_ARC: u32 = 1 << 9;
    pub const K_SYM_LINKS: u32 = 1 << 10;
    pub const K_HARD_LINKS: u32 = 1 << 11;
    pub const K_BY_EXT_ONLY_OPEN: u32 = 1 << 12;
    pub const K_HASH_HANDLER: u32 = 1 << 13;
}

// 7-Zip Extraction Ask Mode (NArchive::NExtract::NAskMode)
pub mod ask_mode {
    pub const K_EXTRACT: i32 = 0;
    pub const K_TEST: i32 = 1;
    pub const K_SKIP: i32 = 2;
    pub const K_READ_EXTERNAL: i32 = 3;
}

// 7-Zip Operation Result (NArchive::NExtract::NOperationResult)
pub mod operation_result {
    pub const K_OK: i32 = 0;
    pub const K_UNSUPPORTED_METHOD: i32 = 1;
    pub const K_DATA_ERROR: i32 = 2;
    pub const K_CRC_ERROR: i32 = 3;
    pub const K_UNAVAILABLE: i32 = 4;
    pub const K_UNEXPECTED_END: i32 = 5;
    pub const K_DATA_AFTER_END: i32 = 6;
    pub const K_IS_NOT_ARC: i32 = 7;
    pub const K_HEADERS_ERROR: i32 = 8;
    pub const K_WRONG_PASSWORD: i32 = 9;
}
