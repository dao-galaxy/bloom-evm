
use common_types::ipc::*;
use common_types::transaction::{SignedTransaction,UnverifiedTransaction};
use rlp;
use evm_executer;
use evm_executer::Error;
use blockchain_db::BlockChain;
use std::sync::Arc;
use std::hash::Hasher;
use common_types::header::Header;
use common_types::block::Block;
use rlp::DecoderError;

pub fn handler(data: Vec<u8>, db: Arc<dyn (::kvdb::KeyValueDB)>, blockchain: &mut BlockChain) -> IpcReply {

    let request: IpcRequest = rlp::decode(data.as_slice()).unwrap();
    let errResp = MessageResp(String::from("error"));
    let mut ret = IpcReply{
        id: request.id,
        status: 1,
        result: rlp::encode(&errResp),
    };
    match request.method.as_str() {
        "CreateHeader" => {
            let req: Result<CreateHeaderReq,DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("CreateHeader, {:?}", req.clone());
                let r = create_header(req, db);
                if r.is_ok() {
                    let resp = r.unwrap();
                    ret = IpcReply {
                        id: request.id,
                        status: 0,
                        result: rlp::encode(&resp),
                    }
                }
            }

        },
        "LatestBlocks" => {
            let req: Result<LatestBlocksReq,DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("LatestBlocks, {:?}", req.clone());
                let resp = latest_blocks(req, blockchain);
                ret = IpcReply {
                    id: request.id,
                    status:0,
                    result: rlp::encode(&resp),
                }
            }
        },
        "ApplyBlock" => {
            let req: Result<ApplyBlockReq,DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("ApplyBlock, {:?}", req.clone());
                let r = apply_block(req, db, blockchain);
                if r.is_ok() {
                    let resp = r.unwrap();
                    ret = IpcReply {
                        id: request.id,
                        status: 0,
                        result: rlp::encode(&resp),
                    }
                }
            }
        },
        "AccountInfo" => {
            let req: Result<AccountInfoReq,DecoderError> = rlp::decode(request.params.as_slice());
            if !req.is_err() {
                let req = req.unwrap();
                println!("AccountInfo, {:?}", req.clone());
                let resp = account_info(req, db, blockchain);
                ret = IpcReply {
                    id: request.id,
                    status:0,
                    result: rlp::encode(&resp)
                }
            }
        },
        _ => {
            println!("Error: Invalid Request!");
        },
    }
    ret
}

fn create_header(req: CreateHeaderReq, db: Arc<dyn (::kvdb::KeyValueDB)>) -> Result<CreateHeaderResp,Error> {
    let mut signed_txs:Vec<SignedTransaction> = vec![];
    for tx in req.transactions {
        let t = SignedTransaction::new(tx).unwrap();
        signed_txs.push(t);
    }

    let ret = evm_executer::create_header(req.parent_block_hash,
                                             req.author,
                                             req.extra_data,
                                             req.gas_limit,
                                             req.difficulty,
                                             signed_txs,
                                             db.clone());
    if ret.is_err() {
        return Err(ret.err().unwrap());
    }
    Ok(CreateHeaderResp(ret.unwrap()))
}

pub fn latest_blocks(req: LatestBlocksReq, blockchain: &BlockChain) -> LatestBlocksResp {
    let n = if req.0 <= 0 { 1 } else { req.0 };
    let block_block_number = blockchain.best_block_number();
    let mut headers: Vec<Header> = vec![];
    for i in 0..n {
        let number = block_block_number - i;
        let block = blockchain.block_by_number(number);
        if(block.is_none()) {
            break;
        }
        headers.push(block.unwrap().header);
    }

    LatestBlocksResp(headers)
}

fn apply_block(req: ApplyBlockReq, db: Arc<dyn (::kvdb::KeyValueDB)>,bc: &mut BlockChain) -> Result<ApplyBlockResp,Error> {
    let mut signed_trx: Vec<SignedTransaction> = vec![];
    for tx in req.1.clone() {
        signed_trx.push(SignedTransaction::new(tx).unwrap());
    }

    let best_header = bc.best_block_header();
    let mut root = best_header.state_root();
    let ret = evm_executer::apply_block(req.0.clone(), signed_trx.clone(), db, root);
    if ret.is_err() {
        return Err(ret.err().unwrap());
    }

    let mut block = Block::default();
    block.header = req.0.clone();
    block.transactions = req.1.clone();
    let ret = bc.insert_block(block);
    if ret.is_err(){
        return Err(Error::ExitReasonFatal);
    }
    Ok(ApplyBlockResp(true))
}

pub fn account_info(req: AccountInfoReq, db: Arc<dyn (::kvdb::KeyValueDB)>, bc: &BlockChain ) -> AccountInfoResp {
    let best_header = bc.best_block_header();
    let state_root = best_header.state_root();
    let (nonce, balance) = evm_executer::account_info(req.0, db, state_root);
    AccountInfoResp(nonce, balance)
}