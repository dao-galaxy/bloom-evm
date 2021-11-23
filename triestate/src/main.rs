extern crate ethereum_types;
extern crate hash_db;
extern crate keccak_hasher;
extern crate rlp;


use memory_db::{MemoryDB, PrefixedKey};
use keccak_hasher::KeccakHasher;
use trie_db::DBValue;
use ethtrie::trie::TrieMut;
use hex_literal::hex;
use trie_db::Trie;
use hex as hhex;

use kvdb_rocksdb::{Database, DatabaseConfig};


use ethtrie;
use trie_db::TrieSpec;
use std::sync::Arc;
use ethereum_types::{Address, H256, U256, H160};
use std::str::FromStr;


fn main() {
    // before calling main, rm -rf test-db
    write();
    read();
}



fn read(){
    let dataPath = "test-db";
    let COLUMN_COUNT = 9;
    let COL_STATE = 0;
    let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);
    let database = Arc::new(Database::open(&config, dataPath).unwrap());

    {
        let db = database.clone();
        let iter = db.iter(COL_STATE);
        for (k,v) in  db.iter(COL_STATE) {
            println!("key={:?}",k);
            println!("val={:?}",v);
        }
    }

    let mut db = journaldb::new(database,journaldb::Algorithm::Archive,COL_STATE);


    let root = hhex::decode("012a15587e70dfb8a4cecdd835fbebad681ad9eea10a874a4a367f2be65965fb").expect("");

    let mut root = H256::from_slice(root.as_slice());
    {
        let db = &db.as_hash_db();
        let t = ethtrie::TrieDB::new(db, &root).unwrap();
        println!("{:?}",t.get(b"foo").unwrap().unwrap());

    }


    let root = hhex::decode("21ac08246963dc92c4cf6181d38731dea96ef3ce7df6e790a6f9bc072d2ffa47").expect("");
    let mut root = H256::from_slice(root.as_slice());
    {
        let db = &db.as_hash_db();
        let t = ethtrie::TrieDB::new(db, &root).unwrap();
        println!("{:?}",t.get(b"foo").unwrap().unwrap());

    }
}

fn write(){
    let dataPath = "test-db";
    let COLUMN_COUNT = 9;
    let COL_STATE = 0;
    let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);
    let database = Arc::new(Database::open(&config, dataPath).unwrap());

    let mut db = journaldb::new(database,journaldb::Algorithm::EarlyMerge,COL_STATE);

    let mut root = H256::zero();

    {
        let mut triedbmut = ethtrie::TrieDBMut::new(db.as_hash_db_mut(), &mut root);
        triedbmut.insert(b"foo",b"hello").unwrap();
        let r = triedbmut.root();
        println!("{:?}",r);
    }
    {
        let res = db.drain_transaction_overlay().unwrap();
        db.backing().write(res);

    }

    let mut v = Vec::new();

    {
        let db = &db.as_hash_db();
        let t = ethtrie::TrieDB::new(db, &root).unwrap();

        let value = t.get(b"foo").unwrap().unwrap();
        for i in value {
            v.push(i);
        }

        let value1 = b"jack";
        for i in &value1[..] {
            v.push(*i);
        }
    }

    {
        let mut triedbmut = ethtrie::TrieDBMut::new(db.as_hash_db_mut(), &mut root);

        triedbmut.insert(b"foo", v.as_slice()).unwrap();
        let r = triedbmut.root();
        println!("{:?}",r);
    }
    {
        let res = db.drain_transaction_overlay().unwrap();
        db.backing().write(res);
    }

    let ret = hhex::encode(root.as_bytes());
    println!("{:?}",ret)
}

fn write2(){
    let dataPath = "test-db";
    let COLUMN_COUNT = 9;
    let COL_STATE = 0;
    let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);
    let database = Arc::new(Database::open(&config, dataPath).unwrap());

    let mut db = journaldb::new(database,journaldb::Algorithm::Archive,COL_STATE);

    let root = hhex::decode("21ac08246963dc92c4cf6181d38731dea96ef3ce7df6e790a6f9bc072d2ffa47").expect("");
    let mut root = H256::from_slice(root.as_slice());

    {
        let mut triedbmut = ethtrie::TrieDBMut::new(db.as_hash_db_mut(), &mut root);
        triedbmut.insert(b"foo",b"helloff").unwrap();
        let r = triedbmut.root();
        println!("{:?}",r);
    }
    {
        let res = db.drain_transaction_overlay().unwrap();
        db.backing().write(res);

    }

    let ret = hhex::encode(root.as_bytes());
    println!("{:?}",ret)
}

//#[cfg(test)]
//mod tests {
//
//    use memory_db::{MemoryDB, PrefixedKey};
//    use keccak_hasher::KeccakHasher;
//    use trie_db::DBValue;
//    use patricia_trie_ethereum as ethtrie;
//    use ethtrie::trie::TrieMut;
//    use hex_literal::hex;
//    use trie_db::Trie;
//    use std::sync::Arc;
//    use hex as hhex;
//
//    use ethereum_types::{H160, H256, U256};
//    use kvdb_rocksdb::{Database, DatabaseConfig};
//
//    #[test]
//    fn test_rocksdb(){
//        let dataPath = "test-kvstorage";
//        let COLUMN_COUNT = 9;
//        let COL_STATE = 0;
//        let mut config = DatabaseConfig::with_columns(COLUMN_COUNT);
//        let database = Arc::new(Database::open(&config, dataPath).unwrap());
//
//        let mut kvstorage = journaldb::new(database,journaldb::Algorithm::Archive,COL_STATE);
//
//
//        let mut root = H256::zero();
//
//        {
//            let mut triedbmut = ethtrie::TrieDBMut::new(kvstorage.as_hash_db_mut(), &mut root);
//            triedbmut.insert(b"foo", b"bar").unwrap();
//            triedbmut.insert(b"fog", b"b").unwrap();
//            triedbmut.insert(b"fot", &vec![0u8;33][..]).unwrap();
//        }
//        {
//            let kvstorage = &kvstorage.as_hash_db();
//            let t = ethtrie::TrieDB::new(kvstorage, &root).unwrap();
//            assert!(t.contains(b"foo").unwrap());
//            assert!(t.contains(b"fog").unwrap());
//            assert_eq!(t.get(b"foo").unwrap().unwrap(), b"bar".to_vec());
//            assert_eq!(t.get(b"fog").unwrap().unwrap(), b"b".to_vec());
//            assert_eq!(t.get(b"fot").unwrap().unwrap(), vec![0u8; 33]);
//        }
//
//        let root1 = root.clone();
//        {
//            let mut triedbmut = ethtrie::TrieDBMut::new(kvstorage.as_hash_db_mut(), &mut root);
//            triedbmut.insert(b"foo", b"bar1").unwrap();
//        }
//        {
//            let kvstorage = &kvstorage.as_hash_db();
//            let t = ethtrie::TrieDB::new(kvstorage, &root).unwrap();
//            assert_eq!(t.get(b"foo").unwrap().unwrap(), b"bar1".to_vec());
//        }
//
//        {
//            let kvstorage = &kvstorage.as_hash_db();
//
//            let t = ethtrie::TrieDB::new(kvstorage, &root1).unwrap();
//            assert_eq!(t.get(b"foo").unwrap().unwrap(), b"bar".to_vec());
//        }
//
//        let ret = hhex::encode(root1.as_bytes());
//        println!("{:?}",ret)
//
//    }
//
//    #[test]
//    fn test_inline_encoding_branch() {
//        let mut memdb = journaldb::new_memory_db();
//        let mut root = H256::zero();
//        {
//            let mut triedbmut = ethtrie::TrieDBMut::new(&mut memdb, &mut root);
//            triedbmut.insert(b"foo", b"bar").unwrap();
//            triedbmut.insert(b"fog", b"b").unwrap();
//            triedbmut.insert(b"fot", &vec![0u8;33][..]).unwrap();
//        }
//        let t = ethtrie::TrieDB::new(&memdb, &root).unwrap();
//        assert!(t.contains(b"foo").unwrap());
//        assert!(t.contains(b"fog").unwrap());
//        assert_eq!(t.get(b"foo").unwrap().unwrap(), b"bar".to_vec());
//        assert_eq!(t.get(b"fog").unwrap().unwrap(), b"b".to_vec());
//        assert_eq!(t.get(b"fot").unwrap().unwrap(), vec![0u8;33]);
//
//        let root1 = root.clone();
//        {
//            let mut triedbmut = ethtrie::TrieDBMut::new(&mut memdb, &mut root);
//            triedbmut.insert(b"foo", b"bar1").unwrap();
//        }
//        let t = ethtrie::TrieDB::new(&memdb, &root).unwrap();
//        assert_eq!(t.get(b"foo").unwrap().unwrap(), b"bar1".to_vec());
//
//        let t = ethtrie::TrieDB::new(&memdb, &root1).unwrap();
//        assert_eq!(t.get(b"foo").unwrap().unwrap(), b"bar".to_vec());
//    }
//
//
//}
