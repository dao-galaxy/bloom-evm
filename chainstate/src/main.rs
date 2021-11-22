
mod handler;
mod query_service;
mod config;
mod genesis;

use zmq::{Context, DEALER, ROUTER};
use common_types::ipc::IpcReply;
use rlp::Encodable;
use kvdb_rocksdb::{Database,DatabaseConfig};
use std::sync::Arc;
use blockchain_db::BlockChain;
use std::{thread, env};
use std::path::Path;
use clap::{App, load_yaml, ArgMatches, Arg};
use log::info;
use env_logger;
use crate::config::*;

use query_service::run_query_service;

// ./target/debug/bloom-chain -c chainstate/src/bloom.conf
fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("clap.yaml");  // src/clap.yaml
    let matches = App::from(yaml).get_matches();
    let config_file = matches.value_of("config").unwrap_or("chainstate/src/bloom.conf");
    // target/debug/bloom-solo -c src/bloom.conf
    let toml_string = read_config_file(config_file);
    let decoded_config = parse_config_string(toml_string.as_str());
    let decoded_config_clone = decoded_config.clone();

    let mut log_level = matches.value_of("log").unwrap_or(
        &decoded_config.log_level.unwrap_or("debug".to_string())
    ).to_string();

    env::set_var("RUST_LOG", log_level.as_str());
    env_logger::init();
    info!("log level: {:?}", log_level.as_str());
    info!("{:#?}", matches);
    info!("{:#?}", decoded_config_clone);

    let data_dir = matches.value_of("data-dir").unwrap_or(
        &decoded_config.data_directory.unwrap_or("chain-data".to_string())
    ).to_string();
    info!("data directory: {:?}", data_dir);

    let chain_socket_str = decoded_config.chain_socket.unwrap_or("tcp://127.0.0.1:8050".to_string());
    info!("consensus end point: {}", chain_socket_str);

    let query_socket_str = decoded_config.query_socket.unwrap_or("tcp://127.0.0.1:9050".to_string());
    info!("query end point: {}", query_socket_str);


    let consensus = decoded_config.consensus.unwrap_or("solo".to_string());
    let my_peer_index = decoded_config.index.unwrap_or(0);
    let block_duration = decoded_config.block_duration.unwrap_or(5);

    info!("my peer index: {:?}", my_peer_index);
    info!("block duration (period, time interval): {:?}", block_duration);

    let is_data_path_exist = Path::new(&data_dir).exists();
    let config = DatabaseConfig::with_columns(bloom_db::NUM_COLUMNS);
    let database = Arc::new(Database::open(&config, data_dir.as_str()).unwrap());
    if !is_data_path_exist {
        info!("init data");
        genesis::init_genesis(database.clone(),decoded_config.accounts.unwrap_or(vec![]));
    }
    let context = Context::new();

    // run query service
    let db = database.clone();
    let ctxt = context.clone();
    let query_thread = thread::spawn(move ||{
        run_query_service(query_socket_str.as_str(), db, ctxt);
    });

    // run consensus service
    let db = database.clone();
    let mut blockchain = BlockChain::new(db);
    let db = database.clone();
    let ctxt = context.clone();
    let chain_thread = thread::spawn(move || {
        run_chain_service(chain_socket_str.as_str(), db, &mut blockchain, ctxt);
    });

    {
        //TODO other things here.
    }

    query_thread.join().unwrap();
    chain_thread.join().unwrap();
}

pub fn run_chain_service(end_point : &str, db: Arc<dyn (::kvdb::KeyValueDB)>, blockchain:&mut BlockChain, ctxt: Context) {
    let socket = ctxt.socket(ROUTER).unwrap();
    socket.bind(end_point).unwrap();
    loop {
        let mut received_parts = socket.recv_multipart(0).unwrap();
        let msg_bytes = received_parts.pop().unwrap();
        let zmq_identity = received_parts.pop().unwrap();

        info!(
            "chainstate thread, received from client, #zmq_identity: {:x?}; #msg_bytes: {:x?}",
            zmq_identity,
            msg_bytes
        );

        let result = handler::handler(msg_bytes.clone(),db.clone(), blockchain);
        let result_data = result.rlp_bytes();

        socket.send_multipart(vec![zmq_identity, result_data.clone()], 0).unwrap();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use zmq::{Context, DEALER, ROUTER};
    use common_types::ipc::*;
    use ethereum_types::{Address, U256};
    use std::str::FromStr;
    use rlp;
    use hex_literal::hex;

    const END_POINT : &'static str = "tcp://127.0.0.1:9050";

    #[test]
    fn account_info_test(){
        let context = Context::new();
        let socket = context.socket(DEALER).unwrap();
        socket.set_identity( b"evm-query" ).unwrap();
        socket.connect(END_POINT).unwrap();

        let address = Address::from_str("26d1ec50b4e62c1d1a40d16e7cacc6a6580757d5").unwrap();
        let req = AccountInfoReq(address);
        let rlp_bytes = rlp::encode(&req);

        let ipc_req = IpcRequest{
            method:"AccountInfo".to_string(),
            id: 1u64,
            params: rlp_bytes,
        };

        let rlp_bytes = rlp::encode(&ipc_req);

        socket.send(rlp_bytes,0).unwrap();
        let mut received_parts = socket.recv_multipart(0).unwrap();
        //println!("client thread, received from server, #received_parts: {:?}", received_parts);
        let resp = received_parts.pop().unwrap();
        println!(
            "\tclient thread, received from server, #received_parts: {:x?};",
            resp.clone()
        );

        let ipc_resp: IpcReply = rlp::decode(resp.as_slice()).unwrap();
        let resp: AccountInfoResp = rlp::decode(ipc_resp.result.as_slice()).unwrap();
        println!("address={:?}",address);
        println!("----balance={:?}",resp.1);
        println!("----nonce={:?}",resp.0);


        // let req = LatestBlocksReq(1);
        // let ipc_req = IpcRequest{
        //     method:"LatestBlocks".to_string(),
        //     id: 1u64,
        //     params: rlp::encode(&req),
        // };
        // let rlp_bytes = rlp::encode(&ipc_req);
        // socket.send(rlp_bytes,0).unwrap();
        // let mut received_parts = socket.recv_multipart(0).unwrap();
        // //println!("client thread, received from server, #received_parts: {:?}", received_parts);
        // let resp = received_parts.pop().unwrap();
        // let ipc_resp: IpcReply = rlp::decode(resp.as_slice()).unwrap();
        // let resp: LatestBlocksResp = rlp::decode(ipc_resp.result.as_slice()).unwrap();
        // println!("LastestBlocksResp={:?}",resp);
        //
        // let latest_header = resp.0.get(0).unwrap();
        // let req = CreateHeaderReq::new(latest_header.hash(),address,b"jack".to_vec(),U256::zero(),U256::zero(),vec![]);
        // let ipc_req = IpcRequest{
        //     method:"CreateHeader".to_string(),
        //     id: 1u64,
        //     params: rlp::encode(&req),
        // };
        // let rlp_bytes = rlp::encode(&ipc_req);
        // socket.send(rlp_bytes,0).unwrap();
        // let mut received_parts = socket.recv_multipart(0).unwrap();
        // //println!("client thread, received from server, #received_parts: {:?}", received_parts);
        // let resp = received_parts.pop().unwrap();
        // let ipc_resp: IpcReply = rlp::decode(resp.as_slice()).unwrap();
        // let resp: CreateHeaderResp = rlp::decode(ipc_resp.result.as_slice()).unwrap();
        // println!("CreateHeader={:?}",resp.0);
        //
        // let req = ApplyBlockReq(resp.0,vec![]);
        // let ipc_req = IpcRequest{
        //     method:"ApplyBlock".to_string(),
        //     id: 1u64,
        //     params: rlp::encode(&req),
        // };
        // let rlp_bytes = rlp::encode(&ipc_req);
        // socket.send(rlp_bytes,0).unwrap();
        // let mut received_parts = socket.recv_multipart(0).unwrap();
        // //println!("client thread, received from server, #received_parts: {:?}", received_parts);
        // let resp = received_parts.pop().unwrap();
        // let ipc_resp: IpcReply = rlp::decode(resp.as_slice()).unwrap();
        // let resp: ApplyBlockResp = rlp::decode(ipc_resp.result.as_slice()).unwrap();
        // println!("CreateHeader={:?}",resp);
        //
        //
        // let req = LatestBlocksReq(1);
        // let ipc_req = IpcRequest{
        //     method:"LatestBlocks".to_string(),
        //     id: 1u64,
        //     params: rlp::encode(&req),
        // };
        // let rlp_bytes = rlp::encode(&ipc_req);
        // socket.send(rlp_bytes,0).unwrap();
        // let mut received_parts = socket.recv_multipart(0).unwrap();
        // //println!("client thread, received from server, #received_parts: {:?}", received_parts);
        // let resp = received_parts.pop().unwrap();
        // let ipc_resp: IpcReply = rlp::decode(resp.as_slice()).unwrap();
        // let resp: LatestBlocksResp = rlp::decode(ipc_resp.result.as_slice()).unwrap();
        // println!("LastestBlocksResp={:?}",resp);

    }
}

