use crate::error::Result;
use crate::mint::{KeysetInfo, MintClient};
use crate::wallet::CashuWalletClient;
use bip39::Mnemonic;
use cdk::mint_url::MintUrl;
use cdk::nuts::CurrencyUnit;
use cdk::nuts::Proofs;

use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use crate::error::Error;
use ::hex;
use base64::{Engine as _, engine::general_purpose::STANDARD as base64};
use cdk::nuts::KeySetInfo;
use cdk::nuts::nut00::Token;

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
    pub privkey: Option<String>,
    pub mints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintInfo {
    pub url: String,
    pub keysets: Vec<KeysetInfo>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub mint: String,
    pub proofs: Proofs,
    #[serde(default)]
    pub del: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingHistory {
    pub direction: String,
    pub amount: String,
    pub events: Vec<(String, String, String, String)>,
    #[serde(default)]
    pub created_at: Option<u64>, // Unix timestamp when this spending event was created
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenEvent {
    pub id: EventId,
    pub data: TokenData,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct WalletState {
    pub balance: u64,
    pub proofs: Proofs,
    pub proof_to_event_id: HashMap<String, String>,
    pub mint_keysets: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHistoryByMint {
    pub mint: String,
    pub total_received: u64,
    pub total_spent: u64,
    pub events: Vec<EventDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDetail {
    pub event_id: String,
    pub direction: String,
    pub amount: u64,
    pub timestamp: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProofBreakdown {
    pub mint_url: String,
    pub total_balance: u64,
    pub proof_count: usize,
    pub unit: Option<String>,
    pub denominations: std::collections::HashMap<u64, u32>,
}

impl ProofBreakdown {
    pub fn new(mint_url: String, unit: Option<String>) -> Self {
        Self {
            mint_url,
            unit,
            total_balance: 0,
            proof_count: 0,
            denominations: std::collections::HashMap::new(),
        }
    }

    pub fn add_proof(&mut self, amount: u64) {
        self.total_balance += amount;
        self.proof_count += 1;
        *self.denominations.entry(amount).or_insert(0) += 1;
    }

    pub fn format_denominations(&self) -> String {
        let mut denom_pairs: Vec<_> = self.denominations.iter().collect();
        denom_pairs.sort_by_key(|&(k, _)| k);

        denom_pairs
            .iter()
            .map(|(amount, count)| format!("{}×{}", amount, count))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}: {} {} ({} proofs: {})",
            self.mint_url,
            self.total_balance,
            self.unit.clone().unwrap_or("sats".to_string()),
            self.proof_count,
            self.format_denominations()
        )
    }
}

pub struct Nip60Wallet {
    client: Client,
    mints: Vec<String>,
    mint_infos: HashMap<String, MintInfo>,
}

impl std::fmt::Debug for Nip60Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Nip60Wallet")
            .field("mints", &self.mints)
            .field("mint_infos", &self.mint_infos)
            .finish_non_exhaustive()
    }
}

impl MintInfo {
    pub async fn from_url(url: String) -> Result<Self> {
        let client = MintClient::new(&url)?;

        let keysets = match client.get_keysets().await {
            Ok(response) => response.keysets,
            Err(_) => Vec::new(),
        };

        let (name, description) = match client.get_info().await {
            Ok(info) => (info.name, info.description),
            Err(_) => (None, None),
        };

        Ok(Self {
            url,
            keysets,
            name,
            description,
            active: true,
        })
    }
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

        let mints = mints.into_iter().collect();
        let mint_infos = HashMap::new();

        let mut wallet = Self {
            client,
            mints,
            mint_infos,
        };
        wallet.initialize_mint_infos().await?;

        Ok(wallet)
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

        let mints = mints.into_iter().collect();
        let mint_infos = HashMap::new();

        let mut wallet = Self {
            client,
            mints,
            mint_infos,
        };

        wallet.publish_wallet_config().await?;

        wallet.initialize_mint_infos().await?;

        Ok(wallet)
    }

    pub async fn initialize_mint_infos(&mut self) -> Result<()> {
        for mint_url in &self.mints {
            if !self.mint_infos.contains_key(mint_url) {
                match MintInfo::from_url(mint_url.clone()).await {
                    Ok(mint_info) => {
                        self.mint_infos.insert(mint_url.clone(), mint_info);
                    }
                    Err(_) => {
                        let basic_info = MintInfo {
                            url: mint_url.clone(),
                            keysets: Vec::new(),
                            name: None,
                            description: None,
                            active: false,
                        };
                        self.mint_infos.insert(mint_url.clone(), basic_info);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_mint_info(&self, mint_url: &str) -> Option<&MintInfo> {
        self.mint_infos.get(mint_url)
    }

    pub fn get_mint_url_by_keyset_id(&self, keyset_id: &str) -> Option<String> {
        for (mint_url, mint_info) in &self.mint_infos {
            for keyset in &mint_info.keysets {
                if keyset.id == keyset_id {
                    return Some(mint_url.clone());
                }
            }
        }
        None
    }

    pub fn get_mint_info_by_keyset_id(&self, keyset_id: &str) -> Option<MintInfo> {
        for mint_info in self.mint_infos.values() {
            for keyset in &mint_info.keysets {
                if keyset.id == keyset_id {
                    return Some(mint_info.clone());
                }
            }
        }
        None
    }

    pub fn get_mint_keysets(&self, mint_url: &str) -> Vec<KeysetInfo> {
        self.mint_infos
            .get(mint_url)
            .map(|info| info.keysets.clone())
            .unwrap_or_default()
    }

    pub fn get_active_keysets(&self, mint_url: &str) -> Vec<KeysetInfo> {
        self.get_mint_keysets(mint_url)
            .into_iter()
            .filter(|k| k.active)
            .collect()
    }

    pub async fn refresh_mint_info(&mut self, mint_url: &str) -> Result<()> {
        match MintInfo::from_url(mint_url.to_string()).await {
            Ok(mint_info) => {
                self.mint_infos.insert(mint_url.to_string(), mint_info);
                Ok(())
            }
            Err(e) => Err(e),
        }
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

            let config: WalletConfig = match serde_json::from_str::<WalletConfig>(&decrypted) {
                Ok(config) => config,
                Err(_) => match serde_json::from_str::<Vec<Vec<String>>>(&decrypted) {
                    Ok(nip60_array) => {
                        let mut privkey = None;
                        let mut mints = Vec::new();

                        for pair in nip60_array {
                            if pair.len() == 2 {
                                match pair[0].as_str() {
                                    "privkey" => privkey = Some(pair[1].clone()),
                                    "mint" => mints.push(pair[1].clone()),
                                    _ => {}
                                }
                            }
                        }

                        WalletConfig { privkey, mints }
                    }
                    Err(e) => {
                        return Err(crate::error::Error::custom(&format!(
                            "Invalid wallet config format: {}",
                            e
                        )));
                    }
                },
            };

            let mut wallet = Self {
                client,
                mints: config.mints,
                mint_infos: HashMap::new(),
            };
            wallet.initialize_mint_infos().await?;
            return Ok(Some(wallet));
        }

        Ok(None)
    }

    /// Publish wallet configuration to Nostr (kind 17375)
    async fn publish_wallet_config(&self) -> Result<()> {
        let mut nip60_config = Vec::new();

        for mint in &self.mints {
            nip60_config.push(vec!["mint".to_string(), mint.clone()]);
        }

        let content_json = serde_json::to_string(&nip60_config)
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

        let event_builder = EventBuilder::new(kinds::WALLET, encrypted_content);

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
        unspent_proofs: Proofs,
    ) -> Result<()> {
        let mut new_token_event_id = None;
        if !unspent_proofs.is_empty() {
            new_token_event_id = Some(
                self.create_rollover_token_event(unspent_proofs, &spent_token_ids)
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
        let empty_keysets: Vec<KeySetInfo> = vec![];
        let proofs = parsed_token.proofs(&empty_keysets).map_err(|e| {
            crate::error::Error::custom(&format!("Failed to get proofs from token: {}", e))
        })?;
        let token_event_id = self.create_token_event(&mint_url, proofs, vec![]).await?;
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

    pub fn calculate_token_amount(&self, token: &Token) -> Result<u64> {
        let _mint_url = token
            .mint_url()
            .map_err(|e| crate::error::Error::custom(&format!("Failed to get mint URL: {}", e)))?
            .to_string();
        let empty_keysets: Vec<KeySetInfo> = vec![];
        let proofs = token.proofs(&empty_keysets).map_err(|e| {
            crate::error::Error::custom(&format!("Failed to get proofs from token: {}", e))
        })?;
        Ok(proofs
            .iter()
            .map(|proof| {
                let amount: u64 = proof.amount.into();
                amount
            })
            .sum())
    }

    pub async fn calculate_balance(&self) -> Result<u64> {
        let wallet_state = self.fetch_wallet_state().await?;
        Ok(wallet_state.balance)
    }

    pub async fn fetch_wallet_state(&self) -> Result<WalletState> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| Error::custom(&format!("Failed to get signer: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| Error::custom(&format!("Failed to get public key: {}", e)))?;

        let filter = Filter::new().author(public_key).kinds(vec![
            kinds::WALLET,
            kinds::TOKEN,
            Kind::EventDeletion,
        ]);

        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await
            .map_err(|e| Error::custom(&format!("Failed to fetch events: {}", e)))?;

        // Find the newest wallet event
        let mut wallet_events: Vec<_> = events.iter().filter(|e| e.kind == kinds::WALLET).collect();
        wallet_events.sort_by_key(|e| e.created_at);

        if let Some(wallet_event) = wallet_events.last() {
            if let Ok(decrypted) = signer
                .nip44_decrypt(&public_key, &wallet_event.content)
                .await
            {
                if let Ok(_wallet_data) = serde_json::from_str::<serde_json::Value>(&decrypted) {}
            }
        }

        // Collect token events and track deleted events
        let mut deleted_ids: HashSet<String> = HashSet::new();
        for event in events.iter() {
            if event.kind == Kind::EventDeletion {
                for tag in event.tags.iter() {
                    if let Some(TagStandard::Event { event_id, .. }) = tag.as_standardized() {
                        deleted_ids.insert(event_id.to_hex());
                    }
                }
            }
        }

        let mut token_events: Vec<_> = events.iter().filter(|e| e.kind == kinds::TOKEN).collect();

        token_events.sort_by_key(|e| std::cmp::Reverse(e.created_at));

        let mut invalid_token_ids = deleted_ids;
        let mut proof_seen: HashSet<String> = HashSet::new();
        let mut all_proofs = Vec::new();
        let mut proof_to_event_id = HashMap::new();
        let mut undecryptable_events = Vec::new();

        for event in token_events {
            if invalid_token_ids.contains(&event.id.to_hex()) {
                continue;
            }

            // Try to decrypt token data
            let decrypted = match signer.nip44_decrypt(&public_key, &event.content).await {
                Ok(d) => d,
                Err(_) => {
                    undecryptable_events.push(event.id.to_hex());
                    continue;
                }
            };

            let token_data: TokenData = match serde_json::from_str(&decrypted) {
                Ok(d) => d,
                Err(_) => continue,
            };

            for del_id in &token_data.del {
                invalid_token_ids.insert(del_id.clone());
                undecryptable_events.retain(|id| id != del_id);
            }

            if invalid_token_ids.contains(&event.id.to_hex()) {
                continue;
            }

            for proof in &token_data.proofs {
                let secret_str = proof.secret.to_string();
                let _hex_secret = if let Ok(secret_bytes) = base64.decode(&secret_str) {
                    hex::encode(secret_bytes)
                } else {
                    secret_str
                };

                let proof_id = format!("{}", proof.c);
                if proof_seen.contains(&proof_id) {
                    continue;
                }
                proof_seen.insert(proof_id.clone());

                all_proofs.push(proof.clone());
                proof_to_event_id.insert(proof_id, event.id.to_hex());
            }
        }

        let balance = all_proofs
            .iter()
            .map(|p| p.amount.to_string().parse::<u64>().unwrap_or(0))
            .sum();

        let mut mint_keysets = HashMap::new();
        for mint in &self.mints {
            let keysets_data: Vec<HashMap<String, String>> = self
                .get_mint_keysets(mint)
                .into_iter()
                .map(|k| {
                    let mut map = HashMap::new();
                    map.insert("id".to_string(), k.id);
                    map.insert("unit".to_string(), k.unit);
                    map.insert("active".to_string(), k.active.to_string());
                    map
                })
                .collect();
            mint_keysets.insert(mint.clone(), keysets_data);
        }

        Ok(WalletState {
            balance,
            proofs: all_proofs,
            proof_to_event_id,
            mint_keysets,
        })
    }

    pub async fn get_balance(&self) -> Result<u64> {
        let state = self.fetch_wallet_state().await?;
        Ok(state.balance)
    }

    pub async fn get_unspent_proofs(&self) -> Result<Proofs> {
        let state = self.fetch_wallet_state().await?;
        println!("{:?}", state);
        Ok(state.proofs)
    }

    pub async fn get_wallet_state(&self) -> Result<WalletState> {
        self.fetch_wallet_state().await
    }

    pub async fn calculate_balance_from_history(
        &self,
    ) -> Result<(u64, u64, i64, Vec<SpendingHistory>)> {
        let history = self.get_spending_history().await?;

        let mut total_in = 0u64;
        let mut total_out = 0u64;
        let mut transactions_in = Vec::new();
        let mut transactions_out = Vec::new();

        for entry in &history {
            match entry.amount.parse::<u64>() {
                Ok(amount) => match entry.direction.as_str() {
                    "in" => {
                        total_in += amount;
                        transactions_in.push(entry.clone());
                    }
                    "out" => {
                        total_out += amount;
                        transactions_out.push(entry.clone());
                    }
                    _ => {
                        continue;
                    }
                },
                Err(_) => {
                    continue;
                }
            }
        }

        // Calculate net balance (can be negative if more out than in)
        let net_balance = total_in as i64 - total_out as i64;

        Ok((total_in, total_out, net_balance, history))
    }

    pub async fn fetch_token_events(&self) -> Result<Vec<TokenEvent>> {
        let state = self.fetch_wallet_state().await?;

        let mut events_map: HashMap<String, (Proofs, String)> = HashMap::new();

        for proof in state.proofs {
            if let Some(event_id) = state.proof_to_event_id.get(&proof.c.to_string()) {
                events_map
                    .entry(event_id.clone())
                    .or_insert_with(|| (Vec::new(), String::new()))
                    .0
                    .push(proof);
            }
        }

        let mut token_events = Vec::new();
        for (event_id_str, (proofs, _)) in events_map {
            if let Ok(event_id) = EventId::from_hex(&event_id_str) {
                let mint = self.mints.first().cloned().unwrap_or_default();

                token_events.push(TokenEvent {
                    id: event_id,
                    data: TokenData {
                        mint,
                        proofs,
                        del: Vec::new(),
                    },
                    created_at: Timestamp::now(),
                });
            }
        }

        Ok(token_events)
    }

    /// Create a new token event (kind 7375)
    async fn create_token_event(
        &self,
        mint: &str,
        proofs: Proofs,
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
        unspent_proofs: Proofs,
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
        let mut encrypted_event_refs = Vec::new();
        let mut unencrypted_tags = Vec::new();

        for (tag_name, event_id, relay, marker) in &event_refs {
            if marker == "redeemed" {
                unencrypted_tags.push(Tag::custom(
                    TagKind::Custom(tag_name.clone().into()),
                    [event_id, relay, marker],
                ));
            } else {
                encrypted_event_refs.push((
                    tag_name.clone(),
                    event_id.clone(),
                    relay.clone(),
                    marker.clone(),
                ));
            }
        }

        let history = SpendingHistory {
            direction: direction.to_string(),
            amount: amount.to_string(),
            events: encrypted_event_refs,
            created_at: Some(Timestamp::now().as_u64()),
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

        let event_builder =
            EventBuilder::new(kinds::SPENDING_HISTORY, encrypted_content).tags(unencrypted_tags);

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

            let mut spending_history = match serde_json::from_str::<SpendingHistory>(&decrypted) {
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

            if spending_history.created_at.is_none() {
                spending_history.created_at = Some(event.created_at.as_u64());
            }

            history.push(spending_history);
        }

        history.sort_by(|a, b| {
            match (a.created_at, b.created_at) {
                (Some(a_time), Some(b_time)) => b_time.cmp(&a_time), // Newest first
                (Some(_), None) => std::cmp::Ordering::Less,         // Some before None
                (None, Some(_)) => std::cmp::Ordering::Greater,      // None after Some
                (None, None) => std::cmp::Ordering::Equal,           // Equal if both None
            }
        });

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
                    _ => {}
                }
            }
        }

        Ok(SpendingHistory {
            direction,
            amount,
            events,
            created_at: None,
        })
    }

    pub fn get_all_mint_infos(&self) -> Vec<&MintInfo> {
        self.mint_infos.values().collect()
    }

    pub async fn get_stats(&self) -> Result<WalletStats> {
        let state = self.fetch_wallet_state().await?;

        Ok(WalletStats {
            balance: state.balance,
            token_events: state.proof_to_event_id.len(),
            mints: self.mints.clone(),
        })
    }

    pub async fn send_to_pubkey(
        &self,
        recipient_pubkey: PublicKey,
        amount: u64,
        memo: Option<String>,
    ) -> Result<EventId> {
        let (selected_proofs, remaining_proofs, spent_event_ids) =
            self.select_proofs_for_amount(amount).await?;

        if selected_proofs.is_empty() {
            return Err(crate::error::Error::custom("Insufficient balance"));
        }

        let mint_url = selected_proofs
            .first()
            .map(|p| p.keyset_id.to_string().clone())
            .unwrap_or_else(|| self.mints.first().cloned().unwrap_or_default());

        let token_string =
            self.create_cashu_token_string(&mint_url, selected_proofs, memo, None)?;

        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Signer error: {}", e)))?;

        let encrypted_content = signer
            .nip44_encrypt(&recipient_pubkey, &token_string)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Encryption failed: {}", e)))?;

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
        proofs: Proofs,
        memo: Option<String>,
        unit: Option<CurrencyUnit>,
    ) -> Result<String> {
        let currency_unit = if let Some(provided_unit) = unit {
            provided_unit
        } else {
            self.determine_unit_from_proofs(mint_url, &proofs)?
        };

        let token = Token::new(
            MintUrl::from_str(mint_url)
                .map_err(|e| crate::error::Error::custom(&format!("Invalid mint URL: {}", e)))?,
            proofs,
            memo,
            currency_unit,
        );

        Ok(token.to_string())
    }

    fn determine_unit_from_proofs(&self, mint_url: &str, proofs: &Proofs) -> Result<CurrencyUnit> {
        let first_proof = proofs
            .first()
            .ok_or_else(|| crate::error::Error::custom("No proofs provided"))?;

        let keyset_id = &first_proof.keyset_id.to_string();

        if let Some(mint_info) = self.get_mint_info(mint_url) {
            for keyset in &mint_info.keysets {
                if keyset.id == *keyset_id {
                    return match keyset.unit.as_str() {
                        "sat" => Ok(CurrencyUnit::Sat),
                        "msat" => Ok(CurrencyUnit::Msat),
                        "usd" => Ok(CurrencyUnit::Usd),
                        "eur" => Ok(CurrencyUnit::Eur),
                        _ => CurrencyUnit::from_str(&keyset.unit)
                            .map_err(|_| {
                                crate::error::Error::custom(&format!(
                                    "Unknown currency unit: {}. Defaulting to sat.",
                                    keyset.unit
                                ))
                            })
                            .or(Ok(CurrencyUnit::Sat)),
                    };
                }
            }
        }

        Ok(CurrencyUnit::Sat)
    }

    async fn select_proofs_for_amount(
        &self,
        amount: u64,
    ) -> Result<(Proofs, Proofs, Vec<EventId>)> {
        let state = self.fetch_wallet_state().await?;

        let mut selected_proofs = Vec::new();
        let mut remaining_proofs = Vec::new();
        let mut spent_event_ids = HashSet::new();
        let mut current_amount = 0u64;

        let mut available_proofs = state.proofs.clone();
        available_proofs.sort_by(|a, b| b.amount.cmp(&a.amount));

        for proof in available_proofs {
            if current_amount < amount {
                selected_proofs.push(proof.clone());
                current_amount += proof.amount.to_string().parse::<u64>().unwrap();

                if let Some(event_id_str) =
                    state.proof_to_event_id.get(&proof.keyset_id.to_string())
                {
                    if let Ok(event_id) = EventId::from_hex(event_id_str) {
                        spent_event_ids.insert(event_id);
                    }
                }
            } else {
                remaining_proofs.push(proof);
            }
        }

        if current_amount < amount {
            return Err(crate::error::Error::custom(&format!(
                "Insufficient balance: need {}, have {}",
                amount, current_amount
            )));
        }

        let spent_event_ids: Vec<EventId> = spent_event_ids.into_iter().collect();
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
            privkey: None,
        }
    }

    pub async fn update_config(&mut self, mints: Option<Vec<String>>) -> Result<()> {
        if let Some(mints) = mints {
            self.mints = mints.into_iter().collect();
            self.initialize_mint_infos().await?;
            self.publish_wallet_config().await?;
        }
        Ok(())
    }

    pub async fn send(&self, amount: u64, memo: Option<String>) -> Result<String> {
        self.send_with_target_mint(amount, None, memo).await
    }

    pub async fn send_with_target_mint(
        &self,
        amount: u64,
        target_mint: Option<String>,
        memo: Option<String>,
    ) -> Result<String> {
        let state = self.fetch_wallet_state().await?;

        if state.balance < amount {
            return Err(crate::error::Error::custom(&format!(
                "Insufficient balance: need {}, have {}",
                amount, state.balance
            )));
        }

        let mint_url =
            target_mint.unwrap_or_else(|| self.mints.first().cloned().unwrap_or_default());

        let (selected_proofs, all_consumed_proofs, spent_event_ids) = self
            .select_proofs_for_exact_amount(amount, &mint_url)
            .await?;

        let token_string =
            self.create_cashu_token_string(&mint_url, selected_proofs.clone(), memo, None)?;

        let remaining_proofs: Vec<_> = all_consumed_proofs
            .into_iter()
            .filter(|proof| !selected_proofs.iter().any(|sp| sp.c == proof.c))
            .collect();

        let mut event_refs = Vec::new();
        for event_id in &spent_event_ids {
            event_refs.push((
                "e".to_string(),
                event_id.to_hex(),
                "".to_string(),
                "destroyed".to_string(),
            ));
        }

        if !remaining_proofs.is_empty() {
            let new_token_event_id = self
                .create_rollover_token_event(remaining_proofs, &spent_event_ids)
                .await?;
            event_refs.push((
                "e".to_string(),
                new_token_event_id.to_hex(),
                "".to_string(),
                "created".to_string(),
            ));
        }

        let total_amount: u64 = selected_proofs
            .iter()
            .map(|p| p.amount.to_string().parse::<u64>().unwrap_or(0))
            .sum();

        self.create_spending_history("out", total_amount, event_refs)
            .await?;

        for event_id in &spent_event_ids {
            self.delete_token_event(event_id).await?;
        }

        Ok(token_string)
    }

    pub async fn redeem(&self, token_string: &str) -> Result<u64> {
        let parsed_token = self.parse_cashu_token(token_string)?;
        let mint_url = parsed_token
            .mint_url()
            .map_err(|e| crate::error::Error::custom(&format!("Failed to get mint URL: {}", e)))?
            .to_string();

        let is_trusted_mint = self.mints.iter().any(|m| m.clone() == mint_url);
        if !is_trusted_mint {
            return Err(crate::error::Error::custom(
                "Cannot redeem tokens from untrusted mint",
            ));
        }

        let mint_unit = if let Some(mint_info) = self.get_mint_info(&mint_url) {
            if let Some(active_keyset) = mint_info.keysets.iter().find(|k| k.active) {
                match active_keyset.unit.as_str() {
                    "sat" => cdk::nuts::CurrencyUnit::Sat,
                    "msat" => cdk::nuts::CurrencyUnit::Msat,
                    "usd" => cdk::nuts::CurrencyUnit::Usd,
                    "eur" => cdk::nuts::CurrencyUnit::Eur,
                    _ => cdk::nuts::CurrencyUnit::from_str(&active_keyset.unit)
                        .unwrap_or(cdk::nuts::CurrencyUnit::Sat),
                }
            } else if let Some(first_keyset) = mint_info.keysets.first() {
                match first_keyset.unit.as_str() {
                    "sat" => cdk::nuts::CurrencyUnit::Sat,
                    "msat" => cdk::nuts::CurrencyUnit::Msat,
                    "usd" => cdk::nuts::CurrencyUnit::Usd,
                    "eur" => cdk::nuts::CurrencyUnit::Eur,
                    _ => cdk::nuts::CurrencyUnit::from_str(&first_keyset.unit)
                        .unwrap_or(cdk::nuts::CurrencyUnit::Sat),
                }
            } else {
                cdk::nuts::CurrencyUnit::Sat
            }
        } else {
            parsed_token.unit().unwrap_or(cdk::nuts::CurrencyUnit::Sat)
        };

        let temp_mnemonic = Mnemonic::generate(12).map_err(|e| {
            crate::error::Error::custom(&format!("Failed to generate mnemonic: {}", e))
        })?;
        let temp_seed = temp_mnemonic.to_string();
        let temp_db_name = format!("temp_redeem_{}", crate::crypto::generate_random_secret());

        let temp_wallet =
            CashuWalletClient::from_seed_with_unit(&mint_url, &temp_seed, &temp_db_name, mint_unit)
                .await
                .map_err(|e| {
                    crate::error::Error::custom(&format!("Failed to create temp wallet: {}", e))
                })?;

        temp_wallet
            .receive(token_string)
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Failed to redeem token: {}", e)))?;

        let balance_str = temp_wallet
            .balance()
            .await
            .map_err(|e| crate::error::Error::custom(&format!("Failed to get balance: {}", e)))?;
        let redeemed_amount: u64 = balance_str
            .parse()
            .map_err(|e| crate::error::Error::custom(&format!("Failed to parse balance: {}", e)))?;

        let new_token_string = temp_wallet.send(redeemed_amount).await.map_err(|e| {
            crate::error::Error::custom(&format!("Failed to send from temp wallet: {}", e))
        })?;

        let new_parsed_token = self.parse_cashu_token(&new_token_string)?;
        let empty_keysets: Vec<KeySetInfo> = vec![];
        let final_proofs = new_parsed_token.proofs(&empty_keysets).map_err(|e| {
            crate::error::Error::custom(&format!("Failed to get proofs from token: {}", e))
        })?;

        let token_event_id = self
            .create_token_event(&mint_url, final_proofs, vec![])
            .await?;

        let event_refs = vec![(
            "e".to_string(),
            token_event_id.to_hex(),
            "".to_string(),
            "created".to_string(),
        )];

        self.create_spending_history("in", redeemed_amount, event_refs)
            .await?;

        Ok(redeemed_amount)
    }

    async fn select_proofs_for_exact_amount(
        &self,
        amount: u64,
        target_mint: &str,
    ) -> Result<(Proofs, Proofs, Vec<EventId>)> {
        let state = self.fetch_wallet_state().await?;
        let available_proofs: Vec<_> = state.proofs.iter().filter(|_p| true).cloned().collect();

        let available_amount: u64 = available_proofs
            .iter()
            .map(|p| p.amount.to_string().parse::<u64>().unwrap_or(0))
            .sum();

        if available_amount < amount {
            return Err(crate::error::Error::custom(&format!(
                "Insufficient balance in target mint {}: need {}, have {}",
                target_mint, amount, available_amount
            )));
        }

        let mut selected_proofs = Vec::new();
        let mut selected_total = 0u64;
        let mut spent_event_ids = HashSet::new();

        let mut sorted_proofs = available_proofs.clone();
        sorted_proofs.sort_by(|a, b| b.amount.cmp(&a.amount)); // largest → smallest

        let mut remaining = amount;
        for proof in &sorted_proofs {
            if remaining == 0 {
                break;
            }

            let value = proof.amount.to_string().parse::<u64>().unwrap_or(0);
            if value == 0 || value > remaining {
                continue; // skip anything that would overshoot
            }

            selected_proofs.push(proof.clone());
            selected_total += value;
            remaining -= value;

            if let Some(event_id_str) = state.proof_to_event_id.get(&proof.c.to_string()) {
                if let Ok(event_id) = EventId::from_hex(event_id_str) {
                    spent_event_ids.insert(event_id);
                }
            }
        }

        if remaining != 0 {
            return Err(crate::error::Error::custom(&format!(
                "Could not find an exact combination of proofs to cover {} sats ({} sats missing)",
                amount, remaining
            )));
        }

        if selected_total < amount {
            return Err(crate::error::Error::custom(&format!(
                "Could not select sufficient proofs: need {}, selected {}",
                amount, selected_total
            )));
        }

        if selected_total == amount {
            let spent_event_ids: Vec<EventId> = spent_event_ids.into_iter().collect();
            return Ok((selected_proofs.clone(), selected_proofs, spent_event_ids));
        }

        let change_amount = selected_total - amount;
        let (send_proofs, change_proofs) = self
            .split_proofs_for_amounts(selected_proofs, amount, change_amount, target_mint)
            .await?;

        if !change_proofs.is_empty() {
            self.create_token_event(target_mint, change_proofs, vec![])
                .await?;
        }

        let spent_event_ids: Vec<EventId> = spent_event_ids.into_iter().collect();
        Ok((send_proofs, available_proofs, spent_event_ids))
    }

    async fn split_proofs_for_amounts(
        &self,
        input_proofs: Proofs,
        send_amount: u64,
        change_amount: u64,
        _mint_url: &str,
    ) -> Result<(Proofs, Proofs)> {
        let total_input: u64 = input_proofs
            .iter()
            .map(|p| p.amount.to_string().parse::<u64>().unwrap_or(0))
            .sum();

        if total_input != send_amount + change_amount {
            return Err(crate::error::Error::custom(&format!(
                "Amount mismatch: input={}, send={}, change={}",
                total_input, send_amount, change_amount
            )));
        }

        let _send_denoms = self.calculate_optimal_denominations(send_amount);
        let _change_denoms = self.calculate_optimal_denominations(change_amount);

        Ok((input_proofs, Vec::new()))
    }

    fn calculate_optimal_denominations(&self, amount: u64) -> std::collections::HashMap<u64, u32> {
        let mut denominations = std::collections::HashMap::new();
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

    pub async fn get_event_history_by_mint(
        &self,
        mint_url: Option<String>,
    ) -> Result<Vec<EventHistoryByMint>> {
        let mut history_by_mint: HashMap<String, EventHistoryByMint> = HashMap::new();

        let target_mints = if let Some(url) = mint_url {
            vec![url]
        } else {
            self.mints.to_vec()
        };

        for mint in &target_mints {
            history_by_mint.insert(
                mint.clone(),
                EventHistoryByMint {
                    mint: mint.clone(),
                    total_received: 0,
                    total_spent: 0,
                    events: Vec::new(),
                },
            );
        }

        let spending_history = self.get_spending_history().await?;

        for history in spending_history {
            for (mint, _, amount, event_id) in history.events {
                if let Some(entry) = history_by_mint.get_mut(&mint) {
                    let amount_value = amount.parse::<u64>().unwrap_or(0);

                    let detail = EventDetail {
                        event_id: event_id.clone(),
                        direction: history.direction.clone(),
                        amount: amount_value,
                        timestamp: history.created_at.unwrap_or(0),
                        memo: None,
                    };

                    if history.direction == "in" {
                        entry.total_received += amount_value;
                    } else if history.direction == "out" {
                        entry.total_spent += amount_value;
                    }

                    entry.events.push(detail);
                }
            }
        }

        for history in history_by_mint.values_mut() {
            history.events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        }

        Ok(history_by_mint.into_values().collect())
    }

    pub fn get_proof_breakdown(&self, proofs: &Proofs) -> Vec<ProofBreakdown> {
        let mut breakdowns: std::collections::HashMap<String, ProofBreakdown> =
            std::collections::HashMap::new();

        for proof in proofs {
            let mint_info = self
                .get_mint_info_by_keyset_id(&proof.keyset_id.to_string())
                .unwrap();
            let entry = breakdowns.entry(mint_info.url.clone()).or_insert_with(|| {
                ProofBreakdown::new(
                    mint_info.url,
                    Some(mint_info.keysets.first().unwrap().unit.clone()),
                )
            });

            entry.add_proof(proof.amount.into());
        }

        let mut result: Vec<_> = breakdowns.into_values().collect();
        result.sort_by(|a, b| b.total_balance.cmp(&a.total_balance));
        result
    }

    pub async fn get_proof_breakdown_string(&self) -> Result<String> {
        println!("{:?}", self.token_events().await.unwrap());
        let proofs = self.get_unspent_proofs().await?;
        let breakdowns = self.get_proof_breakdown(&proofs);

        if breakdowns.is_empty() {
            return Ok("No proofs found".to_string());
        }

        let mut output = String::from("\nProof Breakdown by Mint:");
        for breakdown in breakdowns {
            output.push_str("\n    ");
            output.push_str(&breakdown.to_string());
        }

        Ok(output)
    }

    pub async fn token_events(&self) -> Result<Vec<TokenEvent>> {
        let signer = self
            .client
            .signer()
            .await
            .map_err(|e| Error::custom(&format!("Failed to get signer: {}", e)))?;

        let public_key = signer
            .get_public_key()
            .await
            .map_err(|e| Error::custom(&format!("Failed to get public key: {}", e)))?;

        let filter = Filter::new().author(public_key).kinds(vec![kinds::TOKEN]);

        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await
            .map_err(|e| Error::custom(&format!("Failed to fetch events: {}", e)))?;

        let mut wallet_events: Vec<_> = events.iter().filter(|e| e.kind == kinds::TOKEN).collect();
        wallet_events.sort_by_key(|e| e.created_at);

        if let Some(wallet_event) = wallet_events.last() {
            if let Ok(decrypted) = signer
                .nip44_decrypt(&public_key, &wallet_event.content)
                .await
            {
                println!("{:?}", decrypted);
                if let Ok(wallet_data) = serde_json::from_str::<Vec<TokenEvent>>(&decrypted) {
                    Ok(wallet_data)
                } else {
                    Err(Error::NotEnoughBalance("test".to_string()))
                }
            } else {
                Err(Error::NotEnoughBalance("test".to_string()))
            }
        } else {
            Err(Error::NotEnoughBalance("test".to_string()))
        }
    }
}
