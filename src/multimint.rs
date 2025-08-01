use crate::{
    error::{Error, Result},
    models::SendTokenPendingResponse,
    wallet::CashuWalletClient,
};
use std::{collections::HashMap, str::FromStr, sync::Arc};

use bip39::Mnemonic;

use cdk::{
    Amount,
    cdk_database::{self, WalletDatabase},
    mint_url::MintUrl,
    nuts::CurrencyUnit,
    wallet::{
        ReceiveOptions, SendOptions, multi_mint_wallet::MultiMintWallet as CdkMultiMintWallet,
        types::WalletKey,
    },
};
use cdk_sqlite::WalletSqliteDatabase;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MultimintSendOptions {
    pub preferred_mint: Option<String>,
    pub unit: Option<CurrencyUnit>,
    pub split_across_mints: bool,
}

#[derive(Debug, Clone)]
pub struct MultimintWallet {
    inner: CdkMultiMintWallet,
}

impl MultimintWallet {
    pub async fn new(seed: &str, base_db_path: &str) -> Result<Self> {
        let mnemonic = Mnemonic::from_str(seed).map_err(|_| Error::custom("Invalid mnemonic"))?;
        let seed_full = mnemonic.to_seed_normalized("");
        let seed_bytes: [u8; 32] = seed_full[0..32]
            .try_into()
            .map_err(|_| Error::custom("Invalid seed length"))?;

        let db_path = format!("{}.sqlite", base_db_path);

        let localstore: Arc<dyn WalletDatabase<Err = cdk_database::Error> + Send + Sync> = Arc::new(
            WalletSqliteDatabase::new(&db_path)
                .await
                .map_err(|e| Error::custom(&e.to_string()))?,
        );

        let inner = CdkMultiMintWallet::new(localstore, Arc::new(seed_bytes), vec![]);

        Ok(Self { inner })
    }

    pub async fn from_existing_wallet(
        _wallet: &CashuWalletClient,
        mint_url: &str,
        seed: &str,
        base_db_path: &str,
    ) -> Result<Self> {
        let multimint = Self::new(seed, base_db_path).await?;
        multimint
            .add_mint(mint_url, Some(CurrencyUnit::Msat))
            .await?;
        Ok(multimint)
    }

    pub async fn add_mint(&self, mint_url: &str, unit: Option<CurrencyUnit>) -> Result<()> {
        let currency_unit = unit.unwrap_or(CurrencyUnit::Msat);
        let _mint_url_parsed =
            MintUrl::from_str(mint_url).map_err(|e| Error::custom(&e.to_string()))?;

        let _wallet = self
            .inner
            .create_and_add_wallet(mint_url, currency_unit.clone(), None)
            .await
            .map_err(|e| Error::custom(&e.to_string()))?;

        Ok(())
    }

    pub async fn remove_mint(&self, mint_url: &str) -> Result<()> {
        let balance = self.get_mint_balance(mint_url).await?;
        if balance > 0 {
            return Err(Error::custom(
                "Cannot remove mint with non-zero balance. Transfer funds first.",
            ));
        }

        Ok(())
    }

    pub async fn list_mints(&self) -> Vec<String> {
        let wallets = self.inner.get_wallets().await;
        wallets.iter().map(|f| f.mint_url.to_string()).collect()
    }

    pub async fn get_total_balance(&self) -> Result<MultimintBalance> {
        let mut total_balance = 0u64;
        let mut balances_by_mint = HashMap::new();

        for unit in [CurrencyUnit::Msat, CurrencyUnit::Sat] {
            let balances = self
                .inner
                .get_balances(&unit)
                .await
                .map_err(|e| Error::custom(&e.to_string()))?;

            for (mint_url, amount) in balances {
                let balance: u64 = amount.into();
                let normalized_balance = self.normalize_to_sats(balance, &unit);
                total_balance += normalized_balance;

                balances_by_mint.insert(
                    mint_url.to_string(),
                    MintBalance {
                        mint_url: mint_url.to_string(),
                        balance,
                        unit: unit.clone(),
                        proof_count: 0,
                    },
                );
            }
        }

        Ok(MultimintBalance {
            total_balance,
            balances_by_mint,
        })
    }

    pub async fn get_mint_balance(&self, mint_url: &str) -> Result<u64> {
        let mint_url_parsed =
            MintUrl::from_str(mint_url).map_err(|e| Error::custom(&e.to_string()))?;

        for unit in [CurrencyUnit::Msat, CurrencyUnit::Sat] {
            let balances = self
                .inner
                .get_balances(&unit)
                .await
                .map_err(|e| Error::custom(&e.to_string()))?;

            if let Some(amount) = balances.get(&mint_url_parsed) {
                return Ok((*amount).into());
            }
        }

        Ok(0)
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
        let amount_obj = Amount::from(amount);

        let mint_url = options.preferred_mint.unwrap();
        let wallet = self.get_wallet_for_mint(&mint_url).await.unwrap();

        let prepared_send = wallet
            .prepare_send(amount_obj, SendOptions::default())
            .await
            .map_err(|e| Error::custom(&e.to_string()))?;
        let token = wallet
            .send(prepared_send, None)
            .await
            .map_err(|e| Error::custom(&e.to_string()))?;

        Ok(token.to_string())
    }

    async fn send_split_across_mints(
        &self,
        amount: u64,
        options: MultimintSendOptions,
    ) -> Result<String> {
        let unit = options.unit.unwrap_or(CurrencyUnit::Msat);
        let balances = self
            .inner
            .get_balances(&unit)
            .await
            .map_err(|e| Error::custom(&e.to_string()))?;

        let total_balance: u64 = balances
            .values()
            .map(|a| {
                let val: u64 = (*a).into();
                val
            })
            .sum();
        if total_balance < amount {
            return Err(Error::custom("Insufficient total balance"));
        }

        let mut remaining_amount = amount;
        let mut tokens = Vec::new();

        let mut mint_balances: Vec<_> = balances.iter().collect();
        mint_balances.sort_by(|a, b| b.1.cmp(a.1));

        for (mint_url, mint_balance) in mint_balances {
            if remaining_amount == 0 {
                break;
            }

            let send_amount = remaining_amount.min({
                let balance_u64: u64 = (*mint_balance).into();
                balance_u64
            });
            if send_amount > 0 {
                let wallet_key = WalletKey::new(mint_url.clone(), unit.clone());
                if let Some(wallet) = self.inner.get_wallet(&wallet_key).await {
                    let amount_obj = Amount::from(send_amount);
                    let prepared_send = wallet
                        .prepare_send(amount_obj, SendOptions::default())
                        .await
                        .map_err(|e| Error::custom(&e.to_string()))?;
                    let token = wallet
                        .send(prepared_send, None)
                        .await
                        .map_err(|e| Error::custom(&e.to_string()))?;
                    tokens.push(token.to_string());
                    remaining_amount -= send_amount;
                }
            }
        }

        if remaining_amount > 0 {
            return Err(Error::custom("Could not fulfill amount across all mints"));
        }

        Ok(tokens.join("\n"))
    }

    pub async fn receive(&self, token: &str) -> Result<String> {
        println!("{:?}", token);
        let received = self
            .inner
            .receive(token, ReceiveOptions::default())
            .await
            .map_err(|e| Error::custom(&e.to_string()))?;
        Ok(received.to_string())
    }

    pub async fn redeem_pendings(&self) -> Result<()> {
        self.check_and_redeem_pending().await
    }

    pub async fn get_all_pending(&self) -> Result<HashMap<String, Vec<SendTokenPendingResponse>>> {
        let wallets = self.inner.get_wallets().await;

        let mut table: HashMap<String, Vec<SendTokenPendingResponse>> = HashMap::new();
        for wallet in wallets.iter() {
            let mint_url = wallet.mint_url.clone();

            let pending_proofs = wallet.get_pending_proofs().await?;
            if pending_proofs.is_empty() {
                continue;
            }

            table.insert(
                mint_url.to_string(),
                pending_proofs
                    .into_iter()
                    .map(|proof| SendTokenPendingResponse {
                        token: proof.secret.to_string(),
                        amount: proof.amount.to_string(),
                        key: proof.c.to_string(),
                        key_id: proof.keyset_id.to_string(),
                    })
                    .collect(),
            );
        }

        Ok(table)
    }

    pub async fn set_mint_active(&self, _mint_url: &str, _active: bool) -> Result<()> {
        Ok(())
    }

    fn normalize_to_sats(&self, amount: u64, unit: &CurrencyUnit) -> u64 {
        match unit {
            CurrencyUnit::Msat => amount / 1000,
            CurrencyUnit::Sat => amount,
            _ => amount,
        }
    }

    pub async fn get_wallet_for_mint(&self, mint_url: &str) -> Option<cdk::wallet::Wallet> {
        // Try both currency units that are commonly used
        for unit in [CurrencyUnit::Sat, CurrencyUnit::Msat] {
            let mint_url_parsed = match MintUrl::from_str(mint_url) {
                Ok(url) => url,
                Err(_) => continue,
            };

            let wallet_key = WalletKey::new(mint_url_parsed, unit);
            if let Some(wallet) = self.inner.get_wallet(&wallet_key).await {
                return Some(wallet);
            }
        }
        None
    }

    pub fn get_wallet_for_mint_sync(&self, _mint_url: &str) -> Option<&CashuWalletClient> {
        // This function is kept for backwards compatibility but returns None
        // Use get_wallet_for_mint instead for actual functionality
        None
    }

    pub async fn transfer_between_mints(
        &self,
        from_mint: &str,
        _to_mint: &str,
        amount: u64,
    ) -> Result<String> {
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

    pub fn cdk_wallet(&self) -> &CdkMultiMintWallet {
        &self.inner
    }

    async fn check_and_redeem_pending(&self) -> Result<()> {
        let wallets = self.inner.get_wallets().await;

        for wallet in wallets.iter() {
            let pending_proofs = wallet.get_pending_proofs().await?;
            if pending_proofs.is_empty() {
                continue;
            }

            match wallet.reclaim_unspent(pending_proofs).await {
                Ok(()) => println!("Successfully reclaimed pending proofs"),
                Err(e) => println!("Error reclaimed pending proofs: {e}"),
            }
        }
        Ok(())
    }
}
