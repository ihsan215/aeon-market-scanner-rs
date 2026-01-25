mod scanner_common;

use aeon_market_scanner_rs::dex::chains::{ChainId, Token};
use scanner_common::{
    create_base_usdc, create_bsc_bnb, create_bsc_usdc, create_bsc_usdt, create_eth_usdc,
};

#[test]
fn test_create_token_bsc_usdc() {
    println!("===== Testing Algorithmic Token Creation: BSC USDC =====\n");

    // Create USDC token on BSC algorithmically
    let usdc_token = create_bsc_usdc();

    assert_eq!(usdc_token.symbol, "USDC");
    assert_eq!(usdc_token.name, "USD Coin");
    assert_eq!(usdc_token.decimal, 18);
    assert_eq!(usdc_token.chain_id, ChainId::BSC);
    assert_eq!(
        usdc_token.address,
        "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"
    );

    println!("✓ Successfully created BSC USDC token:");
    println!("  Address: {}", usdc_token.address);
    println!("  Name: {}", usdc_token.name);
    println!("  Symbol: {}", usdc_token.symbol);
    println!("  Decimals: {}", usdc_token.decimal);
    println!("  Chain: {:?}\n", usdc_token.chain_id);
}

#[test]
fn test_create_token_bsc_bnb() {
    println!("===== Testing Algorithmic Token Creation: BSC BNB =====\n");

    // Create BNB token on BSC algorithmically
    let bnb_token = create_bsc_bnb();

    assert_eq!(bnb_token.symbol, "BNB");
    assert_eq!(bnb_token.name, "BNB");
    assert_eq!(bnb_token.decimal, 18);
    assert_eq!(bnb_token.chain_id, ChainId::BSC);
    assert_eq!(
        bnb_token.address,
        "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"
    );

    println!("✓ Successfully created BSC BNB token:");
    println!("  Address: {}", bnb_token.address);
    println!("  Name: {}", bnb_token.name);
    println!("  Symbol: {}", bnb_token.symbol);
    println!("  Decimals: {}", bnb_token.decimal);
    println!("  Chain: {:?}\n", bnb_token.chain_id);
}

#[test]
fn test_create_token_bsc_usdt() {
    println!("===== Testing Algorithmic Token Creation: BSC USDT =====\n");

    // Create USDT token on BSC algorithmically
    let usdt_token = create_bsc_usdt();

    assert_eq!(usdt_token.symbol, "USDT");
    assert_eq!(usdt_token.name, "Tether USD");
    assert_eq!(usdt_token.decimal, 18);
    assert_eq!(usdt_token.chain_id, ChainId::BSC);
    assert_eq!(
        usdt_token.address,
        "0x55d398326f99059fF775485246999027B3197955"
    );

    println!("✓ Successfully created BSC USDT token:");
    println!("  Address: {}", usdt_token.address);
    println!("  Name: {}", usdt_token.name);
    println!("  Symbol: {}", usdt_token.symbol);
    println!("  Decimals: {}", usdt_token.decimal);
    println!("  Chain: {:?}\n", usdt_token.chain_id);
}

#[test]
fn test_create_token_ethereum_usdc() {
    println!("===== Testing Algorithmic Token Creation: Ethereum USDC =====\n");

    // Create USDC token on Ethereum algorithmically
    let usdc_token = create_eth_usdc();

    assert_eq!(usdc_token.symbol, "USDC");
    assert_eq!(usdc_token.name, "USD Coin");
    assert_eq!(usdc_token.decimal, 6); // Ethereum USDC has 6 decimals
    assert_eq!(usdc_token.chain_id, ChainId::ETHEREUM);
    assert_eq!(
        usdc_token.address,
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    );

    println!("✓ Successfully created Ethereum USDC token:");
    println!("  Address: {}", usdc_token.address);
    println!("  Name: {}", usdc_token.name);
    println!("  Symbol: {}", usdc_token.symbol);
    println!("  Decimals: {}", usdc_token.decimal);
    println!("  Chain: {:?}\n", usdc_token.chain_id);
}

#[test]
fn test_create_token_base_usdc() {
    println!("===== Testing Algorithmic Token Creation: Base USDC =====\n");

    // Create USDC token on Base algorithmically
    let usdc_token = create_base_usdc();

    assert_eq!(usdc_token.symbol, "USDC");
    assert_eq!(usdc_token.name, "USDC");
    assert_eq!(usdc_token.decimal, 6);
    assert_eq!(usdc_token.chain_id, ChainId::BASE);
    assert_eq!(
        usdc_token.address,
        "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    );

    println!("✓ Successfully created Base USDC token:");
    println!("  Address: {}", usdc_token.address);
    println!("  Name: {}", usdc_token.name);
    println!("  Symbol: {}", usdc_token.symbol);
    println!("  Decimals: {}", usdc_token.decimal);
    println!("  Chain: {:?}\n", usdc_token.chain_id);
}

#[test]
fn test_create_multiple_tokens_algorithmically() {
    println!("===== Testing Multiple Token Creation Algorithmically =====\n");

    // Create multiple tokens algorithmically for BSC
    let bnb_token = create_bsc_bnb();
    let usdt_token = create_bsc_usdt();
    let usdc_token = create_bsc_usdc();
    let wbnb_token = Token::create(
        "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
        "Wrapped BNB",
        "WBNB",
        18,
        ChainId::BSC,
    );
    let btc_token = Token::create(
        "0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c",
        "Binance-Peg Bitcoin",
        "BTCB",
        18,
        ChainId::BSC,
    );

    // Verify all tokens are on BSC
    assert_eq!(bnb_token.chain_id, ChainId::BSC);
    assert_eq!(usdt_token.chain_id, ChainId::BSC);
    assert_eq!(usdc_token.chain_id, ChainId::BSC);
    assert_eq!(wbnb_token.chain_id, ChainId::BSC);
    assert_eq!(btc_token.chain_id, ChainId::BSC);

    // Verify symbols
    assert_eq!(bnb_token.symbol, "BNB");
    assert_eq!(usdt_token.symbol, "USDT");
    assert_eq!(usdc_token.symbol, "USDC");
    assert_eq!(wbnb_token.symbol, "WBNB");
    assert_eq!(btc_token.symbol, "BTCB");

    println!("✓ Successfully created {} BSC tokens algorithmically:", 5);
    println!("  - {} ({})", bnb_token.symbol, bnb_token.name);
    println!("  - {} ({})", usdt_token.symbol, usdt_token.name);
    println!("  - {} ({})", usdc_token.symbol, usdc_token.name);
    println!("  - {} ({})", wbnb_token.symbol, wbnb_token.name);
    println!("  - {} ({})\n", btc_token.symbol, btc_token.name);
}
