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
    pub fn reward_smt_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(0, 32).unwrap();
        cur.into()
    }
}

impl SelectionLockArgs {
    pub fn issue_lock_hash(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(32, 32).unwrap();
        cur.into()
    }
}
