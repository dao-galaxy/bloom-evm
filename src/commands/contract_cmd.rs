use crate::executer;
use crate::parser;
use crate::commands::account_cmd;
use ethereum_types::{H160};
use evm::backend::{MemoryBackend};
use evm::executor::StackExecutor;
use hex;
use structopt::StructOpt;
use std::path::PathBuf;
use evm::Config;



// ./target/debug/evmbin contract --from 0000000000000000000000000000000000000001 --to 0000000000000000000000000000000000000002 --value 0 --gas_limit 100000 --gas_price 0 --input 6000
#[derive(Debug, StructOpt, Clone)]
pub struct ContractCmd {

    #[structopt(subcommand)]
    cmd: Command
}

#[derive(StructOpt,Debug,Clone)]
enum Command {
    /// Deploy contract
    Deploy{
        /// The address which deploy contact
        #[structopt(long = "from")]
        from: String,

        /// The value for deploying contract(Wei)
        #[structopt(long = "value")]
        value: String,

        /// The gas limit for deploying contract
        #[structopt(long = "gas")]
        gas: u32,

        /// The gas price for deploying contract(Wei)
        #[structopt(long = "gas-price")]
        gas_price: String,

        /// The contract binary code
        #[structopt(long = "code")]
        code: String,

        /// The code file
        #[structopt(long = "code-file",parse(from_os_str))]
        code_file: PathBuf,

    }
}



impl ContractCmd {
    pub fn run(&self, backend: MemoryBackend) {
        match &self.cmd {
            Command::Deploy {from,value,gas,gas_price,code,code_file} => {

                let from: H160 = from.parse().expect("From should be a valid address");
                let value = parser::parse(value.as_str()).expect("Value is invalid");
                let gas_limit = *gas;
                let gas_price = parser::parse(gas_price.as_str()).expect("Gas price is invalid");
                let code = hex::decode(code.as_str()).expect("Code is invalid");
                let config = Config::istanbul();
                let executor = StackExecutor::new(
                    &backend,
                    gas_limit as usize,
                    &config,
                );
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

                let account = account_cmd::Account::new(&backend,create_address.clone());
                println!("Create contract successful, {}", account);
            }
        }
    }
}