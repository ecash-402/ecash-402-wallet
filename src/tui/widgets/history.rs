use crate::tui::state::{AppState, HistoryFilter};
use crate::tui::widgets::{
    create_normal_style, create_selected_style, create_title_style, format_amount, format_timestamp,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub struct HistoryWidget;

impl HistoryWidget {
    pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);

        Self::render_header(f, state, chunks[0]);
        Self::render_filter_bar(f, state, chunks[1]);
        Self::render_history_list(f, state, chunks[2]);
        Self::render_help(f, chunks[3]);
    }

    fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
        let title = if let Some(wallet) = state.get_active_wallet() {
            format!("Transaction History - {}", wallet.config.name)
        } else {
            "Transaction History - No wallet selected".to_string()
        };

        let header = Paragraph::new(title)
            .style(create_title_style())
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(header, area);
    }

    fn render_filter_bar(f: &mut Frame, state: &AppState, area: Rect) {
        let filter_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area);

        let all_style = match state.history_filter {
            HistoryFilter::All => create_selected_style(),
            _ => create_normal_style(),
        };

        let received_style = match state.history_filter {
            HistoryFilter::Received => create_selected_style(),
            _ => create_normal_style(),
        };

        let sent_style = match state.history_filter {
            HistoryFilter::Sent => create_selected_style(),
            _ => create_normal_style(),
        };

        let all_filter = Paragraph::new("All (a)")
            .style(all_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        let received_filter = Paragraph::new("Received (i)")
            .style(received_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        let sent_filter = Paragraph::new("Sent (o)")
            .style(sent_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(all_filter, filter_chunks[0]);
        f.render_widget(received_filter, filter_chunks[1]);
        f.render_widget(sent_filter, filter_chunks[2]);
    }

    fn render_history_list(f: &mut Frame, state: &AppState, area: Rect) {
        let history = state.get_filtered_history();

        if history.is_empty() {
            let no_history_text = Paragraph::new("No transaction history available")
                .style(create_normal_style())
                .alignment(Alignment::Center)
                .block(Block::default().title("Transactions").borders(Borders::ALL));

            f.render_widget(no_history_text, area);
            return;
        }

        let history_items: Vec<ListItem> = history
            .iter()
            .enumerate()
            .map(|(i, transaction)| {
                let style = if i == state.selected_history_index {
                    create_selected_style()
                } else {
                    create_normal_style()
                };

                let direction_symbol = match transaction.direction.as_str() {
                    "in" => "← ",
                    "out" => "→ ",
                    _ => "• ",
                };

                let amount_str = if let Ok(amount) = transaction.amount.parse::<u64>() {
                    format_amount(amount)
                } else {
                    transaction.amount.clone()
                };

                let timestamp_str = if let Some(created_at) = transaction.created_at {
                    format_timestamp(created_at)
                } else {
                    "Unknown".to_string()
                };

                let transaction_text = format!(
                    "{}{} sats - {}",
                    direction_symbol, amount_str, timestamp_str
                );

                ListItem::new(transaction_text).style(style)
            })
            .collect();

        let history_list = List::new(history_items)
            .block(Block::default().title("Transactions").borders(Borders::ALL));

        f.render_widget(history_list, area);
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "j/k: Navigate • a/i/o: Filter All/In/Out • r: Refresh • q/ESC: Back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
