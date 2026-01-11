use crate::create_token_provider;
use crate::dex::chains::{ChainId, Token, TokenMap};
use std::collections::HashMap;

create_token_provider!(BaseTokens, ChainId::BASE, {
    TokenMap::USDC => ("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "USDC", "USDC", 6),
    TokenMap::ETH => ("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE", "Ether", "ETH", 18),
    TokenMap::WETH => ("0x4200000000000000000000000000000000000006", "Wrapped Ether", "WETH", 18),
    TokenMap::BTC => ("0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf", "Coinbase Wrapped BTC", "cbBTC", 8),
});
