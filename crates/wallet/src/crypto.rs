use crate::error::{Error, Result};
use cdk::nuts::{BlindedMessage, Id};
use hex;
use rand::RngCore;
use secp256k1::{Secp256k1, SecretKey};
use std::collections::HashMap;
use std::str::FromStr;

/// Calculate optimal denominations for a given amount
pub fn calculate_optimal_denominations(amount: u64) -> HashMap<u64, u32> {
    let mut denominations = HashMap::new();
    let mut remaining = amount;

    let denoms = [
        16384, 8192, 4096, 2048, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1,
    ];

    for &denom in &denoms {
        if remaining >= denom {
            let count = remaining / denom;
            denominations.insert(denom, count as u32);
            remaining -= denom * count;
        }
    }

    denominations
}

/// Create blinded messages for a given amount using optimal denominations
pub fn create_blinded_messages_for_amount(
    amount: u64,
    keyset_id: &str,
) -> Result<Vec<BlindedMessage>> {
    let mut outputs = Vec::new();

    let denominations = calculate_optimal_denominations(amount);

    for (&denom, &count) in &denominations {
        for _ in 0..count {
            let blinded_msg = create_simple_blinded_message(denom, keyset_id)?;
            outputs.push(blinded_msg);
        }
    }

    Ok(outputs)
}

/// Create a simple blinded message for the mint
fn create_simple_blinded_message(amount: u64, keyset_id: &str) -> Result<BlindedMessage> {
    let secp = Secp256k1::new();

    // Generate a proper secp256k1 private key
    let mut rng = rand::rng();
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);

    let secret_key = SecretKey::from_slice(&secret_bytes)
        .map_err(|e| Error::custom(&format!("Failed to create secret key: {}", e)))?;

    // Generate the corresponding public key
    let public_key = secret_key.public_key(&secp);

    // Convert to compressed format
    let compressed_pubkey = public_key.serialize();

    // Parse keyset ID
    let keyset_id_parsed =
        Id::from_str(keyset_id).map_err(|e| Error::custom(&format!("Invalid keyset ID: {}", e)))?;

    // Create blinded message
    let blinded_msg = BlindedMessage {
        amount: amount.into(),
        keyset_id: keyset_id_parsed,
        blinded_secret: cdk::nuts::PublicKey::from_hex(hex::encode(compressed_pubkey))
            .map_err(|e| Error::custom(&format!("Failed to create blinded secret: {}", e)))?,
        witness: None,
    };

    Ok(blinded_msg)
}

/// Generate a random secret string for cashu proofs
pub fn generate_random_secret() -> String {
    let mut rng = rand::rng();
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);
    hex::encode(secret_bytes)
}
