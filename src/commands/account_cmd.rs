use structopt::StructOpt;
use evm::backend::{Backend, ApplyBackend};
use evm::backend::{MemoryBackend,Apply,Basic};
use evm::executor::StackExecutor;
use evm::Transfer;
use ethereum_types::{H160, H256, U256};
use std::fmt;
use std::collections::BTreeMap;
use evm::Config;
use std::str::FromStr; // !!! Necessary for H160::from_str(address).expect("...");

// target/debug/bloom-evm account create --address 59a5208b32e627891c389ebafc644145224006e8 --value 10 --nonce 12
// target/debug/bloom-evm account query --address 59a5208b32e627891c389ebafc644145224006e8
// target/debug/bloom-evm account query --address 59a5208b32e627891c389ebafc644145224006e8 --storage-trie

#[derive(Debug, Clone, StructOpt)]
pub struct AccountCmd {
	#[structopt(subcommand)]
	cmd: Command
}

#[derive(Debug, Clone, StructOpt)]
enum Command {
	/// Query external or contract account information
	Query{
		/// External address or contract address
		#[structopt(long = "address")]
		address: String,
		/// Flag whether show the storage trie
		#[structopt(long = "storage-trie")]
		storage_trie:bool
	},

	/// Create external account
	Create{
		/// External address will be created
		#[structopt(long = "address")]
		address: String,
		/// Value (Wei) for the given address,  default 1 ether (18 zeros)
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
		/// Value (Wei) for the given address
		#[structopt(long = "value")]
		value: String,
		/// Nonce for the given address
		#[structopt(long = "nonce")]
		nonce: String,
	},

	/// Transfer value between two external accounts
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
	}
}

#[derive(Debug)]
pub enum Account{
	EXTERNAL(H160, U256, U256),
	CONTRACT(H160, U256, U256, H256, H256),
}

impl Account {
	pub fn new(backend: &MemoryBackend, address: H160) -> Self {
		let account = backend.basic(address.clone());
		let code_size = backend.code_size(address.clone());
		if code_size == 0 {
			Account::EXTERNAL(address.clone(), account.balance, account.nonce)
		} else {
			let code_hash = backend.code_hash(address.clone());
			Account::CONTRACT(address.clone(), account.balance, account.nonce, code_hash.clone(), code_hash.clone())
		}
	}
}

impl fmt::Display for Account {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

		match self {
			Account::EXTERNAL(address, value, nonce) => {
				write!(f, "External Account Info: {{address: {:?}, balance: {:?}, nonce: {:?} }}", address, value, nonce)
			},

			Account::CONTRACT(address, value, nonce, code_hash, storage_trie) => {
				write!(f, "Contract Account Info: {{address: {:?}, balance: {:?}, nonce: {:?}, code_hash: {:?}, storage_trie: {:?} }}",
					   address, value, nonce, code_hash, storage_trie)
			},
		}
	}
}


impl AccountCmd {
	pub fn run(&self, mut backend: MemoryBackend) {
		match &self.cmd {
			Command::Query {address, storage_trie} => {
				let from = H160::from_str(address).expect("--address argument must be a valid address");
				if !storage_trie {
					let account = Account::new(&backend, from);
					println!("{}", account);
				} else {
					println!("--storage_trie has not yet supported!");
				}
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
				let account = Account::new(&backend,from);
				println!("{}", account);
			},

			Command::Modify {address, value, nonce} => {
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
				let account = Account::new(&backend,from);
				println!("{}", account);
			},

			Command::Transfer {from, to, value} => {

				let from = H160::from_str(from).expect("--from argument must be a valid address");
				let to  = H160::from_str(to).expect("--to argument must be a valid address");
				let value = U256::from_dec_str(value.as_str()).expect("--value argument must be a valid number");

				let config = Config::istanbul();
				let gas_limit = 100000;
				let mut executor = StackExecutor::new(
					&backend,
					gas_limit as usize,
					&config,
				);

				match executor.transfer(Transfer{ source:from, target:to, value, }) {
					Ok(_) => {
						let account = Account::new(&backend, from);
						println!("{}", account);

						let account = Account::new(&backend, to);
						println!("{}", account);
					},
					Err(err) => {
						println!("Transfer failed: {:?}", err);
					}
				}
			}


		}

	}
}