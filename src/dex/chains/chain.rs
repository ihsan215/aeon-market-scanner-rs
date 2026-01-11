#[derive(Debug, Clone)]
pub enum ChainId {
    ETHEREUM = 0x1,
    BSC = 0x38,
    BASE = 0x2105,
}

impl ChainId {
    pub fn name(&self) -> &'static str {
        match self {
            ChainId::ETHEREUM => "ethereum",
            ChainId::BSC => "bsc",
            ChainId::BASE => "base",
        }
    }
}
