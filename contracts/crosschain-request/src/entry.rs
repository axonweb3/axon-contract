// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{
        load_cell_capacity, load_cell_data, load_cell_lock, load_cell_type_hash, load_script,
    },
};

use protocol::{crosschain, Cursor};
use util::{error::Error, helper::*};

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();

    // only valid in Output
    let exist = load_cell_type_hash(0, Source::GroupOutput)?;
    if let None = exist {
        debug!("unworkable crosschain request");
        return Ok(());
    }

    // convert args to Transfer format
    let transfer: crosschain::Transfer = Cursor::from(args.to_vec()).into();

    // assume first of Output is ACS_Lock
    let metadata_typehash = {
        let acs_lock = load_cell_lock(0, Source::Output)?;
        if acs_lock.args().len() != 32 {
            return Err(Error::BadScriptArgs);
        }
        acs_lock.args().unpack()
    };

    // load metadata from CellDep
    let metadata = get_metadata_from_celldep(&metadata_typehash)?;

    // get token crosschain fee from metadata
    let ckb_fee = metadata.ckb_fee_ratio();
    let mut sudt_fee = None;
    let mut sudt_amount = 0u128;
    let token_config = metadata.token_config();
    for i in 0..token_config.len() {
        let token = token_config.get(i);
        if token.ERC20_address() == transfer.ERC20_address() {
            sudt_fee = Some(token.fee_ratio());
            sudt_amount = {
                let mut amount = 0u128;
                let sudt_typehash = load_cell_type_hash(0, Source::Output)?;
                if let Some(hash) = sudt_typehash {
                    if hash == token.sUDT_typehash().as_slice() {
                        let data = load_cell_data(0, Source::Output)?;
                        if data.len() < 16 {
                            return Err(Error::BadSUDTCell);
                        }
                        amount = bytes_to_u128(&data[..16].to_vec());
                    }
                }
                if amount == 0 {
                    return Err(Error::BadSUDTCell);
                }
                amount
            };
        }
    }
    debug!("ckb_fee = {:?}, sudt_fee = {:?}", ckb_fee, sudt_fee);

    // check CKB crosschain transfer validation
    if transfer.ckb_amount() > 0 {
        if ckb_fee >= 1000 {
            return Err(Error::MetadataCkbFeeError);
        }
        let lock_ckb = load_cell_capacity(0, Source::Output)?;
        debug!(
            "input_ckb = {}, output_ckb = {}",
            lock_ckb,
            transfer.ckb_amount()
        );
        if lock_ckb * (1000 - ckb_fee as u64) < transfer.ckb_amount() * 1000 {
            return Err(Error::InsufficientCrosschainCKB);
        }
    }

    // check sUDT crosschain transfer validation
    let transfer_sudt = bytes_to_u128(&transfer.sUDT_amount());
    if transfer_sudt > 0 {
        if let Some(fee) = sudt_fee {
            if fee >= 1000 {
                return Err(Error::MetadataSUDTFeeError);
            }
            debug!(
                "input_sudt = {}, output_sudt = {}",
                sudt_amount, transfer_sudt
            );
            if sudt_amount * (1000 - fee as u128) < transfer_sudt * 1000 {
                return Err(Error::InsufficientCrosschainSUDT);
            }
        } else {
            return Err(Error::MetadataSUDTFeeError);
        }
    }

    Ok(())
}
