# AXON development contracts

Clone the repo and submodules.
```
git submodule update --init
```

build secp256k1 archive:

``` sh
cd common/secp256k1/ckb-lib-secp256k1
make all-via-docker
```

build blst archive:   

The docker version has some problem, so we have to execute the following beforehand to update `no_asm.h` & `vect.h` mannually.
```
cd common/blst/ckb-lib-secp256k1-blst/blst/deps
git apply ../../blst.patch
```

Then, 
``` sh
cd common/blst/ckb-lib-secp256k1-blst
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
cargo test test_selection_success -- --nocapture
cargo test test_checkpoint_success -- --nocapture
cargo test test_stake_success -- --nocapture
cargo test test_withdrawal_success -- --nocapture
```
