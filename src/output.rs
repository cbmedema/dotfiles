use serde::{Deserialize, Serialize};

#[derive(Clone, Hash, Serialize, Deserialize)]
pub struct Output {
    pub amount: u64,
    pub address: [u8;32],
}