// ─────────────────────────────────────────────────────────────────────────────
// fellowship.rs — la pantalla del equipo: chat, presencia y proyectos compartidos
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{extract_url, App, ModalType};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

// La función principal — pinta toda la pantalla de fellowship, tabs y modales incluidos
// Órale, aquí vive todo: proyectos compartidos, chat, compañeros y búsqueda
pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let size = area;
    let accent_color = theme.primary;

    // Layout horizontal: 30% lista de proyectos, 70% panel derecho con tabs
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(size);

    // Panel izquierdo — solo muestra proyectos que ya están compartidos
    let shared_projects: Vec<_> = app.projects.iter().filter(|p| p.is_shared).collect();

    let mut proj_lines = vec![Line::from("")];
    if shared_projects.is_empty() {
        proj_lines.push(Line::from(" No shared projects yet."));
        proj_lines.push(Line::from(" Invite a companion and "));
        proj_lines.push(Line::from(" share your adventure [v]"));
    } else {
        for (idx, proj) in shared_projects.iter().enumerate() {
            let is_selected = idx == app.selected_fellowship_project_idx;
            let marker = if is_selected { " > " } else { "   " };
            let style = if is_selected {
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Contamos cuántos miembros están online en este proyecto para el badge lateral
            let members = app.db.get_presence_for_project(&proj.id.to_string()).unwrap_or_default();
            let online_n = members.iter().filter(|m| m.3).count();
            let online_badge = if online_n > 0 {
                format!("● {}", online_n)
            } else {
                String::new()
            };

            proj_lines.push(Line::from(vec![
                Span::styled(marker, Style::default().fg(accent_color)),
                Span::styled(proj.name.clone(), style),
                Span::styled(
                    if online_n > 0 { format!("  {}", online_badge) } else { String::new() },
                    Style::default().fg(theme.success).add_modifier(Modifier::BOLD),
                ),
            ]));

            let owner_name = proj.owner_username.as_deref().unwrap_or("Unknown");
            proj_lines.push(Line::from(vec![Span::styled(
                format!("     Owner: {}", owner_name),
                Style::default().fg(theme.muted),
            )]));
            proj_lines.push(Line::from(""));
        }
    }

    let left_focused = app.fellowship_focus_left;
    let left_border_color = if left_focused { accent_color } else { theme.border };
    let left_block = Paragraph::new(proj_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(left_border_color))
            .title(Span::styled(
                " Shared Fellowship Projects ",
                Style::default()
                    .fg(if left_focused { theme.warning } else { Color::Gray })
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(left_block, chunks[0]);

    // Columna derecha: barra de tabs arriba, panel activo en medio, footer de controles abajo
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab navigation bar
            Constraint::Min(10),   // Active Tab Panel
            Constraint::Length(3), // Controls/Actions instructions footer
        ])
        .split(chunks[1]);

    // Barra de tabs — resalta el seleccionado con color warning y fondo de panel
    let tabs_titles = [
        " [c] Chat ",
        " [i] Invites ",
        " [p] Companions ",
        " [a] Activity ",
        " [/] Search ",
    ];
    let mut tab_spans = Vec::new();
    for (idx, title) in tabs_titles.iter().enumerate() {
        let is_selected = idx == app.selected_fellowship_tab;
        let style = if is_selected {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
                .bg(theme.panel)
        } else {
            Style::default().fg(theme.text)
        };
        tab_spans.push(Span::styled(*title, style));
        if idx < tabs_titles.len() - 1 {
            tab_spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        }
    }
    let tab_line = Line::from(tab_spans);
    let tab_p = Paragraph::new(tab_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(tab_p, right_chunks[0]);

    // Aquí se decide qué pintar según el tab activo — cada rama es una pantalla distinta
    match app.selected_fellowship_tab {
        0 => {
            // Tab de chat — si no hay proyectos compartidos muestra notificaciones en su lugar
            if shared_projects.is_empty() {
                let sub_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Select or join shared project description
                        Constraint::Min(5),    // Notification Center list
                    ])
                    .split(right_chunks[1]);

                let desc_p = Paragraph::new("\n   Select or join a shared project to view its Chronicle.")
                    .style(Style::default().fg(theme.text));
                f.render_widget(desc_p, sub_chunks[0]);

                let notifications = app.db.get_notifications().unwrap_or_default();
                let mut notif_lines = vec![Line::from("")];

                if notifications.is_empty() {
                    notif_lines.push(Line::from("   No notifications logged yet."));
                } else {
                    // Cada notif es una tupla: (id, tipo, titulo, cuerpo, ..., leida, timestamp)
                    for (idx, notif) in notifications.iter().enumerate() {
                        let is_selected = idx == app.selected_notification_idx;
                        let marker = if is_selected { "  > " } else { "    " };
                        let style = if is_selected {
                            Style::default()
                                .fg(theme.warning)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };

                        // notif.5 es el flag de "ya leído" — true = leído, false = sin leer
                        let read_marker = if notif.5 {
                            Span::styled(" [Read] ", Style::default().fg(theme.muted))
                        } else {
                            Span::styled(
                                " [Unread] ",
                                Style::default()
                                    .fg(theme.success)
                                    .add_modifier(Modifier::BOLD),
                            )
                        };

                        // Color distinto por tipo de notificación — cada una tiene su onda
                        let notif_type_style = match notif.1.as_str() {
                            "mention" => Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                            "invitation" => Style::default()
                                .fg(theme.warning)
                                .add_modifier(Modifier::BOLD),
                            "project_update" => Style::default().fg(Color::LightCyan),
                            "achievement" => Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                            "task_assignment" => Style::default().fg(accent_color),
                            _ => Style::default().fg(Color::White),
                        };

                        notif_lines.push(Line::from(vec![
                            Span::styled(marker, Style::default().fg(accent_color)),
                            read_marker,
                            Span::styled(format!("[{}] ", notif.1.to_uppercase()), notif_type_style),
                            Span::styled(&notif.2, style),
                        ]));

                        // Parsea el timestamp RFC3339 y lo convierte a hora local legible
                        let ts_formatted = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&notif.6) {
                            dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
                        } else {
                            notif.6.clone()
                        };

                        notif_lines.push(Line::from(vec![
                            Span::styled(
                                format!("      {} - ", ts_formatted),
                                Style::default().fg(theme.muted),
                            ),
                            Span::styled(&notif.3, Style::default().fg(theme.text)),
                        ]));
                        notif_lines.push(Line::from(""));
                    }
                }

                let list_p = Paragraph::new(notif_lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(accent_color))
                        .title(Span::styled(
                            " Fellowship Notification Center ",
                            Style::default()
                                .fg(theme.warning)
                                .add_modifier(Modifier::BOLD),
                        )),
                );
                f.render_widget(list_p, sub_chunks[1]);
            } else if app.selected_fellowship_project_idx >= shared_projects.len() {
                let p = Paragraph::new("\n\n   Invalid selected project index.")
                    .style(Style::default().fg(theme.danger));
                f.render_widget(p, right_chunks[1]);
            } else {
                let current_proj = shared_projects[app.selected_fellowship_project_idx];
                // Jalamos todos los mensajes del chronicle del proyecto activo
                let messages = app
                    .db
                    .get_chronicle_messages(&current_proj.id.to_string())
                    .unwrap_or_default();

                // usize::MAX es el sentinel que indica que no estamos en modo browse
                let browsing = app.fellowship_selected_msg_idx != usize::MAX;

                // Split right panel: messages on top, input bar on bottom
                let chat_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(3), Constraint::Length(3)])
                    .split(right_chunks[1]);

                // Construimos las líneas del chat — también guardamos en qué línea empieza cada msg
                // para poder hacer scroll hacia el mensaje seleccionado, no manches si está complicado
                let mut chat_lines: Vec<Line> = Vec::new();
                let mut msg_start_lines: Vec<u16> = Vec::new();

                if messages.is_empty() {
                    chat_lines.push(Line::from(""));
                    chat_lines.push(Line::from(Span::styled(
                        "   No messages yet. Start the chronicle!",
                        Style::default().fg(theme.muted),
                    )));
                } else {
                    for (msg_idx, msg) in messages.iter().enumerate() {
                        msg_start_lines.push(chat_lines.len() as u16);
                        let is_selected = app.fellowship_selected_msg_idx == msg_idx;
                        // Fondo azulado oscuro para el mensaje seleccionado
                        let sel_bg = if is_selected { Color::Rgb(30, 35, 55) } else { Color::Reset };
                        let msg_type = &msg.5;
                        let ts = &msg.6;
                        let formatted_time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                            dt.with_timezone(&chrono::Local).format("%H:%M").to_string()
                        } else { ts.clone() };

                        // Traemos las reacciones del mensaje — cada DB call aquí, ojo con el performance
                        let reactions = app.db.get_message_reactions(&msg.0).unwrap_or_default();

                        if msg_type == "system" {
                            chat_lines.push(Line::from(vec![
                                Span::styled(
                                    format!(" ── {} ──  ", &msg.4),
                                    Style::default().fg(theme.muted).add_modifier(Modifier::ITALIC).bg(sel_bg),
                                ),
                            ]));
                        } else {
                            // Si el public_key del mensaje es el nuestro, el nombre va en accent color
                            let is_mine = msg.2 == app.identity.public_key;
                            let name_color = if is_mine { accent_color } else { Color::LightCyan };
                            let sel_marker = if is_selected { "▌" } else { " " };

                            // Header line: marker + time + sender
                            chat_lines.push(Line::from(vec![
                                Span::styled(sel_marker, Style::default().fg(accent_color).bg(sel_bg)),
                                Span::styled(
                                    format!(" {}", formatted_time),
                                    Style::default().fg(theme.muted).bg(sel_bg),
                                ),
                                Span::styled("  ", Style::default().bg(sel_bg)),
                                Span::styled(
                                    format!("{}", &msg.3),
                                    Style::default().fg(name_color).add_modifier(Modifier::BOLD).bg(sel_bg),
                                ),
                                Span::styled("  ", Style::default().bg(sel_bg)),
                            ]));

                            // Detección de URLs — si hay link, se pinta en cyan con underline
                            let content = &msg.4;
                            let has_url = extract_url(content).is_some();
                            let mut content_spans = vec![
                                Span::styled(if is_selected { "▌ " } else { "  " }, Style::default().fg(accent_color).bg(sel_bg)),
                            ];
                            if has_url {
                                // Partimos el contenido palabra por palabra para colorear solo las URLs
                                for word in content.split(' ') {
                                    let is_url = word.starts_with("http://") || word.starts_with("https://");
                                    if is_url {
                                        // Imagen o link normal — prefijo distinto para que se note
                                        let is_img = ["jpg","jpeg","png","gif","webp"].iter()
                                            .any(|e| word.to_lowercase().ends_with(e));
                                        let prefix = if is_img { "[img] " } else { "-> " };
                                        content_spans.push(Span::styled(
                                            format!("{}{} ", prefix, word),
                                            Style::default().fg(Color::Cyan)
                                                .add_modifier(Modifier::UNDERLINED).bg(sel_bg),
                                        ));
                                    } else {
                                        content_spans.push(Span::styled(
                                            format!("{} ", word),
                                            Style::default().fg(Color::White).bg(sel_bg),
                                        ));
                                    }
                                }
                            } else {
                                content_spans.push(Span::styled(
                                    content.as_str(),
                                    Style::default().fg(Color::White).bg(sel_bg),
                                ));
                            }
                            chat_lines.push(Line::from(content_spans));

                            // Reactions line (only if there are reactions or it's selected)
                            if !reactions.is_empty() {
                                let r_list: Vec<String> = reactions.iter().map(|r| r.1.clone()).collect();
                                chat_lines.push(Line::from(vec![
                                    Span::styled(if is_selected { "▌ " } else { "  " }, Style::default().fg(accent_color).bg(sel_bg)),
                                    Span::styled(
                                        r_list.join("  "),
                                        Style::default().fg(theme.warning).bg(sel_bg),
                                    ),
                                ]));
                            }

                            // Small gap between messages
                            chat_lines.push(Line::from(Span::styled(" ", Style::default().bg(Color::Reset))));
                        }
                    }
                }

                // Lógica de scroll — auto-baja al fondo si no estamos navegando mensajes
                let visible_h = chat_chunks[0].height.saturating_sub(2) as usize;
                let total_lines = chat_lines.len();
                let scroll: u16 = if !browsing || app.fellowship_selected_msg_idx >= msg_start_lines.len() {
                    // Sin browsing activo: siempre al fondo como chat normal
                    total_lines.saturating_sub(visible_h) as u16
                } else {
                    let msg_line = msg_start_lines[app.fellowship_selected_msg_idx] as usize;
                    // Ajusta el scroll para que el mensaje seleccionado quede visible con 2 líneas de margen
                    msg_line.saturating_sub(2).min(total_lines.saturating_sub(visible_h)) as u16
                };

                let chat_border_color = if app.fellowship_focus_left {
                    theme.muted
                } else if browsing {
                    theme.muted
                } else {
                    accent_color
                };
                // Contamos solo miembros de ESTE proyecto que están online — no contamos a todos
                let proj_members = app.db.get_presence_for_project(&current_proj.id.to_string()).unwrap_or_default();
                let online_count = proj_members.iter().filter(|m| m.3).count();
                let online_badge = if online_count > 0 {
                    format!("● {} online  ", online_count)
                } else {
                    String::new()
                };
                let chat_title = format!(" Chronicle: {}  {}", current_proj.name, online_badge);
                let chat_p = Paragraph::new(chat_lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(chat_border_color))
                            .title(vec![
                                Span::styled(
                                    format!(" Chronicle: {}  ", current_proj.name),
                                    Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    online_badge,
                                    Style::default().fg(theme.success).add_modifier(Modifier::BOLD),
                                ),
                            ]),
                    )
                    .scroll((scroll, 0));
                let _ = chat_title;
                f.render_widget(chat_p, chat_chunks[0]);

                // La barra de input cambia de modo: browse, composing, o idle
                let (input_text, input_border_color, input_title) = if browsing {
                    let total = messages.len();
                    let idx = app.fellowship_selected_msg_idx + 1;
                    let hint = if extract_url(messages.get(app.fellowship_selected_msg_idx).map(|m| m.4.as_str()).unwrap_or("")).is_some() {
                        "  [r] react  [c] copy URL  [↑↓] navigate  [Esc] exit browse"
                    } else {
                        "  [r] react  [c] copy  [↑↓] navigate  [Esc] exit browse"
                    };
                    (format!("  [{}/{}]{}", idx, total, hint), theme.muted, " Browse ")
                } else if app.fellowship_composing {
                    // Cursor parpadeante — alterna cada 500ms usando el timestamp del sistema
                    let cursor = if (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() / 500) % 2 == 0 { "█" } else { " " };
                    (format!("  > {}{}", app.fellowship_chat_input, cursor), accent_color, " Compose ")
                } else {
                    ("  Press [Enter] to compose a message...".to_string(), theme.muted, " Message ")
                };

                let input_p = Paragraph::new(input_text).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(input_border_color))
                        .title(Span::styled(
                            input_title,
                            Style::default().fg(input_border_color),
                        )),
                );
                f.render_widget(input_p, chat_chunks[1]);
            }
        }
        1 => {
            // Tab de invitaciones — muestra las que te mandaron con su estado (Pending/Accepted/etc)
            let invitations = app.db.get_invitations().unwrap_or_default();
            let mut invite_lines = vec![Line::from("")];

            if invitations.is_empty() {
                invite_lines.push(Line::from("   No invitations received yet."));
            } else {
                for (idx, invite) in invitations.iter().enumerate() {
                    let is_selected = idx == app.selected_invitation_idx;
                    let marker = if is_selected { "  > " } else { "    " };
                    let style = if is_selected {
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    // invite.7 es el status — color verde si aceptada, rojo si rechazada
                    invite_lines.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(accent_color)),
                        Span::styled(format!("Invitation to: '{}'", invite.2), style),
                        Span::styled(
                            format!("  [{}]", invite.7),
                            Style::default().fg(if invite.7 == "Pending" {
                                theme.warning
                            } else if invite.7 == "Accepted" {
                                theme.success
                            } else {
                                theme.danger
                            }),
                        ),
                    ]));
                    invite_lines.push(Line::from(vec![Span::styled(
                        format!(
                            "      Invited by: {} ({})",
                            invite.4,
                            &invite.3[..10.min(invite.3.len())]
                        ),
                        Style::default().fg(theme.muted),
                    )]));
                    invite_lines.push(Line::from(vec![Span::styled(
                        format!("      Role:       {}", invite.6),
                        Style::default().fg(theme.muted),
                    )]));
                    invite_lines.push(Line::from(""));
                }
            }

            let invite_p = Paragraph::new(invite_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent_color))
                    .title(Span::styled(
                        " Shared Fellowship Invitations ",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
            f.render_widget(invite_p, right_chunks[1]);
        }
        2 => {
            // Tab de compañeros — muestra miembros DEL PROYECTO SELECCIONADO con su presencia
            let (members, proj_name) = if shared_projects.is_empty() || app.selected_fellowship_project_idx >= shared_projects.len() {
                (Vec::new(), "None".to_string())
            } else {
                let proj = shared_projects[app.selected_fellowship_project_idx];
                let m = app.db.get_presence_for_project(&proj.id.to_string()).unwrap_or_default();
                (m, proj.name.clone())
            };
            let mut comp_lines = vec![Line::from("")];

            if members.is_empty() {
                comp_lines.push(Line::from("   No companions in this project yet."));
                comp_lines.push(Line::from("   Invite members with [i] to see them here."));
            } else {
                let online_n = members.iter().filter(|m| m.3).count();
                comp_lines.push(Line::from(vec![
                    Span::styled(
                        format!("   {} online  •  {} members", online_n, members.len()),
                        Style::default().fg(theme.muted),
                    ),
                ]));
                comp_lines.push(Line::from(""));

                for member in &members {
                    // member: (identity, username, role, is_online, last_seen, current_project)
                    let (identity, username, role, is_online, last_seen, current_proj) =
                        (&member.0, &member.1, &member.2, member.3, &member.4, &member.5);

                    let dot = if is_online { "● " } else { "○ " };
                    let dot_color = if is_online { theme.success } else { theme.muted };
                    let name_color = if is_online { Color::White } else { Color::Gray };

                    comp_lines.push(Line::from(vec![
                        Span::styled("   ", Style::default()),
                        Span::styled(dot, Style::default().fg(dot_color).add_modifier(Modifier::BOLD)),
                        Span::styled(username.clone(), Style::default().fg(name_color).add_modifier(Modifier::BOLD)),
                        Span::styled(
                            format!("  [{}]", role),
                            Style::default().fg(theme.muted),
                        ),
                    ]));

                    // Mostrar en qué proyecto está o cuándo fue la última actividad
                    let detail = if is_online {
                        if let Some(proj) = current_proj {
                            if !proj.is_empty() {
                                format!("       In: {}", proj)
                            } else {
                                "       Active now".to_string()
                            }
                        } else {
                            "       Active now".to_string()
                        }
                    } else if !last_seen.is_empty() && last_seen != "Never" {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_seen) {
                            let mins = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_minutes();
                            if mins < 60 {
                                format!("       Last seen {} min ago", mins)
                            } else if mins < 1440 {
                                format!("       Last seen {} h ago", mins / 60)
                            } else {
                                format!("       Last seen {} days ago", mins / 1440)
                            }
                        } else {
                            format!("       Last seen: {}", last_seen)
                        }
                    } else {
                        "       Never seen online".to_string()
                    };
                    comp_lines.push(Line::from(vec![Span::styled(
                        detail,
                        Style::default().fg(if is_online { Color::LightCyan } else { theme.muted }),
                    )]));
                    comp_lines.push(Line::from(vec![Span::styled(
                        format!("       id: {}…", &identity[..identity.len().min(20)]),
                        Style::default().fg(theme.border),
                    )]));
                    comp_lines.push(Line::from(""));
                }
            }

            let comp_title = format!(" {} — Members ", proj_name);
            let comp_p = Paragraph::new(comp_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent_color))
                    .title(Span::styled(
                        comp_title,
                        Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                    )),
            );
            f.render_widget(comp_p, right_chunks[1]);
        }
        3 => {
            // Feed de actividad reciente — últimas 15 acciones del equipo, pues
            let activities = app.db.get_recent_activities(15).unwrap_or_default();
            let mut act_lines = vec![Line::from("")];

            if activities.is_empty() {
                act_lines.push(Line::from("   No recent activity logged in Fellowship."));
            } else {
                for act in &activities {
                    let formatted_time =
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&act.6) {
                            dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
                        } else {
                            act.6.clone()
                        };

                    // Color por tipo de evento — cada logro tiene su color chido
                    let event_color = match act.2.as_str() {
                        "task_completed" => theme.success,
                        "milestone_completed" => theme.warning,
                        "achievement_unlocked" => Color::Magenta,
                        "member_joined" => Color::Cyan,
                        _ => Color::White,
                    };

                    act_lines.push(Line::from(vec![
                        Span::styled(
                            format!("   [{}] ", formatted_time),
                            Style::default().fg(theme.muted),
                        ),
                        Span::styled(
                            format!("({}) ", act.5),
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(&act.3, Style::default().fg(event_color)),
                    ]));
                }
            }

            let act_p = Paragraph::new(act_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent_color))
                    .title(Span::styled(
                        " Fellowship Chronicle Activity Feed ",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
            f.render_widget(act_p, right_chunks[1]);
        }
        4 => {
            // Tab de búsqueda — muestra resultados del query actual o pide que ingreses uno
            let mut search_lines = vec![Line::from("")];
            search_lines.push(Line::from(vec![
                Span::styled("   Search query: ", Style::default().fg(theme.muted)),
                Span::styled(
                    &app.fellowship_search_query,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            search_lines.push(Line::from(""));

            if app.fellowship_search_results.is_empty() {
                search_lines.push(Line::from(
                    "   No search results. Press [/] to enter search query.",
                ));
            } else {
                // Cada resultado incluye: proyecto, sender, contenido y timestamp
                for res in &app.fellowship_search_results {
                    let proj_name = &res.7;
                    let sender_name = &res.3;
                    let content = &res.4;
                    let ts = &res.6;
                    let formatted_time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                        dt.with_timezone(&chrono::Local).format("%m-%d %H:%M").to_string()
                    } else {
                        ts.clone()
                    };

                    search_lines.push(Line::from(vec![
                        Span::styled(
                            format!("   [{}] ", formatted_time),
                            Style::default().fg(theme.muted),
                        ),
                        Span::styled(
                            format!("[{}] ", proj_name),
                            Style::default().fg(theme.warning),
                        ),
                        Span::styled(
                            format!("{}: ", sender_name),
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(content, Style::default().fg(Color::White)),
                    ]));
                }
            }

            let search_p = Paragraph::new(search_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent_color))
                    .title(Span::styled(
                        " Chronicle Search Engine ",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
            f.render_widget(search_p, right_chunks[1]);
        }
        _ => {}
    }

    // Footer de acciones — cambia dinámicamente según el tab y el estado del focus
    let footer_text = match app.selected_fellowship_tab {
        0 => {
            if shared_projects.is_empty() {
                " [Enter] Mark as Read  |  [a] Mark All as Read  |  [Esc] back"
            } else if app.fellowship_focus_left {
                " [↑↓] select project  |  [Enter/→] open chat  |  [v] invite  |  [Esc] back"
            } else if app.fellowship_composing {
                " Type your message  |  [Enter] send  |  [Esc] cancel compose"
            } else {
                " [Enter] compose  |  [↑↓] browse msgs  |  [←/Esc] projects  |  [v] invite  |  [j] toggle sharing  |  [c/i/p/a] tabs"
            }
        }
        1 => " [Enter] accept invitation  |  [d] decline invitation  |  [Esc] back",
        2 => " [r] refresh presence  |  [Esc] back",
        3 => " [Esc] back",
        4 => " [/] new search  |  [Esc] back",
        _ => " [Esc] back",
    };
    let footer_p = Paragraph::new(format!("  Instructions: {}", footer_text))
        .style(Style::default().fg(theme.muted))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        );
    f.render_widget(footer_p, right_chunks[2]);

    // Modal para escribir un mensaje al Chronicle — simple input + confirm
    if let ModalType::PostMessage { content } = &app.modal_state {
        let area = centered_rect(50, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Post Message to Chronicle ",
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

        let input_p = Paragraph::new(format!("  {}", content)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent_color))
                .title(" Chronicle Message Content "),
        );
        f.render_widget(input_p, inner_layout[1]);

        let help_p = Paragraph::new("  [Enter] send  |  [Esc] cancel")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help_p, inner_layout[2]);
    }

    // Modal de reacciones — el usuario elige con número del 1 al 6, tipo Discord light
    if let ModalType::AddReaction { message_id: _ } = &app.modal_state {
        let area = centered_rect(40, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Add Chronicle Reaction ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Emoji list
                Constraint::Min(1),    // Help footer
            ])
            .split(block.inner(area));

        f.render_widget(block, area);

        let emoji_list = Paragraph::new("  [1] +1   [2] >>   [3] !!   [4] ~   [5] ++   [6] //")
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(emoji_list, inner_layout[1]);

        let help_p = Paragraph::new("  Press number 1-6 to react  |  [Esc] cancel")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help_p, inner_layout[2]);
    }

    // Modal para activar/desactivar sharing de un proyecto — toggle sencillo con [s]
    if let ModalType::ProjectSharing { project_id } = &app.modal_state {
        let area = centered_rect(50, 25, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        // Buscamos el proyecto por ID para saber su estado actual de sharing
        let is_proj_shared = app
            .projects
            .iter()
            .find(|p| p.id == *project_id)
            .map(|p| p.is_shared)
            .unwrap_or(false);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Toggle Project Sharing Status ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3), // Info box
                Constraint::Min(1),    // Help
            ])
            .split(block.inner(area));

        f.render_widget(block, area);

        let status_p = Paragraph::new(format!(
            "  Sharing is currently: {}",
            if is_proj_shared {
                "ENABLED"
            } else {
                "DISABLED (Local-Only)"
            }
        ))
        .style(
            Style::default()
                .fg(if is_proj_shared {
                    theme.success
                } else {
                    theme.danger
                })
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(status_p, inner_layout[0]);

        let help_p = Paragraph::new("  [s] Toggle Sharing Status  |  [Esc] close")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help_p, inner_layout[2]);
    }

    // Modal de búsqueda — cursor parpadeante igual que el de composing
    if let ModalType::SearchMessages { query } = &app.modal_state {
        let area = centered_rect(55, 30, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(accent_color))
            .title(Span::styled(
                " Search Chronicle ",
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            ));

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // spacer
                Constraint::Length(3), // input
                Constraint::Min(1),    // help
            ])
            .split(block.inner(area));

        f.render_widget(block, area);

        // Mismo truco del cursor — divide milisegundos entre 500 para alternar cada medio segundo
        let cursor = if (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() / 500) % 2 == 0 { "█" } else { " " };

        let input_p = Paragraph::new(format!("  > {}{}", query, cursor)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent_color))
                .title(Span::styled(" Query ", Style::default().fg(accent_color))),
        );
        f.render_widget(input_p, inner_layout[1]);

        let help_p = Paragraph::new("  Type to search  |  [Enter] confirm  |  [Esc] cancel")
            .style(Style::default().fg(theme.muted));
        f.render_widget(help_p, inner_layout[2]);
    }
}

// Calcula un Rect centrado dado porcentaje de ancho y alto — para los modales
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
