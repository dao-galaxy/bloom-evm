
use ethtrie::{Result as TrieResult};
use journaldb::JournalDB;
use ethereum_types::{Address, H256, U256, H160};
use evm::backend::{Basic,Log,Backend,ApplyBackend,Apply};

use crate::{BackendVicinity,Factories};
use crate::account::Account;
use trie_db::{Trie,TrieError};
use trie_db::NodeCodec;
use keccak_hash::{keccak, KECCAK_NULL_RLP};
use parity_bytes::Bytes;

use std::str::FromStr;


use std::collections::{HashSet, HashMap, BTreeMap};
use kvdb::DBTransaction;


pub struct State {
    vicinity: BackendVicinity,
    /// Backing database.
    db: Box<dyn JournalDB>,
    root: H256,
    factories: Factories,
    logs: Vec<Log>,
}

impl Clone for State {
    fn clone(&self) -> Self {
        State {
            vicinity: self.vicinity.clone(),
            db: self.db.boxed_clone(),
            root: self.root.clone(),
            factories: self.factories.clone(),
            logs: self.logs.clone(),
        }
    }
}

impl State {
    pub fn new(vicinity: BackendVicinity , db: Box<dyn JournalDB>, factories: Factories) -> Self {
        let root = ethtrie::RlpNodeCodec::hashed_null_node();

        let mut s = State{
            vicinity,
            db,
            root,
            factories,
            logs: vec![],
        };
        //Self::init_genesis(&mut s);
        s
    }

    pub fn setVicinity(&mut self,vicinity: BackendVicinity) {
        self.vicinity = vicinity;
    }

    fn init_genesis(&mut self) {
        // todo read from config file
        let from = H160::from_str("26d1ec50b4e62c1d1a40d16e7cacc6a6580757d5").expect("--address argument must be a valid address");
        let value = U256::from_dec_str("100000000000000000000000000").expect("--value argument must be a valid number");
        let nonce = U256::from_dec_str("0").expect("--nonce argument must be a valid number");

        let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

        applies.push(Apply::Modify {
            address: from.clone(),
            basic: Basic{
                balance: value,
                nonce,
            },
            code: None,
            storage: BTreeMap::new(),
            reset_storage: false,
        });

        self.apply(applies,Vec::new(),false);
        let root = self.commit();
        println!("init genesis state, root={:?}",root);
    }

    pub fn from_existing(root: H256, vicinity: BackendVicinity , db: Box<dyn JournalDB>, factories: Factories) -> TrieResult<State> {
        if !db.as_hash_db().contains(&root, hash_db::EMPTY_PREFIX) {
            return Err(Box::new(TrieError::InvalidStateRoot(root)));
        }

        let state = State {
            vicinity,
            db,
            root,
            factories,
            logs: vec![],
        };

        Ok(state)
    }

    pub fn get_account(&self,address: H160) -> Account {

        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        let mut acc = maybe_acc.unwrap_or_else(| | Account::new_basic(U256::zero(), U256::zero()));
        let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), acc.address_hash(&address));
        acc.cache_code(accountdb.as_hash_db());
        acc
    }

    // get code by address and code hash
    pub fn get_code(&self, address: H160, code_hash: H256) -> Option<Bytes> {

        let address_hash = {
            let hash = keccak(address);
            hash
        };
        let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), address_hash);

        match accountdb.get(&code_hash, hash_db::EMPTY_PREFIX) {
            Some(x) => {
                Some(x)
            },

            _ => {
                None
            }
        }
    }

    // get storage by address and storage root
    pub fn get_storage(&self, address: H160, storage_root: H256) -> BTreeMap<H256,H256> {

        let account = self.get_account(address.clone());
        let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), account.address_hash(&address));
        account.get_storage(accountdb.as_hash_db(), storage_root).unwrap()
    }

    pub fn commit(&mut self) -> H256 {
//        let res = self.db.drain_transaction_overlay().unwrap();
//        self.db.backing().write(res);
//        self.root.clone()
        let mut batch = DBTransaction::new();
        let id = H256::default();
        self.db.journal_under(&mut batch,0,&id);
        self.db.backing().write(batch);
        self.root.clone()
    }

    pub fn root(&self) -> H256 {
        self.root.clone()
    }

    pub fn storage_root(&self,address: H160) -> H256 {
        let db = &self.db.as_hash_db();
        let db_ret = self.factories.trie.readonly(db, &self.root);
        let ret = match db_ret {
            Ok(db) => {
                let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
                let maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
                let acc = maybe_acc.unwrap_or_else(|| Account::new_basic(U256::zero(), U256::zero()));
                acc.storage_root()
            },
            _ => {
                KECCAK_NULL_RLP
            }
        };
        ret
    }

    pub fn list_address(&self) -> Vec<Address> {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();
        let mut address_vec = vec![];
        let iter = db.iter().unwrap();
        for pair in iter {
            let (key, _val) = pair.unwrap();
            if key.len() == 20 {
                let addr = Address::from_slice(key.as_slice());
                address_vec.push(addr);
            }
        }
        address_vec
    }
}

impl Backend for State {
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
        let db_ret = self.factories.trie.readonly(db, &self.root);

        let ret = match db_ret {
            Ok(db) => {
                let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
                let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
                let acc = maybe_acc.unwrap_or_else(|| Account::new_basic(U256::zero(), U256::zero()));
                Basic{
                    balance: acc.balance().clone() ,
                    nonce: acc.nonce().clone(),
                }
            },
            _ => {
                Basic{
                    balance: U256::zero() ,
                    nonce: U256::zero(),
                }
            }
        };
        ret
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
        let mut acc = maybe_acc.unwrap_or_else(|| Account::new_basic(U256::zero(), U256::zero()));

        let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), acc.address_hash(&address));
        acc.cache_code(accountdb.as_hash_db());
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
//            let c_str = hex::encode(code.clone());
//            println!("adress={:?},code={:?}",address.clone(),c_str);
            return code;
        }
        vec![]

    }


    fn storage(&self, address: H160, index: H256) -> H256 {
        let db = &self.db.as_hash_db();
        let db = self.factories.trie.readonly(db, &self.root).unwrap();

        let from_rlp = |b: &[u8]| Account::from_rlp(b).expect("decoding db value failed");
        let mut maybe_acc = db.get_with(address.as_bytes(), from_rlp).unwrap();
        if let Some(acc) = maybe_acc {
            let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), acc.address_hash(&address));
            let code = match acc.storage_at(accountdb.as_hash_db(), &index) {
                Ok(v) => v,
                Err(e) => {
                    H256::zero()
                },
            };
            //println!("1.storage get|address={:?}, index={:?},value={:?}",address,index,code.clone());
            return code;
        }
        //println!("2.storage get| address={:?}, index={:?}, value={:?}",address,index,H256::zero());
        H256::zero()
    }

}


impl ApplyBackend for State {
    fn apply<A, I, L>(
        &mut self,
        values: A,
        logs: L,
        delete_empty: bool,
    ) where
        A: IntoIterator<Item=Apply<I>>,
        I: IntoIterator<Item=(H256, H256)>,
        L: IntoIterator<Item=Log>,
    {
        let mut deletedSet = HashSet::<H160>::new();
        let mut accounts: HashMap<H160,Account> = HashMap::new();
        for apply in values {
            match apply {
                Apply::Modify {
                    address, basic, code, storage, reset_storage,
                } => {
                    let is_empty = {

                        let mut account = {
                            self.get_account(address.clone())
                        };

                        account.set_balance(basic.balance);
                        account.set_nonce(basic.nonce);
                        if let Some(code) = code {
                            account.init_code(code);
                        }
                        for (index, value) in storage {
                            //println!("storage set|address={:?}, index={:?},value={:?}",address.clone(),index.clone(),value.clone());
                            account.set_storage(index,value);
                        }

                        let mut account_db = self.factories.accountdb.create(self.db.as_hash_db_mut(), account.address_hash(&address));
                        account.commit_storage(&self.factories.trie, account_db.as_hash_db_mut());

                        account.commit_code(account_db.as_hash_db_mut());
                        accounts.insert(address.clone(),account);
                        false
                    };

                    if is_empty && delete_empty {
                        deletedSet.insert(address.clone());
                    }
                },
                Apply::Delete {
                    address,
                } => {
                    deletedSet.insert(address.clone());
                },
            }
        }

        let mut trie = self.factories.trie.from_existing(self.db.as_hash_db_mut(), &mut self.root).unwrap();
        for address in deletedSet {
            trie.remove(address.as_bytes());
        }

        for (address,acc) in accounts {
            trie.insert(address.as_bytes(), &acc.rlp()).unwrap();
        }

        for log in logs {
            self.logs.push(log);
        }
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
    use std::str::FromStr;
    use evm::executor::StackExecutor;
    use evm::Config;
    use evm::backend::{Basic,Log,Backend,ApplyBackend,Apply};
    use bloom_db;







    #[test]
    fn test_state() {
        let dataPath = "test-db";
        let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);
        let database = Arc::new(Database::open(&config, dataPath).unwrap());
        let mut db = journaldb::new(database,journaldb::Algorithm::Archive,bloom_db::COL_STATE);
        let trie_layout = ethtrie::Layout::default();
        let trie_spec = TrieSpec::default();

        let gas_limit = 1000000u32;

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

        let mut state = State::new(vicinity,db,factories);
        let address = H160::from_str("0000000000000000000000000000000000000001").expect("not valid address");
        let value = U256::from_dec_str("10").expect("");

        {
            let config = Config::istanbul();
            let mut executor = StackExecutor::new(
                &state,
                gas_limit as usize,
                &config,
            );
            executor.deposit(address,value);
            let (values, logs) = executor.deconstruct();
            state.apply(values, logs, true);
        }

        let acc = state.get_account(address);
        assert_eq!(*acc.balance(),value);
        let root = state.commit();
        println!("root={}",root);

    }

}