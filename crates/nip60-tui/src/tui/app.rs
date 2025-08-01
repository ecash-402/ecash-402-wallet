use crate::error::Result;
use crate::tui::events::EventHandler;
use crate::tui::state::{ActiveView, AppState};
use crate::tui::widgets::*;
use crossterm::event::KeyEvent;
use ratatui::prelude::*;

pub struct App {
    state: AppState,
}

impl App {
    pub async fn new() -> Result<Self> {
        let mut state = AppState::new().await?;

        if let Some(active_wallet_name) = state.config.active_wallet.clone() {
            if let Err(e) = state.load_wallet(&active_wallet_name).await {
                state.set_error(format!("Failed to load active wallet: {}", e));
            }
        }

        Ok(Self { state })
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let main_area = f.area();

        self.render_current_view(f, main_area);

        if let Some(ref error) = self.state.error_message {
            render_error_popup(f, error, main_area);
        }
    }

    fn render_current_view(&mut self, f: &mut Frame, area: Rect) {
        match self.state.active_view {
            ActiveView::Main => {
                MainWidget::render(f, &self.state, area);
            }
            ActiveView::History => {
                HistoryWidget::render(f, &self.state, area);
            }
            ActiveView::Send => {
                SendWidget::render(f, &self.state, area);
            }
            ActiveView::Redeem => {
                RedeemWidget::render(f, &self.state, area);
            }
            ActiveView::Lightning => {
                LightningWidget::render(f, &self.state, area);
            }
            ActiveView::WalletManager => {
                WalletManagerWidget::render(f, &self.state, area);
            }
            ActiveView::AddWallet => {
                AddWalletWidget::render(f, &self.state, area);
            }
        }
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        EventHandler::handle_key_event(&mut self.state, key).await?;
        Ok(())
    }

    pub async fn initialize_wallets(&mut self) -> Result<()> {
        for wallet_config in &self.state.config.wallets.clone() {
            if let Err(e) = self.state.load_wallet(&wallet_config.name).await {
                self.state.set_error(format!(
                    "Failed to load wallet {}: {}",
                    wallet_config.name, e
                ));
            }
        }
        Ok(())
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        self.state.loading = true;

        if let Err(e) = self.state.refresh_all_wallets().await {
            self.state
                .set_error(format!("Failed to refresh wallets: {}", e));
        }

        self.state.loading = false;
        Ok(())
    }

    pub fn is_loading(&self) -> bool {
        self.state.loading
    }

    pub fn get_active_wallet_name(&self) -> Option<String> {
        self.state.config.active_wallet.clone()
    }

    pub fn get_wallet_count(&self) -> usize {
        self.state.config.wallets.len()
    }

    pub fn get_current_view(&self) -> ActiveView {
        self.state.active_view
    }

    pub fn has_error(&self) -> bool {
        self.state.error_message.is_some()
    }
}
