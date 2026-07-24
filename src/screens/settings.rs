use crate::app::App;
use crate::theme::{Theme, ThemeChoice};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(area);
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(chunks[0]);

    let choices = Theme::all_choices();
    let items: Vec<ListItem> = choices
        .iter()
        .enumerate()
        .map(|(idx, choice)| {
            let selected = idx == app.selected_settings_theme_idx;
            let equipped = *choice == app.theme_service.choice();
            let marker = if selected {
                ">"
            } else if equipped {
                "*"
            } else {
                " "
            };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else if equipped {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), style),
                Span::styled(Theme::theme_label(*choice), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                Style::default().fg(if app.selected_settings_focus_idx == 0 {
                    theme.primary
                } else {
                    theme.border
                }),
            )
            .title(Span::styled(
                " Themes ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, left_chunks[0]);

    let controls = vec![
        settings_row(
            "OS Alerts",
            (if app.external_notifications {
                "Enabled"
            } else {
                "Disabled"
            })
            .to_string(),
            "[n]",
            1,
            app,
            theme,
            if app.external_notifications {
                theme.success
            } else {
                theme.danger
            },
        ),
        settings_row(
            "Task Alerts",
            (if app.task_notifications_enabled {
                "Enabled"
            } else {
                "Disabled"
            })
            .to_string(),
            "[t]",
            2,
            app,
            theme,
            if app.task_notifications_enabled {
                theme.success
            } else {
                theme.danger
            },
        ),
        settings_row(
            "Sound Effects",
            (if app.sound_effects_enabled {
                "Enabled"
            } else {
                "Disabled"
            })
            .to_string(),
            "[s]",
            3,
            app,
            theme,
            if app.sound_effects_enabled {
                theme.success
            } else {
                theme.danger
            },
        ),
        settings_row(
            "SFX Volume",
            format!("{}%", (app.sound_effects_volume * 100.0).round() as u8),
            "Up/Down  +/-",
            4,
            app,
            theme,
            theme.warning,
        ),
        Line::from(""),
        Line::from(Span::styled(
            "Tab changes section. Enter toggles the focused setting.",
            Style::default().fg(theme.muted),
        )),
    ];

    let controls_block = Paragraph::new(controls)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if app.selected_settings_focus_idx > 0 {
                    theme.primary
                } else {
                    theme.border
                }))
                .title(Span::styled(
                    " Alerts & Audio ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(controls_block, chunks[1]);

    let selected = choices
        .get(app.selected_settings_theme_idx)
        .copied()
        .unwrap_or(ThemeChoice::ClassDefault);
    let preview_theme = app
        .user
        .as_ref()
        .map(|u| Theme::for_choice(selected, u.class))
        .unwrap_or_else(Theme::default_theme);
    let pywal_missing = selected == ThemeChoice::Pywal
        && std::env::var("HOME")
            .map(|home| {
                !std::path::Path::new(&home)
                    .join(".cache/wal/colors.json")
                    .exists()
            })
            .unwrap_or(true);

    let swatches = vec![
        ("Primary", preview_theme.primary),
        ("Secondary", preview_theme.secondary),
        ("Panel", preview_theme.panel),
        ("Text", preview_theme.text),
        ("Success", preview_theme.success),
        ("Warning", preview_theme.warning),
        ("Danger", preview_theme.danger),
    ];

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                Theme::theme_label(selected),
                Style::default()
                    .fg(preview_theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if selected == app.theme_service.choice() {
                    "  equipped"
                } else {
                    ""
                },
                Style::default().fg(theme.success),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Use Up/Down to choose a theme and Enter to apply it.",
            Style::default().fg(theme.muted),
        )),
        Line::from(Span::styled(
            "Pywal reads ~/.cache/wal/colors.json and falls back to your class theme if it is missing.",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
    ];

    if pywal_missing {
        lines.push(Line::from(Span::styled(
            "Pywal palette not found: ~/.cache/wal/colors.json",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    for (label, color) in swatches {
        lines.push(Line::from(vec![
            Span::styled("  ██  ", Style::default().fg(color)),
            Span::styled(label, Style::default().fg(theme.text)),
        ]));
    }

    let preview = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    " Theme Preview ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(preview, left_chunks[1]);
}

fn settings_row(
    label: &str,
    value: String,
    hint: &str,
    idx: usize,
    app: &App,
    theme: &Theme,
    value_color: Color,
) -> Line<'static> {
    let focused = app.selected_settings_focus_idx == idx;
    let base = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text)
    };
    let value_style = if focused {
        base
    } else {
        Style::default()
            .fg(value_color)
            .add_modifier(Modifier::BOLD)
    };
    let hint_style = if focused {
        base
    } else {
        Style::default().fg(theme.muted)
    };

    Line::from(vec![
        Span::styled(if focused { "> " } else { "  " }, base),
        Span::styled(format!("{:<15}", label), base),
        Span::styled(format!("{:<10}", value), value_style),
        Span::styled(hint.to_string(), hint_style),
    ])
}
