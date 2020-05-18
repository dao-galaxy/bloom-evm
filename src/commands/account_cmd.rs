use structopt::StructOpt;
use evm::backend::Backend;
use evm::backend::MemoryBackend;
use primitive_types::{H160, H256, U256};
use std::fmt;


// ./target/debug/evmbin account --from 0000000000000000000000000000000000000001
#[derive(Debug, StructOpt, Clone)]
pub struct AccountCmd {
	#[structopt(long = "query")]
	pub from: String,
}

#[derive(Debug)]
enum Account{
	EXTERNAL(H160,U256,U256),
	CONTRACT(H160,U256,U256,H256,H256),
}

impl Account {
	pub fn new(backend: MemoryBackend,address: H160) -> Self {
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


impl AccountCmd {
	pub fn run(&self, backend: MemoryBackend) {
		let from:H160 = self.from.parse().expect("From should be a valid address");
		let account = Account::new(backend,from);
		println!("{}",account);
	}
}