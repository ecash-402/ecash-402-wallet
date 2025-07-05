use crate::error::Result;
use crate::nip60::Nip60Wallet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintInfo {
    pub url: String,
    pub unit: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: bool,
    pub keysets: Vec<KeysetDisplayInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetDisplayInfo {
    pub id: String,
    pub unit: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub total_sats: u64,
    pub total_msats: u64,
    pub by_mint: Vec<MintBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintBalance {
    pub mint_url: String,
    pub unit: String,
    pub amount: u64,
    pub amount_display: String,
    pub proof_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub direction: String,
    pub amount: u64,
    pub amount_display: String,
    pub mint_url: String,
    pub unit: String,
    pub timestamp: Option<u64>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint_url: String,
    pub unit: String,
    pub amount: u64,
    pub amount_display: String,
    pub proof_count: usize,
    pub memo: Option<String>,
}

pub struct WalletOperations;

impl WalletOperations {
    /// Convert amount between units and create display string
    pub fn format_amount(amount: u64, unit: &str) -> String {
        match unit {
            "msat" => format!("{} msat", amount),
            "sat" => format!("{} sat", amount),
            "usd" => format!("${:.2}", amount as f64 / 100.0),
            "eur" => format!("€{:.2}", amount as f64 / 100.0),
            _ => format!("{} {}", amount, unit),
        }
    }

    /// Convert msat to sat
    pub fn msat_to_sat(msat: u64) -> u64 {
        msat / 1000
    }

    /// Convert sat to msat
    pub fn sat_to_msat(sat: u64) -> u64 {
        sat * 1000
    }

    /// Get wallet balance with proper unit handling
    pub async fn get_detailed_balance(wallet: &Nip60Wallet) -> Result<WalletBalance> {
        let proofs = wallet.get_unspent_proofs().await?;
        let mut by_mint: HashMap<String, MintBalance> = HashMap::new();
        let mut total_sats = 0u64;
        let mut total_msats = 0u64;

        for proof in &proofs {
            let mint_url = &proof.keyset_id.to_string(); // This should be resolved to actual mint URL
            
            // Try to get the actual mint URL from keyset ID
            let actual_mint_url = Self::resolve_mint_url_from_keyset(wallet, &proof.keyset_id.to_string())
                .unwrap_or_else(|| mint_url.clone());
            
            let unit = Self::get_unit_for_mint(wallet, &actual_mint_url);
            let amount = proof.amount.to_string().parse::<u64>().unwrap_or(0);
            
            // Convert to standard units for totals
            match unit.as_str() {
                "msat" => {
                    total_msats += amount;
                    total_sats += Self::msat_to_sat(amount);
                }
                _ => {
                    total_sats += amount;
                    total_msats += Self::sat_to_msat(amount);
                }
            }
            
            let entry = by_mint.entry(actual_mint_url.clone()).or_insert_with(|| MintBalance {
                mint_url: actual_mint_url.clone(),
                unit: unit.clone(),
                amount: 0,
                amount_display: String::new(),
                proof_count: 0,
            });
            
            entry.amount += amount;
            entry.proof_count += 1;
        }

        // Update display strings
        for mint_balance in by_mint.values_mut() {
            mint_balance.amount_display = Self::format_amount(mint_balance.amount, &mint_balance.unit);
        }

        Ok(WalletBalance {
            total_sats,
            total_msats,
            by_mint: by_mint.into_values().collect(),
        })
    }

    /// Get mint information with display formatting
    pub fn get_mint_info_display(wallet: &Nip60Wallet) -> Vec<MintInfo> {
        wallet.get_all_mint_infos()
            .into_iter()
            .map(|mint| {
                let keysets = mint.keysets
                    .iter()
                    .map(|k| KeysetDisplayInfo {
                        id: k.id.clone(),
                        unit: k.unit.clone(),
                        active: k.active,
                    })
                    .collect();

                MintInfo {
                    url: mint.url.clone(),
                    unit: mint.keysets.first().map(|k| k.unit.clone()).unwrap_or_else(|| "sat".to_string()),
                    name: mint.name.clone(),
                    description: mint.description.clone(),
                    active: mint.active,
                    keysets,
                }
            })
            .collect()
    }

    /// Parse and analyze a Cashu token
    pub async fn analyze_token(wallet: &Nip60Wallet, token_string: &str) -> Result<TokenInfo> {
        let parsed_token = wallet.parse_cashu_token(token_string)?;
        let mint_url = parsed_token.mint_url()
            .map_err(|e| crate::error::Error::custom(&format!("Failed to get mint URL: {}", e)))?
            .to_string();
        
        let amount = wallet.calculate_token_amount(&parsed_token)?;
        let unit = Self::get_unit_for_mint(wallet, &mint_url);
        let amount_display = Self::format_amount(amount, &unit);
        let proof_count = parsed_token.proofs().len();
        let memo = parsed_token.memo().clone();

        Ok(TokenInfo {
            mint_url,
            unit,
            amount,
            amount_display,
            proof_count,
            memo,
        })
    }

    /// Get transaction history with proper formatting
    pub async fn get_formatted_history(wallet: &Nip60Wallet, mint_url: Option<String>) -> Result<Vec<TransactionInfo>> {
        let history = wallet.get_event_history_by_mint(mint_url).await?;
        let mut transactions = Vec::new();

        for mint_history in history {
            let unit = Self::get_unit_for_mint(wallet, &mint_history.mint);
            
            for event in mint_history.events {
                transactions.push(TransactionInfo {
                    direction: event.direction,
                    amount: event.amount,
                    amount_display: Self::format_amount(event.amount, &unit),
                    mint_url: mint_history.mint.clone(),
                    unit: unit.clone(),
                    timestamp: Some(event.timestamp),
                    memo: event.memo,
                });
            }
        }

        // Sort by timestamp (newest first)
        transactions.sort_by(|a, b| {
            match (a.timestamp, b.timestamp) {
                (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        Ok(transactions)
    }

    /// Get unit for a specific mint
    fn get_unit_for_mint(wallet: &Nip60Wallet, mint_url: &str) -> String {
        if let Some(mint_info) = wallet.get_mint_info(mint_url) {
            if let Some(keyset) = mint_info.keysets.first() {
                return keyset.unit.clone();
            }
        }
        "sat".to_string() // Default fallback
    }

    /// Resolve mint URL from keyset ID
    fn resolve_mint_url_from_keyset(wallet: &Nip60Wallet, keyset_id: &str) -> Option<String> {
        for mint_info in wallet.get_all_mint_infos() {
            for keyset in &mint_info.keysets {
                if keyset.id == keyset_id {
                    return Some(mint_info.url.clone());
                }
            }
        }
        None
    }

    /// Get wallet statistics with proper unit handling
    pub async fn get_detailed_stats(wallet: &Nip60Wallet) -> Result<DetailedWalletStats> {
        let balance = Self::get_detailed_balance(wallet).await?;
        let stats = wallet.get_stats().await?;
        let mint_infos = Self::get_mint_info_display(wallet);

        Ok(DetailedWalletStats {
            balance,
            token_events: stats.token_events,
            mint_infos,
        })
    }

    /// Format amount for display based on user preference
    pub fn display_amount_with_conversion(amount: u64, unit: &str, show_conversion: bool) -> String {
        let primary = Self::format_amount(amount, unit);
        
        if !show_conversion {
            return primary;
        }

        match unit {
            "msat" => {
                let sat_amount = Self::msat_to_sat(amount);
                format!("{} ({} sat)", primary, sat_amount)
            }
            "sat" => {
                let msat_amount = Self::sat_to_msat(amount);
                format!("{} ({} msat)", primary, msat_amount)
            }
            _ => primary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedWalletStats {
    pub balance: WalletBalance,
    pub token_events: usize,
    pub mint_infos: Vec<MintInfo>,
} 