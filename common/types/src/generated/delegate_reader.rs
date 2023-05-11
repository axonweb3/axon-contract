#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct DelegateArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateArgs {
    fn from(cursor: Cursor) -> Self {
        DelegateArgs { cursor }
    }
}

impl DelegateArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateArgs {
    pub fn delegator_addr(&self) -> Option<Vec<u8>> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        if cur.option_is_none() {
            None
        } else {
            Some(cur.into())
        }
    }
}

pub struct DelegateRequirement {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateRequirement {
    fn from(cursor: Cursor) -> Self {
        DelegateRequirement { cursor }
    }
}

impl DelegateRequirement {
    pub fn threshold(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateRequirement {
    pub fn max_delegator_size(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl DelegateRequirement {
    pub fn dividend_ratio(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
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
    pub fn delegate_requirement(&self) -> DelegateRequirement {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl DelegateCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

pub struct DelegateInfoDelta {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateInfoDelta {
    fn from(cursor: Cursor) -> Self {
        DelegateInfoDelta { cursor }
    }
}

impl DelegateInfoDelta {
    pub fn is_increase(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateInfoDelta {
    pub fn staker(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl DelegateInfoDelta {
    pub fn amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl DelegateInfoDelta {
    pub fn total_amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl DelegateInfoDelta {
    pub fn inauguration_epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

pub struct DelegateInfoDeltas {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateInfoDeltas {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl DelegateInfoDeltas {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl DelegateInfoDeltas {
    pub fn get(&self, index: usize) -> DelegateInfoDelta {
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
    pub fn l1_address(&self) -> Vec<u8> {
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

impl DelegateAtCellData {
    pub fn delegator_infos(&self) -> DelegateInfoDeltas {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
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

pub struct DelegateInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateInfo {
    fn from(cursor: Cursor) -> Self {
        DelegateInfo { cursor }
    }
}

impl DelegateInfo {
    pub fn delegator_addr(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateInfo {
    pub fn amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct DelegateInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl DelegateInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl DelegateInfos {
    pub fn get(&self, index: usize) -> DelegateInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct StakeGroupInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeGroupInfo {
    fn from(cursor: Cursor) -> Self {
        StakeGroupInfo { cursor }
    }
}

impl StakeGroupInfo {
    pub fn staker(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeGroupInfo {
    pub fn delegate_infos(&self) -> DelegateInfos {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeGroupInfo {
    pub fn delegate_old_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl StakeGroupInfo {
    pub fn delegate_new_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct StakeGroupInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeGroupInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl StakeGroupInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl StakeGroupInfos {
    pub fn get(&self, index: usize) -> StakeGroupInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct DelegateSmtUpdateInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateSmtUpdateInfo {
    fn from(cursor: Cursor) -> Self {
        DelegateSmtUpdateInfo { cursor }
    }
}

impl DelegateSmtUpdateInfo {
    pub fn all_stake_group_infos(&self) -> StakeGroupInfos {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}
