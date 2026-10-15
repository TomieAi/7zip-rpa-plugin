pub mod archive;
pub mod com;
pub mod models;
pub mod rpa;

use std::ffi::c_void;

use crate::archive::RpaInArchive;
use crate::com::guid::{GUID, IID_IINARCHIVE, IID_IUNKNOWN, RPA_HANDLER_GUID};
use crate::com::propvariant::{
    CLASS_E_CLASSNOTAVAILABLE, E_FAIL, E_INVALIDARG, HRESULT, PROPVARIANT, S_OK,
};
use crate::models::props::{arc_info_flags, handler_prop, PROPID};

#[inline]
pub fn dbg_log(_s: &str) {
    #[cfg(debug_assertions)]
    {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:\\Users\\tomie\\AppData\\Local\\Temp\\rpa_debug.log")
        {
            let _ = writeln!(f, "[pid={}] {}", std::process::id(), _s);
        }
    }
}

/// Create an instance of the archive handler COM object.
///
/// Called by 7-Zip when opening an archive.
#[no_mangle]
pub unsafe extern "system" fn CreateObject(
    clsid: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT {
    dbg_log(&format!("CreateObject called: clsid={:?}, iid={:?}", clsid.as_ref(), iid.as_ref()));
    if clsid.is_null() || iid.is_null() || out_object.is_null() {
        return E_INVALIDARG;
    }
    *out_object = std::ptr::null_mut();

    if *clsid == RPA_HANDLER_GUID && (*iid == IID_IINARCHIVE || *iid == IID_IUNKNOWN) {
        let instance = RpaInArchive::new();
        *out_object = instance as *mut c_void;
        dbg_log("CreateObject -> S_OK");
        S_OK
    } else {
        dbg_log("CreateObject -> CLASS_E_CLASSNOTAVAILABLE");
        CLASS_E_CLASSNOTAVAILABLE
    }
}

/// Retrieve format property for format 0.
#[no_mangle]
pub unsafe extern "system" fn GetHandlerProperty(
    prop_id: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    GetHandlerProperty2(0, prop_id, value)
}

/// Retrieve format properties for format index.
#[no_mangle]
pub unsafe extern "system" fn GetHandlerProperty2(
    format_index: u32,
    prop_id: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    if value.is_null() {
        return E_INVALIDARG;
    }
    if format_index != 0 {
        return E_FAIL;
    }

    let val = &mut *value;

    match prop_id {
        handler_prop::K_NAME => val.set_str("RPA"),
        handler_prop::K_CLASS_ID => val.set_guid(&RPA_HANDLER_GUID),
        handler_prop::K_EXTENSION => val.set_str("rpa rpi"),
        handler_prop::K_ADD_EXTENSION => val.set_str(""),
        handler_prop::K_FLAGS => {
            val.set_u32(arc_info_flags::K_KEEP_NAME | arc_info_flags::K_FIND_SIGNATURE)
        }
        handler_prop::K_UPDATE => val.set_bool(false),
        handler_prop::K_SIGNATURE => val.set_binary(b"RPA-"),
        handler_prop::K_SIGNATURE_OFFSET => val.set_u32(0),
        handler_prop::K_TIME_FLAGS => S_OK,
        handler_prop::K_MULTI_SIGNATURE => S_OK,
        _ => S_OK,
    }
}

/// Returns the number of archive formats supported by this plugin (1: Ren'Py RPA).
#[no_mangle]
pub unsafe extern "system" fn GetNumberOfFormats(num_formats: *mut u32) -> HRESULT {
    if !num_formats.is_null() {
        *num_formats = 1;
        S_OK
    } else {
        E_INVALIDARG
    }
}
