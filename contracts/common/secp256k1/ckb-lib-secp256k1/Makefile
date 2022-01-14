TARGET := riscv64-unknown-elf
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
AR := $(TARGET)-ar
OBJCOPY := $(TARGET)-objcopy

CFLAGS := -fPIC -O3 -DCKB_C_STDLIB_PRINTF -fno-builtin-printf -fno-builtin-memcmp -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections -I build -Wall -Wno-nonnull -Wno-nonnull-compare -Wno-unused-function -Wno-array-bounds -Wno-stringop-overflow
LDFLAGS := -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections
SCRIPT_CFLAGS := -I c -I deps/ckb-c-stdlib -I deps/ckb-c-stdlib/libc -I deps/ckb-c-stdlib/molecule

CKB_SCRIPT_SECP256K1 := deps/secp256k1
BUILDER_DOCKER := thewawar/ckb-capsule:2021-12-25

all: build/libckb-lib-secp256k1.a

all-via-docker:
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make all"

build/libckb-lib-secp256k1.a: build/secp256k1_blake2b.o
	$(AR) rcs $@ $^
	$(OBJCOPY) --strip-debug $@

build/secp256k1_blake2b.o: secp256k1_lock.c build/secp256k1_data_info.h secp256k1
	$(CC) $(CFLAGS) ${SCRIPT_CFLAGS} -I ${CKB_SCRIPT_SECP256K1} -I ${CKB_SCRIPT_SECP256K1}/src -D __SHARED_LIBRARY__ -c -o $@ $<

build/secp256k1_data_info.h: build/dump_secp256k1_data
	$<

build/dump_secp256k1_data: c/dump_secp256k1_data.c secp256k1
	mkdir -p build
	gcc -I ${CKB_SCRIPT_SECP256K1}/src -I ${CKB_SCRIPT_SECP256K1} -o $@ $<

secp256k1:
	cd deps/secp256k1 && \
		./autogen.sh && \
		CC=$(CC) LD=$(LD) ./configure --with-bignum=no --enable-ecmult-static-precomputation --enable-endomorphism --enable-module-recovery --host=$(TARGET) && \
		make src/ecmult_static_pre_context.h src/ecmult_static_context.h

clean:
	rm -f build/*.o build/*.a
	rm -rf build/secp256k1_data_info.h build/dump_secp256k1_data
	rm -rf build/secp256k1_data
