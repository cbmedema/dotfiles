use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use libp2p::gossipsub;
use num_format::Locale::se;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::network;

pub struct Node {
    chain: Arc<Mutex<Blockchain>>,
    pool: Mempool,
    utxos: HashMap<[u8;32],(u64,[u8;32])>
}


impl Node {
    pub fn new() -> Node {
        let genesis_block = Block {index: 0, hash: [8;32], previous_hash: [0;32], transactions: Vec::new(),
            time: 0, nonce: 420, target: 2u64.pow(64-24)};
        let chain_v = vec![genesis_block];
        let initial_chain = Blockchain { chain: chain_v};

        Node {chain: Arc::new(Mutex::new(initial_chain)), pool: Mempool::new(), utxos: HashMap::new()}
    }

    pub async fn send_recv_consensus(&mut self) {
        let mut swarm = network::GossipSwarm::new().unwrap();
        let topic_consensus = gossipsub::IdentTopic::new("consensus-block");
        swarm.subscribe(topic_consensus.clone()).unwrap();
        swarm.publish(topic_consensus).unwrap();

        let (mut tx, mut rx) = mpsc::channel(32);
        let swarm_mutex = Arc::new(Mutex::new(swarm));
        let chain_mutex = Arc::clone(&self.chain);


        let send_message = {
            let swarm_mutex = Arc::clone(&swarm_mutex);
            let chain_mutex = Arc::clone(&chain_mutex);
            tokio::spawn(async move {
                loop {
                    let consensus_block = {
                        let chain_lock = chain_mutex.lock().await;
                        chain_lock.chain.last().unwrap().clone()
                    };
                    if let Err(e) = tx.send(consensus_block.clone()).await {
                        eprintln!("Tx Error {e}");
                    } else {
                        let mut swarm_lock = swarm_mutex.lock().await;
                        if let Err(pub_e) = swarm_lock.send_message(&mut rx).await {
                            eprintln!("Publishing Error {pub_e}");
                        } else {
                            // if block is sent successfully, its index is updated
                            println!("Sent Block {} successfully ", consensus_block.index);
                        }
                    }
                }
            })
        };


        let handle_events = {
            let swarm_mutex = Arc::clone(&swarm_mutex);
            let chain_mutex = Arc::clone(&chain_mutex);

            tokio::spawn(async move {
                loop {
                    let events = {
                        let mut swarm_lock = swarm_mutex.lock().await;
                        swarm_lock.handle_events().await
                    };
                    match events {
                        Ok(Some(blk)) => {
                            let mut chain_lock = chain_mutex.lock().await;
                            if blk.index > chain_lock.chain.last().unwrap().index {
                                chain_lock.add_block(blk.clone());
                                println!("Received new block from peer");
                                blk.print();
                            } else {
                                println!("Received consensus block from peer! ");
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
        let _ = tokio::join!(handle_events, send_message);
    }
}