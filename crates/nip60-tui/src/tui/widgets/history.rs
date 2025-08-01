use crate::tui::state::{AppState, HistoryFilter};
use crate::tui::widgets::{
    create_normal_style, create_selected_style, create_title_style, format_amount,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

#[derive(Debug)]
struct EventInfo {
    event_id: String,
    mint: String,
    unit: String,
}

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

                let direction_text = match transaction.direction.as_str() {
                    "in" => "RECEIVED",
                    "out" => "SENT",
                    _ => "UNKNOWN",
                };

                let amount_str = if let Ok(amount) = transaction.amount.parse::<u64>() {
                    format_amount(amount)
                } else {
                    transaction.amount.clone()
                };

                let created_date = if let Some(created_at) = transaction.created_at {
                    let datetime = chrono::DateTime::from_timestamp(created_at as i64, 0)
                        .unwrap_or_else(chrono::Utc::now);
                    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                } else {
                    "Unknown date".to_string()
                };

                let event_info = Self::extract_event_info(transaction, state);

                let transaction_lines = [
                    format!(
                        "{}{} | {} {} | {}",
                        direction_symbol, direction_text, amount_str, event_info.unit, created_date
                    ),
                    format!(
                        "  Event: {} | Mint: {} | Unit: {}",
                        event_info.event_id, event_info.mint, event_info.unit
                    ),
                ];

                ListItem::new(transaction_lines.join("\n")).style(style)
            })
            .collect();

        let history_list = List::new(history_items).block(
            Block::default()
                .title("Transaction History")
                .borders(Borders::ALL),
        );

        f.render_widget(history_list, area);
    }

    fn extract_event_info(
        transaction: &nip60::nip60::SpendingHistory,
        state: &AppState,
    ) -> EventInfo {
        let mut event_id = "N/A".to_string();
        let mut mint = "Unknown".to_string();
        let mut unit = "sat".to_string();

        if !transaction.events.is_empty() {
            if let Some((_, first_event_id, _, _)) = transaction.events.first() {
                event_id = if first_event_id.len() > 16 {
                    format!("{}...", &first_event_id[..16])
                } else {
                    first_event_id.clone()
                };
            }
        }

        println!("{:?}", transaction);

        if let Some(active_wallet) = state.get_active_wallet() {
            if let Some(wallet_state) = &active_wallet.state {
                if let Some(nip60_wallet) = &active_wallet.wallet {
                    if !wallet_state.proofs.is_empty() {
                        let p = nip60_wallet.get_proof_breakdown(&wallet_state.proofs);
                        let b = p.first().unwrap();
                        mint = b.mint_url.clone();
                        unit = b.unit.clone().unwrap_or("sat".to_string());

                        // if let Some(first_proof) = wallet_state.proofs.first() {
                        //     let keyset_id = &first_proof.keyset_id.to_string();
                        //     let mint_info =
                        //         nip60_wallet.get_mint_info_by_keyset_id(&keyset_id).unwrap();

                        //     mint = mint_info.url.clone();
                        //     unit = mint_info.keysets.first().unwrap().unit.clone();
                        // }
                    }
                }
            }
        }

        EventInfo {
            event_id,
            mint,
            unit,
        }
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = "j/k: Navigate • a/i/o: Filter All/In/Out • r: Refresh • q/ESC: Back";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, area);
    }
}
