TARGET := riscv64-unknown-elf
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
AR := $(TARGET)-ar
OBJCOPY := $(TARGET)-objcopy

CFLAGS := -fPIC -O3 -DCKB_C_STDLIB_PRINTF -fno-builtin-printf -fno-builtin-memcmp -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections -I deps/secp256k1/src -I deps/secp256k1 -I deps/ckb-c-stdlib -I deps/ckb-c-stdlib/libc -I deps/ckb-c-stdlib/molecule -I c -I build -Wall  -Wno-nonnull -Wno-nonnull-compare -Wno-unused-function -g
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections
SECP256K1_SRC := deps/secp256k1/src/ecmult_static_pre_context.h
SCRIPT_CFLAGS := -I ckb-production-scripts/c -I ckb-production-scripts/deps/ckb-c-stdlib -I ckb-production-scripts/deps/ckb-c-stdlib/libc -I ckb-production-scripts/deps/ckb-c-stdlib/molecule

CKB_SCRIPT_SECP256K1 := ckb-production-scripts/deps/secp256k1
PROTOCOL_HEADER := ckb-production-scripts/c/blockchain.h
BUILDER_DOCKER := jjy0/ckb-capsule-recipe-rust:2020-9-28

all: build/libckb-lib-secp256k1.a

all-via-docker: ${PROTOCOL_HEADER}
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make all"

build/libckb-lib-secp256k1.a: build/secp256k1_blake2b.o
	$(AR) rcs $@ $^
	$(OBJCOPY) --strip-debug $@

build/secp256k1_blake2b.o: secp256k1_blake2b.c ${PROTOCOL_HEADER} ckb-production-scripts/c/secp256k1_lock.h build/secp256k1_data_info.h $(SECP256K1_SRC)
	$(CC) $(CFLAGS) ${SCRIPT_CFLAGS} -I ${CKB_SCRIPT_SECP256K1} -I ${CKB_SCRIPT_SECP256K1}/src -D __SHARED_LIBRARY__ -c -o $@ $<

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

clean:
	rm -f build/*.o build/*.a
	rm -rf build/secp256k1_data_info.h build/dump_secp256k1_data
	rm -rf build/secp256k1_data
