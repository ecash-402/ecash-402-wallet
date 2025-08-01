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
    fmt,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningInvoice {
    pub mint_url: String,
    pub quote_id: String,
    pub payment_request: String,
    pub amount: u64,
    pub unit: CurrencyUnit,
    pub expiry: u64,
    pub state: InvoiceState,
    pub fee_reserve: Option<u64>,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceState {
    Unpaid,
    Pending,
    Paid,
    Failed,
    Expired,
}

impl fmt::Display for InvoiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvoiceState::Unpaid => write!(f, "unpaid"),
            InvoiceState::Pending => write!(f, "pending"),
            InvoiceState::Paid => write!(f, "paid"),
            InvoiceState::Failed => write!(f, "failed"),
            InvoiceState::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatus {
    pub quote_id: String,
    pub state: InvoiceState,
    pub amount: u64,
    pub expiry: u64,
    pub fee_paid: Option<u64>,
    pub payment_preimage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopupRequest {
    pub amount: u64,
    pub unit: CurrencyUnit,
    pub mint_url: Option<String>,
    pub description: Option<String>,
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
    pub fee_paid: u64,
    pub mint_url: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltRequest {
    pub quote_id: String,
    pub payment_request: String,
    pub amount: u64,
    pub unit: CurrencyUnit,
    pub proofs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltResponse {
    pub success: bool,
    pub payment_preimage: Option<String>,
    pub change_proofs: Option<Vec<String>>,
    pub fee_paid: u64,
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
            state: InvoiceState::Unpaid,
            fee_reserve: None,
            created_at: Instant::now(),
        };

        self.pending_invoices
            .insert(quote_response.quote.clone(), invoice.clone());

        Ok(TopupResponse {
            invoice,
            message: format!("Lightning invoice created for {} sats", amount),
        })
    }

    pub async fn create_invoice_for_multimint(
        &mut self,
        multimint: &MultimintWallet,
        request: TopupRequest,
    ) -> Result<TopupResponse> {
        let mint_url = request
            .mint_url
            .clone()
            .ok_or_else(|| Error::custom("Mint URL is required"))?;

        let available_mints = multimint.list_mints().await;
        if !available_mints.contains(&mint_url) {
            return Err(Error::custom(&format!(
                "Mint {} not found in multimint wallet. Available mints: {:?}",
                mint_url, available_mints
            )));
        }

        let mint_client = MintClient::new(&mint_url)?;
        let quote_response = mint_client
            .request_mint_quote(request.amount, request.unit.clone())
            .await?;

        let invoice = LightningInvoice {
            mint_url: mint_url.clone(),
            quote_id: quote_response.quote.clone(),
            payment_request: quote_response.request.clone(),
            amount: request.amount,
            unit: request.unit,
            expiry: quote_response.expiry,
            state: InvoiceState::Unpaid,
            fee_reserve: None,
            created_at: Instant::now(),
        };

        self.pending_invoices
            .insert(quote_response.quote.clone(), invoice.clone());

        Ok(TopupResponse {
            invoice,
            message: format!(
                "Lightning invoice created for {} sats at mint {}",
                request.amount, mint_url
            ),
        })
    }

    pub async fn check_payment_status(&mut self, quote_id: &str) -> Result<PaymentStatus> {
        if self.pending_invoices.contains_key(quote_id) {
            let (mint_url, amount, expiry) = {
                let invoice = self.pending_invoices.get(quote_id).unwrap();
                (invoice.mint_url.clone(), invoice.amount, invoice.expiry)
            };

            let mint_client = MintClient::new(&mint_url)?;
            let quote_response = mint_client.get_mint_quote(quote_id).await?;

            let new_state = if quote_response.paid {
                InvoiceState::Paid
            } else if self.is_expired(expiry) {
                InvoiceState::Expired
            } else {
                InvoiceState::Unpaid
            };

            if let Some(invoice) = self.pending_invoices.get_mut(quote_id) {
                invoice.state = new_state.clone();
            }

            Ok(PaymentStatus {
                quote_id: quote_id.to_string(),
                state: new_state,
                amount,
                expiry,
                fee_paid: None,
                payment_preimage: None,
            })
        } else {
            Err(Error::custom("Invoice not found"))
        }
    }

    pub async fn request_melt_quote(
        &self,
        mint_url: &str,
        payment_request: &str,
        unit: CurrencyUnit,
    ) -> Result<String> {
        let mint_client = MintClient::new(mint_url)?;
        let quote_response = mint_client
            .request_melt_quote(payment_request, unit)
            .await?;
        Ok(quote_response.quote)
    }

    pub async fn execute_melt(
        &self,
        _mint_url: &str,
        _melt_request: MeltRequest,
    ) -> Result<MeltResponse> {
        Ok(MeltResponse {
            success: false,
            payment_preimage: None,
            change_proofs: None,
            fee_paid: 0,
            message: "Melt operation not yet implemented with proper proof types".to_string(),
        })
    }

    pub async fn check_melt_quote_state(
        &self,
        mint_url: &str,
        quote_id: &str,
    ) -> Result<InvoiceState> {
        let mint_client = MintClient::new(mint_url)?;

        match mint_client.get_melt_quote(quote_id).await {
            Ok(quote) => {
                if quote.paid {
                    Ok(InvoiceState::Paid)
                } else if self.is_expired(quote.expiry) {
                    Ok(InvoiceState::Expired)
                } else {
                    Ok(InvoiceState::Unpaid)
                }
            }
            Err(_) => Ok(InvoiceState::Failed),
        }
    }

    pub async fn complete_topup(
        &mut self,
        quote_id: &str,
        _wallet: &mut CashuWalletClient,
    ) -> Result<TopupResult> {
        let invoice = self
            .pending_invoices
            .get(quote_id)
            .ok_or_else(|| Error::custom("Invoice not found"))?;

        if invoice.state != InvoiceState::Paid {
            return Err(Error::custom("Invoice is not paid"));
        }

        Ok(TopupResult {
            success: true,
            amount_received: invoice.amount,
            fee_paid: 0,
            mint_url: invoice.mint_url.clone(),
            message: format!(
                "Invoice {} is ready for minting. Use the mint client to complete the minting process.",
                quote_id
            ),
        })
    }

    pub fn list_pending_invoices(&self) -> Vec<&LightningInvoice> {
        self.pending_invoices.values().collect()
    }

    pub fn get_invoice(&self, quote_id: &str) -> Option<&LightningInvoice> {
        self.pending_invoices.get(quote_id)
    }

    pub fn cleanup_expired_invoices(&mut self) {
        let expired_quotes: Vec<String> = self
            .pending_invoices
            .iter()
            .filter(|(_, invoice)| self.is_expired(invoice.expiry))
            .map(|(quote_id, _)| quote_id.clone())
            .collect();

        for quote_id in expired_quotes {
            self.pending_invoices.remove(&quote_id);
        }
    }

    pub async fn wait_for_payment(
        &mut self,
        quote_id: &str,
        timeout_seconds: u64,
        poll_interval_seconds: u64,
    ) -> Result<bool> {
        let start_time = Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);
        let poll_interval = Duration::from_secs(poll_interval_seconds);

        while start_time.elapsed() < timeout {
            let status = self.check_payment_status(quote_id).await?;
            if status.state == InvoiceState::Paid {
                return Ok(true);
            }
            if status.state == InvoiceState::Expired || status.state == InvoiceState::Failed {
                return Ok(false);
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

    pub fn is_expired(&self, expiry: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > expiry
    }

    pub fn get_invoice_state(&self, quote_id: &str) -> Option<InvoiceState> {
        self.pending_invoices.get(quote_id).map(|invoice| {
            if self.is_expired(invoice.expiry) {
                InvoiceState::Expired
            } else {
                invoice.state.clone()
            }
        })
    }

    pub fn update_invoice_state(&mut self, quote_id: &str, state: InvoiceState) -> Result<()> {
        if let Some(invoice) = self.pending_invoices.get_mut(quote_id) {
            invoice.state = state;
            Ok(())
        } else {
            Err(Error::custom("Invoice not found"))
        }
    }

    pub fn get_pending_count(&self) -> usize {
        self.pending_invoices
            .values()
            .filter(|invoice| {
                !self.is_expired(invoice.expiry) && invoice.state == InvoiceState::Unpaid
            })
            .count()
    }

    pub fn get_total_pending_amount(&self) -> u64 {
        self.pending_invoices
            .values()
            .filter(|invoice| {
                !self.is_expired(invoice.expiry) && invoice.state == InvoiceState::Unpaid
            })
            .map(|invoice| invoice.amount)
            .sum()
    }
}

impl Default for LightningManager {
    fn default() -> Self {
        Self::new()
    }
}
