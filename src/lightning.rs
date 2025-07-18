use crate::{
    error::{Error, Result},
    mint::MintClient,
    multimint::MultimintWallet,
    wallet::CashuWalletClient,
};
use cdk::nuts::CurrencyUnit;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningInvoice {
    pub mint_url: String,
    pub quote_id: String,
    pub payment_request: String,
    pub amount: u64,
    pub unit: CurrencyUnit,
    pub expiry: u64,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatus {
    pub quote_id: String,
    pub paid: bool,
    pub amount: u64,
    pub expiry: u64,
    pub payment_preimage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopupRequest {
    pub amount: u64,
    pub unit: CurrencyUnit,
    pub mint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopupResponse {
    pub invoice: LightningInvoice,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopupResult {
    pub success: bool,
    pub amount_received: u64,
    pub mint_url: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LightningManager {
    pending_invoices: HashMap<String, LightningInvoice>,
}

impl LightningManager {
    pub fn new() -> Self {
        Self {
            pending_invoices: HashMap::new(),
        }
    }

    pub async fn create_invoice_for_wallet(
        &mut self,
        _wallet: &CashuWalletClient,
        mint_url: &str,
        amount: u64,
        unit: CurrencyUnit,
    ) -> Result<TopupResponse> {
        let mint_client = MintClient::new(mint_url)?;

        let quote_response = mint_client.request_mint_quote(amount, unit.clone()).await?;

        let invoice = LightningInvoice {
            mint_url: mint_url.to_string(),
            quote_id: quote_response.quote.clone(),
            payment_request: quote_response.request.clone(),
            amount,
            unit,
            expiry: quote_response.expiry,
            created_at: Instant::now(),
        };

        self.pending_invoices
            .insert(quote_response.quote.clone(), invoice.clone());

        Ok(TopupResponse {
            invoice,
            message: format!("Lightning invoice created for {} msats", amount),
        })
    }

    pub async fn create_invoice_for_multimint(
        &mut self,
        _multimint: &MultimintWallet,
        request: TopupRequest,
    ) -> Result<TopupResponse> {
        let mint_url = request
            .mint_url
            .clone()
            .ok_or_else(|| Error::custom("Mint URL is required"))?;

        let mint_client = MintClient::new(&mint_url)?;

        let request_unit = request.unit.clone();
        let quote_response = mint_client
            .request_mint_quote(request.amount, request.unit)
            .await?;

        let invoice = LightningInvoice {
            mint_url: mint_url.clone(),
            quote_id: quote_response.quote.clone(),
            payment_request: quote_response.request.clone(),
            amount: request.amount,
            unit: request_unit,
            expiry: quote_response.expiry,
            created_at: Instant::now(),
        };

        self.pending_invoices
            .insert(quote_response.quote.clone(), invoice.clone());

        Ok(TopupResponse {
            invoice,
            message: format!(
                "Lightning invoice created for {} msats at mint {}",
                request.amount, mint_url
            ),
        })
    }

    pub async fn check_payment_status(&self, quote_id: &str) -> Result<PaymentStatus> {
        let invoice = self
            .pending_invoices
            .get(quote_id)
            .ok_or_else(|| Error::custom("Invoice not found"))?;

        let mint_client = MintClient::new(&invoice.mint_url)?;

        let quote_response = mint_client.get_mint_quote(quote_id).await?;

        Ok(PaymentStatus {
            quote_id: quote_id.to_string(),
            paid: quote_response.paid,
            amount: invoice.amount,
            expiry: quote_response.expiry,
            payment_preimage: None,
        })
    }

    pub fn list_pending_invoices(&self) -> Vec<&LightningInvoice> {
        self.pending_invoices.values().collect()
    }

    pub fn get_invoice(&self, quote_id: &str) -> Option<&LightningInvoice> {
        self.pending_invoices.get(quote_id)
    }

    pub fn cleanup_expired_invoices(&mut self) {
        let now = Instant::now();
        self.pending_invoices.retain(|_, invoice| {
            let expiry_duration = Duration::from_secs(invoice.expiry);
            now.duration_since(invoice.created_at) < expiry_duration
        });
    }

    pub async fn wait_for_payment(
        &self,
        quote_id: &str,
        timeout_seconds: u64,
        poll_interval_seconds: u64,
    ) -> Result<bool> {
        let start_time = Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);
        let poll_interval = Duration::from_secs(poll_interval_seconds);

        while start_time.elapsed() < timeout {
            let status = self.check_payment_status(quote_id).await?;
            if status.paid {
                return Ok(true);
            }

            tokio::time::sleep(poll_interval).await;
        }

        Ok(false)
    }

    pub fn remove_invoice(&mut self, quote_id: &str) -> Option<LightningInvoice> {
        self.pending_invoices.remove(quote_id)
    }

    pub fn get_payment_request(&self, quote_id: &str) -> Option<&str> {
        self.pending_invoices
            .get(quote_id)
            .map(|invoice| invoice.payment_request.as_str())
    }
}

impl Default for LightningManager {
    fn default() -> Self {
        Self::new()
    }
}
