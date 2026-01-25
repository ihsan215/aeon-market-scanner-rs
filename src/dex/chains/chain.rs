#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainId {
    ETHEREUM = 0x1,
    BSC = 0x38,
    POLYGON = 0x89,
    AVALANCHE = 0xa86a,
    ARBITRUM = 0xa4b1,
    OPTIMISM = 0xa,
    BASE = 0x2105,
    PLASMA = 0x2611,
    UNICHAIN = 0x82,
    SONIC = 0x92,
    RONIN = 0x7e4,
    HyperEVM = 0x3e7,
    LINEA = 0xe708,
    MANTLE = 0x1388,
}

impl ChainId {
    pub fn name(&self) -> &'static str {
        match self {
            ChainId::ETHEREUM => "ethereum",
            ChainId::BSC => "bsc",
            ChainId::POLYGON => "polygon",
            ChainId::AVALANCHE => "avalanche",
            ChainId::ARBITRUM => "arbitrum",
            ChainId::OPTIMISM => "optimism",
            ChainId::BASE => "base",
            ChainId::PLASMA => "plasma",
            ChainId::UNICHAIN => "unichain",
            ChainId::SONIC => "sonic",
            ChainId::RONIN => "ronin",
            ChainId::HyperEVM => "hyprevm",
            ChainId::LINEA => "linea",
            ChainId::MANTLE => "mantle",
        }
    }
}
