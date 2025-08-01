use crate::tui::state::AppState;
use crate::tui::widgets::{
    create_error_style, create_normal_style, create_success_style, create_title_style,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct RedeemWidget;

impl RedeemWidget {
    pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);

        Self::render_header(f, state, chunks[0]);
        Self::render_input_form(f, state, chunks[1]);
        Self::render_redeem_result(f, state, chunks[2]);
        Self::render_help(f, chunks[3]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = if let Some(wallet) = state.get_active_wallet() {
            let (balance, unit) = state.get_display_balance_info();
            format!(
                "Redeem Token - {} (Balance: {} {})",
                wallet.config.name, balance, unit
            )
        } else {
            "Redeem Token - No wallet selected".to_string()
        };

        let header = Paragraph::new(title)
            .style(create_title_style())
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(header, area);
    }

    fn render_input_form(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let token_input_display = if state.redeem_state.token_input.len() > 60 {
            format!("{}...", &state.redeem_state.token_input[..57])
        } else {
            state.redeem_state.token_input.clone()
        };

        let token_input = Paragraph::new(token_input_display)
            .style(create_normal_style())
            .wrap(Wrap { trim: true })
            .block(Block::default().title("Token Input").borders(Borders::ALL));

        f.render_widget(token_input, chunks[0]);

        let instructions = vec![
            "1. Paste or type token",
            "2. Use Ctrl+V to paste",
            "3. Press Enter to redeem",
            "4. Token will be validated",
        ];

        let instructions_list: Vec<ListItem> = instructions
            .into_iter()
            .map(|item| ListItem::new(item).style(create_normal_style()))
            .collect();

        let instructions_widget = List::new(instructions_list)
            .block(Block::default().title("Instructions").borders(Borders::ALL));

        f.render_widget(instructions_widget, chunks[1]);
    }

    fn render_redeem_result(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(ref error) = state.redeem_state.error {
            let error_text = Paragraph::new(error.clone())
                .style(create_error_style())
                .wrap(Wrap { trim: true })
                .block(Block::default().title("Error").borders(Borders::ALL));

            f.render_widget(error_text, area);
        } else if let Some(ref result) = state.redeem_state.result {
            let result_text = Paragraph::new(result.clone())
                .style(create_success_style())
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title("Redemption Result")
                        .borders(Borders::ALL),
                );

            f.render_widget(result_text, area);
        } else {
            let waiting_text = Paragraph::new("Paste token and press Enter to redeem")
                .style(create_normal_style())
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title("Redemption Status")
                        .borders(Borders::ALL),
                );

            f.render_widget(waiting_text, area);
        }
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "Type/paste token • Ctrl+V: Paste • Enter: Redeem • q/ESC: Back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
