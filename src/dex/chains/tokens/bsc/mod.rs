use crate::create_token_provider;
use crate::dex::chains::{ChainId, Token, TokenMap};
use std::collections::HashMap;

create_token_provider!(BscTokens, ChainId::BSC, {
    TokenMap::USDT => ("0x55d398326f99059fF775485246999027B3197955", "Tether USD", "USDT", 18),
    TokenMap::BNB => ("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE", "BNB", "BNB", 18),
    TokenMap::WBNB => ("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c", "Wrapped BNB", "WBNB", 18),
    TokenMap::BTC => ("0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c", "Binance-Peg Bitcoin", "BTCB", 18),
});
