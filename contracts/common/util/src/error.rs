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
    TypeScriptEmpty,
    ATCellShouldEmpty,
    BadScriptArgs,

    // selection contract
    OmniCheckpointCountError = 20,

    // checkpoint contract
    CheckpointCellError = 30,
    CheckpointCapacityMismatch,
    CheckpointDataMismatch,
    CheckpointRlpDataMismatch,
    CheckpointDataError,
    BadSudtDataFormat,
    WitnessLockError,
    ProposalRlpError,
    ProofRlpError,
    BlockHashMismatch,
    ActiveNodesNotEnough,
    NodesBitmapMismatch,
    ATAmountMismatch,
    StakeCellDepEmpty,
    ProposerAddressMismatch,
    WithdrawalATAmountMismatch,

    // stake contract
    CheckpointDataEmpty = 50,
    StakeDataEmpty,
    UnknownMode,
    AdminModeError,
    CompanionModeError,
    StakeInfoDumplicateError,
    StakeInfoMatchError,
    StakeInfoQuorumError,
    InvaidStakeATAmount,
    StakeATCellError,
    WithdrawCellError,
    WithdrawCellPeriodMismatch,
    WithdrawCellSudtMismatch,

    // withdrawal contract
    NodeIdentityEmpty = 70,
    CheckpointCelldepEmpty,
    BadCheckpointCelldep,
    BadWithdrawalData,
    BadWithdrawalPeriod,
    BadWithdrawalTypeHash,
    SomeWithdrawalTypeEmpty,
    TotalSudtAmountMismatch,

    // corsschain contract
    BadMetadataTypehash = 80,
    BadStakeTypehash,
    BadSUDTCell,
    MetadataCkbFeeError,
    MetadataSUDTFeeError,
    InsufficientCrosschainCKB,
    InsufficientCrosschainSUDT,
    UnexpectedBlsPubkey,
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
