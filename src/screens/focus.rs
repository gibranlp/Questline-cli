// ─────────────────────────────────────────────────────────────────────────────
// screens/focus.rs — el timer de enfoque tipo pomodoro, con soundscapes y estadísticas
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{App, ModalType};
use crate::models::{Project, Task};
use crate::screens::intro::centered_rect;
use crate::theme::Theme;
use chrono::Utc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

// Punto de entrada — decide si mostrar la sesión activa o la pantalla de config
pub fn draw(f: &mut Frame, app: &App, theme: &Theme) {
    let size = f.size();
    let accent_color = theme.primary;

    if app.active_focus_session.is_some() {
        draw_active_session(f, app, theme, size);
    } else {
        draw_config_screen(f, app, theme, size);
    }

    // Modal de duración custom — aparece encima de todo cuando el user quiere otro tiempo
    if let ModalType::CustomFocusDuration { input } = &app.modal_state {
        let area = centered_rect(40, 25, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Custom Focus Duration ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        let text = vec![
            Line::from(""),
            Line::from(" Enter duration in minutes:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("   > ", Style::default().fg(accent_color)),
                Span::styled(
                    input,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("█", Style::default().fg(accent_color)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " [Enter] Start Session  |  [Esc] Cancel ",
                Style::default()
                    .fg(theme.muted),
            )),
        ];
        let p = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, area);
    }
}

// Renderiza el timer en vivo — aquí es donde la sesión ya está corriendo, órale a trabajar
fn draw_active_session(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let active = app.active_focus_session.as_ref().unwrap();
    let total_seconds = (active.duration_mins * 60) as i64;
    let elapsed_seconds = (Utc::now() - active.start_time).num_seconds();
    // remaining nunca baja de 0, por si el timer se pasa un poco
    let remaining = std::cmp::max(0, total_seconds - elapsed_seconds);
    let mins = remaining / 60;
    let secs = remaining % 60;
    let timer_str = format!("{:02}:{:02}", mins, secs);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(8), // Big Timer Box
            Constraint::Min(5),    // Zen Tree & Motivation
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // 1. Header
    let header_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "DEEP WORK FOCUS SANCTUARY",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // 2. Timer Box — busca nombre del proyecto y tarea activos para mostrarlos
    let project_name = if let Some(p_id) = active.project_id {
        app.projects
            .iter()
            .find(|p| p.id == p_id)
            .map(|p| p.name.as_str())
            .unwrap_or("Unknown Project")
    } else {
        "None (General Focus)"
    };

    // Jala la tarea de la DB — puede fallar si el DB está ocupado, no manches
    let task_title = if let Some(t_id) = active.task_id {
        if let Ok(all_tasks) = app.db.get_tasks() {
            all_tasks
                .into_iter()
                .find(|t| t.id == t_id)
                .map(|t| t.title)
                .unwrap_or_else(|| "Unknown Quest".to_string())
        } else {
            "Unknown Quest".to_string()
        }
    } else {
        "General Mind Cleansing".to_string()
    };

    let timer_text = vec![
        Line::from(vec![
            Span::styled("ACTIVE FOCUS: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} mins", active.duration_mins),
                Style::default().fg(theme.warning),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                " [ ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                timer_str,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " ] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Realm: ", Style::default().fg(theme.muted)),
            Span::styled(project_name, Style::default().fg(Color::White)),
            Span::styled("  |  Quest: ", Style::default().fg(theme.muted)),
            Span::styled(
                task_title,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  Soundscape: ", Style::default().fg(theme.muted)),
            Span::styled(active.soundscape.clone(), Style::default().fg(Color::Cyan)),
        ]),
    ];

    let timer_box = Paragraph::new(timer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.primary)),
        )
        .alignment(Alignment::Center);
    f.render_widget(timer_box, chunks[1]);

    // 3. Middle Section: árbol zen a la izquierda, quote motivacional a la derecha
    let mid_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Tree Companion
            Constraint::Percentage(60), // Motivating scroll
        ])
        .split(chunks[2]);

    // Left: el árbol zen — crece con cada sesión completada, chido sistema
    let zen_tree = app.db.get_zen_tree().unwrap();
    let tree_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            zen_tree.ascii_art(),
            Style::default().fg(theme.success),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Companion Tree: ", Style::default().fg(theme.muted)),
            Span::styled(
                zen_tree.stage_name(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Growth: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} pts", zen_tree.growth),
                Style::default().fg(theme.success),
            ),
        ]),
    ];
    let tree_box = Paragraph::new(tree_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Companion Tree "),
        )
        .alignment(Alignment::Center);
    f.render_widget(tree_box, mid_chunks[0]);

    // Right: quote del app — se selecciona aleatoriamente al iniciar la sesión
    let quote_text = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            format!("\"{}\"", app.quote),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("— {}", app.quote_author),
            Style::default()
                .fg(theme.text)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let quote_box = Paragraph::new(quote_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Sanctuary Scrolls (Motivation) "),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(quote_box, mid_chunks[1]);
}

// Pantalla de configuración — el user elige duración, proyecto, tarea y soundscape antes de arrancar
fn draw_config_screen(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(7), // Pickers Layout
            Constraint::Min(4),    // Information / Selected details
            Constraint::Length(3), // Help instructions footer
        ])
        .split(size);

    // Header
    let header_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "DEEP WORK FOCUS SANCTUARY",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let header = Paragraph::new(header_text).alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Pickers — cuatro tarjetas: duración, proyecto, tarea y soundscape
    let picker_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    // Solo proyectos activos — los completados/archivados no cuentan pues
    let active_projects: Vec<Project> = app
        .projects
        .iter()
        .filter(|p| !p.completed && !p.archived)
        .cloned()
        .collect();

    // Las tareas cambian según el proyecto seleccionado — índice 0 = ninguno (focus general)
    let mut active_tasks: Vec<Task> = Vec::new();
    if app.selected_focus_project_idx > 0 && app.selected_focus_project_idx <= active_projects.len()
    {
        let selected_p_id = active_projects[app.selected_focus_project_idx - 1].id;
        if let Ok(all_tasks) = app.db.get_tasks() {
            active_tasks = all_tasks
                .into_iter()
                .filter(|t| t.project_id == Some(selected_p_id) && !t.completed)
                .collect();
        }
    }

    // Options strings
    let duration_options = [
        "15 Minutes",
        "25 Minutes",
        "45 Minutes",
        "60 Minutes",
        "90 Minutes",
        "Custom Duration",
    ];
    let duration_str = duration_options[app.selected_focus_duration_idx];

    let project_str = if app.selected_focus_project_idx == 0 {
        "None (General Focus)".to_string()
    } else {
        active_projects[app.selected_focus_project_idx - 1]
            .name
            .clone()
    };

    let task_str = if app.selected_focus_task_idx == 0 || active_tasks.is_empty() {
        "None (General Focus)".to_string()
    } else {
        active_tasks[app.selected_focus_task_idx - 1].title.clone()
    };

    // Resuelve el nombre del soundscape — índice 0 es silencio, los demás del array global
    let selected_sc_str = {
        use crate::audio::SOUNDSCAPES;
        match app.selected_focus_soundscape_idx {
            0 => "None (Silent)".to_string(),
            idx if idx <= SOUNDSCAPES.len() => SOUNDSCAPES[idx - 1].name.to_string(),
            _ => "None (Silent)".to_string(),
        }
    };

    // Card 0: Focus Duration — resalta el borde si es el campo activo
    let border_style_0 = if app.selected_focus_field_idx == 0 {
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let widget_text_0 = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "▲  ",
                Style::default().fg(if app.selected_focus_field_idx == 0 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
            Span::styled(
                duration_str,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  ▼",
                Style::default().fg(if app.selected_focus_field_idx == 0 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
        ]),
    ];
    let card_0 = Paragraph::new(widget_text_0)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style_0)
                .title(" 1. Focus Duration "),
        )
        .alignment(Alignment::Center);
    f.render_widget(card_0, picker_chunks[0]);

    // Card 1: Linked Project
    let border_style_1 = if app.selected_focus_field_idx == 1 {
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let widget_text_1 = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "▲  ",
                Style::default().fg(if app.selected_focus_field_idx == 1 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
            Span::styled(
                project_str.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  ▼",
                Style::default().fg(if app.selected_focus_field_idx == 1 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
        ]),
    ];
    let card_1 = Paragraph::new(widget_text_1)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style_1)
                .title(" 2. Bound Project "),
        )
        .alignment(Alignment::Center);
    f.render_widget(card_1, picker_chunks[1]);

    // Card 2: Linked Task
    let border_style_2 = if app.selected_focus_field_idx == 2 {
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let widget_text_2 = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "▲  ",
                Style::default().fg(if app.selected_focus_field_idx == 2 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
            Span::styled(
                task_str.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  ▼",
                Style::default().fg(if app.selected_focus_field_idx == 2 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
        ]),
    ];
    let card_2 = Paragraph::new(widget_text_2)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style_2)
                .title(" 3. Focused Task/Quest "),
        )
        .alignment(Alignment::Center);
    f.render_widget(card_2, picker_chunks[2]);

    // Card 3: Soundscape Option
    let border_style_3 = if app.selected_focus_field_idx == 3 {
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let widget_text_3 = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "▲  ",
                Style::default().fg(if app.selected_focus_field_idx == 3 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
            Span::styled(
                selected_sc_str.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  ▼",
                Style::default().fg(if app.selected_focus_field_idx == 3 {
                    theme.warning
                } else {
                    theme.muted
                }),
            ),
        ]),
    ];
    let card_3 = Paragraph::new(widget_text_3)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style_3)
                .title(" 4. Soundscape "),
        )
        .alignment(Alignment::Center);
    f.render_widget(card_3, picker_chunks[3]);

    // XP reward calculado por duración — custom usa el mismo valor que 25 min de placeholder
    let reward_xp = match app.selected_focus_duration_idx {
        0 => 10,
        1 => 20,
        2 => 35,
        3 => 50,
        4 => 80,
        _ => 20, // Custom placeholder
    };

    let details_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "SESSION SUMMARY:",
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from(vec![
            Span::styled("   • Duration: ", Style::default().fg(theme.muted)),
            Span::styled(duration_str, Style::default().fg(Color::White)),
            Span::styled(
                "  |  Potential Reward: ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("+{} XP", reward_xp),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Increases tree growth)",
                Style::default()
                    .fg(theme.muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("   • Target Realm: ", Style::default().fg(theme.muted)),
            Span::styled(project_str, Style::default().fg(Color::White)),
            Span::styled("  |  Active Quest: ", Style::default().fg(theme.muted)),
            Span::styled(task_str, Style::default().fg(theme.primary)),
            Span::styled("  |  Atmosphere: ", Style::default().fg(theme.muted)),
            Span::styled(selected_sc_str, Style::default().fg(Color::Cyan)),
        ]),
    ];
    let details = Paragraph::new(details_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Session Forecast "),
    );
    f.render_widget(details, chunks[2]);

    // Footer instructions
    let footer_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " ←→/hl",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" switch field | ", Style::default().fg(theme.muted)),
            Span::styled(
                "↑↓/jk",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cycle selections | ", Style::default().fg(theme.muted)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " start deep work focus ",
                Style::default().fg(theme.muted),
            ),
        ]),
    ];
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center);
    f.render_widget(footer, chunks[3]);
}
