use ecash_402_wallet::error::Result;
use ecash_402_wallet::nip60::Nip60Wallet;
use nostr_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let keys = Keys::generate();
    let relays = vec!["wss://relay.damus.io", "wss://relay.primal.net"];
    let mints = vec![
        "https://mint.minibits.cash/Bitcoin".to_string(),
        "https://testnut.cashu.space".to_string(),
    ];

    println!("Creating wallet with mints...");
    let wallet = Nip60Wallet::from_config(keys, relays, mints).await?;

    println!("\nMint Information:");
    println!("=================");

    for mint_info in wallet.get_all_mint_infos() {
        println!("\nMint: {}", mint_info.url);
        println!("Name: {}", mint_info.name.as_deref().unwrap_or("Unknown"));
        println!(
            "Description: {}",
            mint_info.description.as_deref().unwrap_or("No description")
        );
        println!("Active: {}", mint_info.active);
        println!("Keysets:");

        for keyset in &mint_info.keysets {
            println!("  - ID: {}", keyset.id);
            println!("    Unit: {}", keyset.unit);
            println!("    Active: {}", keyset.active);
        }
    }

    println!("\nTesting mint-specific keyset queries:");
    for mint_url in &[
        "https://mint.minibits.cash/Bitcoin",
        "https://testnut.cashu.space",
    ] {
        let keysets = wallet.get_mint_keysets(mint_url);
        println!("\n{} has {} keysets", mint_url, keysets.len());

        let active_keysets = wallet.get_active_keysets(mint_url);
        println!("{} has {} active keysets", mint_url, active_keysets.len());
    }

    Ok(())
}
