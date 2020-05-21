
use cita_trie::trie::{PatriciaTrie, Trie,TrieResult};
use cita_trie::codec;
use cita_trie;
use rocksdb::{Writable, DB};
use std::error;
use std::fmt;
use std::fmt::{Display,Formatter};
use std::sync::Arc;


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

/// Handle to RocksDb
#[derive(Clone)]
pub struct RocksDb {
    inner: Arc<DB>,
}

impl RocksDb {
    /// Create or open a database at the give path.  Will panic on error
    pub fn new(dir: &str) -> Self {
        match DB::open_default(dir) {
            Ok(db) => RocksDb {
                inner: Arc::new(db),
            },
            Err(reason) => panic!(reason),
        }
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
        D: cita_trie::db::DB,
{
    trie: PatriciaTrie<'db,C,D>
}

impl<'db,C,D> TrieDb<'db,C,D>
    where
        C: codec::NodeCodec,
        D: cita_trie::db::DB,
{
    pub fn new(db: &'db mut D,  codec: C) -> Self {
        TrieDb{
            trie: PatriciaTrie::new(db, codec)
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
    use crate::{RocksDb, TrieDb};
    use cita_trie::codec::RLPNodeCodec;

    #[test]
    fn test_rocksdb_trie_basics() {
        let test_dir = "data";
        let mut rocks_db = RocksDb::new(test_dir);
        let mut trie_db = TrieDb::new(&mut rocks_db,RLPNodeCodec::default());
        trie_db.insert(b"1",b"2");
    }
}