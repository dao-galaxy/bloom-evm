
mod handler;
mod query_service;

use zmq::{Context, DEALER, ROUTER};
use common_types::ipc::IpcReply;
use rlp::Encodable;
use kvdb_rocksdb::{Database,DatabaseConfig};
use std::sync::Arc;
use blockchain_db::BlockChain;
use std::{thread, env};
use clap::{App, load_yaml, ArgMatches, Arg};
use log::info;
use env_logger;

// const DATA_PATH: &'static str = "chain-data";

fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("clap.yaml");  // src/clap.yaml
    let matches = App::from(yaml).get_matches();

    let log_level = matches.value_of("log").unwrap_or("debug");
    env::set_var("RUST_LOG", log_level);
    env_logger::init();
    info!("log level: {:?}", log_level);
    info!("{:#?}", matches);

    // Gets a value for config if supplied by user, or defaults to "bloom.conf"
    let config = matches.value_of("config").unwrap_or("bloom.conf");
    info!("config: {:?}", config);
    //let nodes_number = hex::decode(matches.value_of("nodes").unwrap()).unwrap();
    //println!("{:?}", nodes_number);
    let data_dir = matches.value_of("data-dir").unwrap_or("chain-data");
    info!("data directory: {:?}", data_dir);
    let ip = matches.value_of("ip-addr").unwrap_or("127.0.0.1");
    info!("ip address: {:?}", ip);
    let consensus_port = matches.value_of("consensus-port").unwrap_or("8050");
    info!("consensus port: {:?}", consensus_port);
    let query_port = matches.value_of("query_port").unwrap_or("9050");
    info!("query port: {:?}", query_port);



    // let ip = std::env::args().nth(1).expect("no given ip");
    // let consensus_port = std::env::args().nth(2).expect("no given consensus port");
    // let query_port = std::env::args().nth(3).expect("no given query port");
    let consensus_end_point = String::from("tcp://") + ip + ":" + consensus_port;
    info!("consensus end point: {}", consensus_end_point);

    let query_end_point = String::from("tcp://") + ip + ":" + query_port;
    info!("query end point: {}", query_end_point);

    let config = DatabaseConfig::with_columns(bloom_db::NUM_COLUMNS);
    let database = Arc::new(Database::open(&config, data_dir).unwrap());

    // run query service
    let db = database.clone();
    thread::spawn(move ||{
        query_service::run_query_service(query_end_point.as_str(),db);
    });

    // run consensus service
    let mut blockchain = BlockChain::new(database.clone());
    run_server(consensus_end_point.as_str(),database.clone(),&mut blockchain);
}

pub fn run_server(end_point : &str,db: Arc<dyn (::kvdb::KeyValueDB)>, blockchain:&mut BlockChain) {
    let context = Context::new();
    let socket = context.socket(ROUTER).unwrap();
    socket.bind(end_point).unwrap();
    loop {
        let mut received_parts = socket.recv_multipart(0).unwrap();
        let msg_bytes = received_parts.pop().unwrap();
        let zmq_identity = received_parts.pop().unwrap();

        info!(
            "chain-state thread, received from client, #zmq_identity: {:x?}; #msg_bytes: {:x?}",
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

    const END_POINT : &'static str = "tcp://127.0.0.1:8060";

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

