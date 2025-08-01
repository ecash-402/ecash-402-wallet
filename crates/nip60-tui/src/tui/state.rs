use crate::error::Result;
use crate::tui::config::{Config, WalletConfig};
use nip60::nip60::{Nip60Wallet, ProofBreakdown, SpendingHistory, WalletState};
use nostr_sdk::prelude::*;
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Main,
    History,
    Send,
    Redeem,
    Lightning,
    WalletManager,
    AddWallet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFilter {
    All,
    Received,
    Sent,
}

#[derive(Debug)]
pub struct WalletInstance {
    pub config: WalletConfig,
    pub wallet: Option<Nip60Wallet>,
    pub state: Option<WalletState>,
    pub balance: u64,
    pub history: Vec<SpendingHistory>,
    pub mint_breakdowns: Vec<ProofBreakdown>,
    pub last_update: SystemTime,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SendState {
    pub amount_input: String,
    pub memo_input: String,
    pub generated_token: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RedeemState {
    pub token_input: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LightningState {
    pub amount_input: String,
    pub description_input: String,
    pub invoice: Option<String>,
    pub qr_code: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AddWalletState {
    pub name_input: String,
    pub nsec_input: String,
    pub mints_input: String,
    pub relays_input: String,
    pub error: Option<String>,
    pub step: u8,
}

pub struct AppState {
    pub config: Config,
    pub wallets: HashMap<String, WalletInstance>,
    pub active_view: ActiveView,
    pub history_filter: HistoryFilter,
    pub send_state: SendState,
    pub redeem_state: RedeemState,
    pub lightning_state: LightningState,
    pub add_wallet_state: AddWalletState,
    pub selected_wallet_index: usize,
    pub selected_mint_index: usize,
    pub selected_history_index: usize,
    pub loading: bool,
    pub error_message: Option<String>,
    pub last_refresh: SystemTime,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let mut wallets = HashMap::new();

        for wallet_config in &config.wallets {
            wallets.insert(
                wallet_config.name.clone(),
                WalletInstance {
                    config: wallet_config.clone(),
                    wallet: None,
                    state: None,
                    balance: 0,
                    history: Vec::new(),
                    mint_breakdowns: Vec::new(),
                    last_update: SystemTime::now(),
                    error: None,
                },
            );
        }

        Ok(Self {
            config,
            wallets,
            active_view: ActiveView::Main,
            history_filter: HistoryFilter::All,
            send_state: SendState::default(),
            redeem_state: RedeemState::default(),
            lightning_state: LightningState::default(),
            add_wallet_state: AddWalletState::default(),
            selected_wallet_index: 0,
            selected_mint_index: 0,
            selected_history_index: 0,
            loading: false,
            error_message: None,
            last_refresh: SystemTime::now(),
        })
    }

    pub fn get_active_wallet(&self) -> Option<&WalletInstance> {
        self.config
            .get_active_wallet()
            .and_then(|config| self.wallets.get(&config.name))
    }

    pub fn get_active_wallet_mut(&mut self) -> Option<&mut WalletInstance> {
        if let Some(config) = self.config.get_active_wallet() {
            let name = config.name.clone();
            self.wallets.get_mut(&name)
        } else {
            None
        }
    }

    pub async fn load_wallet(&mut self, name: &str) -> Result<()> {
        if let Some(wallet_instance) = self.wallets.get_mut(name) {
            if wallet_instance.wallet.is_none() {
                let keys = Keys::parse(&wallet_instance.config.nsec)?;
                let relays: Vec<&str> = wallet_instance
                    .config
                    .relays
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let mints = wallet_instance.config.mints.clone();

                match Nip60Wallet::from_config(keys, relays, mints).await {
                    Ok(wallet) => {
                        wallet_instance.wallet = Some(wallet);
                        self.refresh_wallet_data(name).await?;
                    }
                    Err(e) => {
                        wallet_instance.error = Some(format!("Failed to load wallet: {}", e));
                    }
                }
            } else {
                // Wallet already loaded, just refresh the data
                self.refresh_wallet_data(name).await?;
            }
        }
        Ok(())
    }

    pub async fn refresh_wallet_data(&mut self, name: &str) -> Result<()> {
        if let Some(wallet_instance) = self.wallets.get_mut(name) {
            if let Some(ref wallet) = wallet_instance.wallet {
                wallet_instance.last_update = SystemTime::now();

                match wallet.get_balance().await {
                    Ok(balance) => {
                        wallet_instance.balance = balance;
                        wallet_instance.error = None;
                    }
                    Err(e) => {
                        wallet_instance.error = Some(format!("Failed to get balance: {}", e));
                    }
                }

                match wallet.get_spending_history().await {
                    Ok(history) => {
                        wallet_instance.history = history;
                    }
                    Err(e) => {
                        wallet_instance.error = Some(format!("Failed to get history: {}", e));
                    }
                }

                match wallet.get_wallet_state().await {
                    Ok(state) => {
                        wallet_instance.state = Some(state);
                    }
                    Err(e) => {
                        wallet_instance.error = Some(format!("Failed to get state: {}", e));
                    }
                }

                match wallet.get_unspent_proofs().await {
                    Ok(proofs) => {
                        wallet_instance.mint_breakdowns = wallet.get_proof_breakdown(&proofs);
                    }
                    Err(e) => {
                        wallet_instance.error = Some(format!("Failed to get proofs: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn refresh_all_wallets(&mut self) -> Result<()> {
        let wallet_names: Vec<String> = self.wallets.keys().cloned().collect();
        for name in wallet_names {
            self.refresh_wallet_data(&name).await?;
        }
        self.last_refresh = SystemTime::now();
        Ok(())
    }

    pub fn get_filtered_history(&self) -> Vec<&SpendingHistory> {
        if let Some(wallet) = self.get_active_wallet() {
            match self.history_filter {
                HistoryFilter::All => wallet.history.iter().collect(),
                HistoryFilter::Received => wallet
                    .history
                    .iter()
                    .filter(|h| h.direction == "in")
                    .collect(),
                HistoryFilter::Sent => wallet
                    .history
                    .iter()
                    .filter(|h| h.direction == "out")
                    .collect(),
            }
        } else {
            Vec::new()
        }
    }

    pub fn switch_view(&mut self, view: ActiveView) {
        self.active_view = view;
        self.error_message = None;
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn get_selected_mint_url(&self) -> Option<String> {
        if let Some(wallet) = self.get_active_wallet() {
            if self.selected_mint_index < wallet.config.mints.len() {
                wallet.config.mints.get(self.selected_mint_index).cloned()
            } else {
                // If index is out of bounds, return first mint if available
                wallet.config.mints.first().cloned()
            }
        } else {
            None
        }
    }

    pub async fn get_selected_mint_balance_info(&self) -> Option<(u64, String)> {
        if let Some(wallet) = self.get_active_wallet() {
            if let Some(ref nip60_wallet) = wallet.wallet {
                if let Ok(proofs) = nip60_wallet.get_unspent_proofs().await {
                    let breakdowns = nip60_wallet.get_proof_breakdown(&proofs);
                    if let Some(selected_mint_url) = self.get_selected_mint_url() {
                        for breakdown in breakdowns {
                            if breakdown.mint_url == selected_mint_url {
                                let unit = if let Some(mint_info) =
                                    nip60_wallet.get_mint_info(&selected_mint_url)
                                {
                                    mint_info
                                        .keysets
                                        .first()
                                        .map(|ks| ks.unit.clone())
                                        .unwrap_or_else(|| "sats".to_string())
                                } else {
                                    "sats".to_string()
                                };
                                return Some((breakdown.total_balance, unit));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_display_balance_info(&self) -> (u64, String) {
        if let Some(wallet) = self.get_active_wallet() {
            if let Some(selected_mint_url) = self.get_selected_mint_url() {
                // Find the selected mint in the cached breakdowns
                for breakdown in &wallet.mint_breakdowns {
                    if breakdown.mint_url == selected_mint_url {
                        let unit = if let Some(ref nip60_wallet) = wallet.wallet {
                            if let Some(mint_info) = nip60_wallet.get_mint_info(&selected_mint_url)
                            {
                                mint_info
                                    .keysets
                                    .first()
                                    .map(|ks| ks.unit.clone())
                                    .unwrap_or_else(|| "sats".to_string())
                            } else {
                                "sats".to_string()
                            }
                        } else {
                            "sats".to_string()
                        };
                        return (breakdown.total_balance, unit);
                    }
                }
                // If no breakdown found for selected mint, return 0 balance with estimated unit
                let unit = if let Some(ref nip60_wallet) = wallet.wallet {
                    if let Some(mint_info) = nip60_wallet.get_mint_info(&selected_mint_url) {
                        mint_info
                            .keysets
                            .first()
                            .map(|ks| ks.unit.clone())
                            .unwrap_or_else(|| "sats".to_string())
                    } else {
                        "sats".to_string()
                    }
                } else {
                    "sats".to_string()
                };
                return (0, unit);
            } else {
                // No mint selected - show total balance across all mints
                let total_balance: u64 =
                    wallet.mint_breakdowns.iter().map(|b| b.total_balance).sum();
                return (total_balance.max(wallet.balance), "sats".to_string());
            }
        }
        (0, "sats".to_string())
    }

    pub async fn refresh_mint_breakdowns(&mut self) -> Result<()> {
        if let Some(active_wallet_config) = self.config.get_active_wallet() {
            let wallet_name = active_wallet_config.name.clone();
            if let Some(wallet_instance) = self.wallets.get_mut(&wallet_name) {
                if let Some(ref wallet) = wallet_instance.wallet {
                    match wallet.get_unspent_proofs().await {
                        Ok(proofs) => {
                            wallet_instance.mint_breakdowns = wallet.get_proof_breakdown(&proofs);
                        }
                        Err(e) => {
                            wallet_instance.error = Some(format!("Failed to get proofs: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn ensure_mint_index_valid(&mut self) {
        if let Some(wallet) = self.get_active_wallet() {
            if wallet.config.mints.is_empty() {
                self.selected_mint_index = 0;
            } else if self.selected_mint_index >= wallet.config.mints.len() {
                self.selected_mint_index = wallet.config.mints.len() - 1;
            }
        } else {
            self.selected_mint_index = 0;
        }
    }
}
