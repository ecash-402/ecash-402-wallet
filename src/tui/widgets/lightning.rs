use crate::tui::state::AppState;
use crate::tui::widgets::{
    create_error_style, create_normal_style, create_success_style, create_title_style,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct LightningWidget;

impl LightningWidget {
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
        Self::render_invoice_output(f, state, chunks[2]);
        Self::render_help(f, chunks[3]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = if let Some(wallet) = state.get_active_wallet() {
            format!(
                "Lightning Invoice - {} (Balance: {} sats)",
                wallet.config.name, wallet.balance
            )
        } else {
            "Lightning Invoice - No wallet selected".to_string()
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

        let amount_input =
            Paragraph::new(format!("Amount: {}", state.lightning_state.amount_input))
                .style(create_normal_style())
                .block(
                    Block::default()
                        .title("Amount (sats)")
                        .borders(Borders::ALL),
                );

        let description_input = Paragraph::new(format!(
            "Description: {}",
            state.lightning_state.description_input
        ))
        .style(create_normal_style())
        .block(Block::default().title("Description").borders(Borders::ALL));

        f.render_widget(amount_input, input_chunks[0]);
        f.render_widget(description_input, input_chunks[1]);

        let instructions = vec![
            "1. Enter amount in sats",
            "2. Add description",
            "3. Press Tab to generate",
            "4. QR code will be shown",
        ];

        let instructions_list: Vec<ListItem> = instructions
            .into_iter()
            .map(|item| ListItem::new(item).style(create_normal_style()))
            .collect();

        let instructions_widget = List::new(instructions_list)
            .block(Block::default().title("Instructions").borders(Borders::ALL));

        f.render_widget(instructions_widget, chunks[1]);
    }

    fn render_invoice_output(f: &mut Frame, state: &AppState, area: Rect) {
        if let Some(ref error) = state.lightning_state.error {
            let error_text = Paragraph::new(error.clone())
                .style(create_error_style())
                .wrap(Wrap { trim: true })
                .block(Block::default().title("Error").borders(Borders::ALL));

            f.render_widget(error_text, area);
        } else if let Some(ref invoice) = state.lightning_state.invoice {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3)])
                .split(area);

            let status_text = if invoice.contains("not implemented") {
                Paragraph::new("Lightning functionality is not yet implemented")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center)
                    .block(Block::default().title("Status").borders(Borders::ALL))
            } else {
                Paragraph::new("Lightning invoice generated successfully!")
                    .style(create_success_style())
                    .alignment(Alignment::Center)
                    .block(Block::default().title("Success").borders(Borders::ALL))
            };

            f.render_widget(status_text, chunks[0]);

            let invoice_display = Self::format_invoice_display(invoice);
            let invoice_text = Paragraph::new(invoice_display)
                .style(create_normal_style())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title("Lightning Invoice")
                        .borders(Borders::ALL),
                );

            f.render_widget(invoice_text, chunks[1]);
        } else {
            let waiting_text =
                Paragraph::new("Enter amount and description, then press Tab to generate invoice")
                    .style(create_normal_style())
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .title("Lightning Invoice")
                            .borders(Borders::ALL),
                    );

            f.render_widget(waiting_text, area);
        }
    }

    fn format_invoice_display(invoice: &str) -> String {
        if invoice.len() > 80 {
            let start = &invoice[..40];
            let end = &invoice[invoice.len() - 40..];
            format!("{}...{}", start, end)
        } else {
            invoice.to_string()
        }
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "Type digits for amount • Tab: Generate invoice • q/ESC: Back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
