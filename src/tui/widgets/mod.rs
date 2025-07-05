pub mod add_wallet;
pub mod history;
pub mod lightning;
pub mod main;
pub mod redeem;
pub mod send;
pub mod wallet_manager;

pub use add_wallet::AddWalletWidget;
pub use history::HistoryWidget;
pub use lightning::LightningWidget;
pub use main::MainWidget;
pub use redeem::RedeemWidget;
pub use send::SendWidget;
pub use wallet_manager::WalletManagerWidget;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_error_popup(f: &mut Frame, error: &str, area: Rect) {
    let popup_area = centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let error_block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Red));

    let error_text = Paragraph::new(error)
        .block(error_block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(error_text, popup_area);
}

pub fn render_help_bar(f: &mut Frame, help_text: &str, area: Rect) {
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn create_title_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn create_selected_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

pub fn create_normal_style() -> Style {
    Style::default().fg(Color::White)
}

pub fn create_error_style() -> Style {
    Style::default().fg(Color::Red)
}

pub fn create_success_style() -> Style {
    Style::default().fg(Color::Green)
}

pub fn format_amount(amount: u64) -> String {
    if amount >= 1_000_000 {
        format!("{:.2}M", amount as f64 / 1_000_000.0)
    } else if amount >= 1_000 {
        format!("{:.1}K", amount as f64 / 1_000.0)
    } else {
        format!("{}", amount)
    }
}

pub fn format_timestamp(timestamp: u64) -> String {
    if timestamp == 0 {
        "Unknown".to_string()
    } else {
        let datetime =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(timestamp);
        let duration = datetime
            .duration_since(std::time::SystemTime::now())
            .unwrap_or_else(|_| {
                std::time::SystemTime::now()
                    .duration_since(datetime)
                    .unwrap()
            });

        let seconds = duration.as_secs();
        if seconds < 60 {
            format!("{}s ago", seconds)
        } else if seconds < 3600 {
            format!("{}m ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{}h ago", seconds / 3600)
        } else {
            format!("{}d ago", seconds / 86400)
        }
    }
}
