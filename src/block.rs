use std::error::Error;
use serde::{Serialize, Deserialize};
use num_format::{Locale, ToFormattedString};
use crate::transactions::Tx;

pub const MAX_BLOCK_SIZE: u32 = 100000;
pub const BLOCK_REWARD: u32 = 5000000;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Block {
    pub index: u32,
    pub hash: [u8;32],
    pub previous_hash: [u8;32],
    pub time: u64,
    pub target: u64,
    pub nonce: u64,
    pub transactions: Vec<Tx>,
}

impl Block {
    pub fn print(&self) {
        println!("\n-------------------------------------------------------------------------------");
        print!("Block: {} ",self.index);
        self.hash.iter().for_each(|hex|print!("{:02x}",hex));
        println!("\nHeader Data: ");
        print!("\nPrevious block: ");
        self.previous_hash.iter().for_each(|hex|print!("{:02x}",hex));
        println!("\nUnix Timestamp: {}",self.time);
        println!("Target {:016x}", self.target);
        println!("Nonce: {:016x}", self.nonce);
        println!("\nTransactions: ");
        self.transactions.iter().for_each(|transaction| transaction.print());
        println!("\n\nTotal block size: {} Bytes",self.get_size().to_formatted_string(&Locale::en));
        println!("\nEnd Block: {}",self.index);
        println!("-------------------------------------------------------------------------------");
    }

    pub fn get_size(&self) -> u32{
        const HEADER_BYTES: u32 = 92;
        let tx_bytes: u32 = self.transactions.iter().map(|tx|tx.get_size()).sum();
        HEADER_BYTES + tx_bytes
    }

    pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(self)?)
    }

    // Deserialize a JSON string back into a Block
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_str(json_str)?)
    }
}