use crate::{
    error::{Error, Result},
    models::SendTokenPendingResponse,
    wallet::CashuWalletClient,
};
use std::{collections::HashMap, str::FromStr, sync::Arc};

use bip39::Mnemonic;
use cdk::nuts::CurrencyUnit;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct MintWalletInfo {
    pub url: String,
    pub unit: CurrencyUnit,
    pub wallet: Arc<CashuWalletClient>,
    pub db_name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimintBalance {
    pub total_balance: u64,
    pub balances_by_mint: HashMap<String, MintBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintBalance {
    pub mint_url: String,
    pub balance: u64,
    pub unit: CurrencyUnit,
    pub proof_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimintSendOptions {
    pub preferred_mint: Option<String>,
    pub unit: Option<CurrencyUnit>,
    pub split_across_mints: bool,
}

impl Default for MultimintSendOptions {
    fn default() -> Self {
        Self {
            preferred_mint: None,
            unit: None,
            split_across_mints: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultimintWallet {
    seed: String,
    mint_wallets: HashMap<String, MintWalletInfo>,
    base_db_path: String,
}

impl MultimintWallet {
    pub async fn new(seed: &str, base_db_path: &str) -> Result<Self> {
        let mnemonic = Mnemonic::from_str(seed).map_err(|_| Error::custom("Invalid mnemonic"))?;

        Ok(Self {
            seed: mnemonic.to_string(),
            mint_wallets: HashMap::new(),
            base_db_path: base_db_path.to_string(),
        })
    }

    pub async fn from_existing_wallet(
        wallet: &CashuWalletClient,
        mint_url: &str,
        seed: &str,
        base_db_path: &str,
    ) -> Result<Self> {
        let mut multimint = Self::new(seed, base_db_path).await?;
        multimint.add_mint_from_wallet(mint_url, wallet).await?;
        Ok(multimint)
    }

    async fn add_mint_from_wallet(
        &mut self,
        mint_url: &str,
        wallet: &CashuWalletClient,
    ) -> Result<()> {
        let unit = self.detect_mint_unit(mint_url).await?;
        let db_name = self.generate_db_name(mint_url);

        let mint_info = MintWalletInfo {
            url: mint_url.to_string(),
            unit,
            wallet: Arc::new(wallet.clone()),
            db_name,
            active: true,
        };

        self.mint_wallets.insert(mint_url.to_string(), mint_info);
        Ok(())
    }

    pub async fn add_mint(&mut self, mint_url: &str, _unit: Option<CurrencyUnit>) -> Result<()> {
        if self.mint_wallets.contains_key(mint_url) {
            return Err(Error::custom("Mint already exists"));
        }

        let currency_unit = CurrencyUnit::Msat;

        let db_name = self.generate_db_name(mint_url);
        let wallet = CashuWalletClient::from_seed(mint_url, &self.seed, &db_name).await?;

        let mint_info = MintWalletInfo {
            url: mint_url.to_string(),
            unit: currency_unit,
            wallet: Arc::new(wallet),
            db_name,
            active: true,
        };

        self.mint_wallets.insert(mint_url.to_string(), mint_info);
        Ok(())
    }

    pub async fn remove_mint(&mut self, mint_url: &str) -> Result<()> {
        if !self.mint_wallets.contains_key(mint_url) {
            return Err(Error::custom("Mint not found"));
        }

        let balance = self.get_mint_balance(mint_url).await?;
        if balance > 0 {
            return Err(Error::custom(
                "Cannot remove mint with non-zero balance. Transfer funds first.",
            ));
        }

        self.mint_wallets.remove(mint_url);
        Ok(())
    }

    pub fn list_mints(&self) -> Vec<String> {
        self.mint_wallets.keys().cloned().collect()
    }

    pub fn get_mint_info(&self, mint_url: &str) -> Option<&MintWalletInfo> {
        self.mint_wallets.get(mint_url)
    }

    pub async fn get_total_balance(&self) -> Result<MultimintBalance> {
        let mut total_balance = 0u64;
        let mut balances_by_mint = HashMap::new();

        for (mint_url, mint_info) in &self.mint_wallets {
            if !mint_info.active {
                continue;
            }

            let balance_str = mint_info.wallet.balance().await?;
            let balance = balance_str.parse::<u64>().unwrap_or(0);

            let normalized_balance = self.normalize_to_sats(balance, &mint_info.unit);
            total_balance += normalized_balance;

            let pending = mint_info.wallet.pending().await?;
            let proof_count = pending.len();

            balances_by_mint.insert(
                mint_url.clone(),
                MintBalance {
                    mint_url: mint_url.clone(),
                    balance,
                    unit: mint_info.unit.clone(),
                    proof_count,
                },
            );
        }

        Ok(MultimintBalance {
            total_balance,
            balances_by_mint,
        })
    }

    pub async fn get_mint_balance(&self, mint_url: &str) -> Result<u64> {
        let mint_info = self
            .mint_wallets
            .get(mint_url)
            .ok_or_else(|| Error::custom("Mint not found"))?;

        let balance_str = mint_info.wallet.balance().await?;
        Ok(balance_str.parse::<u64>().unwrap_or(0))
    }

    pub async fn send(&self, amount: u64, options: MultimintSendOptions) -> Result<String> {
        if options.split_across_mints {
            self.send_split_across_mints(amount, options).await
        } else {
            self.send_from_single_mint(amount, options).await
        }
    }

    async fn send_from_single_mint(
        &self,
        amount: u64,
        options: MultimintSendOptions,
    ) -> Result<String> {
        let mint_url = if let Some(preferred) = options.preferred_mint {
            if !self.mint_wallets.contains_key(&preferred) {
                return Err(Error::custom("Preferred mint not found"));
            }
            preferred
        } else {
            self.find_best_mint_for_amount(amount, options.unit).await?
        };

        let mint_info = &self.mint_wallets[&mint_url];
        let balance = self.get_mint_balance(&mint_url).await?;

        if balance < amount {
            return Err(Error::custom(&format!(
                "Insufficient balance in mint {}: need {}, have {}",
                mint_url, amount, balance
            )));
        }

        mint_info.wallet.send(amount).await
    }

    async fn send_split_across_mints(
        &self,
        amount: u64,
        _options: MultimintSendOptions,
    ) -> Result<String> {
        let balance = self.get_total_balance().await?;
        if balance.total_balance < amount {
            return Err(Error::custom("Insufficient total balance"));
        }

        let mut remaining_amount = amount;
        let mut tokens = Vec::new();

        let mut mint_balances: Vec<_> = balance.balances_by_mint.values().collect();
        mint_balances.sort_by(|a, b| b.balance.cmp(&a.balance));

        for mint_balance in mint_balances {
            if remaining_amount == 0 {
                break;
            }

            let mint_info = &self.mint_wallets[&mint_balance.mint_url];
            let send_amount = remaining_amount.min(mint_balance.balance);

            if send_amount > 0 {
                let token = mint_info.wallet.send(send_amount).await?;
                tokens.push(token);
                remaining_amount -= send_amount;
            }
        }

        if remaining_amount > 0 {
            return Err(Error::custom("Could not fulfill amount across all mints"));
        }

        Ok(tokens.join("\n"))
    }

    pub async fn receive(&mut self, token: &str) -> Result<String> {
        let parsed_token = self.parse_token_mint_url(token)?;

        let mint_url = if self.mint_wallets.contains_key(&parsed_token) {
            parsed_token
        } else {
            self.add_mint(&parsed_token, None).await?;
            parsed_token
        };

        let mint_info = &self.mint_wallets[&mint_url];
        mint_info.wallet.receive(token).await
    }

    pub async fn redeem_pendings(&self) -> Result<()> {
        for mint_info in self.mint_wallets.values() {
            if mint_info.active {
                mint_info.wallet.redeem_pendings().await?;
            }
        }
        Ok(())
    }

    pub async fn get_all_pending(&self) -> Result<HashMap<String, Vec<SendTokenPendingResponse>>> {
        let mut all_pending = HashMap::new();

        for (mint_url, mint_info) in &self.mint_wallets {
            if mint_info.active {
                let pending = mint_info.wallet.pending().await?;
                if !pending.is_empty() {
                    all_pending.insert(mint_url.clone(), pending);
                }
            }
        }

        Ok(all_pending)
    }

    pub async fn set_mint_active(&mut self, mint_url: &str, active: bool) -> Result<()> {
        let mint_info = self
            .mint_wallets
            .get_mut(mint_url)
            .ok_or_else(|| Error::custom("Mint not found"))?;

        mint_info.active = active;
        Ok(())
    }

    async fn find_best_mint_for_amount(
        &self,
        amount: u64,
        preferred_unit: Option<CurrencyUnit>,
    ) -> Result<String> {
        let mut candidates = Vec::new();

        for (mint_url, mint_info) in &self.mint_wallets {
            if !mint_info.active {
                continue;
            }

            let balance = self.get_mint_balance(mint_url).await?;
            if balance >= amount {
                let priority =
                    if preferred_unit.is_some() && Some(mint_info.unit.clone()) == preferred_unit {
                        2
                    } else {
                        1
                    };
                candidates.push((mint_url.clone(), balance, priority));
            }
        }

        if candidates.is_empty() {
            return Err(Error::custom("No mint has sufficient balance"));
        }

        candidates.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(&b.1)));

        Ok(candidates[0].0.clone())
    }

    async fn detect_mint_unit(&self, _mint_url: &str) -> Result<CurrencyUnit> {
        Ok(CurrencyUnit::Msat)
    }

    fn parse_token_mint_url(&self, token: &str) -> Result<String> {
        if token.starts_with("cashu") {
            if let Ok(parsed) = cdk::nuts::nut00::Token::from_str(token) {
                if let Ok(mint_url) = parsed.mint_url() {
                    return Ok(mint_url.to_string());
                }
            }
        }
        Err(Error::custom("Could not parse mint URL from token"))
    }

    fn generate_db_name(&self, mint_url: &str) -> String {
        let url_hash = format!("{:x}", md5::compute(mint_url.as_bytes()));
        format!("{}/mint_{}.redb", self.base_db_path, &url_hash[..8])
    }

    fn normalize_to_sats(&self, amount: u64, unit: &CurrencyUnit) -> u64 {
        match unit {
            CurrencyUnit::Msat => amount / 1000,
            CurrencyUnit::Sat => amount,
            _ => amount,
        }
    }

    pub fn get_wallet_for_mint(&self, mint_url: &str) -> Option<&CashuWalletClient> {
        self.mint_wallets
            .get(mint_url)
            .map(|info| info.wallet.as_ref())
    }

    pub async fn transfer_between_mints(
        &mut self,
        from_mint: &str,
        to_mint: &str,
        amount: u64,
    ) -> Result<String> {
        if !self.mint_wallets.contains_key(from_mint) {
            return Err(Error::custom("Source mint not found"));
        }

        if !self.mint_wallets.contains_key(to_mint) {
            return Err(Error::custom("Destination mint not found"));
        }

        let token = self
            .send_from_single_mint(
                amount,
                MultimintSendOptions {
                    preferred_mint: Some(from_mint.to_string()),
                    ..Default::default()
                },
            )
            .await?;

        self.receive(&token).await
    }
}
