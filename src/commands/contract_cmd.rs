use crate::executer;
use crate::commands::account_cmd;
use ethereum_types::{H160,U256};
use evm::backend::{MemoryBackend};
use evm::executor::StackExecutor;
use hex;
use structopt::StructOpt;
use evm::Config;
use std::fs::File;
use std::io::Read;


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
        code: Option<String>,

        /// The code file
        #[structopt(long = "code-file")]
        code_file: Option<String>,

    }
}



impl ContractCmd {
    pub fn run(&self, backend: MemoryBackend) {
        match &self.cmd {
            Command::Deploy {from,value,gas,gas_price,code,code_file} => {

                let from: H160 = from.parse().expect("From should be a valid address");
                let value = U256::from_dec_str(value.as_str()).expect("Value is invalid");
                let gas_limit = *gas;
                let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");

                let mut contents = String::new();

                let code = match code {
                    Some(c) => {
                        Ok(c)
                    }
                    None => {
                        let ret = match code_file {
                            Some(file) => {
                                let mut f = File::open(file).expect(" code file not found");

                                f.read_to_string(&mut contents)
                                    .expect("something went wrong reading the file");
                                Ok(&contents)
                            }

                            None => {
                                Err(())
                            }
                        };
                        ret
                    }
                }.expect("--code or --code-file must be provided one of them ");

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