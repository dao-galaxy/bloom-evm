mod account_cmd;
mod deposit_cmd;
mod contract_cmd;

use std::collections::BTreeMap;
use std::sync::Arc;


use structopt::StructOpt;
use account_cmd::AccountCmd;
use deposit_cmd::DepositCmd;
use contract_cmd::ContractCmd;

use ethereum_types::{U256, H160, H256};
use evm::backend::MemoryBackend as Backend;
use evm::backend::MemoryVicinity as Vicinity;
use evm::backend::MemoryAccount as Account;
use evm::executor::StackExecutor;
use evm::Config;
use bloom_state as state;
use ethtrie;
use kvdb_rocksdb::{Database, DatabaseConfig};
use trie_db::TrieSpec;
use trie_db::DBValue;
use state::AccountFactory;
use state::Factories;




#[derive(Debug, Clone, StructOpt)]
pub enum Subcommand {
	Account(AccountCmd),
	Deposit(DepositCmd),
	Contract(ContractCmd),
}

impl Subcommand {
	pub fn run(&self) {
		let vicinity = state::BackendVicinity {
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
		let data_path = "test-db";
		let mut config = DatabaseConfig::with_columns(state::COLUMN_COUNT);
		let database = Arc::new(Database::open(&config, data_path).unwrap());

		let root =
		{
			let default_ = [0u8;32].to_vec();
			let v =  database.get(state::COL_BLOCK,b"root");

			let root = v.unwrap_or(Some(default_.clone())).unwrap_or(default_.clone());
			root.clone()
		};
		let root = H256::from_slice(root.as_slice());
		//println!("get root={:?}",root.clone());


		let mut db = journaldb::new(database.clone(),journaldb::Algorithm::Archive,state::COL_STATE);
		let trie_layout = ethtrie::Layout::default();
		let trie_spec = TrieSpec::default();

		let gas_limit = 1000000u32;


		let trie_factory =  ethtrie::TrieFactory::new(trie_spec,trie_layout);
		let account_factory = AccountFactory::default();
		let factories = Factories{
			trie: trie_factory,
			accountdb: account_factory,
		};

		let mut backend = match root == H256::zero() {
			true => {
				state::State::new(&vicinity,db,factories)
			},
			false => {
				state::State::from_existing(root,&vicinity,db,factories).unwrap()
			}
		};

		match self {
			Subcommand::Account(cmd) => {
				cmd.run(&mut backend);
			}
			Subcommand::Deposit(cmd) => {
				cmd.run(&mut backend);
			}
			Subcommand::Contract(cmd) => {
				cmd.run(&mut backend);
			}
		}

		let root = backend.commit();
		{
			let mut transaction = database.transaction();
			transaction.put(state::COL_BLOCK, b"root", root.as_bytes());
			database.write(transaction).unwrap();
			//println!("set root={:?}",root.clone());

		}
	}
}
