use crate::error::Result;
use crate::tui::state::{ActiveView, AppState, HistoryFilter};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct EventHandler;

impl EventHandler {
    pub async fn handle_key_event(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match state.active_view {
            ActiveView::Main => Self::handle_main_view(state, key).await,
            ActiveView::History => Self::handle_history_view(state, key).await,
            ActiveView::Send => Self::handle_send_view(state, key).await,
            ActiveView::Redeem => Self::handle_redeem_view(state, key).await,
            ActiveView::Lightning => Self::handle_lightning_view(state, key).await,
            ActiveView::WalletManager => Self::handle_wallet_manager_view(state, key).await,
            ActiveView::AddWallet => Self::handle_add_wallet_view(state, key).await,
        }
    }

    async fn handle_main_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('h') => state.switch_view(ActiveView::History),
            KeyCode::Char('s') => state.switch_view(ActiveView::Send),
            KeyCode::Char('r') => state.switch_view(ActiveView::Redeem),
            KeyCode::Char('l') => state.switch_view(ActiveView::Lightning),
            KeyCode::Char('w') => state.switch_view(ActiveView::WalletManager),
            KeyCode::Char('R') => {
                state.loading = true;
                if let Err(e) = state.refresh_all_wallets().await {
                    state.set_error(format!("Failed to refresh wallets: {}", e));
                }
                state.loading = false;
            }
            KeyCode::Tab => {
                if !state.config.wallets.is_empty() {
                    state.selected_wallet_index =
                        (state.selected_wallet_index + 1) % state.config.wallets.len();
                    if let Some(wallet_config) = state
                        .config
                        .wallets
                        .get(state.selected_wallet_index)
                        .cloned()
                    {
                        state.config.set_active_wallet(&wallet_config.name);
                        if let Err(e) = state.load_wallet(&wallet_config.name).await {
                            state.set_error(format!("Failed to load wallet: {}", e));
                        }
                    }
                }
            }
            KeyCode::Esc => state.clear_error(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_history_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => state.switch_view(ActiveView::Main),
            KeyCode::Char('j') | KeyCode::Down => {
                let history = state.get_filtered_history();
                if !history.is_empty() {
                    state.selected_history_index =
                        (state.selected_history_index + 1) % history.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let history = state.get_filtered_history();
                if !history.is_empty() {
                    state.selected_history_index = if state.selected_history_index == 0 {
                        history.len() - 1
                    } else {
                        state.selected_history_index - 1
                    };
                }
            }
            KeyCode::Char('a') => {
                state.history_filter = HistoryFilter::All;
                state.selected_history_index = 0;
            }
            KeyCode::Char('i') => {
                state.history_filter = HistoryFilter::Received;
                state.selected_history_index = 0;
            }
            KeyCode::Char('o') => {
                state.history_filter = HistoryFilter::Sent;
                state.selected_history_index = 0;
            }
            KeyCode::Char('r') => {
                state.loading = true;
                if let Some(wallet_config) = state.config.get_active_wallet().cloned() {
                    if let Err(e) = state.refresh_wallet_data(&wallet_config.name).await {
                        state.set_error(format!("Failed to refresh wallet: {}", e));
                    }
                }
                state.loading = false;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_send_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                state.switch_view(ActiveView::Main);
                state.send_state = Default::default();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                state.send_state.amount_input.push(c);
            }
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                state.send_state.memo_input.push(c);
            }
            KeyCode::Backspace => {
                if !state.send_state.amount_input.is_empty() {
                    state.send_state.amount_input.pop();
                } else {
                    state.send_state.memo_input.pop();
                }
            }
            KeyCode::Tab => {
                if !state.send_state.amount_input.is_empty() {
                    if let Ok(amount) = state.send_state.amount_input.parse::<u64>() {
                        if let Some(wallet_instance) = state.get_active_wallet() {
                            if let Some(ref wallet) = wallet_instance.wallet {
                                match wallet
                                    .send(
                                        amount,
                                        if state.send_state.memo_input.is_empty() {
                                            None
                                        } else {
                                            Some(state.send_state.memo_input.clone())
                                        },
                                    )
                                    .await
                                {
                                    Ok(token) => {
                                        state.send_state.generated_token = Some(token);
                                        state.send_state.error = None;
                                    }
                                    Err(e) => {
                                        state.send_state.error =
                                            Some(format!("Failed to send: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(ref token) = state.send_state.generated_token {
                    if let Err(e) = arboard::Clipboard::new()
                        .and_then(|mut clipboard| clipboard.set_text(token.clone()))
                    {
                        state.send_state.error =
                            Some(format!("Failed to copy to clipboard: {}", e));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_redeem_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                state.switch_view(ActiveView::Main);
                state.redeem_state = Default::default();
            }
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                state.redeem_state.token_input.push(c);
            }
            KeyCode::Backspace => {
                state.redeem_state.token_input.pop();
            }
            KeyCode::Enter => {
                if !state.redeem_state.token_input.is_empty() {
                    if let Some(wallet_instance) = state.get_active_wallet() {
                        if let Some(ref wallet) = wallet_instance.wallet {
                            match wallet.redeem(&state.redeem_state.token_input).await {
                                Ok(amount) => {
                                    state.redeem_state.result =
                                        Some(format!("Successfully redeemed {} sats", amount));
                                    state.redeem_state.error = None;
                                    state.redeem_state.token_input.clear();
                                }
                                Err(e) => {
                                    state.redeem_state.error =
                                        Some(format!("Failed to redeem: {}", e));
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        state.redeem_state.token_input = text;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_lightning_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                state.switch_view(ActiveView::Main);
                state.lightning_state = Default::default();
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                state.lightning_state.amount_input.push(c);
            }
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                state.lightning_state.description_input.push(c);
            }
            KeyCode::Backspace => {
                if !state.lightning_state.amount_input.is_empty() {
                    state.lightning_state.amount_input.pop();
                } else {
                    state.lightning_state.description_input.pop();
                }
            }
            KeyCode::Tab => {
                state.lightning_state.invoice =
                    Some("Lightning invoice functionality not implemented yet".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_wallet_manager_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => state.switch_view(ActiveView::Main),
            KeyCode::Char('a') => state.switch_view(ActiveView::AddWallet),
            KeyCode::Char('j') | KeyCode::Down => {
                if !state.config.wallets.is_empty() {
                    state.selected_wallet_index =
                        (state.selected_wallet_index + 1) % state.config.wallets.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !state.config.wallets.is_empty() {
                    state.selected_wallet_index = if state.selected_wallet_index == 0 {
                        state.config.wallets.len() - 1
                    } else {
                        state.selected_wallet_index - 1
                    };
                }
            }
            KeyCode::Enter => {
                if let Some(wallet_config) = state.config.wallets.get(state.selected_wallet_index) {
                    let wallet_name = wallet_config.name.clone();
                    state.config.set_active_wallet(&wallet_name);
                    if let Err(e) = state.load_wallet(&wallet_name).await {
                        state.set_error(format!("Failed to load wallet: {}", e));
                    }
                    state.switch_view(ActiveView::Main);
                }
            }
            KeyCode::Char('d') => {
                if let Some(wallet_config) = state.config.wallets.get(state.selected_wallet_index) {
                    let wallet_name = wallet_config.name.clone();
                    state.config.remove_wallet(&wallet_name);
                    state.wallets.remove(&wallet_name);
                    if let Err(e) = state.config.save() {
                        state.set_error(format!("Failed to save config: {}", e));
                    }
                    if state.selected_wallet_index >= state.config.wallets.len()
                        && state.selected_wallet_index > 0
                    {
                        state.selected_wallet_index -= 1;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_add_wallet_view(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                state.switch_view(ActiveView::WalletManager);
                state.add_wallet_state = Default::default();
            }
            KeyCode::Tab => {
                state.add_wallet_state.step = (state.add_wallet_state.step + 1) % 4;
            }
            KeyCode::BackTab => {
                state.add_wallet_state.step = if state.add_wallet_state.step == 0 {
                    3
                } else {
                    state.add_wallet_state.step - 1
                };
            }
            KeyCode::Enter => {
                if !state.add_wallet_state.name_input.is_empty()
                    && !state.add_wallet_state.nsec_input.is_empty()
                {
                    let mut wallet_config = crate::tui::config::WalletConfig::new(
                        state.add_wallet_state.name_input.clone(),
                        state.add_wallet_state.nsec_input.clone(),
                    );

                    if !state.add_wallet_state.mints_input.is_empty() {
                        wallet_config.mints = state
                            .add_wallet_state
                            .mints_input
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }

                    if !state.add_wallet_state.relays_input.is_empty() {
                        wallet_config.relays = state
                            .add_wallet_state
                            .relays_input
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }

                    state.config.add_wallet(wallet_config.clone());
                    state.wallets.insert(
                        wallet_config.name.clone(),
                        crate::tui::state::WalletInstance {
                            config: wallet_config,
                            wallet: None,
                            state: None,
                            balance: 0,
                            history: Vec::new(),
                            last_update: std::time::SystemTime::now(),
                            error: None,
                        },
                    );

                    if let Err(e) = state.config.save() {
                        state.add_wallet_state.error =
                            Some(format!("Failed to save config: {}", e));
                    } else {
                        state.switch_view(ActiveView::WalletManager);
                        state.add_wallet_state = Default::default();
                    }
                }
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        match state.add_wallet_state.step {
                            0 => state.add_wallet_state.name_input.push_str(&text),
                            1 => state.add_wallet_state.nsec_input.push_str(&text),
                            2 => state.add_wallet_state.mints_input.push_str(&text),
                            3 => state.add_wallet_state.relays_input.push_str(&text),
                            _ => {}
                        }
                    }
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                match state.add_wallet_state.step {
                    0 => state.add_wallet_state.name_input.clear(),
                    1 => state.add_wallet_state.nsec_input.clear(),
                    2 => state.add_wallet_state.mints_input.clear(),
                    3 => state.add_wallet_state.relays_input.clear(),
                    _ => {}
                }
            }
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                state.add_wallet_state.error = None;
                match state.add_wallet_state.step {
                    0 => state.add_wallet_state.name_input.push(c),
                    1 => state.add_wallet_state.nsec_input.push(c),
                    2 => state.add_wallet_state.mints_input.push(c),
                    3 => state.add_wallet_state.relays_input.push(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                state.add_wallet_state.error = None;
                match state.add_wallet_state.step {
                    0 => {
                        state.add_wallet_state.name_input.pop();
                    }
                    1 => {
                        state.add_wallet_state.nsec_input.pop();
                    }
                    2 => {
                        state.add_wallet_state.mints_input.pop();
                    }
                    3 => {
                        state.add_wallet_state.relays_input.pop();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
}
