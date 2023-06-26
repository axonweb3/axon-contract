// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    debug,
    high_level::{load_cell_data, load_cell_type, load_script, QueryIter},
};
use ckb_type_id::{load_type_id_from_script_args, validate_type_id};

use axon_types::delegate::DelegateRequirement;
use util::error::Error;

pub fn main() -> Result<(), Error> {
    let script = load_script()?;

    // check type id is unique
    let type_id = load_type_id_from_script_args(32)?;
    debug!("type_id: {:?}", type_id);
    validate_type_id(type_id)?;

    let script_args = script.args();
    let idx = QueryIter::new(load_cell_type, Source::Output)
        .into_iter()
        .position(|type_script| {
            type_script
                .map(|s| s.args() == script_args)
                .unwrap_or_default()
        })
        .unwrap();
    let data = load_cell_data(idx, Source::Output)?;
    let req = DelegateRequirement::from_slice(&data)?;

    if req.commission_rate().as_slice()[0] > 100 {
        return Err(Error::CommissionRateTooLarge);
    }

    Ok(())
}
