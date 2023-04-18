SCHEMA_PATH := schemas
SCHEMA_DEST_PATH := src/generated

schema:
	moleculec --language rust --schema-file ${SCHEMA_PATH}/basic.mol > ${SCHEMA_DEST_PATH}/basic.rs
	moleculec --language rust --schema-file ${SCHEMA_PATH}/stake.mol > ${SCHEMA_DEST_PATH}/stake.rs
	cd src
	cargo fmt
