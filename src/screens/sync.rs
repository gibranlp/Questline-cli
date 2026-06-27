// ─────────────────────────────────────────────────────────────────────────────
// screens/sync.rs — configuración de sync y gestión de identidad multi-dispositivo
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{App, ModalType};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

// pantalla gorda de sync — aquí va todo: identidad, dispositivos, stats y los modales
// divide en dos columnas, izquierda config/stats, derecha devices/snapshots/progresión RPG
pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let size = area;
    let accent_color = theme.primary;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // jalamos cuántos cambios no se han mandado al servidor todavía
    let pending_changes = app.db.get_pending_sync_logs().map(|l| l.len()).unwrap_or(0);
    let last_sync_time = app
        .db
        .get_setting("last_sync")
        .unwrap_or(None)
        .unwrap_or_else(|| "Never".to_string());

    let mut left_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "   === Identity File ===",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   User UUID:  ", Style::default().fg(theme.muted)),
            Span::styled(
                app.identity.user_uuid.to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Public Key (Share Key):", Style::default().fg(theme.muted)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("   {}", app.identity.public_key),
                Style::default().fg(Color::LightCyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Created At: ", Style::default().fg(theme.muted)),
            Span::styled(&app.identity.created_at, Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "   === Configuration ===",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   Server URL: ", Style::default().fg(theme.muted)),
            Span::styled(
                &app.server_url,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  [u] edit", Style::default().fg(theme.muted)),
        ]),
        Line::from(vec![
            Span::styled("   Auto Sync:  ", Style::default().fg(theme.muted)),
            Span::styled(
                if app.auto_sync { "Enabled" } else { "Disabled" },
                Style::default()
                    .fg(if app.auto_sync {
                        theme.success
                    } else {
                        theme.danger
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  [a] toggle", Style::default().fg(theme.muted)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "   === Chronicle Sync State ===",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("   Last Sync:  ", Style::default().fg(theme.muted)),
            Span::styled(
                last_sync_time,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Pending:    ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} changes", pending_changes),
                Style::default()
                    .fg(if pending_changes > 0 {
                        theme.warning
                    } else {
                        theme.success
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        {
            // el CodeWarlock gana XP extra por sincronizar — bonus de clase, chido
            let is_warlock = app.user.as_ref().map(|u| u.class == crate::models::ClassType::CodeWarlock).unwrap_or(false);
            let mut status_spans = vec![
                Span::styled("   Status:     ", Style::default().fg(theme.muted)),
                Span::styled(app.sync_status_msg.clone(), Style::default().fg(Color::White)),
            ];
            if is_warlock && app.last_sync_warlock_xp > 0 {
                status_spans.push(Span::styled(
                    format!("  +{} XP", app.last_sync_warlock_xp),
                    Style::default().fg(accent_color).add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(status_spans)
        },
        Line::from(""),
        Line::from(vec![Span::styled(
            "   >>> Press [s] or [Enter] to Sync Now <<<",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "   [b] Backup | [r] Restore | [c] Copy Share Key | [e] Export Profile | [i] Restore Identity",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    // si hubo conflictos al sincronizar los mostramos aquí abajo — no manches, a veces pasa
    if !app.sync_conflicts.is_empty() {
        left_text.push(Line::from(""));
        left_text.push(Line::from(vec![Span::styled(
            "   === Resolved Conflicts ===",
            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
        )]));
        for conflict in app.sync_conflicts.iter().take(4) {
            left_text.push(Line::from(vec![Span::styled(
                format!("    * {}", conflict),
                Style::default().fg(theme.text),
            )]));
        }
    }

    let left_panel = Paragraph::new(left_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Questline Sync Node Settings ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );

    // columna izquierda: arriba config del nodo, abajo las estadísticas de productividad
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(22), Constraint::Min(8)])
        .split(chunks[0]);

    f.render_widget(left_panel, left_chunks[0]);

    // pues hay que mostrar las estadísticas de trabajo del héroe — tasks, notas, journals, etc.
    let stats = app.db.get_statistics().unwrap();
    let stats_left_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Quests (Tasks) Solved:  ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} completed", stats.tasks_completed),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Scrolls (Notes) Written: ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} created", stats.notes_created),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Chronicles (Journals):   ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} entries", stats.journal_entries),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Realms (Projects) Begun: ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} started", stats.projects_created),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Realms Fully Conquered:   ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} completed", stats.projects_completed),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Project Milestones Met:   ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} milestones", stats.milestones_completed),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let stats_left_panel = Paragraph::new(stats_left_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Chronicles of Labor (Productivity) ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(stats_left_panel, left_chunks[1]);

    // columna derecha: 3 secciones apiladas — dispositivos, snapshots y progresión RPG completa
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Device Mesh Registrations
            Constraint::Length(8),  // Revision Snapshots Logs
            Constraint::Min(12),    // RPG Character Progression Sheet
        ])
        .split(chunks[1]);

    // lista de todos los dispositivos registrados en el mesh — marcamos cuál es el actual
    let devices = app.db.get_devices().unwrap_or_default();
    let mut right_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "   Known Devices in Sync Mesh:",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    if devices.is_empty() {
        right_text.push(Line::from("   No registered devices."));
    } else {
        for (dev_id, dev_name, created, last_sync) in &devices {
            let is_current = dev_id == &app.device_id;
            let current_marker = if is_current { " (Current Device)" } else { "" };

            right_text.push(Line::from(vec![
                Span::styled(
                    format!("   • {}", dev_name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(current_marker, Style::default().fg(accent_color)),
            ]));

            right_text.push(Line::from(vec![
                Span::styled("     ID:      ", Style::default().fg(theme.muted)),
                Span::styled(dev_id, Style::default().fg(theme.muted)),
            ]));

            right_text.push(Line::from(vec![
                Span::styled("     Created: ", Style::default().fg(theme.muted)),
                Span::styled(created, Style::default().fg(theme.text)),
            ]));

            right_text.push(Line::from(vec![
                Span::styled("     Last:    ", Style::default().fg(theme.muted)),
                Span::styled(
                    last_sync.as_deref().unwrap_or("Never"),
                    Style::default().fg(if last_sync.is_some() {
                        theme.success
                    } else {
                        theme.muted
                    }),
                ),
            ]));
            right_text.push(Line::from(""));
        }
    }

    let right_panel = Paragraph::new(right_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Device Mesh Registrations ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(right_panel, right_chunks[0]);

    // historial de snapshots — parseamos el JSON del content para sacar el título, si no "Untitled"
    let revisions = app.db.get_recent_revisions().unwrap_or_default();
    let mut rev_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "   Chronicle Revision Snapshots (History):",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    if revisions.is_empty() {
        rev_text.push(Line::from("   No revision snapshots created yet."));
    } else {
        for (_id, et, content, num, ts) in &revisions {
            // intentamos title, si no name, si no pues "Untitled" — así de simple
            let title = if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
                v["title"]
                    .as_str()
                    .or_else(|| v["name"].as_str())
                    .unwrap_or("Untitled")
                    .to_string()
            } else {
                "Untitled".to_string()
            };

            // formateamos el timestamp a hora local — si falla el parse usamos el string crudo
            let time_formatted = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                dt.with_timezone(&chrono::Local).format("%H:%M:%S").to_string()
            } else {
                ts.clone()
            };

            rev_text.push(Line::from(vec![
                Span::styled(
                    format!("   • v{} ", num),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[{}] ", et.to_uppercase()),
                    Style::default().fg(theme.text),
                ),
                Span::styled(title, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" ({})", time_formatted),
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
    }

    let rev_panel = Paragraph::new(rev_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Revision Snapshots Logs ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(rev_panel, right_chunks[1]);

    // el panel más grueso — aquí van todas las métricas RPG: focus, sidequests, tree, streak, etc.
    let stats_right_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Deep Work Duration:    ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{:.1} focus hours", stats.focus_hours),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({} sessions)", stats.sessions_completed),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Sidequests Completed:  ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} completions", stats.rituals_completed),
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Tree Growth Progress:   ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} growth points", stats.tree_growth),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Streak (Active / Best):  ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} days", stats.current_streak),
                Style::default()
                    .fg(Color::Rgb(249, 115, 22))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {} days record", stats.best_streak),
                Style::default().fg(theme.warning),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Trophy Achievements:    ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{}/14 unlocked", stats.achievements_unlocked),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Total Experience Gain:  ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} XP earned", stats.total_xp_earned),
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Daily Averages:         ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{:.1} Tasks/day", stats.avg_tasks_per_day),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  |  {:.1} XP/day", stats.avg_xp_per_day),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Sync Chronology Events: ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} syncs", stats.sync_count),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  |  {} backups", stats.backup_count),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Devices Registered:     ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} nodes", stats.devices_connected),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  |  {} conflicts", stats.conflict_count),
                Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "   Last Chronicled Restore:",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                &stats.last_restore,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let stats_right_panel = Paragraph::new(stats_right_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " RPG Character Progression Sheet ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(stats_right_panel, right_chunks[2]);

    // modal para editar la URL del servidor — overlay encima de todo, limpia el area con Clear
    if let ModalType::EditServerUrl { input } = &app.modal_state {
        let area = centered_rect(50, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Edit Sync Server URL ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Input box
                Constraint::Min(1),    // Help footer
            ])
            .split(block.inner(area));

        f.render_widget(block, area);

        let input_p = Paragraph::new(format!("  {}", input)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent_color))
                .title(" Server URL "),
        );
        f.render_widget(input_p, inner_layout[1]);

        let help_p = Paragraph::new("  [Enter] save  |  [Esc] cancel")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help_p, inner_layout[2]);
    }

    // modal de exportar perfil — muestra el transfer code partido a la mitad para que quepa
    if let ModalType::ExportProfile { transfer_code } = &app.modal_state {
        let area = centered_rect(70, 40, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Export Profile — Transfer Code ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let label = Paragraph::new("  Your Transfer Code (paste this on the new device):")
            .style(Style::default().fg(theme.muted));
        f.render_widget(label, inner_layout[1]);

        // el transfer code es largo, lo partimos en dos líneas para que se vea bien
        let half = transfer_code.len() / 2;
        let (line1, line2) = transfer_code.split_at(half);
        let code_p = Paragraph::new(vec![
            Line::from(Span::styled(format!("  {}", line1), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(format!("  {}", line2), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD))),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent_color)),
        );
        f.render_widget(code_p, inner_layout[3]);

        let warn = Paragraph::new("  Keep this code secret — it contains your private signing key.")
            .style(Style::default().fg(theme.danger));
        f.render_widget(warn, inner_layout[4]);

        let help = Paragraph::new("  [c] Copy to Clipboard  |  [Esc] Close")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help, inner_layout[5]);
    }

    // modal para restaurar identidad — órale, cuidado con este, reemplaza la llave actual
    if let ModalType::RestoreIdentity { input } = &app.modal_state {
        let area = centered_rect(60, 35, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Restore Identity — Paste Transfer Code ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);

        let desc = Paragraph::new("  Paste the Transfer Code from your other device.\n  Your current DB will be backed up first.")
            .style(Style::default().fg(theme.muted));
        f.render_widget(desc, inner_layout[1]);

        let input_p = Paragraph::new(format!("  {}", input))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent_color))
                    .title(" Transfer Code "),
            );
        f.render_widget(input_p, inner_layout[3]);

        let warn = Paragraph::new("  WARNING: This will replace your current identity key.")
            .style(Style::default().fg(theme.danger));
        f.render_widget(warn, inner_layout[4]);

        let help = Paragraph::new("  [Enter] Restore  |  [Esc] Cancel")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help, inner_layout[5]);
    }
}

// helper clásico para centrar cualquier popup — divide el área en 3x3 y devuelve el centro
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
