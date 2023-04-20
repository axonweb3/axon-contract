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
	cd src
	cargo fmt
