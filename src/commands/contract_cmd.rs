use crate::executer;
use crate::commands::account_cmd;
use ethereum_types::{H160,U256};
use evm::backend::{Backend, ApplyBackend};
use evm::executor::StackExecutor;
use evm::Handler;
use evm::Context;
use evm::Capture;
use evm::ExitReason;
use hex;
use structopt::StructOpt;
use evm::Config;
use std::fs::File;
use std::io::Read;
use std::str::FromStr; // !!! Necessary for H160::from_str(address).expect("...");

// ./target/debug/bloom-evm contract --from 0000000000000000000000000000000000000001 --to 0000000000000000000000000000000000000002 --value 0 --gas_limit 100000 --gas-price 0 --input 6000
// ./target/debug/bloom-evm contract deploy --from 0000000000000000000000000000000000000001  --value 0 --gas 100000 --gas-price 0 --code-file ./code-file
// ./target/debug/bloom-evm contract deploy --from 0000000000000000000000000000000000000001  --value 0 --gas 100000 --gas-price 0 --code 000000

#[derive(Debug, StructOpt, Clone)]
pub struct ContractCmd {
    #[structopt(subcommand)]
    cmd: Command
}

#[derive(StructOpt, Debug, Clone)]
enum Command {
    /// Deploy contract
    Deploy {
        /// The address which deploy contact
        #[structopt(long = "from")]
        from: String,

        /// The value to deposit in contract
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

    },

    /// Message call
    Call {
        /// The address which send messageCall
        #[structopt(long = "from")]
        from: String,

        /// The value (Wei) for messageCall
        #[structopt(long = "value")]
        value: String,

        /// The receiver address for messageCall
        #[structopt(long = "to")]
        to: String,

        /// The gas limit for messageCall
        #[structopt(long = "gas")]
        gas: u32,

        /// The gas price (Wei) for messageCall
        #[structopt(long = "gas-price")]
        gas_price: String,

        /// The input data for messageCall
        #[structopt(long = "data")]
        data: Option<String>,

        /// The input data file for messageCall
        #[structopt(long = "data-file")]
        data_file: Option<String>,

    },

    /// Transaction call
    Transaction {
        /// The address which send messageCall
        #[structopt(long = "from")]
        from: String,

        /// The value (Wei) for messageCall
        #[structopt(long = "value")]
        value: String,

        /// The receiver address for messageCall
        #[structopt(long = "to")]
        to: String,

        /// The gas limit for messageCall
        #[structopt(long = "gas")]
        gas: u32,

        /// The gas price (Wei) for messageCall
        #[structopt(long = "gas-price")]
        gas_price: String,

        /// The input data for messageCall
        #[structopt(long = "data")]
        data: Option<String>,

        /// The input data file for messageCall
        #[structopt(long = "data-file")]
        data_file: Option<String>,

    }
}



impl ContractCmd {
    pub fn run<B: Backend + ApplyBackend + Clone>(&self, backend: &mut B) {
        match &self.cmd {
            Command::Deploy {from,value,gas,gas_price,code,code_file} => {

                let from = H160::from_str(from).expect("From should be a valid address");
                let value = U256::from_dec_str(value.as_str()).expect("Value is invalid");
                let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");
                let gas_limit = *gas;

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
                    backend,
                    gas_limit as usize,
                    &config,
                );
                let nonce = Some(executor.nonce(from.clone()));

                let contract_address = executer::execute_evm(
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
                    backend
                ).expect("Create contract failed");

                let account = account_cmd::Account::new(backend, contract_address.clone());
                println!("Create contract successful, {}", account);
            }

            Command::Transaction {from,value,to,gas,gas_price,data,data_file} => {
                let from = H160::from_str(from).expect("From should be a valid address");
                let to = H160::from_str(to).expect("To should be a valid address");
                let value = U256::from_dec_str(value.as_str()).expect("Value is invalid");
                let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");
                let gas_limit = *gas;

                let mut contents = String::new();

                let data = match data {
                    Some(d) => {
                        Ok(d)
                    }
                    None => {
                        let ret = match data_file {
                            Some(file) => {
                                let mut f = File::open(file).expect(" data file not found");

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
                }.unwrap_or(&contents);

                let input = hex::decode(data.as_str()).expect("Input is invalid");
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
                ).expect("Call message failed");

                println!("Contract Called, State OK.");
            }

            Command::Call {from,value,to,gas,gas_price,data,data_file} => {
                let from = H160::from_str(from).expect("From should be a valid address");
                let to = H160::from_str(to).expect("To should be a valid address");
                let value = U256::from_dec_str(value.as_str()).expect("Value is invalid");
                let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");
                let gas_limit = *gas;

                let mut contents = String::new();

                let data = match data {
                    Some(d) => {
                        Ok(d)
                    }
                    None => {
                        let ret = match data_file {
                            Some(file) => {
                                let mut f = File::open(file).expect(" data file not found");

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
                }.unwrap_or(&contents);

                let input = hex::decode(data.as_str()).expect("Input is invalid");
                let config = Config::istanbul();
                let mut executor = StackExecutor::new(
                    backend,
                    gas_limit as usize,
                    &config,
                );
                let nonce = Some(executor.nonce(from.clone()));
                let context = Context {
                    caller: from.clone(),
                    address: to.clone(),
                    apparent_value: value,
                };
                let retv =  executor.call(
                        to,
                        None,
                        input,
                        None,
                        true,
                        context);

                let (reason,retv) = match retv {
                    Capture::Exit(s) => s,
                    Capture::Trap(_) => unreachable!(),
                };

                match reason {
                    ExitReason::Succeed(_) => {
                        let r = hex::encode(retv);
                        println!("Contract Message Called, State OK. result: {:?}",r);
                    }
                    ExitReason::Error(e) => {
                        println!("Contract message call encounter error. {:?}",e);

                    }
                    ExitReason::Revert(e) => {
                        println!("Contract message call encounter error. {:?}",e);
                    },
                    ExitReason::Fatal(e) => {
                        println!("Contract message call encounter error. {:?}",e);
                    },
                };


            }
        }
    }
}