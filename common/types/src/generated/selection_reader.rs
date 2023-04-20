#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct SelectionLockArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for SelectionLockArgs {
    fn from(cursor: Cursor) -> Self {
        SelectionLockArgs { cursor }
    }
}

impl SelectionLockArgs {
    pub fn omni_lock_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl SelectionLockArgs {
    pub fn checkpoint_lock_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}
