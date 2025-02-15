use core::num::NonZeroUsize;

use crate::Z_MAX_MTU;
use heapless::Vec;
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy)]
pub struct DidntWrite;

impl core::fmt::Display for DidntWrite {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Didn't write")
    }
}

#[derive(Error, Debug, Clone, Copy)]
pub struct DidntRead;

impl core::fmt::Display for DidntRead {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Didn't read")
    }
}

pub trait Writer {
    fn write(&mut self, bytes: &[u8]) -> Result<(), DidntWrite>;
    fn write_exact(&mut self, bytes: &[u8]) -> Result<(), DidntWrite>;

    fn write_u8(&mut self, byte: u8) -> Result<(), DidntWrite> {
        self.write_exact(core::slice::from_ref(&byte))
    }
}

pub trait Reader {
    fn read(&mut self, into: &mut [u8]) -> Result<NonZeroUsize, DidntRead>;
    fn read_exact(&mut self, into: &mut [u8]) -> Result<(), DidntRead>;

    fn read_u8(&mut self) -> Result<u8, DidntRead> {
        let mut byte = 0;
        let read = self.read(core::slice::from_mut(&mut byte))?;
        if read.get() == 1 {
            Ok(byte)
        } else {
            Err(DidntRead)
        }
    }

    fn read_slice_in_place(&mut self, _len: usize) -> Result<&[u8], DidntRead> {
        unimplemented!("read_slice_in_place")
    }
}

pub struct ZVec {
    vec: Vec<u8, Z_MAX_MTU>,
}

impl ZVec {
    pub fn new() -> Self {
        ZVec { vec: Vec::new() }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.vec.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.vec.as_mut_slice()
    }

    pub fn clear(&mut self) {
        self.vec.clear()
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub unsafe fn set_len(&mut self, len: usize) {
        self.vec.set_len(len)
    }

    pub fn extract_slice<'a>(&'a mut self, len: usize) -> Result<ZVecSlice<'a>, DidntRead> {
        if len > self.vec.capacity() {
            return Err(DidntRead);
        }
        Ok(ZVecSlice::new(self, len))
    }
}

pub struct ZVecSlice<'a> {
    vec: &'a mut ZVec,
    len: usize,
    idx: usize,
}

impl<'a> ZVecSlice<'a> {
    fn new(v: &'a mut ZVec, len: usize) -> Self {
        let o_len = v.len();

        unsafe {
            v.set_len(len);
        }

        ZVecSlice {
            vec: v,
            len: o_len,
            idx: 0,
        }
    }

    pub fn truncate(&mut self, len: usize) {
        if len > self.vec.len() {
            return;
        }

        unsafe {
            self.vec.set_len(len);
        }
    }
}

impl Drop for ZVecSlice<'_> {
    fn drop(&mut self) {
        unsafe {
            self.vec.set_len(self.len);
        }
    }
}

impl AsMut<[u8]> for ZVecSlice<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.vec.as_mut_slice()
    }
}

impl AsRef<[u8]> for ZVecSlice<'_> {
    fn as_ref(&self) -> &[u8] {
        self.vec.as_slice()
    }
}

impl Writer for ZVec {
    fn write(&mut self, bytes: &[u8]) -> Result<(), DidntWrite> {
        self.vec.extend_from_slice(bytes).map_err(|_| DidntWrite)
    }

    fn write_exact(&mut self, bytes: &[u8]) -> Result<(), DidntWrite> {
        self.vec.extend_from_slice(bytes).map_err(|_| DidntWrite)
    }
}

impl<'a> Reader for ZVecSlice<'a> {
    fn read(&mut self, into: &mut [u8]) -> Result<NonZeroUsize, DidntRead> {
        let len = into.len();
        let remaining = self.vec.len() - self.idx;
        if remaining == 0 {
            return Err(DidntRead);
        }
        let to_read = core::cmp::min(len, remaining);
        into[..to_read].copy_from_slice(&self.vec.as_slice()[self.idx..self.idx + to_read]);
        self.idx += to_read;
        Ok(NonZeroUsize::new(to_read).unwrap())
    }

    fn read_exact(&mut self, into: &mut [u8]) -> Result<(), DidntRead> {
        let len = into.len();
        let remaining = self.vec.len() - self.idx;
        if remaining == 0 {
            return Err(DidntRead);
        }
        if len > remaining {
            return Err(DidntRead);
        }
        into.copy_from_slice(&self.vec.as_slice()[self.idx..self.idx + len]);
        self.idx += len;
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, DidntRead> {
        let mut byte = 0;
        let read = self.read(core::slice::from_mut(&mut byte))?;
        if read.get() == 1 {
            Ok(byte)
        } else {
            Err(DidntRead)
        }
    }

    fn read_slice_in_place(&mut self, len: usize) -> Result<&[u8], DidntRead> {
        let remaining = self.vec.len() - self.idx;
        if remaining == 0 {
            return Err(DidntRead);
        }
        if len > remaining {
            return Err(DidntRead);
        }
        let slice = &self.vec.as_slice()[self.idx..self.idx + len];
        self.idx += len;
        Ok(slice)
    }
}
