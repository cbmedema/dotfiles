use tokio::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use libp2p::gossipsub;
use rand::random;
use tokio::sync::mpsc;
use tokio::time::sleep;
use crate::{block, network, input};
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::output::Output;
use crate::transactions::Tx;
use std::sync::Arc;
use async_std::prelude::FutureExt;
use num_format::Locale::ca;
use block::Block;
use input::Input;

#[derive(Clone)]


pub struct Miner {
    address: [u8;32],
    consensus: Arc<Mutex<Block>>,
}
impl Miner {

    pub fn new(wallet_addr: [u8;32]) -> Miner {
        let genesis_block = Block {index: 0, hash: [8;32], previous_hash: [0;32], transactions: Vec::new(),
            time: 0, nonce: 420, target: 2u64.pow(64-24)};
        Miner { address: wallet_addr, consensus: Arc::new(Mutex::new(genesis_block.clone()))}
    }

    pub async fn mine(&mut self) {
        let mut swarm = network::GossipSwarm::new().unwrap();
        let topic_consensus = gossipsub::IdentTopic::new("consensus-block");
        swarm.subscribe(topic_consensus.clone()).unwrap();
        swarm.publish(topic_consensus).unwrap();

        let swarm_mutex = Arc::new(Mutex::new(swarm));
        let consensus_mutex = Arc::clone(&self.consensus);

        let send_consensus = {
            let swarm_mutex = Arc::clone(&swarm_mutex);
            let consensus_mutex = Arc::clone(&consensus_mutex);
            let (mut tx, mut rx) = mpsc::channel(32);
            tokio::spawn(async move {
                loop {
                    let consensus_block = {
                        let consensus_lock = consensus_mutex.lock().await;
                        consensus_lock.clone()
                    };
                    if let Err(e) = tx.send(consensus_block.clone()).await {
                        eprintln!("Tx Error {e}");
                    }
                    else {
                        let mut swarm_lock = swarm_mutex.lock().await;
                        if let Err(pub_e) = swarm_lock.send_message(&mut rx).await {
                            eprintln!("Publishing Error {pub_e}");
                        }
                        else {
                            // if block is sent successfully, its index is updated
                            println!("Sent Block {} successfully ", consensus_block.index);
                        }
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            })
        };

        let handle_events = {
            let mut swarm_mutex = Arc::clone(&swarm_mutex);
            let mut consensus_mutex = Arc::clone(&consensus_mutex);
            tokio::spawn(async move {
                loop {
                    let events = {
                        let mut swarm_lock = swarm_mutex.lock().await;
                        swarm_lock.handle_events().await
                    };
                    match events {
                        Ok(Some(blk)) => {
                            let mut consensus_block = {
                                let consensus_lock = consensus_mutex.lock().await;
                                consensus_lock.clone()
                            };
                            if blk.index > consensus_block.index {
                                let mut consensus_lock = consensus_mutex.lock().await;
                                consensus_lock.previous_hash = blk.previous_hash;
                                consensus_lock.hash = blk.hash;
                                consensus_lock.index += blk.index;

                                println!("Received new consensus from node!");
                                print!("Hash: ");
                                blk.hash.iter().for_each(|hex| print!("{:02x}", hex));
                                println!();
                                blk.print();
                            } else {
                                println!("Received consensus block from node! ");
                                print!("Hash: ");
                                blk.hash.iter().for_each(|hex| print!("{:02x}", hex));
                                println!();
                            }
                        }
                        Ok(None) => {
                            println!("No event received");
                        }
                        Err(e) => {
                            eprintln!("Error handling event: {:?}", e);
                        }
                    }
                }
            })
        };
        let send_candidate = {
            let consensus_mutex = Arc::clone(&consensus_mutex);
            let swarm_mutex = Arc::clone(&swarm_mutex);
            let address = self.address;
            let (mut tx, mut rx) = mpsc::channel(32);
            tokio::spawn(async move {
                loop {
                    let candidate_data = {
                        // Acquire the lock briefly to clone the data
                        let consensus_lock = consensus_mutex.lock().await;
                        consensus_lock.clone() // Clone only the data needed for block generation
                    };
                    println!("Trying to find candidate block!");
                    let candidate_block = Self::generate_candidate_block(candidate_data, address).await;
                    candidate_block.print();

                    // sends candidate block to network
                    let mut swarm_lock = swarm_mutex.lock().await;
                    if let Err(e) = tx.send(candidate_block.clone()).await {
                        eprintln!("Tx Error {e}");
                    }
                    else {
                        let mut consensus_lock = consensus_mutex.lock().await;
                        if let Err(pub_e) = swarm_lock.send_message(&mut rx).await {
                            eprintln!("Publishing Error {pub_e}");
                        }
                        else {
                            // if block is sent successfully, its index is updated
                            println!("Sent Block {} successfully ", candidate_block.index);
                            consensus_lock.hash = candidate_block.hash;
                            consensus_lock.previous_hash = candidate_block.previous_hash;
                            consensus_lock.index = candidate_block.index;
                        }
                    }
                }
            })
        };



       let _ =  tokio::join!(handle_events,send_consensus,send_candidate);
    }

    async fn generate_candidate_block(consensus: Block, address: [u8;32]) -> Block {
        let (hash,nonce) = Self::gen_valid_hash(consensus.index+1, consensus.hash, consensus.target).await;
        //let (mut transactions, fees) = pool.calc_valid_tx_pool_and_fees(&chain);
        let mut transactions = vec![];
        transactions.push(Self::generate_coinbase(0, address));

        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut candidate = Block { index: consensus.index +1, hash, previous_hash: consensus.hash,
            time, target: consensus.target, nonce, transactions };
        candidate
    }
    fn generate_coinbase(fees: u64, address: [u8;32]) -> Tx {
        const REWARD: u64 = 5000000;
        let mut inputs = vec![];
        let mut outputs = vec![];
        // inputs aren't important to coinbase Tx, however random signature given to prevent duplicate txid hash
        let mut signature: [u8; 64] = [0; 64];
        signature.iter_mut().for_each(|elm| *elm = random());


        let coinbase_input = Input { txid: [0; 32], signature,};
        let coinbase_output = Output { amount: REWARD+fees, address};


        inputs.push(coinbase_input);
        outputs.push(coinbase_output);
        let txid = Tx::generate_txid(&inputs, &outputs);
        Tx { txid, inputs, outputs }
    }

    async fn gen_valid_hash(index: u32, prev_hash: [u8;32], target: u64) -> ([u8;32],u64) {
        let (mut hash, mut nonce) = Self::gen_hash_nonce(index, prev_hash).await;
        while Miner::h2_u64(hash) > target {
            (hash, nonce) = Self::gen_hash_nonce(index, prev_hash).await;
        }

        (hash, nonce)
    }

    async fn gen_hash_nonce(index: u32, prev_hash: [u8;32]) -> ([u8;32],u64) {
        let mut hasher = blake3::Hasher::new();
        let nonce: u64 = random();
        hasher.update(&index.to_be_bytes());
        hasher.update(&prev_hash);
        hasher.update(&nonce.to_be_bytes());
        (*hasher.finalize().as_bytes(),nonce)
    }

    fn h2_u64(hash: [u8; 32]) -> u64 {
        let mut value: u64 = 0;
        for i in 0..8 {
            value |= (hash[i] as u64) << (8 * (7 - i)); // Shifting from the most significant byte to the least
        }
        value
    }
}
