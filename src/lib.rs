pub mod archive;
pub mod com;
pub mod models;
pub mod rpa;

use std::ffi::c_void;

use crate::archive::RpaInArchive;
use crate::com::guid::{GUID, IID_IINARCHIVE, IID_IOUTARCHIVE, IID_IUNKNOWN, RPA_HANDLER_GUID};
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
/// Called by 7-Zip when opening or creating an archive.
///
/// # Safety
/// The caller must ensure that `clsid`, `iid`, and `out_object` are valid pointers.
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

    let default_ver = if *clsid == RPA_HANDLER_GUID {
        Some(crate::models::version::RpaVersion::Rpa3)
    } else if *clsid == crate::com::guid::RPA2_HANDLER_GUID {
        Some(crate::models::version::RpaVersion::Rpa2)
    } else if *clsid == crate::com::guid::RPA3_2_HANDLER_GUID {
        Some(crate::models::version::RpaVersion::Rpa3_2)
    } else {
        None
    };

    if let Some(v) = default_ver {
        if *iid == IID_IINARCHIVE || *iid == IID_IUNKNOWN {
            let instance = RpaInArchive::new(Some(v));
            *out_object = instance as *mut c_void;
            dbg_log("CreateObject -> S_OK (IInArchive)");
            return S_OK;
        } else if *iid == IID_IOUTARCHIVE {
            let instance = RpaInArchive::new(Some(v));
            *out_object = (instance as *mut u8).add(std::mem::size_of::<usize>()) as *mut c_void;
            dbg_log("CreateObject -> S_OK (IOutArchive)");
            return S_OK;
        } else if *iid == crate::com::guid::IID_ISETPROPERTIES {
            let instance = RpaInArchive::new(Some(v));
            *out_object = (instance as *mut u8).add(2 * std::mem::size_of::<usize>()) as *mut c_void;
            dbg_log("CreateObject -> S_OK (ISetProperties)");
            return S_OK;
        }
    }

    dbg_log("CreateObject -> CLASS_E_CLASSNOTAVAILABLE");
    CLASS_E_CLASSNOTAVAILABLE
}

/// Retrieve format property for format 0.
///
/// # Safety
/// The caller must ensure `value` is a valid pointer to a PROPVARIANT.
#[no_mangle]
pub unsafe extern "system" fn GetHandlerProperty(
    prop_id: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    GetHandlerProperty2(0, prop_id, value)
}

/// Retrieve format properties for format index.
///
/// # Safety
/// The caller must ensure `value` is a valid pointer to a PROPVARIANT.
#[no_mangle]
pub unsafe extern "system" fn GetHandlerProperty2(
    format_index: u32,
    prop_id: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    if value.is_null() {
        return E_INVALIDARG;
    }

    let val = &mut *value;

    match format_index {
        0 => match prop_id {
            handler_prop::K_NAME => val.set_str("RPA-3.0"),
            handler_prop::K_CLASS_ID => val.set_guid(&RPA_HANDLER_GUID),
            handler_prop::K_EXTENSION => val.set_str("rpa rpi"),
            handler_prop::K_ADD_EXTENSION => val.set_str(""),
            handler_prop::K_FLAGS => {
                val.set_u32(arc_info_flags::K_KEEP_NAME | arc_info_flags::K_FIND_SIGNATURE)
            }
            handler_prop::K_UPDATE => val.set_bool(true),
            handler_prop::K_SIGNATURE => val.set_binary(b"RPA-3.0 "),
            handler_prop::K_SIGNATURE_OFFSET => val.set_u32(0),
            _ => S_OK,
        },
        1 => match prop_id {
            handler_prop::K_NAME => val.set_str("RPA-2.0"),
            handler_prop::K_CLASS_ID => val.set_guid(&crate::com::guid::RPA2_HANDLER_GUID),
            handler_prop::K_EXTENSION => val.set_str("rpa"),
            handler_prop::K_ADD_EXTENSION => val.set_str(""),
            handler_prop::K_FLAGS => {
                val.set_u32(arc_info_flags::K_KEEP_NAME | arc_info_flags::K_FIND_SIGNATURE)
            }
            handler_prop::K_UPDATE => val.set_bool(true),
            handler_prop::K_SIGNATURE => val.set_binary(b"RPA-2.0 "),
            handler_prop::K_SIGNATURE_OFFSET => val.set_u32(0),
            _ => S_OK,
        },
        2 => match prop_id {
            handler_prop::K_NAME => val.set_str("RPA-3.2"),
            handler_prop::K_CLASS_ID => val.set_guid(&crate::com::guid::RPA3_2_HANDLER_GUID),
            handler_prop::K_EXTENSION => val.set_str("rpa"),
            handler_prop::K_ADD_EXTENSION => val.set_str(""),
            handler_prop::K_FLAGS => {
                val.set_u32(arc_info_flags::K_KEEP_NAME | arc_info_flags::K_FIND_SIGNATURE)
            }
            handler_prop::K_UPDATE => val.set_bool(true),
            handler_prop::K_SIGNATURE => val.set_binary(b"RPA-3.2 "),
            handler_prop::K_SIGNATURE_OFFSET => val.set_u32(0),
            _ => S_OK,
        },
        _ => E_FAIL,
    }
}

/// Returns the number of archive formats supported by this plugin (3: RPA, RPA-2.0, RPA-3.2).
///
/// # Safety
/// The caller must ensure `num_formats` is a valid pointer.
#[no_mangle]
pub unsafe extern "system" fn GetNumberOfFormats(num_formats: *mut u32) -> HRESULT {
    if !num_formats.is_null() {
        *num_formats = 3;
        S_OK
    } else {
        E_INVALIDARG
    }
}
