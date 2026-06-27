// ─────────────────────────────────────────────────────────────────────────────
// screens/intro.rs — la pantalla de splash/intro antes de que arranque todo
// ─────────────────────────────────────────────────────────────────────────────
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

// Helper to center a rectangular block on the terminal frame.
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

// The QUESTLINE ASCII art logo — each line is a string.
const LOGO: &[&str] = &[
    r" ██████╗ ██╗   ██╗███████╗███████╗████████╗██╗     ██╗███╗   ██╗███████╗",
    r"██╔═══██╗██║   ██║██╔════╝██╔════╝╚══██╔══╝██║     ██║████╗  ██║██╔════╝",
    r"██║   ██║██║   ██║█████╗  ███████╗   ██║   ██║     ██║██╔██╗ ██║█████╗  ",
    r"██║▄▄ ██║██║   ██║██╔══╝  ╚════██║   ██║   ██║     ██║██║╚██╗██║██╔══╝  ",
    r"╚██████╔╝╚██████╔╝███████╗███████║   ██║   ███████╗██║██║ ╚████║███████╗",
    r" ╚══▀▀═╝  ╚═════╝ ╚══════╝╚══════╝   ╚═╝   ╚══════╝╚═╝╚═╝  ╚═══╝╚══════╝",
];

// The six class accent colours — the wave cycles through them in order:
// CodeWarlock (purple) → TaskPaladin (pink) → MindSage (cyan) →
// SystemsArchitect (blue) → TimeChronomancer (orange) → ArchAccountant (gold)
const PALETTE: &[Color] = &[
    Color::Rgb(168,  85, 247), // CodeWarlock     — Purple
    Color::Rgb(255, 105, 180), // TaskPaladin     — Pink
    Color::Rgb(  6, 182, 212), // MindSage        — Cyan
    Color::Rgb( 59, 130, 246), // SystemsArchitect — Blue
    Color::Rgb(249, 115,  22), // TimeChronomancer — Orange
    Color::Rgb(245, 158,  11), // ArchAccountant   — Gold
];


// Renders the skippable splash screen with animated ASCII logo and a Questline quote.
// `ticks` comes from `app.intro_ticks` and advances at 50 ms per tick.
pub fn draw(
    f: &mut Frame,
    quote: &str,
    author: &str,
    ticks: usize,
    theme: &Theme,
) {
    let size = f.size();

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(theme.background));
    f.render_widget(bg_block, size);

    // The logo and the border change colour together every 8 ticks (~400 ms).
    let color_idx = (ticks / 8) % PALETTE.len();
    let current_color = PALETTE[color_idx];

    // ── Layout ────────────────────────────────────────────────────────────────
    // Split vertically: logo block (top) + quote block (bottom-ish)
    let logo_height = LOGO.len() as u16 + 2; // +2 for top/bottom border
    let quote_height = 9u16;                  // border + lines
    let total_needed = logo_height + 1 + quote_height;

    // Center the whole composition vertically
    let v_margin = size.height.saturating_sub(total_needed) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_margin),          // top padding
            Constraint::Length(logo_height),        // logo
            Constraint::Length(1),                  // gap
            Constraint::Length(quote_height),       // quote box
            Constraint::Min(0),                     // bottom padding
        ])
        .split(size);

    // ── ASCII Logo ────────────────────────────────────────────────────────────
    let logo_lines: Vec<Line<'static>> = LOGO
        .iter()
        .map(|row| {
            let spans: Vec<Span<'static>> = row
                .chars()
                .map(|ch| {
                    let style = if ch == ' ' || ch == '\t' {
                        Style::default()
                    } else {
                        Style::default().fg(current_color).add_modifier(Modifier::BOLD)
                    };
                    Span::styled(ch.to_string(), style)
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let logo_para = Paragraph::new(logo_lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(logo_para, chunks[1]);

    // ── Quote box ─────────────────────────────────────────────────────────────
    // Horizontally center the quote box (65 % width)
    let quote_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(17),
            Constraint::Percentage(66),
            Constraint::Percentage(17),
        ])
        .split(chunks[3])[1];

    let quote_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("\"{}\"", quote),
            Style::default()
                .fg(theme.text)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(Span::styled(
            format!("— {}", author),
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press [ANY KEY] to begin your quest...",
            Style::default().fg(theme.muted),
        )),
        Line::from(Span::styled(
            "Found a bug? Go to About [?] and press [R] to report it.",
            Style::default().fg(Color::Rgb(249, 115, 22)),
        )),
    ];

    let quote_para = Paragraph::new(quote_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(current_color))
                .title(Span::styled(
                    concat!(" [ v", env!("CARGO_PKG_VERSION"), " ] "),
                    Style::default()
                        .fg(Color::Rgb(249, 115, 22))
                        .add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Right),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(quote_para, quote_area);
}
