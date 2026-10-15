#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_void;
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::com::guid::{GUID, IID_IINARCHIVE, IID_IUNKNOWN};
use crate::com::interfaces::{
    IArchiveExtractCallback, IArchiveOpenCallback, IInArchiveVTable, IInStream,
    ISequentialOutStream,
};
use crate::com::propvariant::{
    E_FAIL, E_INVALIDARG, E_NOINTERFACE, HRESULT, PROPVARIANT, S_FALSE, S_OK, VARTYPE,
};
use crate::com::stream::StreamReader;
use crate::models::archive::RpaArchive;
use crate::models::props::*;
use crate::rpa::parser::RpaParser;
use crate::rpa::reader::RpaReader;

struct ArchiveState {
    archive: RpaArchive,
    stream: *mut IInStream,
}

#[repr(C)]
pub struct RpaInArchive {
    pub vtable: *const IInArchiveVTable,
    ref_count: AtomicU32,
    state: Mutex<Option<ArchiveState>>,
}

// Global static VTable for RpaInArchive
static RPA_IN_ARCHIVE_VTABLE: IInArchiveVTable = IInArchiveVTable {
    QueryInterface: rpa_query_interface,
    AddRef: rpa_add_ref,
    Release: rpa_release,
    Open: rpa_open,
    Close: rpa_close,
    GetNumberOfItems: rpa_get_number_of_items,
    GetProperty: rpa_get_property,
    Extract: rpa_extract,
    GetArchiveProperty: rpa_get_archive_property,
    GetNumberOfProperties: rpa_get_number_of_properties,
    GetPropertyInfo: rpa_get_property_info,
    GetNumberOfArchiveProperties: rpa_get_number_of_archive_properties,
    GetArchivePropertyInfo: rpa_get_archive_property_info,
};

impl RpaInArchive {
    pub fn new() -> *mut Self {
        let instance = Box::new(Self {
            vtable: &RPA_IN_ARCHIVE_VTABLE,
            ref_count: AtomicU32::new(1),
            state: Mutex::new(None),
        });
        Box::into_raw(instance)
    }
}

unsafe extern "system" fn rpa_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppvObject: *mut *mut c_void,
) -> HRESULT {
    if ppvObject.is_null() {
        return E_INVALIDARG;
    }
    *ppvObject = std::ptr::null_mut();

    if riid.is_null() {
        return E_INVALIDARG;
    }

    if *riid == IID_IINARCHIVE || *riid == IID_IUNKNOWN {
        *ppvObject = this;
        rpa_add_ref(this);
        S_OK
    } else {
        E_NOINTERFACE
    }
}

unsafe extern "system" fn rpa_add_ref(this: *mut c_void) -> u32 {
    let arc = &*(this as *mut RpaInArchive);
    arc.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn rpa_release(this: *mut c_void) -> u32 {
    let arc = &*(this as *mut RpaInArchive);
    let prev = arc.ref_count.fetch_sub(1, Ordering::SeqCst);
    if prev == 1 {
        // Drop the object
        let boxed = Box::from_raw(this as *mut RpaInArchive);
        if let Ok(mut guard) = boxed.state.lock() {
            if let Some(state) = guard.take() {
                if !state.stream.is_null() {
                    ((*(*state.stream).vtable).Release)(state.stream as *mut c_void);
                }
            }
        }
        0
    } else {
        prev - 1
    }
}

unsafe extern "system" fn rpa_open(
    this: *mut c_void,
    stream: *mut IInStream,
    _maxCheckStartPosition: *const u64,
    openCallback: *mut IArchiveOpenCallback,
) -> HRESULT {
    if stream.is_null() {
        return E_INVALIDARG;
    }

    let arc = &*(this as *mut RpaInArchive);
    let mut reader = StreamReader::new(stream);

    match RpaParser::parse(&mut reader) {
        Ok(archive) => {
            // Retain stream reference
            ((*(*stream).vtable).AddRef)(stream as *mut c_void);

            if !openCallback.is_null() {
                let num_files = archive.len() as u64;
                ((*(*openCallback).vtable).SetTotal)(
                    openCallback as *mut c_void,
                    &num_files,
                    std::ptr::null(),
                );
                ((*(*openCallback).vtable).SetCompleted)(
                    openCallback as *mut c_void,
                    &num_files,
                    std::ptr::null(),
                );
            }

            let mut guard = match arc.state.lock() {
                Ok(g) => g,
                Err(_) => return E_FAIL,
            };

            // Release previous stream if any
            if let Some(prev) = guard.take() {
                if !prev.stream.is_null() {
                    ((*(*prev.stream).vtable).Release)(prev.stream as *mut c_void);
                }
            }

            *guard = Some(ArchiveState { archive, stream });
            S_OK
        }
        Err(_) => {
            // Not a recognized RPA archive
            S_FALSE
        }
    }
}

unsafe extern "system" fn rpa_close(this: *mut c_void) -> HRESULT {
    let arc = &*(this as *mut RpaInArchive);
    if let Ok(mut guard) = arc.state.lock() {
        if let Some(state) = guard.take() {
            if !state.stream.is_null() {
                ((*(*state.stream).vtable).Release)(state.stream as *mut c_void);
            }
        }
    }
    S_OK
}

unsafe extern "system" fn rpa_get_number_of_items(
    this: *mut c_void,
    numItems: *mut u32,
) -> HRESULT {
    if numItems.is_null() {
        return E_INVALIDARG;
    }

    let arc = &*(this as *mut RpaInArchive);
    let guard = match arc.state.lock() {
        Ok(g) => g,
        Err(_) => return E_FAIL,
    };

    match &*guard {
        Some(state) => {
            *numItems = state.archive.len() as u32;
            S_OK
        }
        None => {
            *numItems = 0;
            S_OK
        }
    }
}

unsafe extern "system" fn rpa_get_property(
    this: *mut c_void,
    index: u32,
    propID: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    if value.is_null() {
        return E_INVALIDARG;
    }

    let val = &mut *value;
    let arc = &*(this as *mut RpaInArchive);
    let guard = match arc.state.lock() {
        Ok(g) => g,
        Err(_) => return E_FAIL,
    };

    let state = match &*guard {
        Some(s) => s,
        None => return val.set_empty(),
    };

    let entry = match state.archive.get(index as usize) {
        Some(e) => e,
        None => return val.set_empty(),
    };

    match propID {
        KPID_PATH => val.set_str(&entry.path),
        KPID_NAME => val.set_str(&entry.name),
        KPID_IS_DIR => val.set_bool(entry.is_dir),
        KPID_SIZE => val.set_u64(entry.length()),
        KPID_PACK_SIZE => val.set_u64(entry.length()),
        KPID_POSITION => val.set_u64(entry.offset()),
        KPID_ENCRYPTED => val.set_bool(false),
        KPID_SOLID => val.set_bool(false),
        KPID_READ_ONLY => val.set_bool(true),
        _ => val.set_empty(),
    }
}

struct OutStreamWriter(*mut ISequentialOutStream);

impl Write for OutStreamWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut processed: u32 = 0;
        let hr = unsafe {
            ((*(*self.0).vtable).Write)(
                self.0 as *mut c_void,
                buf.as_ptr() as *const c_void,
                buf.len() as u32,
                &mut processed,
            )
        };
        if hr == S_OK {
            Ok(processed as usize)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("ISequentialOutStream::Write failed (HRESULT 0x{:08X})", hr),
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

unsafe extern "system" fn rpa_extract(
    this: *mut c_void,
    indices: *const u32,
    numItems: u32,
    testMode: i32,
    extractCallback: *mut IArchiveExtractCallback,
) -> HRESULT {
    if extractCallback.is_null() {
        return E_INVALIDARG;
    }

    let arc = &*(this as *mut RpaInArchive);
    let guard = match arc.state.lock() {
        Ok(g) => g,
        Err(_) => return E_FAIL,
    };

    let state = match &*guard {
        Some(s) => s,
        None => return E_FAIL,
    };

    let total_archive_items = state.archive.len();
    let is_all_items = indices.is_null() || numItems == u32::MAX;
    let count = if is_all_items {
        total_archive_items as u32
    } else {
        numItems
    };

    let mut stream_reader = StreamReader::new(state.stream);

    for i in 0..count {
        let item_index = if is_all_items {
            i
        } else {
            *indices.add(i as usize)
        };

        if item_index as usize >= total_archive_items {
            continue;
        }

        let entry = match state.archive.get(item_index as usize) {
            Some(e) => e,
            None => continue,
        };

        let ask_extract_mode = if testMode != 0 {
            ask_mode::K_TEST
        } else {
            ask_mode::K_EXTRACT
        };

        ((*(*extractCallback).vtable).PrepareOperation)(
            extractCallback as *mut c_void,
            ask_extract_mode,
        );

        if testMode != 0 {
            // Test mode: we can optionally verify reading the stream
            ((*(*extractCallback).vtable).SetOperationResult)(
                extractCallback as *mut c_void,
                operation_result::K_OK,
            );
            continue;
        }

        let mut out_stream: *mut ISequentialOutStream = std::ptr::null_mut();
        let hr = ((*(*extractCallback).vtable).GetStream)(
            extractCallback as *mut c_void,
            item_index,
            &mut out_stream,
            ask_extract_mode,
        );

        if hr != S_OK {
            continue;
        }

        if out_stream.is_null() {
            // User skipped file
            continue;
        }

        let mut writer = OutStreamWriter(out_stream);
        let stream_result = RpaReader::stream_entry(&mut stream_reader, entry, &mut writer);

        // Always release the out_stream COM reference
        ((*(*out_stream).vtable).Release)(out_stream as *mut c_void);

        let op_result = match stream_result {
            Ok(_) => operation_result::K_OK,
            Err(_) => operation_result::K_DATA_ERROR,
        };

        ((*(*extractCallback).vtable).SetOperationResult)(
            extractCallback as *mut c_void,
            op_result,
        );
    }

    S_OK
}

unsafe extern "system" fn rpa_get_archive_property(
    this: *mut c_void,
    propID: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    if value.is_null() {
        return E_INVALIDARG;
    }

    let val = &mut *value;
    let arc = &*(this as *mut RpaInArchive);
    let guard = match arc.state.lock() {
        Ok(g) => g,
        Err(_) => return E_FAIL,
    };

    match propID {
        KPID_READ_ONLY => val.set_bool(true),
        KPID_OFFSET => val.set_u64(0),
        KPID_HEADERS_SIZE => {
            if let Some(state) = &*guard {
                val.set_u64(state.archive.index_offset)
            } else {
                val.set_empty()
            }
        }
        _ => val.set_empty(),
    }
}

unsafe extern "system" fn rpa_get_number_of_properties(
    _this: *mut c_void,
    numProps: *mut u32,
) -> HRESULT {
    if !numProps.is_null() {
        *numProps = 0;
        S_OK
    } else {
        E_INVALIDARG
    }
}

unsafe extern "system" fn rpa_get_property_info(
    _this: *mut c_void,
    _index: u32,
    _name: *mut *mut u16,
    _propID: *mut PROPID,
    _varType: *mut VARTYPE,
) -> HRESULT {
    S_OK
}

unsafe extern "system" fn rpa_get_number_of_archive_properties(
    _this: *mut c_void,
    numProps: *mut u32,
) -> HRESULT {
    if !numProps.is_null() {
        *numProps = 0;
        S_OK
    } else {
        E_INVALIDARG
    }
}

unsafe extern "system" fn rpa_get_archive_property_info(
    _this: *mut c_void,
    _index: u32,
    _name: *mut *mut u16,
    _propID: *mut PROPID,
    _varType: *mut VARTYPE,
) -> HRESULT {
    S_OK
}
