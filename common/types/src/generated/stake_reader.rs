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
        let cur = self.cursor.slice_by_offset(0, 32).unwrap();
        cur.into()
    }
}

impl StakeArgs {
    pub fn stake_addr(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(32, 20).unwrap();
        cur.into()
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

pub struct DelegateRequirementArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateRequirementArgs {
    fn from(cursor: Cursor) -> Self {
        DelegateRequirementArgs { cursor }
    }
}

impl DelegateRequirementArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(0, 32).unwrap();
        cur.into()
    }
}

impl DelegateRequirementArgs {
    pub fn requirement_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(32, 32).unwrap();
        cur.into()
    }
}

pub struct DelegateRequirementInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateRequirementInfo {
    fn from(cursor: Cursor) -> Self {
        DelegateRequirementInfo { cursor }
    }
}

impl DelegateRequirementInfo {
    pub fn code_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateRequirementInfo {
    pub fn requirement(&self) -> DelegateRequirementArgs {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct StakeAtCellLockData {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeAtCellLockData {
    fn from(cursor: Cursor) -> Self {
        StakeAtCellLockData { cursor }
    }
}

impl StakeAtCellLockData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn l1_pub_key(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn bls_pub_key(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn l1_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn l2_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn requirement_info(&self) -> DelegateRequirementInfo {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        cur.into()
    }
}

impl StakeAtCellLockData {
    pub fn delta(&self) -> StakeInfoDelta {
        let cur = self.cursor.table_slice_by_index(7).unwrap();
        cur.into()
    }
}

pub struct BytesVec {
    pub cursor: Cursor,
}

impl From<Cursor> for BytesVec {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl BytesVec {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl BytesVec {
    pub fn get(&self, index: usize) -> Vec<u8> {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
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
    pub fn lock(&self) -> StakeAtCellLockData {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeAtCellData {
    pub fn data(&self) -> BytesVec {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct StakeAtWitness {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeAtWitness {
    fn from(cursor: Cursor) -> Self {
        StakeAtWitness { cursor }
    }
}

impl StakeAtWitness {
    pub fn mode(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeAtWitness {
    pub fn eth_sig(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
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
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeSmtCellData {
    pub fn smt_root(&self) -> Vec<u8> {
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

pub struct StakeSmtWitness {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeSmtWitness {
    fn from(cursor: Cursor) -> Self {
        StakeSmtWitness { cursor }
    }
}

impl StakeSmtWitness {
    pub fn mode(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeSmtWitness {
    pub fn update_info(&self) -> StakeSmtUpdateInfo {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}
