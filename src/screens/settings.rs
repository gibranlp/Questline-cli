use crate::app::App;
use crate::theme::{Theme, ThemeChoice};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
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
                theme.primary_selected_style()
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

    let theme_block = Block::default()
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
            " Theme Atelier ",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));
    let theme_inner = inset(left_chunks[0], 1);
    f.render_widget(theme_block, left_chunks[0]);

    let theme_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(theme_inner);
    let list = List::new(items);
    f.render_widget(list, theme_cols[0]);

    let alerts = vec![
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
            "Completion Effect: ",
            App::ambient_effect_label(app.task_completion_ambient_effect).to_string(),
            "",
            4,
            app,
            theme,
            theme.primary,
        ),
        settings_row(
            "SFX Volume",
            format!("{}%", (app.sound_effects_volume * 100.0).round() as u8),
            "Left/Right +/-",
            5,
            app,
            theme,
            theme.warning,
        ),
    ];

    let alerts_block = Paragraph::new(alerts)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(
                    if (1..=5).contains(&app.selected_settings_focus_idx) {
                        theme.primary
                    } else {
                        theme.border
                    },
                ))
                .title(Span::styled(
                    " Alerts & Audio ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(alerts_block, chunks[1]);

    let oath_calendar = vec![
        settings_row(
            "Monday",
            checked(app.streak_weekday_enabled(0)),
            "Space",
            6,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Tuesday",
            checked(app.streak_weekday_enabled(1)),
            "Space",
            7,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Wednesday",
            checked(app.streak_weekday_enabled(2)),
            "Space",
            8,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Thursday",
            checked(app.streak_weekday_enabled(3)),
            "Space",
            9,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Friday",
            checked(app.streak_weekday_enabled(4)),
            "Space",
            10,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Saturday",
            checked(app.streak_weekday_enabled(5)),
            "Space",
            11,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Sunday",
            checked(app.streak_weekday_enabled(6)),
            "Space",
            12,
            app,
            theme,
            theme.success,
        ),
        settings_row(
            "Streak Start",
            format!("{:02}:00", app.streak_active_from),
            "Left/Right +/-",
            13,
            app,
            theme,
            theme.secondary,
        ),
        settings_row(
            "Streak End",
            if app.streak_active_to == 24 {
                "24:00".to_string()
            } else {
                format!("{:02}:00", app.streak_active_to)
            },
            "Left/Right +/-",
            14,
            app,
            theme,
            theme.secondary,
        ),
        Line::from(""),
        Line::from(Span::styled(
            "The flame honors only the sworn days and watch hours.",
            Style::default().fg(theme.muted),
        )),
    ];

    let oath_block = Paragraph::new(oath_calendar)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(
                    if (6..=14).contains(&app.selected_settings_focus_idx) {
                        theme.primary
                    } else {
                        theme.border
                    },
                ))
                .title(Span::styled(
                    " Oath Calendar ",
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(oath_block, left_chunks[1]);

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

    let preview = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(preview, theme_cols[1]);
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
        theme.primary_selected_style()
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
        Span::styled(format!("{:<16}", value), value_style),
        Span::styled(hint.to_string(), hint_style),
    ])
}

fn checked(enabled: bool) -> String {
    if enabled {
        "[x]".to_string()
    } else {
        "[ ]".to_string()
    }
}

fn inset(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(margin),
        y: area.y.saturating_add(margin),
        width: area.width.saturating_sub(margin.saturating_mul(2)),
        height: area.height.saturating_sub(margin.saturating_mul(2)),
    }
}
