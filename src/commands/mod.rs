mod account_cmd;
mod deposit_cmd;
mod contract_cmd;
mod state_cmd;

use std::sync::Arc;


use structopt::StructOpt;
use account_cmd::AccountCmd;
use deposit_cmd::DepositCmd;
use contract_cmd::ContractCmd;
use state_cmd::StateCmd;

use ethereum_types::{U256, H160, H256};
use bloom_state as state;
use ethtrie;
use kvdb_rocksdb::{Database, DatabaseConfig};
use trie_db::TrieSpec;
use state::AccountFactory;
use state::Factories;
use std::str::FromStr; // !!! Necessary for H160::from_str(address).expect("...");
use hex;



#[derive(Debug, Clone, StructOpt)]
pub enum Subcommand {
	Account(AccountCmd),
	Deposit(DepositCmd),
	Contract(ContractCmd),
	State(StateCmd),
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
		let config = DatabaseConfig::with_columns(state::COLUMN_COUNT);
		let database = Arc::new(Database::open(&config, data_path).unwrap());

		let count =
			{
				let default_ = [0u8;32].to_vec();
				let v =  database.get(state::COL_BLOCK,b"root-count");

				let count = v.unwrap_or(Some(default_.clone())).unwrap_or(default_.clone());
				U256::from(count.as_slice())
			};

		let root =
			{
				let default_ = [0u8;32].to_vec();

				let mut arr = [0u8;32];
				count.to_big_endian(&mut arr);
				let v =  database.get(state::COL_BLOCK,&arr[..]);

				let root = v.unwrap_or(Some(default_.clone())).unwrap_or(default_.clone());
				root.clone()
			};


		let root = H256::from_slice(root.as_slice());
		//let root = H256::from_str("80df0689f530e11705a45c4f18a0da978902cc4b10b9728b244af8332b44ed2a").expect("");
		//println!("get root={:?}",root.clone());


		let db = journaldb::new(database.clone(),journaldb::Algorithm::Archive,state::COL_STATE);
		let trie_layout = ethtrie::Layout::default();
		let trie_spec = TrieSpec::Generic;

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

		let is_commit = match self {
			Subcommand::Account(cmd) => {
				cmd.run(&mut backend)
			}
			Subcommand::Deposit(cmd) => {
				cmd.run(&mut backend)
			}
			Subcommand::Contract(cmd) => {
				cmd.run(&mut backend)
			}
			Subcommand::State(cmd) => {
				cmd.run(database.clone(),count.clone())
			}
		};

		if is_commit {
			let root = backend.commit();
			let v = count.as_u64();
			let v = v + 1;
			let new_count = U256::from(v);
			let mut arr = [0u8;32];
			new_count.to_big_endian(&mut arr);

			let mut transaction = database.transaction();
			transaction.put(state::COL_BLOCK, b"root-count", &arr[..]);
			transaction.put(state::COL_BLOCK, &arr[..],root.as_bytes());
			database.write(transaction).unwrap();
			println!("set root={:?}",root.clone());
		}

		{
			let db = database.clone();
			for (k,v) in  db.iter(state::COL_BLOCK) {
				let kk = hex::encode(k);
				//println!("key={:?}",kk);
				//println!("val={:?}",v);
			}
		}
	}
}
