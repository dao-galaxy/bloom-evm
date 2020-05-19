use structopt::StructOpt;
use evm::backend::{Backend, ApplyBackend};
use evm::backend::{MemoryBackend,Apply,Basic};
use evm::executor::StackExecutor;
use evm::Transfer;
use primitive_types::{H160, H256, U256};
use std::fmt;
use std::collections::BTreeMap;
use evm::Config;



// ./target/debug/evmbin account --from 0000000000000000000000000000000000000001
#[derive(Debug, StructOpt, Clone)]
pub struct AccountCmd {
	#[structopt(subcommand)]
	cmd: Command
}

#[derive(StructOpt,Debug,Clone)]
enum Command {
	/// Query the information for given address
	Query{

		/// External address or contract address
		#[structopt(long = "address")]
		address: String,

		/// Flag whether show the storage trie
		#[structopt(long = "storage-trie")]
		storage_trie:bool
	},

	/// Create external address with given info
	Create{
		/// External address will be created
		#[structopt(long = "address")]
		address: String,

		/// Value for the given address(Wei), default 1ether
		#[structopt(long = "value",default_value = "1000000000000000000000000")]
		value: String,

		/// Nonce for the given address, default 0
		#[structopt(long = "nonce",default_value = "0")]
		nonce: u64,
	},

	/// Modify account information
	Modify{
		/// External address will be modified
		#[structopt(long = "address")]
		address: String,

		/// Value for the given address(Wei)
		#[structopt(long = "value")]
		value: String,

		/// Nonce for the given address
		#[structopt(long = "nonce")]
		nonce: u64,
	},

	/// Transfer value from A to B
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
enum Account{
	EXTERNAL(H160,U256,U256),
	CONTRACT(H160,U256,U256,H256,H256),
}

impl Account {
	pub fn new(backend: &MemoryBackend,address: H160) -> Self {
		let account = backend.basic(address.clone());
		let code_size = backend.code_size(address.clone());
		if code_size == 0 {
			Account::EXTERNAL(address.clone(),account.balance,account.nonce)
		}else {
			let code_hash = backend.code_hash(address.clone());
			Account::CONTRACT(address.clone(),account.balance,account.nonce,code_hash.clone(),code_hash.clone())
		}
	}
}

impl fmt::Display for Account {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

		match self {
			Account::EXTERNAL(address,value,nonce) => {
				write!(f,"External Account Info: {{address: {:?}, balance: {:?}, nonce: {:?} }}",address,value,nonce)
			},

			Account::CONTRACT(address,value,nonce,code_hash,storage_trie) => {
				write!(f,"Contract Account Info: {{address: {:?}, balance: {:?}, nonce: {:?}, code_hash: {:?}, storage_trie: {:?} }}",
					   address,value,nonce,code_hash,storage_trie)
			},
		}
	}
}

// Decimal system string to U256
fn parse(s: &str) -> Result<U256,String> {
	let mut ret = U256::zero();
	for (_, &item) in s.as_bytes().iter().enumerate() {
		if item < 48 || item > 57 {
			return Err("Invalid value".to_string());
		}
		let (r , _ )= ret.overflowing_mul(U256::from(10u64));
		let value = item - b'0';
		ret = r + value;
	}
	Ok(ret)
}


impl AccountCmd {
	pub fn run(&self,mut backend: MemoryBackend) {
		match &self.cmd {
			Command::Query {address,storage_trie} => {
				let from:H160 = address.parse().expect("From should be a valid address");
				if !storage_trie {
					let account = Account::new(&backend,from);
					println!("{}",account);
				}else {
					println!("no root");
				}
			},

			Command::Create {address,value,nonce} => {
				let from:H160 = address.parse().expect("address should be a valid address");
				//let value:U256 = value.parse().expect("value must be a valid value");
				let value:U256 = parse(value.as_str()).expect("value must be a valid value");
				let nonce:U256 = U256::from(*nonce);

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
				println!("{}",account);
			},

			Command::Modify {address,value,nonce} => {
				let from:H160 = address.parse().expect("address should be a valid address");
				let value:U256 = parse(value.as_str()).expect("value must be a valid value");
				let nonce:U256 = U256::from(*nonce);

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
				println!("{}",account);
			},

			Command::Transfer {from,to, value} => {

				let from:H160 = from.parse().expect("from address should be a valid address");
				let to:H160 = to.parse().expect("to address should be a valid address");
				let value:U256 = parse(value.as_str()).expect("value must be a valid value");

				let config = Config::istanbul();
				let gas_limit = 100000;
				let mut executor = StackExecutor::new(
					&backend,
					gas_limit as usize,
					&config,
				);

				match executor.transfer(Transfer{
					source:from,
					target:to,
					value,
				}) {
					Ok(_) => {
						let account = Account::new(&backend,from);
						println!("{}",account);

						let account = Account::new(&backend,to);
						println!("{}",account);
					},
					Err(err) => {
						println!("Transfer failed: {:?}",err);
					}
				}
			}


		}

	}
}