use futures::{FutureExt, TryFutureExt};
mod network;
mod block;

mod blockchain;
mod mempool;
mod transactions;
mod input;
mod output;
mod miner;


#[tokio::main]
async fn main() {
    let mut miner = miner::Miner::new([0xbb;32]);
    miner.mine().await;
}
