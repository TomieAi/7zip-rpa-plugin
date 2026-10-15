use std::ffi::c_void;
use std::io::{Error, ErrorKind, Read, Result as IoResult, Seek, SeekFrom, Write};

use crate::com::interfaces::{IInStream, IOutStream, ISequentialInStream, ISequentialOutStream};
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
            Err(Error::other(format!(
                "IInStream::Read failed with HRESULT 0x{:08X}",
                hr
            )))
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
            Err(Error::other(format!(
                "IInStream::Seek failed with HRESULT 0x{:08X}",
                hr
            )))
        }
    }
}

pub struct SequentialInStreamReader {
    stream: *mut ISequentialInStream,
}

impl SequentialInStreamReader {
    /// Create a new SequentialInStreamReader wrapper around an ISequentialInStream pointer.
    ///
    /// # Safety
    /// The caller must ensure `stream` is non-null and valid for the lifetime of this reader.
    pub unsafe fn new(stream: *mut ISequentialInStream) -> Self {
        Self { stream }
    }
}

impl Read for SequentialInStreamReader {
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
            Err(Error::other(format!(
                "ISequentialInStream::Read failed with HRESULT 0x{:08X}",
                hr
            )))
        }
    }
}

pub struct StreamWriter {
    seq_stream: *mut ISequentialOutStream,
    seek_stream: Option<*mut IOutStream>,
    pos: u64,
}

impl StreamWriter {
    /// Create a new StreamWriter wrapper around output streams.
    ///
    /// # Safety
    /// The caller must ensure `seq_stream` (and optional `seek_stream`) are non-null and valid.
    pub unsafe fn new(
        seq_stream: *mut ISequentialOutStream,
        seek_stream: Option<*mut IOutStream>,
    ) -> Self {
        Self {
            seq_stream,
            seek_stream,
            pos: 0,
        }
    }

    pub fn can_seek(&self) -> bool {
        self.seek_stream.is_some()
    }

    pub fn position(&self) -> u64 {
        self.pos
    }
}

impl Write for StreamWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut processed: u32 = 0;
        let hr = unsafe {
            ((*(*self.seq_stream).vtable).Write)(
                self.seq_stream as *mut c_void,
                buf.as_ptr() as *const c_void,
                buf.len() as u32,
                &mut processed,
            )
        };

        if hr == S_OK {
            self.pos += processed as u64;
            Ok(processed as usize)
        } else {
            Err(Error::other(format!(
                "ISequentialOutStream::Write failed with HRESULT 0x{:08X}",
                hr
            )))
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

impl Seek for StreamWriter {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        if let Some(seek_ptr) = self.seek_stream {
            let (offset, origin) = match pos {
                SeekFrom::Start(n) => (n as i64, 0u32),
                SeekFrom::Current(n) => (n, 1u32),
                SeekFrom::End(n) => (n, 2u32),
            };

            let mut new_pos: u64 = 0;
            let hr = unsafe {
                ((*(*seek_ptr).vtable).Seek)(
                    seek_ptr as *mut c_void,
                    offset,
                    origin,
                    &mut new_pos,
                )
            };

            if hr == S_OK {
                self.pos = new_pos;
                Ok(new_pos)
            } else {
                Err(Error::other(format!(
                    "IOutStream::Seek failed with HRESULT 0x{:08X}",
                    hr
                )))
            }
        } else {
            Err(Error::new(
                ErrorKind::Unsupported,
                "Underlying output stream does not support seeking",
            ))
        }
    }
}
