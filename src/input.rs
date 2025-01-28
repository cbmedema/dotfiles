use serde::{Deserialize, Serialize};
use serde_with::serde_as;


#[serde_as]
#[derive(Clone, Copy, Hash, Serialize, Deserialize)]

pub struct Input {
    pub txid: [u8;32],
    #[serde_as(as = "serde_with::Bytes")]
    pub signature: [u8;64],
}