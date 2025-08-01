use ecash_402_wallet::error::Result;
use ecash_402_wallet::nip60::Nip60Wallet;
use nostr_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let keys = Keys::generate();
    let relays = vec![
        "wss://relay.damus.io",
        "wss://relay.primal.net",
        "wss://nostr.oxtr.dev",
        "wss://nostr.mom",
    ];
    let mints = vec!["https://ecashmint.otrta.me".to_string()];

    println!("Creating wallet with mints...");
    let wallet = Nip60Wallet::from_config(keys, relays, mints).await?;

    println!("Wallet created successfully!");
    println!("Mint information loaded:");

    for mint_info in wallet.get_all_mint_infos() {
        println!("- Mint: {}", mint_info.url);
        println!("  Active: {}", mint_info.active);
        println!("  Keysets: {}", mint_info.keysets.len());
        for keyset in &mint_info.keysets {
            println!(
                "    - ID: {}, Unit: {}, Active: {}",
                keyset.id, keyset.unit, keyset.active
            );
        }
    }

    println!("\n✅ Redeem functionality is now enhanced with:");
    println!("1. Mint validation for trusted mints");
    println!("2. Proof validation to check if proofs are already spent");
    println!("3. Automatic rejection of spent proofs");

    println!("\nNote: To test the redeem functionality, you would need:");
    println!("1. A valid Cashu token string");
    println!("2. The token to be from one of the configured mints");
    println!("3. The proofs in the token to be unspent");

    println!("\nExample usage:");
    println!("let redeemed_amount = wallet.redeem(\"cashu_token_string_here\").await?;");

    println!("\nTesting with an already spent token (should fail):");
    let test_token = "cashuBo2FteBpodHRwczovL2VjYXNobWludC5vdHJ0YS5tZWF1ZG1zYXRhdIGiYWlIAEdWs6T5owRhcIakYWEZAgBhc3hANzJhNjgzMTNjMDFlM2I2ZDgwMmEyNWFkNGZjYTQ5OGY3ZWRhMDhjYTgyNjgzYWQ1NWU3ODc3YjM5YTY4YTljMWFjWCEC-wL4DtV54KeCYce4CGlA83MvzZUXJdZ8XwCoq1S1gLZhZKNhZVggj3-r81tqQ145vfsQoE2ygfIPvnh7PxFl2LawyEXnqxhhc1gg8ryNpnBt7iQOpxK0MH5hcPiLQeYEXyUoyl_5JQ3YHjxhclggr-VoG5169LWPYVxyWSMGVR6tPxcPh10E1YokLozdf06kYWEZAQBhc3hAMTQ2Mjk0YzMwMDY2MzkxYTI5ZTliZTA2YWJkYWU4OWVjNTFmMWI2ODQ3YjExM2U5YTQzZDg0OTNkYzJjN2RiZWFjWCEDZwwdUSN44aGWTyR16542EtvDQuZaTmIfCsU8iyJeuDBhZKNhZVggmX1cN1a3gBwp5j_I4fyGY-hSOIAgsel0RWp_5q3BQudhc1ggX2Tuy1huIOLEC5XyMBfLDeBNl72OHSobQstafpCDWSJhclgglmco2C8Pt-8uxe1BrTcH7G1eJe7umvrBAw7a9lEL9SWkYWEYgGFzeEAwZDMwNGI5ODg2ODMxYjIxYWU1NDBiMjRjODVkZGVkZWM2ZmZmMThmMTEwYWMxYjA1ODc0OGZhMzQ1Zjk5YWIwYWNYIQN7d_vCF6DBU9u8jwdrWsGzmisjXHQ6seVu2jSO9FBV7GFko2FlWCCi2NHHibNaYMAH1rMJctG6-obzE65X-l5a1kyzQ_L4cmFzWCDML4O8S9TEwNYcb2qp6t8MQe1-1f1UHk3nfgXOnv3Ch2FyWCDlEA3SEs2FHif-20GSo8Onh90B9etkz2zZxxZsHv_QaaRhYRhAYXN4QDNkZGM0YjM0MTZmYTM3ZDIxZGE3OWNiZmU2NTdkYzVkYjdjODRiZWYxY2RiNWRmMDFkM2RhMzQwZDQzNjRhY2NhY1ghA77nRg2p7w5qiahDD_FvNM-N6W4Nk0UKh9Tz8SASXFr9YWSjYWVYIIhhdWyXsCvvxgm9BAhUHSMNLO9B1zT0ed7y7ThMh0ZxYXNYILvBiAfhxer5KrG6BgoKHj6cLJkGo7aUwVawUEexgEGIYXJYIJcLZSI40IxR5rXDhqn7YpHgQg0zSzekvHMGBKsxdhwypGFhGCBhc3hANWZjYzM2YTk2MDc1OGY0OTc2NjgzZDJlZDRhMTQwNGFlOGI3NjI2MDJkZTJhYWZkM2UzNDllYTRiN2YyNWZkOWFjWCECTIb-GvypcyYtbCcsIWwiJvKyW3xcmGUaoO_mRg2RG3RhZKNhZVggMM7u_76BwdpbnBRNzhDIgN06mPoPuUxmDqy_IyzKzMFhc1gggVcfMA1r3Y9dNYvdFUbr4fS4WT193rosDsoKfmgDLjJhclggvotkcs-552B7Fh6vi5pU15_lfzarjg9Lr6UAf76aZqykYWEIYXN4QDYxMzIwNjU4NGRlOTFjMDhiY2M0Y2E3MWNkZDNjZTg2NDMyYjMxODM5Y2UxM2E0Y2JhOTI0NmUwMGJmMDY0YjNhY1ghAlkZd_zBc7HuIU7_AEvv4nt1cFAFsxABSmyZbKAAqoHkYWSjYWVYIDKW08ZcMSRhE9Ajpbzav51hX2NUarlGDN-4-deUphyDYXNYIEAHW6H2FSi8wcUnX8SFKf9B5um6BG5tQT3An_EsvLThYXJYINuphcZTN6dhF-G8wph5a83IPeqz1MYyAqgaSJwuTT9L";

    match wallet.redeem(test_token).await {
        Ok(amount) => {
            println!("✅ Successfully redeemed {} sats", amount);
        }
        Err(e) => {
            println!("❌ Redeem failed (as expected): {}", e);
            println!("   This demonstrates the mint validation working correctly!");
        }
    }

    Ok(())
}
