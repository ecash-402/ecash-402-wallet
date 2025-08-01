use crate::error::{Error, Result};
use cdk::mint_url::MintUrl;
use cdk::nuts::{BlindSignature, BlindedMessage, CurrencyUnit, Proof};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInfoResponse {
    pub name: Option<String>,
    pub pubkey: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub description_long: Option<String>,
    pub contact: Option<Vec<Vec<String>>>,
    pub motd: Option<String>,
    pub nuts: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysResponse {
    pub keysets: Vec<KeysetResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetResponse {
    pub id: String,
    pub unit: String,
    pub keys: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetsResponse {
    pub keysets: Vec<KeysetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetInfo {
    pub id: String,
    pub unit: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMintQuoteRequest {
    pub amount: u64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMintQuoteResponse {
    pub quote: String,
    pub request: String,
    pub paid: bool,
    pub expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMintRequest {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMintResponse {
    pub signatures: Vec<BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMeltQuoteRequest {
    pub request: String,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMeltQuoteResponse {
    pub quote: String,
    pub amount: u64,
    pub fee_reserve: u64,
    pub paid: bool,
    pub expiry: u64,
    pub payment_preimage: Option<String>,
    pub change: Option<Vec<BlindSignature>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMeltRequest {
    pub quote: String,
    pub inputs: Vec<Proof>,
    pub outputs: Option<Vec<BlindedMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSwapRequest {
    pub inputs: Vec<Proof>,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostSwapResponse {
    pub signatures: Vec<BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCheckStateRequest {
    pub ys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCheckStateResponse {
    pub states: Vec<ProofState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofState {
    pub y: String,
    pub state: String,
    pub witness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRestoreRequest {
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRestoreResponse {
    pub outputs: Vec<BlindedMessage>,
    pub signatures: Vec<BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPValidationError {
    pub detail: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub loc: Vec<serde_json::Value>,
    pub msg: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

pub struct MintClient {
    client: Client,
    mint_url: MintUrl,
}

impl MintClient {
    pub fn new(mint_url: &str) -> Result<Self> {
        let mint_url = MintUrl::from_str(mint_url)
            .map_err(|e| Error::custom(&format!("Invalid mint URL: {}", e)))?;

        let client = Client::new();

        Ok(Self { client, mint_url })
    }

    pub async fn get_info(&self) -> Result<GetInfoResponse> {
        let url = format!("{}/v1/info", self.mint_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let info: GetInfoResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(info)
    }

    pub async fn get_keys(&self) -> Result<KeysResponse> {
        let url = format!("{}/v1/keys", self.mint_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let keys: KeysResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(keys)
    }

    pub async fn get_keyset_keys(&self, keyset_id: &str) -> Result<KeysResponse> {
        let url = format!("{}/v1/keys/{}", self.mint_url, keyset_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let keys: KeysResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(keys)
    }

    pub async fn get_keysets(&self) -> Result<KeysetsResponse> {
        let url = format!("{}/v1/keysets", self.mint_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let keysets: KeysetsResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(keysets)
    }

    pub async fn request_mint_quote(
        &self,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<PostMintQuoteResponse> {
        let url = format!("{}/v1/mint/quote/bolt11", self.mint_url);

        let request = PostMintQuoteRequest {
            amount,
            unit: unit.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let quote: PostMintQuoteResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(quote)
    }

    pub async fn get_mint_quote(&self, quote: &str) -> Result<PostMintQuoteResponse> {
        let url = format!("{}/v1/mint/quote/bolt11/{}", self.mint_url, quote);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let quote_response: PostMintQuoteResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(quote_response)
    }

    pub async fn mint_tokens(
        &self,
        quote: &str,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostMintResponse> {
        let url = format!("{}/v1/mint/bolt11", self.mint_url);

        let request = PostMintRequest {
            quote: quote.to_string(),
            outputs,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let mint_response: PostMintResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(mint_response)
    }

    pub async fn request_melt_quote(
        &self,
        request: &str,
        unit: CurrencyUnit,
    ) -> Result<PostMeltQuoteResponse> {
        let url = format!("{}/v1/melt/quote/bolt11", self.mint_url);

        let melt_request = PostMeltQuoteRequest {
            request: request.to_string(),
            unit: unit.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&melt_request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let quote: PostMeltQuoteResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(quote)
    }

    pub async fn get_melt_quote(&self, quote: &str) -> Result<PostMeltQuoteResponse> {
        let url = format!("{}/v1/melt/quote/bolt11/{}", self.mint_url, quote);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let quote_response: PostMeltQuoteResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(quote_response)
    }

    pub async fn melt_tokens(
        &self,
        quote: &str,
        inputs: Vec<Proof>,
        outputs: Option<Vec<BlindedMessage>>,
    ) -> Result<PostMeltQuoteResponse> {
        let url = format!("{}/v1/melt/bolt11", self.mint_url);

        let request = PostMeltRequest {
            quote: quote.to_string(),
            inputs,
            outputs,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let melt_response: PostMeltQuoteResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(melt_response)
    }

    pub async fn swap_tokens(
        &self,
        inputs: Vec<Proof>,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSwapResponse> {
        let url = format!("{}/v1/swap", self.mint_url);

        let request = PostSwapRequest { inputs, outputs };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let swap_response: PostSwapResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(swap_response)
    }

    pub async fn check_proof_state(&self, ys: Vec<String>) -> Result<PostCheckStateResponse> {
        let url = format!("{}/v1/checkstate", self.mint_url);

        let request = PostCheckStateRequest { ys };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let state_response: PostCheckStateResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(state_response)
    }

    pub async fn restore_signatures(
        &self,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostRestoreResponse> {
        let url = format!("{}/v1/restore", self.mint_url);

        let request = PostRestoreRequest { outputs };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::custom(&format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::custom(&format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let restore_response: PostRestoreResponse = response
            .json()
            .await
            .map_err(|e| Error::custom(&format!("Failed to parse response: {}", e)))?;

        Ok(restore_response)
    }

    pub async fn check_proofs_spent(&self, proofs: &[Proof]) -> Result<Vec<bool>> {
        let ys: Vec<String> = proofs
            .iter()
            .map(|p| match p.y() {
                Ok(y) => y.to_string(),
                Err(_) => String::new(),
            })
            .collect();

        let response = self.check_proof_state(ys).await?;

        let spent_states: Vec<bool> = response
            .states
            .iter()
            .map(|state| state.state == "SPENT")
            .collect();

        Ok(spent_states)
    }

    pub async fn validate_proofs(&self, proofs: &[Proof]) -> Result<Vec<Proof>> {
        let spent_states = self.check_proofs_spent(proofs).await?;

        let valid_proofs: Vec<Proof> = proofs
            .iter()
            .zip(spent_states.iter())
            .filter_map(
                |(proof, &is_spent)| {
                    if !is_spent {
                        Some(proof.clone())
                    } else {
                        None
                    }
                },
            )
            .collect();

        Ok(valid_proofs)
    }

    pub fn mint_url(&self) -> &MintUrl {
        &self.mint_url
    }

    pub fn url_string(&self) -> String {
        self.mint_url.to_string()
    }
}
