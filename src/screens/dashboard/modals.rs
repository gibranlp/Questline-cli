// ─────────────────────────────────────────────────────────────────────────────
// dashboard/modals.rs — floating modals drawn on top of whichever dashboard
// layout is active. Lifted verbatim out of the old single-layout draw() tail:
// it only ever depended on `app`/`theme`/the outer `area`, never on any
// layout-local Rect from a panel split, so it renders identically regardless
// of `app.dashboard_layout`.
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{App, ModalType};
use crate::screens::intro::centered_rect;
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

pub(super) fn draw_modals(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    match &app.modal_state {
        ModalType::DailyReflection {
            what_went_well,
            what_can_improve,
            focus_idx,
        } => {
            let modal_area = centered_rect(55, 45, area);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(theme.background)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    " Daily Reflection Journal ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Min(2),
                ])
                .split(block.inner(modal_area));
            f.render_widget(block, modal_area);

            let border_well = if *focus_idx == 0 {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.muted)
            };
            let border_improve = if *focus_idx == 1 {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.muted)
            };

            f.render_widget(
                Paragraph::new(format!(" > {}", what_went_well)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_well)
                        .title(" 1. What went well today? "),
                ),
                content[1],
            );
            f.render_widget(
                Paragraph::new(format!(" > {}", what_can_improve)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_improve)
                        .title(" 2. What can be improved? "),
                ),
                content[2],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] switch  |  [Enter] submit  |  [Esc] cancel ",
                    Style::default().fg(theme.muted),
                ))
                .alignment(Alignment::Center),
                content[3],
            );
        }
        ModalType::NewRitual {
            name,
            desc,
            frequency_idx,
            reward_xp,
            focus_idx,
        } => {
            let modal_area = centered_rect(55, 55, area);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(theme.background)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    " New Sidequest (Habit) ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(2),
                ])
                .split(block.inner(modal_area));
            f.render_widget(block, modal_area);

            let border = |idx: usize| {
                if *focus_idx == idx {
                    Style::default().fg(theme.primary)
                } else {
                    Style::default().fg(theme.muted)
                }
            };
            let freqs = [
                "Daily", "2x Daily", "3x Daily", "5x Daily", "Weekdays", "Weekly", "Monthly",
            ];
            let freq_str = format!("<  {}  >", freqs[*frequency_idx]);

            f.render_widget(
                Paragraph::new(format!(" > {}", name)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(0))
                        .title(" 1. Name "),
                ),
                content[1],
            );
            f.render_widget(
                Paragraph::new(format!(" > {}", desc)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(1))
                        .title(" 2. Description (optional) "),
                ),
                content[2],
            );
            f.render_widget(
                Paragraph::new(freq_str).alignment(Alignment::Center).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(2))
                        .title(" 3. Frequency "),
                ),
                content[3],
            );
            f.render_widget(
                Paragraph::new(format!(" > {}", reward_xp)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(3))
                        .title(" 4. XP Reward "),
                ),
                content[4],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] switch  |  [<->] frequency  |  [Enter] create  |  [Esc] cancel ",
                    Style::default().fg(theme.muted),
                ))
                .alignment(Alignment::Center),
                content[5],
            );
        }
        // HydrationReminder ya no se dibuja aquí — main.rs lo pinta como overlay global
        // (independiente de active_screen) porque el recordatorio debe interrumpir
        // cualquier pantalla, no solo el Dashboard. Tenerlo duplicado aquí hacía que,
        // estando en el Dashboard, se dibujaran dos cajas "Hydration Reminder" a la vez.
        ModalType::HydrationSettings {
            interval_idx,
            from_hour,
            to_hour,
            target,
            pause_focus,
            focus_idx,
        } => {
            let modal_area = centered_rect(52, 55, area);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(theme.background)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.secondary))
                .title(Span::styled(
                    " Hydration Settings ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3), // interval
                    Constraint::Length(3), // from
                    Constraint::Length(3), // to
                    Constraint::Length(3), // target
                    Constraint::Length(3), // pause focus
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(block.inner(modal_area));
            f.render_widget(block, modal_area);

            let border = |idx: usize| {
                if *focus_idx == idx {
                    Style::default().fg(theme.primary)
                } else {
                    Style::default().fg(theme.muted)
                }
            };

            let intervals = [30i32, 45, 60, 90, 120];
            let interval_str = format!("<  {} min  >", intervals[*interval_idx]);
            f.render_widget(
                Paragraph::new(interval_str)
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(0))
                            .title(" 1. Reminder Interval "),
                    ),
                content[1],
            );
            f.render_widget(
                Paragraph::new(format!("<  {:02}:00  >", from_hour))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(1))
                            .title(" 2. Active From (hour) "),
                    ),
                content[2],
            );
            f.render_widget(
                Paragraph::new(format!("<  {:02}:00  >", to_hour))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(2))
                            .title(" 3. Active To (hour) "),
                    ),
                content[3],
            );
            f.render_widget(
                Paragraph::new(format!("<  {}  glasses  >", target))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(3))
                            .title(" 4. Daily Target "),
                    ),
                content[4],
            );
            let pause_str = if *pause_focus {
                "[x] Pause during focus sessions"
            } else {
                "[ ] Pause during focus sessions"
            };
            f.render_widget(
                Paragraph::new(pause_str).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(4))
                        .title(" 5. Focus Pause "),
                ),
                content[5],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] switch  |  [<->] adjust  |  [Enter] save  |  [x] Disable  |  [Esc] cancel ",
                    Style::default().fg(theme.muted),
                ))
                .alignment(Alignment::Center),
                content[7],
            );
        }
        _ => {}
    }
}
