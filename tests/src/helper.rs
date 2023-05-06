#![allow(dead_code)]

use crate::axon;
use ckb_testtool::ckb_types::{packed::*, prelude::*};

pub fn axon_byte32(bytes: &Byte32) -> axon::Byte32 {
    let bytes: [u8; 32] = bytes.unpack().into();
    axon::Byte32::new_unchecked(bytes.to_vec().into())
}
