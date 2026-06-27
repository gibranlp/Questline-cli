// ─────────────────────────────────────────────────────────────────────────────
// screens/prologue.rs — la secuencia introductoria de la historia, puro ambiente
// ─────────────────────────────────────────────────────────────────────────────

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{app::App, theme::Theme};

// órale, cada orden tiene su color — hay que que se vean chidos en el typewriter
const ORDER_COLORS: &[Color] = &[
    Color::Rgb(139, 92, 246),  // 0 Code Warlocks     — violet
    Color::Rgb(34, 197, 94),   // 1 Task Paladins      — green
    Color::Rgb(56, 189, 248),  // 2 Mind Sages         — sky
    Color::Rgb(249, 115, 22),  // 3 Systems Architects — orange
    Color::Rgb(232, 121, 249), // 4 Time Chronomancers — fuchsia
    Color::Rgb(250, 204, 21),  // 5 Arch Accountants   — yellow
];

// pues aquí están todos los "sabores" de línea — cada uno tiene su estilo y comportamiento
#[derive(Clone, Copy)]
pub enum LineKind {
    Empty,
    Title,       // e.g. "T H E   S T O R Y   S O   F A R"  — gold, bold, centered
    ChapterNum,  // "C H A P T E R   O N E"                  — primary, bold, centered
    Separator,   // decorative rule                           — dim, centered, instant
    Normal,      // regular narrative text
    Muted,       // quiet/reflective lines
    Dramatic,    // short punchy sentences                    — red-ish, bold
    Highlight,   // important moments                         — orange, bold
    Order(usize),// Order names with class colour, italic
    Call,        // epic call-to-action line                  — cyan, bold, centered
    ColumnBreak, // marks where the two-column split happens (instant, renders empty)
    Checkbox,    // the "don't show again" toggle (instant, rendered dynamically in two-col)
}

pub struct StoryLine {
    pub text: &'static str,
    pub kind:  LineKind,
    pub instant: bool, // show immediately, no typewriter
}

// t = typewriter, i = instant — así de simple, cuate
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
    StoryLine::t("T H E   S T O R Y   S O   F A R", LineKind::Title),
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
    StoryLine::t("    The Arch Accountants.",     LineKind::Order(5)),
    StoryLine::t("    The Code Warlocks.",        LineKind::Order(0)),
    StoryLine::t("    The Mind Sages.",           LineKind::Order(2)),
    StoryLine::t("    The Task Paladins.",        LineKind::Order(1)),
    StoryLine::t("    The Systems Architects.",   LineKind::Order(3)),
    StoryLine::t("    The Time Chronomancers.",   LineKind::Order(4)),
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
    StoryLine::t("C H A P T E R   O N E", LineKind::ChapterNum),
    StoryLine::t("The Notification Swarm", LineKind::Title),
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
    // ColumnBreak: everything after this renders in the right column when done
    StoryLine::i("", LineKind::ColumnBreak),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("T H E   C A L L   H A S   B E E N   I S S U E D", LineKind::ChapterNum),
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
    // Checkbox: rendered dynamically based on prologue_skip_checked
    StoryLine::i("", LineKind::Checkbox),
    StoryLine::i("", LineKind::Empty),
    StoryLine::t("Press  [ 8 ]  or  [ g ]  to see the state of the Realm.", LineKind::Call),
    StoryLine::i("", LineKind::Empty),
    StoryLine::i("", LineKind::Empty),
];

// despacha la página correcta — página 0 es el inicio épico, página 1 el call to action
pub fn page_lines(page: u8) -> &'static [StoryLine] {
    if page == 0 { PROLOGUE } else { CHAPTER_ONE }
}

// convierte el texto crudo en un Line con estilo según su LineKind — aquí vive todo el drama visual
fn style_line(text: String, kind: LineKind, theme: &Theme) -> Line<'static> {
    match kind {
        LineKind::Empty | LineKind::ColumnBreak | LineKind::Checkbox => Line::from(""),

        LineKind::Title => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(245, 158, 11))
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),

        LineKind::ChapterNum => Line::from(Span::styled(
            text,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),

        LineKind::Separator => Line::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(45, 45, 45)),
        ))
        .alignment(Alignment::Center),

        LineKind::Normal => Line::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(195, 195, 195)),
        )),

        LineKind::Muted => Line::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(105, 105, 105)),
        )),

        LineKind::Dramatic => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(210, 65, 65))
                .add_modifier(Modifier::BOLD),
        )),

        LineKind::Highlight => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Rgb(249, 115, 22))
                .add_modifier(Modifier::BOLD),
        )),

        LineKind::Order(idx) => {
            let color = ORDER_COLORS.get(idx).copied().unwrap_or(Color::White);
            Line::from(Span::styled(
                text,
                Style::default().fg(color).add_modifier(Modifier::ITALIC),
            ))
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

// el render principal — maneja tanto el modo typewriter como la vista final de dos columnas
pub fn draw(f: &mut Frame, app: &App, theme: &Theme) {
    let size = f.size();

    // Pure black canvas
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        size,
    );

    // Horizontal: 15% empty | 70% text column | 15% empty
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(size);

    // Vertical: scrollable content | 3-row hint footer
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(cols[1]);

    let lines_def = page_lines(app.prologue_page);
    let line_idx  = app.prologue_line_idx;
    let char_pos  = app.prologue_char_in_line;
    let page_done = line_idx >= lines_def.len();

    // Blink cursor every ~500 ms (10 ticks x 50 ms)
    let show_cursor = (app.intro_ticks / 10) % 2 == 0;

    if app.prologue_page == 1 && page_done {
        // página 1 terminada = modo épico de dos columnas: historia a la izq, CTA a la der
        // ── Two-column final view ─────────────────────────────────────────────
        let split_at = lines_def
            .iter()
            .position(|sl| matches!(sl.kind, LineKind::ColumnBreak))
            .unwrap_or(lines_def.len());

        let left_lines: Vec<Line<'static>> = lines_def[..split_at]
            .iter()
            .map(|sl| style_line(sl.text.to_string(), sl.kind, theme))
            .collect();

        let right_lines: Vec<Line<'static>> = lines_def[split_at + 1..]
            .iter()
            .map(|sl| {
                if matches!(sl.kind, LineKind::Checkbox) {
                    let (cb_text, color) = if app.prologue_skip_checked {
                        ("  [x] Don't show this again", Color::Rgb(6, 182, 212))
                    } else {
                        ("  [ ] Don't show this again", Color::Rgb(70, 70, 70))
                    };
                    Line::from(Span::styled(
                        cb_text.to_string(),
                        Style::default().fg(color),
                    ))
                } else {
                    style_line(sl.text.to_string(), sl.kind, theme)
                }
            })
            .collect();

        let two_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);

        // Left: story — auto-scroll to bottom so the dramatic ending is visible
        let left_scroll = left_lines.len().saturating_sub(two_cols[0].height as usize) as u16;
        f.render_widget(
            Paragraph::new(left_lines)
                .block(Block::default().borders(Borders::NONE))
                .wrap(Wrap { trim: false })
                .scroll((left_scroll, 0)),
            two_cols[0],
        );

        // Right: CTA — always from the top (fits in viewport)
        f.render_widget(
            Paragraph::new(right_lines)
                .block(Block::default().borders(Borders::NONE))
                .wrap(Wrap { trim: false }),
            two_cols[1],
        );
    } else {
        // modo typewriter: va línea por línea, carácter por carácter — qué rollo tan chido
        // ── Single-column typewriter view ─────────────────────────────────────
        let mut rendered: Vec<Line<'static>> = Vec::new();

        for (i, sl) in lines_def.iter().enumerate() {
            if i > line_idx {
                break;
            }

            // ColumnBreak and Checkbox render as empty lines during typing
            if matches!(sl.kind, LineKind::ColumnBreak | LineKind::Checkbox) {
                rendered.push(Line::from(""));
                continue;
            }

            let is_current = i == line_idx && !page_done;
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

            rendered.push(style_line(with_cursor, sl.kind, theme));
        }

        // Auto-scroll so the latest line stays visible
        let viewport = rows[0].height as usize;
        let scroll_y = rendered.len().saturating_sub(viewport) as u16;

        f.render_widget(
            Paragraph::new(rendered)
                .block(Block::default().borders(Borders::NONE))
                .wrap(Wrap { trim: false })
                .scroll((scroll_y, 0)),
            rows[0],
        );
    }

    // Footer hint
    let hint = if app.prologue_page == 1 && page_done {
        "[ SPACE ]  begin your quest    press [x] to toggle don't show again"
    } else if page_done {
        "[ SPACE ]  continue  ·  Chapter One awaits"
    } else {
        "[ SPACE ]  skip"
    };

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::Rgb(120, 120, 120)),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE)),
        rows[1],
    );
}
