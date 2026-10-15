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
