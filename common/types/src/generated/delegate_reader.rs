#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct DelegateLimit {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateLimit {
    fn from(cursor: Cursor) -> Self {
        DelegateLimit { cursor }
    }
}

impl DelegateLimit {
    pub fn threshold(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateLimit {
    pub fn max_delegator_size(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct DelegateInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateInfo {
    fn from(cursor: Cursor) -> Self {
        DelegateInfo { cursor }
    }
}

impl DelegateInfo {
    pub fn dividend_ratio(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

pub struct DelegateCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateCellData {
    fn from(cursor: Cursor) -> Self {
        DelegateCellData { cursor }
    }
}

impl DelegateCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn l1_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn l2_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn delegate_limit(&self) -> DelegateLimit {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn delegate_info(&self) -> DelegateInfo {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn checkpoint_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn xudt_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        cur.into()
    }
}

pub struct DelegatorInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegatorInfo {
    fn from(cursor: Cursor) -> Self {
        DelegatorInfo { cursor }
    }
}

impl DelegatorInfo {
    pub fn staker(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegatorInfo {
    pub fn delegate_amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl DelegatorInfo {
    pub fn inauguration_epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct DelegatorInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegatorInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl DelegatorInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl DelegatorInfos {
    pub fn get(&self, index: usize) -> DelegatorInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct DelegateAtCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateAtCellData {
    fn from(cursor: Cursor) -> Self {
        DelegateAtCellData { cursor }
    }
}

impl DelegateAtCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateAtCellData {
    pub fn delegator_infos(&self) -> DelegatorInfos {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl DelegateAtCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct StakerSmtRoot {
    pub cursor: Cursor,
}

impl From<Cursor> for StakerSmtRoot {
    fn from(cursor: Cursor) -> Self {
        StakerSmtRoot { cursor }
    }
}

impl StakerSmtRoot {
    pub fn staker(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakerSmtRoot {
    pub fn root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct StakerSmtRoots {
    pub cursor: Cursor,
}

impl From<Cursor> for StakerSmtRoots {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl StakerSmtRoots {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl StakerSmtRoots {
    pub fn get(&self, index: usize) -> StakerSmtRoot {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct DelegateSmtCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateSmtCellData {
    fn from(cursor: Cursor) -> Self {
        DelegateSmtCellData { cursor }
    }
}

impl DelegateSmtCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateSmtCellData {
    pub fn smt_roots(&self) -> StakerSmtRoots {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl DelegateSmtCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}
