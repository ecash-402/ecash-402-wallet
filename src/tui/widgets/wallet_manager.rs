use crate::tui::state::AppState;
use crate::tui::widgets::{
    create_normal_style, create_selected_style, create_title_style, format_amount,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct WalletManagerWidget;

impl WalletManagerWidget {
    pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(6),
                Constraint::Length(3),
            ])
            .split(area);

        Self::render_header(f, state, chunks[0]);
        Self::render_wallet_list(f, state, chunks[1]);
        Self::render_actions(f, chunks[2]);
        Self::render_help(f, chunks[3]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = format!(
            "Wallet Manager - {} wallets configured",
            state.config.wallets.len()
        );
        let header = Paragraph::new(title)
            .style(create_title_style())
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(header, area);
    }

    fn render_wallet_list(f: &mut Frame, state: &AppState, area: Rect) {
        if state.config.wallets.is_empty() {
            let no_wallets_text =
                Paragraph::new("No wallets configured\n\nPress 'a' to add a new wallet")
                    .style(create_normal_style())
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(Block::default().title("Wallets").borders(Borders::ALL));

            f.render_widget(no_wallets_text, area);
            return;
        }

        let wallet_items: Vec<ListItem> = state
            .config
            .wallets
            .iter()
            .enumerate()
            .map(|(i, wallet_config)| {
                let style = if i == state.selected_wallet_index {
                    create_selected_style()
                } else {
                    create_normal_style()
                };

                let is_active = state.config.active_wallet.as_ref() == Some(&wallet_config.name);
                let active_marker = if is_active { "● " } else { "○ " };

                let balance = state
                    .wallets
                    .get(&wallet_config.name)
                    .map(|w| format_amount(w.balance))
                    .unwrap_or_else(|| "Unknown".to_string());

                let status = if let Some(wallet_instance) = state.wallets.get(&wallet_config.name) {
                    if wallet_instance.wallet.is_some() {
                        "Ready"
                    } else if wallet_instance.error.is_some() {
                        "Error"
                    } else {
                        "Not loaded"
                    }
                } else {
                    "Not loaded"
                };

                let wallet_text = format!(
                    "{}{} | {} sats | {} mints | {}",
                    active_marker,
                    wallet_config.name,
                    balance,
                    wallet_config.mints.len(),
                    status
                );

                ListItem::new(wallet_text).style(style)
            })
            .collect();

        let wallet_list = List::new(wallet_items).block(
            Block::default()
                .title("Configured Wallets")
                .borders(Borders::ALL),
        );

        f.render_widget(wallet_list, area);
    }

    fn render_actions(f: &mut Frame, area: Rect) {
        let actions = vec![
            "a: Add new wallet",
            "Enter: Select wallet",
            "d: Delete wallet",
            "j/k: Navigate",
        ];

        let actions_list: Vec<ListItem> = actions
            .into_iter()
            .map(|item| ListItem::new(item).style(create_normal_style()))
            .collect();

        let actions_widget =
            List::new(actions_list).block(Block::default().title("Actions").borders(Borders::ALL));

        f.render_widget(actions_widget, area);
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "j/k: Navigate • a: Add • Enter: Select • d: Delete • q/ESC: Back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
