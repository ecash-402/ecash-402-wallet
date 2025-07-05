use crate::tui::state::AppState;
use crate::tui::widgets::{
    create_error_style, create_normal_style, create_selected_style, create_title_style,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct AddWalletWidget;

impl AddWalletWidget {
    pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);

        Self::render_header(f, state, chunks[0]);
        Self::render_form(f, state, chunks[1]);
        Self::render_defaults_info(f, state, chunks[2]);
        Self::render_help(f, chunks[3]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = format!(
            "Add New Wallet - Step {} of 4",
            state.add_wallet_state.step + 1
        );
        let header = Paragraph::new(title)
            .style(create_title_style())
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(header, area);
    }

    fn render_form(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        let name_style = if state.add_wallet_state.step == 0 {
            create_selected_style()
        } else {
            create_normal_style()
        };

        let nsec_style = if state.add_wallet_state.step == 1 {
            create_selected_style()
        } else {
            create_normal_style()
        };

        let mints_style = if state.add_wallet_state.step == 2 {
            create_selected_style()
        } else {
            create_normal_style()
        };

        let relays_style = if state.add_wallet_state.step == 3 {
            create_selected_style()
        } else {
            create_normal_style()
        };

        let name_input = Paragraph::new(format!("Name: {}", state.add_wallet_state.name_input))
            .style(name_style)
            .block(
                Block::default()
                    .title("1. Wallet Name")
                    .borders(Borders::ALL),
            );

        let nsec_display = if state.add_wallet_state.nsec_input.is_empty() {
            "nsec: ".to_string()
        } else {
            format!(
                "nsec: {}...",
                &state.add_wallet_state.nsec_input
                    [..state.add_wallet_state.nsec_input.len().min(10)]
            )
        };

        let nsec_input = Paragraph::new(nsec_display).style(nsec_style).block(
            Block::default()
                .title("2. Private Key (nsec)")
                .borders(Borders::ALL),
        );

        let mints_display = if state.add_wallet_state.mints_input.is_empty() {
            "Mints: (leave empty for defaults)".to_string()
        } else {
            format!("Mints: {}", state.add_wallet_state.mints_input)
        };

        let mints_input = Paragraph::new(mints_display)
            .style(mints_style)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title("3. Mints (comma-separated)")
                    .borders(Borders::ALL),
            );

        let relays_display = if state.add_wallet_state.relays_input.is_empty() {
            "Relays: (leave empty for defaults)".to_string()
        } else {
            format!("Relays: {}", state.add_wallet_state.relays_input)
        };

        let relays_input = Paragraph::new(relays_display)
            .style(relays_style)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title("4. Relays (comma-separated)")
                    .borders(Borders::ALL),
            );

        f.render_widget(name_input, chunks[0]);
        f.render_widget(nsec_input, chunks[1]);
        f.render_widget(mints_input, chunks[2]);
        f.render_widget(relays_input, chunks[3]);
    }

    fn render_defaults_info(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(ref error) = state.add_wallet_state.error {
            let error_text = Paragraph::new(error.clone())
                .style(create_error_style())
                .wrap(Wrap { trim: true })
                .block(Block::default().title("Error").borders(Borders::ALL));

            f.render_widget(error_text, area);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let default_mints = vec![
                "• https://ecashmint.otrta.me",
                "• https://mint.minibits.cash/Bitcoin",
            ];

            let default_relays = vec![
                "• wss://relay.primal.net",
                "• wss://relay.damus.io",
                "• wss://nostr.oxtr.dev",
                "• wss://nostr.mom",
            ];

            let mints_list: Vec<ListItem> = default_mints
                .into_iter()
                .map(|item| ListItem::new(item).style(create_normal_style()))
                .collect();

            let relays_list: Vec<ListItem> = default_relays
                .into_iter()
                .map(|item| ListItem::new(item).style(create_normal_style()))
                .collect();

            let mints_widget = List::new(mints_list).block(
                Block::default()
                    .title("Default Mints")
                    .borders(Borders::ALL),
            );

            let relays_widget = List::new(relays_list).block(
                Block::default()
                    .title("Default Relays")
                    .borders(Borders::ALL),
            );

            f.render_widget(mints_widget, chunks[0]);
            f.render_widget(relays_widget, chunks[1]);
        }
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "Tab/Shift+Tab: Navigate • Ctrl+V: Paste • Ctrl+A: Clear field • Enter: Create • q/ESC: Cancel";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
