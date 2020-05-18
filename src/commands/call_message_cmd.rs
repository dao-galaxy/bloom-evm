use crate::executer;
use ethereum_types::{H160, U256};
use evm::backend::MemoryBackend as Backend;
use evm::executor::StackExecutor;
use hex;
use structopt::StructOpt;

// ./target/debug/evmbin call --from 0000000000000000000000000000000000000001 --to 0000000000000000000000000000000000000002 --value 0 --gas_limit 100000 --gas_price 0 --input 6000
#[derive(Debug, StructOpt, Clone)]
pub struct CallMessageCmd {
	#[structopt(long = "from")]
	pub from: String,
	#[structopt(long = "to")]
	pub to: String,
	#[structopt(long = "value")]
	pub value: String,
	#[structopt(long = "gas_limit")]
	pub gas_limit: u32,
	#[structopt(long = "gas_price")]
	pub gas_price: U256,
	#[structopt(long = "input")]
	pub input: String,
}

impl CallMessageCmd {
	pub fn run(&self, executor: &mut StackExecutor<Backend>) {
		let from: H160 = self.from.parse().expect("From should be a valid address");
		let to = self.to.parse().expect("To should be a valid address");
		let value = self.value.parse().expect("Value is invalid");
		let gas_limit = self.gas_limit;
		let gas_price = self.gas_price;
		let input = hex::decode(self.input.as_str()).expect("Input is invalid");

		let nonce = Some(executor.nonce(from.clone()));

		executer::execute_evm(
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
		).expect("Call message failed");

		println!("Call message successful");
	}
}