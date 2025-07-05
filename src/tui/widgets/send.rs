use crate::tui::state::AppState;
use crate::tui::widgets::{
    create_error_style, create_normal_style, create_success_style, create_title_style,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct SendWidget;

impl SendWidget {
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
        Self::render_token_output(f, state, chunks[2]);
        Self::render_help(f, chunks[3]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = if let Some(wallet) = state.get_active_wallet() {
            format!(
                "Send Token - {} (Balance: {} sats)",
                wallet.config.name, wallet.balance
            )
        } else {
            "Send Token - No wallet selected".to_string()
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

        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3)])
            .split(chunks[0]);

        let amount_input = Paragraph::new(format!("Amount: {}", state.send_state.amount_input))
            .style(create_normal_style())
            .block(
                Block::default()
                    .title("Amount (sats)")
                    .borders(Borders::ALL),
            );

        let memo_input = Paragraph::new(format!("Memo: {}", state.send_state.memo_input))
            .style(create_normal_style())
            .block(
                Block::default()
                    .title("Memo (optional)")
                    .borders(Borders::ALL),
            );

        f.render_widget(amount_input, input_chunks[0]);
        f.render_widget(memo_input, input_chunks[1]);

        let instructions = vec![
            "1. Enter amount in sats",
            "2. Add optional memo",
            "3. Press Tab to generate token",
            "4. Use Ctrl+C to copy token",
        ];

        let instructions_list: Vec<ListItem> = instructions
            .into_iter()
            .map(|item| ListItem::new(item).style(create_normal_style()))
            .collect();

        let instructions_widget = List::new(instructions_list)
            .block(Block::default().title("Instructions").borders(Borders::ALL));

        f.render_widget(instructions_widget, chunks[1]);
    }

    fn render_token_output(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(ref error) = state.send_state.error {
            let error_text = Paragraph::new(error.clone())
                .style(create_error_style())
                .wrap(Wrap { trim: true })
                .block(Block::default().title("Error").borders(Borders::ALL));

            f.render_widget(error_text, area);
        } else if let Some(ref token) = state.send_state.generated_token {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3)])
                .split(area);

            let success_text = Paragraph::new("Token generated successfully! Use Ctrl+C to copy.")
                .style(create_success_style())
                .alignment(Alignment::Center)
                .block(Block::default().title("Success").borders(Borders::ALL));

            f.render_widget(success_text, chunks[0]);

            let token_display = Self::format_token_display(token);
            let token_text = Paragraph::new(token_display)
                .style(create_normal_style())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title("Generated Token")
                        .borders(Borders::ALL),
                );

            f.render_widget(token_text, chunks[1]);
        } else {
            let waiting_text = Paragraph::new("Enter amount and press Tab to generate token")
                .style(create_normal_style())
                .alignment(Alignment::Center)
                .block(Block::default().title("Token Output").borders(Borders::ALL));

            f.render_widget(waiting_text, area);
        }
    }

    fn format_token_display(token: &str) -> String {
        if token.len() > 80 {
            let start = &token[..40];
            let end = &token[token.len() - 40..];
            format!("{}...{}", start, end)
        } else {
            token.to_string()
        }
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text =
            "Type digits for amount • Tab: Generate token • Ctrl+C: Copy token • q/ESC: Back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
