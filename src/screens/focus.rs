// ─────────────────────────────────────────────────────────────────────────────
// screens/focus.rs — el timer de enfoque tipo pomodoro, con soundscapes y estadísticas
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{App, ModalType};
use crate::models::{Project, Task};
use crate::screens::intro::centered_rect;
use crate::services::bonsai::BonsaiGrid;
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
            Constraint::Length(5), // Visualizador de música
            Constraint::Min(5),    // Zen Tree & Motivación
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
            .unwrap_or("Unknown Campaign")
    } else {
        "None (General Focus)"
    };

    // Jala la tarea de la DB — puede fallar si el DB está ocupado, no manches
    let task_title = if let Some(t_id) = active.task_id {
        match app.db.get_tasks() { Ok(all_tasks) => {
            all_tasks
                .into_iter()
                .find(|t| t.id == t_id)
                .map(|t| t.title)
                .unwrap_or_else(|| "Unknown Quest".to_string())
        } _ => {
            "Unknown Quest".to_string()
        }}
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
            Span::styled(" Campaign: ", Style::default().fg(theme.muted)),
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

    // 3. Visualizador de música — misma lógica que en el config screen
    draw_visualizer(f, app, theme, chunks[2]);

    // 4. Middle Section: árbol zen a la izquierda, quote motivacional a la derecha
    let mid_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Tree Companion
            Constraint::Percentage(60), // Motivating scroll
        ])
        .split(chunks[3]);

    // Animación del árbol: crece lentamente de etapa 1 a la actual, luego espera
    // 160 ticks/etapa = 8 segundos por transición; 18 000 ticks = 15 minutos en la etapa final
    let zen_tree = app.db.get_zen_tree().unwrap();
    let current_stage = zen_tree.stage.max(1) as usize;
    const STAGE_TICKS: usize = 160;
    const HOLD_TICKS: usize = 18_000;
    let grow_ticks = current_stage * STAGE_TICKS;
    let cycle_len = grow_ticks + HOLD_TICKS;
    let cycle_pos = app.music_scroll_ticks % cycle_len;
    let animated_stage = if cycle_pos >= grow_ticks {
        current_stage as i32
    } else {
        (cycle_pos / STAGE_TICKS + 1).min(current_stage) as i32
    };

    // Borde del bloque; el área interior se divide en árbol (arriba) y estadísticas (abajo)
    let tree_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(" Companion Tree ");
    let tree_inner = tree_block.inner(mid_chunks[0]);
    f.render_widget(tree_block, mid_chunks[0]);

    if tree_inner.height > 3 && tree_inner.width > 4 {
        let tree_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(2)])
            .split(tree_inner);

        let grid = BonsaiGrid::generate(
            tree_sections[0].height as usize,
            tree_sections[0].width as usize,
            zen_tree.growth as u64,
            animated_stage,
            zen_tree.health,
        );
        f.render_widget(Paragraph::new(grid.into_lines()), tree_sections[0]);

        let tree_stats = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Stage: ", Style::default().fg(theme.muted)),
                Span::styled(
                    zen_tree.stage_name(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  Growth: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{} pts", zen_tree.growth),
                    Style::default().fg(theme.success),
                ),
            ]),
        ])
        .alignment(Alignment::Center);
        f.render_widget(tree_stats, tree_sections[1]);
    }

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
            Constraint::Length(5), // Visualizador de música
            Constraint::Min(4),    // Session Forecast
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
                .title(" 2. Bound Campaign"),
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
                .title(" 3. Focused Quest "),
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
            Span::styled("   • Target Campaign: ", Style::default().fg(theme.muted)),
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

    // renderiza el visualizador entre los pickers y el Session Forecast
    draw_visualizer(f, app, theme, chunks[2]);

    f.render_widget(details, chunks[3]);

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
    f.render_widget(footer, chunks[4]);
}

// barras de espectro FFT en tiempo real — datos reales del audio local, animación sutil si no hay
fn draw_visualizer(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    use crate::audio::PlaybackStatus;
    use crate::audio::spectrum::NUM_BARS;

    let status = app.audio_player.get_state().status;
    let spectrum = app.audio_player.get_spectrum();
    let t = app.music_scroll_ticks as f32;

    // caracteres de bloque Unicode de menor a mayor altura
    const BAR_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    // si hay señal real (local FFT o captura del sistema), úsala — si no, animación de idle
    let has_signal = spectrum.iter().any(|&v| v > 0.015);

    let bar_spans: Vec<Span> = (0..NUM_BARS)
        .map(|i| {
            let height = if has_signal {
                // datos reales: soundscape local vía SpectrumSource o MPRIS vía captura del monitor
                match status {
                    PlaybackStatus::Paused => spectrum[i].max(0.04),
                    _ => spectrum[i],
                }
            } else {
                // animación de idle cuando no hay señal de audio en el sistema
                match status {
                    PlaybackStatus::Playing => {
                        let h = 0.5
                            + 0.30 * f32::sin(t * 0.07 + i as f32 * 0.31)
                            + 0.20 * f32::sin(t * 0.11 + i as f32 * 0.53)
                            + 0.10 * f32::sin(t * 0.19 + i as f32 * 0.17);
                        h.clamp(0.0, 1.0)
                    }
                    PlaybackStatus::Paused => 0.38,
                    PlaybackStatus::Stopped => {
                        let h = 0.10
                            + 0.05 * f32::sin(t * 0.03 + i as f32 * 0.31)
                            + 0.03 * f32::sin(t * 0.05 + i as f32 * 0.53);
                        h.clamp(0.0, 1.0)
                    }
                }
            };

            let char_idx = ((height * 7.99) as usize).min(7);
            let bar_char = BAR_CHARS[char_idx];

            // gradiente de color según la altura: bajo→muted, medio→primary, alto→warning
            let color = if height < 0.35 {
                theme.muted
            } else if height < 0.72 {
                theme.primary
            } else {
                theme.warning
            };

            Span::styled(
                format!("{} ", bar_char),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    let title_str = match status {
        PlaybackStatus::Playing => " ♪ Soundscape Visualizer ",
        PlaybackStatus::Paused => " ⏸ Soundscape Visualizer ",
        PlaybackStatus::Stopped => " ○ Soundscape Visualizer ",
    };
    let title_color = match status {
        PlaybackStatus::Playing => theme.primary,
        PlaybackStatus::Paused => theme.warning,
        PlaybackStatus::Stopped => theme.muted,
    };

    let vis_text = vec![Line::from(""), Line::from(bar_spans)];
    let vis = Paragraph::new(vis_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(title_color))
                .title(Span::styled(
                    title_str,
                    Style::default().fg(title_color).add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(vis, area);
}
