
use std::convert::AsRef;

use common_types::{
    BlockNumber,
    block::{BlockHashList,Block},
    transaction::{TransactionBody,TransactionHashList},
    header::Header,
};
use ethereum_types::{H256};

use crate::db::Key;

pub struct BlockNumberKey([u8; 4]);

impl AsRef<[u8]> for BlockNumberKey {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl Key<BlockHashList> for BlockNumber {
    type Target = BlockNumberKey;

    fn key(&self) -> Self::Target {
        let mut result = [0u8; 4];
        result[0] = (self >> 24) as u8;
        result[1] = (self >> 16) as u8;
        result[2] = (self >> 8) as u8;
        result[3] = *self as u8;
        BlockNumberKey(result)
    }
}


impl Key<Block> for H256 {
    type Target = Self;

    fn key(&self) -> H256 {
        *self
    }
}

impl Key<TransactionBody> for H256 {
    type Target = Self;
    fn key(&self) -> H256 {
        *self
    }
}

impl Key<TransactionHashList> for H256 {
    type Target = Self;
    fn key(&self) -> H256 {
        *self
    }
}

impl Key<Header> for H256 {
    type Target = Self;
    fn key(&self) -> H256 {
        *self
    }
}