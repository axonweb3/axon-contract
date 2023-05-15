SCHEMA_PATH := schemas
SCHEMA_DEST_PATH := src/generated

schema:
	moleculec --language rust --schema-file ${SCHEMA_PATH}/basic.mol > ${SCHEMA_DEST_PATH}/basic.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/selection.mol > ${SCHEMA_DEST_PATH}/selection.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/checkpoint.mol > ${SCHEMA_DEST_PATH}/checkpoint.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/metadata.mol > ${SCHEMA_DEST_PATH}/metadata.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/stake.mol > ${SCHEMA_DEST_PATH}/stake.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/delegate.mol > ${SCHEMA_DEST_PATH}/delegate.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/reward.mol > ${SCHEMA_DEST_PATH}/reward.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/withdraw.mol > ${SCHEMA_DEST_PATH}/withdraw.rs
	cd src
	cargo fmt

# moleculec-c2 create reader files
mc2:
	moleculec --language - --schema-file ${SCHEMA_PATH}/selection.mol --format json > ${SCHEMA_PATH}/selection.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/selection.json | rustfmt > ${SCHEMA_DEST_PATH}/selection_reader.rs
	moleculec --language - --schema-file ${SCHEMA_PATH}/metadata.mol --format json > ${SCHEMA_PATH}/metadata.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/metadata.json | rustfmt > ${SCHEMA_DEST_PATH}/metadata_reader.rs
	moleculec --language - --schema-file ${SCHEMA_PATH}/checkpoint.mol --format json > ${SCHEMA_PATH}/checkpoint.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/checkpoint.json | rustfmt > ${SCHEMA_DEST_PATH}/checkpoint_reader.rs 	
	moleculec --language - --schema-file ${SCHEMA_PATH}/stake.mol --format json > ${SCHEMA_PATH}/stake.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/stake.json | rustfmt > ${SCHEMA_DEST_PATH}/stake_reader.rs	
	moleculec --language - --schema-file ${SCHEMA_PATH}/delegate.mol --format json > ${SCHEMA_PATH}/delegate.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/delegate.json | rustfmt > ${SCHEMA_DEST_PATH}/delegate_reader.rs
	moleculec --language - --schema-file ${SCHEMA_PATH}/withdraw.mol --format json > ${SCHEMA_PATH}/withdraw.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/withdraw.json | rustfmt > ${SCHEMA_DEST_PATH}/withdraw_reader.rs
	moleculec --language - --schema-file ${SCHEMA_PATH}/reward.mol --format json > ${SCHEMA_PATH}/reward.json
	/root/git/moleculec-c2/target/release/moleculec-c2  --rust --input ${SCHEMA_PATH}/reward.json | rustfmt > ${SCHEMA_DEST_PATH}/reward_reader.rs