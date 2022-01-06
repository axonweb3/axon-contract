// uncomment to enable printf in CKB-VM
// #define CKB_C_STDLIB_PRINTF

// it's used by blockchain-api2.h, the behavior when panic
#ifndef MOL2_EXIT
#define MOL2_EXIT ckb_exit
#endif
int ckb_exit(signed char);

// clang-format off
#include <stdio.h>
#include <blake2b.h>
#include "blockchain-api2.h"
#include "blockchain.h"
#include "ckb_consts.h"
#include "ckb_syscalls.h"
#include "blst.h"

// clang-format on

#define CHECK2(cond, code) \
  do {                     \
    if (!(cond)) {         \
      err = code;          \
      ASSERT(0);           \
      goto exit;           \
    }                      \
  } while (0)

#define CHECK(_code)    \
  do {                  \
    int code = (_code); \
    if (code != 0) {    \
      err = code;       \
      ASSERT(0);        \
      goto exit;        \
    }                   \
  } while (0)

#define SCRIPT_SIZE 32768
#define MAX_LOCK_SCRIPT_HASH_COUNT 2048

#define CKB_IDENTITY_LEN 21
#define RECID_INDEX 64
#define ONE_BATCH_SIZE 32768
#define BLST_PUBKEY_SIZE 48
#define MAX_WITNESS_SIZE 32768
#define BLST_SIGNAUTRE_SIZE (48 + 96)
#define BLAKE2B_BLOCK_SIZE 32
#define BLAKE160_SIZE 20

const static uint8_t g_dst_label[] =
    "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
const static size_t g_dst_label_len = 43;

enum CkbIdentityErrorCode {
  ERROR_IDENTITY_ARGUMENTS_LEN = -1,
  ERROR_IDENTITY_ENCODING = -2,
  ERROR_IDENTITY_SYSCALL = -3,

  // compatible with secp256k1 pubkey hash verification
  ERROR_IDENTITY_SECP_RECOVER_PUBKEY = -11,
  ERROR_IDENTITY_SECP_PARSE_SIGNATURE = -14,
  ERROR_IDENTITY_SECP_SERIALIZE_PUBKEY = -15,
  ERROR_IDENTITY_PUBKEY_BLAKE160_HASH = -31,
  // new error code
  ERROR_IDENTITY_LOCK_SCRIPT_HASH_NOT_FOUND = 70,
  ERROR_INVALID_MOL_FORMAT,
  ERROR_BLST_VERIFY_FAILED,
};

static BLST_ERROR blst_verify(const uint8_t *sig, const uint8_t *pk,
                              const uint8_t *msg, size_t msg_len) {
  BLST_ERROR err;
  blst_p1_affine pk_p1_affine;
  blst_p1_uncompress(&pk_p1_affine, pk);
  blst_p2_affine sig_p2_affine;
  blst_p2_uncompress(&sig_p2_affine, sig);

#if 1
  // using one-shot
  printf("using one-shot\n");
  err =
      blst_core_verify_pk_in_g1(&pk_p1_affine, &sig_p2_affine, true, msg,
                                msg_len, g_dst_label, g_dst_label_len, NULL, 0);
  CHECK(err);
#else
  // using pairing interface

  // pubkey must be checked
  // signature will be checked internally later.
  printf("using pairing interface\n");
  uint8_t ctx_buff[blst_pairing_sizeof()];

  bool in_g1 = blst_p1_affine_in_g1(&pk_p1_affine);
  CHECK2(in_g1, -1);

  blst_pairing *ctx = (blst_pairing *)ctx_buff;
  blst_pairing_init(ctx, true, g_dst_label, g_dst_label_len);
  err = blst_pairing_aggregate_pk_in_g1(ctx, &pk_p1_affine, &sig_p2_affine, msg,
                                        msg_len, NULL, 0);
  CHECK(err);
  blst_pairing_commit(ctx);

  bool b = blst_pairing_finalverify(ctx, NULL);
  CHECK2(b, -1);
#endif

exit:
  return err;
}

static int extract_witness_lock(uint8_t *witness, uint64_t len,
                                mol_seg_t *lock_bytes_seg) {
  if (len < 20) {
    return ERROR_IDENTITY_ENCODING;
  }
  uint32_t lock_length = *((uint32_t *)(&witness[16]));
  if (len < 20 + lock_length) {
    return ERROR_IDENTITY_ENCODING;
  } else {
    lock_bytes_seg->ptr = &witness[20];
    lock_bytes_seg->size = lock_length;
  }
  return CKB_SUCCESS;
}

int load_and_hash_witness(blake2b_state *ctx, size_t start, size_t index,
                          size_t source, bool hash_length) {
  uint8_t temp[ONE_BATCH_SIZE];
  uint64_t len = ONE_BATCH_SIZE;
  int ret = ckb_load_witness(temp, &len, start, index, source);
  if (ret != CKB_SUCCESS) {
    return ret;
  }
  if (hash_length) {
    blake2b_update(ctx, (char *)&len, sizeof(uint64_t));
  }
  uint64_t offset = (len > ONE_BATCH_SIZE) ? ONE_BATCH_SIZE : len;
  blake2b_update(ctx, temp, offset);
  while (offset < len) {
    uint64_t current_len = ONE_BATCH_SIZE;
    ret = ckb_load_witness(temp, &current_len, start + offset, index, source);
    if (ret != CKB_SUCCESS) {
      return ret;
    }
    uint64_t current_read =
        (current_len > ONE_BATCH_SIZE) ? ONE_BATCH_SIZE : current_len;
    blake2b_update(ctx, temp, current_read);
    offset += current_read;
  }
  return CKB_SUCCESS;
}

int verify_bls12_381_blake160_sighash_all(uint8_t *pubkey_hash) {
  int ret;
  uint64_t len = 0;
  unsigned char temp[MAX_WITNESS_SIZE];
  unsigned char lock_bytes[BLST_SIGNAUTRE_SIZE];
  uint64_t read_len = MAX_WITNESS_SIZE;
  uint64_t witness_len = MAX_WITNESS_SIZE;

  /* Load witness of first input */
  ret = ckb_load_witness(temp, &read_len, 0, 0, CKB_SOURCE_GROUP_INPUT);
  if (ret != CKB_SUCCESS) {
    return ERROR_IDENTITY_SYSCALL;
  }
  witness_len = read_len;
  if (read_len > MAX_WITNESS_SIZE) {
    read_len = MAX_WITNESS_SIZE;
  }

  /* load signature */
  mol_seg_t lock_bytes_seg;
  ret = extract_witness_lock(temp, read_len, &lock_bytes_seg);
  if (ret != 0) {
    return ERROR_IDENTITY_ENCODING;
  }
  if (lock_bytes_seg.size < BLST_SIGNAUTRE_SIZE) {
    return ERROR_IDENTITY_ARGUMENTS_LEN;
  }
  memcpy(lock_bytes, lock_bytes_seg.ptr, lock_bytes_seg.size);

  /* Load tx hash */
  unsigned char tx_hash[BLAKE2B_BLOCK_SIZE];
  len = BLAKE2B_BLOCK_SIZE;
  ret = ckb_load_tx_hash(tx_hash, &len, 0);
  if (ret != CKB_SUCCESS) {
    return ret;
  }
  if (len != BLAKE2B_BLOCK_SIZE) {
    return ERROR_IDENTITY_SYSCALL;
  }

  /* Prepare sign message */
  unsigned char message[BLAKE2B_BLOCK_SIZE];
  blake2b_state blake2b_ctx;
  blake2b_init(&blake2b_ctx, BLAKE2B_BLOCK_SIZE);
  blake2b_update(&blake2b_ctx, tx_hash, BLAKE2B_BLOCK_SIZE);

  /* Clear lock field to zero, then digest the first witness
   * lock_bytes_seg.ptr actually points to the memory in temp buffer
   * */
  memset((void *)lock_bytes_seg.ptr, 0, lock_bytes_seg.size);
  blake2b_update(&blake2b_ctx, (char *)&witness_len, sizeof(uint64_t));
  blake2b_update(&blake2b_ctx, temp, read_len);

  // remaining of first witness
  if (read_len < witness_len) {
    ret = load_and_hash_witness(&blake2b_ctx, read_len, 0,
                                CKB_SOURCE_GROUP_INPUT, false);
    if (ret != CKB_SUCCESS) {
      return ERROR_IDENTITY_SYSCALL;
    }
  }

  // Digest same group witnesses
  size_t i = 1;
  while (1) {
    ret =
        load_and_hash_witness(&blake2b_ctx, 0, i, CKB_SOURCE_GROUP_INPUT, true);
    if (ret == CKB_INDEX_OUT_OF_BOUND) {
      break;
    }
    if (ret != CKB_SUCCESS) {
      return ERROR_IDENTITY_SYSCALL;
    }
    i += 1;
  }

  // Digest witnesses that not covered by inputs
  i = (size_t)ckb_calculate_inputs_len();
  while (1) {
    ret = load_and_hash_witness(&blake2b_ctx, 0, i, CKB_SOURCE_INPUT, true);
    if (ret == CKB_INDEX_OUT_OF_BOUND) {
      break;
    }
    if (ret != CKB_SUCCESS) {
      return ERROR_IDENTITY_SYSCALL;
    }
    i += 1;
  }

  blake2b_final(&blake2b_ctx, message, BLAKE2B_BLOCK_SIZE);

  const uint8_t *pubkey = lock_bytes;
  const uint8_t *sig = pubkey + BLST_PUBKEY_SIZE;

  BLST_ERROR err = blst_verify(sig, pubkey, message, BLAKE2B_BLOCK_SIZE);
  if (err != 0) {
    return ERROR_BLST_VERIFY_FAILED;
  }

  unsigned char temp2[BLAKE2B_BLOCK_SIZE];
  blake2b_state blake2b_ctx2;
  blake2b_init(&blake2b_ctx2, BLAKE2B_BLOCK_SIZE);
  blake2b_update(&blake2b_ctx2, pubkey, BLST_PUBKEY_SIZE);
  blake2b_final(&blake2b_ctx2, temp2, BLAKE2B_BLOCK_SIZE);

  if (memcmp(pubkey_hash, temp2, BLAKE160_SIZE) != 0) {
    return ERROR_IDENTITY_PUBKEY_BLAKE160_HASH;
  }

  return 0;
}
