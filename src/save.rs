use std::collections::HashMap;
use std::io;
use std::iter::{Extend, FromIterator};
use std::mem;
use std::ptr::null_mut;

use bincode;
use libc::c_void;

use bw;
use send_pointer::SendPtr;

quick_error! {
    #[derive(Debug)]
    pub enum SaveError {
        BwIo {
            display("Broodwar I/O error")
        }
        Serialize(err: bincode::Error) {
            display("Serialization error: {}", err)
            from()
        }
        Io(err: io::Error) {
            display("I/O error: {}", err)
            from()
        }
        SizeLimit(amt: u64) {
            display("Too large chunk: {}", amt)
        }
        InvalidPointer {
            display("Internal error: Invalid pointer")
        }
        InvalidGrpPointer {
            display("Internal error: Invalid grp pointer")
        }
        InvalidRemapPalette {
            display("Internal error: Invalid remap palette")
        }
        InvalidUnitAi(ai: u8) {
            display("Internal error: Invalid unit ai type {}", ai)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum LoadError {
        BwIo {
            display("Broodwar I/O error")
        }
        Serialize(err: bincode::Error) {
            display("Deserialization error: {}", err)
            from()
        }
        SizeLimit {
            display("Too large chunk")
        }
        WrongMagic(m: u16) {
            display("Incorrect magic: 0x{:x}", m)
        }
        Version(ver: u32) {
            display("Unsupported (newer?) version {}", ver)
        }
        Corrupted(info: String) {
            display("Invalid save data ({})", info)
        }
    }
}

pub unsafe fn fread_num<T>(file: *mut c_void) -> Result<T, LoadError> {
    let mut val: T = mem::uninitialized();
    let ok = bw::fread(&mut val as *mut T as *mut c_void, mem::size_of::<T>() as u32, 1, file);
    if ok != 1 {
        Err(LoadError::BwIo)
    } else {
        Ok(val)
    }
}

pub unsafe fn fread(file: *mut c_void, size: u32) -> Result<Vec<u8>, LoadError> {
    let mut buf = Vec::with_capacity(size as usize);
    let ok = bw::fread(buf.as_mut_ptr() as *mut c_void, size, 1, file);
    if ok != 1 {
        Err(LoadError::BwIo)
    } else {
        buf.set_len(size as usize);
        Ok(buf)
    }
}

pub unsafe fn fwrite_num<T>(file: *mut c_void, value: T) -> Result<(), SaveError> {
    let amount =
        bw::fwrite(&value as *const T as *const c_void, mem::size_of::<T>() as u32, 1, file);
    if amount != 1 {
        Err(SaveError::BwIo)
    } else {
        Ok(())
    }
}

pub unsafe fn fwrite(file: *mut c_void, buf: &[u8]) -> Result<(), SaveError> {
    let amount = bw::fwrite(buf.as_ptr() as *const c_void, buf.len() as u32, 1, file);
    if amount != 1 {
        Err(SaveError::BwIo)
    } else {
        Ok(())
    }
}

pub unsafe fn print_text(msg: &str) {
    let mut buf: Vec<u8> = msg.as_bytes().into();
    buf.push(0);
    bw::print_text(buf.as_ptr(), 0, 8);
}

pub struct SaveMapping<T>(pub HashMap<SendPtr<T>, u32>);

impl<T> SaveMapping<T> {
    pub fn new() -> SaveMapping<T> {
        SaveMapping(HashMap::new())
    }

    pub fn id(&self, val: *mut T) -> Result<u32, SaveError> {
        if val == null_mut() {
            Ok(0)
        } else {
            self.0.get(&val.into()).cloned().ok_or(SaveError::InvalidPointer)
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T> FromIterator<(*mut T, u32)> for SaveMapping<T> {
    fn from_iter<I: IntoIterator<Item=(*mut T, u32)>>(iter: I) -> SaveMapping<T> {
        SaveMapping(iter.into_iter().map(|(x, y)| (x.into(), y)).collect())
    }
}

pub struct LoadMapping<T>(pub Vec<SendPtr<T>>);

impl<T> LoadMapping<T> {
    pub fn new() -> LoadMapping<T> {
        LoadMapping(Vec::new())
    }

    pub fn pointer(&self, id: u32) -> Result<*mut T, LoadError> {
        if id == 0 {
            Ok(null_mut())
        } else {
            self.0.get(id as usize - 1).map(|&SendPtr(x)| x).ok_or_else(|| {
                LoadError::Corrupted(format!("Invalid id 0x{:x}", id))
            })
        }
    }
}

impl<T> Extend<*mut T> for LoadMapping<T> {
    fn extend<I: IntoIterator<Item=*mut T>>(&mut self, iter: I) {
        self.0.extend(iter.into_iter().map(|x| SendPtr(x)))
    }
}

impl<T> Default for LoadMapping<T> {
    fn default() -> LoadMapping<T> {
        LoadMapping(vec![])
    }
}
