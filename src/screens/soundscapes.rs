// ─────────────────────────────────────────────────────────────────────────────
// screens/soundscapes.rs — pantalla del reproductor: soundscapes + Media Player MPRIS
// ─────────────────────────────────────────────────────────────────────────────
use crate::app::App;
use crate::audio::{PlaybackStatus, SOUNDSCAPES};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};
use std::path::{Path, PathBuf};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let accent_color = theme.primary;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    let player_state = app.audio_player.get_state();
    let is_playing = player_state.status == PlaybackStatus::Playing;
    let playing_name = player_state.current_soundscape.clone();

    // LEFT PANEL: source selector
    let mut list_items = Vec::new();
    for (idx, sc) in SOUNDSCAPES.iter().enumerate() {
        let is_selected = idx == app.selected_soundscape_idx;
        let is_active_playing = is_playing && playing_name == sc.name;

        let marker = if is_selected { ">" } else { " " };
        let play_icon = if is_active_playing {
            "PLAY"
        } else if playing_name.starts_with("Local:") && sc.name == "Local Folder" {
            "PLAY"
        } else {
            "    "
        };

        let name_style = if is_selected {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
        } else if is_active_playing {
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let row_title = Line::from(vec![
            Span::styled(format!(" {} ", marker), Style::default().fg(theme.warning)),
            Span::styled(
                format!("{:<4} ", play_icon),
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{:<23}", sc.name), name_style),
        ]);
        let desc_span = Span::styled(
            format!("       {}", compact_text(sc.description, 44)),
            Style::default().fg(theme.muted),
        );
        let bonus_span = Span::styled(
            format!("       {}", compact_text(sc.bonus, 44)),
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
                " Sources ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
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
    app: &App,
    theme: &Theme,
    area: Rect,
    player_state: &crate::audio::state::AudioState,
    playing_name: &str,
) {
    let accent_color = theme.primary;
    let selected = SOUNDSCAPES[app.selected_soundscape_idx].name;
    let is_local = selected == "Local Folder";

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Min(8),
            Constraint::Length(7),
        ])
        .split(area);

    let status_str = match player_state.status {
        PlaybackStatus::Playing => "PLAYING",
        PlaybackStatus::Paused => "PAUSED",
        PlaybackStatus::Stopped => "STOPPED",
    };
    let status_style = match player_state.status {
        PlaybackStatus::Playing => Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD),
        PlaybackStatus::Paused => Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD),
        PlaybackStatus::Stopped => Style::default().fg(theme.muted),
    };

    let track_name = if playing_name.starts_with("Local: ") {
        playing_name.trim_start_matches("Local: ").to_string()
    } else {
        playing_name.to_string()
    };
    let vol_bar = volume_bar(player_state.volume, 22);

    let deck_text = vec![
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(status_str, status_style),
            Span::styled("  /  ", Style::default().fg(theme.border)),
            Span::styled(
                selected,
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Now  ", Style::default().fg(theme.muted)),
            Span::styled(
                compact_text(&track_name, area.width.saturating_sub(10) as usize),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Vol  ", Style::default().fg(theme.muted)),
            Span::styled(
                vol_bar,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let deck = Paragraph::new(deck_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Now Playing ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(deck, chunks[0]);

    if is_local {
        draw_local_files_panel(f, app, theme, chunks[1]);
    } else {
        draw_source_detail_panel(f, theme, chunks[1], selected);
    }

    let help_text = vec![
        Line::from(Span::styled(
            "  Enter Play selected    p Pause/Resume    s Stop    Up/Down Sources",
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "  j/k ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Local tracks    ", Style::default().fg(theme.muted)),
            Span::styled(
                "r ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Random    ", Style::default().fg(theme.muted)),
            Span::styled(
                "n ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Next track    ", Style::default().fg(theme.muted)),
            Span::styled(
                "b ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Previous    ", Style::default().fg(theme.muted)),
            Span::styled(
                "+/- ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Volume    ", Style::default().fg(theme.muted)),
            Span::styled(
                "* ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Reset    ", Style::default().fg(theme.muted)),
            Span::styled(
                "f ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Local folder", Style::default().fg(theme.muted)),
        ]),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Controls ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(help, chunks[2]);
}

fn draw_source_detail_panel(f: &mut Frame, theme: &Theme, area: Rect, selected: &str) {
    let (title, lines): (&str, Vec<Line>) = match selected {
        "Rain Sounds" => (
            " Atmosphere Deck ",
            vec![
                Line::from(Span::styled(
                    "  Rain layer loaded from assets/sounds/rain.mp3",
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Looping WAV ambience",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "  Good for focus sessions that should feel steady and grounded.",
                    Style::default().fg(theme.muted),
                )),
            ],
        ),
        "Forest Sounds" => (
            " Atmosphere Deck ",
            vec![
                Line::from(Span::styled(
                    "  Forest layer loaded from assets/sounds/forest.mp3",
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Looping WAV ambience",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "  Use this for quieter sessions with natural texture.",
                    Style::default().fg(theme.muted),
                )),
            ],
        ),
        "Ocean Waves" => (
            " Atmosphere Deck ",
            vec![
                Line::from(Span::styled(
                    "  Ocean layer loaded from assets/sounds/seawash.mp3",
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Looping WAV ambience",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "  Slow wave motion without synthetic white-noise foam.",
                    Style::default().fg(theme.muted),
                )),
            ],
        ),
        _ => (
            " Atmosphere Deck ",
            vec![
                Line::from(Span::styled(
                    format!("  {}", selected),
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Select a source on the left and press Enter to start.",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "  Your active source keeps playing while you browse.",
                    Style::default().fg(theme.muted),
                )),
            ],
        ),
    };

    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(panel, area);
}

fn draw_local_files_panel(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let folder = app
        .db
        .get_setting("local_music_folder")
        .unwrap_or_default()
        .unwrap_or_default();
    let mut tracks = local_audio_files(&folder);
    if tracks.is_empty() {
        tracks = app.local_music_tracks_cache.clone();
    }

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Folder  ", Style::default().fg(theme.muted)),
            Span::styled(
                compact_text(
                    if folder.trim().is_empty() {
                        "Not configured"
                    } else {
                        &folder
                    },
                    area.width.saturating_sub(12) as usize,
                ),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(""),
    ];

    if folder.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            "  Press f to choose a folder with mp3, ogg, wav, or flac files.",
            Style::default().fg(theme.muted),
        )));
    } else if tracks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No supported audio files found here.",
            Style::default().fg(theme.warning),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Queue   ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} supported tracks", tracks.len()),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        let random_selected = app.selected_local_track_idx == 0;
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                if random_selected { "> " } else { "  " },
                Style::default().fg(theme.primary),
            ),
            Span::styled(
                "Random shuffle",
                if random_selected {
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                },
            ),
        ]));
        for (idx, path) in tracks
            .iter()
            .take(area.height.saturating_sub(8) as usize)
            .enumerate()
        {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown track");
            let selected = app.selected_local_track_idx == idx + 1;
            let marker = if selected { ">" } else { " " };
            let style = if selected {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} {:02}  ", marker, idx + 1),
                    Style::default().fg(theme.primary),
                ),
                Span::styled(
                    compact_text(name, area.width.saturating_sub(11) as usize),
                    style,
                ),
            ]));
        }
        if tracks.len() > area.height.saturating_sub(8) as usize {
            lines.push(Line::from(Span::styled(
                format!(
                    "      ...and {} more",
                    tracks.len() - area.height.saturating_sub(8) as usize
                ),
                Style::default().fg(theme.muted),
            )));
        }
    }

    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                " Local Library ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(panel, area);
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
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
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
            Span::styled(
                "  Enter / p  ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Play / Pause", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(
                "  n          ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Next track", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(
                "  b          ",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
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
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(panel, area);
}

fn local_audio_files(folder: &str) -> Vec<PathBuf> {
    let folder = expand_home(folder.trim());
    if folder.is_empty() {
        return Vec::new();
    }

    let dir = Path::new(&folder);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut tracks: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_supported_audio(path))
        .collect();
    tracks.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        a_name.to_lowercase().cmp(&b_name.to_lowercase())
    });
    tracks
}

fn is_supported_audio(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .as_deref(),
        Some("mp3" | "ogg" | "wav" | "flac")
    )
}

fn expand_home(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| path.to_string());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

fn compact_text(text: &str, max_chars: usize) -> String {
    if max_chars <= 1 {
        return String::new();
    }
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = text.chars().take(keep).collect();
    out.push('…');
    out
}

fn volume_bar(volume: f32, width: usize) -> String {
    let filled = (volume.clamp(0.0, 1.0) * width as f32).round() as usize;
    format!(
        "[{}{}] {:>3}%",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled)),
        (volume.clamp(0.0, 1.0) * 100.0).round() as i32
    )
}
