
use std::cell::{Cell};
use std::sync::Arc;
use ethereum_types::{Address, H256, U256, H160};
use keccak_hash::{keccak, KECCAK_EMPTY, KECCAK_NULL_RLP};
use parity_bytes::{Bytes, ToPretty};
use rlp::{DecoderError, encode};

use crate::BasicAccount;



pub struct Account {
    balance: U256,
    nonce: U256,
    storage_root: H256,
    code_hash: H256,
    code_size: Option<usize>,
    code_version: U256,
    address_hash: Cell<Option<H256>>,
}

impl From<BasicAccount> for Account {
    fn from(basic: BasicAccount) -> Self {
        Account {
            balance: basic.balance,
            nonce: basic.nonce,
            storage_root: basic.storage_root,
            code_hash: basic.code_hash,
            code_size: None,
            code_version: basic.code_version,
            address_hash: Cell::new(None),
        }
    }
}

impl Account {
    pub fn new_basic(balance: U256, nonce: U256) -> Account {
        Account {
            balance,
            nonce,
            storage_root: KECCAK_NULL_RLP,
            code_hash: KECCAK_EMPTY,
            code_size: None,
            code_version: U256::zero(),
            address_hash: Cell::new(None),
        }
    }

    pub fn from_rlp(rlp: &[u8]) -> Result<Account, DecoderError> {
        ::rlp::decode::<BasicAccount>(rlp).map(|ba| ba.into())
    }


    pub fn address_hash(&self, address: &Address) -> H256 {
        let hash = self.address_hash.get();
        hash.unwrap_or_else(|| {
            let hash = keccak(address);
            self.address_hash.set(Some(hash.clone()));
            hash
        })
    }

    /// return the balance associated with this account.
    pub fn balance(&self) -> &U256 { &self.balance }

    /// return the nonce associated with this account.
    pub fn nonce(&self) -> &U256 { &self.nonce }

    /// return the code version associated with this account.
    pub fn code_version(&self) -> &U256 { &self.code_version }

    /// return the code hash associated with this account.
    pub fn code_hash(&self) -> H256 {
        self.code_hash.clone()
    }
}