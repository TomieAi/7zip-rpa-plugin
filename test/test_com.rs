#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_void;

use rpa::com::guid::{
    IID_IINARCHIVE, IID_IOUTARCHIVE, IID_IOUTSTREAM, IID_ISEQUENTIALOUTSTREAM, IID_ISETPROPERTIES,
    IID_IUNKNOWN, RPA_HANDLER_GUID,
};
use rpa::com::interfaces::{
    IArchiveUpdateCallback, IArchiveUpdateCallbackVTable, IInArchive, IOutArchive,
    IOutStreamVTable, ISequentialInStream, ISequentialInStreamVTable, ISetProperties,
};
use rpa::com::propvariant::{PROPVARIANT, S_OK, VT_BOOL, VT_BSTR, VT_UI4};
use rpa::models::props::handler_prop;
use rpa::{CreateObject, GetHandlerProperty, GetHandlerProperty2, GetNumberOfFormats};

#[test]
fn test_get_number_of_formats() {
    let mut num_formats: u32 = 0;
    let hr = unsafe { GetNumberOfFormats(&mut num_formats) };
    assert_eq!(hr, S_OK);
    assert_eq!(num_formats, 3);
}

#[test]
fn test_get_handler_property_metadata() {
    let mut val = PROPVARIANT::new();

    // Name
    let hr = unsafe { GetHandlerProperty(handler_prop::K_NAME, &mut val) };
    assert_eq!(hr, S_OK);
    assert_eq!(val.vt, VT_BSTR);
    val.clear();

    // ClassID
    let hr = unsafe { GetHandlerProperty(handler_prop::K_CLASS_ID, &mut val) };
    assert_eq!(hr, S_OK);
    assert_eq!(val.vt, VT_BSTR);
    val.clear();

    // Extension
    let hr = unsafe { GetHandlerProperty2(0, handler_prop::K_EXTENSION, &mut val) };
    assert_eq!(hr, S_OK);
    assert_eq!(val.vt, VT_BSTR);
    val.clear();

    // Flags
    let hr = unsafe { GetHandlerProperty2(0, handler_prop::K_FLAGS, &mut val) };
    assert_eq!(hr, S_OK);
    assert_eq!(val.vt, VT_UI4);
    val.clear();

    // Update
    let hr = unsafe { GetHandlerProperty2(0, handler_prop::K_UPDATE, &mut val) };
    assert_eq!(hr, S_OK);
    assert_eq!(val.vt, VT_BOOL);
    val.clear();

    // Signature
    let hr = unsafe { GetHandlerProperty2(0, handler_prop::K_SIGNATURE, &mut val) };
    assert_eq!(hr, S_OK);
    assert_eq!(val.vt, VT_BSTR);
    val.clear();
}

#[test]
fn test_create_object_and_com_lifecycle() {
    let mut obj: *mut c_void = std::ptr::null_mut();

    // Create with RPA handler GUID and IID_IINARCHIVE
    let hr = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IINARCHIVE, &mut obj) };
    assert_eq!(hr, S_OK);
    assert!(!obj.is_null());

    let archive = obj as *mut IInArchive;

    // Test AddRef & Release
    let ref1 = unsafe { ((*(*archive).vtable).AddRef)(archive as *mut c_void) };
    assert_eq!(ref1, 2);

    let ref2 = unsafe { ((*(*archive).vtable).Release)(archive as *mut c_void) };
    assert_eq!(ref2, 1);

    // Test QueryInterface for IID_IUNKNOWN
    let mut unk_obj: *mut c_void = std::ptr::null_mut();
    let hr_qi =
        unsafe { ((*(*archive).vtable).QueryInterface)(archive as *mut c_void, &IID_IUNKNOWN, &mut unk_obj) };
    assert_eq!(hr_qi, S_OK);
    assert_eq!(unk_obj, archive as *mut c_void);

    // Release unk_obj
    unsafe { ((*(*archive).vtable).Release)(archive as *mut c_void) };

    // Test GetNumberOfItems before open (should be 0)
    let mut count: u32 = 999;
    let hr_items = unsafe { ((*(*archive).vtable).GetNumberOfItems)(archive as *mut c_void, &mut count) };
    assert_eq!(hr_items, S_OK);
    assert_eq!(count, 0);

    // Final Release: should destroy object without leaking or crashing
    let final_ref = unsafe { ((*(*archive).vtable).Release)(archive as *mut c_void) };
    assert_eq!(final_ref, 0);
}

// =========================================================================
// End-to-end COM Extraction Test
// =========================================================================

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use rpa::com::interfaces::{
    IArchiveExtractCallback, IArchiveExtractCallbackVTable, IInStreamVTable,
    ISequentialOutStream, ISequentialOutStreamVTable,
};
use rpa::com::propvariant::{E_FAIL, E_NOINTERFACE};
use rpa::models::props::*;

// Mock IInStream wrapping std::fs::File
#[repr(C)]
struct MockInStream {
    vtable: *const IInStreamVTable,
    file: Mutex<File>,
}

static MOCK_IN_STREAM_VTABLE: IInStreamVTable = IInStreamVTable {
    QueryInterface: mock_instream_qi,
    AddRef: mock_instream_addref,
    Release: mock_instream_release,
    Read: mock_instream_read,
    Seek: mock_instream_seek,
};

unsafe extern "system" fn mock_instream_qi(
    _this: *mut c_void,
    _riid: *const rpa::com::guid::GUID,
    _ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    E_NOINTERFACE
}
unsafe extern "system" fn mock_instream_addref(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_instream_release(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_instream_read(
    this: *mut c_void,
    data: *mut c_void,
    size: u32,
    processed: *mut u32,
) -> rpa::com::propvariant::HRESULT {
    let s = &*(this as *mut MockInStream);
    let mut f = s.file.lock().unwrap();
    let slice = std::slice::from_raw_parts_mut(data as *mut u8, size as usize);
    match f.read(slice) {
        Ok(n) => {
            if !processed.is_null() {
                *processed = n as u32;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}
unsafe extern "system" fn mock_instream_seek(
    this: *mut c_void,
    offset: i64,
    origin: u32,
    new_pos: *mut u64,
) -> rpa::com::propvariant::HRESULT {
    let s = &*(this as *mut MockInStream);
    let mut f = s.file.lock().unwrap();
    let seek_from = match origin {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return E_FAIL,
    };
    match f.seek(seek_from) {
        Ok(pos) => {
            if !new_pos.is_null() {
                *new_pos = pos;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}

// Mock ISequentialOutStream
#[repr(C)]
struct MockOutStream {
    vtable: *const ISequentialOutStreamVTable,
    buffer: Arc<Mutex<Vec<u8>>>,
}

static MOCK_OUT_STREAM_VTABLE: ISequentialOutStreamVTable = ISequentialOutStreamVTable {
    QueryInterface: mock_outstream_qi,
    AddRef: mock_outstream_addref,
    Release: mock_outstream_release,
    Write: mock_outstream_write,
};

unsafe extern "system" fn mock_outstream_qi(
    _this: *mut c_void,
    _riid: *const rpa::com::guid::GUID,
    _ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    E_NOINTERFACE
}
unsafe extern "system" fn mock_outstream_addref(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_outstream_release(this: *mut c_void) -> u32 {
    let _ = Box::from_raw(this as *mut MockOutStream);
    0
}
unsafe extern "system" fn mock_outstream_write(
    this: *mut c_void,
    data: *const c_void,
    size: u32,
    processed: *mut u32,
) -> rpa::com::propvariant::HRESULT {
    let s = &*(this as *mut MockOutStream);
    let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
    let mut buf = s.buffer.lock().unwrap();
    buf.extend_from_slice(slice);
    if !processed.is_null() {
        *processed = size;
    }
    S_OK
}

// Mock IArchiveExtractCallback
#[repr(C)]
struct MockExtractCallback {
    vtable: *const IArchiveExtractCallbackVTable,
    total: AtomicU64,
    completed: AtomicU64,
    last_op_result: Mutex<Option<i32>>,
    files: Mutex<HashMap<u32, Arc<Mutex<Vec<u8>>>>>,
}

static MOCK_EXTRACT_CALLBACK_VTABLE: IArchiveExtractCallbackVTable = IArchiveExtractCallbackVTable {
    QueryInterface: mock_callback_qi,
    AddRef: mock_callback_addref,
    Release: mock_callback_release,
    SetTotal: mock_callback_set_total,
    SetCompleted: mock_callback_set_completed,
    GetStream: mock_callback_get_stream,
    PrepareOperation: mock_callback_prepare_op,
    SetOperationResult: mock_callback_set_op_result,
};

unsafe extern "system" fn mock_callback_qi(
    _this: *mut c_void,
    _riid: *const rpa::com::guid::GUID,
    _ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    E_NOINTERFACE
}
unsafe extern "system" fn mock_callback_addref(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_callback_release(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_callback_set_total(this: *mut c_void, total: u64) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockExtractCallback);
    cb.total.store(total, std::sync::atomic::Ordering::SeqCst);
    S_OK
}
unsafe extern "system" fn mock_callback_set_completed(
    this: *mut c_void,
    complete_value: *const u64,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockExtractCallback);
    if !complete_value.is_null() {
        cb.completed.store(*complete_value, std::sync::atomic::Ordering::SeqCst);
    }
    S_OK
}
unsafe extern "system" fn mock_callback_get_stream(
    this: *mut c_void,
    index: u32,
    out_stream: *mut *mut ISequentialOutStream,
    _ask_mode: i32,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockExtractCallback);
    let buf = Arc::new(Mutex::new(Vec::new()));
    cb.files.lock().unwrap().insert(index, buf.clone());

    let stream = Box::new(MockOutStream {
        vtable: &MOCK_OUT_STREAM_VTABLE,
        buffer: buf,
    });
    *out_stream = Box::into_raw(stream) as *mut ISequentialOutStream;
    S_OK
}
unsafe extern "system" fn mock_callback_prepare_op(
    _this: *mut c_void,
    _ask_mode: i32,
) -> rpa::com::propvariant::HRESULT {
    S_OK
}
unsafe extern "system" fn mock_callback_set_op_result(
    this: *mut c_void,
    op_res: i32,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockExtractCallback);
    *cb.last_op_result.lock().unwrap() = Some(op_res);
    assert_eq!(op_res, operation_result::K_OK, "Operation result must be kOK (0)");
    S_OK
}

#[test]
fn test_com_extract_sample_rpa() {
    let sample_path = "sample.rpa";
    if !std::path::Path::new(sample_path).exists() {
        println!("sample.rpa not found, skipping");
        return;
    }

    let file = File::open(sample_path).expect("Failed to open sample.rpa");
    let mut in_stream = MockInStream {
        vtable: &MOCK_IN_STREAM_VTABLE,
        file: Mutex::new(file),
    };

    // 1. Create COM InArchive
    let mut obj: *mut c_void = std::ptr::null_mut();
    let hr = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IINARCHIVE, &mut obj) };
    assert_eq!(hr, S_OK);

    let archive = obj as *mut IInArchive;

    // 2. Open archive via COM
    let hr_open = unsafe {
        ((*(*archive).vtable).Open)(
            archive as *mut c_void,
            &mut in_stream as *mut _ as *mut rpa::com::interfaces::IInStream,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(hr_open, S_OK);

    // 3. Verify item count
    let mut num_items: u32 = 0;
    let hr_count = unsafe { ((*(*archive).vtable).GetNumberOfItems)(archive as *mut c_void, &mut num_items) };
    assert_eq!(hr_count, S_OK);
    assert_eq!(num_items, 67);

    // 4. Verify KPID_POSITION is NOT set (crucial for 7-Zip not detecting split archives)
    let mut prop_pos = PROPVARIANT::new();
    let hr_pos = unsafe { ((*(*archive).vtable).GetProperty)(archive as *mut c_void, 0, KPID_POSITION, &mut prop_pos) };
    assert_eq!(hr_pos, S_OK);
    assert_eq!(prop_pos.vt, rpa::com::propvariant::VT_EMPTY);

    // 5. Test COM Extract for all files (numItems = u32::MAX, indices = null)
    let callback = MockExtractCallback {
        vtable: &MOCK_EXTRACT_CALLBACK_VTABLE,
        total: AtomicU64::new(0),
        completed: AtomicU64::new(0),
        last_op_result: Mutex::new(None),
        files: Mutex::new(HashMap::new()),
    };

    let hr_extract = unsafe {
        ((*(*archive).vtable).Extract)(
            archive as *mut c_void,
            std::ptr::null(),
            u32::MAX,
            0, // Extract mode
            &callback as *const _ as *mut IArchiveExtractCallback,
        )
    };
    assert_eq!(hr_extract, S_OK);

    // 6. Verify that SetTotal was called and matches sum of all items
    let total_size = callback.total.load(std::sync::atomic::Ordering::SeqCst);
    assert!(total_size > 0, "Total size must be greater than 0");
    assert_eq!(total_size, callback.completed.load(std::sync::atomic::Ordering::SeqCst));

    let extracted = callback.files.lock().unwrap();
    assert_eq!(extracted.len(), 67, "All 67 files must be extracted");

    // 7. Find font and check bytes
    for i in 0..67 {
        let mut path_val = PROPVARIANT::new();
        unsafe { ((*(*archive).vtable).GetProperty)(archive as *mut c_void, i, KPID_PATH, &mut path_val) };
        let path = path_val.get_str().unwrap_or_default();
        path_val.clear();

        if path == "0x52-URM/Caveat.ttf" {
            let font_bytes = extracted.get(&i).unwrap().lock().unwrap();
            assert_eq!(font_bytes.len(), 391068);
            assert_eq!(&font_bytes[0..4], &[0x00, 0x01, 0x00, 0x00]);
        } else if path == "0x52-URM/docs/Banner_URM.jpg" {
            let jpg_bytes = extracted.get(&i).unwrap().lock().unwrap();
            assert_eq!(jpg_bytes.len(), 80277);
            assert_eq!(&jpg_bytes[0..3], &[0xFF, 0xD8, 0xFF]);
        }
    }

    // 8. Test selective extraction of specific indices
    let sel_indices = [0u32, 5u32];
    let sel_callback = MockExtractCallback {
        vtable: &MOCK_EXTRACT_CALLBACK_VTABLE,
        total: AtomicU64::new(0),
        completed: AtomicU64::new(0),
        last_op_result: Mutex::new(None),
        files: Mutex::new(HashMap::new()),
    };
    let hr_sel = unsafe {
        ((*(*archive).vtable).Extract)(
            archive as *mut c_void,
            sel_indices.as_ptr(),
            sel_indices.len() as u32,
            0,
            &sel_callback as *const _ as *mut IArchiveExtractCallback,
        )
    };
    assert_eq!(hr_sel, S_OK);
    assert_eq!(sel_callback.files.lock().unwrap().len(), 2);

    // 9. Close and clean up
    unsafe {
        ((*(*archive).vtable).Close)(archive as *mut c_void);
        ((*(*archive).vtable).Release)(archive as *mut c_void);
    }
}

// =========================================================================
// IOutArchive COM Interface & UpdateItems Tests
// =========================================================================

#[test]
fn test_com_ioutarchive_lifecycle_and_properties() {
    let mut out_obj: *mut c_void = std::ptr::null_mut();

    // 1. CreateObject with IID_IOUTARCHIVE
    let hr = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IOUTARCHIVE, &mut out_obj) };
    assert_eq!(hr, S_OK);
    assert!(!out_obj.is_null());

    let out_archive = out_obj as *mut IOutArchive;

    // 2. QueryInterface for IID_IINARCHIVE
    let mut in_obj: *mut c_void = std::ptr::null_mut();
    let hr_qi = unsafe {
        ((*(*out_archive).vtable).QueryInterface)(
            out_archive as *mut c_void,
            &IID_IINARCHIVE,
            &mut in_obj,
        )
    };
    assert_eq!(hr_qi, S_OK);
    assert!(!in_obj.is_null());

    let in_archive = in_obj as *mut IInArchive;

    // 3. Verify KPID_READ_ONLY is false
    let mut val = PROPVARIANT::new();
    let hr_prop = unsafe {
        ((*(*in_archive).vtable).GetArchiveProperty)(
            in_archive as *mut c_void,
            KPID_READ_ONLY,
            &mut val,
        )
    };
    assert_eq!(hr_prop, S_OK);
    assert_eq!(val.vt, VT_BOOL);
    assert_eq!(unsafe { val.data.boolVal }, 0); // VARIANT_FALSE (read_only = false)
    val.clear();

    // 4. Test GetFileTimeType
    let mut time_type = u32::MAX;
    let hr_time = unsafe {
        ((*(*out_archive).vtable).GetFileTimeType)(out_archive as *mut c_void, &mut time_type)
    };
    assert_eq!(hr_time, S_OK);
    assert_eq!(time_type, 0); // kWindows

    // 5. Clean up both interfaces
    unsafe {
        ((*(*in_archive).vtable).Release)(in_archive as *mut c_void);
        ((*(*out_archive).vtable).Release)(out_archive as *mut c_void);
    }
}

// Mock sequential input stream for feeding data into UpdateItems
#[repr(C)]
struct MockSeqInStream {
    vtable: *const ISequentialInStreamVTable,
    data: Vec<u8>,
    pos: Mutex<usize>,
}

static MOCK_SEQ_IN_STREAM_VTABLE: ISequentialInStreamVTable = ISequentialInStreamVTable {
    QueryInterface: mock_seq_in_qi,
    AddRef: mock_seq_in_addref,
    Release: mock_seq_in_release,
    Read: mock_seq_in_read,
};

unsafe extern "system" fn mock_seq_in_qi(
    _this: *mut c_void,
    _riid: *const rpa::com::guid::GUID,
    _ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    E_NOINTERFACE
}
unsafe extern "system" fn mock_seq_in_addref(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_seq_in_release(this: *mut c_void) -> u32 {
    let _ = Box::from_raw(this as *mut MockSeqInStream);
    0
}
unsafe extern "system" fn mock_seq_in_read(
    this: *mut c_void,
    data: *mut c_void,
    size: u32,
    processed: *mut u32,
) -> rpa::com::propvariant::HRESULT {
    let s = &*(this as *mut MockSeqInStream);
    let mut pos = s.pos.lock().unwrap();
    let remaining = s.data.len().saturating_sub(*pos);
    let to_read = (size as usize).min(remaining);
    if to_read > 0 {
        std::ptr::copy_nonoverlapping(s.data[*pos..].as_ptr(), data as *mut u8, to_read);
        *pos += to_read;
    }
    if !processed.is_null() {
        *processed = to_read as u32;
    }
    S_OK
}

// Mock output stream supporting both ISequentialOutStream and IOutStream
#[repr(C)]
struct MockSeekableOutStream {
    seq_vtable: *const ISequentialOutStreamVTable,
    seek_vtable: *const IOutStreamVTable,
    cursor: Mutex<std::io::Cursor<Vec<u8>>>,
}

static MOCK_SEEK_OUT_SEQ_VTABLE: ISequentialOutStreamVTable = ISequentialOutStreamVTable {
    QueryInterface: mock_seek_out_qi,
    AddRef: mock_seek_out_addref,
    Release: mock_seek_out_release,
    Write: mock_seek_out_write,
};

static MOCK_SEEK_OUT_SEEK_VTABLE: IOutStreamVTable = IOutStreamVTable {
    QueryInterface: mock_seek_out_seek_qi,
    AddRef: mock_seek_out_addref,
    Release: mock_seek_out_release,
    Write: mock_seek_out_seek_write,
    Seek: mock_seek_out_seek,
    SetSize: mock_seek_out_setsize,
};

unsafe extern "system" fn mock_seek_out_qi(
    this: *mut c_void,
    riid: *const rpa::com::guid::GUID,
    ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    if riid.is_null() || ppv.is_null() {
        return E_FAIL;
    }
    if *riid == IID_ISEQUENTIALOUTSTREAM || *riid == IID_IUNKNOWN {
        *ppv = this;
        S_OK
    } else if *riid == IID_IOUTSTREAM {
        *ppv = (this as *mut u8).add(std::mem::size_of::<usize>()) as *mut c_void;
        S_OK
    } else {
        E_NOINTERFACE
    }
}
unsafe extern "system" fn mock_seek_out_seek_qi(
    this: *mut c_void,
    riid: *const rpa::com::guid::GUID,
    ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    let parent = (this as *mut u8).sub(std::mem::size_of::<usize>()) as *mut c_void;
    mock_seek_out_qi(parent, riid, ppv)
}
unsafe extern "system" fn mock_seek_out_addref(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_seek_out_release(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_seek_out_write(
    this: *mut c_void,
    data: *const c_void,
    size: u32,
    processed: *mut u32,
) -> rpa::com::propvariant::HRESULT {
    use std::io::Write;
    let s = &*(this as *mut MockSeekableOutStream);
    let mut c = s.cursor.lock().unwrap();
    let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
    match c.write(slice) {
        Ok(n) => {
            if !processed.is_null() {
                *processed = n as u32;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}
unsafe extern "system" fn mock_seek_out_seek_write(
    this: *mut c_void,
    data: *const c_void,
    size: u32,
    processed: *mut u32,
) -> rpa::com::propvariant::HRESULT {
    let parent = (this as *mut u8).sub(std::mem::size_of::<usize>()) as *mut c_void;
    mock_seek_out_write(parent, data, size, processed)
}
unsafe extern "system" fn mock_seek_out_seek(
    this: *mut c_void,
    offset: i64,
    origin: u32,
    new_pos: *mut u64,
) -> rpa::com::propvariant::HRESULT {
    use std::io::Seek;
    let parent = (this as *mut u8).sub(std::mem::size_of::<usize>()) as *mut MockSeekableOutStream;
    let s = &*parent;
    let mut c = s.cursor.lock().unwrap();
    let seek_from = match origin {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return E_FAIL,
    };
    match c.seek(seek_from) {
        Ok(pos) => {
            if !new_pos.is_null() {
                *new_pos = pos;
            }
            S_OK
        }
        Err(_) => E_FAIL,
    }
}
unsafe extern "system" fn mock_seek_out_setsize(
    _this: *mut c_void,
    _new_size: u64,
) -> rpa::com::propvariant::HRESULT {
    S_OK
}

// Mock update callback
struct MockUpdateItemDef {
    path: String,
    data: Vec<u8>,
}

#[repr(C)]
struct MockUpdateCallback {
    vtable: *const IArchiveUpdateCallbackVTable,
    items: Vec<MockUpdateItemDef>,
    results: Mutex<Vec<i32>>,
}

static MOCK_UPDATE_CALLBACK_VTABLE: IArchiveUpdateCallbackVTable = IArchiveUpdateCallbackVTable {
    QueryInterface: mock_update_qi,
    AddRef: mock_update_addref,
    Release: mock_update_release,
    SetTotal: mock_update_set_total,
    SetCompleted: mock_update_set_completed,
    GetUpdateItemInfo: mock_update_get_item_info,
    GetProperty: mock_update_get_prop,
    GetStream: mock_update_get_stream,
    SetOperationResult: mock_update_set_op_result,
};

unsafe extern "system" fn mock_update_qi(
    _this: *mut c_void,
    _riid: *const rpa::com::guid::GUID,
    _ppv: *mut *mut c_void,
) -> rpa::com::propvariant::HRESULT {
    E_NOINTERFACE
}
unsafe extern "system" fn mock_update_addref(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_update_release(_this: *mut c_void) -> u32 {
    1
}
unsafe extern "system" fn mock_update_set_total(
    _this: *mut c_void,
    _total: u64,
) -> rpa::com::propvariant::HRESULT {
    S_OK
}
unsafe extern "system" fn mock_update_set_completed(
    _this: *mut c_void,
    _complete: *const u64,
) -> rpa::com::propvariant::HRESULT {
    S_OK
}
unsafe extern "system" fn mock_update_get_item_info(
    this: *mut c_void,
    index: u32,
    newData: *mut i32,
    newProps: *mut i32,
    indexInArchive: *mut u32,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockUpdateCallback);
    if (index as usize) >= cb.items.len() {
        return E_FAIL;
    }
    if !newData.is_null() {
        *newData = 1;
    }
    if !newProps.is_null() {
        *newProps = 1;
    }
    if !indexInArchive.is_null() {
        *indexInArchive = u32::MAX;
    }
    S_OK
}
unsafe extern "system" fn mock_update_get_prop(
    this: *mut c_void,
    index: u32,
    propID: PROPID,
    value: *mut PROPVARIANT,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockUpdateCallback);
    let item = match cb.items.get(index as usize) {
        Some(it) => it,
        None => return E_FAIL,
    };
    let val = &mut *value;
    match propID {
        KPID_PATH => {
            val.set_str(&item.path);
            S_OK
        }
        KPID_IS_DIR => {
            val.set_bool(false);
            S_OK
        }
        KPID_SIZE => {
            val.set_u64(item.data.len() as u64);
            S_OK
        }
        _ => S_OK,
    }
}
unsafe extern "system" fn mock_update_get_stream(
    this: *mut c_void,
    index: u32,
    inStream: *mut *mut ISequentialInStream,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockUpdateCallback);
    let item = match cb.items.get(index as usize) {
        Some(it) => it,
        None => return E_FAIL,
    };
    let mock_stream = Box::new(MockSeqInStream {
        vtable: &MOCK_SEQ_IN_STREAM_VTABLE,
        data: item.data.clone(),
        pos: Mutex::new(0),
    });
    *inStream = Box::into_raw(mock_stream) as *mut ISequentialInStream;
    S_OK
}
unsafe extern "system" fn mock_update_set_op_result(
    this: *mut c_void,
    opRes: i32,
) -> rpa::com::propvariant::HRESULT {
    let cb = &*(this as *mut MockUpdateCallback);
    cb.results.lock().unwrap().push(opRes);
    S_OK
}

#[test]
fn test_com_update_items_create_and_reopen() {
    let mut out_obj: *mut c_void = std::ptr::null_mut();
    let hr = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IOUTARCHIVE, &mut out_obj) };
    assert_eq!(hr, S_OK);
    let out_archive = out_obj as *mut IOutArchive;

    let mock_out = MockSeekableOutStream {
        seq_vtable: &MOCK_SEEK_OUT_SEQ_VTABLE,
        seek_vtable: &MOCK_SEEK_OUT_SEEK_VTABLE,
        cursor: Mutex::new(std::io::Cursor::new(Vec::new())),
    };

    let items_def = vec![
        MockUpdateItemDef {
            path: "test/hello.txt".into(),
            data: b"Hello COM RPA Creation!".to_vec(),
        },
        MockUpdateItemDef {
            path: "images/icon.png".into(),
            data: vec![0x89, 0x50, 0x4E, 0x47, 0xAA, 0xBB, 0xCC],
        },
    ];

    let update_cb = MockUpdateCallback {
        vtable: &MOCK_UPDATE_CALLBACK_VTABLE,
        items: items_def,
        results: Mutex::new(Vec::new()),
    };

    // Call UpdateItems
    let hr_update = unsafe {
        ((*(*out_archive).vtable).UpdateItems)(
            out_archive as *mut c_void,
            &mock_out as *const _ as *mut ISequentialOutStream,
            2,
            &update_cb as *const _ as *mut IArchiveUpdateCallback,
        )
    };
    assert_eq!(hr_update, S_OK);
    assert_eq!(update_cb.results.lock().unwrap().len(), 2);
    for res in update_cb.results.lock().unwrap().iter() {
        assert_eq!(*res, 0); // kOK
    }

    // Release out_archive
    unsafe {
        ((*(*out_archive).vtable).Release)(out_archive as *mut c_void);
    }

    // Now verify the generated archive can be opened by IInArchive!
    let written_bytes = mock_out.cursor.into_inner().unwrap().into_inner();
    assert!(!written_bytes.is_empty());

    // Write bytes to a temp file and test opening with IInArchive
    let mut temp = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut temp, &written_bytes).unwrap();
    let file = File::open(temp.path()).unwrap();

    let mock_in = MockInStream {
        vtable: &MOCK_IN_STREAM_VTABLE,
        file: Mutex::new(file),
    };

    let mut in_obj: *mut c_void = std::ptr::null_mut();
    let hr_in = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IINARCHIVE, &mut in_obj) };
    assert_eq!(hr_in, S_OK);
    let in_arc = in_obj as *mut IInArchive;

    let hr_open = unsafe {
        ((*(*in_arc).vtable).Open)(
            in_arc as *mut c_void,
            &mock_in as *const _ as *mut rpa::com::interfaces::IInStream,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(hr_open, S_OK);

    let mut num_items = 0u32;
    unsafe {
        ((*(*in_arc).vtable).GetNumberOfItems)(in_arc as *mut c_void, &mut num_items);
    }
    assert_eq!(num_items, 2);

    // Extract all items and verify contents
    let extract_cb = MockExtractCallback {
        vtable: &MOCK_EXTRACT_CALLBACK_VTABLE,
        total: AtomicU64::new(0),
        completed: AtomicU64::new(0),
        last_op_result: Mutex::new(None),
        files: Mutex::new(HashMap::new()),
    };

    let hr_ext = unsafe {
        ((*(*in_arc).vtable).Extract)(
            in_arc as *mut c_void,
            std::ptr::null(),
            u32::MAX,
            0,
            &extract_cb as *const _ as *mut IArchiveExtractCallback,
        )
    };
    assert_eq!(hr_ext, S_OK);
    assert_eq!(extract_cb.files.lock().unwrap().len(), 2);

    let extracted = extract_cb.files.lock().unwrap();
    let mut extracted_names = Vec::new();
    for i in 0..2 {
        let mut val = PROPVARIANT::new();
        unsafe {
            ((*(*in_arc).vtable).GetProperty)(in_arc as *mut c_void, i, KPID_PATH, &mut val);
        }
        let name = val.get_str().unwrap();
        val.clear();
        extracted_names.push(name);
    }

    assert!(extracted_names.contains(&"test/hello.txt".to_string()));
    assert!(extracted_names.contains(&"images/icon.png".to_string()));

    // Verify file data
    for (idx, name) in extracted_names.iter().enumerate() {
        let data = extracted.get(&(idx as u32)).unwrap().lock().unwrap();
        if name == "test/hello.txt" {
            assert_eq!(&*data, b"Hello COM RPA Creation!");
        } else if name == "images/icon.png" {
            assert_eq!(&*data, &[0x89, 0x50, 0x4E, 0x47, 0xAA, 0xBB, 0xCC]);
        }
    }

    unsafe {
        ((*(*in_arc).vtable).Close)(in_arc as *mut c_void);
        ((*(*in_arc).vtable).Release)(in_arc as *mut c_void);
    }
}

#[test]
fn test_com_set_properties_rpa2() {
    let mut obj: *mut c_void = std::ptr::null_mut();
    let hr = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IOUTARCHIVE, &mut obj) };
    assert_eq!(hr, S_OK);
    let out_archive = obj as *mut IOutArchive;

    // Query ISetProperties
    let mut set_props_ptr: *mut c_void = std::ptr::null_mut();
    let hr_qi = unsafe {
        ((*(*out_archive).vtable).QueryInterface)(
            out_archive as *mut c_void,
            &IID_ISETPROPERTIES,
            &mut set_props_ptr,
        )
    };
    assert_eq!(hr_qi, S_OK);
    assert!(!set_props_ptr.is_null());
    let set_props = set_props_ptr as *mut ISetProperties;

    // Set v=2
    let name_v: Vec<u16> = "v\0".encode_utf16().collect();
    let names = [name_v.as_ptr()];
    let mut val = PROPVARIANT::new();
    val.set_u64(2);
    let values = [val];

    let hr_set = unsafe {
        ((*(*set_props).vtable).SetProperties)(
            set_props as *mut c_void,
            names.as_ptr(),
            values.as_ptr(),
            1,
        )
    };
    assert_eq!(hr_set, S_OK);

    // Now call UpdateItems to write an archive
    let mock_out = MockSeekableOutStream {
        seq_vtable: &MOCK_SEEK_OUT_SEQ_VTABLE,
        seek_vtable: &MOCK_SEEK_OUT_SEEK_VTABLE,
        cursor: Mutex::new(std::io::Cursor::new(Vec::new())),
    };

    let items_def = vec![MockUpdateItemDef {
        path: "script.rpy".into(),
        data: b"label start:\n    return\n".to_vec(),
    }];

    let update_cb = MockUpdateCallback {
        vtable: &MOCK_UPDATE_CALLBACK_VTABLE,
        items: items_def,
        results: Mutex::new(Vec::new()),
    };

    let hr_update = unsafe {
        ((*(*out_archive).vtable).UpdateItems)(
            out_archive as *mut c_void,
            &mock_out as *const _ as *mut ISequentialOutStream,
            1,
            &update_cb as *const _ as *mut IArchiveUpdateCallback,
        )
    };
    assert_eq!(hr_update, S_OK);

    unsafe {
        ((*(*set_props).vtable).Release)(set_props as *mut c_void);
        ((*(*out_archive).vtable).Release)(out_archive as *mut c_void);
    }

    let written = mock_out.cursor.into_inner().unwrap().into_inner();
    let header_str = String::from_utf8_lossy(&written[..25]);
    assert!(header_str.starts_with("RPA-2.0 "), "Expected RPA-2.0 header, got: {}", header_str);
}

#[test]
fn test_com_set_properties_custom_key() {
    let mut obj: *mut c_void = std::ptr::null_mut();
    let hr = unsafe { CreateObject(&RPA_HANDLER_GUID, &IID_IOUTARCHIVE, &mut obj) };
    assert_eq!(hr, S_OK);
    let out_archive = obj as *mut IOutArchive;

    // Query ISetProperties
    let mut set_props_ptr: *mut c_void = std::ptr::null_mut();
    let hr_qi = unsafe {
        ((*(*out_archive).vtable).QueryInterface)(
            out_archive as *mut c_void,
            &IID_ISETPROPERTIES,
            &mut set_props_ptr,
        )
    };
    assert_eq!(hr_qi, S_OK);
    let set_props = set_props_ptr as *mut ISetProperties;

    // Set v=3.0, key=0xdeadbeef
    let name_v: Vec<u16> = "version\0".encode_utf16().collect();
    let name_k: Vec<u16> = "key\0".encode_utf16().collect();
    let names = [name_v.as_ptr(), name_k.as_ptr()];

    let mut val_v = PROPVARIANT::new();
    val_v.set_str("3.0");
    let mut val_k = PROPVARIANT::new();
    val_k.set_str("0xdeadbeef");
    let values = [val_v, val_k];

    let hr_set = unsafe {
        ((*(*set_props).vtable).SetProperties)(
            set_props as *mut c_void,
            names.as_ptr(),
            values.as_ptr(),
            2,
        )
    };
    assert_eq!(hr_set, S_OK);

    let mock_out = MockSeekableOutStream {
        seq_vtable: &MOCK_SEEK_OUT_SEQ_VTABLE,
        seek_vtable: &MOCK_SEEK_OUT_SEEK_VTABLE,
        cursor: Mutex::new(std::io::Cursor::new(Vec::new())),
    };

    let items_def = vec![MockUpdateItemDef {
        path: "script.rpy".into(),
        data: b"label start:\n    return\n".to_vec(),
    }];

    let update_cb = MockUpdateCallback {
        vtable: &MOCK_UPDATE_CALLBACK_VTABLE,
        items: items_def,
        results: Mutex::new(Vec::new()),
    };

    let hr_update = unsafe {
        ((*(*out_archive).vtable).UpdateItems)(
            out_archive as *mut c_void,
            &mock_out as *const _ as *mut ISequentialOutStream,
            1,
            &update_cb as *const _ as *mut IArchiveUpdateCallback,
        )
    };
    assert_eq!(hr_update, S_OK);

    unsafe {
        ((*(*set_props).vtable).Release)(set_props as *mut c_void);
        ((*(*out_archive).vtable).Release)(out_archive as *mut c_void);
    }

    let written = mock_out.cursor.into_inner().unwrap().into_inner();
    let header_str = String::from_utf8_lossy(&written[..34]);
    assert!(header_str.starts_with("RPA-3.0 "), "Expected RPA-3.0 header, got: {}", header_str);
    assert!(header_str.contains("deadbeef"), "Expected deadbeef key in header: {}", header_str);
}

