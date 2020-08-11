

extern crate alloc;

use ethereum_types::{H160,H256,U256};

mod state;
mod account_db;
mod account;

pub use state::State;
pub use account_db::Factory as AccountFactory;
use ethtrie;


#[derive(Clone,Debug,Eq,PartialEq,Default)]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BackendVicinity {
    pub gas_price: U256,
    pub origin: H160,
    pub chain_id: U256,
    pub block_hashes: Vec<H256>,
    pub block_number: U256,
    pub block_coinbase: H160,
    pub block_timestamp: U256,
    pub block_difficulty: U256,
    pub block_gas_limit: U256,
}


/// Basic account type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicAccount {
    /// Nonce of the account.
    pub nonce: U256,
    /// Balance of the account.
    pub balance: U256,
    /// Storage root of the account.
    pub storage_root: H256,
    /// Code hash of the account.
    pub code_hash: H256,
    /// Code version of the account.
    pub code_version: U256,
}

impl rlp::Encodable for BasicAccount {
    fn rlp_append(&self, stream: &mut rlp::RlpStream) {
        let use_short_version = self.code_version == U256::zero();

        match use_short_version {
            true => { stream.begin_list(4); }
            false => { stream.begin_list(5); }
        }

        stream.append(&self.nonce);
        stream.append(&self.balance);
        stream.append(&self.storage_root);
        stream.append(&self.code_hash);

        if !use_short_version {
            stream.append(&self.code_version);
        }
    }
}

impl rlp::Decodable for BasicAccount {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        let use_short_version = match rlp.item_count()? {
            4 => true,
            5 => false,
            _ => return Err(rlp::DecoderError::RlpIncorrectListLen),
        };

        Ok(BasicAccount {
            nonce: rlp.val_at(0)?,
            balance: rlp.val_at(1)?,
            storage_root: rlp.val_at(2)?,
            code_hash: rlp.val_at(3)?,
            code_version: if use_short_version {
                U256::zero()
            } else {
                rlp.val_at(4)?
            },
        })
    }
}


/// Collection of factories.
#[derive(Default, Clone)]
pub struct Factories {
    /// factory for tries.
    pub trie: ethtrie::TrieFactory,
    /// factory for account databases.
    pub accountdb: AccountFactory,
}



