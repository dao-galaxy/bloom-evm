use structopt::StructOpt;
use evm::backend::{ApplyBackend};
use evm::backend::{Apply,Basic};
use evm::executor::StackExecutor;
use evm::Config;
use ethereum_types::{H160, H256, U256};
use bloom_state::State;
use std::collections::BTreeMap;
use std::str::FromStr; // !!! Necessary for H160::from_str(address).expect("...");

use crate::executer;

// target/debug/bloom-evm account create --address 59a5208b32e627891c389ebafc644145224006e8 --value 10 --nonce 12
// target/debug/bloom-evm account query --address 59a5208b32e627891c389ebafc644145224006e8

#[derive(Debug, StructOpt, Clone)]
pub struct AccountCmd {
	#[structopt(subcommand)]
	cmd: Command
}

#[derive(StructOpt,Debug,Clone)]
enum Command {
	/// Query external or contract account information
	Query{

		/// External address or contract address
		#[structopt(long = "address")]
		address: String,

		/// Show the storage trie
		#[structopt(long = "storage-trie")]
		storage_trie: Option<String>,

		/// Show the storage trie
		#[structopt(long = "code-hash")]
		code_hash: Option<String>,
	},

	/// Create external account
	Create{
		/// External address will be created
		#[structopt(long = "address")]
		address: String,

		/// Value(Wei) for the given address,  default 1 ether (18 zeros)
		#[structopt(long = "value", default_value = "1000000000000000000")]
		value: String,

		/// Nonce for the given address, default 0
		#[structopt(long = "nonce", default_value = "0")]
		nonce: String,
	},

	/// Modify external account
	Modify{
		/// External address will be modified
		#[structopt(long = "address")]
		address: String,

		/// Value(Wei) for the given address
		#[structopt(long = "value")]
		value: String,

		/// Nonce for the given address
		#[structopt(long = "nonce")]
		nonce: String,
	},

	/// Transfer value from one external account to another
	Transfer{
		/// The address from which transfer from
		#[structopt(long = "from")]
		from: String,

		/// The address from which transfer to
		#[structopt(long = "to")]
		to: String,

		/// Value for transfer
		#[structopt(long = "value")]
		value: String,

		/// The gas limit for messageCall
		#[structopt(long = "gas")]
		gas: u32,

		/// The gas price (Wei) for messageCall
		#[structopt(long = "gas-price")]
		gas_price: String,

		/// The input data for messageCall
		#[structopt(long = "data")]
		data: Option<String>,
	},

	/// List all the account
	List{}
}

//#[derive(Debug)]
//pub enum Account{
//	EXTERNAL(H160, U256, U256),
//	CONTRACT(H160, U256, U256, H256, H256),
//}
//
//impl Account {
//	pub fn new(backend: &mut State,address: H160) -> Self {
//		let a = backend.get_account(address.clone());
//
//		println!("{:?}",a);
//
//		let account = backend.basic(address.clone());
//		let code_size = backend.code_size(address.clone());
//		if code_size == 0 {
//			Account::EXTERNAL(address.clone(), account.balance, account.nonce)
//		}else {
//			let code_hash = backend.code_hash(address.clone());
//			let storage_root = backend.storage_root(address.clone());
//			Account::CONTRACT(address.clone(), account.balance, account.nonce, code_hash.clone(), storage_root)
//		}
//	}
//}
//
//impl fmt::Display for Account {
//	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//
//		match self {
//			Account::EXTERNAL(address, value, nonce) => {
//				write!(f,"External Account Info: {{address: {:?}, balance: {:?}, nonce: {:?} }}",address,value,nonce)
//			},
//
//			Account::CONTRACT(address, value, nonce, code_hash, storage_trie) => {
//				write!(f,"Contract Account Info: {{address: {:?}, balance: {:?}, nonce: {:?}, code_hash: {:?}, storage_trie: {:?} }}",
//					   address, value, nonce, code_hash, storage_trie)
//			},
//		}
//	}
//}


impl AccountCmd {
	pub fn run(&self,backend: &mut State) -> bool {
		match &self.cmd {
			Command::Query {address, storage_trie, code_hash} => {
				let from = H160::from_str(address).expect("--address argument must be a valid address");
				let account = backend.get_account(from.clone());

				match (storage_trie,code_hash) {

					(None, None) => {
						println!("{:?}", account);
					},

					(Some(x), None) => {
						let storage_root = H256::from_str(x).expect("--storage-root argument must be a valid root");
						let kv = backend.get_storage(from.clone(), storage_root.clone());
						for (k, v) in kv.iter(){
							println!("{:?} -> {:?}",k,v);
						}
					},

					(None, Some(code_hash)) => {
						let code_hash = H256::from_str(code_hash).expect("--code-hash argument must be a valid code hash");
						let code = backend.get_code(from.clone(),code_hash.clone());
						let code = code.unwrap_or(vec![]);
						let len = {
							let c = &code;
							c.len()
						};
						let code_str = hex::encode(code);
						println!("Contract Account code info:{{ size: {:}, code: {:?}}}",len,code_str);
					},

					(_, _) => {

					}
				}
				return false;
			},

			Command::Create {address,value,nonce} => {
				let from = H160::from_str(address).expect("--address argument must be a valid address");
				let value = U256::from_dec_str(value.as_str()).expect("--value argument must be a valid number");
				let nonce = U256::from_dec_str(nonce.as_str()).expect("--nonce argument must be a valid number");

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

				backend.apply(applies,Vec::new(),false);
				let account = backend.get_account(from);
				println!("{}", account);
				return true;
			},

			Command::Modify {address,value,nonce} => {
				let from = H160::from_str(address).expect("--address argument must be a valid address");
				let value = U256::from_dec_str(value.as_str()).expect("--value argument must be a valid number");
				let nonce = U256::from_dec_str(nonce.as_str()).expect("--nonce argument must be a valid number");

				let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

				applies.push(Apply::Delete {
					address: from.clone(),
				});

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

				backend.apply(applies,Vec::new(),false);
				let account = backend.get_account(from);
				println!("{}", account);
				return true;
			},

			Command::Transfer {from, to, value, gas, gas_price,data} => {

				let from = H160::from_str(from).expect("--from argument must be a valid address");
				let to  = H160::from_str(to).expect("--to argument must be a valid address");
				let value = U256::from_dec_str(value.as_str()).expect("--value argument must be a valid number");
				let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");
				let gas_limit = *gas;



				let input = data.as_ref().map_or(vec![], |d| hex::decode(d.as_str()).expect("Input is invalid"));

				let config = Config::istanbul();
				let executor = StackExecutor::new(
					backend,
					gas_limit as usize,
					&config,
				);
				let nonce = Some(executor.nonce(from.clone()));

				let retv = executer::execute_evm(
					from.clone(),
					value,
					gas_limit,
					gas_price,
					nonce,
					|executor| ((), executor.transact_call(
						from,
						to,
						value,
						input,
						gas_limit as usize,
					)),
					backend
				).expect("Transfer failed");

				println!("Transfer Called, State OK.");

				return true;
			},

			Command::List{} => {
				let all_account = backend.list_address();
				for a in all_account {
					let account = backend.get_account(a.clone());
					if account.is_contract() {
						println!("{:?}, contract account",a);
					}else {
						println!("{:?}, external account",a);
					}
				}
				return false;
			}


		}

	}
}