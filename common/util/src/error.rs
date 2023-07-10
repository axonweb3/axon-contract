use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,
    // type id
    InvalidTypeIDCellNum,
    TypeIDNotMatch,
    ArgsLengthNotEnough,

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
    WitnessInputTypeError,
    UpdateDataError,

    // SMT
    MerkleProof = 30,
    SmterrorCodeErrorInsufficientCapacity,
    SmterrorCodeErrorNotFound,
    SmterrorCodeErrorInvalidStack,
    SmterrorCodeErrorInvalidSibling,
    SmterrorCodeErrorInvalidProof,
    SmterrorCodeErrorUpdate,
    SmterrorCodeErrorVerify,

    // stake smt
    StakeSmtTypeIdMismatch,
    StakeSmtUpdateDataError,
    StakeSmtVerifyOldError,
    StakeSmtVerifySelectionError,

    // selection contract
    OmniRewardCountError,

    // stake AT type script
    StakeDataEmpty = 50,
    StakeL2AddrMismatch,
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

    // delegate
    FirstRedeemError = 80,
    BadDelegateChange,
    StaleDelegateInfo,
    IllegalDefaultDelegateInfo,
    DelegateSmtTypeIdMismatch,
    DelegateSmtVerifySelectionError,

    // checkpoint
    CheckpointDataEmpty = 90,
    CheckpointCellError,
    CheckpointCapacityMismatch,
    CheckpointDataMismatch,
    CheckpointDataError,
    ProofRlpError, // mock multisig verify
    CheckpointLackOfQuorum,
    CheckpointProposalHashMismatch,

    // metadata
    MetadataNoStakeSmt = 100,
    MetadataEpochWrong,
    MetadataSizeWrong,
    MetadataInputOutputMismatch,
    NotLastCheckpoint,
    StakerNonExist,
    StakerNotFound,
    MetadataNotFound,
    MetadataProposeCountVerifyFail,

    // withdraw
    WrongOutWithdrawArraySize = 110,
    WrongLockEpoch,
    WrongOutWithdrawEpoch,
    WrongOutWithdraw,
    WrongIncreasedXudt,
    WithdrawTotalAmountError,
    OutLessThanIn,
    WithdrawUpdateDataError,
    BadUnstake,
    WithdrawDataEmpty,

    // reward
    RewardWrongAmount,
    RewardProposeCountBottomFail,
    RewardProposeCountTopFail,
    RewardStakeAmountBottomFail,
    RewardStakeAmountTopFail,

    // requirement
    CommissionRateTooLarge,

    // molecule::error::VerificationError
    TotalSizeNotMatch = -10,
    HeaderIsBroken,
    UnknownItem,
    OffsetsNotMatch,
    FieldCountNotMatch,
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

impl From<ckb_type_id::Error> for Error {
    fn from(err: ckb_type_id::Error) -> Self {
        match err {
            ckb_type_id::Error::Syscall(err) => err.into(),
            ckb_type_id::Error::Native(err) => match err {
                ckb_type_id::TypeIDError::InvalidTypeIDCellNum => Self::InvalidTypeIDCellNum,
                ckb_type_id::TypeIDError::TypeIDNotMatch => Self::TypeIDNotMatch,
                ckb_type_id::TypeIDError::ArgsLengthNotEnough => Self::ArgsLengthNotEnough,
            },
        }
    }
}

impl From<molecule::error::VerificationError> for Error {
    fn from(err: molecule::error::VerificationError) -> Self {
        match err {
            molecule::error::VerificationError::TotalSizeNotMatch(_, _, _) => {
                Self::TotalSizeNotMatch
            }
            molecule::error::VerificationError::HeaderIsBroken(_, _, _) => Self::HeaderIsBroken,
            molecule::error::VerificationError::UnknownItem(_, _, _) => Self::UnknownItem,
            molecule::error::VerificationError::OffsetsNotMatch(_) => Self::OffsetsNotMatch,
            molecule::error::VerificationError::FieldCountNotMatch(_, _, _) => {
                Self::FieldCountNotMatch
            }
        }
    }
}
