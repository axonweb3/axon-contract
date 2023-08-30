extern crate alloc;

use alloc::collections::BTreeMap;

pub struct WithdrawAmountMap {
    pub map: BTreeMap<[u8; 20], u128>,
}

impl WithdrawAmountMap {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    // if exist, it will accumulate
    pub fn insert(&mut self, addr: [u8; 20], amount: u128) {
        let entry = self.map.entry(addr).or_insert(0);
        *entry += amount;
    }

    // if exist, it will clear then set
    pub fn reset_insert(&mut self, addr: [u8; 20], amount: u128) {
        let entry = self.map.entry(addr).or_insert(0);
        *entry = amount;
    }
}

pub enum EpochClass {
    CURRENT = 0,
    NEXT = 1,
}

impl Into<usize> for EpochClass {
    fn into(self) -> usize {
        self as usize
    }
}
