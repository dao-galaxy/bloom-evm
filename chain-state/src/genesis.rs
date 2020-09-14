
use std::sync::Arc;
use crate::config::GenesisAccout;
use log::info;
use ethereum_types::{Address, H256, U256};
use std::str::FromStr;
use evm_executer;

pub fn init_genesis(db: Arc<dyn (::kvdb::KeyValueDB)>,accounts: Vec<GenesisAccout>) {
    info!("account number {:}",accounts.len());

    let mut init_data = Vec::<(Address,U256)>::new();
    for account in accounts.iter() {
        let address = Address::from_str(account.address.as_ref().unwrap().as_str()).expect("--address argument must be a valid address");
        let value = U256::from_dec_str(account.value.as_ref().unwrap().as_str()).expect("--value argument must be a valid number");
        init_data.push((address,value));
    }
    info!("genesis data:{:?}",init_data);
    evm_executer::init_genesis(db,init_data);
}