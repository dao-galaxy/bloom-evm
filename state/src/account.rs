
use std::cell::{Cell};
use std::sync::Arc;
use hash_db::HashDB;
use ethereum_types::{Address, H256, U256, H160, BigEndianHash};
use keccak_hash::{keccak, KECCAK_EMPTY, KECCAK_NULL_RLP};
use parity_bytes::{Bytes, ToPretty};
use rlp::{DecoderError, encode};
use kvdb::DBValue;
use ethtrie::{Result as TrieResult, SecTrieDB, TrieDB, TrieFactory};
use trie_db::{Recorder, Trie};
use keccak_hasher::KeccakHasher;


use std::fmt;
use std::collections::HashMap;

use crate::BasicAccount;

/// Boolean type for clean/dirty status.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Filth {
    /// Data has not been changed.
    Clean,
    /// Data has been changed.
    Dirty,
}

pub struct Account {
    balance: U256,
    nonce: U256,
    storage_root: H256,
    code_hash: H256,
    code_size: Option<usize>,
    code_cache: Arc<Bytes>,
    code_version: U256,
    code_filth: Filth,
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
            code_cache: Arc::new(vec![]),
            code_version: basic.code_version,
            code_filth: Filth::Clean,
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
            code_size: Some(0),
            code_cache: Arc::new(vec![]),
            code_version: U256::zero(),
            code_filth: Filth::Dirty,
            address_hash: Cell::new(None),
        }
    }

    pub fn from_rlp(rlp: &[u8]) -> Result<Account, DecoderError> {
        ::rlp::decode::<BasicAccount>(rlp).map(|ba| ba.into())
    }

    pub fn rlp(&self) -> Bytes {
        let basic = BasicAccount{
            nonce: self.nonce,
            balance: self.balance,
            storage_root: self.storage_root,
            code_hash: self.code_hash,
            code_version: self.code_version,
        };
        rlp::encode(&basic)
    }

    pub fn init_code(&mut self, code: Bytes) {
        self.code_hash = keccak(&code);
        self.code_cache = Arc::new(code);
        self.code_size = Some(self.code_cache.len());
        self.code_filth = Filth::Dirty;
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

    pub fn code_size(&self) -> Option<usize> {
        self.code_size.clone()
    }



    pub fn is_cached(&self) -> bool {
        !self.code_cache.is_empty() || (self.code_cache.is_empty() && self.code_hash == KECCAK_EMPTY)
    }

    #[must_use]
    pub fn cache_code(&mut self, db: &dyn HashDB<KeccakHasher, DBValue>) -> Option<Arc<Bytes>> {
        if self.is_cached() {
            return Some(self.code_cache.clone());
        }
        match db.get(&self.code_hash, hash_db::EMPTY_PREFIX) {
            Some(x) => {
                self.code_size = Some(x.len());
                self.code_cache = Arc::new(x);
                Some(self.code_cache.clone())
            },

            _ => {
                None
            }
        }
    }

    pub fn inc_nonce(&mut self) {
        self.nonce = self.nonce.saturating_add(U256::from(1u8));
    }
    pub fn add_balance(&mut self, x: &U256) {
        self.balance = self.balance.saturating_add(*x);
    }

    pub fn sub_balance(&mut self, x: &U256) {
        assert!(self.balance >= *x);
        self.balance = self.balance - *x;
    }

    pub fn storage_at(&self, db: &dyn HashDB<KeccakHasher, DBValue>, key: &H256) -> TrieResult<H256> {
        let db = SecTrieDB::new(&db, &self.storage_root)?;
        let decoder = |bytes: &[u8]| ::rlp::decode(&bytes).expect("decoding db value failed");
        let item: U256 = db.get_with(key.as_bytes(),decoder)?.unwrap_or_else(U256::zero);
        let value: H256 = BigEndianHash::from_uint(&item);
        Ok(value)
    }

    pub fn commit_storage(&mut self, trie_factory: &TrieFactory,
                          db: &mut dyn HashDB<KeccakHasher, DBValue>,
                          storage: &HashMap<H256,H256>) -> TrieResult<()> {
        let mut t = trie_factory.from_existing(db, &mut self.storage_root).unwrap();
        for(k, v) in storage {
            match v.is_zero() {
                true => t.remove(k.as_bytes())?,
                false => t.insert(k.as_bytes(), &encode(&v.into_uint()))?,
            };
        }

        Ok(())
    }



}

impl fmt::Debug for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Account").field("balance",&self.balance)
            .field("nonce",&self.nonce)
            .field("code",&self.code_hash)
            .finish()
    }
}