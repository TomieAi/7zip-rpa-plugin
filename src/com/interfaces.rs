#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::c_void;
use crate::com::guid::GUID;
use crate::com::propvariant::{HRESULT, PROPVARIANT, VARTYPE};
use crate::models::props::PROPID;

// =========================================================================
// IUnknown
// =========================================================================
#[repr(C)]
pub struct IUnknownVTable {
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
}

// =========================================================================
// ISequentialInStream
// =========================================================================
#[repr(C)]
pub struct ISequentialInStreamVTable {
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Read: unsafe extern "system" fn(
        this: *mut c_void,
        data: *mut c_void,
        size: u32,
        processedSize: *mut u32,
    ) -> HRESULT,
}

// =========================================================================
// ISequentialOutStream
// =========================================================================
#[repr(C)]
pub struct ISequentialOutStreamVTable {
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Write: unsafe extern "system" fn(
        this: *mut c_void,
        data: *const c_void,
        size: u32,
        processedSize: *mut u32,
    ) -> HRESULT,
}

#[repr(C)]
pub struct ISequentialOutStream {
    pub vtable: *const ISequentialOutStreamVTable,
}

// =========================================================================
// IInStream
// =========================================================================
#[repr(C)]
pub struct IInStreamVTable {
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Read: unsafe extern "system" fn(
        this: *mut c_void,
        data: *mut c_void,
        size: u32,
        processedSize: *mut u32,
    ) -> HRESULT,
    pub Seek: unsafe extern "system" fn(
        this: *mut c_void,
        offset: i64,
        seekOrigin: u32,
        newPosition: *mut u64,
    ) -> HRESULT,
}

#[repr(C)]
pub struct IInStream {
    pub vtable: *const IInStreamVTable,
}

// =========================================================================
// IArchiveOpenCallback
// =========================================================================
#[repr(C)]
pub struct IArchiveOpenCallbackVTable {
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub SetTotal: unsafe extern "system" fn(
        this: *mut c_void,
        files: *const u64,
        bytes: *const u64,
    ) -> HRESULT,
    pub SetCompleted: unsafe extern "system" fn(
        this: *mut c_void,
        files: *const u64,
        bytes: *const u64,
    ) -> HRESULT,
}

#[repr(C)]
pub struct IArchiveOpenCallback {
    pub vtable: *const IArchiveOpenCallbackVTable,
}

// =========================================================================
// IArchiveExtractCallback
// =========================================================================
#[repr(C)]
pub struct IArchiveExtractCallbackVTable {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    // IProgress
    pub SetTotal: unsafe extern "system" fn(this: *mut c_void, total: u64) -> HRESULT,
    pub SetCompleted: unsafe extern "system" fn(this: *mut c_void, completeValue: *const u64) -> HRESULT,
    // IArchiveExtractCallback
    pub GetStream: unsafe extern "system" fn(
        this: *mut c_void,
        index: u32,
        outStream: *mut *mut ISequentialOutStream,
        askExtractMode: i32,
    ) -> HRESULT,
    pub PrepareOperation: unsafe extern "system" fn(this: *mut c_void, askExtractMode: i32) -> HRESULT,
    pub SetOperationResult: unsafe extern "system" fn(this: *mut c_void, opRes: i32) -> HRESULT,
}

#[repr(C)]
pub struct IArchiveExtractCallback {
    pub vtable: *const IArchiveExtractCallbackVTable,
}

// =========================================================================
// IInArchive
// =========================================================================
#[repr(C)]
pub struct IInArchiveVTable {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    // IInArchive
    pub Open: unsafe extern "system" fn(
        this: *mut c_void,
        stream: *mut IInStream,
        maxCheckStartPosition: *const u64,
        openCallback: *mut IArchiveOpenCallback,
    ) -> HRESULT,
    pub Close: unsafe extern "system" fn(this: *mut c_void) -> HRESULT,
    pub GetNumberOfItems: unsafe extern "system" fn(this: *mut c_void, numItems: *mut u32) -> HRESULT,
    pub GetProperty: unsafe extern "system" fn(
        this: *mut c_void,
        index: u32,
        propID: PROPID,
        value: *mut PROPVARIANT,
    ) -> HRESULT,
    pub Extract: unsafe extern "system" fn(
        this: *mut c_void,
        indices: *const u32,
        numItems: u32,
        testMode: i32,
        extractCallback: *mut IArchiveExtractCallback,
    ) -> HRESULT,
    pub GetArchiveProperty: unsafe extern "system" fn(
        this: *mut c_void,
        propID: PROPID,
        value: *mut PROPVARIANT,
    ) -> HRESULT,
    pub GetNumberOfProperties: unsafe extern "system" fn(this: *mut c_void, numProps: *mut u32) -> HRESULT,
    pub GetPropertyInfo: unsafe extern "system" fn(
        this: *mut c_void,
        index: u32,
        name: *mut *mut u16,
        propID: *mut PROPID,
        varType: *mut VARTYPE,
    ) -> HRESULT,
    pub GetNumberOfArchiveProperties: unsafe extern "system" fn(
        this: *mut c_void,
        numProps: *mut u32,
    ) -> HRESULT,
    pub GetArchivePropertyInfo: unsafe extern "system" fn(
        this: *mut c_void,
        index: u32,
        name: *mut *mut u16,
        propID: *mut PROPID,
        varType: *mut VARTYPE,
    ) -> HRESULT,
}

#[repr(C)]
pub struct IInArchive {
    pub vtable: *const IInArchiveVTable,
}
