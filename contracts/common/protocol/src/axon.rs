
#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

pub struct Byte4 {
    cursor: Cursor,
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
    cursor: Cursor,
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

pub struct Byte10 {
    cursor: Cursor,
}

impl From<Cursor> for Byte10 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Byte10 {
    pub fn len(&self) -> usize {
        10
    }
}

impl Byte10 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Byte16 {
    cursor: Cursor,
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
    cursor: Cursor,
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
    cursor: Cursor,
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

pub struct Byte97 {
    cursor: Cursor,
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

pub struct Identity {
    cursor: Cursor,
}

impl From<Cursor> for Identity {
    fn from(cursor: Cursor) -> Self {
        Identity { cursor }
    }
}

impl Identity {
    pub fn flag(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl Identity {
    pub fn content(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct StakeInfo {
    cursor: Cursor,
}

impl From<Cursor> for StakeInfo {
    fn from(cursor: Cursor) -> Self {
        StakeInfo { cursor }
    }
}

impl StakeInfo {
    pub fn identity(&self) -> Identity {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeInfo {
    pub fn l2_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeInfo {
    pub fn bls_pub_key(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl StakeInfo {
    pub fn stake_amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl StakeInfo {
    pub fn inauguration_era(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

pub struct StakeInfoVec {
    cursor: Cursor,
}

impl From<Cursor> for StakeInfoVec {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl StakeInfoVec {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl StakeInfoVec {
    pub fn get(&self, index: usize) -> StakeInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct Bytes {
    cursor: Cursor,
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
    cursor: Cursor,
}
impl From<Cursor> for BytesOpt {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

pub struct StakeLockCellData {
    cursor: Cursor,
}

impl From<Cursor> for StakeLockCellData {
    fn from(cursor: Cursor) -> Self {
        StakeLockCellData { cursor }
    }
}

impl StakeLockCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeLockCellData {
    pub fn stake_infos(&self) -> StakeInfoVec {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeLockCellData {
    pub fn checkpoint_type_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl StakeLockCellData {
    pub fn sudt_type_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl StakeLockCellData {
    pub fn quorum_size(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

pub struct CheckpointLockArgs {
    cursor: Cursor,
}

impl From<Cursor> for CheckpointLockArgs {
    fn from(cursor: Cursor) -> Self {
        CheckpointLockArgs { cursor }
    }
}

impl CheckpointLockArgs {
    pub fn admin_identity(&self) -> Identity {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl CheckpointLockArgs {
    pub fn type_id_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct CheckpointLockCellData {
    cursor: Cursor,
}

impl From<Cursor> for CheckpointLockCellData {
    fn from(cursor: Cursor) -> Self {
        CheckpointLockCellData { cursor }
    }
}

impl CheckpointLockCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn state(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn period(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn era(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn block_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn period_interval(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn era_period(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn unlock_period(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(7).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn base_reward(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(8).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn half_period(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(9).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn common_ref(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(10).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn sudt_type_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(11).unwrap();
        cur.into()
    }
}

impl CheckpointLockCellData {
    pub fn stake_type_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(12).unwrap();
        cur.into()
    }
}

pub struct CheckpointLockWitnessLock {
    cursor: Cursor,
}

impl From<Cursor> for CheckpointLockWitnessLock {
    fn from(cursor: Cursor) -> Self {
        CheckpointLockWitnessLock { cursor }
    }
}

impl CheckpointLockWitnessLock {
    pub fn checkpoint(&self) -> Option<Vec<u8>> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        if cur.option_is_none() {
            None
        } else {
            Some(cur.into())
        }
    }
}

impl CheckpointLockWitnessLock {
    pub fn signature(&self) -> Option<Vec<u8>> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        if cur.option_is_none() {
            None
        } else {
            Some(cur.into())
        }
    }
}
