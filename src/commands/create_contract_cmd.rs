use structopt::StructOpt;
use ethereum_types::{H160, U256};
use crate::executer;
use evm::backend::MemoryBackend as Backend;
use hex;
use evm::executor::StackExecutor;

// ./target/debug/evmbin create --from 0000000000000000000000000000000000000001 --value 0 --gas_limit 100000 --gas_price 0 --code 6000
#[derive(Debug, StructOpt, Clone)]
pub struct CreateContractCmd {
	#[structopt(long = "from")]
	pub from: String,
	#[structopt(long = "value")]
	pub value: String,
	#[structopt(long = "gas_limit")]
	pub gas_limit: u32,
	#[structopt(long = "gas_price")]
	pub gas_price: U256,
	#[structopt(long = "code")]
	pub code: String,
}

impl CreateContractCmd {
	pub fn run(&self, executor: &mut StackExecutor<Backend>) {
		let from: H160 = self.from.parse().expect("From should be a valid address");
		let value = self.value.parse().expect("Value is invalid");
		let gas_limit = self.gas_limit;
		let gas_price = self.gas_price;
		let code = hex::decode(self.code.as_str()).expect("Code is invalid");

		let nonce = Some(executor.nonce(from.clone()));

		let create_address = executer::execute_evm(
			from.clone(),
			value,
			gas_limit,
			gas_price,
			nonce,
			|executor| {
				(executor.create_address(
					evm::CreateScheme::Legacy { caller: from.clone() },
				), executor.transact_create(
					from,
					value,
					code,
					gas_limit as usize,
				))
			},
		).expect("Create contract failed");

		println!("Create contract successful, {:?}", create_address);
	}
}