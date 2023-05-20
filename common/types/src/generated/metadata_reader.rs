#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct Validator {
    pub cursor: Cursor,
}

impl From<Cursor> for Validator {
    fn from(cursor: Cursor) -> Self {
        Validator { cursor }
    }
}

impl Validator {
    pub fn bls_pub_key(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl Validator {
    pub fn pub_key(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl Validator {
    pub fn address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl Validator {
    pub fn propose_weight(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl Validator {
    pub fn vote_weight(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl Validator {
    pub fn propose_count(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

pub struct MetadataList {
    pub cursor: Cursor,
}

impl From<Cursor> for MetadataList {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl MetadataList {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl MetadataList {
    pub fn get(&self, index: usize) -> Metadata {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct ValidatorList {
    pub cursor: Cursor,
}

impl From<Cursor> for ValidatorList {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl ValidatorList {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl ValidatorList {
    pub fn get(&self, index: usize) -> Validator {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct Metadata {
    pub cursor: Cursor,
}

impl From<Cursor> for Metadata {
    fn from(cursor: Cursor) -> Self {
        Metadata { cursor }
    }
}

impl Metadata {
    pub fn epoch_len(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn period_len(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn quorum(&self) -> u16 {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn gas_limit(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn gas_price(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn interval(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn validators(&self) -> ValidatorList {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn propose_ratio(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(7).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn prevote_ratio(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(8).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn precommit_ratio(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(9).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn brake_ratio(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(10).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn tx_num_limit(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(11).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn max_tx_size(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(12).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn block_height(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(13).unwrap();
        cur.into()
    }
}

pub struct TypeIds {
    pub cursor: Cursor,
}

impl From<Cursor> for TypeIds {
    fn from(cursor: Cursor) -> Self {
        TypeIds { cursor }
    }
}

impl TypeIds {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl TypeIds {
    pub fn checkpoint_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl TypeIds {
    pub fn stake_smt_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl TypeIds {
    pub fn delegate_smt_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl TypeIds {
    pub fn reward_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl TypeIds {
    pub fn xudt_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

impl TypeIds {
    pub fn withdraw_code_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        cur.into()
    }
}

pub struct MetadataCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for MetadataCellData {
    fn from(cursor: Cursor) -> Self {
        MetadataCellData { cursor }
    }
}

impl MetadataCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl MetadataCellData {
    pub fn epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl MetadataCellData {
    pub fn propose_count_smt_root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl MetadataCellData {
    pub fn type_ids(&self) -> TypeIds {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl MetadataCellData {
    pub fn metadata(&self) -> MetadataList {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

pub struct MetadataArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for MetadataArgs {
    fn from(cursor: Cursor) -> Self {
        MetadataArgs { cursor }
    }
}

impl MetadataArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

pub struct MetadataWitness {
    pub cursor: Cursor,
}

impl From<Cursor> for MetadataWitness {
    fn from(cursor: Cursor) -> Self {
        MetadataWitness { cursor }
    }
}

impl MetadataWitness {
    pub fn new_exist_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
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
    pub fn addr(&self) -> Vec<u8> {
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
// warning: Uint128Opt not implemented for Rust
pub struct Uint128Opt {
    pub cursor: Cursor,
}
impl From<Cursor> for Uint128Opt {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

pub struct MinerGroupInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for MinerGroupInfo {
    fn from(cursor: Cursor) -> Self {
        MinerGroupInfo { cursor }
    }
}

impl MinerGroupInfo {
    pub fn staker(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl MinerGroupInfo {
    pub fn amount(&self) -> Option<Vec<u8>> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        if cur.option_is_none() {
            None
        } else {
            Some(cur.into())
        }
    }
}

impl MinerGroupInfo {
    pub fn delegate_infos(&self) -> DelegateInfos {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl MinerGroupInfo {
    pub fn delegate_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct MinerGroupInfos {
    pub cursor: Cursor,
}

impl From<Cursor> for MinerGroupInfos {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl MinerGroupInfos {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl MinerGroupInfos {
    pub fn get(&self, index: usize) -> MinerGroupInfo {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct ElectionSmtProof {
    pub cursor: Cursor,
}

impl From<Cursor> for ElectionSmtProof {
    fn from(cursor: Cursor) -> Self {
        ElectionSmtProof { cursor }
    }
}

impl ElectionSmtProof {
    pub fn miners(&self) -> MinerGroupInfos {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl ElectionSmtProof {
    pub fn staker_epoch_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct DelegateProof {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateProof {
    fn from(cursor: Cursor) -> Self {
        DelegateProof { cursor }
    }
}

impl DelegateProof {
    pub fn staker(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl DelegateProof {
    pub fn proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct DelegateProofs {
    pub cursor: Cursor,
}

impl From<Cursor> for DelegateProofs {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl DelegateProofs {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl DelegateProofs {
    pub fn get(&self, index: usize) -> DelegateProof {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct StakeSmtElectionInfo {
    pub cursor: Cursor,
}

impl From<Cursor> for StakeSmtElectionInfo {
    fn from(cursor: Cursor) -> Self {
        StakeSmtElectionInfo { cursor }
    }
}

impl StakeSmtElectionInfo {
    pub fn n1(&self) -> ElectionSmtProof {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl StakeSmtElectionInfo {
    pub fn n2(&self) -> ElectionSmtProof {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl StakeSmtElectionInfo {
    pub fn new_stake_proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl StakeSmtElectionInfo {
    pub fn new_delegate_proofs(&self) -> DelegateProofs {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}
