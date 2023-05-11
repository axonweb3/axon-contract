#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct StakeArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeArgs {
    fn from(cursor: Cursor) -> Self {
        StakeArgs { cursor }
    }
}

impl StakeArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeArgs {
    pub fn stake_addr(&self) -> Option<Vec<u8>> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        if cur.option_is_none() {
            None
        } else {
            Some(cur.into())
        }
    }
}

pub struct StakeInfoDelta {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeInfoDelta {
    fn from(cursor: Cursor) -> Self {
        StakeInfoDelta { cursor }
    }
}

impl StakeInfoDelta {
    pub fn is_increase(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeInfoDelta {
    pub fn amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeInfoDelta {
    pub fn inauguration_epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct StakeAtCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeAtCellData {
    fn from(cursor: Cursor) -> Self {
        StakeAtCellData { cursor }
    }
}

impl StakeAtCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeAtCellData {
    pub fn l1_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeAtCellData {
    pub fn l2_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl StakeAtCellData {
    pub fn stake_info(&self) -> StakeInfoDelta {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl StakeAtCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

pub struct StakeSmtCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeSmtCellData {
    fn from(cursor: Cursor) -> Self {
        StakeSmtCellData { cursor }
    }
}

impl StakeSmtCellData {
    pub fn smt_root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeSmtCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeSmtCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct StakeInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeInfo {
    fn from(cursor: Cursor) -> Self {
        StakeInfo { cursor }
    }
}

impl StakeInfo {
    pub fn addr(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeInfo {
    pub fn amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct StakeInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl StakeInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl StakeInfos {
    pub fn get(&self, index: usize) -> StakeInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct StakeSmtUpdateInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeSmtUpdateInfo {
    fn from(cursor: Cursor) -> Self {
        StakeSmtUpdateInfo { cursor }
    }
}

impl StakeSmtUpdateInfo {
    pub fn all_stake_infos(&self) -> StakeInfos {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeSmtUpdateInfo {
    pub fn old_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl StakeSmtUpdateInfo {
    pub fn new_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}
