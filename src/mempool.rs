use std::collections::BTreeSet;

use crate::block;
use crate::blockchain::Blockchain;
use crate::transactions::Tx;

pub const MAX_MEMPOOL_SIZE: u32 = 150000;

pub struct Mempool {
    // key is txid, and value is mining fee / bytes
    pub pool: BTreeSet<(u64,Tx)>,

}

impl Mempool {
    pub fn new() -> Mempool {
        Mempool { pool: BTreeSet::new()}
    }

    pub fn add_tx(&mut self, tx: Tx, chain: &Blockchain, utxos: &Vec<(u64, [u8;32])>) {
        // if mempool has space, simply add tx to pool
        if self.get_size() + tx.get_size() < MAX_MEMPOOL_SIZE {
            self.verify(tx,chain,utxos);
        }
        // otherwise, if transaction would add higher fees to mempool, we add it
        else {
            let size = tx.get_size();
            let mut sum = 0;
            let mut insertion_index= 0;
            for (index,(mfpb,ptx)) in self.pool.iter().enumerate(){
                if sum < size{
                    sum += ptx.get_size();
                }
                else {
                    // only replaces transactions if the new transaction fees are higher than the ones it is replacing
                    if tx.calc_mining_fee_per_byte(chain) > *mfpb {
                        insertion_index = index + 1;
                    }
                    break;
                }
            }
            // if the transaction fees are less than the minimum, they are not added to the chain
            if insertion_index > 0 {
                let to_remove: Vec<(u64,Tx)> = self.pool.iter().enumerate().filter(|(index,(_,_))|*index<insertion_index)
                    .map(|(index,pair)| pair.clone()).collect();
                for (pair) in to_remove.iter(){
                    self.pool.remove(pair);
                }
                self.verify(tx,chain,utxos);
            }
        }
    }

    fn verify(&mut self,tx: Tx, chain: &Blockchain, utxos: &Vec<(u64, [u8;32])>) {
        let mut unspent_txids = vec![];
        utxos.iter().for_each(|(_,tx)| unspent_txids.push(*tx));
        if tx.inputs.iter().any(|input| !unspent_txids.contains(&input.txid)){
            println!("I should totaly handle this erorr");
        }
        else {
            self.pool.insert((tx.calc_mining_fee_per_byte(chain), tx));
        }
    }
    pub fn calc_valid_tx_pool_and_fees(&mut self, chain: &Blockchain) -> (Vec<Tx>,u64) {
        let mut total_fees: u64 = 0;
        let mut transactions = vec![];
        let mut tx_pool_size: u32 = 0;
        let mut removal_pairs = vec![];

        self.pool.iter().rev().for_each(|(fee,ptx)|{
            // for each transaction in pool, we calculate mining fees
            if ptx.get_size() + tx_pool_size + 228 < block::MAX_BLOCK_SIZE {
                transactions.push(ptx.clone());
                tx_pool_size += ptx.get_size();
                total_fees += ptx.calc_mining_fee(chain);
                removal_pairs.push((*fee,ptx.clone()));
            }
        });
        for pair in removal_pairs.iter(){
            self.pool.remove(pair);
        }
        (transactions,total_fees)
    }

    pub fn get_size(&self) -> u32 { self.pool.iter().map(|(_,tx)|tx.get_size()).sum() }


}