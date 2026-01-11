use crate::create_token_provider;
use crate::dex::chains::{ChainId, Token, TokenMap};
use std::collections::HashMap;

create_token_provider!(EthereumTokens, ChainId::ETHEREUM, {
    TokenMap::USDT => ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "Tether USD", "USDT", 6),
    TokenMap::ETH => ("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE", "Ether", "ETH", 18),
    TokenMap::WETH => ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "Wrapped Ether", "WETH", 18),
    TokenMap::BTC => ("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "Wrapped BTC", "WBTC", 8),
});
