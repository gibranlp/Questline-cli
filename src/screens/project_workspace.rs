// ─────────────────────────────────────────────────────────────────────────────
// project_workspace.rs — el workspace donde ocurre todo: tareas, notas, journal y milestones
// ─────────────────────────────────────────────────────────────────────────────
use crate::app::{App, DueDateType, ModalType};
use crate::models::RecurrenceType;
use crate::milestone_templates::{self, ProjectStats, Tier};
use crate::models::{JournalEntry, Milestone, Note, Project, Task, TaskPriority};
use crate::screens::intro::centered_rect;
use crate::theme::Theme;
use chrono::{DateTime, Duration, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};

// El jefe máximo de renderizado — desde aquí se coordina todo el workspace
pub fn draw(f: &mut Frame, app: &App, theme: &Theme) {
    let size = f.size();
    let accent_color = theme.primary;

    let p_id = app.active_project_id.unwrap();
    let project = app.projects.iter().find(|p| p.id == p_id).unwrap();
    let is_shared = project.is_shared;
    let active_tab = app.workspace_tab_idx;

    let tasks = &app.all_tasks;
    let notes = &app.all_notes;
    let journals = &app.all_journals;
    let milestones = app
        .db
        .get_milestones_for_project(project.id)
        .unwrap_or_default();

    // Armamos las stats del proyecto — se usan para calcular el progreso de milestones
    let streak = app.db.get_streak().unwrap_or(crate::models::Streak {
        id: String::new(),
        current_streak: 0,
        best_streak: 0,
        last_active_day: None,
    });
    let project_stats = ProjectStats {
        project_age_days: (Utc::now() - project.created_at).num_days(),
        completed_tasks_in_project: tasks
            .iter()
            .filter(|t| t.project_id == Some(p_id) && t.completed && t.parent_task_id.is_none())
            .count() as i64,
        notes_in_project: notes
            .iter()
            .filter(|n| n.project_id == Some(p_id))
            .count() as i64,
        journal_entries_in_project: journals
            .iter()
            .filter(|j| j.project_id == p_id)
            .count() as i64,
        active_days_in_project: app
            .db
            .get_active_days_for_project(p_id)
            .unwrap_or(0),
        total_completed_tasks: tasks.iter().filter(|t| t.completed && t.parent_task_id.is_none()).count() as i64,
        current_streak: streak.current_streak as i64,
        focus_sessions_total: app.db.get_focus_sessions().unwrap_or_default().len() as i64,
        daily_adventures_completed: app
            .db
            .get_daily_adventures_completed_count()
            .unwrap_or(0),
    };

    let selected_item_idx = match active_tab {
        0 => app.selected_task_idx,
        1 => app.selected_notes_flat_idx,
        2 => app.selected_journal_idx,
        3 => app.selected_milestone_idx,
        _ => 0,
    };

    let modal = &app.modal_state;
    let overlay = &app.overlay_modal;
    let searching = app.searching;
    let search_query = &app.search_query;
    let task_filter = &app.task_filter;
    let task_sort = &app.task_sort;

    // Build task list — tiene que quedar idéntica al handler o se pierde el índice seleccionado
    let sorted_tasks: Vec<&Task> = if let Some(parent_id) = app.viewing_step_for_task {
        // Modo drill-down: mostramos solo los steps del padre, abiertos primero
        let mut steps: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.parent_task_id == Some(parent_id))
            .collect();
        steps.sort_by(|a, b| {
            a.completed.cmp(&b.completed).then_with(|| match task_sort.as_str() {
                "DueDate" => match (a.due_date, b.due_date) {
                    (Some(d1), Some(d2)) => d1.cmp(&d2),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.created_at.cmp(&b.created_at),
                },
                "Priority" => b.priority.cmp(&a.priority),
                "Alphabetical" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                _ => b.created_at.cmp(&a.created_at),
            })
        });
        steps
    } else {
        // Vista principal: padres filtrados y ordenados, con sus steps inline debajo
        let mut parents: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.project_id == Some(project.id) && t.parent_task_id.is_none())
            .filter(|t| match task_filter.as_str() {
                "Incomplete" => !t.completed,
                "Completed" => t.completed,
                _ => true,
            })
            .filter(|t| {
                if searching && !search_query.is_empty() {
                    t.title.to_lowercase().contains(&search_query.to_lowercase())
                        || t.description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&search_query.to_lowercase())
                } else {
                    true
                }
            })
            .collect();
        // Órale, aquí aplicamos el sort según lo que eligió el usuario
        match task_sort.as_str() {
            "DueDate" => parents.sort_by(|a, b| {
                a.completed.cmp(&b.completed).then_with(|| match (a.due_date, b.due_date) {
                    (Some(d1), Some(d2)) => d1.cmp(&d2),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.created_at.cmp(&b.created_at),
                })
            }),
            "Priority" => parents.sort_by(|a, b| {
                a.completed.cmp(&b.completed).then_with(|| b.priority.cmp(&a.priority))
            }),
            "Alphabetical" => parents.sort_by(|a, b| {
                a.completed.cmp(&b.completed)
                    .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            }),
            _ => parents.sort_by(|a, b| {
                a.completed.cmp(&b.completed).then_with(|| b.created_at.cmp(&a.created_at))
            }),
        }
        // Intercalamos los steps incompletos justo debajo de su padre — así queda la lista plana
        let mut flat: Vec<&Task> = Vec::new();
        for parent in parents {
            flat.push(parent);
            if !parent.completed {
                let mut steps: Vec<&Task> = tasks
                    .iter()
                    .filter(|t| t.parent_task_id == Some(parent.id) && !t.completed)
                    .collect();
                steps.sort_by(|a, b| match task_sort.as_str() {
                    "DueDate" => match (a.due_date, b.due_date) {
                        (Some(d1), Some(d2)) => d1.cmp(&d2),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.created_at.cmp(&b.created_at),
                    },
                    "Priority" => b.priority.cmp(&a.priority),
                    "Alphabetical" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                    _ => b.created_at.cmp(&a.created_at),
                });
                flat.extend(steps);
            }
        }
        flat
    };

    let filtered_notes: Vec<&Note> = notes
        .iter()
        .filter(|n| n.project_id == Some(project.id))
        .filter(|n| {
            if searching && !search_query.is_empty() {
                n.title
                    .to_lowercase()
                    .contains(&search_query.to_lowercase())
                    || n.markdown_content
                        .to_lowercase()
                        .contains(&search_query.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    let filtered_journals: Vec<&JournalEntry> = journals
        .iter()
        .filter(|j| j.project_id == project.id)
        .filter(|j| {
            if searching && !search_query.is_empty() {
                j.content
                    .to_lowercase()
                    .contains(&search_query.to_lowercase())
                    || j.entry_date.to_string().contains(search_query)
            } else {
                true
            }
        })
        .collect();

    // Layout de 3-4 zonas: header, body (sidebar+content), barra de búsqueda opcional, footer de ayuda
    let constraints = if searching {
        vec![
            Constraint::Length(3), // Workspace Header
            Constraint::Min(5),    // Body splits
            Constraint::Length(3), // Search bar
            Constraint::Length(3), // Keyboard help
        ]
    } else {
        vec![
            Constraint::Length(3), // Workspace Header
            Constraint::Min(5),    // Body splits
            Constraint::Length(3), // Keyboard help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    // 1. Header (Project Name)
    let header_text = format!("Campaign War Room: {}", project.name);
    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border)),
        );
    f.render_widget(header, chunks[0]);

    // 2. Main split area: Left (Menu tabs), Right (Context pane)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Left menu width
            Constraint::Min(10),    // Right content width
        ])
        .split(chunks[1]);

    // 2a. Left tab options list
    let menu_items = [
        "  1 Quests",
        "  2 Scrolls",
        "  3 Chronicles",
        "  4 Overview",
    ];
    let list_items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == active_tab {
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(*item).style(style)
        })
        .collect();
    let sidebar_border_style = if app.workspace_sidebar_focused {
        Style::default().fg(accent_color)
    } else {
        Style::default().fg(theme.border)
    };
    let menu_list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(sidebar_border_style)
            .title(" Workspace "),
    );
    f.render_widget(menu_list, body_chunks[0]);

    // 2b. El panel derecho cambia según el tab activo — aquí resolvemos qué va ahí
    let task_assignees: Vec<(String, String)> = if is_shared && active_tab == 0 {
        sorted_tasks
            .get(selected_item_idx)
            .and_then(|t| {
                app.db
                    .get_task_assignments(&t.id.to_string())
                    .ok()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };
    let sidebar_focused = app.workspace_sidebar_focused;
    let viewing_step_for_task = app.viewing_step_for_task;
    // Título del padre para el breadcrumb cuando estamos viendo steps
    let parent_quest_title = viewing_step_for_task.and_then(|parent_id| {
        tasks.iter().find(|t| t.id == parent_id).map(|t| t.title.clone())
    });
    match active_tab {
        0 => draw_tasks_tab(
            f,
            body_chunks[1],
            &sorted_tasks,
            selected_item_idx,
            task_filter,
            task_sort,
            &task_assignees,
            theme,
            sidebar_focused,
            &tasks,
            viewing_step_for_task,
            parent_quest_title.as_deref(),
            is_shared,
            &app.identity.public_key,
        ),
        1 => {
            draw_notes_tab(f, body_chunks[1], &filtered_notes, selected_item_idx, theme, sidebar_focused, &app.codices, app.note_preview_focused, app.note_preview_scroll);
        }
        2 => draw_journal_tab(
            f,
            body_chunks[1],
            &filtered_journals,
            selected_item_idx,
            theme,
            sidebar_focused,
            is_shared,
        ),
        _ => {
            let (overview_members, overview_activity) = if is_shared {
                (
                    app.db.get_presence_for_project(&p_id.to_string()).unwrap_or_default(),
                    app.db.get_activity_log_for_project(&p_id.to_string(), 20).unwrap_or_default(),
                )
            } else {
                (vec![], vec![])
            };
            draw_overview_tab(
                f,
                body_chunks[1],
                project,
                &tasks,
                &notes,
                &journals,
                &milestones,
                selected_item_idx,
                &project_stats,
                theme,
                sidebar_focused,
                &overview_members,
                &overview_activity,
            )
        }
    }

    // 3. Barra de búsqueda — solo aparece cuando el usuario presionó '/'
    let help_chunk_idx = if searching {
        let search_text = format!("Lore search: {}_", search_query);
        let search_p = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent_color)),
        );
        f.render_widget(search_p, chunks[2]);
        3
    } else {
        2
    };

    // 4. Footer de ayuda contextual — compacto, con [?] para el codex completo de atajos
    let key = |s: &'static str| Span::styled(s, Style::default().fg(accent_color).add_modifier(Modifier::BOLD));
    let sep = || Span::styled(" | ", Style::default().fg(theme.muted));
    let txt = |s: &'static str| Span::styled(s, Style::default().fg(theme.muted));

    let mut footer_spans: Vec<Span> = match active_tab {
        0 if viewing_step_for_task.is_some() => vec![
            txt(" Steps  "),
            key("n"), txt(" New Step"),
            sep(),
            key("Space"), txt(" ✓/↩"),
            sep(),
            key("Del"), txt(" Remove"),
            sep(),
            key("←"), txt(" Back"),
        ],
        0 => vec![
            txt(" Quests  "),
            key("n"), txt(" New"),
            sep(),
            key("Space"), txt(" ✓/↩"),
            sep(),
            key("→"), txt(" Steps"),
            sep(),
            key("f"), txt(" Filter"),
            sep(),
            key("s"), txt(" Sort"),
        ],
        1 => vec![
            txt(" Scrolls  "),
            key("n"), txt(" New"),
            sep(),
            key("Enter"), txt(" Open/Toggle"),
            sep(),
            key("e"), txt(" Rename"),
            sep(),
            key("r"), txt(" Move"),
            sep(),
            key("d"), txt(" Codex"),
            sep(),
            key("Del"), txt(" Remove"),
            sep(),
            key("→"), txt(" Preview"),
        ],
        2 => vec![
            txt(" Journal  "),
            key("j"), txt(" New Log"),
            sep(),
            key("v"), txt(" Visibility"),
        ],
        _ => vec![
            txt(" Overview  "),
            key("m"), txt(" New Milestone"),
            sep(),
            key("Space"), txt(" Toggle"),
            sep(),
            key("c"), txt(" Conquer"),
        ],
    };

    footer_spans.extend(vec![
        sep(),
        key("1-4"), txt(" Tabs"),
        sep(),
        key("/"), txt(" Search"),
        sep(),
        key("ESC"), txt(" Exit"),
        sep(),
        Span::styled("?", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
        txt(" Help"),
    ]);

    let footer = Paragraph::new(Line::from(footer_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(footer, chunks[help_chunk_idx]);

    // 5. Los modales van encima de todo — cuál se muestra depende del ModalType activo
    match modal {
        ModalType::NewTask {
            title,
            desc,
            desc_cursor,
            priority,
            due_date_type,
            due_date_val,
            set_date_val,
            focus_idx,
            parent_task_id,
            recurrence,
        } => {
            let is_step = parent_task_id.is_some();
            let modal_title = if is_step { " Create Step " } else { " Create Quest " };
            draw_task_modal(
                f,
                modal_title,
                title,
                desc,
                *desc_cursor,
                *priority,
                *due_date_type,
                due_date_val,
                set_date_val,
                *focus_idx,
                theme,
                None,
                0,
                false,
                !is_step,
                *recurrence,
            );
        }
        ModalType::EditTask {
            id,
            title,
            desc,
            desc_cursor,
            priority,
            due_date_type,
            due_date_val,
            set_date_val,
            focus_idx,
            step_selected_idx,
            is_step,
            recurrence,
        } => {
            let steps: Vec<&Task> = tasks.iter()
                .filter(|t| t.parent_task_id == Some(*id))
                .collect();
            let steps_opt = if *is_step { None } else { Some(steps.as_slice()) };
            draw_task_modal(
                f,
                " Edit Quest ",
                title,
                desc,
                *desc_cursor,
                *priority,
                *due_date_type,
                due_date_val,
                set_date_val,
                *focus_idx,
                theme,
                steps_opt,
                *step_selected_idx,
                false,
                !is_step,
                *recurrence,
            );
        }
        ModalType::NewJournalEntry { content } => {
            draw_journal_modal(f, content, theme);
        }
        ModalType::MilestoneTierSelect { selected_idx, .. } => {
            draw_tier_select_modal(f, *selected_idx, theme);
        }
        ModalType::MilestoneTemplateSelect {
            tier,
            selected_idx,
            ..
        } => {
            draw_template_select_modal(f, *tier, *selected_idx, theme);
        }
        ModalType::AssignTask {
            selected_member_idx,
            ..
        } => {
            draw_assign_task_modal(f, app, *selected_member_idx, theme);
        }
        ModalType::ShareNote { permission_idx, .. } => {
            draw_share_note_modal(f, *permission_idx, theme);
        }
        ModalType::JournalVisibility { visibility_idx, .. } => {
            draw_journal_visibility_modal(f, *visibility_idx, theme);
        }
        ModalType::NewCodex { name, .. } => {
            draw_new_codex_modal(f, name, theme);
        }
        ModalType::RenameCodex { name, .. } => {
            draw_rename_codex_modal(f, name, theme);
        }
        ModalType::RefileScroll { selected_idx, .. } => {
            draw_refile_scroll_modal(f, &app.codices, *selected_idx, theme);
        }
        ModalType::RefileCodex { codex_id, selected_idx } => {
            let targets = app.refile_codex_targets(*codex_id);
            let codex_name = app.codices.iter().find(|c| c.id == *codex_id).map(|c| c.name.as_str()).unwrap_or("");
            draw_refile_codex_modal(f, &app.codices, &targets, *selected_idx, codex_name, theme);
        }
        _ => {}
    }

    // 6. Overlay modal — NewTask encima del EditTask cuando agregas un step desde ahí
    // No manches, dos modales apilados — overlay_modal es el de arriba
    if let ModalType::NewTask {
        title,
        desc,
        desc_cursor,
        priority,
        due_date_type,
        due_date_val,
        set_date_val,
        focus_idx,
        parent_task_id,
        recurrence,
    } = overlay
    {
        let is_step = parent_task_id.is_some();
        let modal_title = if is_step { " Create Step " } else { " Create Quest " };
        draw_task_modal(
            f,
            modal_title,
            title,
            desc,
            *desc_cursor,
            *priority,
            *due_date_type,
            due_date_val,
            set_date_val,
            *focus_idx,
            theme,
            None,
            0,
            false,
            !is_step,
            *recurrence,
        );
    }

    // 7. Codex de atajos — se abre con [?] y cubre todo
    if app.workspace_help_open {
        draw_workspace_help(f, theme, is_shared);
    }
}

fn draw_workspace_help(f: &mut Frame, theme: &Theme, is_shared: bool) {
    let area = centered_rect(78, 88, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.primary))
        .title(Span::styled(
            " ✦ Quest Codex: Keybindings ✦ ",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    let k = |s: &str| Span::styled(s.to_string(), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD));
    let d = |s: &str| Span::styled(s.to_string(), Style::default().fg(theme.text));
    let h = |s: &str| Span::styled(s.to_string(), Style::default().fg(theme.warning).add_modifier(Modifier::BOLD));
    let m = |s: &str| Span::styled(s.to_string(), Style::default().fg(theme.muted));

    let col_w = (area.width.saturating_sub(6)) / 2;

    let left: Vec<Line> = vec![
        Line::from(vec![h("  QUESTS (Tab 1)")]),
        Line::from(vec![k("  n"), d("          New quest")]),
        Line::from(vec![k("  Enter / e"), d("    Edit quest")]),
        Line::from(vec![k("  Space"), d("       Complete / Reopen")]),
        Line::from(vec![k("  Delete"), d("      Delete quest")]),
        Line::from(vec![k("  →"), d("           View steps")]),
        Line::from(vec![k("  +"), d("           Add step to quest")]),
        Line::from(vec![k("  f"), d("           Cycle filter")]),
        Line::from(vec![k("  s"), d("           Cycle sort")]),
        if is_shared { Line::from(vec![k("  a"), d("           Assign member")]) }
        else          { Line::from(vec![m("  a"), m("           Assign (shared only)")]) },
        Line::from(vec![]),
        Line::from(vec![h("  STEP VIEW")]),
        Line::from(vec![k("  n"), d("          New step")]),
        Line::from(vec![k("  Enter / e"), d("    Edit step")]),
        Line::from(vec![k("  Space"), d("       Complete / Reopen")]),
        Line::from(vec![k("  Delete"), d("      Delete step")]),
        Line::from(vec![k("  ← / ESC"), d("     Back to quests")]),
        Line::from(vec![]),
        Line::from(vec![h("  NAVIGATION")]),
        Line::from(vec![k("  1 2 3 4"), d("     Switch tabs")]),
        Line::from(vec![k("  ← / →"), d("      Sidebar / content")]),
        Line::from(vec![k("  ↑ / ↓"), d("      Navigate items")]),
        Line::from(vec![k("  /"), d("           Search")]),
        Line::from(vec![k("  ESC"), d("         Exit workspace")]),
    ];

    let right: Vec<Line> = vec![
        Line::from(vec![h("  SCROLLS (Tab 2)")]),
        Line::from(vec![k("  n"), d("          New scroll")]),
        Line::from(vec![k("  Enter / e"), d("    Edit scroll")]),
        Line::from(vec![k("  d"), d("           New codex")]),
        Line::from(vec![k("  r"), d("           Move to codex")]),
        Line::from(vec![k("  Delete"), d("      Delete scroll")]),
        Line::from(vec![]),
        Line::from(vec![h("  JOURNAL (Tab 3)")]),
        Line::from(vec![k("  j"), d("           New journal log")]),
        Line::from(vec![k("  v"), d("           Toggle visibility")]),
        Line::from(vec![k("  Delete"), d("      Delete entry")]),
        Line::from(vec![]),
        Line::from(vec![h("  OVERVIEW (Tab 4)")]),
        Line::from(vec![k("  m"), d("           New milestone")]),
        Line::from(vec![k("  Space"), d("       Toggle milestone")]),
        Line::from(vec![k("  Delete"), d("      Remove milestone")]),
        Line::from(vec![k("  c"), d("           Conquer campaign")]),
        Line::from(vec![]),
        Line::from(vec![h("  SORT OPTIONS  "), m("(press s to cycle)")]),
        Line::from(vec![m("  Created Date → Due Date → Priority → A→Z")]),
        Line::from(vec![]),
        Line::from(vec![h("  FILTER OPTIONS  "), m("(press f to cycle)")]),
        Line::from(vec![m("  All → Incomplete → Completed")]),
        Line::from(vec![]),
        Line::from(vec![h("  RECURRENCE  "), m("(L/R/Space in quest modal)")]),
        Line::from(vec![m("  None → Daily → Weekly → Monthly → Yearly")]),
    ];

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(col_w), Constraint::Min(1)])
        .split(inner);

    let left_p = Paragraph::new(left);
    let right_p = Paragraph::new(right);
    f.render_widget(left_p, cols[0]);
    f.render_widget(right_p, cols[1]);

    let close_line = Line::from(vec![
        Span::styled("  Press ", Style::default().fg(theme.muted)),
        Span::styled("?", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
        Span::styled(" or ", Style::default().fg(theme.muted)),
        Span::styled("ESC", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" to close this codex", Style::default().fg(theme.muted)),
    ]);
    let close_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(3),
        width: area.width.saturating_sub(4),
        height: 1,
    };
    f.render_widget(Paragraph::new(close_line), close_area);
}

fn draw_new_codex_modal(f: &mut Frame, name: &str, theme: &Theme) {
    use ratatui::layout::Margin;
    let size = f.size();
    let area = ratatui::layout::Rect {
        x: size.width / 4,
        y: size.height / 3,
        width: size.width / 2,
        height: 5,
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .title(" New Codex ");
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
    f.render_widget(block, area);
    let inner = area.inner(&Margin { vertical: 1, horizontal: 2 });
    let text = format!("Name: {}█", name);
    let p = Paragraph::new(text).style(Style::default().fg(Color::White));
    f.render_widget(p, inner);
}

fn draw_rename_codex_modal(f: &mut Frame, name: &str, theme: &Theme) {
    use ratatui::layout::Margin;
    let size = f.size();
    let area = ratatui::layout::Rect {
        x: size.width / 4,
        y: size.height / 3,
        width: size.width / 2,
        height: 5,
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .title(" Rename Codex ");
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
    f.render_widget(block, area);
    let inner = area.inner(&Margin { vertical: 1, horizontal: 2 });
    let text = format!("Name: {}█", name);
    let p = Paragraph::new(text).style(Style::default().fg(Color::White));
    f.render_widget(p, inner);
}

// Modal para mover un scroll (note) a otro codex — lista todos los codices disponibles
fn draw_refile_scroll_modal(f: &mut Frame, codices: &[crate::models::Codex], selected_idx: usize, theme: &Theme) {
    let size = f.size();
    let item_count = (codices.len() + 1) as u16; // +1 por la opción "Ungrouped"
    let height = (item_count + 4).min(size.height.saturating_sub(4));
    let width = (size.width / 3).max(36).min(50);
    let area = ratatui::layout::Rect {
        x: (size.width.saturating_sub(width)) / 2,
        y: (size.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .title(" Move Scroll to Codex ");
    f.render_widget(block, area);

    use ratatui::layout::Margin;
    let inner = area.inner(&Margin { vertical: 1, horizontal: 1 });

    let mut items: Vec<ListItem> = Vec::new();
    // Option 0: Ungrouped
    let ungrouped_style = if selected_idx == 0 {
        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    items.push(ListItem::new("  ── Ungrouped ──").style(ungrouped_style));
    // Options 1..=n: each codex, indented if it has a parent
    for (i, codex) in codices.iter().enumerate() {
        let style = if selected_idx == i + 1 {
            Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let indent = if codex.parent_codex_id.is_some() { "    " } else { "  " };
        let parent_hint = if let Some(pid) = codex.parent_codex_id {
            codices.iter().find(|c| c.id == pid)
                .map(|p| format!(" ↳ {}", p.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        items.push(ListItem::new(format!("{}◆ {}{}", indent, codex.name, parent_hint)).style(style));
    }

    let hint = Paragraph::new("↑↓ navigate · Enter confirm · Esc cancel")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(List::new(items), chunks[0]);
    f.render_widget(hint, chunks[1]);
}

// Modal para mover un codex a otro padre — muestra Root + codices elegibles (no descendientes)
fn draw_refile_codex_modal(f: &mut Frame, all_codices: &[crate::models::Codex], targets: &[uuid::Uuid], selected_idx: usize, codex_name: &str, theme: &Theme) {
    let size = f.size();
    let item_count = (targets.len() + 1) as u16; // +1 for Root
    let height = (item_count + 4).min(size.height.saturating_sub(4));
    let width = (size.width / 3).max(40).min(56);
    let area = ratatui::layout::Rect {
        x: (size.width.saturating_sub(width)) / 2,
        y: (size.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
    let title = format!(" Move Codex: {} ", codex_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .title(title);
    f.render_widget(block, area);

    use ratatui::layout::Margin;
    let inner = area.inner(&Margin { vertical: 1, horizontal: 1 });

    let mut items: Vec<ListItem> = Vec::new();
    let root_style = if selected_idx == 0 {
        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    items.push(ListItem::new("  ── Root (Top Level) ──").style(root_style));

    for (i, &target_id) in targets.iter().enumerate() {
        let style = if selected_idx == i + 1 {
            Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        if let Some(codex) = all_codices.iter().find(|c| c.id == target_id) {
            let indent = if codex.parent_codex_id.is_some() { "    " } else { "  " };
            let parent_hint = if let Some(pid) = codex.parent_codex_id {
                all_codices.iter().find(|c| c.id == pid)
                    .map(|p| format!(" ↳ {}", p.name))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            items.push(ListItem::new(format!("{}◆ {}{}", indent, codex.name, parent_hint)).style(style));
        }
    }

    let hint = Paragraph::new("↑↓ navigate · Enter move · Esc cancel")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(List::new(items), chunks[0]);
    f.render_widget(hint, chunks[1]);
}

// Renderiza el tab de tareas — lista de quests a la izquierda, detalles de la seleccionada a la derecha
fn draw_tasks_tab(
    f: &mut Frame,
    area: Rect,
    tasks: &[&Task],
    selected_idx: usize,
    filter: &str,
    sort: &str,
    assignees: &[(String, String)],
    theme: &Theme,
    sidebar_focused: bool,
    all_tasks: &[Task],
    viewing_step_for_task: Option<uuid::Uuid>,
    parent_quest_title: Option<&str>,
    is_shared: bool,
    my_identity: &str,
) {
    let accent_color = theme.primary;
    let content_border = if sidebar_focused { theme.border } else { accent_color };

    let sub_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left list (Active Quests)
            Constraint::Percentage(50), // Right details (Task Ledger)
        ])
        .split(area);

    let empty_msg = if viewing_step_for_task.is_some() {
        "  No steps yet. Press [n] to add a step."
    } else {
        "  No matching quests. Press [n] for new."
    };
    let list_items: Vec<ListItem> = if tasks.is_empty() {
        vec![ListItem::new(empty_msg)]
    } else {
        tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let status = if t.completed { "[x]" } else { "[ ]" };
                let is_sel = i == selected_idx;

                // Si tiene parent y no estamos en modo drill-down, es un step inline — se indenta distinto
                let is_inline_step = t.parent_task_id.is_some() && viewing_step_for_task.is_none();

                if is_inline_step {
                    let (prefix_style, title_style) = if is_sel {
                        (
                            Style::default().fg(accent_color).add_modifier(Modifier::BOLD),
                            Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD),
                        )
                    } else {
                        (
                            Style::default().fg(theme.secondary),
                            Style::default().fg(theme.muted),
                        )
                    };
                    let prefix = if is_sel { "   > o " } else { "     o " };
                    ListItem::new(Line::from(vec![
                        Span::styled(prefix, prefix_style),
                        Span::styled(format!("{} ", status), prefix_style),
                        Span::styled(&t.title, title_style),
                    ]))
                } else {
                    // Fila de tarea padre — coloreamos según prioridad, chido
                    let prio_style = match t.priority {
                        TaskPriority::High => Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
                        TaskPriority::Medium => Style::default().fg(theme.warning),
                        TaskPriority::Low => Style::default().fg(Color::Cyan),
                    };
                    let select_style = if is_sel {
                        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    // Badge de progreso de steps [done/total] — solo para padres completados
                    let step_badge = if viewing_step_for_task.is_none() && t.parent_task_id.is_none() && t.completed {
                        let total = all_tasks.iter().filter(|s| s.parent_task_id == Some(t.id)).count();
                        if total > 0 {
                            let done = all_tasks.iter().filter(|s| s.parent_task_id == Some(t.id) && s.completed).count();
                            format!(" [{}/{}]", done, total)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    let recur_badge = if t.parent_task_id.is_none() {
                        match t.recurrence {
                            Some(r) => format!(" ↻{}", r.name()),
                            None => String::new(),
                        }
                    } else {
                        String::new()
                    };

                    let today = Utc::now().date_naive();
                    let date_badge = t
                        .due_date
                        .or(t.set_date);
                    let date_badge_style = |date: DateTime<Utc>| {
                        let date_day = date.date_naive();
                        let fg = if date_day <= today {
                            theme.danger
                        } else if date_day <= today + Duration::days(7) {
                            theme.warning
                        } else {
                            theme.success
                        };

                        if is_sel {
                            Style::default().fg(fg).bg(theme.selection).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(fg)
                        }
                    };

                    if t.completed {
                        let (fg, bg) = if is_sel { (theme.muted, accent_color) } else { (theme.muted, Color::Reset) };
                        let mut spans = vec![
                            Span::styled(format!(" {} ", status), Style::default().fg(fg).bg(bg)),
                            Span::styled(format!("({}) ", t.priority.name()), Style::default().fg(fg).bg(bg)),
                            Span::styled(&t.title, Style::default().fg(fg).bg(bg).add_modifier(Modifier::CROSSED_OUT)),
                        ];
                        if !step_badge.is_empty() {
                            spans.push(Span::styled(step_badge, Style::default().fg(fg).bg(bg)));
                        }
                        if !recur_badge.is_empty() {
                            spans.push(Span::styled(recur_badge, Style::default().fg(fg).bg(bg)));
                        }
                        if let Some(date) = date_badge {
                            spans.push(Span::styled(
                                format!(" [{}]", date.format("%Y-%m-%d")),
                                Style::default().fg(fg).bg(bg),
                            ));
                        }
                        ListItem::new(Line::from(spans))
                    } else {
                        let mut spans = vec![
                            Span::styled(format!(" {} ", status), select_style),
                            Span::styled(format!("({}) ", t.priority.name()), prio_style),
                            Span::styled(&t.title, select_style),
                        ];
                        if !step_badge.is_empty() {
                            spans.push(Span::styled(step_badge, Style::default().fg(theme.muted)));
                        }
                        if !recur_badge.is_empty() {
                            spans.push(Span::styled(recur_badge, Style::default().fg(Color::Cyan)));
                        }
                        if let Some(date) = date_badge {
                            spans.push(Span::styled(
                                format!(" [{}]", date.format("%Y-%m-%d")),
                                date_badge_style(date),
                            ));
                        }
                        // Badge de compañero — muestra quién creó la tarea en proyectos compartidos
                        if is_shared {
                            if let Some(ref owner) = t.owner_username {
                                let is_mine = t.owner_identity.as_deref()
                                    .map(|id| id == my_identity)
                                    .unwrap_or(true);
                                if !is_mine && !owner.is_empty() {
                                    spans.push(Span::styled(
                                        format!(" @{}", owner),
                                        Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC),
                                    ));
                                }
                            }
                        }
                        ListItem::new(Line::from(spans))
                    }
                }
            })
            .collect()
    };

    let list_title = if let Some(parent_title) = parent_quest_title {
        format!(" Steps for: {} ", parent_title)
    } else {
        let sort_label = match sort {
            "DueDate" => "Due Date",
            "Priority" => "Priority",
            "Alphabetical" => "A→Z",
            _ => "Created Date",
        };
        format!(
            " Quests [Filter: {}] [Sort: {} — open first] ",
            filter, sort_label
        )
    };

    let list_widget = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(content_border))
            .title(list_title),
    );
    f.render_widget(list_widget, sub_chunks[0]);

    // Panel derecho de detalles — muestra todo lo importante de la quest seleccionada
    let details_widget = if tasks.is_empty() || selected_idx >= tasks.len() {
        Paragraph::new("\n  Select a quest details report.").block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Quest Ledger "),
        )
    } else {
        let t = tasks[selected_idx];
        let desc = t
            .description
            .as_deref()
            .unwrap_or("No description provided.");
        let due_str = t
            .due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "None".to_string());

        let mut text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Quest Title: ", Style::default().fg(theme.muted)),
                Span::styled(
                    &t.title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Priority:    ", Style::default().fg(theme.muted)),
                Span::styled(
                    t.priority.name(),
                    match t.priority {
                        TaskPriority::High => {
                            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD)
                        }
                        TaskPriority::Medium => Style::default().fg(theme.warning),
                        TaskPriority::Low => Style::default().fg(Color::Cyan),
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Due Date:    ", Style::default().fg(theme.muted)),
                Span::styled(due_str, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Status:      ", Style::default().fg(theme.muted)),
                Span::styled(
                    if t.completed {
                        "Completed Quest"
                    } else {
                        "Active Adventure"
                    },
                    if t.completed {
                        Style::default().fg(theme.success)
                    } else {
                        Style::default().fg(theme.warning)
                    },
                ),
            ]),
            Line::from(""),
            Line::from("  Description:"),
        ];
        for line in desc.lines() {
            text.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme.text),
            )));
        }
        // Los steps de esta tarea van al final del panel de detalles
        let steps: Vec<&Task> = all_tasks
            .iter()
            .filter(|s| s.parent_task_id == Some(t.id))
            .collect();
        if !steps.is_empty() {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "  Steps:",
                Style::default().fg(theme.muted),
            )));
            for s in &steps {
                let check = if s.completed { "[x]" } else { "[ ]" };
                let due_str = s
                    .due_date
                    .map(|d| format!(" - {}", d.format("%Y-%m-%d")))
                    .unwrap_or_default();
                let (fg, modifier) = if s.completed {
                    (theme.muted, Modifier::CROSSED_OUT)
                } else {
                    (Color::White, Modifier::empty())
                };
                text.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", check),
                        Style::default().fg(if s.completed { theme.muted } else { theme.success }),
                    ),
                    Span::styled(
                        format!("{}{}", s.title, due_str),
                        Style::default().fg(fg).add_modifier(modifier),
                    ),
                ]));
            }
        }

        if !assignees.is_empty() {
            let names: Vec<&str> = assignees.iter().map(|(_, name)| name.as_str()).collect();
            text.push(Line::from(""));
            text.push(Line::from(vec![
                Span::styled("  Assigned:    ", Style::default().fg(theme.muted)),
                Span::styled(
                    names.join(", "),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
                    .title(" Quest Ledger "),
            )
            .wrap(ratatui::widgets::Wrap { trim: true })
    };
    f.render_widget(details_widget, sub_chunks[1]);
}

// ── Markdown preview helpers ─────────────────────────────────────────────────

// Parser de markdown inline — maneja bold, italic, code, links y URLs de a poco, caracter por caracter
fn md_inline(text: &str, base: Style, accent: Color, muted: Color) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut s = text;

    while !s.is_empty() {
        // **bold**
        if let Some(rest) = s.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(rest[..end].to_string(), base.add_modifier(Modifier::BOLD)));
                s = &rest[end + 2..];
                continue;
            }
        }
        // ~~strikethrough~~
        if let Some(rest) = s.strip_prefix("~~") {
            if let Some(end) = rest.find("~~") {
                spans.push(Span::styled(rest[..end].to_string(), base.add_modifier(Modifier::CROSSED_OUT)));
                s = &rest[end + 2..];
                continue;
            }
        }
        // *italic* (single, not **)
        if s.starts_with('*') && !s.starts_with("**") {
            let rest = &s[1..];
            if let Some(end) = rest.find('*') {
                spans.push(Span::styled(rest[..end].to_string(), base.add_modifier(Modifier::ITALIC)));
                s = &rest[end + 1..];
                continue;
            }
        }
        // _italic_
        if s.starts_with('_') {
            let rest = &s[1..];
            if let Some(end) = rest.find('_') {
                spans.push(Span::styled(rest[..end].to_string(), base.add_modifier(Modifier::ITALIC)));
                s = &rest[end + 1..];
                continue;
            }
        }
        // `inline code`
        if s.starts_with('`') {
            let rest = &s[1..];
            if let Some(end) = rest.find('`') {
                spans.push(Span::styled(
                    format!("`{}`", &rest[..end]),
                    Style::default().fg(accent),
                ));
                s = &rest[end + 1..];
                continue;
            }
        }
        // [text](url)
        if s.starts_with('[') {
            if let Some(bracket_end) = s[1..].find(']') {
                let link_text = &s[1..bracket_end + 1];
                let after = &s[bracket_end + 2..];
                if after.starts_with('(') {
                    if let Some(paren_end) = after[1..].find(')') {
                        let url = &after[1..paren_end + 1];
                        spans.push(Span::styled(
                            link_text.to_string(),
                            base.fg(accent).add_modifier(Modifier::UNDERLINED),
                        ));
                        spans.push(Span::styled(
                            format!(" ↗ {}", url),
                            Style::default().fg(muted),
                        ));
                        s = &after[paren_end + 2..];
                        continue;
                    }
                }
            }
        }
        // bare URL
        if s.starts_with("https://") || s.starts_with("http://") {
            let end = s.find(char::is_whitespace).unwrap_or(s.len());
            spans.push(Span::styled(
                s[..end].to_string(),
                Style::default().fg(accent).add_modifier(Modifier::UNDERLINED),
            ));
            s = &s[end..];
            continue;
        }
        // Texto plano — avanzamos hasta el siguiente token de markdown potencial
        let next = s
            .find(|c: char| matches!(c, '*' | '`' | '[' | '_' | '~'))
            .and_then(|p| {
                // also check for URL starts
                let url_pos = [s.find("https://"), s.find("http://")]
                    .into_iter()
                    .flatten()
                    .min();
                Some(url_pos.map_or(p, |u| p.min(u)))
            })
            .unwrap_or(s.len());
        let take = next.max(s.chars().next().map(|c| c.len_utf8()).unwrap_or(1));
        spans.push(Span::styled(s[..take].to_string(), base));
        s = &s[take..];
    }

    spans
}

// Convierte una línea de markdown a un Line de ratatui — detecta headers, bullets, blockquotes etc.
fn md_line<'a>(raw: &str, theme: &Theme) -> Line<'a> {
    let trimmed = raw.trim_start();
    let leading = raw.len() - trimmed.len();
    let default_style = Style::default().fg(theme.text);
    let accent = theme.primary;
    let muted = theme.muted;

    // Blank line
    if trimmed.is_empty() {
        return Line::from("");
    }

    // Horizontal rule
    if trimmed.len() >= 3
        && (trimmed.chars().all(|c| c == '-')
            || trimmed.chars().all(|c| c == '*')
            || trimmed.chars().all(|c| c == '='))
    {
        return Line::from(Span::styled(
            "  ──────────────────────────────────────",
            Style::default().fg(theme.muted),
        ));
    }

    // Task-list items (must come before plain bullet)
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        let mut spans = vec![Span::styled("  ☐ ".to_string(), Style::default().fg(muted))];
        spans.extend(md_inline(rest, default_style, accent, muted));
        return Line::from(spans);
    }
    if let Some(rest) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        let mut spans = vec![Span::styled("  ☑ ".to_string(), Style::default().fg(theme.success))];
        spans.extend(md_inline(
            rest,
            Style::default().fg(muted).add_modifier(Modifier::CROSSED_OUT),
            accent,
            muted,
        ));
        return Line::from(spans);
    }

    // Headings
    if let Some(rest) = trimmed.strip_prefix("# ") {
        let mut spans = vec![Span::styled("  ".to_string(), Style::default())];
        spans.extend(md_inline(
            rest,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            accent,
            muted,
        ));
        return Line::from(spans);
    }
    if let Some(rest) = trimmed.strip_prefix("## ") {
        let mut spans = vec![Span::styled("  ".to_string(), Style::default())];
        spans.extend(md_inline(
            rest,
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
            accent,
            muted,
        ));
        return Line::from(spans);
    }
    if let Some(rest) = trimmed.strip_prefix("### ") {
        let mut spans = vec![Span::styled("  ".to_string(), Style::default())];
        spans.extend(md_inline(
            rest,
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            accent,
            muted,
        ));
        return Line::from(spans);
    }
    if let Some(rest) = trimmed
        .strip_prefix("#### ")
        .or_else(|| trimmed.strip_prefix("##### "))
        .or_else(|| trimmed.strip_prefix("###### "))
    {
        let mut spans = vec![Span::styled("  ".to_string(), Style::default())];
        spans.extend(md_inline(rest, Style::default().fg(theme.warning), accent, muted));
        return Line::from(spans);
    }

    // Blockquote
    if let Some(rest) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix('>')) {
        let mut spans = vec![Span::styled("  │ ".to_string(), Style::default().fg(muted))];
        spans.extend(md_inline(
            rest.trim(),
            Style::default().fg(muted).add_modifier(Modifier::ITALIC),
            accent,
            muted,
        ));
        return Line::from(spans);
    }

    // Unordered bullets (- * +), with indent awareness
    let bullet_style = Style::default().fg(accent);
    if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")).or_else(|| trimmed.strip_prefix("+ ")) {
        let prefix = if leading >= 4 {
            "      ◦ "
        } else if leading >= 2 {
            "    ◦ "
        } else {
            "  • "
        };
        let mut spans = vec![Span::styled(prefix.to_string(), bullet_style)];
        spans.extend(md_inline(rest, default_style, accent, muted));
        return Line::from(spans);
    }

    // Numbered list "N. text"
    if let Some(dot) = trimmed.find(". ") {
        let maybe_num = &trimmed[..dot];
        if !maybe_num.is_empty() && maybe_num.chars().all(|c| c.is_ascii_digit()) {
            let rest = &trimmed[dot + 2..];
            let prefix = format!("  {}. ", maybe_num);
            let mut spans =
                vec![Span::styled(prefix, Style::default().fg(accent).add_modifier(Modifier::BOLD))];
            spans.extend(md_inline(rest, default_style, accent, muted));
            return Line::from(spans);
        }
    }

    // Regular paragraph — preserve leading indentation up to 8 spaces, then inline parse
    let pad = " ".repeat(leading.min(8) + 2);
    let mut spans = vec![Span::raw(pad)];
    spans.extend(md_inline(trimmed, default_style, accent, muted));
    Line::from(spans)
}

// Append one level of the DFS tree into flat_list (text, note_idx, is_header)
fn append_display_subtree(
    flat_list: &mut Vec<(String, Option<usize>, bool)>,
    notes: &[&Note],
    codices: &[crate::models::Codex],
    parent: Option<uuid::Uuid>,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    let note_indent = "  ".repeat(depth + 1);
    let children: Vec<&crate::models::Codex> = codices.iter()
        .filter(|c| c.parent_codex_id == parent)
        .collect();
    for codex in children {
        let icon = if codex.collapsed { "▶" } else { "▼" };
        flat_list.push((format!("{}{} {} ", indent, icon, codex.name), None, true));
        if !codex.collapsed {
            append_display_subtree(flat_list, notes, codices, Some(codex.id), depth + 1);
            let mut codex_notes: Vec<(usize, &&Note)> = notes.iter().enumerate()
                .filter(|(_, n)| n.codex_id == Some(codex.id))
                .collect();
            codex_notes.sort_by(|(_, a), (_, b)| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            for (idx, note) in codex_notes {
                let lock = if note.sharing_permission == "read_only" { "[R] " } else { "" };
                flat_list.push((format!("{}· {}{} ", note_indent, lock, note.title), Some(idx), false));
            }
        }
    }
}

// Tab de notas — lista de scrolls a la izquierda (árbol expandido con indentación), preview a la derecha
fn draw_notes_tab(f: &mut Frame, area: Rect, notes: &[&Note], selected_flat_idx: usize, theme: &Theme, sidebar_focused: bool, codices: &[crate::models::Codex], preview_focused: bool, preview_scroll: usize) {
    let accent_color = theme.primary;
    let list_border = if sidebar_focused || preview_focused { theme.border } else { accent_color };
    let preview_border = if preview_focused { accent_color } else { theme.border };
    let content_border = list_border;

    let sub_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Note index
            Constraint::Percentage(60), // Selected note preview
        ])
        .split(area);

    // DFS tree — collapsed codices (codex.collapsed == true) hide their children
    let mut flat_list: Vec<(String, Option<usize>, bool)> = Vec::new();
    append_display_subtree(&mut flat_list, notes, codices, None, 0);

    // Ungrouped notes (no codex or codex from a different project)
    let mut ungrouped: Vec<usize> = notes.iter().enumerate()
        .filter(|(_, n)| n.codex_id.is_none() || !codices.iter().any(|c| Some(c.id) == n.codex_id))
        .map(|(i, _)| i)
        .collect();
    ungrouped.sort_by(|&a, &b| notes[a].title.to_lowercase().cmp(&notes[b].title.to_lowercase()));
    if !codices.is_empty() && !ungrouped.is_empty() {
        flat_list.push(("  ── Ungrouped ──".to_string(), None, false));
    }
    for idx in &ungrouped {
        let lock = if notes[*idx].sharing_permission == "read_only" { "[R] " } else { "" };
        flat_list.push((format!("· {}{} ", lock, notes[*idx].title), Some(*idx), false));
    }

    // scroll_padding = half the visible list height → selected stays near the middle
    let visible_height = sub_chunks[0].height.saturating_sub(2) as usize;
    let scroll_padding = (visible_height / 2).max(1);

    let list_items: Vec<ListItem> = if flat_list.is_empty() && notes.is_empty() {
        vec![ListItem::new("  No campaign scrolls. Press [n] to write.")]
    } else if flat_list.is_empty() {
        // No codices — plain note list
        notes.iter().enumerate().map(|(i, n)| {
            let style = if i == selected_flat_idx {
                Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let lock = if n.sharing_permission == "read_only" { "[R] " } else { "" };
            ListItem::new(format!(" {}{} ", lock, n.title)).style(style)
        }).collect()
    } else {
        flat_list.iter().enumerate().map(|(flat_i, (text, note_idx, is_header))| {
            if note_idx.is_none() && !is_header {
                // Divider
                ListItem::new(text.as_str()).style(Style::default().fg(theme.muted))
            } else if *is_header {
                let style = if flat_i == selected_flat_idx {
                    Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(accent_color).add_modifier(Modifier::BOLD)
                };
                ListItem::new(text.as_str()).style(style)
            } else {
                let style = if flat_i == selected_flat_idx {
                    Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(text.as_str()).style(style)
            }
        }).collect()
    };

    let total_items = list_items.len();
    let list_widget = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(content_border))
                .title(" Campaign Scrolls"),
        )
        .scroll_padding(scroll_padding);

    let mut list_state = ratatui::widgets::ListState::default()
        .with_selected(Some(selected_flat_idx.min(total_items.saturating_sub(1))));
    f.render_stateful_widget(list_widget, sub_chunks[0], &mut list_state);

    // Convertimos la posición plana a índice real de nota — los headers no tienen nota
    let selected_note_idx: Option<usize> = flat_list.get(selected_flat_idx)
        .and_then(|(_, ni, _)| *ni)
        .or_else(|| if !notes.is_empty() && flat_list.is_empty() && selected_flat_idx < notes.len() {
            Some(selected_flat_idx)
        } else {
            None
        });

    // Note preview panel
    let preview_title = if preview_focused { " Document Preview  ↑↓ Scroll " } else { " Document Preview " };
    let preview_widget = if notes.is_empty() || selected_note_idx.map_or(true, |i| i >= notes.len()) {
        Paragraph::new("\n  No scroll selected.").block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(preview_border))
                .title(preview_title),
        )
    } else {
        let n = notes[selected_note_idx.unwrap()];

        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    n.title.clone(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "  {}",
                    n.updated_at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M")
                ),
                Style::default().fg(theme.muted),
            )),
            Line::from(Span::styled(
                "  ──────────────────────────────────────",
                Style::default().fg(theme.muted),
            )),
            Line::from(""),
        ];

        // Renderizamos el markdown — los code blocks van amarillos, el resto pasa por md_line
        if n.markdown_content.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (empty)",
                Style::default().fg(theme.muted).add_modifier(Modifier::ITALIC),
            )));
        } else {
            let mut in_code_block = false;
            for raw_line in n.markdown_content.lines() {
                if raw_line.trim_start().starts_with("```") {
                    in_code_block = !in_code_block;
                    lines.push(Line::from(Span::styled(
                        "  ─── code ─────────────────────────",
                        Style::default().fg(theme.muted),
                    )));
                } else if in_code_block {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            raw_line.to_string(),
                            Style::default().fg(theme.warning),
                        ),
                    ]));
                } else {
                    lines.push(md_line(raw_line, theme));
                }
            }
        }

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(preview_border))
                    .title(preview_title),
            )
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((preview_scroll as u16, 0))
    };
    f.render_widget(preview_widget, sub_chunks[1]);
}

// El tab de journal — muestra las entradas cronológicas del proyecto, con autor si es proyecto compartido
fn draw_journal_tab(
    f: &mut Frame,
    area: Rect,
    journals: &[&JournalEntry],
    selected_idx: usize,
    theme: &Theme,
    sidebar_focused: bool,
    is_shared: bool,
) {
    let accent_color = theme.primary;
    let content_border = if sidebar_focused { theme.border } else { accent_color };

    let items: Vec<ListItem> = if journals.is_empty() {
        vec![ListItem::new(
            "  No daily logs recorded. Press [j] to write chronicle.",
        )]
    } else {
        journals
            .iter()
            .enumerate()
            .map(|(i, j)| {
                let bullet = if i == selected_idx { ">" } else { "-" };
                let date_str = j.entry_date.to_string();
                let highlight_style = if i == selected_idx {
                    Style::default()
                        .fg(accent_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.warning)
                };

                let mut header_spans = vec![
                    Span::styled(format!("{} ", bullet), highlight_style),
                    Span::styled(format!("[{}] ", date_str), highlight_style),
                ];
                if is_shared && !j.author_username.is_empty() {
                    header_spans.push(Span::styled(
                        format!("by {} ", j.author_username),
                        Style::default().fg(theme.muted),
                    ));
                }
                let mut spans = vec![Line::from(header_spans)];

                for line in j.content.lines() {
                    spans.push(Line::from(Span::styled(
                        format!("     {}", line),
                        Style::default().fg(Color::White),
                    )));
                }
                spans.push(Line::from(""));

                ListItem::new(spans)
            })
            .collect()
    };

    let list_widget = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(content_border))
            .title(" Campaign Chronicles"),
    );
    f.render_widget(list_widget, area);
}

// El tab de overview — métricas del proyecto, barra de progreso y lista de milestones con su avance
fn draw_overview_tab(
    f: &mut Frame,
    area: Rect,
    project: &Project,
    tasks: &[Task],
    notes: &[Note],
    journals: &[JournalEntry],
    milestones: &[Milestone],
    selected_milestone_idx: usize,
    project_stats: &ProjectStats,
    theme: &Theme,
    sidebar_focused: bool,
    // (identity, username, role, is_online, last_seen, current_project)
    members: &[(String, String, String, bool, String, Option<String>)],
    activity: &[(String, Option<String>, String, String, String, String, String)],
) {
    let accent_color = theme.primary;
    let content_border = if sidebar_focused { theme.border } else { accent_color };

    let proj_tasks: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.project_id == Some(project.id))
        .collect();
    let completed_count = proj_tasks.iter().filter(|t| t.completed).count();
    let remaining_count = proj_tasks.iter().filter(|t| !t.completed).count();

    let now = Utc::now();
    let overdue_count = proj_tasks
        .iter()
        .filter(|t| !t.completed && t.due_date.map(|d| d < now).unwrap_or(false))
        .count();

    let note_count = notes
        .iter()
        .filter(|n| n.project_id == Some(project.id))
        .count();
    let journal_count = journals
        .iter()
        .filter(|j| j.project_id == project.id)
        .count();

    // Porcentaje de completion — si no hay tareas se considera 100% para no llorar
    let completion_pct = if proj_tasks.is_empty() {
        100.0
    } else {
        (completed_count as f64 / proj_tasks.len() as f64) * 100.0
    };

    // Calculate project age
    let diff = Utc::now().signed_duration_since(project.created_at);
    let age_str = if diff.num_days() == 0 {
        "0 days old (Initialized today)".to_string()
    } else {
        format!("{} days old", diff.num_days())
    };

    let constraints: Vec<Constraint> = if project.is_shared {
        vec![
            Constraint::Length(5),  // Project Metadata
            Constraint::Length(4),  // Progress Bar
            Constraint::Length(15), // Stats & Milestones (fixed so fellowship gets space)
            Constraint::Min(8),     // Fellowship: Companions + Activity
        ]
    } else {
        vec![
            Constraint::Length(5), // Project Metadata
            Constraint::Length(4), // Progress Bar
            Constraint::Min(4),    // Stats & Milestones
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(area);

    // 1. Metadata Block
    let mut metadata_text = Vec::new();
    let desc = project.description.as_deref().unwrap_or("None");
    let mut desc_lines = desc.lines();
    if let Some(first_line) = desc_lines.next() {
        metadata_text.push(Line::from(vec![
            Span::styled("  Description: ", Style::default().fg(theme.muted)),
            Span::styled(first_line, Style::default().fg(Color::White)),
        ]));
        for remaining_line in desc_lines {
            metadata_text.push(Line::from(vec![
                Span::styled("               ", Style::default().fg(theme.muted)),
                Span::styled(remaining_line, Style::default().fg(Color::White)),
            ]));
        }
    } else {
        metadata_text.push(Line::from(vec![
            Span::styled("  Description: ", Style::default().fg(theme.muted)),
            Span::styled("None", Style::default().fg(Color::White)),
        ]));
    }

    metadata_text.extend(vec![
        Line::from(vec![
            Span::styled("  Created At:  ", Style::default().fg(theme.muted)),
            Span::styled(
                project.created_at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string(),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Chronicle Age: ", Style::default().fg(theme.muted)),
            Span::styled(age_str, Style::default().fg(theme.warning)),
        ]),
    ]);
    let metadata_p = Paragraph::new(metadata_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Chronicle Context "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(metadata_p, chunks[0]);

    // 2. Progress Gauge
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Quest Completion Index "),
        )
        .gauge_style(Style::default().fg(accent_color).bg(Color::Rgb(30, 30, 30)))
        .label(format!("{:.1}% Resolved", completion_pct))
        .ratio(completion_pct / 100.0);
    f.render_widget(gauge, chunks[1]);

    // 3. Bottom horizontal split
    let bottom_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    // 3a. Stats details
    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  RESOLVED QUESTS:   ", Style::default().fg(theme.success)),
            Span::styled(
                format!("{}  ", completed_count),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  PENDING ADVENTURES: ", Style::default().fg(theme.warning)),
            Span::styled(
                format!("{}  ", remaining_count),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  OVERDUE PENALTIES:  ", Style::default().fg(theme.danger)),
            Span::styled(
                format!("{}  ", overdue_count),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  SCROLLS (NOTES):    ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}  ", note_count),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  CHRONICLES (LOGS):  ",
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                format!("{}  ", journal_count),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let stats_p = Paragraph::new(stats_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(content_border))
            .title(" Adventure Metrics "),
    );
    f.render_widget(stats_p, bottom_split[0]);

    // 3b. Lista de milestones — cada uno muestra su progreso de requisitos si tiene template
    let milestone_items: Vec<ListItem> = if milestones.is_empty() {
        vec![ListItem::new(
            "  No milestones established. Press [m] to formulate one.",
        )]
    } else {
        milestones
            .iter()
            .enumerate()
            .flat_map(|(idx, m)| {
                let check = if m.completed { "[x]" } else { "[ ]" };
                let highlight = if idx == selected_milestone_idx { "> " } else { "  " };
                let is_selected = idx == selected_milestone_idx;
                let name_style = if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                let arrow_style = Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD);

                // Tier label
                let tier_label = match m.tier {
                    1 => " [T1-Initiate]",
                    2 => " [T2-Veteran]",
                    3 => " [T3-Legendary]",
                    _ => "",
                };

                let header = ListItem::new(Line::from(vec![
                    Span::styled(highlight, arrow_style),
                    Span::styled(format!("{} ", check), Style::default()),
                    Span::styled(&m.name, name_style),
                    Span::styled(
                        format!(" +{} XP", m.xp_reward),
                        Style::default().fg(theme.warning),
                    ),
                    Span::styled(tier_label, Style::default().fg(theme.muted)),
                ]));

                let mut rows: Vec<ListItem> = vec![header];

                // Si tiene template y no está completo, mostramos cada requisito con ✓ o ✗ — qué chido
                if !m.completed && !m.template_id.is_empty() {
                    if let Some(tmpl) = milestone_templates::get_template_by_id(&m.template_id) {
                        let progress =
                            milestone_templates::compute_progress(tmpl.requirements, project_stats);
                        for req in &progress {
                            let icon = if req.met { "  ✓ " } else { "  ✗ " };
                            let icon_color = if req.met { theme.success } else { theme.danger };
                            let val_str = format!("{}/{}", req.current, req.target);
                            rows.push(ListItem::new(Line::from(vec![
                                Span::styled(icon, Style::default().fg(icon_color)),
                                Span::styled(
                                    req.label.clone(),
                                    Style::default().fg(theme.muted),
                                ),
                                Span::styled(
                                    format!(": {}", val_str),
                                    Style::default().fg(theme.text),
                                ),
                            ])));
                        }
                    }
                }

                rows
            })
            .collect()
    };

    let mil_list = List::new(milestone_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(content_border))
            .title(" Campaign Milestones — [Space] Toggle | [Delete] Slay | [m] New"),
    );
    f.render_widget(mil_list, bottom_split[1]);

    // Sección de Fellowship — solo se muestra si el proyecto es compartido
    if project.is_shared && chunks.len() > 3 {
        let fellowship_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[3]);

        let online_count = members.iter().filter(|m| m.3).count();
        let companion_items: Vec<ListItem> = if members.is_empty() {
            vec![ListItem::new("  No companions yet.").style(Style::default().fg(theme.muted))]
        } else {
            members.iter().map(|(_, username, role, is_online, _, _)| {
                let dot = if *is_online { "● " } else { "○ " };
                let dot_color = if *is_online { theme.success } else { theme.muted };
                let name_color = if *is_online { Color::White } else { Color::Gray };
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(dot, Style::default().fg(dot_color).add_modifier(Modifier::BOLD)),
                    Span::styled(username.clone(), Style::default().fg(name_color)),
                    Span::styled(format!(" [{}]", role), Style::default().fg(theme.muted)),
                ]))
            }).collect()
        };
        let companion_title = if online_count > 0 {
            format!(" Fellowship  ● {} online ", online_count)
        } else {
            " Fellowship Companions ".to_string()
        };
        let companion_list = List::new(companion_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent_color))
                .title(Span::styled(companion_title, Style::default().fg(theme.success).add_modifier(Modifier::BOLD))),
        );
        f.render_widget(companion_list, fellowship_split[0]);

        let activity_items: Vec<ListItem> = if activity.is_empty() {
            vec![ListItem::new("  No fellowship activity yet.").style(Style::default().fg(theme.muted))]
        } else {
            activity.iter().map(|(_, _, event_type, description, _, username, _)| {
                ListItem::new(format!("  [{}] {} — {}", event_type, description, username))
                    .style(Style::default().fg(theme.text))
            }).collect()
        };
        let activity_list = List::new(activity_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent_color))
                .title(" Fellowship Activity "),
        );
        f.render_widget(activity_list, fellowship_split[1]);
    }
}

// El modal más complejo del workspace — sirve tanto para crear como para editar tareas y sus steps
fn draw_task_modal(
    f: &mut Frame,
    title: &str,
    task_title: &str,
    task_desc: &str,
    desc_cursor: usize,
    priority: TaskPriority,
    due_date_type: DueDateType,
    due_date_val: &str,
    set_date_val: &str,
    focus_idx: usize,
    theme: &Theme,
    // None = NewTask (no steps section); Some(slice) = EditTask (always show steps section)
    steps_opt: Option<&[&Task]>,
    step_selected_idx: usize,
    hide_desc: bool,
    show_recurrence: bool,
    recurrence: Option<RecurrenceType>,
) {
    let show_steps = steps_opt.is_some();
    let steps: &[&Task] = steps_opt.unwrap_or(&[]);

    let has_due_value = matches!(due_date_type, DueDateType::InDays | DueDateType::Specific);
    // índice de foco dinámico para recurrencia y steps
    let set_date_focus_idx: usize = if has_due_value { 5 } else { 4 };
    let recurrence_focus_idx: usize = set_date_focus_idx + 1;
    let steps_focus_idx: usize = recurrence_focus_idx + 1;

    // Los índices de chunk cambian según si mostramos description o no — no manches qué rollo
    // Con desc:    [0]=Title [1]=Desc [2]=Prio/Due [3]=Recurrence? [4]=Steps? [last]=Help
    // Sin desc:    [0]=Title [1]=Prio/Due [2]=Recurrence? [3]=Steps? [last]=Help
    let prio_chunk = if hide_desc { 1 } else { 2 };
    let recur_chunk = prio_chunk + 1;
    let steps_chunk = recur_chunk + if show_recurrence { 1 } else { 0 };

    let base_height: u16 = match (hide_desc, show_steps) {
        (false, true)  => 85,
        (false, false) => 62,
        (true,  true)  => 72,
        (true,  false) => 45,
    };
    let modal_height = base_height + if show_recurrence { 4 } else { 0 };
    let area = centered_rect(65, modal_height, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;

    let mut constraints = vec![Constraint::Length(3)]; // Title
    if !hide_desc {
        constraints.push(Constraint::Length(10)); // Description
    }
    constraints.push(Constraint::Length(3)); // Priority & Due
    if show_recurrence {
        constraints.push(Constraint::Length(3)); // Recurrence
    }
    if show_steps {
        constraints.push(Constraint::Min(1)); // Steps (fills remaining space)
    }
    constraints.push(Constraint::Length(2)); // Help

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(constraints)
        .split(area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(main_block, area);

    // Title Input
    let title_border_style = if focus_idx == 0 {
        Style::default().fg(accent_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let title_owned;
    let title_text = if task_title.is_empty() {
        if focus_idx == 0 { "_" } else { "" }
    } else if focus_idx == 0 {
        title_owned = format!("{}_", task_title);
        &title_owned
    } else {
        task_title
    };
    let title_p = Paragraph::new(title_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(title_border_style)
            .title(" Quest Title "),
    );
    f.render_widget(title_p, chunks[0]);

    // Description con cursor real y scroll vertical — el cursor se resalta en la posición exacta
    if !hide_desc {
        let desc_border_style = if focus_idx == 1 {
            Style::default().fg(accent_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border)
        };

        // Cuántas líneas caben en el box de descripción — fijo en 8 por el tamaño del modal
        let visible_lines: usize = 8;

        // Split desc into lines and locate cursor line/col
        let lines: Vec<&str> = task_desc.split('\n').collect();
        let cursor = desc_cursor.min(task_desc.len());
        let (cursor_line, cursor_col) = {
            let mut pos = 0usize;
            let mut cl = 0usize;
            let mut cc = 0usize;
            for (i, line) in lines.iter().enumerate() {
                let end = pos + line.len();
                if cursor <= end || i + 1 == lines.len() {
                    cl = i;
                    cc = cursor - pos;
                    break;
                }
                pos = end + 1; // +1 for '\n'
            }
            (cl, cc)
        };

        // Scroll para mantener el cursor visible — se ajusta automáticamente al escribir
        let scroll_top = if cursor_line >= visible_lines {
            cursor_line + 1 - visible_lines
        } else {
            0
        };

        let desc_lines: Vec<Line> = if task_desc.is_empty() && focus_idx == 1 {
            vec![Line::from(Span::styled("_", Style::default().fg(accent_color)))]
        } else {
            lines.iter().enumerate()
                .skip(scroll_top)
                .take(visible_lines)
                .map(|(i, line)| {
                    if focus_idx == 1 && i == cursor_line {
                        // Render cursor highlight at cursor_col
                        let col = cursor_col.min(line.len());
                        let before = &line[..col];
                        let (cur_char, after) = if col < line.len() {
                            let next = line[col..].char_indices().nth(1).map(|(j, _)| col + j).unwrap_or(line.len());
                            (&line[col..next], &line[next..])
                        } else {
                            ("_", "")
                        };
                        Line::from(vec![
                            Span::raw(before),
                            Span::styled(cur_char, Style::default().fg(Color::Black).bg(theme.selection)),
                            Span::raw(after),
                        ])
                    } else {
                        Line::from(*line)
                    }
                })
                .collect()
        };

        let desc_p = Paragraph::new(desc_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(desc_border_style)
                    .title(" Quest Description "),
            );
        f.render_widget(desc_p, chunks[1]);
    }

    // Priority, Due Date, and Set Date row
    let row_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(38), Constraint::Percentage(30)])
        .split(chunks[prio_chunk]);

    // Priority selector field
    let prio_border_style = if focus_idx == 2 {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let priority_span = match priority {
        TaskPriority::Low => Span::styled(
            "Low",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        TaskPriority::Medium => Span::styled(
            "Medium",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        TaskPriority::High => Span::styled(
            "High",
            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
        ),
    };
    let prio_p = Paragraph::new(Line::from(vec![priority_span])).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(prio_border_style)
            .title(" Priority Level "),
    );
    f.render_widget(prio_p, row_chunks[0]);

    // Due Date Row
    match due_date_type {
        DueDateType::InDays | DueDateType::Specific => {
            let due_sub_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(row_chunks[1]);

            let type_border_style = if focus_idx == 3 {
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.border)
            };
            let type_text = if focus_idx == 3 {
                format!("< {} >", due_date_type.name())
            } else {
                due_date_type.name().to_string()
            };
            let type_p = Paragraph::new(type_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(type_border_style)
                    .title(if focus_idx == 3 { " Due Type  <-/-> " } else { " Due Type " }),
            );
            f.render_widget(type_p, due_sub_chunks[0]);

            let val_border_style = if focus_idx == 4 {
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.border)
            };
            let val_placeholder = if due_date_val.is_empty() {
                if focus_idx == 4 {
                    "_"
                } else {
                    match due_date_type {
                        DueDateType::Specific => "yyyy-mm-dd",
                        _ => "Value",
                    }
                }
            } else if focus_idx == 4 {
                &format!("{}_", due_date_val)
            } else {
                due_date_val
            };
            let val_title = match due_date_type {
                DueDateType::InDays => " Days ",
                _ => " Date (yyyy-mm-dd) ",
            };
            let val_p = Paragraph::new(val_placeholder)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(val_border_style)
                        .title(val_title),
                )
                .style(if due_date_val.is_empty() && focus_idx != 4 {
                    Style::default().fg(theme.muted)
                } else {
                    Style::default().fg(Color::White)
                });
            f.render_widget(val_p, due_sub_chunks[1]);
        }
        _ => {
            let due_border_style = if focus_idx == 3 {
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.border)
            };
            let due_text = if focus_idx == 3 {
                format!("< {} >", due_date_type.name())
            } else {
                due_date_type.name().to_string()
            };
            let due_p = Paragraph::new(due_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(due_border_style)
                    .title(if focus_idx == 3 { " Due Date  <-/-> " } else { " Due Date " }),
            );
            f.render_widget(due_p, row_chunks[1]);
        }
    }

    let set_border_style = if focus_idx == set_date_focus_idx {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let set_date_owned;
    let set_date_text = if set_date_val.is_empty() {
        if focus_idx == set_date_focus_idx { "_" } else { "yyyy-mm-dd" }
    } else if focus_idx == set_date_focus_idx {
        set_date_owned = format!("{}_", set_date_val);
        &set_date_owned
    } else {
        set_date_val
    };
    let set_p = Paragraph::new(set_date_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(set_border_style)
                .title(" Set Date "),
        )
        .style(if set_date_val.is_empty() && focus_idx != set_date_focus_idx {
            Style::default().fg(theme.muted)
        } else {
            Style::default().fg(Color::White)
        });
    f.render_widget(set_p, row_chunks[2]);

    // Campo de recurrencia — solo para tareas padre (no pasos)
    if show_recurrence {
        let recur_border_style = if focus_idx == recurrence_focus_idx {
            Style::default().fg(accent_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border)
        };
        let recur_label = match recurrence {
            None => Span::styled("None  (↻ Off)", Style::default().fg(theme.muted)),
            Some(RecurrenceType::Daily) => Span::styled("↻ Daily", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Some(RecurrenceType::Weekly) => Span::styled("↻ Weekly", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
            Some(RecurrenceType::Monthly) => Span::styled("↻ Monthly", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            Some(RecurrenceType::Yearly) => Span::styled("↻ Yearly", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
        };
        let recur_p = Paragraph::new(Line::from(vec![recur_label])).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(recur_border_style)
                .title(" Recurrence "),
        );
        f.render_widget(recur_p, chunks[recur_chunk]);
    }

    // Sección de steps — solo aparece en EditTask, muestra el progreso de steps con scroll
    let help_chunk_idx = if show_steps {
        let steps_border_style = if focus_idx == steps_focus_idx {
            Style::default().fg(accent_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border)
        };

        // Filas visibles del box de steps — dinámico según el espacio que queda en el modal
        let steps_visible: usize = chunks[steps_chunk].height.saturating_sub(2) as usize;
        let steps_scroll_top: usize = if focus_idx == steps_focus_idx && step_selected_idx >= steps_visible {
            step_selected_idx + 1 - steps_visible
        } else {
            0
        };

        let step_items: Vec<Line> = if steps.is_empty() {
            vec![
                Line::from(Span::styled(
                    "  No steps yet.",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    "  Press [n] or [+] to add a step.",
                    Style::default().fg(theme.muted),
                )),
            ]
        } else {
            steps.iter().enumerate().map(|(i, s)| {
                let check = if s.completed { "[x]" } else { "[ ]" };
                let due_str = s.due_date
                    .map(|d| format!(" - {}", d.format("%Y-%m-%d")))
                    .unwrap_or_default();
                let is_sel = focus_idx == steps_focus_idx && i == step_selected_idx;
                let style = if is_sel {
                    Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD)
                } else if s.completed {
                    Style::default().fg(theme.muted).add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(format!("  {} {}{}", check, s.title, due_str), style))
            }).collect()
        };

        let steps_title = format!(" Steps [{}/{}] ", steps.iter().filter(|s| s.completed).count(), steps.len());
        let steps_p = Paragraph::new(step_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(steps_border_style)
                    .title(steps_title),
            )
            .scroll((steps_scroll_top as u16, 0));
        f.render_widget(steps_p, chunks[steps_chunk]);
        steps_chunk + 1
    } else {
        prio_chunk + 1
    };

    // Help Text
    let helper_text = if focus_idx == steps_focus_idx && show_steps {
        "↑/↓: select step | Space: toggle | Del: remove | n/+: new step | Tab: back"
    } else if focus_idx == 1 && !hide_desc {
        "Tab: cycle field | Enter: newline | ESC: cancel"
    } else if show_recurrence && focus_idx == recurrence_focus_idx {
        "L/R/Space: cycle recurrence | Tab: next field | Enter: save | ESC: cancel"
    } else {
        "Tab: cycle field | Space/L/R: Priority & Due Type | Enter: save | ESC: cancel"
    };
    let helper = Paragraph::new(helper_text)
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, chunks[help_chunk_idx]);
}

// Modal sencillo para escribir la entrada de journal del día — sin mucho rollo
fn draw_journal_modal(f: &mut Frame, content: &str, theme: &Theme) {
    let area = centered_rect(55, 30, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Min(3),    // Content textbox
            Constraint::Length(2), // Help line
        ])
        .split(area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            " Write Daily Chronicle (Journal) ",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(main_block, area);

    let content_text = if content.is_empty() { "_" } else { content };
    let content_p = Paragraph::new(content_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent_color))
                .title(" Today's Chronicle Log "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(content_p, chunks[0]);

    let helper = Paragraph::new("Enter: save chronicle | ESC: cancel")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, chunks[1]);
}

// Primer paso de creación de milestone — el usuario elige entre Initiate, Veteran o Legendary
fn draw_tier_select_modal(f: &mut Frame, selected_idx: usize, theme: &Theme) {
    let area = centered_rect(60, 50, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            " Select Milestone Tier ",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // spacer
            Constraint::Length(3), // Tier 1
            Constraint::Length(1), // spacer
            Constraint::Length(3), // Tier 2
            Constraint::Length(1), // spacer
            Constraint::Length(3), // Tier 3
            Constraint::Min(1),    // spacer
            Constraint::Length(1), // help
        ])
        .split(inner);

    let tiers = [
        (Tier::Initiate, 0usize),
        (Tier::Veteran, 1usize),
        (Tier::Legendary, 2usize),
    ];

    for (tier, tier_idx) in &tiers {
        let is_sel = *tier_idx == selected_idx;
        let marker = if is_sel { "> " } else { "  " };
        let bg = if is_sel {
            Color::Rgb(30, 30, 50)
        } else {
            Color::Reset
        };
        let name_style = if is_sel {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let border_style = if is_sel {
            Style::default().fg(accent_color)
        } else {
            Style::default().fg(theme.border)
        };

        let tier_text = vec![
            Line::from(vec![
                Span::styled(marker, Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
                Span::styled(tier.name(), name_style),
                Span::styled(
                    format!("  ({})", tier.xp_range()),
                    Style::default().fg(theme.muted),
                ),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(tier.description(), Style::default().fg(theme.text)),
            ]),
        ];

        let chunk_idx = tier_idx * 2 + 1; // índices impares: 1, 3, 5 (los pares son spacers)
        let tier_p = Paragraph::new(tier_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)
                    .style(Style::default().bg(bg)),
            );
        f.render_widget(tier_p, chunks[chunk_idx]);
    }

    let help = Paragraph::new("↑/↓ Navigate | Enter: Select | ESC: Cancel")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[7]);
}

// Segundo paso — muestra todos los templates del tier elegido con sus requisitos y XP reward
fn draw_template_select_modal(f: &mut Frame, tier: u8, selected_idx: usize, theme: &Theme) {
    let area = centered_rect(70, 80, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;

    let tier_enum = Tier::from_u8(tier).unwrap_or(Tier::Initiate);
    let templates: Vec<&'static crate::milestone_templates::MilestoneTemplate> =
        milestone_templates::templates_for_tier(tier_enum).collect();

    let title = format!(
        " Select {} Milestone Template ",
        tier_enum.name()
    );

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = main_block.inner(area);
    f.render_widget(main_block, area);

    let n = templates.len();
    // Cada template ocupa un bloque de altura mínima 5 — nombre+XP, requisitos, flavor text
    let mut constraints: Vec<Constraint> = Vec::new();
    for _ in 0..n {
        constraints.push(Constraint::Min(5));
    }
    constraints.push(Constraint::Length(1)); // help line

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(inner);

    for (idx, tmpl) in templates.iter().enumerate() {
        let is_sel = idx == selected_idx;
        let marker = if is_sel { "> " } else { "  " };
        let border_style = if is_sel {
            Style::default().fg(accent_color)
        } else {
            Style::default().fg(theme.border)
        };
        let name_style = if is_sel {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(marker, Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
                Span::styled(tmpl.name, name_style),
                Span::styled(
                    format!("  +{} XP", tmpl.xp_reward),
                    Style::default().fg(theme.warning),
                ),
            ]),
        ];

        // Requirements line
        let req_parts: Vec<Span> = {
            let mut parts = vec![Span::styled("    Req: ", Style::default().fg(theme.muted))];
            for (i, req) in tmpl.requirements.iter().enumerate() {
                if i > 0 {
                    parts.push(Span::styled(", ", Style::default().fg(theme.muted)));
                }
                parts.push(Span::styled(
                    format!("{} ≥{}", req.short_label(), req.target()),
                    Style::default().fg(theme.text),
                ));
            }
            parts
        };
        lines.push(Line::from(req_parts));

        // Flavor text
        lines.push(Line::from(vec![Span::styled(
            format!("    \"{}\"", tmpl.flavor_text),
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::ITALIC),
        )]));

        let p = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style),
            )
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(p, chunks[idx]);
    }

    let help = Paragraph::new("↑/↓ Navigate | Enter: Create Milestone | ESC: Back")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[n]);
}

// Modal para asignar una tarea a un compañero del proyecto — solo proyectos compartidos llegan aquí
fn draw_assign_task_modal(f: &mut Frame, app: &App, selected_member_idx: usize, theme: &Theme) {
    let area = centered_rect(50, 40, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;
    let proj_id = app.active_project_id.unwrap().to_string();
    let members = app.db.get_project_members(&proj_id).unwrap_or_default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            " Assign Quest to Companion ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Min(5),    // Member list
            Constraint::Length(2), // Help line
        ])
        .split(block.inner(area));

    f.render_widget(block, area);

    let mut list_lines = Vec::new();
    if members.is_empty() {
        list_lines.push(Line::from(
            "  No companions in this campaign's fellowship yet.",
        ));
    } else {
        for (idx, m) in members.iter().enumerate() {
            let is_sel = idx == selected_member_idx;
            let marker = if is_sel { " > " } else { "   " };
            let style = if is_sel {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            list_lines.push(Line::from(vec![
                Span::styled(marker, Style::default().fg(accent_color)),
                Span::styled(format!("{} ({})", m.1, m.2), style),
            ]));
        }
    }

    let list_p = Paragraph::new(list_lines);
    f.render_widget(list_p, inner_layout[1]);

    let helper = Paragraph::new("↑↓: select companion | Enter: toggle assignment | ESC: close")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, inner_layout[2]);
}

// Popup para definir los permisos del scroll compartido: solo lectura, editable, o colaborativo
fn draw_share_note_modal(f: &mut Frame, permission_idx: usize, theme: &Theme) {
    let area = centered_rect(50, 30, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;
    let permissions = ["Read Only", "Editable", "Collaborative"];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            " Share Scroll Note Permissions ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Permission choices
            Constraint::Length(2), // Help line
        ])
        .split(block.inner(area));

    f.render_widget(block, area);

    let mut spans = Vec::new();
    for (idx, perm) in permissions.iter().enumerate() {
        let is_sel = idx == permission_idx;
        let style = if is_sel {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
                .bg(theme.panel)
        } else {
            Style::default().fg(theme.text)
        };
        spans.push(Span::styled(format!(" {} ", perm), style));
        if idx < permissions.len() - 1 {
            spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        }
    }

    let choice_p = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    f.render_widget(choice_p, inner_layout[1]);

    let helper = Paragraph::new("←→: change permission | Enter: share note | ESC: close")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, inner_layout[2]);
}

// Popup de visibilidad del journal — privado, visible al proyecto, o al fellowship completo
fn draw_journal_visibility_modal(f: &mut Frame, visibility_idx: usize, theme: &Theme) {
    let area = centered_rect(50, 30, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;
    let options = ["Private", "Campaign Visible", "Fellowship Visible"];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            " Set Journal Entry Visibility ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Visibility choices
            Constraint::Length(2), // Help line
        ])
        .split(block.inner(area));

    f.render_widget(block, area);

    let mut spans = Vec::new();
    for (idx, opt) in options.iter().enumerate() {
        let is_sel = idx == visibility_idx;
        let style = if is_sel {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
                .bg(theme.panel)
        } else {
            Style::default().fg(theme.text)
        };
        spans.push(Span::styled(format!(" {} ", opt), style));
        if idx < options.len() - 1 {
            spans.push(Span::styled(" | ", Style::default().fg(theme.muted)));
        }
    }

    let choice_p = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    f.render_widget(choice_p, inner_layout[1]);

    let helper = Paragraph::new("←→: change visibility | Enter: save visibility | ESC: close")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, inner_layout[2]);
}
