// ─────────────────────────────────────────────────────────────────────────────
// screens/soundscapes.rs — pantalla del reproductor: soundscapes + Media Player MPRIS
// ─────────────────────────────────────────────────────────────────────────────
use crate::app::App;
use crate::audio::{PlaybackStatus, SOUNDSCAPES};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let accent_color = theme.primary;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let player_state = app.audio_player.get_state();
    let is_playing = player_state.status == PlaybackStatus::Playing;
    let playing_name = player_state.current_soundscape.clone();

    // LEFT PANEL: lista de soundscapes
    let mut list_items = Vec::new();
    for (idx, sc) in SOUNDSCAPES.iter().enumerate() {
        let is_selected = idx == app.selected_soundscape_idx;
        let is_active_playing = is_playing && playing_name == sc.name;

        let marker = if is_selected { " -> " } else { "   " };
        let play_icon = if is_active_playing { " > ".to_string() } else { "   ".to_string() };

        let name_style = if is_selected {
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)
        } else if is_active_playing {
            Style::default().fg(accent_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let row_title = Line::from(vec![
            Span::styled(marker, Style::default().fg(theme.warning)),
            Span::styled(play_icon, Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(sc.name, name_style),
        ]);
        let desc_span = Span::styled(
            format!("      {}", sc.description),
            Style::default().fg(theme.muted),
        );
        let bonus_span = Span::styled(
            format!("      RPG Bonus: {}", sc.bonus),
            Style::default().fg(Color::Rgb(16, 185, 129)),
        );
        list_items.push(ListItem::new(vec![
            row_title,
            Line::from(desc_span),
            Line::from(bonus_span),
            Line::from(""),
        ]));
    }

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Music ",
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, chunks[0]);

    // RIGHT PANEL — cambia según si Media Player está seleccionado o no
    if SOUNDSCAPES[app.selected_soundscape_idx].name == "Media Player" {
        draw_mpris_panel(f, app, theme, chunks[1]);
    } else {
        draw_audio_control_panel(f, app, theme, chunks[1], &player_state, &playing_name);
    }
}

fn draw_audio_control_panel(
    f: &mut Frame,
    _app: &App,
    theme: &Theme,
    area: Rect,
    player_state: &crate::audio::state::AudioState,
    playing_name: &str,
) {
    let accent_color = theme.primary;

    let vol_bar_width = 15;
    let filled_segments = (player_state.volume * vol_bar_width as f32).round() as usize;
    let vol_bar = format!(
        "[{}{}] {}%",
        "█".repeat(filled_segments),
        "░".repeat(vol_bar_width - filled_segments),
        (player_state.volume * 100.0) as i32
    );

    let status_str = match player_state.status {
        PlaybackStatus::Playing => "PLAYING",
        PlaybackStatus::Paused => "PAUSED",
        PlaybackStatus::Stopped => "STOPPED",
    };
    let status_style = match player_state.status {
        PlaybackStatus::Playing => Style::default().fg(theme.success).add_modifier(Modifier::BOLD),
        PlaybackStatus::Paused => Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        PlaybackStatus::Stopped => Style::default().fg(theme.muted),
    };

    let right_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Status:      ", Style::default().fg(theme.muted)),
            Span::styled(status_str, status_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Volume:      ", Style::default().fg(theme.muted)),
            Span::styled(vol_bar, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Atmosphere:  ", Style::default().fg(theme.muted)),
            Span::styled(
                playing_name.to_string(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from("   ──────────────────────────────"),
        Line::from(""),
        Line::from(Span::styled(
            "   [Playback Keyboard Controls]",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("     Enter   ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Play selected atmosphere", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("     p       ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Pause / Resume current audio", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("     s       ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Stop / Mute current playback", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("     n       ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Cycle to next track", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("     + / -   ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Increase / Decrease volume", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("     *       ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Reset volume to 50%", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("     f       ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Change local music folder path", Style::default().fg(theme.text)),
        ]),
    ];

    let right_panel = Paragraph::new(right_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Audio Control Panel ",
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(right_panel, area);
}

fn draw_mpris_panel(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let accent_color = theme.primary;

    let (status_icon, track_line, artist_line, player_line, status_color) =
        match &app.mpris_now_playing {
            Some(np) => {
                let icon = if np.is_playing { "▶" } else { "⏸" };
                let color = if np.is_playing {
                    Color::Rgb(30, 215, 96)
                } else {
                    theme.warning
                };
                let player_display = {
                    let mut p = np.player.clone();
                    if let Some(c) = p.get_mut(0..1) {
                        c.make_ascii_uppercase();
                    }
                    p
                };
                (
                    icon,
                    format!("  {} {}", icon, np.title),
                    format!("     by {}", np.artist),
                    format!("     via {}", player_display),
                    color,
                )
            }
            None => (
                "—",
                "  No media player detected".to_string(),
                "  Open Spotify, VLC, or any".to_string(),
                "  MPRIS-compatible player".to_string(),
                theme.muted,
            ),
        };

    let _ = status_icon; // usado solo para construir track_line

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            track_line,
            Style::default().fg(status_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(artist_line, Style::default().fg(theme.text))),
        Line::from(Span::styled(player_line, Style::default().fg(theme.muted))),
        Line::from(""),
        Line::from(Span::styled(
            "  ──────────────────────────────",
            Style::default().fg(theme.border),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Works with any MPRIS player:",
            Style::default().fg(theme.muted),
        )),
        Line::from(Span::styled(
            "  Spotify, VLC, Rhythmbox, mpv...",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ──────────────────────────────",
            Style::default().fg(theme.border),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter / p  ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Play / Pause", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  n          ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Next track", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  b          ", Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
            Span::styled(" Previous track", Style::default().fg(theme.text)),
        ]),
    ];

    let panel = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Media Player (MPRIS) ",
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(panel, area);
}
