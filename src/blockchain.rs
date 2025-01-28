use crate::block::Block;

pub struct Blockchain {
    pub chain: Vec<Block>
}

impl Blockchain {
    pub fn print(&self) {
        self.chain.iter().for_each(|block| block.print());
    }

    pub fn get_height(&self) -> u32{
        self.chain.last().unwrap().index
    }

    pub fn get_current_hash(&self) -> [u8;32] { self.chain.last().unwrap().hash }

    pub fn add_block(&mut self, candidate_block: Block) {
            self.chain.push(candidate_block);
    }

    pub fn create_from_genesis(genesis: Block) -> Blockchain {
        let mut chain = vec![];
        chain.push(genesis);
        Blockchain { chain }
    }
}