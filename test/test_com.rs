use std::ffi::c_void;

use rpa::com::guid::{IID_IINARCHIVE, IID_IUNKNOWN, RPA_HANDLER_GUID};
use rpa::com::interfaces::IInArchive;
use rpa::com::propvariant::{PROPVARIANT, S_OK, VT_BOOL, VT_BSTR, VT_UI4};
use rpa::models::props::handler_prop;
use rpa::{CreateObject, GetHandlerProperty, GetHandlerProperty2, GetNumberOfFormats};

#[test]
fn test_get_number_of_formats() {
    let mut num_formats: u32 = 0;
    let hr = unsafe { GetNumberOfFormats(&mut num_formats) };
    assert_eq!(hr, S_OK);
    assert_eq!(num_formats, 1);
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
