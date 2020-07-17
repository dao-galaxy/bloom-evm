

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


