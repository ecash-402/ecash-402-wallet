use crate::tui::state::AppState;
use crate::tui::widgets::{
    create_normal_style, create_selected_style, create_title_style, format_timestamp,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct MainWidget;

impl MainWidget {
    pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(5), // General wallet info
                Constraint::Length(5), // Selected mint balance
                Constraint::Min(8),    // Main content (mint list)
                Constraint::Length(8), // Navigation
                Constraint::Length(3), // Help
            ])
            .split(area);

        Self::render_header(f, state, chunks[0]);
        Self::render_general_wallet_info(f, state, chunks[1]);
        Self::render_selected_mint_info(f, state, chunks[2]);
        Self::render_mint_list(f, state, chunks[3]);
        Self::render_navigation(f, chunks[4]);
        Self::render_help(f, chunks[5]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = if let Some(wallet) = state.get_active_wallet() {
            format!(
                "NIP-60 Wallet - {} [{}]",
                wallet.config.name,
                if state.loading { "Loading..." } else { "Ready" }
            )
        } else {
            "NIP-60 Wallet - No wallet selected".to_string()
        };

        let header = Paragraph::new(title)
            .style(create_title_style())
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(header, area);
    }

    fn render_general_wallet_info(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(wallet) = state.get_active_wallet() {
            let last_update = format_timestamp(
                wallet
                    .last_update
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );

            let info_items = vec![
                format!("Wallet: {}", wallet.config.name),
                format!("Total Mints: {}", wallet.config.mints.len()),
                format!("Relays: {}", wallet.config.relays.len()),
                format!("Last update: {}", last_update),
            ];

            let info_list: Vec<ListItem> = info_items
                .into_iter()
                .map(|item| ListItem::new(item).style(create_normal_style()))
                .collect();

            let info_widget = List::new(info_list).block(
                Block::default()
                    .title("Wallet Information")
                    .borders(Borders::ALL),
            );

            f.render_widget(info_widget, area);
        } else {
            let no_wallet_text =
                Paragraph::new("No wallet selected\n\nPress 'w' to manage wallets")
                    .style(create_normal_style())
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .title("Wallet Status")
                            .borders(Borders::ALL),
                    );

            f.render_widget(no_wallet_text, area);
        }
    }

    fn render_selected_mint_info(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(wallet) = state.get_active_wallet() {
            let (balance, unit) = state.get_display_balance_info();
            let selected_mint_url = state
                .get_selected_mint_url()
                .unwrap_or_else(|| "No mint selected".to_string());

            let mut info_items = vec![
                format!("Selected Mint: {}", selected_mint_url),
                format!("Balance: {} {}", balance, unit),
            ];

            if let Some(error) = &wallet.error {
                info_items.push(format!("Error: {}", error));
            }

            let info_list: Vec<ListItem> = info_items
                .into_iter()
                .map(|item| ListItem::new(item).style(create_normal_style()))
                .collect();

            let info_widget = List::new(info_list).block(
                Block::default()
                    .title("Selected Mint")
                    .borders(Borders::ALL),
            );

            f.render_widget(info_widget, area);
        } else {
            let no_mint_text = Paragraph::new("No mint selected")
                .style(create_normal_style())
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title("Selected Mint")
                        .borders(Borders::ALL),
                );

            f.render_widget(no_mint_text, area);
        }
    }

    fn render_mint_list(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(wallet) = state.get_active_wallet() {
            let mint_items: Vec<ListItem> = wallet
                .config
                .mints
                .iter()
                .enumerate()
                .map(|(i, mint)| {
                    let style = if i == state.selected_mint_index {
                        create_selected_style()
                    } else {
                        create_normal_style()
                    };
                    ListItem::new(format!("• {}", mint)).style(style)
                })
                .collect();

            let mint_list = List::new(mint_items).block(
                Block::default()
                    .title("Configured Mints (j/k to navigate)")
                    .borders(Borders::ALL),
            );

            f.render_widget(mint_list, area);
        } else {
            let no_mints_text = Paragraph::new("No mints configured")
                .style(create_normal_style())
                .alignment(Alignment::Center)
                .block(Block::default().title("Mints").borders(Borders::ALL));

            f.render_widget(no_mints_text, area);
        }
    }

    fn render_navigation(f: &mut Frame, area: Rect) {
        let nav_items = ["h: History",
            "s: Send",
            "r: Redeem",
            "n: Lightning",
            "w: Wallets",
            "R: Refresh",
            "Tab: Switch Wallet"];

        let nav_text = nav_items.join(" | ");
        let nav_paragraph = Paragraph::new(nav_text)
            .style(create_normal_style())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().title("Navigation").borders(Borders::ALL));

        f.render_widget(nav_paragraph, area);
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "Press q or Ctrl+Q to quit • Use j/k to navigate mints, h/l for vim-style navigation • ESC to go back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
