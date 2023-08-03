# AXON development contracts

Clone the repo and submodules. Because the submodules we used contains submodules too, we should add `--recursive` parameter.
```
git submodule update --init --recursive
```

Build blst archive (only contract checkpoint needs it now, so you can skip this step if checkpoint is not needed)   

The docker version has some problem, so we have to execute the following beforehand to update `no_asm.h` & `vect.h` mannually.
```
cd common/blst/ckb-lib-secp256k1-blst/deps/blst
git apply ../../blst/blst.patch
```
After runing the above command, you should see changes in `no_asm.h` & `vect.h`.

Then, We can build lib blst:   
``` sh
cd common/blst/ckb-lib-secp256k1-blst
make all-via-docker
```

Build CKB contracts:
You can use ```capsule build``` to build all contracts at once.
or build sepecific contract using following commands.
``` sh
capsule build -n selection
capsule build -n checkpoint
...
```

run tests:
You can run `cargo test` to run all tests.
Also, you can run test for specific contract using following commands.
``` sh
cd tests
cargo test test_selection_success -- --nocapture
cargo test test_checkpoint_success -- --nocapture
...
```
