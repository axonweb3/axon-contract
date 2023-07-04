#![allow(dead_code)]
#![allow(unused_imports)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

use super::basic::*;
pub struct ProposeCount {
    pub cursor: Cursor,
}

impl From<Cursor> for ProposeCount {
    fn from(cursor: Cursor) -> Self {
        ProposeCount { cursor }
    }
}

impl ProposeCount {
    pub fn address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl ProposeCount {
    pub fn count(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

pub struct ProposeCounts {
    pub cursor: Cursor,
}

impl From<Cursor> for ProposeCounts {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl ProposeCounts {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl ProposeCounts {
    pub fn get(&self, index: usize) -> ProposeCount {
        let cur = self.cursor.dynvec_slice_by_index(index).unwrap();
        cur.into()
    }
}

pub struct CheckpointCellData {
    pub cursor: Cursor,
}

impl From<Cursor> for CheckpointCellData {
    fn from(cursor: Cursor) -> Self {
        CheckpointCellData { cursor }
    }
}

impl CheckpointCellData {
    pub fn version(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn epoch(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn period(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn state_root(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn latest_block_height(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(4).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn latest_block_hash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(5).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(6).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn timestamp(&self) -> u64 {
        let cur = self.cursor.table_slice_by_index(7).unwrap();
        cur.into()
    }
}

impl CheckpointCellData {
    pub fn propose_count(&self) -> ProposeCounts {
        let cur = self.cursor.table_slice_by_index(8).unwrap();
        cur.into()
    }
}

pub struct CheckpointWitness {
    pub cursor: Cursor,
}

impl From<Cursor> for CheckpointWitness {
    fn from(cursor: Cursor) -> Self {
        CheckpointWitness { cursor }
    }
}

impl CheckpointWitness {
    pub fn proposal(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl CheckpointWitness {
    pub fn proof(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

pub struct CheckpointArgs {
    pub cursor: Cursor,
}

impl From<Cursor> for CheckpointArgs {
    fn from(cursor: Cursor) -> Self {
        CheckpointArgs { cursor }
    }
}

impl CheckpointArgs {
    pub fn metadata_type_id(&self) -> Vec<u8> {
        let cur = self.cursor.slice_by_offset(0, 32).unwrap();
        cur.into()
    }
}
