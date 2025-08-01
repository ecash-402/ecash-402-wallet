use bip39::Mnemonic;
use cdk::nuts::CurrencyUnit;
use ecash_402_wallet::{
    error::Result,
    lightning::{LightningManager, TopupRequest},
    multimint::MultimintWallet,
    wallet::CashuWalletClient,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Lightning Topup Example");
    println!("==========================\n");

    let seed = Mnemonic::generate(12).unwrap().to_string();
    let home_dir = home::home_dir().unwrap();
    let db_path = home_dir
        .join(".cashu_lightning_test")
        .to_string_lossy()
        .to_string();

    let mut lightning_manager = LightningManager::new();

    println!("📝 Single Wallet Lightning Topup");
    println!("----------------------------------");

    let mint_url = "https://testnut.cashu.space";
    let wallet = CashuWalletClient::from_seed(mint_url, &seed, "lightning_test_wallet").await?;

    println!("✅ Created wallet for mint: {}", mint_url);

    let amount = 100;
    let unit = CurrencyUnit::Msat;

    println!(
        "\n💡 Creating lightning invoice for {} {:?}...",
        amount, unit
    );
    let topup_response = lightning_manager
        .create_invoice_for_wallet(&wallet, mint_url, amount, unit.clone())
        .await?;

    println!("✅ Invoice created:");
    println!("   Quote ID: {}", topup_response.invoice.quote_id);
    println!(
        "   Amount: {} {:?}",
        topup_response.invoice.amount, topup_response.invoice.unit
    );
    println!(
        "   Payment Request: {}...",
        &topup_response.invoice.payment_request[..60]
    );
    println!("   Expiry: {}", topup_response.invoice.expiry);
    println!("   Message: {}", topup_response.message);

    println!("\n🔍 Checking payment status...");
    let status = lightning_manager
        .check_payment_status(&topup_response.invoice.quote_id)
        .await?;

    println!("✅ Payment Status:");
    println!("   Quote ID: {}", status.quote_id);
    println!("   Paid: {}", status.paid);
    println!("   Amount: {} msats", status.amount);
    println!("   Expiry: {}", status.expiry);

    if !status.paid {
        println!("\n⏳ Invoice is not paid yet. In a real scenario, you would:");
        println!("   1. Display the payment request to the user");
        println!("   2. Wait for payment using wait_for_payment()");
        println!("   3. Manually process the mint operation when payment is received");
    }

    println!("\n📝 Multimint Lightning Topup");
    println!("-----------------------------");

    let mut multimint_wallet = MultimintWallet::new(&seed, &db_path).await?;

    multimint_wallet
        .add_mint("https://testnut.cashu.space", Some(CurrencyUnit::Msat))
        .await?;
    multimint_wallet
        .add_mint(
            "https://mint.minibits.cash/Bitcoin",
            Some(CurrencyUnit::Msat),
        )
        .await?;

    println!(
        "✅ Created multimint wallet with {} mints",
        multimint_wallet.list_mints().len()
    );
    for mint in multimint_wallet.list_mints() {
        println!("   - {}", mint);
    }

    let topup_request = TopupRequest {
        amount: 200,
        unit: CurrencyUnit::Msat,
        mint_url: Some("https://testnut.cashu.space".to_string()),
    };

    println!("\n💡 Creating lightning invoice for multimint topup...");
    println!(
        "   Amount: {} {:?}",
        topup_request.amount, topup_request.unit
    );
    println!("   Target Mint: {:?}", topup_request.mint_url);

    let multimint_response = lightning_manager
        .create_invoice_for_multimint(&multimint_wallet, topup_request)
        .await?;

    println!("✅ Multimint invoice created:");
    println!("   Quote ID: {}", multimint_response.invoice.quote_id);
    println!(
        "   Amount: {} {:?}",
        multimint_response.invoice.amount, multimint_response.invoice.unit
    );
    println!("   Mint URL: {}", multimint_response.invoice.mint_url);
    println!(
        "   Payment Request: {}...",
        &multimint_response.invoice.payment_request[..60]
    );
    println!("   Message: {}", multimint_response.message);

    println!("\n📋 Lightning Manager State");
    println!("---------------------------");

    let pending_invoices = lightning_manager.list_pending_invoices();
    println!("📊 Pending invoices: {}", pending_invoices.len());

    for (i, invoice) in pending_invoices.iter().enumerate() {
        println!("   {}. Quote ID: {}", i + 1, invoice.quote_id);
        println!("      Amount: {} {:?}", invoice.amount, invoice.unit);
        println!("      Mint: {}", invoice.mint_url);
        println!("      Expiry: {}", invoice.expiry);
    }

    println!("\n🔍 Testing invoice retrieval...");
    if let Some(invoice) = lightning_manager.get_invoice(&topup_response.invoice.quote_id) {
        println!(
            "✅ Found invoice: {} {:?} at {}",
            invoice.amount, invoice.unit, invoice.mint_url
        );
    }

    println!("\n📱 Testing payment request access...");
    if let Some(payment_request) =
        lightning_manager.get_payment_request(&topup_response.invoice.quote_id)
    {
        println!(
            "✅ Payment request accessible: {}...",
            &payment_request[..50]
        );
    }

    println!("\n⏳ Testing payment waiting (will timeout since not paid)...");
    let paid = lightning_manager
        .wait_for_payment(&multimint_response.invoice.quote_id, 3, 1)
        .await?;
    println!("✅ Payment received within 3 seconds: {}", paid);

    println!("\n🧹 Cleanup expired invoices...");
    lightning_manager.cleanup_expired_invoices();
    println!("✅ Cleanup completed");

    println!("\n🗑️ Testing invoice removal...");
    let removed = lightning_manager.remove_invoice(&topup_response.invoice.quote_id);
    if removed.is_some() {
        println!("✅ Successfully removed invoice");
    }

    let remaining = lightning_manager.list_pending_invoices();
    println!("📊 Remaining invoices: {}", remaining.len());

    println!("\n🎯 Summary");
    println!("----------");
    println!("✅ Successfully demonstrated:");
    println!("   • Lightning invoice creation for single wallet");
    println!("   • Lightning invoice creation for multimint wallet");
    println!("   • Payment status checking");
    println!("   • Invoice management and cleanup");
    println!("   • Payment request access for QR codes");
    println!("   • Payment waiting with timeout");

    println!("\n📚 Integration Notes for UX/Backend:");
    println!("   • Use create_invoice_for_wallet() for single mint topups");
    println!("   • Use create_invoice_for_multimint() for multimint topups");
    println!("   • Display payment_request as QR code for user scanning");
    println!("   • Poll check_payment_status() or use wait_for_payment()");
    println!("   • Once paid=true, use wallet's mint methods to convert to tokens");
    println!("   • Use cleanup_expired_invoices() periodically");

    Ok(())
}
