
#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct WithdrawInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for WithdrawInfo {
    fn from(cursor: Cursor) -> Self {
        WithdrawInfo { cursor }
    }
}

impl WithdrawInfo {
    pub fn amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl WithdrawInfo {
    pub fn epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct WithdrawInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for WithdrawInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl WithdrawInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl WithdrawInfos {
    pub fn get(&self, index: usize) -> WithdrawInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct WithdrawAtCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for WithdrawAtCellData {
    fn from(cursor: Cursor) -> Self {
        WithdrawAtCellData { cursor }
    }
}

impl WithdrawAtCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl WithdrawAtCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl WithdrawAtCellData {
    pub fn withdraw_infos(&self) -> WithdrawInfos {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct WithdrawArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for WithdrawArgs {
    fn from(cursor: Cursor) -> Self {
        WithdrawArgs { cursor }
    }
}

impl WithdrawArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl WithdrawArgs {
    pub fn addr(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct WithdrawWitness {
    pub cursor: Cursor,
}

impl From<Cursor> for WithdrawWitness {
    fn from(cursor: Cursor) -> Self {
        WithdrawWitness { cursor }
    }
}

impl WithdrawWitness {
    pub fn signature(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}
