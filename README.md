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
cd common/blst/ckb-lib-secp256k1-blst/deps
git apply ../blst/blst.patch
```
To avoid duplicate symbol errors, we must delete some code(from line 168 to 465) in `contracts/ckb-smt/c/deps/ckb-c-stdlib/blake2b.h`.  

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
capsule build -n withdraw
```

run tests:

``` sh
cd tests
cargo test test_selection_success -- --nocapture
cargo test test_checkpoint_success -- --nocapture
cargo test test_stake_success -- --nocapture
```
