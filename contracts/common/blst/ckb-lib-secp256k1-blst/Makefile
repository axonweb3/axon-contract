TARGET := riscv64-unknown-elf
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
AR := $(TARGET)-ar
OBJCOPY := $(TARGET)-objcopy

CFLAGS := -fPIC -O3 -DCKB_C_STDLIB_PRINTF -fno-builtin-printf -fno-builtin-memcmp -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections -I deps/secp256k1/src -I deps/secp256k1 -I deps/ckb-c-stdlib -I deps/ckb-c-stdlib/libc -I deps/ckb-c-stdlib/molecule -I c -I build -Wall  -Wno-nonnull -Wno-nonnull-compare -Wno-unused-function -g
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections
SECP256K1_SRC := deps/secp256k1/src/ecmult_static_pre_context.h
SCRIPT_CFLAGS := -I ckb-production-scripts/c -I ckb-production-scripts/deps/ckb-c-stdlib -I ckb-production-scripts/deps/ckb-c-stdlib/libc -I ckb-production-scripts/deps/ckb-c-stdlib/molecule
CFLAGS_BLST := -fno-builtin-printf -Ickb-production-scripts/deps/blst/bindings $(CFLAGS) $(SCRIPT_CFLAGS)

CKB_SCRIPT_SECP256K1 := ckb-production-scripts/deps/secp256k1
PROTOCOL_HEADER := ckb-production-scripts/c/blockchain.h
BUILDER_DOCKER := thewawar/ckb-capsule:2021-12-25

all: build/libckb-lib-secp256k1-blst.a #build/bls12_381_sighash_all 

all-via-docker: ${PROTOCOL_HEADER}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make all"

build/libckb-lib-secp256k1-blst.a: build/bls12_381_sighash_all.o build/server-asm.o build/blst_mul_mont_384.o build/blst_mul_mont_384x.o
	$(AR) rcs $@ $^
	$(OBJCOPY) --strip-debug $@

build/bls12_381_sighash_all: bls12_381_sighash_all_old.c build/server-asm.o build/blst_mul_mont_384.o build/blst_mul_mont_384x.o
	$(CC) $(CFLAGS_BLST) ${LDFLAGS} -o $@ $^
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@

build/bls12_381_sighash_all.o: bls12_381_sighash_all.c ${PROTOCOL_HEADER} ckb-production-scripts/c/secp256k1_lock.h build/secp256k1_data_info.h $(SECP256K1_SRC)
	$(CC) $(CFLAGS_BLST) ${SCRIPT_CFLAGS} -I ${CKB_SCRIPT_SECP256K1} -I ${CKB_SCRIPT_SECP256K1}/src -D __SHARED_LIBRARY__ -c -o $@ $<

build/secp256k1_data_info.h: build/dump_secp256k1_data
	$<

build/dump_secp256k1_data: ckb-production-scripts/c/dump_secp256k1_data.c $(SECP256K1_SRC)
	mkdir -p build
	gcc -I ${CKB_SCRIPT_SECP256K1}/src -I ${CKB_SCRIPT_SECP256K1} -o $@ $<

$(SECP256K1_SRC):
	cd ckb-production-scripts/deps/secp256k1 && \
		./autogen.sh && \
		CC=$(CC) LD=$(LD) ./configure --with-bignum=no --enable-ecmult-static-precomputation --enable-endomorphism --enable-module-recovery --host=$(TARGET) && \
		make src/ecmult_static_pre_context.h src/ecmult_static_context.h

build/server-asm.o: ckb-production-scripts/deps/blst/src/server.c
	$(CC) -c -DUSE_MUL_MONT_384_ASM -DCKB_DECLARATION_ONLY $(CFLAGS_BLST) $(LDFLAGS) -o $@ $<

build/blst_mul_mont_384.o: blst/blst_mul_mont_384.riscv.S
	$(CC) -c -DCKB_DECLARATION_ONLY $(CFLAGS_BLST) -o $@ $^

build/blst_mul_mont_384x.o: blst/blst_mul_mont_384x.riscv.S
	$(CC) -c -DCKB_DECLARATION_ONLY $(CFLAGS_BLST) -o $@ $^

clean:
	rm -f build/*.o build/*.a build/bls*
	rm -rf build/secp256k1_data_info.h build/dump_secp256k1_data
	rm -rf build/secp256k1_data
