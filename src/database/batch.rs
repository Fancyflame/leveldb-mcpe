//! Module providing write batches

use super::error::Error;
use super::Database;
use database::key::Key;
use leveldb_mcpe_sys::*;
use libc::{c_char, c_void, size_t};
use options::{c_writeoptions, WriteOptions};
use std::marker::PhantomData;
use std::ptr;
use std::slice;

#[allow(missing_docs)]
struct RawWritebatch {
    ptr: *mut leveldb_writebatch_t,
}

impl Drop for RawWritebatch {
    fn drop(&mut self) {
        unsafe {
            leveldb_writebatch_destroy(self.ptr);
        }
    }
}

#[allow(missing_docs)]
pub struct Writebatch<'a, K: Key<'a>> {
    #[allow(dead_code)]
    writebatch: RawWritebatch,
    marker: PhantomData<&'a K>,
}

/// Batch access to the database
pub trait Batch<'a, K: Key<'a>> {
    /// Write a batch to the database, ensuring success for all items or an error
    fn write(&self, options: WriteOptions, batch: &Writebatch<'a, K>) -> Result<(), Error>;
}

impl<'a, K: Key<'a>> Batch<'a, K> for Database<'a, K> {
    fn write(&self, options: WriteOptions, batch: &Writebatch<'a, K>) -> Result<(), Error> {
        unsafe {
            let mut error = ptr::null_mut();
            let c_writeoptions = c_writeoptions(options);

            leveldb_write(
                self.database.ptr,
                c_writeoptions,
                batch.writebatch.ptr,
                &mut error,
            );
            leveldb_writeoptions_destroy(c_writeoptions);

            if error == ptr::null_mut() {
                Ok(())
            } else {
                Err(Error::new_from_char(error))
            }
        }
    }
}

impl<'a, K: Key<'a>> Writebatch<'a, K> {
    /// Create a new writebatch
    pub fn new() -> Writebatch<'a, K> {
        let ptr = unsafe { leveldb_writebatch_create() };
        let raw = RawWritebatch { ptr };
        Writebatch {
            writebatch: raw,
            marker: PhantomData,
        }
    }

    /// Clear the writebatch
    pub fn clear(&mut self) {
        unsafe { leveldb_writebatch_clear(self.writebatch.ptr) };
    }

    /// Batch a put operation
    pub fn put(&mut self, key: K, value: &[u8]) {
        unsafe {
            let k = key.as_ref();
            leveldb_writebatch_put(
                self.writebatch.ptr,
                k.as_ptr() as *mut c_char,
                k.len() as size_t,
                value.as_ptr() as *mut c_char,
                value.len() as size_t,
            );
        }
    }

    /// Batch a delete operation
    pub fn delete(&mut self, key: K) {
        unsafe {
            let k = key.as_ref();
            leveldb_writebatch_delete(
                self.writebatch.ptr,
                k.as_ptr() as *mut c_char,
                k.len() as size_t,
            );
        }
    }

    /// Iterate over the writebatch, returning the resulting iterator
    pub fn iterate<T: WritebatchIterator<'a, K = K>>(&mut self, iterator: Box<T>) -> Box<T> {
        unsafe {
            let iter = Box::into_raw(iterator);
            leveldb_writebatch_iterate(
                self.writebatch.ptr,
                iter as *mut c_void,
                put_callback::<K, T>,
                deleted_callback::<K, T>,
            );
            Box::from_raw(iter)
        }
    }
}

/// A trait for iterators to iterate over written batches and check their validity.
pub trait WritebatchIterator<'a> {
    /// The database Key<'a> type this iterates over
    type K: Key<'a>;

    /// Callback for put items
    fn put(&mut self, key: Self::K, value: &[u8]);

    /// Callback for deleted items
    fn deleted(&mut self, key: Self::K);
}

extern "C" fn put_callback<'a, K: Key<'a>, T: WritebatchIterator<'a, K = K>>(
    state: *mut c_void,
    key: *const c_char,
    keylen: size_t,
    val: *const c_char,
    vallen: size_t,
) {
    unsafe {
        let iter: &mut T = &mut *(state as *mut T);
        let key_slice = slice::from_raw_parts::<u8>(key as *const u8, keylen as usize);
        let val_slice = slice::from_raw_parts::<u8>(val as *const u8, vallen as usize);
        let k = <<T as WritebatchIterator>::K>::from(key_slice);
        iter.put(k, val_slice);
    }
}

extern "C" fn deleted_callback<'a, K: Key<'a>, T: WritebatchIterator<'a, K = K>>(
    state: *mut c_void,
    key: *const c_char,
    keylen: size_t,
) {
    unsafe {
        let iter: &mut T = &mut *(state as *mut T);
        let key_slice = slice::from_raw_parts::<u8>(key as *const u8, keylen as usize);
        let k = <<T as WritebatchIterator>::K>::from(key_slice);
        iter.deleted(k);
    }
}
