use std::fmt;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl fmt::Debug for GUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        )
    }
}

pub const IID_IUNKNOWN: GUID = GUID {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

// 7-Zip Common Base GUID: 23170F69-40C1-278A-0000-00<groupId>00<subId>0000
const K_7ZIP_DATA1: u32 = 0x23170F69;
const K_7ZIP_DATA2: u16 = 0x40C1;
const K_7ZIP_DATA3: u16 = 0x278A;

pub const fn make_7zip_guid(group_id: u8, sub_id: u8) -> GUID {
    GUID {
        data1: K_7ZIP_DATA1,
        data2: K_7ZIP_DATA2,
        data3: K_7ZIP_DATA3,
        data4: [0x00, 0x00, 0x00, group_id, 0x00, sub_id, 0x00, 0x00],
    }
}

// 7-Zip interface GUIDs
pub const IID_ISEQUENTIALINSTREAM: GUID = make_7zip_guid(3, 0x01);
pub const IID_ISEQUENTIALOUTSTREAM: GUID = make_7zip_guid(3, 0x02);
pub const IID_IINSTREAM: GUID = make_7zip_guid(3, 0x03);
pub const IID_IARCHIVEOPENCALLBACK: GUID = make_7zip_guid(6, 0x10);
pub const IID_IARCHIVEEXTRACTCALLBACK: GUID = make_7zip_guid(6, 0x20);
pub const IID_IINARCHIVE: GUID = make_7zip_guid(6, 0x60);

// Unique GUID for Ren'Py RPA format handler:
// {9f1a7b42-5c8e-47e1-bf2a-3d8e4f12996a}
pub const RPA_HANDLER_GUID: GUID = GUID {
    data1: 0x9f1a7b42,
    data2: 0x5c8e,
    data3: 0x47e1,
    data4: [0xbf, 0x2a, 0x3d, 0x8e, 0x4f, 0x12, 0x99, 0x6a],
};
