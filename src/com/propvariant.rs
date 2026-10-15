#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_void;
use crate::com::guid::GUID;

pub type VARTYPE = u16;
pub type HRESULT = i32;

pub const S_OK: HRESULT = 0;
pub const S_FALSE: HRESULT = 1;
pub const E_FAIL: HRESULT = -2147467259; // 0x80004005
pub const E_INVALIDARG: HRESULT = -2147024809; // 0x80070057
pub const E_NOINTERFACE: HRESULT = -2147467262; // 0x80004002
pub const CLASS_E_CLASSNOTAVAILABLE: HRESULT = -2147221231; // 0x80040111

pub const VT_EMPTY: VARTYPE = 0;
pub const VT_BSTR: VARTYPE = 8;
pub const VT_BOOL: VARTYPE = 11;
pub const VT_UI4: VARTYPE = 19;
pub const VT_UI8: VARTYPE = 21;
pub const VT_FILETIME: VARTYPE = 64;

pub const VARIANT_TRUE: i16 = -1;
pub const VARIANT_FALSE: i16 = 0;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct FILETIME {
    pub dwLowDateTime: u32,
    pub dwHighDateTime: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union PropVariantData {
    pub boolVal: i16,
    pub uintVal: u32,
    pub uhVal: u64,
    pub bstrVal: *mut u16,
    pub filetime: FILETIME,
    pub ptr: *mut c_void,
}

#[repr(C)]
pub struct PROPVARIANT {
    pub vt: VARTYPE,
    pub wReserved1: u16,
    pub wReserved2: u16,
    pub wReserved3: u16,
    pub data: PropVariantData,
}

#[cfg(windows)]
#[link(name = "oleaut32")]
extern "system" {
    fn SysAllocStringLen(psz: *const u16, len: u32) -> *mut u16;
    fn SysAllocStringByteLen(psz: *const i8, len: u32) -> *mut u16;
    fn SysFreeString(bstr: *mut u16);
    fn SysStringLen(bstr: *mut u16) -> u32;
    fn VariantClear(pvarg: *mut PROPVARIANT) -> HRESULT;
}

#[cfg(not(windows))]
unsafe fn SysAllocStringLen(_psz: *const u16, _len: u32) -> *mut u16 {
    std::ptr::null_mut()
}
#[cfg(not(windows))]
unsafe fn SysAllocStringByteLen(_psz: *const i8, _len: u32) -> *mut u16 {
    std::ptr::null_mut()
}
#[cfg(not(windows))]
unsafe fn SysFreeString(_bstr: *mut u16) {}
#[cfg(not(windows))]
unsafe fn SysStringLen(_bstr: *mut u16) -> u32 {
    0
}
#[cfg(not(windows))]
unsafe fn VariantClear(_pvarg: *mut PROPVARIANT) -> HRESULT {
    S_OK
}

impl PROPVARIANT {
    pub fn new() -> Self {
        Self {
            vt: VT_EMPTY,
            wReserved1: 0,
            wReserved2: 0,
            wReserved3: 0,
            data: PropVariantData { uhVal: 0 },
        }
    }

    pub fn get_str(&self) -> Option<String> {
        if self.vt == VT_BSTR && !unsafe { self.data.bstrVal }.is_null() {
            let bstr = unsafe { self.data.bstrVal };
            let len = unsafe { SysStringLen(bstr) } as usize;
            let slice = unsafe { std::slice::from_raw_parts(bstr, len) };
            Some(String::from_utf16_lossy(slice))
        } else {
            None
        }
    }

    pub fn get_bool(&self) -> Option<bool> {
        if self.vt == VT_BOOL {
            Some(unsafe { self.data.boolVal != 0 })
        } else {
            None
        }
    }

    pub fn get_u64(&self) -> Option<u64> {
        if self.vt == VT_UI8 {
            Some(unsafe { self.data.uhVal })
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            VariantClear(self);
        }
        self.vt = VT_EMPTY;
    }

    pub fn set_empty(&mut self) -> HRESULT {
        self.clear();
        S_OK
    }

    pub fn set_bool(&mut self, val: bool) -> HRESULT {
        self.clear();
        self.vt = VT_BOOL;
        self.data.boolVal = if val { VARIANT_TRUE } else { VARIANT_FALSE };
        S_OK
    }

    pub fn set_u32(&mut self, val: u32) -> HRESULT {
        self.clear();
        self.vt = VT_UI4;
        self.data.uintVal = val;
        S_OK
    }

    pub fn set_u64(&mut self, val: u64) -> HRESULT {
        self.clear();
        self.vt = VT_UI8;
        self.data.uhVal = val;
        S_OK
    }

    pub fn set_str(&mut self, s: &str) -> HRESULT {
        self.clear();
        let wide: Vec<u16> = s.encode_utf16().collect();
        let bstr = unsafe { SysAllocStringLen(wide.as_ptr(), wide.len() as u32) };
        if bstr.is_null() && !s.is_empty() {
            return E_FAIL;
        }
        self.vt = VT_BSTR;
        self.data.bstrVal = bstr;
        S_OK
    }

    pub fn set_binary(&mut self, bytes: &[u8]) -> HRESULT {
        self.clear();
        let bstr = unsafe {
            SysAllocStringByteLen(bytes.as_ptr() as *const i8, bytes.len() as u32)
        };
        if bstr.is_null() && !bytes.is_empty() {
            return E_FAIL;
        }
        self.vt = VT_BSTR;
        self.data.bstrVal = bstr;
        S_OK
    }

    pub fn set_guid(&mut self, guid: &GUID) -> HRESULT {
        let ptr = guid as *const GUID as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<GUID>()) };
        self.set_binary(slice)
    }

    pub fn set_filetime(&mut self, filetime: FILETIME) -> HRESULT {
        self.clear();
        self.vt = VT_FILETIME;
        self.data.filetime = filetime;
        S_OK
    }
}

impl Default for PROPVARIANT {
    fn default() -> Self {
        Self::new()
    }
}

pub fn alloc_bstr(s: &str) -> *mut u16 {
    let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SysAllocStringLen(wide.as_ptr(), (wide.len() - 1) as u32) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn free_bstr(bstr: *mut u16) {
    if !bstr.is_null() {
        unsafe { SysFreeString(bstr) };
    }
}
