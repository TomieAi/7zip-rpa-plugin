pub type PROPID = u32;

// 7-Zip Item Property IDs (PropID.h)
pub const KPID_NO_PROPERTY: PROPID = 0;
pub const KPID_MAIN_SUBFILE: PROPID = 1;
pub const KPID_HANDLER_ITEM_INDEX: PROPID = 2;
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
pub const KPID_COMMENTED: PROPID = 14;
pub const KPID_ENCRYPTED: PROPID = 15;
pub const KPID_SPLIT_BEFORE: PROPID = 16;
pub const KPID_SPLIT_AFTER: PROPID = 17;
pub const KPID_DICTIONARY_SIZE: PROPID = 18;
pub const KPID_CRC: PROPID = 19;
pub const KPID_TYPE: PROPID = 20;
pub const KPID_IS_ANTI: PROPID = 21;
pub const KPID_METHOD: PROPID = 22;
pub const KPID_HOST_OS: PROPID = 23;
pub const KPID_FILE_SYSTEM: PROPID = 24;
pub const KPID_USER: PROPID = 25;
pub const KPID_GROUP: PROPID = 26;
pub const KPID_BLOCK: PROPID = 27;
pub const KPID_COMMENT: PROPID = 28;
pub const KPID_POSITION: PROPID = 29;
pub const KPID_PREFIX: PROPID = 30;
pub const KPID_NUM_SUB_DIRS: PROPID = 31;
pub const KPID_NUM_SUB_FILES: PROPID = 32;
pub const KPID_UNPACK_VER: PROPID = 33;
pub const KPID_VOLUME: PROPID = 34;
pub const KPID_IS_VOLUME: PROPID = 35;
pub const KPID_OFFSET: PROPID = 36;
pub const KPID_LINKS: PROPID = 37;
pub const KPID_NUM_BLOCKS: PROPID = 38;
pub const KPID_NUM_VOLUMES: PROPID = 39;
pub const KPID_TIME_TYPE: PROPID = 40;
pub const KPID_BIT_64: PROPID = 41;
pub const KPID_BIG_ENDIAN: PROPID = 42;
pub const KPID_CPU: PROPID = 43;
pub const KPID_PHY_SIZE: PROPID = 44;
pub const KPID_HEADERS_SIZE: PROPID = 45;
pub const KPID_CHECKSUM: PROPID = 46;
pub const KPID_CHARACTS: PROPID = 47;
pub const KPID_VA: PROPID = 48;
pub const KPID_ID: PROPID = 49;
pub const KPID_SHORT_NAME: PROPID = 50;
pub const KPID_CREATOR_APP: PROPID = 51;
pub const KPID_SECTOR_SIZE: PROPID = 52;
pub const KPID_POSIX_ATTRIB: PROPID = 53;
pub const KPID_SYM_LINK: PROPID = 54;
pub const KPID_ERROR: PROPID = 55;
pub const KPID_TOTAL_SIZE: PROPID = 56;
pub const KPID_FREE_SPACE: PROPID = 57;
pub const KPID_CLUSTER_SIZE: PROPID = 58;
pub const KPID_VOLUME_NAME: PROPID = 59;
pub const KPID_LOCAL_NAME: PROPID = 60;
pub const KPID_PROVIDER: PROPID = 61;
pub const KPID_NT_SECURE: PROPID = 62;
pub const KPID_IS_ALT_STREAM: PROPID = 63;
pub const KPID_IS_AUX: PROPID = 64;
pub const KPID_IS_DELETED: PROPID = 65;
pub const KPID_IS_TREE: PROPID = 66;
pub const KPID_SHA1: PROPID = 67;
pub const KPID_SHA256: PROPID = 68;
pub const KPID_ERROR_TYPE: PROPID = 69;
pub const KPID_NUM_ERRORS: PROPID = 70;
pub const KPID_ERROR_FLAGS: PROPID = 71;
pub const KPID_WARNING_FLAGS: PROPID = 72;
pub const KPID_WARNING: PROPID = 73;
pub const KPID_NUM_STREAMS: PROPID = 74;
pub const KPID_NUM_ALT_STREAMS: PROPID = 75;
pub const KPID_ALT_STREAMS_SIZE: PROPID = 76;
pub const KPID_VIRTUAL_SIZE: PROPID = 77;
pub const KPID_UNPACK_SIZE: PROPID = 78;
pub const KPID_TOTAL_PHY_SIZE: PROPID = 79;
pub const KPID_VOLUME_INDEX: PROPID = 80;
pub const KPID_SUB_TYPE: PROPID = 81;
pub const KPID_SHORT_COMMENT: PROPID = 82;
pub const KPID_CODE_PAGE: PROPID = 83;
pub const KPID_IS_NOT_ARC_TYPE: PROPID = 84;
pub const KPID_PHY_SIZE_CANT_BE_DETECTED: PROPID = 85;
pub const KPID_ZEROS_TAIL_IS_ALLOWED: PROPID = 86;
pub const KPID_TAIL_SIZE: PROPID = 87;
pub const KPID_EMBEDDED_STUB_SIZE: PROPID = 88;
pub const KPID_NT_REPARSE: PROPID = 89;
pub const KPID_HARD_LINK: PROPID = 90;
pub const KPID_INODE: PROPID = 91;
pub const KPID_STREAM_ID: PROPID = 92;
pub const KPID_READ_ONLY: PROPID = 93;

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
