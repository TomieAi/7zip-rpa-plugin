use std::ffi::c_void;
use std::io::{Error, ErrorKind, Read, Result as IoResult, Seek, SeekFrom};

use crate::com::interfaces::IInStream;
use crate::com::propvariant::S_OK;

pub struct StreamReader {
    stream: *mut IInStream,
}

impl StreamReader {
    /// Create a new StreamReader wrapper around a raw 7-Zip IInStream pointer.
    ///
    /// # Safety
    /// The caller must ensure stream is non-null and valid for the lifetime of StreamReader.
    pub unsafe fn new(stream: *mut IInStream) -> Self {
        Self { stream }
    }

    pub fn raw(&self) -> *mut IInStream {
        self.stream
    }
}

impl Read for StreamReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut processed: u32 = 0;
        let hr = unsafe {
            ((*(*self.stream).vtable).Read)(
                self.stream as *mut c_void,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
                &mut processed,
            )
        };

        if hr == S_OK {
            Ok(processed as usize)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("IInStream::Read failed with HRESULT 0x{:08X}", hr),
            ))
        }
    }
}

impl Seek for StreamReader {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        // 7-Zip seek origin constants:
        // STREAM_SEEK_SET = 0
        // STREAM_SEEK_CUR = 1
        // STREAM_SEEK_END = 2
        let (offset, origin) = match pos {
            SeekFrom::Start(n) => (n as i64, 0u32),
            SeekFrom::Current(n) => (n, 1u32),
            SeekFrom::End(n) => (n, 2u32),
        };

        let mut new_position: u64 = 0;
        let hr = unsafe {
            ((*(*self.stream).vtable).Seek)(
                self.stream as *mut c_void,
                offset,
                origin,
                &mut new_position,
            )
        };

        if hr == S_OK {
            Ok(new_position)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("IInStream::Seek failed with HRESULT 0x{:08X}", hr),
            ))
        }
    }
}
