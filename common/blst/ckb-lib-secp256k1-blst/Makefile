TARGET := riscv64-unknown-elf
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
AR := $(TARGET)-ar
OBJCOPY := $(TARGET)-objcopy

CFLAGS := -fPIC -O3 -DCKB_C_STDLIB_PRINTF -fno-builtin-printf -fno-builtin-memcmp -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections -I build -Wall -Wno-nonnull -Wno-nonnull-compare -Wno-unused-function -Wno-array-bounds -Wno-stringop-overflow -Wno-array-parameter
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections
SCRIPT_CFLAGS := -I c -I deps/ckb-c-stdlib -I deps/ckb-c-stdlib/libc -I deps/ckb-c-stdlib/molecule
CFLAGS_BLST := -fno-builtin-printf -Ideps/blst/bindings $(CFLAGS) $(SCRIPT_CFLAGS)

CKB_SCRIPT_SECP256K1 := deps/secp256k1
BUILDER_DOCKER := thewawar/ckb-capsule:2022-08-01

all: build/libckb-lib-secp256k1-blst.a

all-via-docker:
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make all"

build/libckb-lib-secp256k1-blst.a: build/bls12_381_sighash_all.o build/server-asm.o build/blst_mul_mont_384.o build/blst_mul_mont_384x.o
	$(AR) rcs $@ $^
	$(OBJCOPY) --strip-debug $@

build/bls12_381_sighash_all.o: bls12_381_sighash_all.c build/secp256k1_data_info.h
	$(CC) $(CFLAGS_BLST) -I ${CKB_SCRIPT_SECP256K1} -I ${CKB_SCRIPT_SECP256K1}/src -D __SHARED_LIBRARY__ -c -o $@ $<

build/blake2b_ref.o: c/blake2b_ref.c
	gcc -c -o $@ $<

build/secp256k1_data_info.h: build/dump_secp256k1_data
	$<

build/dump_secp256k1_data: c/dump_secp256k1_data.c secp256k1 build/blake2b_ref.o
	mkdir -p build
	gcc -I ${CKB_SCRIPT_SECP256K1}/src -I ${CKB_SCRIPT_SECP256K1} -o $@ c/dump_secp256k1_data.c build/blake2b_ref.o

secp256k1:
	cd deps/secp256k1 && \
		./autogen.sh && \
		CC=$(CC) LD=$(LD) ./configure --with-bignum=no --enable-ecmult-static-precomputation --enable-endomorphism --enable-module-recovery --host=$(TARGET) && \
		make src/ecmult_static_pre_context.h src/ecmult_static_context.h
	
blst-apply-patch:
	cd deps/blst && \
		git apply ../../blst/blst.patch || echo "applying patch: ignore errors if applied."

build/server-asm.o: deps/blst/src/server.c
	$(CC) -c -DUSE_MUL_MONT_384_ASM -DCKB_DECLARATION_ONLY $(CFLAGS_BLST) $(LDFLAGS) -o $@ $<

build/blst_mul_mont_384.o: blst/blst_mul_mont_384.riscv.S
	$(CC) -c -DCKB_DECLARATION_ONLY $(CFLAGS_BLST) -o $@ $^

build/blst_mul_mont_384x.o: blst/blst_mul_mont_384x.riscv.S
	$(CC) -c -DCKB_DECLARATION_ONLY $(CFLAGS_BLST) -o $@ $^

clean:
	rm -f build/*.o build/*.a build/bls*
	rm -rf build/secp256k1_data_info.h build/dump_secp256k1_data
	rm -rf build/secp256k1_data
