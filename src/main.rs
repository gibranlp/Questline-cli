// ─────────────────────────────────────────────────────────────────────────────
// main.rs — arranca todo el show: terminal, loop principal, renders y modales
// ─────────────────────────────────────────────────────────────────────────────

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::should_implement_trait,
    clippy::needless_range_loop,
    clippy::unnecessary_sort_by,
    clippy::redundant_pattern_matching,
    clippy::if_same_then_else
)]

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Terminal,
};
use std::io;

use questline::app::App;
use questline::database;
use questline::screens;
use questline::screens::ActiveScreen;
use questline::storage;
// Centra un rect en la pantalla con altura fija — los modales lo usan para no sobrepasar el tamaño
pub fn centered_rect_fixed_height(percent_x: u16, height_lines: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height_lines) / 2),
            Constraint::Length(height_lines),
            Constraint::Length(r.height.saturating_sub(height_lines) / 2),
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

// Aquí empieza todo el desmadre — inicializa la terminal, corre el loop y maneja el shutdown
#[tokio::main]
async fn main() -> Result<()> {
    let startup_start = std::time::Instant::now();
    questline::services::init_panic_hook();
    // Checa si corrieron el binario con un subcomando (export / import / backup / --version)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let cmd = args[1].as_str();
        let storage_dir = storage::ensure_storage_dir_exists()?;
        let db_path = storage_dir.join("questline.db");
        match cmd {
            "export" => {
                let db = database::Database::new(&db_path)?;
                let json = db.export_to_json()?;
                std::fs::write(".questline-export", json)?;
                println!("Exported database to .questline-export");
                return Ok(());
            }
            "import" => {
                let db = database::Database::new(&db_path)?;
                if std::path::Path::new(".questline-export").exists() {
                    let json = std::fs::read_to_string(".questline-export")?;
                    db.import_from_json(&json)?;
                    println!("Imported database from .questline-export");
                } else {
                    println!("Error: .questline-export file not found");
                }
                return Ok(());
            }
            "backup" => {
                let date_str = chrono::Utc::now().format("%Y_%m_%d").to_string();
                let backup_filename = format!("questline_backup_{}.db", date_str);
                std::fs::copy(&db_path, &backup_filename)?;
                println!("Created backup: {}", backup_filename);

                // Lleva conteo de cuántos backups se han hecho — se guarda en settings de la DB
                let db = database::Database::new(&db_path)?;
                let count = db
                    .get_setting("backup_count")?
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                db.set_setting("backup_count", &(count + 1).to_string())?;
                return Ok(());
            }
            "--version" | "-v" | "version" => {
                println!("questline {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {
                println!("Unknown command. Use: export, import, backup, --version");
                return Ok(());
            }
        }
    }

    // Órale, a preparar la terminal — raw mode, pantalla alterna, backend de crossterm
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Jala el directorio de storage y construye la ruta a la DB
    let storage_dir = storage::ensure_storage_dir_exists()?;
    let db_path = storage_dir.join("questline.db");

    // Inicializa el estado global de la app — si falla aquí, limpiamos la terminal antes de morir
    let mut app = match App::new(&db_path) {
        Ok(a) => a,
        Err(e) => {
            print!("\x1b]111\x07");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            return Err(e);
        }
    };

    let startup_duration = startup_start.elapsed();
    questline::services::log_structured(
        "INFO",
        "startup",
        &format!(
            "Questline initialized. Startup time: {:?}",
            startup_duration
        ),
        Some(&format!("Duration: {}ms", startup_duration.as_millis())),
    );

    // Verifica y repara la integridad de los datos — no manches si la DB está corrupta
    match app.db.verify_data_integrity() {
        Ok(reports) => {
            for report in reports {
                questline::services::log_structured("INFO", "data_integrity", &report, None);
            }
        }
        Err(e) => {
            questline::services::log_structured(
                "ERROR",
                "data_integrity",
                "Failed to verify/repair data integrity.",
                Some(&e.to_string()),
            );
        }
    }

    // Escanea los backups del directorio actual y avisa si alguno está cagado
    let mut corrupted_backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("questline_backup_") && filename.ends_with(".db") {
                    match questline::database::Database::verify_db_backup(&path) {
                        Ok(true) => {
                            questline::services::log_structured(
                                "INFO",
                                "backup_verification",
                                &format!("Backup validated successfully: {}", filename),
                                None,
                            );
                        }
                        _ => {
                            questline::services::log_structured(
                                "WARNING",
                                "backup_verification",
                                &format!("Corrupted backup detected: {}", filename),
                                Some(&format!("Path: {:?}", path)),
                            );
                            corrupted_backups.push(filename.to_string());
                        }
                    }
                }
            }
        }
    }
    app.corrupted_backups_found = corrupted_backups;

    // Loop principal — 50ms por tick = ~20fps, primero input luego render para reducir latencia
    let tick_rate = std::time::Duration::from_millis(50);
    loop {
        // Primero checar si hay tecla presionada antes de dibujar — así el input se siente más rápido
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    app.handle_key_event(key)?;
                }
            }
        }

        if app.should_quit {
            break;
        }

        app.terminal_height = terminal.size().map(|s| s.height).unwrap_or(40);
        // Todos los ticks del frame: sync, chat, focus timer, partículas, updates, animaciones
        app.tick_auto_sync()?;
        app.tick_chat_poll()?;
        app.tick_focus_session()?;
        app.tick_particles();
        app.tick_update_check();
        app.tick_chapter_progress();
        app.tick_sprite_notifications();
        app.quit_confirm_ticks = app.quit_confirm_ticks.wrapping_add(1);
        app.intro_ticks = app.intro_ticks.wrapping_add(1);
        app.music_scroll_ticks = app.music_scroll_ticks.wrapping_add(1);
        app.tick_prologue();

        // Aquí empieza el render del frame — todo lo que se ve en pantalla viene de aquí
        terminal.draw(|f| {
            let size = f.size();
            let theme = app.theme_service.theme();

            // Pinta el fondo con el color del tema activo para limpiar artefactos del frame anterior
            let bg_block = Block::default().style(Style::default().bg(theme.background));
            f.render_widget(bg_block, size);

            // Ajusta el color de fondo de la terminal (para pintar el padding/borde)
            match theme.background {
                Color::Rgb(r, g, b) => {
                    print!("\x1b]11;#{:02x}{:02x}{:02x}\x07", r, g, b);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                Color::Black => {
                    print!("\x1b]11;#000000\x07");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                Color::White => {
                    print!("\x1b]11;#ffffff\x07");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                _ => {}
            }

            match app.active_screen {
                ActiveScreen::Intro => {
                    screens::intro::draw(
                        f,
                        &app.quote,
                        &app.quote_author,
                        app.intro_ticks,
                        &theme,
                    );
                }
                ActiveScreen::Prologue => {
                    screens::prologue::draw(f, &app, &theme);
                }
                ActiveScreen::Onboarding => {
                    screens::onboarding::draw(
                        f,
                        &app.onboarding_username,
                        app.onboarding_class_idx,
                        app.onboarding_focus,
                        &app.onboarding_classes,
                        app.onboarding_error.as_deref(),
                    );
                }
                ActiveScreen::Editor => {
                    if let Some(ref s) = app.editor_state {
                        screens::editor::draw(f, s, &theme);
                    }
                }
                ActiveScreen::Workspace => {
                    screens::project_workspace::draw(
                        f,
                        &app,
                        &theme,
                    );
                }

                _ => {
                    // Layout de dos filas: área principal arriba, footer de ayuda abajo
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(5),    // Body screen
                            Constraint::Length(3), // Footer status bar
                        ])
                        .split(size);

                    // Render Screen Body
                    match app.active_screen {
                        ActiveScreen::Dashboard => {
                            screens::dashboard::draw(f, &app, &theme, chunks[0]);
                        },

                        ActiveScreen::Focus => {
                            screens::focus::draw(f, &app, &theme);
                        },
                        ActiveScreen::Projects => {
                            screens::projects::draw(f, &app.projects, app.selected_project_idx, &app.modal_state, &theme, chunks[0]);
                        }

                        ActiveScreen::Character => {
                            // Jala un chorro de datos de la DB para la pantalla de personaje — no se cachean
                            let achievements = app.db.get_achievements().unwrap_or_default();
                            let achievements_count = achievements.iter().filter(|a| a.unlocked_at.is_some()).count() as i32;
                            let tree = app.db.get_zen_tree().unwrap();
                            let streak_obj = app.db.get_streak().unwrap();
                            let xp_history = app.db.get_xp_history().unwrap_or_default();
                            let most_productive = app.db.get_most_productive_project().unwrap_or_else(|_| "None yet".to_string());
                            let reflections = app.db.get_reflections().unwrap_or_default();
                            let devices = app.db.get_devices().unwrap_or_default();
                            let chronicle_entries = app.db.get_chronicle_entries().unwrap_or_default();

                            screens::character::draw(
                                f,
                                app.user.as_ref().unwrap(),
                                achievements_count,
                                tree.stage_name(),
                                tree.growth,
                                tree.health,
                                streak_obj.current_streak,
                                streak_obj.best_streak,
                                &xp_history,
                                &most_productive,
                                &reflections,
                                app.selected_reflection_idx,
                                &app.modal_state,
                                &devices,
                                &chronicle_entries,
                                app.selected_chronicle_idx,
                                app.character_focus,
                                app.reflection_detail_scroll,
                                &theme,
                                chunks[0],
                            );
                        }
                        ActiveScreen::Archive => {
                            screens::archive::draw(f, &app.projects, app.selected_archive_idx, &theme);
                        }

                        ActiveScreen::Soundscapes => {
                            screens::soundscapes::draw(f, &app, &theme, chunks[0]);
                        }
                        ActiveScreen::SyncSettings => {
                            screens::sync::draw(f, &app, &theme, chunks[0]);
                        }
                        ActiveScreen::Fellowship => {
                            screens::fellowship::draw(f, &app, &theme, chunks[0]);
                        }
                        ActiveScreen::About => {
                            screens::about::draw(f, &app, &theme, chunks[0]);
                        }
                        ActiveScreen::GreatChronicle => {
                            screens::great_chronicle::draw(f, &app, &theme, chunks[0]);
                        }

                        ActiveScreen::Library => {
                            let class_name = app.user.as_ref().map(|u| u.class.name()).unwrap_or("");
                            let quests = app.db.get_class_quests(class_name).unwrap_or_default();
                            let lore = app.db.get_lore_entries().unwrap_or_default();
                            screens::library::draw(
                                f,
                                app.library_active_col,
                                app.selected_library_cat_idx,
                                app.selected_library_item_idx,
                                app.library_scroll_offset,
                                &quests,
                                &lore,
                                class_name,
                                &theme,
                            );
                        }
                        ActiveScreen::Legends => {
                            let stats = app.db.get_statistics().unwrap();
                            let relics = app.db.get_relics().unwrap_or_default();
                            screens::legends::draw(f, &stats, app.selected_relic_idx, &relics, &theme);
                        }
                        _ => {}
                    }

                    // Render Footer Help Info
                    let tabs_str = "[1]D [2]P [3]H [4]L [5]M [6]S [7]F [8]G ?";

                    let footer_block = Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray));
                    
                    let footer_area = footer_block.inner(chunks[1]);
                    f.render_widget(footer_block, chunks[1]);
                    
                    let footer_cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Min(40),
                            Constraint::Length(30),
                            Constraint::Length(45),
                        ])
                        .split(footer_area);

                    let footer_text = vec![
                        Line::from(vec![
                            Span::styled(" Keys: ", Style::default().fg(Color::Rgb(140, 140, 140))),
                            Span::styled(tabs_str, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                            Span::styled(" | ", Style::default().fg(Color::Rgb(140, 140, 140))),
                            Span::styled("Tab/Shift+Tab", Style::default().fg(theme.primary)),
                            Span::styled(" cycle | ", Style::default().fg(Color::Rgb(140, 140, 140))),
                            Span::styled("Ctrl+P", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                            Span::styled(" palette | ", Style::default().fg(Color::Rgb(140, 140, 140))),
                            Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                            Span::styled(" quit", Style::default().fg(Color::Rgb(140, 140, 140))),
                        ])
                    ];

                    // Sync status: 10s transient para éxito/fallo — permanente si hay 3+ fallos consecutivos
                    let sync_line = if app.sync_failure_count >= 3 {
                        Line::from(vec![
                            Span::styled("⚠ Sync offline", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ])
                    } else if let Some(t) = app.last_sync_status_time {
                        if t.elapsed().as_secs() < 10 {
                            let color = if app.sync_status_msg.starts_with("Sync failed") {
                                Color::Red
                            } else {
                                Color::Cyan
                            };
                            Line::from(vec![
                                Span::styled(app.sync_status_msg.clone(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                            ])
                        } else {
                            Line::from(vec![])
                        }
                    } else {
                        Line::from(vec![])
                    };

                    let audio_state = app.audio_player.get_state();

                    // Quita el prefijo "Local: " para que se vea limpio en el footer
                    let raw_track = audio_state.current_soundscape
                        .trim_start_matches("Local: ")
                        .to_string();

                    // El panel derecho tiene 45 chars — "Playing: " + nombre + volumen deben caber ahí
                    let max_name_chars: usize = 28;
                    let vol_str = format!("({}%)", (audio_state.volume * 100.0) as i32);

                    // Marquee que avanza un char cada 5 ticks (250ms) — para nombres largos de tracks
                    let scroll_name = |name: &str| -> String {
                        let char_count = name.chars().count();
                        if char_count <= max_name_chars {
                            return format!("{:<width$}", name, width = max_name_chars);
                        }
                        let padded: Vec<char> = format!("{}   |   ", name).chars().collect();
                        let total = padded.len();
                        let offset = (app.music_scroll_ticks / 5) % total;
                        (0..max_name_chars).map(|i| padded[(offset + i) % total]).collect()
                    };

                    let music_text = match audio_state.status {
                        questline::audio::state::PlaybackStatus::Playing => {
                            Line::from(vec![
                                Span::styled("Playing: ", Style::default().fg(Color::Cyan)),
                                Span::styled(scroll_name(&raw_track), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                Span::styled(format!(" {}", vol_str), Style::default().fg(Color::Cyan)),
                            ])
                        }
                        questline::audio::state::PlaybackStatus::Paused => {
                            Line::from(vec![
                                Span::styled("Paused:  ", Style::default().fg(Color::Yellow)),
                                Span::styled(scroll_name(&raw_track), Style::default().fg(Color::Rgb(200, 200, 200))),
                                Span::styled(format!(" {}", vol_str), Style::default().fg(Color::Yellow)),
                            ])
                        }
                        questline::audio::state::PlaybackStatus::Stopped => {
                            Line::from(vec![
                                Span::styled("Silent", Style::default().fg(Color::Rgb(140, 140, 140))),
                            ])
                        }
                    };

                    let left_p = Paragraph::new(footer_text);
                    let sync_p = Paragraph::new(sync_line).alignment(ratatui::layout::Alignment::Center);
                    let right_p = Paragraph::new(music_text).alignment(ratatui::layout::Alignment::Right);
                    
                    f.render_widget(left_p, footer_cols[0]);
                    f.render_widget(sync_p, footer_cols[1]);
                    f.render_widget(right_p, footer_cols[2]);
                }
            }

            // Limpia notificaciones viejas cada frame — solo duran 4 segundos y bye
            app.notifications.retain(|n| n.unlocked_at.elapsed().as_secs() < 4);

            // Overlay de notificación flotante — aparece sobre cualquier pantalla de gameplay
            if app.active_screen != ActiveScreen::Intro
                && app.active_screen != ActiveScreen::Prologue
                && app.active_screen != ActiveScreen::Onboarding
            {
                if let Some(notif) = app.notifications.last() {
                    let overlay_area = questline::screens::intro::centered_rect(50, 15, size);
                    f.render_widget(Clear, overlay_area);
                    f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        .title(Span::styled(" Quest Alert ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

                    let paragraph = Paragraph::new(format!("\n {}", notif.message))
                        .block(block)
                        .alignment(ratatui::layout::Alignment::Center)
                        .style(Style::default().fg(Color::White));

                    f.render_widget(paragraph, overlay_area);
                }
            }

            // Modal para configurar la carpeta de música local — incluye autocompletado de paths
            if let questline::app::ModalType::LocalMusicFolder { ref input, ref suggestions, selected } = app.modal_state {
                let has_suggestions = !suggestions.is_empty();
                let modal_height = if has_suggestions { 40 } else { 20 };
                let area = questline::screens::intro::centered_rect(60, modal_height, size);
                f.render_widget(Clear, area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(Span::styled(
                        " Set Local Music Folder Path ",
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                let sug_count = suggestions.len().min(6) as u16;
                let constraints = if has_suggestions {
                    vec![
                        ratatui::layout::Constraint::Length(1),
                        ratatui::layout::Constraint::Length(3),
                        ratatui::layout::Constraint::Length(sug_count + 2),
                        ratatui::layout::Constraint::Min(1),
                    ]
                } else {
                    vec![
                        ratatui::layout::Constraint::Length(1),
                        ratatui::layout::Constraint::Length(3),
                        ratatui::layout::Constraint::Min(1),
                    ]
                };
                let inner = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(constraints)
                    .split(block.inner(area));
                f.render_widget(block, area);
                let input_theme = &theme;
                let input_p = Paragraph::new(format!("  {}_", input)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(input_theme.primary))
                        .title(" Folder Path "),
                );
                f.render_widget(input_p, inner[1]);
                if has_suggestions {
                    let items: Vec<ListItem> = suggestions.iter().take(6).enumerate().map(|(i, s)| {
                        if i == selected {
                            ListItem::new(format!("  > {}", s))
                                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        } else {
                            ListItem::new(format!("    {}", s))
                                .style(Style::default().fg(Color::Rgb(200, 200, 200)))
                        }
                    }).collect();
                    let list = List::new(items).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::DarkGray))
                            .title(Span::styled(" Suggestions  [Tab/↓↑] navigate ", Style::default().fg(Color::Rgb(140, 140, 140)))),
                    );
                    f.render_widget(list, inner[2]);
                    let help_p = Paragraph::new("  [Tab/↓] next  [↑] prev  [Enter] save  [Esc] cancel")
                        .style(Style::default().fg(Color::Rgb(140, 140, 140)));
                    f.render_widget(help_p, inner[3]);
                } else {
                    let help_p = Paragraph::new("  [Enter] save  |  [Esc] cancel")
                        .style(Style::default().fg(Color::Rgb(140, 140, 140)));
                    f.render_widget(help_p, inner[2]);
                }
            }

            // Overlay de Memory Fragment — chido cuando aparece, dura 6 segundos y se va solo
            let frag_expired = app.fragment_notification.as_ref()
                .map(|a| a.shown_at.elapsed().as_secs() >= 6)
                .unwrap_or(false);
            if frag_expired {
                app.fragment_notification = None;
            }
            if let Some(ref alert) = app.fragment_notification {
                let rarity_color = match alert.rarity.as_str() {
                    "Legendary" => Color::Yellow,
                    "Rare" => Color::Cyan,
                    _ => Color::White,
                };
                let overlay_area = questline::screens::intro::centered_rect(52, 22, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(rarity_color).add_modifier(Modifier::BOLD))
                    .title(Span::styled(
                        " MEMORY FRAGMENT DISCOVERED ",
                        Style::default().fg(rarity_color).add_modifier(Modifier::BOLD),
                    ));

                let text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Fragment {}", alert.number),
                        Style::default().fg(rarity_color).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        alert.attribution.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("[ {} ]", alert.rarity),
                        Style::default().fg(rarity_color),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Added to Chronicle Archives",
                        Style::default().fg(Color::Rgb(140, 140, 140)),
                    )),
                ];

                let paragraph = Paragraph::new(text)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Center);

                f.render_widget(paragraph, overlay_area);
            }

            // Dibuja las partículas ambientales directo en el buffer — sin widget, cell por cell
            if app.active_screen != ActiveScreen::Intro
                && app.active_screen != ActiveScreen::Onboarding
                && app.active_screen != ActiveScreen::Editor
                && app.ambient_effects_enabled
                && app.active_ambient_effect > 0
            {
                for p in &app.ambient_particles {
                    let px = p.x;
                    let py = p.y as u16;
                    if px < size.width && py < size.height {
                        let cell = f.buffer_mut().get_mut(px, py);
                        cell.set_char(p.symbol);
                        cell.set_fg(p.color);
                    }
                }
            }

            // Modal para elegir el tema visual — lista de temas legendarios con highlight del seleccionado
            if let questline::app::ModalType::ThemeSelect { ref choices, selected_idx } = app.modal_state {
                let overlay_area = questline::screens::intro::centered_rect(40, 30, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(" Choose Legendary Theme ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                    
                let mut list_items = Vec::new();
                for (idx, choice) in choices.iter().enumerate() {
                    let prefix = if idx == selected_idx { "> " } else { "  " };
                    let style = if idx == selected_idx {
                        Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(200, 200, 200))
                    };
                    list_items.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                        Span::styled(choice, style),
                    ]));
                }
                
                let p = Paragraph::new(list_items)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(p, overlay_area);
            }

            // Modal de celebración — se abre cuando el usuario logra algo chingón, puede traer ASCII art
            if let questline::app::ModalType::Celebration { ref title, ref details, ref ascii_art } = app.modal_state {
                let overlay_area = questline::screens::intro::centered_rect(65, 45, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .title(Span::styled(" CELEBRATION MOMENT ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                
                let mut content = vec![
                    Line::from(""),
                    Line::from(Span::styled(title.to_uppercase(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                ];
                
                if !ascii_art.is_empty() {
                    for line in ascii_art.lines() {
                        content.push(Line::from(Span::styled(line, Style::default().fg(Color::Cyan))));
                    }
                    content.push(Line::from(""));
                }
                
                for line in details.lines() {
                    content.push(Line::from(Span::styled(line, Style::default().fg(Color::White))));
                }
                content.push(Line::from(""));
                content.push(Line::from(Span::styled("Press [Enter] or [Space] or [Esc] to continue your journey...", Style::default().fg(Color::Rgb(140, 140, 140)))));
                
                let p = Paragraph::new(content)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(p, overlay_area);
            }

            // Modal de Chapter Complete — el más elaborado del juego, usa bordes manuales con spans
            if app.modal_state == questline::app::ModalType::ChapterComplete {
                // Overlay de tamaño fijo — la caja es 60 chars de ancho, 31 líneas de contenido
                let modal_w = 66u16;
                let modal_h = 35u16;
                let ox = size.width.saturating_sub(modal_w) / 2;
                let oy = size.height.saturating_sub(modal_h) / 2;
                let overlay_area = ratatui::layout::Rect {
                    x: ox,
                    y: oy,
                    width: modal_w.min(size.width),
                    height: modal_h.min(size.height),
                };
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(Color::Black)), overlay_area);

                // Cada línea debe medir exactamente 58 chars visuales para que el ║ derecho quede alineado
                // format!("  {:<56}", s) = 2 espacios + texto padded a 56 = 58 en total
                // Las bullets: 3 + 1 (•) + 54 = 58 — si no, la caja se ve chueca
                let gold      = Color::Rgb(212, 175, 55);
                let bdr       = Style::default().fg(gold);
                let hdr       = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
                let body      = Style::default().fg(Color::White);
                let em        = Style::default().fg(gold).add_modifier(Modifier::ITALIC);
                let rwd       = Style::default().fg(gold).add_modifier(Modifier::BOLD);
                let blt       = Style::default().fg(Color::Rgb(100, 220, 130));
                let dim       = Style::default().fg(Color::Rgb(140, 140, 140));

                // Closures para construir líneas con bordes — Style es Copy así que no hay borrow drama
                let bl = |text: &str, style: Style| -> Line<'static> {
                    Line::from(vec![
                        Span::styled("║", bdr),
                        Span::styled(format!("  {:<56}", text), style),
                        Span::styled("║", bdr),
                    ])
                };
                let pad = || -> Line<'static> {
                    Line::from(vec![
                        Span::styled("║", bdr),
                        Span::styled(format!("{:<58}", ""), body),
                        Span::styled("║", bdr),
                    ])
                };
                // Bullet: "   •" = 4 chars visuales, el resto 54 — todo ASCII puro para que cuadre
                let bul = |text: &str| -> Line<'static> {
                    Line::from(vec![
                        Span::styled("║", bdr),
                        Span::styled("   ", body),
                        Span::styled("•", blt),
                        Span::styled(format!(" {:<53}", text), body),
                        Span::styled("║", bdr),
                    ])
                };

                let lines: Vec<Line> = vec![
                    Line::from(Span::styled("╔══════════════════════════════════════════════════════════╗", bdr)),
                    Line::from(vec![
                        Span::styled("║", bdr),
                        Span::styled(format!("{:^58}", "CHAPTER COMPLETE"), hdr),
                        Span::styled("║", bdr),
                    ]),
                    Line::from(Span::styled("╠══════════════════════════════════════════════════════════╣", bdr)),
                    pad(),
                    Line::from(vec![
                        Span::styled("║", bdr),
                        Span::styled(format!("{:^58}", "The Notification Swarm Has Been Dispersed"), em),
                        Span::styled("║", bdr),
                    ]),
                    pad(),
                    bl("Across the Realm, heroes completed quests, wrote", body),
                    bl("scrolls, tended their Zen Trees, honored their", body),
                    bl("rituals, and returned day after day.", body),
                    pad(),
                    bl("The Swarm fed upon distraction.", body),
                    bl("It was defeated by persistence.", body),
                    pad(),
                    bl("No single hero saved the Realm.", body),
                    bl("Thousands simply continued their work.", body),
                    pad(),
                    bl("The Chronicle records this victory.", body),
                    pad(),
                    bl("Rewards Unlocked:", rwd),
                    pad(),
                    bul("5 000 Experience Points"),
                    bul("World Lore: The Fate of the Notification Sprites"),
                    bul("Memory Fragment #001"),
                    bul("Chapter Record Added to History"),
                    pad(),
                    bl("Yet even as the final Sprite vanished...", em),
                    pad(),
                    bl("Something stirred beyond the horizon.", em),
                    pad(),
                    bl("The Chronicle turns to the next page.", body),
                    pad(),
                    Line::from(Span::styled("╚══════════════════════════════════════════════════════════╝", bdr)),
                    Line::from(""),
                    Line::from(Span::styled("Press  Enter / Space / Esc  to continue your journey", dim)),
                ];

                let p = Paragraph::new(lines)
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(p, overlay_area);
            }

            // Search Everywhere — busca en proyectos, tareas, notas y demás de un jalón
            if let questline::app::ModalType::SearchEverywhere { ref query, selected_idx, ref results } = app.modal_state {
                let overlay_area = questline::screens::intro::centered_rect(70, 45, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(" Search Everywhere ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Search Input
                        Constraint::Min(1),    // Results list
                    ])
                    .margin(1)
                    .split(overlay_area);

                // Draw input box
                let input_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Type to search... ");
                let input_p = Paragraph::new(format!("> {}", query))
                    .block(input_block)
                    .style(Style::default().fg(Color::White));
                f.render_widget(input_p, inner_layout[0]);

                // Draw results list
                let mut list_items = Vec::new();
                if results.is_empty() {
                    list_items.push(ListItem::new(Line::from(vec![
                        Span::styled("  No results found. Try a different query.", Style::default().fg(Color::Rgb(140, 140, 140)))
                    ])));
                } else {
                    for (idx, r) in results.iter().enumerate() {
                        let is_selected = idx == selected_idx;
                        let prefix = if is_selected { "> " } else { "  " };
                        let style = if is_selected {
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Rgb(200, 200, 200))
                        };
                        let type_style = if is_selected {
                            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.secondary)
                        };

                        let item_line = Line::from(vec![
                            Span::styled(prefix, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("[{}] ", r.result_type.label()), type_style),
                            Span::styled(&r.title, style),
                            Span::styled(format!(" - {}", r.details), Style::default().fg(Color::Rgb(140, 140, 140))),
                        ]);

                        let list_item = if is_selected {
                            ListItem::new(item_line).style(Style::default().bg(Color::Rgb(30, 30, 40)))
                        } else {
                            ListItem::new(item_line)
                        };
                        list_items.push(list_item);
                    }
                }

                let results_list = List::new(list_items)
                    .block(Block::default().borders(Borders::ALL).title(" Results ").border_style(Style::default().fg(Color::DarkGray)));
                f.render_widget(results_list, inner_layout[1]);
                f.render_widget(block, overlay_area);
            }

            // Command palette — Ctrl+P abre esto, es básicamente el centro de control del app
            if let questline::app::ModalType::CommandPalette { ref query, selected_idx, ref actions } = app.modal_state {
                let overlay_area = questline::screens::intro::centered_rect(70, 45, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(" Command Palette ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Search Input
                        Constraint::Min(1),    // Actions list
                    ])
                    .margin(1)
                    .split(overlay_area);

                // Draw input box
                let input_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Search commands... ");
                let input_p = Paragraph::new(format!("> {}", query))
                    .block(input_block)
                    .style(Style::default().fg(Color::White));
                f.render_widget(input_p, inner_layout[0]);

                // Draw actions list
                let mut list_items = Vec::new();
                if actions.is_empty() {
                    list_items.push(ListItem::new(Line::from(vec![
                        Span::styled("  No commands match the query.", Style::default().fg(Color::Rgb(140, 140, 140)))
                    ])));
                } else {
                    for (idx, act) in actions.iter().enumerate() {
                        let is_selected = idx == selected_idx;
                        let prefix = if is_selected { "> " } else { "  " };
                        let name_style = if is_selected {
                            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        let shortcut_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

                        let item_line = Line::from(vec![
                            Span::styled(prefix, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                            Span::styled(act.name, name_style),
                            Span::styled(format!(" - {}", act.description), Style::default().fg(Color::Rgb(200, 200, 200))),
                            Span::styled(if act.shortcut.is_empty() { "".to_string() } else { format!("  [{}]", act.shortcut) }, shortcut_style),
                        ]);

                        let list_item = if is_selected {
                            ListItem::new(item_line).style(Style::default().bg(Color::Rgb(30, 30, 40)))
                        } else {
                            ListItem::new(item_line)
                        };
                        list_items.push(list_item);
                    }
                }

                let actions_list = List::new(list_items)
                    .block(Block::default().borders(Borders::ALL).title(" Commands ").border_style(Style::default().fg(Color::DarkGray)))
                    .highlight_symbol("")
                    .highlight_style(Style::default());
                let mut list_state = ratatui::widgets::ListState::default();
                if !actions.is_empty() {
                    list_state.select(Some(selected_idx));
                }
                f.render_stateful_widget(actions_list, inner_layout[1], &mut list_state);
                f.render_widget(block, overlay_area);
            }

            // Picker de proyecto para una acción — filtra los activos (no archivados, no completados)
            if let questline::app::ModalType::SelectProjectForAction { action_id, selected_idx } = app.modal_state {
                let projects: Vec<&questline::models::Project> = app.projects.iter()
                    .filter(|p| !p.archived && !p.completed)
                    .collect();
                let overlay_area = questline::screens::intro::centered_rect(55, 50, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let title = if action_id == "create_task" { " Select Project — New Task " } else { " Select Project — New Note " };
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

                let inner = block.inner(overlay_area);
                f.render_widget(block, overlay_area);

                let chunks = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([ratatui::layout::Constraint::Min(1), ratatui::layout::Constraint::Length(1)])
                    .split(inner);

                let items: Vec<ListItem> = if projects.is_empty() {
                    vec![ListItem::new(Span::styled("  No active projects found.", Style::default().fg(Color::Rgb(140, 140, 140))))]
                } else {
                    projects.iter().enumerate().map(|(i, p)| {
                        let is_sel = i == selected_idx;
                        let style = if is_sel {
                            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD).bg(Color::Rgb(30, 30, 40))
                        } else {
                            Style::default().fg(Color::White)
                        };
                        let prefix = if is_sel { "> " } else { "  " };
                        ListItem::new(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(theme.primary)),
                            Span::styled(p.name.clone(), style),
                        ]))
                    }).collect()
                };
                let mut list_state = ratatui::widgets::ListState::default();
                if !projects.is_empty() { list_state.select(Some(selected_idx)); }
                let list = List::new(items)
                    .highlight_symbol("")
                    .highlight_style(Style::default());
                f.render_stateful_widget(list, chunks[0], &mut list_state);

                let help = Paragraph::new(Span::styled("  ↑/↓ select  |  Enter open  |  Esc cancel", Style::default().fg(Color::Rgb(140, 140, 140))));
                f.render_widget(help, chunks[1]);
            }

            // Modal de confirmación de salida — con fogata animada y una quote, pura vibra RPG
            if let questline::app::ModalType::QuitConfirm { ref quote } = app.modal_state {
                // Popup de altura fija para que no se pase en terminales pequeñas
                let overlay_area = centered_rect_fixed_height(60, 13, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(" Campfire Rest ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

                // 4 frames de llama ASCII, cambian cada 6 ticks (300ms) — animación barata pero chida
                let frame_idx = (app.quit_confirm_ticks / 6) % 4;
                let flame_lines = match frame_idx {
                    0 => vec![
                        Line::from(Span::styled("      )  .  ", Style::default().fg(Color::LightRed))),
                        Line::from(Span::styled("    (      )  ", Style::default().fg(Color::LightYellow))),
                        Line::from(Span::styled("   (  .  )  ) ", Style::default().fg(Color::Red))),
                    ],
                    1 => vec![
                        Line::from(Span::styled("      .  (  ", Style::default().fg(Color::LightRed))),
                        Line::from(Span::styled("    )      (  ", Style::default().fg(Color::LightYellow))),
                        Line::from(Span::styled("   (  )  .  ( ", Style::default().fg(Color::Red))),
                    ],
                    2 => vec![
                        Line::from(Span::styled("      (  )  ", Style::default().fg(Color::LightRed))),
                        Line::from(Span::styled("    (      )  ", Style::default().fg(Color::LightYellow))),
                        Line::from(Span::styled("   )  .  (  ) ", Style::default().fg(Color::Red))),
                    ],
                    _ => vec![
                        Line::from(Span::styled("      .  )  ", Style::default().fg(Color::LightRed))),
                        Line::from(Span::styled("    (      (  ", Style::default().fg(Color::LightYellow))),
                        Line::from(Span::styled("   (  (  )  ) ", Style::default().fg(Color::Red))),
                    ],
                };

                let mut lines = vec![];
                lines.extend(flame_lines);
                lines.push(Line::from(Span::styled("  ____________", Style::default().fg(Color::Red))));
                lines.push(Line::from(Span::styled("  \\__________/", Style::default().fg(Color::Rgb(140, 140, 140)))));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(format!("\"{}\"", quote), Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC))));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("Are you sure you want to quit? [Y/N]", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))));

                let p = Paragraph::new(lines)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Center)
                    .wrap(ratatui::widgets::Wrap { trim: false });
                
                f.render_widget(p, overlay_area);
            }

            // Confirm de archivar proyecto — pregunta antes de mandarlo al baúl de los recuerdos
            if let questline::app::ModalType::ConfirmArchiveProject { ref project_name, .. } = app.modal_state {
                let overlay_area = centered_rect_fixed_height(55, 9, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .title(Span::styled(
                        " [!] Archive Realm ",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ));

                let lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Realm: ", Style::default().fg(Color::Rgb(140, 140, 140))),
                        Span::styled(project_name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  This realm will be moved to the Archives.",
                        Style::default().fg(Color::Rgb(200, 200, 200)),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Archive this realm?  [Y] Yes   [N] No / Esc",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )),
                ];

                let p = Paragraph::new(lines)
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false });
                f.render_widget(p, overlay_area);
            }

            // Confirm de borrar proyecto — advertencia roja porque esto no tiene vuelta atrás
            if let questline::app::ModalType::ConfirmDeleteProject { ref project_name, .. } = app.modal_state {
                let overlay_area = centered_rect_fixed_height(55, 10, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    .title(Span::styled(
                        " Slay Realm Permanently ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ));

                let lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Realm: ", Style::default().fg(Color::Rgb(140, 140, 140))),
                        Span::styled(project_name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  WARNING: This action is PERMANENT and cannot be undone!",
                        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        "  All tasks, notes and milestones will be lost forever.",
                        Style::default().fg(Color::Rgb(200, 200, 200)),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Slay this realm forever?  [Y] Yes   [N] No / Esc",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                ];

                let p = Paragraph::new(lines)
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false });
                f.render_widget(p, overlay_area);
            }

            // Confirm de borrar codex — las notas adentro no se pierden, solo quedan sin grupo
            if let questline::app::ModalType::ConfirmDeleteCodex { ref codex_name, .. } = app.modal_state {
                let overlay_area = centered_rect_fixed_height(55, 9, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .title(Span::styled(
                        " Delete Codex ",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ));

                let lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  Codex: ", Style::default().fg(Color::Rgb(140, 140, 140))),
                        Span::styled(codex_name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Scrolls inside will become ungrouped, not deleted.",
                        Style::default().fg(Color::Rgb(200, 200, 200)),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Delete this codex?  [Y] Yes   [N] No / Esc",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )),
                ];

                let p = Paragraph::new(lines)
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false });
                f.render_widget(p, overlay_area);
            }

            // Avisa que hay una versión nueva disponible — Y para instalar directo desde el app
            if let questline::app::ModalType::UpdateAvailable { ref latest_version } = app.modal_state {
                let overlay_area = centered_rect_fixed_height(62, 12, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let install_cmd = match std::env::consts::OS {
                    "windows" => "irm https://raw.githubusercontent.com/gibranlp/Questline-cli/main/server/install.ps1 | iex",
                    _ => "curl -fsSL https://raw.githubusercontent.com/gibranlp/Questline-cli/main/server/install.sh | bash",
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    .title(Span::styled(
                        " * Update Available ",
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));

                let lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  New version : ", Style::default().fg(Color::Rgb(140, 140, 140))),
                        Span::styled(latest_version.clone(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("  Installed   : ", Style::default().fg(Color::Rgb(140, 140, 140))),
                        Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::Rgb(200, 200, 200))),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Install now? [Y] Yes — exit & update    [N] Skip",
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  $ ", Style::default().fg(Color::Rgb(140, 140, 140))),
                        Span::styled(install_cmd, Style::default().fg(Color::Cyan)),
                    ]),
                ];

                let p = Paragraph::new(lines)
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: false });
                f.render_widget(p, overlay_area);
            }

            // Ayuda de teclado contextual — los shortcuts cambian según la pantalla activa
            if let questline::app::ModalType::KeyboardHelp = app.modal_state {
                let overlay_area = questline::screens::intro::centered_rect(75, 35, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(" Keyboard Shortcuts (Esc/Enter/Space to close) ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

                let mut lines = vec![
                    Line::from(Span::styled("Global Shortcuts:", Style::default().fg(theme.primary).add_modifier(Modifier::UNDERLINED | Modifier::BOLD))),
                    Line::from("  Ctrl+P / : / Ctrl+K / F1  Command Palette (Fuzzy Navigation & Commands)"),
                    Line::from("  ?            Show Keyboard Shortcuts Help (Context-Sensitive)"),
                    Line::from("  D/P/H/L/G/M/S Switch tabs directly"),
                    Line::from("  Tab          Cycle input focus/fields"),
                    Line::from("  Shift+Tab    Cycle fields backwards"),
                    Line::from("  Ctrl+C       Force Quit Application"),
                    Line::from(""),
                ];

                // Los shortcuts de contexto cambian según qué pantalla y hasta qué tab está activo
                lines.push(Line::from(Span::styled(
                    format!("Context Shortcuts (Active Screen: {:?}):", app.active_screen),
                    Style::default().fg(theme.secondary).add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
                )));

                match app.active_screen {
                    ActiveScreen::Dashboard => {
                        lines.push(Line::from("  w            Water the Zen Tree (Growth & XP)"));
                        lines.push(Line::from("  f            Quick start Focus Session"));
                        lines.push(Line::from("  m            Go to Music Screen"));
                    }
                    ActiveScreen::Projects => {
                        lines.push(Line::from("  n            Create a New Project"));
                        lines.push(Line::from("  e            Edit selected Project"));
                        lines.push(Line::from("  a            Archive selected Project"));
                        lines.push(Line::from("  c            Mark selected Project as Complete"));
                    }
                    ActiveScreen::Workspace => {
                        lines.push(Line::from("  Workspace Tabs: Tasks (0) | Notes (1) | Journal (2) | Milestones (3)"));
                        match app.workspace_tab_idx {
                            0 => {
                                lines.push(Line::from("  n            Create New Task"));
                                lines.push(Line::from("  e            Edit selected Task"));
                                lines.push(Line::from("  d            Delete selected Task"));
                                lines.push(Line::from("  c            Toggle Task completed status (Earn XP!)"));
                                lines.push(Line::from("  s            Cycle Task sorting method (DueDate -> Priority -> CreatedDate)"));
                                lines.push(Line::from("  f            Toggle Tasks filter (All -> Incomplete -> Completed)"));
                            }
                            1 => {
                                lines.push(Line::from("  n            Create New Note"));
                                lines.push(Line::from("  e            Edit/Open selected Note in Editor"));
                                lines.push(Line::from("  d            Delete selected Note"));
                                lines.push(Line::from("  s            Open Note Sharing modal (Multi-device Sync)"));
                            }
                            2 => {
                                lines.push(Line::from("  n            Create New Journal Entry (Reflections)"));
                                lines.push(Line::from("  v            Toggle Journal Visibility (Public / Personal)"));
                            }
                            3 => {
                                lines.push(Line::from("  m            Create New Milestone for this project"));
                                lines.push(Line::from("  c            Toggle Milestone completed status"));
                            }
                            _ => {}
                        }
                    }
                    ActiveScreen::Fellowship => {
                        lines.push(Line::from("  p            Switch to Fellowship Companions Presence list"));
                        lines.push(Line::from("  m            Switch to Project Chronicle Chat Messages"));
                        lines.push(Line::from("  i            Switch to Fellowship Invitations tab"));
                        lines.push(Line::from("  a            Switch to Activity Log tab"));
                        lines.push(Line::from("  n            Write/Post a new chat message to the Chronicle"));
                    }
                    ActiveScreen::Soundscapes => {
                        lines.push(Line::from("  p            Pause current ambient soundscape"));
                        lines.push(Line::from("  s            Stop playing soundscape"));
                        lines.push(Line::from("  n            Cycle through available Soundscapes"));
                        lines.push(Line::from("  + / *        Increase audio volume"));
                        lines.push(Line::from("  -            Decrease audio volume"));
                    }

                    ActiveScreen::Legends => {
                        lines.push(Line::from("  j / k / Up/Down  Scroll and inspect your collection of Legendary Relics"));
                    }
                    ActiveScreen::GreatChronicle => {
                        lines.push(Line::from("  ↑ / ↓        Scroll the realm achievement feed"));
                        lines.push(Line::from("  R            Refresh — pull latest entries from the server"));
                        lines.push(Line::from("  P            Cycle privacy level (off / everything)"));
                        lines.push(Line::from("  X            Replay the Story So Far & Chapter One prologue"));
                    }
                    _ => {
                        lines.push(Line::from("  Esc          Return to parent view"));
                    }
                }

                let p = Paragraph::new(lines)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Left)
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .style(Style::default().fg(Color::White));
                
                f.render_widget(p, overlay_area);
            }

            // Modal de bug report — manda feedback al servidor de Questline sin salir del app
            if let Some(ref modal) = app.bug_report_modal {
                let overlay_area = centered_rect_fixed_height(70, 22, size);
                f.render_widget(Clear, overlay_area);
                f.render_widget(Block::default().style(Style::default().bg(theme.background)), overlay_area);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                    .title(Span::styled(
                        " Send Report to Questline ",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ));

                let inner = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // type selector
                        Constraint::Min(1),    // description
                        Constraint::Length(3), // footer
                    ])
                    .margin(1)
                    .split(overlay_area);

                // Selector de tipo: Bug / Feature / Feedback — resaltado con bg del tema
                let types = [
                    questline::app::ReportType::Bug,
                    questline::app::ReportType::Feature,
                    questline::app::ReportType::Feedback,
                ];
                let type_spans: Vec<Span> = types.iter().flat_map(|t| {
                    let selected = *t == modal.report_type;
                    let style = if selected {
                        Style::default().fg(Color::Black).bg(theme.primary).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(140, 140, 140))
                    };
                    vec![Span::styled(format!(" {} ", t.label()), style), Span::raw("  ")]
                }).collect();
                let type_p = Paragraph::new(Line::from(type_spans))
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Type  [←/→] "));
                f.render_widget(type_p, inner[0]);

                // El área de descripción muestra el estado del envío si ya se mandó el reporte
                let desc_display = format!("{}_", modal.description);
                let desc_content = if let Some(ref status) = modal.status {
                    let style = if status.starts_with("Report sent") {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Red)
                    };
                    Paragraph::new(Span::styled(status.as_str(), style))
                        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
                        .wrap(ratatui::widgets::Wrap { trim: false })
                } else {
                    Paragraph::new(desc_display.as_str())
                        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.primary)).title(" Description "))
                        .wrap(ratatui::widgets::Wrap { trim: false })
                        .style(Style::default().fg(Color::White))
                };
                f.render_widget(desc_content, inner[1]);

                // Footer
                let footer_text = if modal.status.is_some() {
                    "Press any key to close"
                } else {
                    "[Ctrl+S] Send  |  [Esc] Cancel  |  [Enter] New line"
                };
                let footer_p = Paragraph::new(Span::styled(footer_text, Style::default().fg(Color::Rgb(140, 140, 140))))
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(footer_p, inner[2]);

                f.render_widget(block, overlay_area);
            }
        })?;

    }

    // Para el audio limpiamente antes de soltar la terminal
    app.audio_player.stop();

    // Restaura la terminal a su estado normal — sin esto la consola queda cagada
    print!("\x1b]111\x07");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Si el usuario aceptó el update, corre el installer después de limpiar la terminal
    if app.run_installer_on_exit {
        println!("\n  Running installer...\n");
        match std::env::consts::OS {
            "windows" => {
                let _ = std::process::Command::new("powershell")
                    .args(["-Command", "irm https://raw.githubusercontent.com/gibranlp/Questline-cli/main/server/install.ps1 | iex"])
                    .status();
            }
            _ => {
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg("curl -fsSL https://raw.githubusercontent.com/gibranlp/Questline-cli/main/server/install.sh | bash")
                    .status();
            }
        }
    }

    Ok(())
}
