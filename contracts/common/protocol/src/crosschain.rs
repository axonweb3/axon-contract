
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]
extern crate alloc;
use alloc::vec::Vec;
use molecule2::Cursor;

pub struct Uint32 {
    pub cursor: Cursor,
}

impl From<Cursor> for Uint32 {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Uint32 {
    pub fn len(&self) -> usize {
        4
    }
}

impl Uint32 {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Address {
    pub cursor: Cursor,
}

impl From<Cursor> for Address {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Address {
    pub fn len(&self) -> usize {
        20
    }
}

impl Address {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Hash {
    pub cursor: Cursor,
}

impl From<Cursor> for Hash {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Hash {
    pub fn len(&self) -> usize {
        32
    }
}

impl Hash {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct Pubkey {
    pub cursor: Cursor,
}

impl From<Cursor> for Pubkey {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl Pubkey {
    pub fn len(&self) -> usize {
        32
    }
}

impl Pubkey {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.slice_by_offset(1 * index, 1).unwrap();
        cur.into()
    }
}

pub struct String {
    pub cursor: Cursor,
}

impl From<Cursor> for String {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl String {
    pub fn len(&self) -> usize {
        self.cursor.fixvec_length()
    }
}

impl String {
    pub fn get(&self, index: usize) -> u8 {
        let cur = self.cursor.fixvec_slice_by_index(1, index).unwrap();
        cur.into()
    }
}

pub struct PubkeyList {
    pub cursor: Cursor,
}

impl From<Cursor> for PubkeyList {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl PubkeyList {
    pub fn len(&self) -> usize {
        self.cursor.fixvec_length()
    }
}

impl PubkeyList {
    pub fn get(&self, index: usize) -> Vec<u8> {
        let cur = self.cursor.fixvec_slice_by_index(32, index).unwrap();
        cur.into()
    }
}

pub struct Token {
    pub cursor: Cursor,
}

impl From<Cursor> for Token {
    fn from(cursor: Cursor) -> Self {
        Token { cursor }
    }
}

impl Token {
    pub fn symbol(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}

impl Token {
    pub fn sUDT_codehash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl Token {
    pub fn sUDT_owner_lockhash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl Token {
    pub fn fee_ratio(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        cur.into()
    }
}

pub struct TokenConfig {
    pub cursor: Cursor,
}

impl From<Cursor> for TokenConfig {
    fn from(cursor: Cursor) -> Self {
        Self { cursor }
    }
}

impl TokenConfig {
    pub fn len(&self) -> usize {
        self.cursor.dynvec_length()
    }
}

impl TokenConfig {
    pub fn get(&self, index: usize) -> Token {
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
    pub fn chain_id(&self) -> u8 {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn token_config(&self) -> TokenConfig {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl Metadata {
    pub fn checkpoint_typehash(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

pub struct Transfer {
    pub cursor: Cursor,
}

impl From<Cursor> for Transfer {
    fn from(cursor: Cursor) -> Self {
        Transfer { cursor }
    }
}

impl Transfer {
    pub fn axon_address(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(0).unwrap();
        cur.into()
    }
}

impl Transfer {
    pub fn ckb_amount(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(1).unwrap();
        cur.into()
    }
}

impl Transfer {
    pub fn sUDT_amount(&self) -> u32 {
        let cur = self.cursor.table_slice_by_index(2).unwrap();
        cur.into()
    }
}

impl Transfer {
    pub fn symbol(&self) -> Vec<u8> {
        let cur = self.cursor.table_slice_by_index(3).unwrap();
        let cur2 = cur.convert_to_rawbytes().unwrap();
        cur2.into()
    }
}
