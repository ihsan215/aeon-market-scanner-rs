use crate::dex::chains::ChainId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenMap {
    USDT,
    USDC,
    BTC,

    // Wrapped native
    WETH,
    WBNB,

    // Native
    BNB, // bnb
    ETH, // eth & base
}
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
}

// Token provider macro
#[macro_export]
macro_rules! create_token_provider {
    ($struct_name:ident, $chain_id:expr, { $($token_map:expr => ($address:expr, $name:expr, $symbol:expr, $decimals:expr)),* $(,)? }) => {
        #[derive(Debug, Clone)]
        pub struct $struct_name {
            tokens: HashMap<TokenMap, Token>,
        }

        impl $struct_name {
            pub fn new() -> Self {
                let mut tokens = HashMap::new();

                $(
                    tokens.insert($token_map, Token {
                        address: $address.to_string(),
                        name: $name.to_string(),
                        symbol: $symbol.to_string(),
                        decimal: $decimals,
                        chain_id: $chain_id,
                    });
                )*

                Self { tokens }
            }

            pub fn get(&self, token_map: &TokenMap) -> Option<&Token> {
                self.tokens.get(token_map)
            }
        }
    };
}
