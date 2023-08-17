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
}
