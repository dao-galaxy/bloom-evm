
use ethtrie;
use journaldb::JournalDB;
use ethereum_types::{Address, H256, U256, H160};
use evm::backend::{Basic,Log,Backend,ApplyBackend};

use crate::{BackendVicinity,Factories};
use crate::account::Account;
use trie_db::Trie;



pub struct State<'vicinity> {
    vicinity: &'vicinity BackendVicinity,
    /// Backing database.
    db: Box<dyn JournalDB>,
    root: H256,
    factories: Factories,
}

impl<'vicinity> State<'vicinity> {
    pub fn new(vicinity: &'vicinity BackendVicinity , db: Box<dyn JournalDB>, factories: Factories) -> Self {
        State{
            vicinity,
            db,
            root: H256::default(),
            factories,
        }
    }

}

impl <'vicinity> Backend for State<'vicinity> {
    fn gas_price(&self) -> U256 {self.vicinity.gas_price}
    fn origin(&self) -> H160 {self.vicinity.origin}
    fn block_hash(&self, number: U256) -> H256  {
        if number >= self.vicinity.block_number ||
            self.vicinity.block_number - number - U256::one() >= U256::from(self.vicinity.block_hashes.len()){
            H256::default()
        }else {
            let index = (self.vicinity.block_number - number - U256::one()).as_usize();
            self.vicinity.block_hashes[index]
        }
    }
    fn block_number(&self) -> U256 {self.vicinity.block_number}
    fn block_coinbase(&self) -> H160 {self.vicinity.block_coinbase}
    fn block_timestamp(&self) -> U256 {self.vicinity.block_timestamp}
    fn block_difficulty(&self) -> U256 {self.vicinity.block_difficulty}
    fn block_gas_limit(&self) -> U256 {self.vicinity.block_gas_limit}

    fn chain_id(&self) -> U256 {self.vicinity.chain_id}

    fn exists(&self, address: H160) -> bool {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        maybe_acc.is_some()
    }

    fn basic(&self,address: H160) -> Basic {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        let acc = maybe_acc.unwrap_or_else(|| Account::new_basic(U256::zero(), U256::zero()));
        Basic{
            balance: acc.balance().clone() ,
            nonce: acc.balance().clone(),
        }
    }

    fn code_hash(&self, address: H160) -> H256 {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        let acc = maybe_acc.unwrap_or_else(|| Account::new_basic(U256::zero(), U256::zero()));
        acc.code_hash()
    }

    fn code_size(&self, address: H160) -> usize {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        let acc = maybe_acc.unwrap_or_else(|| Account::new_basic(U256::zero(), U256::zero()));
        let code_size = match acc.code_size() {
            Some(s) => s,
            None => 0usize,
        };
        code_size
    }

    fn code(&self,address: H160) -> Vec<u8> {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        if let Some(ref mut acc) = maybe_acc.as_mut() {
            let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), acc.address_hash(&address));
            let code = match acc.cache_code(accountdb.as_hash_db()) {
                Some(c) => c.to_vec(),
                None => vec![],
            };
            return code;
        }
        vec![]

    }


    fn storage(&self, address: H160, index: H256) -> H256 {
        H256::default()
    }

}

#[cfg(test)]
mod tests {
    use crate::State;
    use crate::BackendVicinity;
    use kvdb_rocksdb::{Database, DatabaseConfig};
    use crate::{COLUMN_COUNT,COL_STATE};
    use ethtrie;
    use trie_db::TrieSpec;
    use std::sync::Arc;
    use ethereum_types::{Address, H256, U256, H160};
    use crate::account_db::Factory;
    use crate::Factories;





    #[test]
    fn test_state() {
        let dataPath = "test-db";
        let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);
        let database = Arc::new(Database::open(&config, dataPath).unwrap());
        let mut db = journaldb::new(database,journaldb::Algorithm::Archive,COL_STATE);
        let trie_layout = ethtrie::Layout::default();
        let trie_spec = TrieSpec::default();

        let vicinity = BackendVicinity {
            gas_price: U256::zero(),
            origin: H160::zero(),
            chain_id: U256::zero(),
            block_hashes: Vec::new(),
            block_number: U256::zero(),
            block_coinbase: H160::zero(),
            block_timestamp: U256::zero(),
            block_difficulty: U256::zero(),
            block_gas_limit: U256::zero(),
        };

        let trie_factory =  ethtrie::TrieFactory::new(trie_spec,trie_layout);
        let account_factory = Factory::default();
        let factories = Factories{
            trie: trie_factory,
            accountdb: account_factory,
        };

        let state = State::new(&vicinity,db,factories);
    }

}