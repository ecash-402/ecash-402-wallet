use cashu::CurrencyUnit;
use clap::{Parser, Subcommand};
use ecash_402_wallet::nip60::Nip60Wallet;
use ecash_402_wallet::wallet_operations::WalletOperations;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalConfig {
    pub relays: Vec<String>,
    pub mints: Vec<MintInfo>,
    pub default_private_key: Option<String>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            relays: vec![
                "wss://relay.damus.io".to_string(),
                "wss://nostr.wine".to_string(),
            ],
            mints: vec![],
            default_private_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMint {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintInfo {
    pub url: String,
    pub unit: String,
}

fn default_unit() -> String {
    "sat".to_string()
}

impl Default for MintInfo {
    fn default() -> Self {
        Self {
            url: String::new(),
            unit: default_unit(),
        }
    }
}

impl MintInfo {
    pub fn new(url: String, unit: String) -> Self {
        // Validate and normalize the unit
        let unit = match unit.to_lowercase().as_str() {
            "sat" | "sats" => "sat".to_string(),
            "msat" | "msats" => "msat".to_string(),
            _ => default_unit(), // fallback to sat for unknown units
        };

        Self { url, unit }
    }

    pub fn currency_unit(&self) -> CurrencyUnit {
        match self.unit.as_str() {
            "msat" => CurrencyUnit::Msat,
            _ => CurrencyUnit::Sat,
        }
    }
}

impl LocalConfig {
    fn config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
        Ok(home_dir.join(".config").join("nip60"))
    }

    fn config_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(Self::config_dir()?.join(".config.yaml"))
    }

    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_file = Self::config_file()?;
        if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            println!("{:?}", content);
            let config: LocalConfig = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = Self::config_dir()?;
        fs::create_dir_all(&config_dir)?;

        let config_file = Self::config_file()?;
        let content = serde_yaml::to_string(self)?;
        fs::write(&config_file, content)?;

        println!("Configuration saved to: {}", config_file.display());
        Ok(())
    }

    fn merge_with_args(
        &self,
        relays: Vec<String>,
        mints: Vec<String>,
        private_key: Option<String>,
    ) -> (Vec<String>, Vec<String>, Option<String>) {
        let final_relays = if relays.is_empty() {
            self.relays.clone()
        } else {
            relays
        };

        let final_mints = if mints.is_empty() {
            self.mints.iter().map(|m| m.url.clone()).collect()
        } else {
            mints
        };

        let final_key = private_key.or_else(|| self.default_private_key.clone());

        (final_relays, final_mints, final_key)
    }
}

#[derive(Parser)]
#[command(name = "nip60-cli")]
#[command(about = "A CLI tool for testing NIP-60 wallet functionality")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new wallet
    New {
        #[arg(
            short,
            long,
            help = "Private key in nsec format (generates new one if not provided, uses local config if available)"
        )]
        private_key: Option<String>,
        #[arg(
            short,
            long,
            help = "Nostr relay URLs (uses local config if not specified)"
        )]
        relays: Vec<String>,
        #[arg(
            short,
            long,
            help = "Cashu mint URLs (uses local config if not specified)"
        )]
        mints: Vec<String>,
    },
    /// Load wallet from existing configuration
    FromConfig {
        #[arg(short, long, help = "Private key in nsec format or hex")]
        private_key: String,
        #[arg(short, long, help = "Nostr relay URLs")]
        relays: Vec<String>,
        #[arg(short, long, help = "Cashu mint URLs (can specify multiple)")]
        mints: Vec<String>,
    },
    /// Load wallet from Nostr
    LoadFromNostr {},
    /// Get wallet balance
    GetBalance {},
    /// Calculate balance
    CalculateBalance {},
    /// Get wallet state
    GetWalletState {},
    /// Get unspent proofs
    GetUnspentProofs {},
    /// Calculate balance from history
    CalculateBalanceFromHistory {},
    /// Fetch token events
    FetchTokenEvents {},
    /// Get spending history
    GetSpendingHistory {},
    /// Get wallet stats
    GetStats {},
    /// Parse cashu token
    ParseCashuToken {
        #[arg(short, long)]
        token: String,
    },
    /// Calculate token amount
    CalculateTokenAmount {
        #[arg(short, long)]
        token: String,
    },
    /// Record receive
    RecordReceive {
        #[arg(short, long)]
        token: String,
    },
    /// Record spend
    RecordSpend {
        #[arg(short, long)]
        amount: u64,
        #[arg(short, long)]
        token_ids: Vec<String>,
    },
    /// Send to public key
    SendToPubkey {
        #[arg(short, long)]
        recipient: String,
        #[arg(short, long)]
        amount: u64,
        #[arg(short, long)]
        memo: Option<String>,
    },
    /// Send to self
    SendToSelf {
        #[arg(short, long)]
        amount: u64,
        #[arg(short, long)]
        memo: Option<String>,
    },
    /// Send token string to self
    SendTokenStringToSelf {
        #[arg(short, long)]
        token: String,
    },
    /// Send (create token)
    Send {
        #[arg(short, long)]
        amount: u64,
        #[arg(short, long)]
        memo: Option<String>,
    },
    /// Send with target mint
    SendWithTargetMint {
        #[arg(short, long)]
        amount: u64,
        #[arg(short, long)]
        target_mint: Option<String>,
        #[arg(short, long)]
        memo: Option<String>,
    },
    /// Redeem token
    Redeem {
        #[arg(short, long)]
        token: String,
    },
    /// Check incoming tokens
    CheckIncomingTokens {},
    /// Get config
    GetConfig {},
    /// Update config
    UpdateConfig {
        #[arg(short, long)]
        private_key: String,
        #[arg(short, long)]
        relays: Vec<String>,
        #[arg(short, long)]
        mints: Option<Vec<String>>,
    },
    /// Show local configuration
    ShowLocalConfig,
    /// Set local configuration
    SetLocalConfig {
        #[arg(short, long, help = "Nostr relay URLs (overwrites existing)")]
        relays: Vec<String>,
        #[arg(short, long, help = "Cashu mint URLs (overwrites existing)")]
        mints: Vec<String>,
        #[arg(short, long, help = "Default private key in nsec format or hex")]
        default_private_key: Option<String>,
    },
    /// Add to local configuration
    AddToLocalConfig {
        #[arg(long, help = "Add relay URL")]
        relay: Option<String>,
        #[arg(long, help = "Add mint URL")]
        mint: Option<String>,
    },
    /// Remove from local configuration
    RemoveFromLocalConfig {
        #[arg(long, help = "Remove relay URL")]
        relay: Option<String>,
        #[arg(long, help = "Remove mint URL")]
        mint: Option<String>,
        #[arg(long, help = "Clear default private key", action)]
        clear_key: bool,
    },
    /// Get event history by mint
    GetEventHistoryByMint {
        #[arg(short, long, help = "Specific mint URL to filter by")]
        mint: Option<String>,
    },
    /// Get proof breakdown by mint
    GetProofBreakdown,
    /// Show mint information
    ShowMintInfo {
        #[arg(short, long, help = "Specific mint URL to show info for")]
        mint: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            private_key,
            relays,
            mints,
        } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let (final_relays, final_mints, final_private_key) =
                local_config.merge_with_args(relays, mints, private_key);

            if final_relays.is_empty() {
                println!(
                    "Error: At least one relay must be specified (use --relays or set local config)"
                );
                return Ok(());
            }

            if final_mints.is_empty() {
                println!(
                    "Error: At least one mint URL must be specified (use --mints or set local config)"
                );
                return Ok(());
            }

            let (keys, is_generated) = if let Some(sk) = final_private_key {
                println!("Using provided private key...");
                match Keys::from_str(&sk) {
                    Ok(keys) => (keys, false),
                    Err(e) => {
                        println!(
                            "Error: Invalid private key format. Expected nsec format or hex: {}",
                            e
                        );
                        return Ok(());
                    }
                }
            } else {
                println!("Generating new private key...");
                (Keys::generate(), true)
            };

            let relay_refs: Vec<&str> = final_relays.iter().map(|s| s.as_str()).collect();

            println!("\nWallet Configuration:");
            println!(
                "  Public key (npub): {}",
                keys.public_key()
                    .to_bech32()
                    .unwrap_or_else(|_| keys.public_key().to_hex())
            );
            if is_generated {
                println!(
                    "  Private key (nsec): {}",
                    keys.secret_key()
                        .to_bech32()
                        .unwrap_or_else(|_| keys.secret_key().display_secret().to_string())
                );
                println!("  ⚠️  SAVE YOUR PRIVATE KEY - it won't be shown again!");
            }
            println!("  Relays: {:?}", final_relays);
            println!("  Mints: {:?}", final_mints);

            println!("\nCreating wallet on Nostr...");
            let keys_backup = if is_generated {
                Some(keys.clone())
            } else {
                None
            };
            match Nip60Wallet::new(keys, relay_refs, final_mints.clone()).await {
                Ok(wallet) => {
                    println!("✅ Wallet created successfully!");
                    println!("Config published to relays: {:?}", wallet.get_config());
                }
                Err(e) => {
                    println!("❌ Failed to create wallet: {:?}", e);
                    if let Some(backup_keys) = keys_backup {
                        println!(
                            "Your generated private key (save this!): {}",
                            backup_keys
                                .secret_key()
                                .to_bech32()
                                .unwrap_or_else(|_| backup_keys
                                    .secret_key()
                                    .display_secret()
                                    .to_string())
                        );
                    }
                }
            }
        }

        Commands::FromConfig {
            private_key,
            relays,
            mints,
        } => {
            // Load local config and merge with command line args
            let local_config = LocalConfig::load().unwrap_or_default();
            let (final_relays, final_mints, final_private_key) =
                local_config.merge_with_args(relays, mints, Some(private_key));

            let keys = Keys::from_str(&final_private_key.unwrap())?;
            let relay_refs: Vec<&str> = final_relays.iter().map(|s| s.as_str()).collect();

            println!("Loading wallet from config...");
            println!("  Relays: {:?}", final_relays);
            println!("  Mints: {:?}", final_mints);
            let wallet = Nip60Wallet::from_config(keys, relay_refs, final_mints).await?;
            println!("Wallet loaded successfully!");
            println!("Config: {:?}", wallet.get_config());
        }

        Commands::LoadFromNostr {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            println!("Loading wallet from Nostr...");
            match Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                Some(wallet) => {
                    println!("Wallet loaded from Nostr!");
                    println!("Config: {:?}", wallet.get_config());
                }
                None => {
                    println!("No wallet found on Nostr");
                }
            }
        }

        Commands::GetBalance {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let detailed_balance = WalletOperations::get_detailed_balance(&wallet).await?;

                println!("=== Wallet Balance ===");
                println!(
                    "Total: {} sat ({} msat)",
                    detailed_balance.total_sats, detailed_balance.total_msats
                );
                println!("\nBy Mint:");
                for mint_balance in &detailed_balance.by_mint {
                    println!("  • {}", mint_balance.mint_url);
                    println!(
                        "    Amount: {}",
                        WalletOperations::display_amount_with_conversion(
                            mint_balance.amount,
                            &mint_balance.unit,
                            true
                        )
                    );
                    println!("    Proofs: {}", mint_balance.proof_count);
                }

                if detailed_balance.by_mint.is_empty() {
                    println!("  No unspent proofs found");
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::CalculateBalance {} => {
            let local_config = LocalConfig::load().unwrap_or_default();

            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let balance = wallet.calculate_balance().await?;
                println!("Calculated balance: {} sats", balance);
            } else {
                println!("No wallet found");
            }
        }

        Commands::GetWalletState {} => {
            let local_config = LocalConfig::load().unwrap_or_default();

            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let state = wallet.get_wallet_state().await?;
                println!("Wallet state:");
                println!("  Balance: {} sats", state.balance);
                println!("  Proofs count: {}", state.proofs.len());
                println!("  Event mappings: {}", state.proof_to_event_id.len());
            } else {
                println!("No wallet found");
            }
        }

        Commands::GetUnspentProofs {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let detailed_balance = WalletOperations::get_detailed_balance(&wallet).await?;
                println!("=== Unspent Proofs ===");
                println!(
                    "Total proofs: {}",
                    detailed_balance
                        .by_mint
                        .iter()
                        .map(|m| m.proof_count)
                        .sum::<usize>()
                );

                for mint_balance in &detailed_balance.by_mint {
                    println!("\n• {}", mint_balance.mint_url);
                    println!("  Proofs: {}", mint_balance.proof_count);
                    println!(
                        "  Amount: {}",
                        WalletOperations::display_amount_with_conversion(
                            mint_balance.amount,
                            &mint_balance.unit,
                            true
                        )
                    );
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::CalculateBalanceFromHistory {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let (total_in, total_out, net_balance, history) =
                    wallet.calculate_balance_from_history().await?;
                println!("Balance from history:");
                println!("  Total in: {} sats", total_in);
                println!("  Total out: {} sats", total_out);
                println!("  Net balance: {} sats", net_balance);
                println!("  History entries: {}", history.len());
            } else {
                println!("No wallet found");
            }
        }

        Commands::FetchTokenEvents {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let events = wallet.fetch_token_events().await?;
                println!("Token events: {} events", events.len());
                for (i, event) in events.iter().enumerate() {
                    println!(
                        "  Event {}: {} proofs from mint {}",
                        i + 1,
                        event.data.proofs.len(),
                        event.data.mint
                    );
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::GetSpendingHistory {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let history = wallet.get_spending_history().await?;
                println!("{:?}", history);
                println!("Spending history: {} entries", history.len());
                for (i, entry) in history.iter().enumerate() {
                    println!("{:?}", entry);
                    println!("  Entry {}: {} sats", i + 1, entry.amount);
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::GetStats {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let stats = WalletOperations::get_detailed_stats(&wallet).await?;

                println!("=== Wallet Statistics ===");
                println!(
                    "Total Balance: {} sat ({} msat)",
                    stats.balance.total_sats, stats.balance.total_msats
                );
                println!("Token Events: {}", stats.token_events);

                println!("\n=== Mints ===");
                for mint in &stats.mint_infos {
                    println!("• {}", mint.url);
                    println!("  Name: {}", mint.name.as_deref().unwrap_or("Unknown"));
                    println!("  Unit: {}", mint.unit);
                    println!("  Active: {}", mint.active);
                    println!("  Keysets: {}", mint.keysets.len());
                    for keyset in &mint.keysets {
                        println!(
                            "    - {} ({}) - Active: {}",
                            keyset.id, keyset.unit, keyset.active
                        );
                    }
                    println!();
                }

                println!("=== Balance by Mint ===");
                for mint_balance in &stats.balance.by_mint {
                    println!("• {}", mint_balance.mint_url);
                    println!(
                        "  Amount: {}",
                        WalletOperations::display_amount_with_conversion(
                            mint_balance.amount,
                            &mint_balance.unit,
                            true
                        )
                    );
                    println!("  Proofs: {}", mint_balance.proof_count);
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::ParseCashuToken { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token_info = WalletOperations::analyze_token(&wallet, &token).await?;
                println!("=== Token Analysis ===");
                println!("Mint: {}", token_info.mint_url);
                println!("Unit: {}", token_info.unit);
                println!(
                    "Amount: {}",
                    WalletOperations::display_amount_with_conversion(
                        token_info.amount,
                        &token_info.unit,
                        true
                    )
                );
                println!("Proofs: {}", token_info.proof_count);
                println!("Memo: {}", token_info.memo.as_deref().unwrap_or("None"));
            } else {
                println!("No wallet found");
            }
        }

        Commands::CalculateTokenAmount { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token_info = WalletOperations::analyze_token(&wallet, &token).await?;
                println!("=== Token Amount ===");
                println!("Mint: {}", token_info.mint_url);
                println!(
                    "Amount: {}",
                    WalletOperations::display_amount_with_conversion(
                        token_info.amount,
                        &token_info.unit,
                        true
                    )
                );
            } else {
                println!("No wallet found");
            }
        }

        Commands::RecordReceive { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                wallet.record_receive(&token).await?;
                println!("Receive recorded successfully");
            } else {
                println!("No wallet found");
            }
        }

        Commands::RecordSpend { amount, token_ids } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let event_ids: Vec<EventId> = token_ids
                    .iter()
                    .map(|id| EventId::from_hex(id))
                    .collect::<Result<_, _>>()?;
                wallet.record_spend(amount, event_ids, vec![]).await?;
                println!("Spend recorded successfully");
            } else {
                println!("No wallet found");
            }
        }

        Commands::SendToPubkey {
            recipient,
            amount,
            memo,
        } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();
            let recipient_pk = PublicKey::from_str(&recipient)?;

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token = wallet
                    .send_to_pubkey(recipient_pk, amount, memo.map(String::from))
                    .await?;
                println!("Token sent successfully: {}", token);
            } else {
                println!("No wallet found");
            }
        }

        Commands::SendToSelf { amount, memo } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token = wallet.send_to_self(amount, memo.map(String::from)).await?;
                println!("Token created successfully: {}", token);
            } else {
                println!("No wallet found");
            }
        }

        Commands::SendTokenStringToSelf { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let new_token = wallet.send_token_string_to_self(&token).await?;
                println!("Token created successfully: {}", new_token);
            } else {
                println!("No wallet found");
            }
        }

        Commands::Send { amount, memo } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                println!("=== Creating Token ===");
                println!("Amount: {} sat", amount);
                if let Some(memo_text) = &memo {
                    println!("Memo: {}", memo_text);
                }

                match wallet.send(amount, memo.map(String::from)).await {
                    Ok(token) => {
                        println!("✅ Token created successfully!");
                        println!("Token: {}", token);

                        // Analyze the created token
                        if let Ok(token_info) =
                            WalletOperations::analyze_token(&wallet, &token).await
                        {
                            println!("\n=== Token Details ===");
                            println!("Mint: {}", token_info.mint_url);
                            println!(
                                "Amount: {}",
                                WalletOperations::display_amount_with_conversion(
                                    token_info.amount,
                                    &token_info.unit,
                                    true
                                )
                            );
                            println!("Proofs: {}", token_info.proof_count);
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to create token: {}", e);
                    }
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::SendWithTargetMint {
            amount,
            target_mint,
            memo,
        } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token = wallet
                    .send_with_target_mint(
                        amount,
                        target_mint.map(String::from),
                        memo.map(String::from),
                    )
                    .await?;
                println!("Token created successfully: {}", token);
            } else {
                println!("No wallet found");
            }
        }

        Commands::Redeem { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token_info = WalletOperations::analyze_token(&wallet, &token).await?;

                println!("=== Redeeming Token ===");
                println!("Mint: {}", token_info.mint_url);
                println!(
                    "Amount: {}",
                    WalletOperations::display_amount_with_conversion(
                        token_info.amount,
                        &token_info.unit,
                        true
                    )
                );
                println!("Proofs: {}", token_info.proof_count);

                match wallet.redeem(&token).await {
                    Ok(_) => {
                        println!(
                            "✅ Successfully redeemed {}",
                            WalletOperations::display_amount_with_conversion(
                                token_info.amount,
                                &token_info.unit,
                                true
                            )
                        );
                    }
                    Err(e) => {
                        println!("❌ Failed to redeem token: {}", e);
                    }
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::CheckIncomingTokens {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let tokens = wallet.check_incoming_tokens().await?;
                println!("Found {} incoming tokens", tokens.len());
                for (i, (_, _, amount)) in tokens.iter().enumerate() {
                    println!("  Token {}: {} sats", i + 1, amount);
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::GetConfig {} => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let config = wallet.get_config();
                println!("Wallet configuration:");
                println!("  Relays: {:?}", relay_refs);
                println!("  Mints: {:?}", config.mints);
            } else {
                println!("No wallet found");
            }
        }

        Commands::UpdateConfig {
            private_key,
            relays,
            mints,
        } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let (final_relays, final_mints, final_private_key) =
                local_config.merge_with_args(relays, mints.unwrap_or_default(), Some(private_key));

            let keys = Keys::from_str(&final_private_key.unwrap())?;
            let relay_refs: Vec<&str> = final_relays.iter().map(|s| s.as_str()).collect();

            if let Some(mut wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await?
            {
                wallet.update_config(Some(final_mints)).await?;
                println!("Configuration updated successfully");
                let config = wallet.get_config();
                println!("New configuration:");
                println!("  Relays: {:?}", relay_refs);
                println!("  Mints: {:?}", config.mints);
            } else {
                println!("No wallet found");
            }
        }

        Commands::ShowLocalConfig => match LocalConfig::load() {
            Ok(config) => {
                println!("Current local configuration:");
                println!("\nRelays:");
                for relay in &config.relays {
                    println!("  - {}", relay);
                }
                println!("\nMints:");
                for mint in &config.mints {
                    println!("  - URL: {}", mint.url);
                    println!("    Unit: {}", mint.unit);
                }
                if let Some(_) = config.default_private_key {
                    println!("\nDefault private key: [SET]");
                } else {
                    println!("\nDefault private key: [NOT SET]");
                }
            }
            Err(e) => {
                println!("Failed to load local config: {}", e);
            }
        },

        Commands::SetLocalConfig {
            relays,
            mints,
            default_private_key,
        } => {
            let mut config = LocalConfig::default();
            config.relays = relays;
            config.mints = mints
                .into_iter()
                .map(|url| MintInfo {
                    url,
                    unit: "sat".to_string(),
                })
                .collect();
            config.default_private_key = default_private_key;
            config.save()?;
        }

        Commands::AddToLocalConfig { relay, mint } => {
            let mut config = LocalConfig::load().unwrap_or_default();
            let mut updated = false;

            if let Some(relay_url) = relay {
                if !config.relays.contains(&relay_url) {
                    config.relays.push(relay_url.clone());
                    println!("Added relay: {}", relay_url);
                    updated = true;
                }
            }

            if let Some(mint_url) = mint {
                if !config.mints.iter().any(|m| m.url == mint_url) {
                    config.mints.push(MintInfo {
                        url: mint_url.clone(),
                        unit: "sat".to_string(),
                    });
                    println!("Added mint: {}", mint_url);
                    updated = true;
                }
            }

            if updated {
                config.save()?;
            } else {
                println!("No changes needed");
            }
        }

        Commands::RemoveFromLocalConfig {
            relay,
            mint,
            clear_key,
        } => match LocalConfig::load() {
            Ok(mut config) => {
                let mut updated = false;

                if let Some(relay_url) = relay {
                    if let Some(pos) = config.relays.iter().position(|r| r == &relay_url) {
                        config.relays.remove(pos);
                        println!("Removed relay: {}", relay_url);
                        updated = true;
                    } else {
                        println!("Relay not found: {}", relay_url);
                    }
                }

                if let Some(mint_url) = mint {
                    if let Some(pos) = config.mints.iter().position(|m| m.url == mint_url) {
                        config.mints.remove(pos);
                        println!("Removed mint: {}", mint_url);
                        updated = true;
                    } else {
                        println!("Mint not found: {}", mint_url);
                    }
                }

                if clear_key {
                    if config.default_private_key.is_some() {
                        config.default_private_key = None;
                        println!("Cleared default private key");
                        updated = true;
                    } else {
                        println!("No default private key set");
                    }
                }

                if updated {
                    if let Err(e) = config.save() {
                        println!("Failed to save config: {}", e);
                    }
                } else {
                    println!("No changes made");
                }
            }
            Err(e) => {
                println!("Failed to load local config: {}", e);
            }
        },

        Commands::GetEventHistoryByMint { mint } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let transactions = WalletOperations::get_formatted_history(&wallet, mint).await?;

                println!("=== Transaction History ===");

                // Group by mint for summary
                let mut mint_summaries: std::collections::HashMap<String, (u64, u64, String)> =
                    std::collections::HashMap::new();
                for tx in &transactions {
                    let entry = mint_summaries.entry(tx.mint_url.clone()).or_insert((
                        0,
                        0,
                        tx.unit.clone(),
                    ));
                    match tx.direction.as_str() {
                        "in" => entry.0 += tx.amount,
                        "out" => entry.1 += tx.amount,
                        _ => {}
                    }
                }

                println!("\n=== Summary by Mint ===");
                for (mint_url, (total_in, total_out, unit)) in &mint_summaries {
                    println!("• {}", mint_url);
                    println!(
                        "  Total In: {}",
                        WalletOperations::display_amount_with_conversion(*total_in, unit, true)
                    );
                    println!(
                        "  Total Out: {}",
                        WalletOperations::display_amount_with_conversion(*total_out, unit, true)
                    );
                    println!(
                        "  Net: {}",
                        WalletOperations::display_amount_with_conversion(
                            if *total_in > *total_out {
                                *total_in - *total_out
                            } else {
                                *total_out - *total_in
                            },
                            unit,
                            true
                        )
                    );
                    println!();
                }

                println!("=== Detailed Transactions ===");
                for (i, tx) in transactions.iter().enumerate() {
                    println!(
                        "{}. {} - {}",
                        i + 1,
                        tx.direction.to_uppercase(),
                        tx.mint_url
                    );
                    println!(
                        "   Amount: {}",
                        WalletOperations::display_amount_with_conversion(tx.amount, &tx.unit, true)
                    );
                    if let Some(timestamp) = tx.timestamp {
                        println!("   Time: {}", timestamp);
                    }
                    if let Some(memo) = &tx.memo {
                        println!("   Memo: {}", memo);
                    }
                    println!();
                }

                if transactions.is_empty() {
                    println!("No transactions found");
                }
            } else {
                println!("No wallet found");
            }
        }

        Commands::GetProofBreakdown => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let breakdown_str = wallet.get_proof_breakdown_string().await?;
                println!("{}", breakdown_str);
            } else {
                println!("No wallet found");
            }
        }

        Commands::ShowMintInfo { mint } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let mint_infos = WalletOperations::get_mint_info_display(&wallet);

                if let Some(mint_url) = mint {
                    if let Some(mint_info) = mint_infos.iter().find(|m| m.url == mint_url) {
                        println!("=== Mint Information ===");
                        println!("URL: {}", mint_info.url);
                        println!("Name: {}", mint_info.name.as_deref().unwrap_or("Unknown"));
                        println!(
                            "Description: {}",
                            mint_info.description.as_deref().unwrap_or("No description")
                        );
                        println!("Primary Unit: {}", mint_info.unit);
                        println!("Active: {}", mint_info.active);
                        println!("\n=== Keysets ===");
                        for keyset in &mint_info.keysets {
                            println!("• ID: {}", keyset.id);
                            println!("  Unit: {}", keyset.unit);
                            println!("  Active: {}", keyset.active);
                            println!();
                        }
                    } else {
                        println!("Mint not found: {}", mint_url);
                    }
                } else {
                    println!("=== All Mint Information ===");
                    for mint_info in &mint_infos {
                        println!("\n• {}", mint_info.url);
                        println!("  Name: {}", mint_info.name.as_deref().unwrap_or("Unknown"));
                        println!(
                            "  Description: {}",
                            mint_info.description.as_deref().unwrap_or("No description")
                        );
                        println!("  Primary Unit: {}", mint_info.unit);
                        println!("  Active: {}", mint_info.active);
                        println!("  Keysets: {}", mint_info.keysets.len());
                        for keyset in &mint_info.keysets {
                            println!(
                                "    - {} ({}) - Active: {}",
                                keyset.id, keyset.unit, keyset.active
                            );
                        }
                    }
                }
            } else {
                println!("No wallet found");
            }
        }
    }

    Ok(())
}
