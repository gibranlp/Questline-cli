// ─────────────────────────────────────────────────────────────────────────────
// screens/prologue.rs — la secuencia introductoria de la historia, puro ambiente
// ─────────────────────────────────────────────────────────────────────────────

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{app::App, theme::Theme};

// órale, cada orden tiene su color — hay que que se vean chidos en el typewriter
const ORDER_COLORS: &[Color] = &[
    Color::Rgb(139, 92, 246),  // 0 Code Warlocks
    Color::Rgb(34, 197, 94),   // 1 Task Paladins
    Color::Rgb(56, 189, 248),  // 2 Mind Sages
    Color::Rgb(249, 115, 22),  // 3 Systems Architects
    Color::Rgb(232, 121, 249), // 4 Time Chronomancers
    Color::Rgb(250, 204, 21),  // 5 Arch Accountants
];

// todos los "sabores" de línea — cada uno dicta estilo y comportamiento
#[derive(Clone, Copy)]
pub enum LineKind {
    Empty,
    Title,
    ChapterNum,
    Separator,
    Normal,
    Muted,
    Dramatic,
    Highlight,
    Order(usize),
    Call,
    ColumnBreak, // ya no hace split, solo se trata como Empty
    Checkbox,
}

pub struct StoryLine {
    pub text: &'static str,
    pub kind: LineKind,
    pub instant: bool,
}

impl StoryLine {
    const fn t(text: &'static str, kind: LineKind) -> Self {
        Self { text, kind, instant: false }
    }
    const fn i(text: &'static str, kind: LineKind) -> Self {
        Self { text, kind, instant: true }
    }
}

// ── Page 0 — The Story So Far ─────────────────────────────────────────────────
pub static PROLOGUE: &[StoryLine] = &[
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("T H E   S T O R Y   S O   F A R", LineKind::Title),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("──────────────────────────────────────────────────────────────", LineKind::Separator),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("In the beginning, there was the Void.", LineKind::Normal),
    StoryLine::t("The Void was full of things no one had finished yet.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Then came the Age of Open Tabs,", LineKind::Normal),
    StoryLine::t("when people tried to organize their lives", LineKind::Normal),
    StoryLine::t("and instead buried themselves under scattered notebooks", LineKind::Normal),
    StoryLine::t("and browser windows numbering in the hundreds.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("From the rubble rose the Great Backlog.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("A force of accumulated neglect so vast,", LineKind::Normal),
    StoryLine::t("so patient,", LineKind::Normal),
    StoryLine::t("so deeply uninterested in your excuses,", LineKind::Normal),
    StoryLine::t("that entire organizations vanished beneath it.", LineKind::Dramatic),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Then the First Cursor appeared.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Someone typed \"Hello World.\"", LineKind::Dramatic),
    StoryLine::t("Light spread.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Six Great Orders emerged from the chaos.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("The Arch Accountants.",   LineKind::Order(5)),
    StoryLine::t("The Code Warlocks.",      LineKind::Order(0)),
    StoryLine::t("The Mind Sages.",         LineKind::Order(2)),
    StoryLine::t("The Task Paladins.",      LineKind::Order(1)),
    StoryLine::t("The Systems Architects.", LineKind::Order(3)),
    StoryLine::t("The Time Chronomancers.", LineKind::Order(4)),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("For a time, things were good.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Projects were completed.", LineKind::Muted),
    StoryLine::t("Goals were achieved.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("The Great Backlog retreated to the horizon,", LineKind::Normal),
    StoryLine::t("where it sat quietly, thinking.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("It was still thinking.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("That was the problem.", LineKind::Dramatic),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
];

// ── Page 1 — Chapter One: The Notification Swarm ──────────────────────────────
pub static CHAPTER_ONE: &[StoryLine] = &[
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("C H A P T E R   O N E", LineKind::ChapterNum),
    StoryLine::i("The Notification Swarm", LineKind::Title),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Long before the Orders tracked every task and timed every session,", LineKind::Normal),
    StoryLine::t("a quieter world existed.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Notifications were rare.", LineKind::Muted),
    StoryLine::t("Most were helpful.", LineKind::Muted),
    StoryLine::t("Some were urgent.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Then something changed.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("No one remembers exactly when the Swarm began.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("A single red circle appearing where there had been none.", LineKind::Normal),
    StoryLine::t("A banner arriving for a task that did not require one.", LineKind::Normal),
    StoryLine::t("A reminder about a reminder.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Then the numbers grew.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Pings multiplied.", LineKind::Normal),
    StoryLine::t("Badges propagated.", LineKind::Normal),
    StoryLine::t("Alerts arrived to inform heroes that other alerts had arrived.", LineKind::Dramatic),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("The Notification Sprites had been messengers once.", LineKind::Normal),
    StoryLine::t("Harmless creatures carrying messages across the Realm.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("But something fed them.", LineKind::Highlight),
    StoryLine::t("Something nurtured their numbers beyond any natural limit.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Without intervention,", LineKind::Normal),
    StoryLine::t("the Swarm would consume all remaining attention in the Realm.", LineKind::Dramatic),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("The Orders have convened.", LineKind::Normal),
    StoryLine::t("The diagnosis is clear.", LineKind::Normal),
    StoryLine::t("The remedy is simple.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Heroes must begin working again.", LineKind::Dramatic),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("T H E   C A L L   H A S   B E E N   I S S U E D", LineKind::ChapterNum),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("The Great Chronicle tracks the war against the Swarm.", LineKind::Normal),
    StoryLine::t("Every hero across the Realm contributes.", LineKind::Normal),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Every quest completed.", LineKind::Muted),
    StoryLine::t("Every focus session honored.", LineKind::Muted),
    StoryLine::t("Every sidequest fulfilled.", LineKind::Muted),
    StoryLine::t("Every reflection written.", LineKind::Muted),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("All of it weakens the Swarm.", LineKind::Highlight),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Checkbox),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Press  [ 8 ]  or  [ g ]  to see the state of the Realm.", LineKind::Call),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
];

pub fn page_lines(page: u8) -> &'static [StoryLine] {
    if page == 0 { PROLOGUE } else { CHAPTER_ONE }
}

// detecta cuántas líneas al inicio son "header" — las que preceden al primer texto de cuerpo
fn header_line_count(lines: &[StoryLine]) -> usize {
    lines.iter().take_while(|sl| {
        !matches!(sl.kind, LineKind::Normal | LineKind::Muted | LineKind::Dramatic
            | LineKind::Highlight | LineKind::Order(_) | LineKind::Call)
    }).count()
}

// todo el estilo visual de línea — centrado en todos los tipos, sin excepciones
fn style_line(text: String, kind: LineKind, class_color: Color) -> Line<'static> {
    match kind {
        LineKind::Empty | LineKind::ColumnBreak | LineKind::Checkbox => {
            Line::from("")
        }

        LineKind::Title => Line::from(Span::styled(
            text,
            Style::default().fg(class_color).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),

        LineKind::ChapterNum => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(180, 180, 180))
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),

        LineKind::Separator => Line::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(40, 40, 40)),
        ))
        .alignment(Alignment::Center),

        LineKind::Normal => Line::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(195, 195, 195)),
        ))
        .alignment(Alignment::Center),

        LineKind::Muted => Line::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(105, 105, 105)),
        ))
        .alignment(Alignment::Center),

        LineKind::Dramatic => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(210, 65, 65))
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),

        LineKind::Highlight => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(249, 115, 22))
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),

        LineKind::Order(idx) => {
            let color = ORDER_COLORS.get(idx).copied().unwrap_or(Color::White);
            Line::from(Span::styled(
                text,
                Style::default().fg(color).add_modifier(Modifier::ITALIC),
            ))
            .alignment(Alignment::Center)
        }

        LineKind::Call => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(6, 182, 212))
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    }
}

// construye el header fijo — título de página visible desde el primer frame
fn build_header_lines(lines: &[StoryLine], class_color: Color, n: usize) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    for sl in &lines[..n] {
        match sl.kind {
            LineKind::Empty => {}
            LineKind::ChapterNum => {
                out.push(Line::from(""));
                out.push(
                    Line::from(Span::styled(
                        sl.text.to_string(),
                        Style::default()
                            .fg(Color::Rgb(160, 160, 160))
                            .add_modifier(Modifier::BOLD),
                    ))
                    .alignment(Alignment::Center),
                );
            }
            LineKind::Title => {
                out.push(Line::from(""));
                out.push(
                    Line::from(vec![
                        Span::styled("  ✦  ", Style::default().fg(class_color)),
                        Span::styled(
                            sl.text.to_string(),
                            Style::default().fg(class_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("  ✦", Style::default().fg(class_color)),
                    ])
                    .alignment(Alignment::Center),
                );
                out.push(Line::from(""));
            }
            _ => {}
        }
    }
    out.push(
        Line::from(Span::styled(
            "─".repeat(60),
            Style::default().fg(Color::Rgb(35, 35, 35)),
        ))
        .alignment(Alignment::Center),
    );
    out.push(Line::from(""));
    out
}

// render principal — pantalla completa, borde en color de clase, todo centrado
pub fn draw(f: &mut Frame, app: &App, theme: &Theme) {
    let size = f.size();
    let class_color = theme.primary;

    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), size);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(class_color));
    let inner_area = outer.inner(size);
    f.render_widget(outer, size);

    let lines_def   = page_lines(app.prologue_page);
    let header_n    = header_line_count(lines_def);
    let body_def    = &lines_def[header_n..];
    let line_idx    = app.prologue_line_idx;
    let char_pos    = app.prologue_char_in_line;
    let page_done   = line_idx >= lines_def.len();
    let show_cursor = (app.intro_ticks / 10) % 2 == 0;
    let body_line_idx = line_idx.saturating_sub(header_n);

    let header_lines = build_header_lines(lines_def, class_color, header_n);
    let header_h     = header_lines.len() as u16;
    let footer_h: u16 = 2;

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Min(1),
            Constraint::Length(footer_h),
        ])
        .split(inner_area);

    // ── Header: título de página, siempre visible ────────────────────────────
    f.render_widget(
        Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center),
        inner_chunks[0],
    );

    // ── Cuerpo: columna única, todo centrado ─────────────────────────────────
    let mut rendered: Vec<Line<'static>> = Vec::new();

    for (i, sl) in body_def.iter().enumerate() {
        if !page_done && i > body_line_idx {
            break;
        }
        if !page_done && line_idx < header_n {
            break;
        }

        // Checkbox solo visible cuando la página termina
        if matches!(sl.kind, LineKind::Checkbox) {
            if page_done {
                let (cb_text, color) = if app.prologue_skip_checked {
                    ("[x]  Don't show this again", class_color)
                } else {
                    ("[ ]  Don't show this again", Color::Rgb(60, 60, 60))
                };
                rendered.push(
                    Line::from(Span::styled(cb_text.to_string(), Style::default().fg(color)))
                        .alignment(Alignment::Center),
                );
            } else {
                rendered.push(Line::from(""));
            }
            continue;
        }

        if matches!(sl.kind, LineKind::ColumnBreak) {
            rendered.push(Line::from(""));
            continue;
        }

        let is_current      = !page_done && i == body_line_idx;
        let full_char_count = sl.text.chars().count();

        let display: String = if is_current {
            sl.text.chars().take(char_pos.min(full_char_count)).collect()
        } else {
            sl.text.to_string()
        };

        let still_typing = is_current && char_pos < full_char_count;
        let with_cursor = if still_typing && show_cursor {
            format!("{}▌", display)
        } else {
            display
        };

        rendered.push(style_line(with_cursor, sl.kind, class_color));
    }

    // auto-scroll para mantener el cursor siempre visible
    let viewport = inner_chunks[1].height as usize;
    let scroll_y  = rendered.len().saturating_sub(viewport) as u16;

    f.render_widget(
        Paragraph::new(rendered)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false })
            .scroll((scroll_y, 0))
            .alignment(Alignment::Center),
        inner_chunks[1],
    );

    // ── Footer ──────────────────────────────────────────────────────────────
    let hint = if app.prologue_page == 1 && page_done {
        "[ SPACE ]  begin your quest    ·    [ x ]  toggle don't show again"
    } else if page_done {
        "[ SPACE ]  continue  ·  Chapter One awaits"
    } else {
        "[ SPACE ]  skip"
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::Rgb(80, 80, 80)),
        )))
        .alignment(Alignment::Center),
        inner_chunks[2],
    );
}
