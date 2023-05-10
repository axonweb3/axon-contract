use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,

    // common
    BadWitnessInputType = 10,
    BadWitnessLock,
    SignatureMismatch,
    LockScriptEmpty,
    TypeScriptEmpty,
    InputOutputAtAmountNotEqual,
    ATCellShouldEmpty,
    BadScriptArgs,
    UnknownMode,
    WitnessLockError,
    UpdateDataError,

    // SMT
    MerkleProof,
    SmterrorCodeErrorInsufficientCapacity,
    SmterrorCodeErrorNotFound,
    SmterrorCodeErrorInvalidStack,
    SmterrorCodeErrorInvalidSibling,
    SmterrorCodeErrorInvalidProof,

    // selection contract
    OmniRewardCountError = 30,

    // stake AT type script
    StakeDataEmpty,
    MisMatchMetadataTypeId,
    UpdateModeError,
    BadSudtDataFormat,
    BadInaugurationEpoch,
    BadStakeChange,
    RedeemExceedLimit,
    BadStakeStakeChange,
    BadStakeRedeemChange,
    IllegalDefaultStakeInfo,
    IllegalInputStakeInfo,
    IllegalOutputStakeInfo,
    BadRedeem,
    BadElectionTime,
    OldStakeInfosErr,
    StaleStakeInfo,
    NewStakeInfosErr,
    BadInputStakeSmtCellCount,
    BadOutputStakeSmtCellCount,
    BadInputMetadataCellCount,
    BadOutputMetadataCellCount,
    MismatchXudtTypeId,

    // checkpoint
    CheckpointDataEmpty,
    CheckpointCellError,
    CheckpointCapacityMismatch,
    CheckpointDataMismatch,
    CheckpointDataError,
    ProofRlpError, // mock multisig verify

    // metadata
    MetadataNoStakeSmt,
    MetadataEpochWrong,
    MetadataSizeWrong,
    MetadataInputOutputMismatch,
    NotLastCheckpoint,
    StakerNonExist,
    StakerNotFound,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}

impl From<u32> for Error {
    fn from(err: u32) -> Self {
        match err {
            80 => Self::SmterrorCodeErrorInsufficientCapacity,
            81 => Self::SmterrorCodeErrorNotFound,
            82 => Self::SmterrorCodeErrorInvalidStack,
            83 => Self::SmterrorCodeErrorInvalidSibling,
            84 => Self::SmterrorCodeErrorInvalidProof,
            _ => panic!("unexpected smt error"),
        }
    }
}
