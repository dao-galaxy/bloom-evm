
use std::convert::AsRef;
use std::hash::Hash;
use kvdb::{DBTransaction, KeyValueDB};
use rlp;


// Database column indexes.
/// Column for State
pub const COL_STATE: u32 = 0;
/// Column for Block headers
pub const COL_HEADERS: u32 = 1;
/// Column for Block bodies
pub const COL_BODIES: u32 = 2;
/// Column for Extras
pub const COL_EXTRA: u32 = 3;
/// Column for Traces
pub const COL_TRACE: u32 = 4;
/// Column for the accounts existence bloom filter.
#[deprecated(since = "3.0.0", note = "Accounts bloom column is deprecated")]
pub const COL_ACCOUNT_BLOOM: u32 = 5;
/// Column for general information from the local node which can persist.
pub const COL_NODE_INFO: u32 = 6;
/// Column for the light client chain.
pub const COL_LIGHT_CHAIN: u32 = 7;
/// Column for the private transactions state.
pub const COL_PRIVATE_TRANSACTIONS_STATE: u32 = 8;
/// Column for block
pub const COL_BLOCK: u32 = 9;
/// Number of columns in DB
pub const NUM_COLUMNS: u32 = 10;



/// Should be used to get database key associated with given value.
pub trait Key<T> {
    /// The db key associated with this value.
    type Target: AsRef<[u8]>;

    /// Returns db key.
    fn key(&self) -> Self::Target;
}


/// Should be used to write value into database.
pub trait Writable {
    /// Writes the value into the database.
    fn write<T, R>(&mut self, col: u32, key: &dyn Key<T, Target = R>, value: &T) where T: rlp::Encodable, R: AsRef<[u8]>;

    /// Deletes key from the database.
    fn delete<T, R>(&mut self, col: u32, key: &dyn Key<T, Target = R>) where T: rlp::Encodable, R: AsRef<[u8]>;
}

/// Should be used to read values from database.
pub trait Readable {
    /// Returns value for given key.
    fn read<T, R>(&self, col: u32, key: &dyn Key<T, Target = R>) -> Option<T> where
        T: rlp::Decodable,
        R: AsRef<[u8]>;

    /// Returns true if given value exists.
    fn exists<T, R>(&self, col: u32, key: &dyn Key<T, Target = R>) -> bool where R: AsRef<[u8]>;

}

impl Writable for DBTransaction {
    fn write<T, R>(&mut self, col: u32, key: &dyn Key<T, Target = R>, value: &T) where T: rlp::Encodable, R: AsRef<[u8]> {
        self.put(col, key.key().as_ref(), &rlp::encode(value));
    }

    fn delete<T, R>(&mut self, col: u32, key: &dyn Key<T, Target = R>) where T: rlp::Encodable, R: AsRef<[u8]> {
        self.delete(col, key.key().as_ref());
    }
}

impl<KVDB: KeyValueDB + ?Sized> Readable for KVDB {
    fn read<T, R>(&self, col: u32, key: &dyn Key<T, Target = R>) -> Option<T>
        where T: rlp::Decodable, R: AsRef<[u8]> {
        self.get(col, key.key().as_ref())
            .expect(&format!("db get failed, key: {:?}", key.key().as_ref()))
            .map(|v| rlp::decode(&v).expect("decode db value failed") )

    }

    fn exists<T, R>(&self, col: u32, key: &dyn Key<T, Target = R>) -> bool where R: AsRef<[u8]> {
        let result = self.get(col, key.key().as_ref());

        match result {
            Ok(v) => v.is_some(),
            Err(err) => {
                panic!("db get failed, key: {:?}, err: {:?}", key.key().as_ref(), err);
            }
        }
    }
}