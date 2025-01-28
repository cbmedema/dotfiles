use std::cmp::Ordering;

use blake3;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use crate::blockchain::Blockchain;
use crate::input::Input;
use crate::output::Output;

#[derive(Clone, Hash, Serialize, Deserialize)]
pub struct Tx {
    pub txid: [u8;32],
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

impl Tx {
    pub fn generate_txid(inputs: &Vec<Input>, outputs: &Vec<Output>) -> [u8;32]{
        let mut hasher = blake3::Hasher::new();
        inputs.iter().for_each(|input|{
            hasher.update(&input.txid);
            hasher.update(&input.signature);
            //hasher.update(&input.timestamp.to_be_bytes());
        });
        outputs.iter().for_each(|output|{
            hasher.update(&output.amount.to_be_bytes());
            hasher.update(&output.address);
        });

        *hasher.finalize().as_bytes()
    }

    pub fn print(&self) {
        print!("------------------------------------------------------------\nTransaction ");
        self.txid.iter().for_each(|hex| print!("{:02x}",hex));
        for (index, input) in self.inputs.iter().enumerate(){
            println!("\n\nInput {index}");
            print!("Txid: ");
            input.txid.iter().for_each(|hex|print!("{:02x}",hex));
            print!("\nSignature: ");
            input.signature.iter().for_each(|hex|print!("{:02x}",hex));
        }
        for (index, output) in self.outputs.iter().enumerate() {
            println!("\n\nOutput {index}");
            println!("Amount: {}",output.amount.to_formatted_string(&Locale::en));
            print!("Address: ");
            output.address.iter().for_each(|hex|print!("{:02x}",hex));
        }
        println!("\n\nEnd Transaction ");
        self.txid.iter().for_each(|hex| print!("{:02x}",hex));
        println!("\n------------------------------------------------------------");
    }

    pub fn get_size(&self) -> u32{
        const TXID_BYTES: u32 = 32;
        // inputs are always 96 bytes ( 32 bytes for txid, and 64 bytes for signature)
        let input_bytes: u32 = self.inputs.iter().map(|_|96).sum();
        // outputs are always 40 bytes (8 bytes for amount, 32 bytes for address)
        let output_bytes: u32 = self.outputs.iter().map(|_|40).sum();
        TXID_BYTES + input_bytes + output_bytes
    }

    fn calc_sum_of_inputs(&self, chain: &Blockchain) -> u64{
        self.inputs.iter().flat_map(|input| {
            // for each input, we scan the chain for a corresponding output
            chain.chain.iter().flat_map(|block| {
                block.transactions.iter().flat_map(|btx| {
                    btx.outputs.iter().map(|out| {
                        // if the output matches the input, and the output is being sent to the correct address, it is the value used in input
                        if btx.txid == input.txid && VerifyingKey::from_bytes(&out.address).unwrap().verify(&input.txid, &Signature::from_bytes(&input.signature)).is_ok() {
                            out.amount
                        } else { 0 }
                    })
                })
            })
        }).sum()
    }

    fn calc_sum_of_outputs(&self) -> u64{
        self.outputs.iter().map(|out|out.amount).sum()
    }

    pub fn calc_mining_fee_per_byte(&self, chain: &Blockchain) -> u64 {
        let fee = self.calc_sum_of_inputs(chain) - self.calc_sum_of_outputs();
        let size = self.get_size();

        fee << 16 / size as u64
    }

    pub fn calc_mining_fee(&self, chain: &Blockchain) -> u64 {
        self.calc_sum_of_inputs(chain) - self.calc_sum_of_outputs()
    }
}


impl PartialEq for Tx {
    fn eq(&self, other: &Self) -> bool {
        self.txid == other.txid // Compare based on transaction ID
    }
}

impl Eq for Tx {}

impl PartialOrd for Tx {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.txid.cmp(&other.txid)) // Compare based on transaction ID
    }
}

impl Ord for Tx {
    fn cmp(&self, other: &Self) -> Ordering {
        self.txid.cmp(&other.txid) // Compare based on transaction ID
    }
}
#[derive(Debug)]
pub enum TxError{
    InsufficientBalance,
}