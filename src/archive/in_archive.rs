#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_void;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::com::guid::{GUID, IID_IINARCHIVE, IID_IOUTARCHIVE, IID_IOUTSTREAM, IID_IUNKNOWN};
use crate::com::interfaces::{
    IArchiveExtractCallback, IArchiveOpenCallback, IArchiveUpdateCallback, IInArchiveVTable,
    IInStream, IOutArchiveVTable, IOutStream, ISequentialInStream, ISequentialOutStream,
};
use crate::com::propvariant::{
    E_FAIL, E_INVALIDARG, E_NOINTERFACE, HRESULT, PROPVARIANT, S_FALSE, S_OK, VARTYPE, VT_BOOL,
    VT_BSTR, VT_UI8,
};
use crate::com::stream::{SequentialInStreamReader, StreamReader, StreamWriter};
use crate::models::archive::RpaArchive;
use crate::models::error::RpaError;
use crate::models::props::*;
use crate::models::version::RpaVersion;
use crate::rpa::parser::RpaParser;
use crate::rpa::reader::RpaReader;
use crate::rpa::writer::{normalize_rpa_path, RpaItemSource, RpaWriter};

struct ArchiveState {
    archive: Arc<RpaArchive>,
    stream: *mut IInStream,
    phy_size: u64,
}

#[repr(C)]
pub struct RpaHandler {
    pub in_archive_vtable: *const IInArchiveVTable,
    pub out_archive_vtable: *const IOutArchiveVTable,
    ref_count: AtomicU32,
    state: Mutex<Option<ArchiveState>>,
}

pub type RpaInArchive = RpaHandler;

// Global static VTable for IInArchive
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

// Global static VTable for IOutArchive
static RPA_OUT_ARCHIVE_VTABLE: IOutArchiveVTable = IOutArchiveVTable {
    QueryInterface: rpa_out_query_interface,
    AddRef: rpa_out_add_ref,
    Release: rpa_out_release,
    UpdateItems: rpa_update_items,
    GetFileTimeType: rpa_get_file_time_type,
};

impl RpaHandler {
    pub fn new() -> *mut Self {
        let instance = Box::new(Self {
            in_archive_vtable: &RPA_IN_ARCHIVE_VTABLE,
            out_archive_vtable: &RPA_OUT_ARCHIVE_VTABLE,
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
    crate::dbg_log(&format!("rpa_query_interface: riid={:?}", riid.as_ref()));
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
        crate::dbg_log("rpa_query_interface -> S_OK (IInArchive)");
        S_OK
    } else if *riid == IID_IOUTARCHIVE {
        *ppvObject = (this as *mut u8).add(std::mem::size_of::<usize>()) as *mut c_void;
        rpa_add_ref(this);
        crate::dbg_log("rpa_query_interface -> S_OK (IOutArchive)");
        S_OK
    } else {
        crate::dbg_log(&format!("rpa_query_interface -> E_NOINTERFACE for {:?}", *riid));
        E_NOINTERFACE
    }
}

unsafe extern "system" fn rpa_add_ref(this: *mut c_void) -> u32 {
    let arc = &*(this as *mut RpaHandler);
    arc.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn rpa_release(this: *mut c_void) -> u32 {
    let arc = &*(this as *mut RpaHandler);
    let prev = arc.ref_count.fetch_sub(1, Ordering::SeqCst);
    if prev == 1 {
        // Drop the object
        let boxed = Box::from_raw(this as *mut RpaHandler);
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

// -------------------------------------------------------------------------
// IOutArchive interface forwarding
// -------------------------------------------------------------------------
#[inline]
unsafe fn get_handler_from_out(this: *mut c_void) -> *mut RpaHandler {
    (this as *mut u8).sub(std::mem::size_of::<usize>()) as *mut RpaHandler
}

unsafe extern "system" fn rpa_out_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppvObject: *mut *mut c_void,
) -> HRESULT {
    let handler = get_handler_from_out(this);
    rpa_query_interface(handler as *mut c_void, riid, ppvObject)
}

unsafe extern "system" fn rpa_out_add_ref(this: *mut c_void) -> u32 {
    let handler = get_handler_from_out(this);
    rpa_add_ref(handler as *mut c_void)
}

unsafe extern "system" fn rpa_out_release(this: *mut c_void) -> u32 {
    let handler = get_handler_from_out(this);
    rpa_release(handler as *mut c_void)
}

unsafe extern "system" fn rpa_open(
    this: *mut c_void,
    stream: *mut IInStream,
    maxCheckStartPosition: *const u64,
    openCallback: *mut IArchiveOpenCallback,
) -> HRESULT {
    crate::dbg_log(&format!(
        "rpa_open called: stream={:?}, maxCheckStart={:?}, callback={:?}",
        stream,
        maxCheckStartPosition.as_ref(),
        openCallback
    ));
    if stream.is_null() {
        return E_INVALIDARG;
    }

    let arc = &*(this as *mut RpaInArchive);
    let mut reader = StreamReader::new(stream);

    match RpaParser::parse(&mut reader) {
        Ok(archive) => {
            crate::dbg_log(&format!(
                "RpaParser::parse OK: version={:?}, files={}",
                archive.version,
                archive.len()
            ));
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

            let mut stream_len = 0u64;
            use std::io::{Seek, SeekFrom};
            if let Ok(len) = reader.seek(SeekFrom::End(0)) {
                stream_len = len;
                let _ = reader.seek(SeekFrom::Start(0));
            }

            *guard = Some(ArchiveState {
                archive: Arc::new(archive),
                stream,
                phy_size: stream_len,
            });
            S_OK
        }
        Err(e) => {
            crate::dbg_log(&format!("RpaParser::parse FAILED: {:?}", e));
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
    let count = {
        let guard = match arc.state.lock() {
            Ok(g) => g,
            Err(_) => return E_FAIL,
        };
        match &*guard {
            Some(state) => state.archive.len() as u32,
            None => 0,
        }
    };

    *numItems = count;
    S_OK
}

unsafe extern "system" fn rpa_get_property(
    this: *mut c_void,
    index: u32,
    propID: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    crate::dbg_log(&format!("rpa_get_property: index={}, propID={}", index, propID));
    if value.is_null() {
        return E_INVALIDARG;
    }

    let val = &mut *value;
    let arc = &*(this as *mut RpaInArchive);
    let archive = {
        let guard = match arc.state.lock() {
            Ok(g) => g,
            Err(_) => return E_FAIL,
        };
        match &*guard {
            Some(s) => s.archive.clone(),
            None => return val.set_empty(),
        }
    };

    let entry = match archive.get(index as usize) {
        Some(e) => e,
        None => return val.set_empty(),
    };

    match propID {
        KPID_PATH => val.set_str(&entry.path),
        KPID_NAME => val.set_str(&entry.name),
        KPID_IS_DIR => val.set_bool(entry.is_dir),
        KPID_SIZE => val.set_u64(entry.length()),
        KPID_PACK_SIZE => val.set_u64(entry.length()),
        KPID_ATTRIB => {
            let attr = if entry.is_dir { 0x10u32 } else { 0x20u32 };
            val.set_u32(attr)
        }
        KPID_ENCRYPTED => val.set_bool(false),
        KPID_SOLID => val.set_bool(false),
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
    crate::dbg_log(&format!(
        "rpa_extract called: indices={:?}, numItems={}, testMode={}, callback={:?}",
        indices, numItems, testMode, extractCallback
    ));
    if extractCallback.is_null() {
        crate::dbg_log("rpa_extract -> E_INVALIDARG (extractCallback is null)");
        return E_INVALIDARG;
    }

    let arc = &*(this as *mut RpaInArchive);
    let (archive, stream) = {
        let guard = match arc.state.lock() {
            Ok(g) => g,
            Err(_) => {
                crate::dbg_log("rpa_extract -> E_FAIL (mutex lock failed)");
                return E_FAIL;
            }
        };

        match &*guard {
            Some(s) => (s.archive.clone(), s.stream),
            None => {
                crate::dbg_log("rpa_extract -> E_FAIL (state is None)");
                return E_FAIL;
            }
        }
    }; // Lock is dropped immediately!

    let total_archive_items = archive.len();
    let is_all_items = indices.is_null() || numItems == u32::MAX;
    let count = if is_all_items {
        total_archive_items as u32
    } else {
        numItems
    };

    if count == 0 {
        return S_OK;
    }

    // 1. Calculate total uncompressed size across requested items
    let mut total_unpack_size = 0u64;
    for i in 0..count {
        let item_index = if is_all_items {
            i
        } else {
            *indices.add(i as usize)
        };
        if let Some(entry) = archive.get(item_index as usize) {
            total_unpack_size += entry.length();
        }
    }

    // 2. Report total size to 7-Zip progress bar
    let hr = ((*(*extractCallback).vtable).SetTotal)(
        extractCallback as *mut c_void,
        total_unpack_size,
    );
    if hr != S_OK {
        return hr;
    }

    let mut completed_bytes = 0u64;
    let mut stream_reader = StreamReader::new(stream);

    for i in 0..count {
        let item_index = if is_all_items {
            i
        } else {
            *indices.add(i as usize)
        };

        if item_index as usize >= total_archive_items {
            continue;
        }

        let entry = match archive.get(item_index as usize) {
            Some(e) => e,
            None => continue,
        };

        let ask_extract_mode = if testMode != 0 {
            ask_mode::K_TEST
        } else {
            ask_mode::K_EXTRACT
        };

        crate::dbg_log(&format!(
            "Item {}/{}: index={}, path='{}', len={}, ask_mode={}",
            i + 1, count, item_index, entry.path, entry.length(), ask_extract_mode
        ));

        // 3. In 7-Zip's COM protocol, GetStream is called FIRST
        let mut out_stream: *mut ISequentialOutStream = std::ptr::null_mut();
        let hr = ((*(*extractCallback).vtable).GetStream)(
            extractCallback as *mut c_void,
            item_index,
            &mut out_stream,
            ask_extract_mode,
        );

        crate::dbg_log(&format!("  GetStream returned 0x{:08X}, out_stream={:?}", hr, out_stream));
        if hr != S_OK {
            return hr;
        }

        if entry.is_dir {
            crate::dbg_log("  Entry is dir, skipping stream write");
            ((*(*extractCallback).vtable).PrepareOperation)(
                extractCallback as *mut c_void,
                ask_extract_mode,
            );
            ((*(*extractCallback).vtable).SetOperationResult)(
                extractCallback as *mut c_void,
                operation_result::K_OK,
            );
            continue;
        }

        if testMode == 0 && out_stream.is_null() {
            crate::dbg_log("  out_stream is null in extract mode (file skipped)");
            // User skipped file (e.g. skip overwrite)
            continue;
        }

        // 4. PrepareOperation is called SECOND
        let hr = ((*(*extractCallback).vtable).PrepareOperation)(
            extractCallback as *mut c_void,
            ask_extract_mode,
        );
        crate::dbg_log(&format!("  PrepareOperation returned 0x{:08X}", hr));
        if hr != S_OK {
            if !out_stream.is_null() {
                ((*(*out_stream).vtable).Release)(out_stream as *mut c_void);
            }
            return hr;
        }

        // 5. Perform extraction or testing
        let op_result = if testMode != 0 {
            let mut sink = std::io::sink();
            match RpaReader::stream_entry(&mut stream_reader, entry, &mut sink) {
                Ok(_) => operation_result::K_OK,
                Err(e) => {
                    crate::dbg_log(&format!("  Test mode stream_entry error: {:?}", e));
                    operation_result::K_DATA_ERROR
                }
            }
        } else {
            let mut writer = OutStreamWriter(out_stream);
            let res = RpaReader::stream_entry(&mut stream_reader, entry, &mut writer);

            // CRITICAL: Release out_stream BEFORE SetOperationResult.
            // In 7-Zip, SetOperationResult finalizes file timestamps/attributes and closes handles.
            ((*(*out_stream).vtable).Release)(out_stream as *mut c_void);

            match res {
                Ok(written) => {
                    crate::dbg_log(&format!("  stream_entry wrote {} bytes OK", written));
                    operation_result::K_OK
                }
                Err(e) => {
                    crate::dbg_log(&format!("  stream_entry FAILED: {:?}", e));
                    operation_result::K_DATA_ERROR
                }
            }
        };

        // 6. Update progress bar
        completed_bytes += entry.length();
        let hr_progress = ((*(*extractCallback).vtable).SetCompleted)(
            extractCallback as *mut c_void,
            &completed_bytes,
        );
        if hr_progress != S_OK {
            crate::dbg_log(&format!("  SetCompleted returned error 0x{:08X}", hr_progress));
            return hr_progress;
        }

        // 7. SetOperationResult is called LAST for each item
        let hr_res = ((*(*extractCallback).vtable).SetOperationResult)(
            extractCallback as *mut c_void,
            op_result,
        );
        crate::dbg_log(&format!("  SetOperationResult({}) returned 0x{:08X}", op_result, hr_res));
        if hr_res != S_OK {
            return hr_res;
        }
    }

    crate::dbg_log("rpa_extract completed successfully with S_OK");
    S_OK
}

unsafe extern "system" fn rpa_get_archive_property(
    this: *mut c_void,
    propID: PROPID,
    value: *mut PROPVARIANT,
) -> HRESULT {
    crate::dbg_log(&format!("rpa_get_archive_property: propID={}", propID));
    if value.is_null() {
        return E_INVALIDARG;
    }

    let val = &mut *value;
    let arc = &*(this as *mut RpaInArchive);
    let (read_only, headers_size, phy_size) = {
        let guard = match arc.state.lock() {
            Ok(g) => g,
            Err(_) => return E_FAIL,
        };
        match &*guard {
            Some(state) => (false, Some(state.archive.index_offset), Some(state.phy_size)),
            None => (false, None, None),
        }
    };

    match propID {
        KPID_READ_ONLY => val.set_bool(read_only),
        KPID_PHY_SIZE => {
            if let Some(sz) = phy_size {
                val.set_u64(sz)
            } else {
                val.set_empty()
            }
        }
        KPID_HEADERS_SIZE => {
            if let Some(offset) = headers_size {
                val.set_u64(offset)
            } else {
                val.set_empty()
            }
        }
        KPID_OFFSET => val.set_empty(),
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

// =========================================================================
// IOutArchive Implementation
// =========================================================================

unsafe extern "system" fn rpa_get_file_time_type(
    _this: *mut c_void,
    type_: *mut u32,
) -> HRESULT {
    if !type_.is_null() {
        *type_ = 0; // NFileTimeType::kWindows
        S_OK
    } else {
        E_INVALIDARG
    }
}

enum UpdateSource {
    Stream(*mut ISequentialInStream),
    Existing(Arc<RpaArchive>, *mut IInStream, usize),
}

struct UpdateItem {
    #[allow(dead_code)]
    client_index: u32,
    path: String,
    size_hint: Option<u64>,
    source: UpdateSource,
}

impl RpaItemSource for UpdateItem {
    fn path(&self) -> &str {
        &self.path
    }

    fn size_hint(&self) -> Option<u64> {
        self.size_hint
    }

    fn write_to<W: Write>(&mut self, writer: &mut W) -> std::io::Result<u64> {
        match &self.source {
            UpdateSource::Stream(in_stream) => {
                let mut reader = unsafe { SequentialInStreamReader::new(*in_stream) };
                std::io::copy(&mut reader, writer)
            }
            UpdateSource::Existing(archive, stream, idx) => {
                let entry = &archive.entries[*idx];
                let mut reader = unsafe { StreamReader::new(*stream) };
                reader.seek(SeekFrom::Start(entry.offset()))?;

                let mut total_written = 0u64;
                if !entry.prefix().is_empty() {
                    writer.write_all(entry.prefix())?;
                    total_written += entry.prefix().len() as u64;
                }

                let remaining = entry.length().saturating_sub(entry.prefix().len() as u64);
                let mut taken = (&mut reader).take(remaining);
                let copied = std::io::copy(&mut taken, writer)?;
                total_written += copied;
                Ok(total_written)
            }
        }
    }
}

unsafe fn cleanup_update_items(items: &mut [UpdateItem], seek_opt: Option<*mut IOutStream>) {
    for item in items.iter_mut() {
        if let UpdateSource::Stream(ptr) = item.source {
            if !ptr.is_null() {
                ((*(*ptr).vtable).Release)(ptr as *mut c_void);
            }
        }
    }
    if let Some(s) = seek_opt {
        ((*(*s).vtable).Release)(s as *mut c_void);
    }
}

unsafe extern "system" fn rpa_update_items(
    this: *mut c_void,
    outStream: *mut ISequentialOutStream,
    numItems: u32,
    updateCallback: *mut IArchiveUpdateCallback,
) -> HRESULT {
    crate::dbg_log(&format!("rpa_update_items: numItems={}", numItems));
    if outStream.is_null() || updateCallback.is_null() {
        return E_INVALIDARG;
    }

    let handler = get_handler_from_out(this);

    // 1. Inspect existing state if open
    let (existing_archive, existing_stream, target_version, target_key) = {
        let guard = match (*handler).state.lock() {
            Ok(g) => g,
            Err(_) => return E_FAIL,
        };
        if let Some(state) = &*guard {
            (
                Some(Arc::clone(&state.archive)),
                Some(state.stream),
                state.archive.version,
                state.archive.key,
            )
        } else {
            (None, None, RpaVersion::Rpa3, 0u32)
        }
    };

    // 2. Query IOutStream on outStream to check if seeking is supported
    let mut out_seek: *mut IOutStream = std::ptr::null_mut();
    let hr_seek = ((*(*outStream).vtable).QueryInterface)(
        outStream as *mut c_void,
        &IID_IOUTSTREAM,
        &mut out_seek as *mut *mut IOutStream as *mut *mut c_void,
    );
    let seek_opt = if hr_seek == S_OK && !out_seek.is_null() {
        Some(out_seek)
    } else {
        None
    };

    let mut update_items = Vec::with_capacity(numItems as usize);
    let mut total_unpack_size = 0u64;

    // 3. Collect update items
    for i in 0..numItems {
        let mut new_data: i32 = 0;
        let mut new_props: i32 = 0;
        let mut index_in_arc: u32 = u32::MAX;

        let hr = ((*(*updateCallback).vtable).GetUpdateItemInfo)(
            updateCallback as *mut c_void,
            i,
            &mut new_data,
            &mut new_props,
            &mut index_in_arc,
        );
        if hr != S_OK {
            cleanup_update_items(&mut update_items, seek_opt);
            return hr;
        }

        // Check if directory
        let mut is_dir = false;
        if new_props != 0 {
            let mut var = PROPVARIANT::new();
            let hr = ((*(*updateCallback).vtable).GetProperty)(
                updateCallback as *mut c_void,
                i,
                KPID_IS_DIR,
                &mut var,
            );
            if hr == S_OK && var.vt == VT_BOOL {
                is_dir = unsafe { var.data.boolVal != 0 };
            }
            var.clear();
        }
        if is_dir {
            continue;
        }

        // Path
        let path = if new_props != 0 {
            let mut var = PROPVARIANT::new();
            let hr = ((*(*updateCallback).vtable).GetProperty)(
                updateCallback as *mut c_void,
                i,
                KPID_PATH,
                &mut var,
            );
            let p = if hr == S_OK && var.vt == VT_BSTR {
                var.get_str().unwrap_or_default()
            } else {
                String::new()
            };
            var.clear();
            normalize_rpa_path(&p)
        } else if let Some(ref arc) = existing_archive {
            if (index_in_arc as usize) < arc.len() {
                normalize_rpa_path(&arc.entries[index_in_arc as usize].path)
            } else {
                continue;
            }
        } else {
            continue;
        };

        if path.is_empty() {
            continue;
        }

        // Size hint for progress bar
        let mut size_hint: Option<u64> = None;
        if new_props != 0 {
            let mut var = PROPVARIANT::new();
            let hr = ((*(*updateCallback).vtable).GetProperty)(
                updateCallback as *mut c_void,
                i,
                KPID_SIZE,
                &mut var,
            );
            if hr == S_OK && var.vt == VT_UI8 {
                let sz = unsafe { var.data.uhVal };
                size_hint = Some(sz);
                total_unpack_size += sz;
            }
            var.clear();
        } else if let Some(ref arc) = existing_archive {
            if (index_in_arc as usize) < arc.len() {
                let sz = arc.entries[index_in_arc as usize].length();
                size_hint = Some(sz);
                total_unpack_size += sz;
            }
        }

        if new_data != 0 {
            let mut in_stream: *mut ISequentialInStream = std::ptr::null_mut();
            let hr = ((*(*updateCallback).vtable).GetStream)(
                updateCallback as *mut c_void,
                i,
                &mut in_stream,
            );
            if hr == S_FALSE || in_stream.is_null() {
                continue;
            } else if hr != S_OK {
                cleanup_update_items(&mut update_items, seek_opt);
                return hr;
            }

            update_items.push(UpdateItem {
                client_index: i,
                path,
                size_hint,
                source: UpdateSource::Stream(in_stream),
            });
        } else if let (Some(arc), Some(stream)) = (&existing_archive, existing_stream) {
            if (index_in_arc as usize) < arc.len() {
                update_items.push(UpdateItem {
                    client_index: i,
                    path,
                    size_hint,
                    source: UpdateSource::Existing(Arc::clone(arc), stream, index_in_arc as usize),
                });
            }
        }
    }

    // 4. Report total size to progress bar
    let _ = ((*(*updateCallback).vtable).SetTotal)(updateCallback as *mut c_void, total_unpack_size);

    let mut completed_bytes = 0u64;
    let on_progress = |bytes: u64| {
        completed_bytes += bytes;
        let _ = ((*(*updateCallback).vtable).SetCompleted)(
            updateCallback as *mut c_void,
            &completed_bytes,
        );
    };

    // 5. Perform archive writing
    let write_res = if let Some(s) = seek_opt {
        let mut stream_writer = StreamWriter::new(outStream, Some(s));
        RpaWriter::write_archive(
            &mut stream_writer,
            &mut update_items,
            target_version,
            target_key,
            on_progress,
        )
    } else {
        // Fallback for non-seekable streams: buffer to tempfile
        match tempfile::tempfile() {
            Ok(mut temp) => {
                let res = RpaWriter::write_archive(
                    &mut temp,
                    &mut update_items,
                    target_version,
                    target_key,
                    on_progress,
                );
                if res.is_ok() {
                    let _ = temp.seek(SeekFrom::Start(0));
                    let mut seq_writer = StreamWriter::new(outStream, None);
                    let _ = std::io::copy(&mut temp, &mut seq_writer);
                }
                res
            }
            Err(e) => Err(RpaError::Io(e)),
        }
    };

    // 6. Report results for each item and clean up resources
    for item in update_items.iter_mut() {
        if write_res.is_ok() {
            let _ = ((*(*updateCallback).vtable).SetOperationResult)(
                updateCallback as *mut c_void,
                0, // kOK
            );
        }
        if let UpdateSource::Stream(ptr) = item.source {
            if !ptr.is_null() {
                ((*(*ptr).vtable).Release)(ptr as *mut c_void);
            }
        }
    }

    if let Some(s) = seek_opt {
        ((*(*s).vtable).Release)(s as *mut c_void);
    }

    match write_res {
        Ok(_) => {
            crate::dbg_log("rpa_update_items completed successfully with S_OK");
            S_OK
        }
        Err(e) => {
            crate::dbg_log(&format!("rpa_update_items failed: {}", e));
            E_FAIL
        }
    }
}
