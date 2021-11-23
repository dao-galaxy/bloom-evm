
use ethereum_types::H256;
use keccak_hash::{KECCAK_NULL_RLP, keccak};
use hash_db::{HashDB, AsHashDB, Prefix};
use keccak_hasher::KeccakHasher;
use kvdb::DBValue;
use rlp::NULL_RLP;

#[inline]
fn combine_key<'a>(address_hash: &'a H256, key: &'a H256) -> H256 {
    let mut dst = key.clone();
    {
        let last_src: &[u8] = address_hash.as_bytes();
        let last_dst: &mut [u8] = dst.as_bytes_mut();

        for (k, a) in last_dst[12..].iter_mut().zip(&last_src[12..]) {
            *k ^= *a
        }
    }
    dst
}

#[derive(Debug, Clone)]
pub enum Factory {
    Mangled,
    Plain,
}

impl Default for Factory {
    fn default() -> Self { Factory::Mangled }
}

impl Factory {
    pub fn readonly<'db>(&self, db: &'db dyn HashDB<KeccakHasher, DBValue>, address_hash: H256) -> Box<dyn HashDB<KeccakHasher, DBValue> + 'db> {
        match *self {
            Factory::Mangled => Box::new(AccountDB::from_hash(db, address_hash)),
            Factory::Plain => Box::new(Wrapping(db)),
        }
    }

    pub fn create<'db>(&self, db: &'db mut dyn HashDB<KeccakHasher, DBValue>, address_hash: H256) -> Box<dyn HashDB<KeccakHasher, DBValue> + 'db> {
        match *self {
            Factory::Mangled => Box::new(AccountDBMut::from_hash(db, address_hash)),
            Factory::Plain => Box::new(WrappingMut(db)),
        }
    }
}

pub struct AccountDB<'db> {
    db: &'db dyn HashDB<KeccakHasher, DBValue>,
    address_hash: H256,
}

impl <'db> AccountDB<'db> {
    pub fn from_hash(db: &'db dyn HashDB<KeccakHasher,DBValue>, address_hash: H256) -> Self {
        AccountDB{
            db,address_hash,
        }
    }
}

impl <'db> AsHashDB<KeccakHasher, DBValue> for AccountDB<'db> {
    fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher,DBValue> {
        self
    }

    fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, DBValue> {
        self
    }
}

impl <'db> HashDB<KeccakHasher, DBValue> for AccountDB<'db> {
    fn get(&self, key: &H256, prefix: Prefix) -> Option<DBValue> {
        if key == &KECCAK_NULL_RLP {
            return Some(NULL_RLP.to_vec())
        }

        self.db.get(&combine_key(&self.address_hash, key), prefix)
    }

    fn contains(&self, key: &H256, prefix: Prefix) -> bool {
        if key == &KECCAK_NULL_RLP {
            return true;
        }
        self.db.contains(&combine_key(&self.address_hash, key), prefix)
    }

    fn insert(&mut self, _prefix: Prefix, _value: &[u8]) -> H256 {
        unimplemented!()
    }

    fn emplace(&mut self, _key: H256, _prefix: Prefix, _value: DBValue) {
        unimplemented!()
    }

    fn remove(&mut self, _key: &H256, _prefix: Prefix) {
        unimplemented!()
    }
}

pub struct AccountDBMut<'db> {
    db: &'db mut dyn HashDB<KeccakHasher, DBValue>,
    address_hash: H256,
}

impl <'db> AccountDBMut<'db> {
    pub fn from_hash(db: &'db mut dyn HashDB<KeccakHasher, DBValue>, address_hash: H256) -> Self {
        AccountDBMut{
            db,address_hash
        }
    }

    pub fn immutable(&'db self) -> AccountDB<'db> {
        AccountDB{
            db: self.db,
            address_hash: self.address_hash.clone()
        }
    }
}


impl <'db> HashDB<KeccakHasher, DBValue> for AccountDBMut<'db> {
    fn get(&self, key: &H256, prefix: Prefix) -> Option<DBValue> {
        if key == &KECCAK_NULL_RLP {
            return Some(NULL_RLP.to_vec());
        }

        self.db.get(&combine_key(&self.address_hash, key), prefix)
    }

    fn contains(&self, key: &H256, prefix: Prefix) -> bool {
        if key == &KECCAK_NULL_RLP {
            return true;
        }
        self.db.contains(&combine_key(&self.address_hash, key), prefix)
    }

    fn insert(&mut self, prefix: Prefix, value: &[u8]) -> H256 {
        if value == &NULL_RLP {
            return KECCAK_NULL_RLP.clone();
        }
        let k = keccak(value);
        let ak = combine_key(&self.address_hash, &k);
        self.db.emplace(ak, prefix, value.to_vec());
        k
    }

    fn emplace(&mut self, key: H256, prefix: Prefix, value: DBValue) {
        if key == KECCAK_NULL_RLP {
            return;
        }
        let key = combine_key(&self.address_hash, &key);
        self.db.emplace(key, prefix, value)
    }

    fn remove(&mut self, key: &H256, prefix: Prefix) {
        if key == &KECCAK_NULL_RLP {
            return;
        }
        let key = combine_key(&self.address_hash, key);
        self.db.remove(&key, prefix)
    }
}

impl<'db> AsHashDB<KeccakHasher, DBValue> for AccountDBMut<'db> {
    fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher, DBValue> { self }
    fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, DBValue> { self }
}

struct Wrapping<'db>(&'db dyn HashDB<KeccakHasher, DBValue>);

impl<'db> AsHashDB<KeccakHasher, DBValue> for Wrapping<'db> {
    fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher, DBValue> { self }
    fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, DBValue> { self }
}

impl<'db> HashDB<KeccakHasher, DBValue> for Wrapping<'db> {
    fn get(&self, key: &H256, prefix: Prefix) -> Option<DBValue> {
        if key == &KECCAK_NULL_RLP {
            return Some(NULL_RLP.to_vec());
        }
        self.0.get(key, prefix)
    }

    fn contains(&self, key: &H256, prefix: Prefix) -> bool {
        if key == &KECCAK_NULL_RLP {
            return true;
        }
        self.0.contains(key, prefix)
    }

    fn insert(&mut self, _prefix: Prefix, _value: &[u8]) -> H256 {
        unimplemented!()
    }

    fn emplace(&mut self, _key: H256, _prefix: Prefix, _value: DBValue) {
        unimplemented!()
    }

    fn remove(&mut self, _key: &H256, _prefix: Prefix) {
        unimplemented!()
    }
}

struct WrappingMut<'db>(&'db mut dyn HashDB<KeccakHasher, DBValue>);
impl<'db> AsHashDB<KeccakHasher, DBValue> for WrappingMut<'db> {
    fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher, DBValue> { self }
    fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, DBValue> { self }
}

impl<'db> HashDB<KeccakHasher, DBValue> for WrappingMut<'db>{
    fn get(&self, key: &H256, prefix: Prefix) -> Option<DBValue> {
        if key == &KECCAK_NULL_RLP {
            return Some(NULL_RLP.to_vec());
        }
        self.0.get(key, prefix)
    }

    fn contains(&self, key: &H256, prefix: Prefix) -> bool {
        if key == &KECCAK_NULL_RLP {
            return true;
        }
        self.0.contains(key, prefix)
    }

    fn insert(&mut self, prefix: Prefix, value: &[u8]) -> H256 {
        if value == &NULL_RLP {
            return KECCAK_NULL_RLP.clone();
        }
        self.0.insert(prefix, value)
    }

    fn emplace(&mut self, key: H256, prefix: Prefix, value: DBValue) {
        if key == KECCAK_NULL_RLP {
            return;
        }
        self.0.emplace(key, prefix, value)
    }

    fn remove(&mut self, key: &H256, prefix: Prefix) {
        if key == &KECCAK_NULL_RLP {
            return;
        }
        self.0.remove(key, prefix)
    }
}