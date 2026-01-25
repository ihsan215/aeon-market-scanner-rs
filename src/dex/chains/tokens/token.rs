use crate::dex::chains::ChainId;

#[derive(Debug, Clone)]
pub struct Token {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimal: u8,
    pub chain_id: ChainId,
}

impl Token {
    pub fn new(
        address: String,
        name: String,
        symbol: String,
        decimal: u8,
        chain_id: ChainId,
    ) -> Self {
        Self {
            address,
            name,
            symbol,
            decimal,
            chain_id,
        }
    }

    /// Creates a token generically with provided parameters
    /// This allows dynamic token creation without hard-coded token providers or enums
    pub fn create(address: &str, name: &str, symbol: &str, decimal: u8, chain_id: ChainId) -> Self {
        Self::new(
            address.to_string(),
            name.to_string(),
            symbol.to_string(),
            decimal,
            chain_id,
        )
    }
}
