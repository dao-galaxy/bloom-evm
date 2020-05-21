
use cita_trie::trie::{PatriciaTrie, Trie,TrieResult};
use cita_trie::codec;
use cita_trie;
use cita_trie::db::DB;
use rocksdb;
use rocksdb::Writable;
use std::error;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::sync::Arc;
use ethereum_types::{H160, H256, U256};


#[derive(Debug)]
pub struct RocksDbError(pub String);

impl From<String> for RocksDbError {
    fn from(err: String) -> RocksDbError {
        RocksDbError(err)
    }
}
impl Display for RocksDbError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "RocksDb error: {}", self.0)
    }
}
impl error::Error for RocksDbError {
    fn description(&self) -> &str {
        &self.0
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

pub trait Root {
    fn set_root(&mut self, root: &[u8]) -> Result<(),RocksDbError>;
    fn get_root(&mut self) -> Result<Option<Vec<u8>>, RocksDbError>;
}

/// Handle to RocksDb
#[derive(Clone)]
pub struct RocksDb {
    inner: Arc<rocksdb::DB>,
}

impl RocksDb {
    /// Create or open a database at the give path.  Will panic on error
    pub fn new(dir: &str) -> Self {
        match rocksdb::DB::open_default(dir) {
            Ok(db) => RocksDb {
                inner: Arc::new(db),
            },
            Err(reason) => panic!(reason),
        }
    }
}

impl Root for RocksDb {
    /// set root
    fn set_root(&mut self, root: &[u8]) -> Result<(),RocksDbError> {
        self.insert(b"root",root)
    }

    /// get root
    fn get_root(&mut self) -> Result<Option<Vec<u8>>, RocksDbError> {
        self.get(b"root")
    }
}

// Implemented to satisfy the DB Trait
impl fmt::Debug for RocksDb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rocksdb trie stores")
    }
}

impl cita_trie::db::DB for RocksDb {
    type Error = RocksDbError;
    /// Get a value from the database.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        match self.inner.get(key) {
            Ok(Some(val)) => Ok(Some(val.to_owned())),
            Err(reason) => Err(RocksDbError::from(reason)),
            _ => Ok(None),
        }
    }

    /// Insert a key value
    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        self.inner
            .put(key, value)
            .map_err(|r| RocksDbError::from(r))
    }

    /// Check if a key is in the database
    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        if let Ok(Some(_)) = self.get(key) {
            return Ok(true);
        }
        return Ok(false);
    }

    /// Remove a key/value pair
    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        self.inner.delete(key).map_err(|r| RocksDbError::from(r))
    }
}


pub struct TrieDb<'db,C,D>
    where
        C: codec::NodeCodec,
        D: cita_trie::db::DB + Root,
{
    trie: PatriciaTrie<'db,C,D>
}

impl<'db,C,D> TrieDb<'db,C,D>
    where
        C: codec::NodeCodec,
        D: cita_trie::db::DB + Root,
{
    pub fn new(db: &'db mut D,  codec: C) -> Self {
        match db.get_root() {
            Ok(t) => {
                match t {
                    Some(root) => {
                        let r = codec.decode_hash(root.as_slice(),true);
                        Self::from(db,codec,&r).unwrap()
                    },
                    None => {
                        TrieDb{
                            trie: PatriciaTrie::new(db, codec)
                        }
                    }
                }
            }
            Err(_) => {
                TrieDb{
                    trie: PatriciaTrie::new(db, codec)
                }
            }
        }
    }

    pub fn from(db: &'db mut D, codec: C, root: &C::Hash) -> TrieResult<Self,C,D> {
        match PatriciaTrie::from(db,codec,root) {
            Ok(trie) => Ok(TrieDb{trie}),
            Err(e) => Err(e),
        }
    }

    fn get(&self, key: &[u8]) -> TrieResult<Option<Vec<u8>>, C, D> {
        self.trie.get(key)
    }

    fn contains(&self, key: &[u8]) -> TrieResult<bool, C, D> {
        self.trie.contains(key)
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) -> TrieResult<(), C, D> {
        self.trie.insert(key,value)
    }

    fn remove(&mut self, key: &[u8]) -> TrieResult<bool, C, D> {
        self.trie.remove(key)
    }

    fn root(&mut self) -> TrieResult<C::Hash, C, D> {
        self.trie.root()
    }
}

#[cfg(test)]
mod tests {
    use crate::{RocksDb, TrieDb, Root};
    use cita_trie::codec::RLPNodeCodec;

    #[test]
    fn test_rocksdb_trie_basics() {
        let test_dir = "data";
        let mut rocks_db = RocksDb::new(test_dir);
        let mut trie_db = TrieDb::new(&mut rocks_db,RLPNodeCodec::default());
        trie_db.insert(b"1",b"2");
        let root = trie_db.root().unwrap();

        let test_dir = "data";
        let mut td = TrieDb::new(&mut rocks_db,RLPNodeCodec::default());
        let ret = td.get(b"1");
        assert_eq!(ret.unwrap_or(Some(Vec::new())),None);

        let test_dir = "data";
        let mut td = TrieDb::from(&mut rocks_db,RLPNodeCodec::default(),&root).unwrap();
        let ret = td.get(b"1");
        assert_eq!(ret.unwrap().unwrap().as_slice(),b"2");
    }

    #[test]
    fn test_rocksdb_trie_root() {
        let test_dir = "data";
        let mut rocks_db = RocksDb::new(test_dir);
        let mut trie_db = TrieDb::new(&mut rocks_db,RLPNodeCodec::default());
        trie_db.insert(b"0001",b"0002");
        let root = trie_db.root().unwrap();
        rocks_db.set_root(&root);

        let mut td = TrieDb::new(&mut rocks_db,RLPNodeCodec::default());
        let ret = td.get(b"0001");
        assert_eq!(ret.unwrap().unwrap().as_slice(),b"0002");
    }
}