# AXON development contracts

build secp256k1 archive:

``` sh
cd contracts/common/secp256k1/ckb-lib-secp256k1
make all-via-docker
```

build blst archive:

``` sh
cd contracts/common/blst/ckb-lib-secp256k1-blst
make all-via-docker
```

build contracts:

``` sh
capsule build -n selection
capsule build -n checkpoint
capsule build -n stake
capsule build -n withdrawal
```

run tests:

``` sh
cd tests
cargo test_selection_success -- --nocapture
cargo test_checkpoint_success -- --nocapture
cargo test_stake_success -- --nocapture
cargo test_withdrawal_success -- --nocapture
```
