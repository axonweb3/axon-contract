pub mod basic;
pub mod basic_reader;
pub mod checkpoint;
pub mod checkpoint_reader;
pub mod delegate;
pub mod delegate_reader;
pub mod metadata;
pub mod metadata_reader;
pub mod reward;
pub mod selection;
pub mod selection_reader;
pub mod stake;
pub mod stake_reader;
pub mod withdraw;

// use molecule::{bytes::Bytes, prelude::Entity};

// macro_rules! impl_conversion {
//     ($type_: ident, $uint: ty) => {
//         impl From<basic::$type_> for $uint {
//             fn from(val: basic::$type_) -> $uint {
//                 let mut buf = [0u8; std::mem::size_of::<$uint>()];
//                 buf.copy_from_slice(&val.raw_data());
//                 <$uint>::from_le_bytes(buf)
//             }
//         }

//         impl From<$uint> for basic::$type_ {
//             fn from(val: $uint) -> basic::$type_ {
//                 basic::$type_::new_unchecked(Bytes::from(val.to_le_bytes().to_vec()))
//             }
//         }
//     };

//     ($type_: ident, $h: path, $len: expr) => {
//         impl From<basic::$type_> for $h {
//             fn from(val: basic::$type_) -> $h {
//                 let mut buf = [0u8; $len];
//                 buf.copy_from_slice(&val.as_slice()[0..$len]);
//                 $h(buf)
//             }
//         }

//         impl From<$h> for basic::$type_ {
//             fn from(val: $h) -> basic::$type_ {
//                 basic::$type_::new_unchecked(Bytes::from(val.0.to_vec()))
//             }
//         }
//     };
// }

// impl_conversion!(Uint16, u16);
// impl_conversion!(Uint32, u32);
// impl_conversion!(Uint64, u64);
// impl_conversion!(Uint128, u128);

// impl_conversion!(Byte20, ckb_types::H160, 20);
// impl_conversion!(Byte20, ethereum_types::H160, 20);

// impl_conversion!(Identity, ckb_types::H160, 20);
// impl_conversion!(Identity, ethereum_types::H160, 20);

// impl_conversion!(Byte32, ckb_types::H256, 32);
// impl_conversion!(Byte32, ethereum_types::H256, 32);
