#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

pub struct Byte4 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte4 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte4 {
    pub fn len(&self) -> usize {
        4
    }
}

impl Byte4 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte8 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte8 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte8 {
    pub fn len(&self) -> usize {
        8
    }
}

impl Byte8 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte16 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte16 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte16 {
    pub fn len(&self) -> usize {
        16
    }
}

impl Byte16 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte20 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte20 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte20 {
    pub fn len(&self) -> usize {
        20
    }
}

impl Byte20 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte32 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte32 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte32 {
    pub fn len(&self) -> usize {
        32
    }
}

impl Byte32 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte65 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte65 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte65 {
    pub fn len(&self) -> usize {
        65
    }
}

impl Byte65 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte97 {
    pub cursor: Cursor,
}

impl From<Cursor> for Byte97 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte97 {
    pub fn len(&self) -> usize {
        97
    }
}

impl Byte97 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Uint16 {
    pub cursor: Cursor,
}

impl From<Cursor> for Uint16 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Uint16 {
    pub fn len(&self) -> usize {
        2
    }
}

impl Uint16 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Uint32 {
    pub cursor: Cursor,
}

impl From<Cursor> for Uint32 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Uint32 {
    pub fn len(&self) -> usize {
        4
    }
}

impl Uint32 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Uint64 {
    pub cursor: Cursor,
}

impl From<Cursor> for Uint64 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Uint64 {
    pub fn len(&self) -> usize {
        8
    }
}

impl Uint64 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Uint128 {
    pub cursor: Cursor,
}

impl From<Cursor> for Uint128 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Uint128 {
    pub fn len(&self) -> usize {
        16
    }
}

impl Uint128 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Bytes {
    pub cursor: Cursor,
}

impl From<Cursor> for Bytes {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Bytes {
    pub fn len(&self) -> usize {
        self.cursor.fixvec_length()
    }
}

impl Bytes {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.fixvec_slice_by_index(1, index).unwrap();
        cur.into()
    }
}
// warning: BytesOpt not implemented for Rust
pub struct BytesOpt {
    pub cursor: Cursor,
}
impl From<Cursor> for BytesOpt {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}
// warning: Byte32Opt not implemented for Rust
pub struct Byte32Opt {
    pub cursor: Cursor,
}
impl From<Cursor> for Byte32Opt {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

pub struct Identity {
    pub cursor: Cursor,
}

impl From<Cursor> for Identity {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Identity {
    pub fn len(&self) -> usize {
        20
    }
}

impl Identity {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}
// warning: IdentityOpt not implemented for Rust
pub struct IdentityOpt {
    pub cursor: Cursor,
}
impl From<Cursor> for IdentityOpt {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}
