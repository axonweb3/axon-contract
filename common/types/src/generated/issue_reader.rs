
#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct IssueCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for IssueCellData {
    fn from(cursor: Cursor) -> Self {
        IssueCellData { cursor }
    }
}

impl IssueCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.slice_by_offset(0, 1).unwrap();
        cur.into()
    }
}

impl IssueCellData {
    pub fn current_supply(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(1, 16).unwrap();
        cur.into()
    }
}

impl IssueCellData {
    pub fn max_suppley(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(17, 16).unwrap();
        cur.into()
    }
}

impl IssueCellData {
    pub fn xudt_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(33, 32).unwrap();
        cur.into()
    }
}
