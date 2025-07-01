use crate::error::Result;
use base64::Engine;
use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;

use cdk::nuts::nut00::{Proof, Token};

pub mod kinds {
    use nostr_sdk::Kind;

    pub const WALLET: Kind = Kind::Custom(17375);
    pub const TOKEN: Kind = Kind::Custom(7375);
    pub const SPENDING_HISTORY: Kind = Kind::Custom(7376);
    pub const QUOTE: Kind = Kind::Custom(7374);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStats {
    pub balance: u64,
    pub token_events: usize,
    pub mints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub mints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub mint: String,
    pub proofs: Vec<CashuProof>,
    #[serde(default)]
    pub del: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashuProof {
    pub id: String,
    pub amount: u64,
    pub secret: String,
    #[serde(rename = "C")]
    pub c: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingHistory {
    pub direction: String,
    pub amount: String,
    pub events: Vec<(String, String, String, String)>, // [e, event_id, relay, marker]
}

#[derive(Debug, Clone)]
pub struct TokenEvent {
    pub id: EventId,
    pub data: TokenData,
}

pub struct Nip60Wallet {
    client: Client,
    mints: Vec<String>,
}

impl Nip60Wallet {
    pub async fn from_config(
        nostr_keys: Keys,
        relays: Vec<&str>,
        mints: Vec<String>,
    ) -> Result<Self> {
        let client = Client::new(nostr_keys);

        for relay in relays {
            client
                .add_relay(relay)
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Failed to add relay: {}", e)))?;
        }

        client.connect().await;

        Ok(Self { client, mints })
    }

    pub async fn new(nostr_keys: Keys, relays: Vec<&str>, mints: Vec<String>) -> Result<Self> {
        let client = Client::new(nostr_keys);

        for relay in relays {
            client
                .add_relay(relay)
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Failed to add relay: {}", e)))?;
        }

        client.connect().await;

        let wallet = Self { client, mints };

        wallet.publish_wallet_config().await?;

        Ok(wallet)
    }

    pub async fn load_from_nostr(nostr_keys: Keys, relays: Vec<&str>) -> Result<Option<Self>> {
        let client = Client::new(nostr_keys.clone());

        for relay in relays {
            client
                .add_relay(relay)
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Failed to add relay: {}", e)))?;
        }

        client.connect().await;

        let filter = Filter::new()
            .author(nostr_keys.public_key())
            .kind(kinds::WALLET)
            .limit(1);

        let events = client
            .fetch_events(filter, Duration::from_secs(10))
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Failed to fetch events: {}", e)))?;

        if let Some(wallet_event) = events.first() {
            let decrypted = client
                .signer()
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?
                .nip44_decrypt(&nostr_keys.public_key(), &wallet_event.content)
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Decryption failed: {}", e)))?;

            let config: WalletConfig = serde_json::from_str(&decrypted).map_err(|e| {
                crate::error::Error::custom(&format!("Invalid wallet config: {}", e))
            })?;

            return Ok(Some(Self {
                client,
                mints: config.mints,
            }));
        }

        Ok(None)
    }

    /// Publish wallet configuration to Nostr (kind 17375)
    async fn publish_wallet_config(&self) -> Result<()> {
        let config = WalletConfig {
            mints: self.mints.clone(),
        };

        let content_json = serde_json::to_string(&config)
            .map_err(|e| crate::error::Error::custom(&format!("Serialization failed: {}", e)))?;

        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let encrypted_content = signer
            .nip44_encrypt(&public_key, &content_json)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Encryption failed: {}", e)))?;

        let mut tags = Vec::new();
        for mint in &self.mints {
            tags.push(Tag::custom(TagKind::Custom("mint".into()), [mint]));
        }

        let event_builder = EventBuilder::new(kinds::WALLET, encrypted_content).tags(tags);

        self.client
            .send_event_builder(event_builder)
            .await
            .map_err(|e| {
                crate::error::Error::custom(&format!("Failed to publish wallet config: {}", e))
            })?;

        Ok(())
    }

    pub async fn record_spend(
        &self,
        amount: u64,
        spent_token_ids: Vec<EventId>,
        unspent_proofs: Vec<CashuProof>,
    ) -> Result<()> {
        let mut new_token_event_id = None;
        if !unspent_proofs.is_empty() {
            new_token_event_id = Some(
                self.create_rollover_token_event(&unspent_proofs, &spent_token_ids)
                    .await?,
            );
        }

        for token_id in &spent_token_ids {
            self.delete_token_event(token_id).await?;
        }

        let mut event_refs = Vec::new();
        for token_id in &spent_token_ids {
            event_refs.push((
                "e".to_string(),
                token_id.to_hex(),
                "".to_string(),
                "destroyed".to_string(),
            ));
        }
        if let Some(new_id) = new_token_event_id {
            event_refs.push((
                "e".to_string(),
                new_id.to_hex(),
                "".to_string(),
                "created".to_string(),
            ));
        }

        self.create_spending_history("out", amount, event_refs)
            .await?;

        Ok(())
    }

    pub async fn record_receive(&self, token_string: &str) -> Result<u64> {
        let parsed_token = self.parse_cashu_token(token_string)?;
        let total_amount = self.calculate_token_amount(&parsed_token)?;
        let mut created_event_ids = Vec::new();
        let mint_url = parsed_token
            .mint_url()
            .map_err(|e| crate::error::Error::custom(&format!("Failed to get mint URL: {}", e)))?
            .to_string();
        let proofs = self.convert_proofs_to_cashu(&parsed_token.proofs())?;
        let token_event_id = self.create_token_event(&mint_url, &proofs, vec![]).await?;
        created_event_ids.push(token_event_id);

        let event_refs: Vec<_> = created_event_ids
            .iter()
            .map(|id| {
                (
                    "e".to_string(),
                    id.to_hex(),
                    "".to_string(),
                    "created".to_string(),
                )
            })
            .collect();

        self.create_spending_history("in", total_amount, event_refs)
            .await?;

        Ok(total_amount)
    }

    pub fn parse_cashu_token(&self, token_string: &str) -> Result<Token> {
        Token::from_str(token_string).map_err(|e| {
            crate::error::Error::custom(&format!("Failed to parse cashu token: {}", e))
        })
    }

    fn calculate_token_amount(&self, token: &Token) -> Result<u64> {
        let amount: u64 = token
            .proofs()
            .iter()
            .map(|proof| proof.amount.to_string().parse::<u64>().unwrap())
            .sum();
        Ok(amount)
    }

    fn convert_proofs_to_cashu(&self, proofs: &[Proof]) -> Result<Vec<CashuProof>> {
        let mut cashu_proofs = Vec::new();

        for proof in proofs {
            cashu_proofs.push(CashuProof {
                id: proof.keyset_id.to_string(),
                amount: proof.amount.to_string().parse::<u64>().unwrap(),
                secret: proof.secret.to_string(),
                c: proof.c.to_string(),
            });
        }

        Ok(cashu_proofs)
    }

    pub async fn calculate_balance(&self) -> Result<u64> {
        let token_events = self.fetch_token_events().await?;
        let balance = token_events
            .iter()
            .flat_map(|event| &event.data.proofs)
            .map(|proof| proof.amount)
            .sum();
        Ok(balance)
    }

    pub async fn fetch_token_events(&self) -> Result<Vec<TokenEvent>> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let filter = Filter::new().author(public_key).kind(kinds::TOKEN);

        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await
            .map_err(|e| {
                crate::error::Error::custom(&format!("Failed to fetch token events: {}", e))
            })?;

        let mut token_events = Vec::new();

        for event in events {
            let decrypted = signer
                .nip44_decrypt(&public_key, &event.content)
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Decryption failed: {}", e)))?;

            let token_data: TokenData = serde_json::from_str(&decrypted)
                .map_err(|e| crate::error::Error::custom(&format!("Invalid token data: {}", e)))?;

            token_events.push(TokenEvent {
                id: event.id,
                data: token_data,
            });
        }

        Ok(token_events)
    }

    /// Create a new token event (kind 7375)
    async fn create_token_event(
        &self,
        mint: &str,
        proofs: &[CashuProof],
        del: Vec<String>,
    ) -> Result<EventId> {
        let token_data = TokenData {
            mint: mint.to_string(),
            proofs: proofs.to_vec(),
            del,
        };

        let content_json = serde_json::to_string(&token_data)
            .map_err(|e| crate::error::Error::custom(&format!("Serialization failed: {}", e)))?;

        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let encrypted_content = signer
            .nip44_encrypt(&public_key, &content_json)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Encryption failed: {}", e)))?;

        let event_builder = EventBuilder::new(kinds::TOKEN, encrypted_content);

        let output = self
            .client
            .send_event_builder(event_builder)
            .await
            .map_err(|e| {
                crate::error::Error::custom(&format!("Failed to publish token event: {}", e))
            })?;

        Ok(output.val)
    }

    async fn create_rollover_token_event(
        &self,
        unspent_proofs: &[CashuProof],
        deleted_token_ids: &[EventId],
    ) -> Result<EventId> {
        let del: Vec<String> = deleted_token_ids.iter().map(|id| id.to_hex()).collect();

        if let Some(mint) = &self.mints.first() {
            self.create_token_event(mint, unspent_proofs, del).await
        } else {
            Err(crate::error::Error::custom("No mint configured"))
        }
    }

    async fn delete_token_event(&self, token_id: &EventId) -> Result<()> {
        // Create a NIP-09 delete event (kind 5)
        let delete_builder = EventBuilder::new(Kind::EventDeletion, "").tags([
            Tag::event(*token_id),
            Tag::custom(TagKind::Custom("k".into()), [kinds::TOKEN.to_string()]),
        ]);

        self.client
            .send_event_builder(delete_builder)
            .await
            .map_err(|e| {
                crate::error::Error::custom(&format!("Failed to delete token event: {}", e))
            })?;

        Ok(())
    }

    /// Create spending history event (kind 7376)
    async fn create_spending_history(
        &self,
        direction: &str,
        amount: u64,
        event_refs: Vec<(String, String, String, String)>,
    ) -> Result<()> {
        let history = SpendingHistory {
            direction: direction.to_string(),
            amount: amount.to_string(),
            events: event_refs.clone(),
        };

        let content_json = serde_json::to_string(&history)
            .map_err(|e| crate::error::Error::custom(&format!("Serialization failed: {}", e)))?;

        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let encrypted_content = signer
            .nip44_encrypt(&public_key, &content_json)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Encryption failed: {}", e)))?;

        // Create tags for event references (unencrypted for 'redeemed' marker)
        let mut tags = Vec::new();
        for (tag_name, event_id, relay, marker) in &event_refs {
            if marker == "redeemed" {
                tags.push(Tag::custom(
                    TagKind::Custom(tag_name.clone().into()),
                    [event_id, relay, marker],
                ));
            }
        }

        let event_builder =
            EventBuilder::new(kinds::SPENDING_HISTORY, encrypted_content).tags(tags);

        self.client
            .send_event_builder(event_builder)
            .await
            .map_err(|e| {
                crate::error::Error::custom(&format!("Failed to publish spending history: {}", e))
            })?;

        Ok(())
    }

    pub async fn get_spending_history(&self) -> Result<Vec<SpendingHistory>> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let filter = Filter::new()
            .author(public_key)
            .kind(kinds::SPENDING_HISTORY);

        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await
            .map_err(|e| {
                crate::error::Error::custom(&format!("Failed to fetch spending history: {}", e))
            })?;

        let mut history = Vec::new();

        for event in events {
            let decrypted = signer
                .nip44_decrypt(&public_key, &event.content)
                .await
                .map_err(|e| crate::error::Error::custom(&format!("Decryption failed: {}", e)))?;

            let spending_history = match serde_json::from_str::<SpendingHistory>(&decrypted) {
                Ok(history) => history,
                Err(_) => match serde_json::from_str::<Vec<Vec<String>>>(&decrypted) {
                    Ok(legacy_data) => self.parse_legacy_spending_history(legacy_data)?,
                    Err(e) => {
                        return Err(crate::error::Error::custom(&format!(
                            "Invalid spending history format: {} - Content: {}",
                            e, decrypted
                        )));
                    }
                },
            };

            history.push(spending_history);
        }

        Ok(history)
    }

    fn parse_legacy_spending_history(
        &self,
        legacy_data: Vec<Vec<String>>,
    ) -> Result<SpendingHistory> {
        let mut direction = String::new();
        let mut amount = String::new();
        let mut events = Vec::new();

        for item in legacy_data {
            if item.len() >= 2 {
                match item[0].as_str() {
                    "direction" => direction = item[1].clone(),
                    "amount" => amount = item[1].clone(),
                    "e" => {
                        if item.len() >= 4 {
                            events.push((
                                item[0].clone(), // "e"
                                item[1].clone(), // event_id
                                item[2].clone(), // relay (usually empty)
                                item[3].clone(), // marker
                            ));
                        }
                    }
                    _ => {} // Ignore unknown fields
                }
            }
        }

        Ok(SpendingHistory {
            direction,
            amount,
            events,
        })
    }

    pub async fn get_stats(&self) -> Result<WalletStats> {
        let balance = self.calculate_balance().await?;
        let token_events = self.fetch_token_events().await?;

        Ok(WalletStats {
            balance,
            token_events: token_events.len(),
            mints: self.mints.clone(),
        })
    }

    /// Send cashu tokens to another user via encrypted DM
    pub async fn send_to_pubkey(
        &self,
        recipient_pubkey: PublicKey,
        amount: u64,
        memo: Option<String>,
    ) -> Result<EventId> {
        let token_events = self.fetch_token_events().await?;
        let (selected_proofs, remaining_proofs, spent_event_ids) =
            self.select_proofs_for_amount(&token_events, amount)?;

        if selected_proofs.is_empty() {
            return Err(crate::error::Error::custom("Insufficient balance"));
        }

        let mint_url = selected_proofs
            .first()
            .map(|p| p.id.clone()) // Using id as mint reference for now
            .unwrap_or_else(|| self.mints.first().cloned().unwrap_or_default());

        let token_string = self.create_cashu_token_string(&mint_url, &selected_proofs, memo)?;

        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let encrypted_content = signer
            .nip44_encrypt(&recipient_pubkey, &token_string)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Encryption failed: {}", e)))?;

        // Create DM event (kind 4)
        let dm_builder = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted_content)
            .tag(Tag::public_key(recipient_pubkey));

        let dm_output = self
            .client
            .send_event_builder(dm_builder)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Failed to send DM: {}", e)))?;

        self.record_spend(amount, spent_event_ids, remaining_proofs)
            .await?;

        Ok(dm_output.val)
    }

    pub async fn send_to_self(&self, amount: u64, memo: Option<String>) -> Result<EventId> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        self.send_to_pubkey(public_key, amount, memo).await
    }

    pub async fn send_token_string_to_self(&self, token_string: &str) -> Result<EventId> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let encrypted_content = signer
            .nip44_encrypt(&public_key, token_string)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Encryption failed: {}", e)))?;

        // Create DM event (kind 4)
        let dm_builder = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted_content)
            .tag(Tag::public_key(public_key));

        let dm_output = self
            .client
            .send_event_builder(dm_builder)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Failed to send DM: {}", e)))?;

        Ok(dm_output.val)
    }

    fn create_cashu_token_string(
        &self,
        mint_url: &str,
        proofs: &[CashuProof],
        memo: Option<String>,
    ) -> Result<String> {
        let token_data = serde_json::json!({
            "mint": mint_url,
            "proofs": proofs,
            "memo": memo
        });

        let token_json = serde_json::to_string(&token_data).map_err(|e| {
            crate::error::Error::custom(&format!("Token serialization failed: {}", e))
        })?;

        // Encode as base64 for transport
        Ok(format!(
            "cashuA{}",
            base64::engine::general_purpose::STANDARD.encode(token_json)
        ))
    }

    fn select_proofs_for_amount(
        &self,
        token_events: &[TokenEvent],
        amount: u64,
    ) -> Result<(Vec<CashuProof>, Vec<CashuProof>, Vec<EventId>)> {
        let mut selected_proofs = Vec::new();
        let mut remaining_proofs = Vec::new();
        let mut spent_event_ids = Vec::new();
        let mut current_amount = 0u64;

        for event in token_events {
            if current_amount >= amount {
                remaining_proofs.extend(event.data.proofs.clone());
            } else {
                spent_event_ids.push(event.id);
                for proof in &event.data.proofs {
                    if current_amount < amount {
                        selected_proofs.push(proof.clone());
                        current_amount += proof.amount;
                    } else {
                        remaining_proofs.push(proof.clone());
                    }
                }
            }
        }

        if current_amount < amount {
            return Err(crate::error::Error::custom(&format!(
                "Insufficient balance: need {}, have {}",
                amount, current_amount
            )));
        }

        Ok((selected_proofs, remaining_proofs, spent_event_ids))
    }

    pub async fn check_incoming_tokens(&self) -> Result<Vec<(EventId, String, u64)>> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Public key error: {}", e)))?;

        let filter = Filter::new()
            .kind(Kind::EncryptedDirectMessage)
            .pubkey(public_key)
            .limit(50);

        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Failed to fetch DMs: {}", e)))?;

        let mut incoming_tokens = Vec::new();

        for event in events {
            if let Ok(decrypted) = signer.nip44_decrypt(&event.pubkey, &event.content).await {
                if decrypted.starts_with("cashu") {
                    if let Ok(parsed_token) = self.parse_cashu_token(&decrypted) {
                        let amount = self.calculate_token_amount(&parsed_token)?;
                        incoming_tokens.push((event.id, decrypted, amount));
                    }
                }
            }
        }

        Ok(incoming_tokens)
    }

    pub fn get_config(&self) -> WalletConfig {
        WalletConfig {
            mints: self.mints.clone(),
        }
    }

    pub async fn update_config(&mut self, mints: Option<Vec<String>>) -> Result<()> {
        if let Some(new_mints) = mints {
            self.mints = new_mints;
        }
        self.publish_wallet_config().await
    }
}
