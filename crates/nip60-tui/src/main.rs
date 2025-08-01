use nip60_tui::tui;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting NIP-60 Wallet TUI...");

    match tui::run().await {
        Ok(()) => {
            println!("TUI exited normally");
        }
        Err(e) => {
            eprintln!("TUI error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
