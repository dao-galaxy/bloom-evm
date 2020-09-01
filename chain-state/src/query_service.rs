use zmq::{Context, DEALER, ROUTER};
use common_types::ipc::*;
use rlp::{Encodable,DecoderError};
use kvdb_rocksdb::{Database,DatabaseConfig};
use std::sync::Arc;
use blockchain_db::BlockChain;
use crate::handler::{latest_blocks, account_info};
use ethereum_types::H256;

pub fn run_query_service(end_point : &str, db: Arc<dyn (::kvdb::KeyValueDB)>, ctxt: Context) {
    let socket = ctxt.socket(ROUTER).unwrap();
    socket.bind(end_point).unwrap();
    loop {
        let mut received_parts = socket.recv_multipart(0).unwrap();
        let msg_bytes = received_parts.pop().unwrap();
        let zmq_identity = received_parts.pop().unwrap();
        println!(
            "Query service received message, #zmq_identity: {:x?}; #msg_bytes: {:x?}",
            zmq_identity,
            msg_bytes
        );

        let result = query_handler(msg_bytes.clone(),db.clone());
        let result_data = result.rlp_bytes();

        socket.send_multipart(vec![zmq_identity, result_data.clone()], 0).unwrap();
    }
}

fn query_handler(data: Vec<u8>, db: Arc<dyn (::kvdb::KeyValueDB)>) -> IpcReply {
    let blockchain = BlockChain::new(db.clone());
    let request: IpcRequest = rlp::decode(data.as_slice()).unwrap();
    let mut ret = IpcReply::default();
    match request.method.as_str() {
        "AccountInfo" => {
            let req: Result<AccountInfoReq, DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("AccountInfo, {:?}", req.clone());
                let resp = account_info(req, db, &blockchain);
                ret = IpcReply {
                    id: request.id,
                    result: rlp::encode(&resp)
                };
            }
        },
        "LatestBlocks" => {
            let req: Result<LatestBlocksReq, DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("LatestBlocks, {:?}", req.clone());
                let resp = latest_blocks(req, &blockchain);
                ret = IpcReply {
                    id: request.id,
                    result: rlp::encode(&resp),
                };
            }
        },
        "TxHashList" => {
            let req: Result<TxHashListReq, DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("TxHashList, {:?}", req.clone());
                let resp = block_tx_hash_list(req, &blockchain);
                ret = IpcReply {
                    id: request.id,
                    result: rlp::encode(&resp),
                };
            }
        },
        _ => {
            println!("Error: Invalid Request!");
        },
    }
    ret
}

pub fn block_tx_hash_list(req: TxHashListReq, blockchain: &BlockChain) -> TxHashListResp {
    let block_hash = req.0;
    let list = blockchain.transaction_hash_list_by_block_hash(block_hash);
    let mut hash_list: Vec<H256> = vec![];
    let x = list.unwrap().transactions().clone();

    TxHashListResp(x)
}



