use crate::executer;
use crate::commands::account_cmd;
use ethereum_types::{H160, U256, H256};
use evm::backend::{MemoryBackend, Apply, Basic, ApplyBackend, Backend};
use evm::executor::StackExecutor;
use hex;
use structopt::StructOpt;
use evm::Config;
use std::fs::File;
use std::io::Read;
use std::str::FromStr; // !!! Necessary for H160::from_str(address).expect("...");
use std::collections::BTreeMap;
use crate::commands::account_cmd::Account;

// target/debug/bloom-evm contract deploy --from 0000000000000000000000000000000000000001  --value 0 --gas 100000 --gas-price 0 --code-file ./code-file
// target/debug/bloom-evm contract deploy --from 0000000000000000000000000000000000000001  --value 0 --gas 100000 --gas-price 0 --code 000000
// target/debug/bloom-evm contract call --from 0000000000000000000000000000000000000001  --to 0000000000000000000000000000000000000002 --value 0 --gas 100000 --gas-price 0 --data 000000

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

    }
}



impl ContractCmd {
    pub fn run(&self, mut backend: MemoryBackend) {
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


                {
                    let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

                    applies.push(Apply::Delete {
                        address: from.clone(),
                    });

                    applies.push(Apply::Modify {
                        address: from.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("90000000000000000").expect("error"),
                            nonce : U256::from_dec_str("1").expect(""),
                        },
                        code: None,
                        storage: BTreeMap::new(),
                        reset_storage: false,
                    });

                    backend.apply(applies,Vec::new(),false);
                    let account = Account::new(&backend,from);
                    println!("{}", account);
                    println!("{:#?}", backend);
                }

                let code = hex::decode(code.as_str()).expect("Code is invalid");
                let config = Config::istanbul();
                let executor = StackExecutor::new(
                    &backend,
                    gas_limit as usize,
                    &config,
                );
                let nonce = Some(executor.nonce(from.clone()));

                let contract_address = executer::execute_evm(
                    &mut backend,
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

                let account = account_cmd::Account::new(&backend, contract_address.clone());
                println!("Create contract successful, {}", account);
                println!("{:#?}", backend);

                let code = backend.code(contract_address.clone());
                let code_str = hex::encode(code);
                println!("code={:?}",code_str);
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


                {
                    let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

                    applies.push(Apply::Delete {
                        address: from.clone(),
                    });

                    applies.push(Apply::Modify {
                        address: from.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("90000000000000000").expect("error"),
                            nonce : U256::zero(),
                        },
                        code: None,
                        storage: BTreeMap::new(),
                        reset_storage: false,
                    });

                    // Contract A
                    let code_str = String::from("60806040526004361061006d576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680630c55699c146100725780636f3c03401461009d57806393c1b33f146100f8578063a7126c2d1461018a578063a9421619146101cd575b600080fd5b34801561007e57600080fd5b50610087610210565b6040518082815260200191505060405180910390f35b3480156100a957600080fd5b506100de600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610216565b604051808215151515815260200191505060405180910390f35b34801561010457600080fd5b50610170600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291908035600019169060200190929190803560ff16906020019092919080356000191690602001909291908035600019169060200190929190505050610273565b604051808215151515815260200191505060405180910390f35b34801561019657600080fd5b506101cb600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061034b565b005b3480156101d957600080fd5b5061020e600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506103fa565b005b60005481565b600081604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166c0100000000000000000000000002815260140191505060405180910390205060019050919050565b6000600185858585604051600081526020016040526040518085600019166000191681526020018460ff1660ff1681526020018360001916600019168152602001826000191660001916815260200194505050505060206040516020810390808403906000865af11580156102ec573d6000803e3d6000fd5b5050506020604051035173ffffffffffffffffffffffffffffffffffffffff168673ffffffffffffffffffffffffffffffffffffffff161415600160006101000a81548160ff0219169083151502179055506001905095945050505050565b8073ffffffffffffffffffffffffffffffffffffffff1660405180807f696e632829000000000000000000000000000000000000000000000000000000815250600501905060405180910390207c010000000000000000000000000000000000000000000000000000000090046040518163ffffffff167c0100000000000000000000000000000000000000000000000000000000028152600401600060405180830381865af4925050505050565b8073ffffffffffffffffffffffffffffffffffffffff1660405180807f696e632829000000000000000000000000000000000000000000000000000000815250600501905060405180910390207c010000000000000000000000000000000000000000000000000000000090046040518163ffffffff167c01000000000000000000000000000000000000000000000000000000000281526004016000604051808303816000875af19250505050505600a165627a7a723058209e69217e89d69fbb6e1f9a6c123da4d184d81691046e7faf6d3fca00981a15800029");
                    let code = hex::decode(code_str.as_str()).expect("code is invalid");
                    let mut storage:BTreeMap<H256, H256> = BTreeMap::new();
//                    let key1 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000001").expect("");
//                    let val1 = H256::from_str("000000000000000000000000000000000000000000000000000000000000000b").expect("");
//                    storage.insert(key1,val1);
//
//                    let key1 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000002").expect("");
//                    let val1 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000002").expect("");
//                    storage.insert(key1,val1);

                    let to = H160::from_str("5a443704dd4b594b382c22a083e2bd3090a6fef3").expect("");
                    applies.push(Apply::Modify {
                        address: to.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("10000000000000000").expect("error"),
                            nonce : U256::one(),
                        },
                        code: Some(code),
                        storage: storage,
                        reset_storage: false,
                    });


                    // Contract B
                    let code_str = String::from("608060405260043610610057576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680630c55699c1461005c578063a7126c2d14610087578063a9421619146100ca575b600080fd5b34801561006857600080fd5b5061007161010d565b6040518082815260200191505060405180910390f35b34801561009357600080fd5b506100c8600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610113565b005b3480156100d657600080fd5b5061010b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506101c2565b005b60005481565b8073ffffffffffffffffffffffffffffffffffffffff1660405180807f696e632829000000000000000000000000000000000000000000000000000000815250600501905060405180910390207c010000000000000000000000000000000000000000000000000000000090046040518163ffffffff167c0100000000000000000000000000000000000000000000000000000000028152600401600060405180830381865af4925050505050565b8073ffffffffffffffffffffffffffffffffffffffff1660405180807f696e632829000000000000000000000000000000000000000000000000000000815250600501905060405180910390207c010000000000000000000000000000000000000000000000000000000090046040518163ffffffff167c01000000000000000000000000000000000000000000000000000000000281526004016000604051808303816000875af19250505050505600a165627a7a72305820d4e666caa8902efb06b9389361dff695b954a3713006ff68d5c8e3da104f489a0029");
                    let code = hex::decode(code_str.as_str()).expect("code is invalid");
                    let mut storage:BTreeMap<H256, H256> = BTreeMap::new();

                    let to = H160::from_str("5a443704dd4b594b382c22a083e2bd3090a6fef3").expect("");
                    applies.push(Apply::Modify {
                        address: to.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("5000000000000000").expect("error"),
                            nonce : U256::one(),
                        },
                        code: Some(code),
                        storage: storage,
                        reset_storage: false,
                    });

                    backend.apply(applies,Vec::new(),false);
                    let account = Account::new(&backend,from);
                    println!("{}", account);
                    println!("{:#?}", backend);
                }

                let input = hex::decode(data.as_str()).expect("Input is invalid");
                let config = Config::istanbul();
                let executor = StackExecutor::new(
                    &backend,
                    gas_limit as usize,
                    &config,
                );
                let nonce = Some(executor.nonce(from.clone()));

                executer::execute_evm(
                    &mut backend,
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

                println!("Contract Called, State OK.");
                println!("{:#?}", backend);
            }
        }
    }
}