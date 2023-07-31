# AXON development contracts

Clone the repo and submodules.
```
git submodule update --init
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
You can use ```capsule build``` to build all contracts at once.
or build sepecific contract using following commands.
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
Also, you can run `cargo test` to run all tests.
