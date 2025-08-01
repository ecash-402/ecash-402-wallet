use cdk::nuts::CurrencyUnit;
use ecash_402_wallet::mint::MintClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌱 Mint Client Test Example");
    println!("==========================\n");

    let mint_url = "https://ecashmint.otrta.me";
    println!("🔗 Connecting to mint: {}", mint_url);

    let client = MintClient::new(mint_url)?;
    println!("✅ Mint client created successfully\n");

    println!("📋 Testing mint info...");
    match client.get_info().await {
        Ok(info) => {
            println!("✅ Mint Info:");
            if let Some(name) = &info.name {
                println!("   Name: {}", name);
            }
            if let Some(version) = &info.version {
                println!("   Version: {}", version);
            }
            if let Some(description) = &info.description {
                println!("   Description: {}", description);
            }
            if let Some(pubkey) = &info.pubkey {
                println!("   Pubkey: {}", pubkey);
            }
            println!("   Supported NUTs: {}", info.nuts.len());
        }
        Err(e) => {
            println!("❌ Failed to get mint info: {}", e);
        }
    }
    println!();

    println!("🔑 Testing keysets...");
    match client.get_keysets().await {
        Ok(keysets) => {
            println!("✅ Found {} keysets:", keysets.keysets.len());
            for keyset in &keysets.keysets {
                println!("   - ID: {}", keyset.id);
                println!("     Unit: {}", keyset.unit);
                println!("     Active: {}", keyset.active);
            }
        }
        Err(e) => {
            println!("❌ Failed to get keysets: {}", e);
        }
    }
    println!();

    println!("🗝️  Testing keys...");
    match client.get_keys().await {
        Ok(keys) => {
            println!("✅ Found {} keysets with keys:", keys.keysets.len());
            for keyset in &keys.keysets {
                println!("   - Keyset ID: {}", keyset.id);
                println!("     Unit: {}", keyset.unit);
                println!("     Keys count: {}", keyset.keys.len());

                if !keyset.keys.is_empty() {
                    println!("     Sample keys:");
                    let mut count = 0;
                    for (amount, pubkey) in &keyset.keys {
                        if count < 3 {
                            println!("       {}: {}...", amount, &pubkey[..20]);
                            count += 1;
                        }
                    }
                    if keyset.keys.len() > 3 {
                        println!("       ... and {} more", keyset.keys.len() - 3);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get keys: {}", e);
        }
    }
    println!();

    println!("💰 Testing mint quote request...");
    let amount = 100;
    let unit = CurrencyUnit::Sat;

    match client.request_mint_quote(amount, unit).await {
        Ok(quote) => {
            println!("✅ Mint quote created:");
            println!("   Quote ID: {}", quote.quote);
            println!("   Amount: {} sats", amount);
            println!("   Payment request: {}...", &quote.request[..50]);
            println!("   Paid: {}", quote.paid);
            println!("   Expiry: {}", quote.expiry);

            println!("\n🔍 Checking quote status...");
            match client.get_mint_quote(&quote.quote).await {
                Ok(status) => {
                    println!("✅ Quote status:");
                    println!("   Quote ID: {}", status.quote);
                    println!("   Paid: {}", status.paid);
                    println!("   Expiry: {}", status.expiry);
                }
                Err(e) => {
                    println!("❌ Failed to check quote status: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create mint quote: {}", e);
        }
    }
    println!();

    println!("🔄 Testing melt quote request...");
    let bolt11_invoice = "lnbc1000n1pn2s2pp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcqca784w";

    match client
        .request_melt_quote(bolt11_invoice, CurrencyUnit::Sat)
        .await
    {
        Ok(quote) => {
            println!("✅ Melt quote created:");
            println!("   Quote ID: {}", quote.quote);
            println!("   Amount: {} sats", quote.amount);
            println!("   Fee reserve: {} sats", quote.fee_reserve);
            println!("   Paid: {}", quote.paid);
            println!("   Expiry: {}", quote.expiry);

            println!("\n🔍 Checking melt quote status...");
            match client.get_melt_quote(&quote.quote).await {
                Ok(status) => {
                    println!("✅ Melt quote status:");
                    println!("   Quote ID: {}", status.quote);
                    println!("   Amount: {} sats", status.amount);
                    println!("   Fee reserve: {} sats", status.fee_reserve);
                    println!("   Paid: {}", status.paid);
                }
                Err(e) => {
                    println!("❌ Failed to check melt quote status: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create melt quote: {}", e);
        }
    }
    println!();

    println!("🔍 Testing proof state check...");
    let test_ys = vec![
        "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2".to_string(),
        "02c020f2e928471bf9f8b4e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5".to_string(),
    ];

    match client.check_proof_state(test_ys).await {
        Ok(response) => {
            println!("✅ Proof state check completed:");
            for state in &response.states {
                println!("   Y: {}...", &state.y[..20]);
                println!("   State: {}", state.state);
                if let Some(witness) = &state.witness {
                    println!("   Witness: {}", witness);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to check proof state: {}", e);
        }
    }
    println!();

    println!("🏁 Test completed!");
    println!("\nNote: This example tests the basic API connectivity and responses.");
    println!("For full functionality testing (minting, melting, swapping), you would need:");
    println!("- Valid Lightning invoices");
    println!("- Actual proofs to test with");
    println!("- Blinded messages for minting/swapping");

    Ok(())
}
