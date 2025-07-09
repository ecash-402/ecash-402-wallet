use crate::{
    error::{Error, Result},
    models::SendTokenPendingResponse,
    multimint::MultimintWallet,
};
use std::{str::FromStr, sync::Arc};

use bip39::Mnemonic;
use cdk::wallet::{HttpClient, ReceiveOptions, SendOptions, Wallet, WalletBuilder};
use cdk_redb::WalletRedbDatabase;

pub fn prepare_seed(seed: &str) -> Result<[u8; 64]> {
    let mnemonic = Mnemonic::from_str(&seed).map_err(|_| Error::custom("Invalid mnemonic seed"))?;
    Ok(mnemonic.to_seed_normalized(""))
}

#[derive(Debug, Clone)]
pub struct CashuWalletClient {
    pub wallet: Wallet,
}

impl CashuWalletClient {
    pub async fn from_seed(mint_url: &str, seed: &str, db_name: &str) -> Result<Self> {
        let s = Mnemonic::from_str(seed).map_err(|_| Error::custom("Invalid mnemonic seed"))?;
        CashuWalletClient::wallet(mint_url, s, db_name, cdk::nuts::CurrencyUnit::Msat).await
    }

    pub async fn from_seed_with_unit(
        mint_url: &str,
        seed: &str,
        db_name: &str,
        unit: cdk::nuts::CurrencyUnit,
    ) -> Result<Self> {
        let s = Mnemonic::from_str(seed).map_err(|_| Error::custom("Invalid mnemonic seed"))?;
        CashuWalletClient::wallet(mint_url, s, db_name, unit).await
    }

    pub async fn new(mint_url: &str, seed: &mut String, db_name: &str) -> Result<Self> {
        let s = Mnemonic::generate(12).map_err(|_| Error::custom("Failed to generate mnemonic"))?;
        seed.push_str(&s.to_string());
        CashuWalletClient::wallet(mint_url, s, db_name, cdk::nuts::CurrencyUnit::Msat).await
    }

    pub async fn send(&self, amount: u64) -> Result<String> {
        let prepared_send = self
            .wallet
            .prepare_send((amount as u64).into(), SendOptions::default())
            .await?;
        Ok(self.wallet.send(prepared_send, None).await?.to_string())
    }

    pub async fn receive(&self, token: &str) -> Result<String> {
        Ok(self
            .wallet
            .receive(token, ReceiveOptions::default())
            .await?
            .to_string())
    }

    pub async fn balance(&self) -> Result<String> {
        Ok(self.wallet.total_balance().await?.to_string())
    }

    pub async fn pending(&self) -> Result<Vec<SendTokenPendingResponse>> {
        let proofs = self.wallet.get_pending_spent_proofs().await?;

        Ok(proofs
            .into_iter()
            .map(|proof| SendTokenPendingResponse {
                token: proof.secret.to_string(),
                amount: proof.amount.to_string(),
                key: proof.c.to_string(),
                key_id: proof.keyset_id.to_string(),
            })
            .collect())
    }

    async fn wallet(
        mint_url: &str,
        s: Mnemonic,
        db_name: &str,
        unit: cdk::nuts::CurrencyUnit,
    ) -> Result<Self> {
        let home_dir =
            home::home_dir().ok_or_else(|| Error::custom("Could not determine home directory"))?;
        let db_path = home_dir.join(db_name);

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::custom(&format!("Failed to create database directory: {}", e))
            })?;
        }

        let localstore = WalletRedbDatabase::new(&db_path)
            .map_err(|e| Error::custom(&format!("Failed to create database: {}", e)))?;

        let seed = s.to_seed_normalized("");
        let mint_url = cdk::mint_url::MintUrl::from_str(mint_url)
            .map_err(|_| Error::custom("Invalid mint URL"))?;
        let mut builder = WalletBuilder::new()
            .mint_url(mint_url.clone())
            .unit(unit)
            .localstore(Arc::new(localstore))
            .seed(&seed);
        let http_client = HttpClient::new(mint_url);
        builder = builder.client(http_client);

        Ok(Self {
            wallet: builder.build()?,
        })
    }

    pub async fn redeem_pendings(&self) -> Result<()> {
        let proofs = self.wallet.get_pending_spent_proofs().await?;
        self.wallet
            .receive_proofs(proofs, ReceiveOptions::default(), None)
            .await?;
        Ok(())
    }

    pub async fn to_multimint_wallet(
        &self,
        mint_url: &str,
        seed: &str,
        base_db_path: &str,
    ) -> Result<MultimintWallet> {
        MultimintWallet::from_existing_wallet(self, mint_url, seed, base_db_path).await
    }

    pub async fn create_multimint_wallet(
        seed: &str,
        base_db_path: &str,
    ) -> Result<MultimintWallet> {
        MultimintWallet::new(seed, base_db_path).await
    }
}
