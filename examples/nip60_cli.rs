use clap::{Parser, Subcommand};
use ecash_402_wallet::nip60::Nip60Wallet;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalConfig {
    pub relays: Vec<String>,
    pub mints: Vec<String>,
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
            self.mints.clone()
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
            // Load local config and merge with command line args
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
                    println!("❌ Failed to create wallet: {}", e);
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
                let balance = wallet.get_balance().await?;
                println!("Balance: {} sats", balance);
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
                let proofs = wallet.get_unspent_proofs().await?;
                println!("Unspent proofs: {} proofs", proofs.len());
                for (i, proof) in proofs.iter().enumerate() {
                    println!("  Proof {}: {} sats", i + 1, proof.amount);
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
                println!("Spending history: {} entries", history.len());
                for (i, entry) in history.iter().enumerate() {
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
                let stats = wallet.get_stats().await?;
                println!("Wallet stats:");
                println!("  Balance: {} sats", stats.balance);
                println!("  Token events: {}", stats.token_events);
                println!("  Mints: {:?}", stats.mints);
            } else {
                println!("No wallet found");
            }
        }

        Commands::ParseCashuToken { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let token_info = wallet.parse_cashu_token(&token)?;
                println!("Token info:");
                println!("  Amount: {} sats", token_info.proofs().len());
                println!("  Mint: {}", token_info.mint_url()?.to_string());
                println!(
                    "  Memo: {}",
                    token_info
                        .memo()
                        .as_ref()
                        .map(String::as_str)
                        .unwrap_or("None")
                );
            } else {
                println!("No wallet found");
            }
        }

        Commands::CalculateTokenAmount { token } => {
            let local_config = LocalConfig::load().unwrap_or_default();
            let keys = Keys::from_str(&local_config.default_private_key.unwrap())?;
            let relay_refs: Vec<&str> = local_config.relays.iter().map(|s| s.as_str()).collect();

            if let Some(wallet) = Nip60Wallet::load_from_nostr(keys, relay_refs.clone()).await? {
                let parsed_token = wallet.parse_cashu_token(&token)?;
                let amount = wallet.calculate_token_amount(&parsed_token)?;
                println!("Token amount: {} sats", amount);
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
                let token = wallet.send(amount, memo.map(String::from)).await?;
                println!("Token created successfully: {}", token);
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
                wallet.redeem(&token).await?;
                println!("Token redeemed successfully");
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
                if let Ok(config_file) = LocalConfig::config_file() {
                    println!("Local configuration ({})", config_file.display());
                }
                println!("  Relays: {:?}", config.relays);
                println!("  Mints: {:?}", config.mints);
                if config.default_private_key.is_some() {
                    println!("  Default private key: [CONFIGURED]");
                } else {
                    println!("  Default private key: [NOT SET]");
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
            let config = LocalConfig {
                relays,
                mints,
                default_private_key,
            };

            match config.save() {
                Ok(_) => {
                    println!("Local configuration updated:");
                    println!("  Relays: {:?}", config.relays);
                    println!("  Mints: {:?}", config.mints);
                    if config.default_private_key.is_some() {
                        println!("  Default private key: [SET]");
                    } else {
                        println!("  Default private key: [NOT SET]");
                    }
                }
                Err(e) => {
                    println!("Failed to save local config: {}", e);
                }
            }
        }

        Commands::AddToLocalConfig { relay, mint } => match LocalConfig::load() {
            Ok(mut config) => {
                let mut updated = false;

                if let Some(relay_url) = relay {
                    if !config.relays.contains(&relay_url) {
                        config.relays.push(relay_url.clone());
                        println!("Added relay: {}", relay_url);
                        updated = true;
                    } else {
                        println!("Relay already exists: {}", relay_url);
                    }
                }

                if let Some(mint_url) = mint {
                    if !config.mints.contains(&mint_url) {
                        config.mints.push(mint_url.clone());
                        println!("Added mint: {}", mint_url);
                        updated = true;
                    } else {
                        println!("Mint already exists: {}", mint_url);
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
                    if let Some(pos) = config.mints.iter().position(|m| m == &mint_url) {
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
    }

    Ok(())
}
