#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct RewardSmtCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardSmtCellData {
    fn from(cursor: Cursor) -> Self {
        RewardSmtCellData { cursor }
    }
}

impl RewardSmtCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl RewardSmtCellData {
    pub fn claim_smt_root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl RewardSmtCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct RewardArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardArgs {
    fn from(cursor: Cursor) -> Self {
        RewardArgs { cursor }
    }
}

impl RewardArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

pub struct NotClaimInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for NotClaimInfo {
    fn from(cursor: Cursor) -> Self {
        NotClaimInfo { cursor }
    }
}

impl NotClaimInfo {
    pub fn epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl NotClaimInfo {
    pub fn proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct RewardDelegateInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardDelegateInfo {
    fn from(cursor: Cursor) -> Self {
        RewardDelegateInfo { cursor }
    }
}

impl RewardDelegateInfo {
    pub fn delegator_addr(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl RewardDelegateInfo {
    pub fn amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct RewardDelegateInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardDelegateInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl RewardDelegateInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl RewardDelegateInfos {
    pub fn get(&self, index: usize) -> RewardDelegateInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct RewardStakeInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardStakeInfo {
    fn from(cursor: Cursor) -> Self {
        RewardStakeInfo { cursor }
    }
}

impl RewardStakeInfo {
    pub fn validator(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl RewardStakeInfo {
    pub fn propose_count(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl RewardStakeInfo {
    pub fn staker_amount(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl RewardStakeInfo {
    pub fn delegate_infos(&self) -> RewardDelegateInfos {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl RewardStakeInfo {
    pub fn delegate_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct RewardStakeInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardStakeInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl RewardStakeInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl RewardStakeInfos {
    pub fn get(&self, index: usize) -> RewardStakeInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct EpochRewardStakeInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for EpochRewardStakeInfo {
    fn from(cursor: Cursor) -> Self {
        EpochRewardStakeInfo { cursor }
    }
}

impl EpochRewardStakeInfo {
    pub fn reward_stake_infos(&self) -> RewardStakeInfos {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl EpochRewardStakeInfo {
    pub fn count_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl EpochRewardStakeInfo {
    pub fn count_root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl EpochRewardStakeInfo {
    pub fn count_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl EpochRewardStakeInfo {
    pub fn amount_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl EpochRewardStakeInfo {
    pub fn amount_root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl EpochRewardStakeInfo {
    pub fn amount_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct EpochRewardStakeInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for EpochRewardStakeInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl EpochRewardStakeInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl EpochRewardStakeInfos {
    pub fn get(&self, index: usize) -> EpochRewardStakeInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct RewardWitness {
    pub cursor: Cursor,
}

impl From<Cursor> for RewardWitness {
    fn from(cursor: Cursor) -> Self {
        RewardWitness { cursor }
    }
}

impl RewardWitness {
    pub fn miner(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl RewardWitness {
    pub fn old_not_claim_info(&self) -> NotClaimInfo {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl RewardWitness {
    pub fn reward_infos(&self) -> EpochRewardStakeInfos {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl RewardWitness {
    pub fn new_not_claim_info(&self) -> NotClaimInfo {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}
