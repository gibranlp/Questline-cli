// ─────────────────────────────────────────────────────────────────────────────
// screens/about.rs — info del app: versión, changelog, créditos y reporte de bugs
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

// Banco de datos curiosos del Questline lore — se elige uno random por sesión
const FACTS: &[&[&str]] = &[
    &[
        "• The Great Backlog grows stronger when ignored.",
    ],
    &[
        "• Scope Dragons feed on \"just one more feature.\"",
    ],
    &[
        "• The Zen Tree has witnessed every excuse ever recorded.",
    ],
    &[
        "• Future You has filed several complaints.",
    ],
    &[
        "• The Chronicle remembers everything.",
    ],
    &[
        "• The Chronicle records achievements, mistakes,",
        "  and suspiciously optimistic deadlines.",
        "• No hero has ever defeated the Great Backlog.",
        "  Some merely keep it contained.",
        "• Meeting Mimics disguise themselves as productive discussions.",
        "• The first task is always stronger than the second.",
        "• The second task is stronger than the first.",
        "• Nobody has successfully explained this phenomenon.",
    ],
    &[
        "• The Fellowship was created after one hero attempted",
        "  to manage everything alone.",
        "• The Fellowship still debates whether that hero survived.",
        "• The average Scope Dragon begins as a harmless feature request.",
        "• Every abandoned note eventually becomes archaeology.",
        "• Every archaeological discovery was once an unread note.",
    ],
    &[
        "• The Zen Tree grows from consistency, not intensity.",
        "• The Zen Tree does not care about your motivational speeches.",
        "• The Zen Tree prefers action.",
        "• The Zen Tree has heard \"I'll do it tomorrow\"",
        "  approximately 8.4 million times.",
        "• The Zen Tree remains unconvinced.",
    ],
    &[
        "• Future You remembers every promise.",
        "• Past You keeps making new ones.",
        "• Present You inherited the negotiations.",
        "• Future You has requested fewer side quests.",
        "• Future You is currently disappointed in your browser tab count.",
    ],
    &[
        "• A Deadline Wraith can smell procrastination from several weeks away.",
        "• Deadline Wraith sightings increase dramatically before project launches.",
        "• Most Deadline Wraiths are self-inflicted.",
        "• Researchers continue to investigate why they become visible",
        "  at 4:57 PM on Fridays.",
    ],
    &[
        "• The average adventurer creates three tasks for every one completed.",
        "• Legendary adventurers create four.",
        "• Nobody knows why.",
    ],
    &[
        "• Every completed task weakens the Great Backlog.",
        "• The Great Backlog considers this extremely rude.",
        "• The Great Backlog would like to remind you about",
        "  seventeen unfinished projects.",
    ],
    &[
        "• The Coffee Machine has solved more problems than several management teams.",
        "• The Coffee Machine refuses further comment.",
        "• The Coffee Machine is currently on break.",
    ],
    &[
        "• The Chronicle once recorded a completely empty inbox.",
        "• Most scholars consider the account fictional.",
        "• Others believe it was a miracle.",
    ],
    &[
        "• Scope Dragons fear only two things:",
        "  consistency and project cancellation.",
    ],
    &[
        "• The oldest known unfinished task predates written history.",
        "• Historians suspect it was labeled \"quick fix.\"",
    ],
    &[
        "• Meeting Mimics reproduce when someone says:",
        "  \"Let's schedule another meeting.\"",
    ],
    &[
        "• The Archive is not a graveyard.",
        "• It is a retirement village for completed adventures.",
    ],
    &[
        "• The first checkbox is ceremonial.",
        "• The second checkbox is momentum.",
        "• The third checkbox is where legends begin.",
    ],
    &[
        "• Every productivity system eventually becomes a productivity project.",
        "• Questline is aware of the irony.",
    ],
    &[
        "• There are more unfinished side projects in existence",
        "  than stars visible in the night sky.",
        "• Several constellations are actually abandoned repositories.",
    ],
    &[
        "• The Six Great Orders agree on very little.",
        "• All six agree that unnecessary meetings are evil.",
    ],
    &[
        "• The Task Paladins believe discipline conquers all.",
        "• The Code Warlocks automated discipline.",
        "• The debate continues.",
    ],
    &[
        "• Mind Sages can remember where they saved the file.",
        "• Allegedly.",
    ],
    &[
        "• Systems Architects can organize anything.",
        "• Except their Downloads folder.",
    ],
    &[
        "• Time Chronomancers know exactly where their hours go.",
        "• They are rarely happy about it.",
    ],
    &[
        "• Arch Accountants can account for every coin.",
        "• They cannot account for where the weekend went.",
    ],
    &[
        "• Code Warlocks claim every bug is obvious in hindsight.",
        "• The bugs remain unconvinced.",
    ],
    &[
        "• The Great Backlog is currently doing pushups.",
        "• It recommends you do not underestimate it.",
    ],
    &[
        "• Every hero starts with a single task.",
        "• Every villain starts with \"I'll do it later.\"",
    ],
    &[
        "• The most dangerous phrase in the realm is:",
        "  \"This should only take five minutes.\"",
    ],
    &[
        "• Thousands have entered the Realm of Productivity.",
        "• Most got distracted on the way.",
    ],
    &[
        "• The Chronicle does not judge.",
        "• The Chronicle merely records.",
    ],
    &[
        "• Questline cannot complete your tasks.",
        "• It can, however, make ignoring them significantly more embarrassing.",
    ],
    &["• Somewhere, right now, an unread notification is gaining experience points."],
    &["• A project without notes eventually becomes folklore."],
    &["• Every masterpiece was once a task someone did not want to start."],
    &["• The road to greatness is surprisingly administrative."],
    &[
        "• The final boss is rarely difficult.",
        "• Reaching the final boss is the difficult part.",
    ],
    &[
        "• The Great Backlog would like a word with you.",
        "• Several words, actually.",
    ],
];

fn divider(accent: Color) -> Line<'static> {
    Line::from(Span::styled(
        "  ═══════════════════════════════════════════════════════════════",
        Style::default().fg(accent),
    ))
}

fn cl_div() -> Line<'static> {
    Line::from(Span::styled(
        "  ──────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    ))
}

// Arma todas las líneas del changelog — cada versión con su codename y lista de cambios
fn changelog_lines(theme: &Theme, accent: Color) -> Vec<Line<'static>> {
    // Historial de versiones hardcodeado — pues hay que actualizar esto con cada release
    const VERSIONS: &[(&str, &str, &str, &[&str])] = &[
        (
            "v1.0.6", "Jun 26, 2026",
            "The Notification Swarm",
            &[
                "Living Chapters: global cooperative quests",
                "Chapter One: The Notification Swarm",
                "Chapter reward: 5 000 XP per contributor",
                "Realm Activity Feed on Great Chronicle",
                "Chapter completion modal (shown once)",
                "Clipboard fixed for macOS & Windows",
                "Sync: push-then-pull ordering",
                "Sync: UNIX epoch timestamp fallback",
                "Sync: batch UPDATE replaces N queries",
                "Sync: N+1 fix for project lookups",
                "Sync: index on sync_log",
                "API: retry with exponential backoff",
                "Server: heartbeat throttled (60 s)",
                "Server: atomic chapter completion",
                "Server: mail() errors logged",
            ],
        ),
        (
            "v1.0.5", "Jun 24, 2026",
            "The Instant Messenger Update",
            &[
                "Real-time fellowship chat (4 s)",
                "Duplicate message prevention",
                "Fixed sender showing hostname",
                "Export / Restore Profile",
                "Shared project flag propagation",
                "Task modal border bleed fixed",
                "Per-frame SQLite reads eliminated",
            ],
        ),
        (
            "v1.0.4", "Jun 23, 2026",
            "The Distribution Chronicle",
            &[
                "Cross-platform binary releases",
                "One-command installer (Linux/macOS)",
                "PowerShell installer (Windows)",
                "AUR & Homebrew support",
                "Ritual sync error fixes",
            ],
        ),
        (
            "v1.0.3", "Jun 22, 2026",
            "The Great Refactor",
            &[
                "Full UI / UX overhaul",
                "Class passive abilities (all 6)",
                "Memory Fragments in Lore Library",
                "In-app bug report system",
                "Shared project workflow fixes",
                "Sync interval tuning",
            ],
        ),
        (
            "v1.0.2", "Jun 21, 2026",
            "Sync & Media",
            &[
                "Shared key display in Sync pane",
                "macOS soundscape playback fixed",
                "Multi-device sync improvements",
            ],
        ),
        (
            "v1.0.1", "Jun 20, 2026",
            "First Patch",
            &[
                "Post-launch stability fixes",
                "Release engineering setup",
            ],
        ),
        (
            "v1.0.0", "Jun 20, 2026",
            "The First Chronicle",
            &[
                "Initial public release",
                "Projects, Tasks, and the Chronicle",
                "Six playable classes with lore",
                "Zen Tree (7 growth stages)",
                "Focus sessions & 8 soundscapes",
                "Daily Adventures & streaks",
                "Cloud sync via Ed25519 identity",
                "Fellowship & shared projects",
                "Lore Library with achievements",
            ],
        ),
    ];

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  CHANGELOG",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        cl_div(),
    ];

    // Itera versiones y agrega separador entre ellas — el último no lleva divider
    for (i, (ver, date, codename, changes)) in VERSIONS.iter().enumerate() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(*ver, Style::default().fg(accent).add_modifier(Modifier::BOLD)),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(*date, Style::default().fg(theme.muted)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  {}", codename),
            Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
        )));
        lines.push(Line::from(""));
        for change in *changes {
            lines.push(Line::from(vec![
                Span::styled("  ▸ ", Style::default().fg(accent)),
                Span::styled(*change, Style::default().fg(theme.text)),
            ]));
        }
        lines.push(Line::from(""));
        if i < VERSIONS.len() - 1 {
            lines.push(cl_div());
        }
    }

    lines.push(cl_div());
    lines.push(Line::from(""));
    lines
}

// Pantalla principal about — dos columnas: info/lore a la izq, changelog a la der
// Ambas comparten el mismo scroll_offset, qué rollo si los contenidos tienen largo diferente
pub fn draw(f: &mut ratatui::Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let accent = theme.primary;
    // La versión viene del Cargo.toml en tiempo de compilación — no hay que hardcodearla
    let version = env!("CARGO_PKG_VERSION");

    // Elige el grupo de facts con el seed aleatorio guardado en app state
    let fact_group = FACTS[app.about_fact_seed as usize % FACTS.len()];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        divider(accent),
        Line::from(Span::styled(
            "                         ABOUT QUESTLINE",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  Questline is a terminal-first productivity RPG designed",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "  for adventurers, builders, thinkers, coders, planners,",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "  and anyone attempting to bring order to the Great Backlog.",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Rather than treating productivity as a list of obligations,",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  Questline treats it as a long campaign.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Every task is a quest.",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Every project is an adventure.",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Every completed checkbox leaves a mark upon the Chronicle.",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Features include:",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for feat in &[
        "Projects, Tasks, Notes, and Journals",
        "Fellowship Collaboration and Shared Realms",
        "Project Chronicles and Persistent Chat",
        "RPG Progression and Character Classes",
        "Daily Adventures and Streak Tracking",
        "The Zen Tree Growth System",
        "Focus Sessions and Ambient Soundscapes",
        "Local-First Architecture with Cloud Synchronization",
        "A Ridiculous Amount of Lore",
    ] {
        lines.push(Line::from(vec![
            Span::styled("  • ", Style::default().fg(accent)),
            Span::styled(*feat, Style::default().fg(Color::White)),
        ]));
    }

    lines.extend([
        Line::from(""),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  The World of Questline",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  According to the Chronicle, the world began long before the First Cursor.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  The Six Great Orders emerged to combat the spread of Chaos,",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  Scope Dragons, Meeting Mimics, Deadline Wraiths,",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  and the ever-growing Great Backlog.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Though many heroes have fallen, countless tasks have been completed in their name.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  The war continues.",
            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  The Creator",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Questline was created by Gibranlp.",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Originally conceived as a personal productivity tool, it gradually evolved",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  into a terminal-based RPG where projects became adventures and task",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  completion became a form of heroism.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Questline is built with Rust, powered by persistence, and maintained",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  through equal parts discipline, curiosity, and caffeine.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  Help Defeat the Great Backlog (Support)",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  The Great Backlog is ancient.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Each day it grows stronger, feeding on unfinished features,",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  postponed ideas, and the phrase \"I'll do it tomorrow.\"",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Your support helps keep the forces of productivity supplied,",
            Style::default().fg(theme.text),
        )),
        Line::from(Span::styled(
            "  the servers running, and the Scope Dragons at bay.",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Future You has already voted in favor of this decision.",
            Style::default().fg(theme.muted).add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  https://ko-fi.com/Y4H021XN7F",
            Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )),
        Line::from(""),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  Technical Information",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ]);

    for (label, value) in &[
        ("Application", "Questline"),
        ("Language   ", "Rust"),
        ("Interface  ", "Ratatui + Crossterm"),
        ("Database   ", "SQLite"),
        ("Sync       ", "Questline Cloud"),
        ("License    ", "Proprietary"),
    ] {
        lines.push(Line::from(vec![
            Span::styled(format!("  {label} : "), Style::default().fg(theme.muted)),
            Span::styled(*value, Style::default().fg(Color::White)),
        ]));
    }
    // La versión se inyecta en build time — siempre está actualizada, órale
    lines.push(Line::from(vec![
        Span::styled("  Version    : ", Style::default().fg(theme.muted)),
        Span::styled(
            version,
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.extend([
        Line::from(""),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  \"Motivation is temporary.",
            Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
        )),
        Line::from(Span::styled(
            "   The Chronicle is forever.\"",
            Style::default().fg(Color::White).add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "                                          — The Chronicle",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        divider(accent),
        Line::from(""),
        Line::from(Span::styled(
            "  Did You Know?",
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ]);

    // Agrega las líneas del fact group seleccionado — puede ser uno o varios
    for fact_line in fact_group {
        lines.push(Line::from(Span::styled(
            format!("  {fact_line}"),
            Style::default().fg(theme.text).add_modifier(Modifier::ITALIC),
        )));
    }

    lines.extend([
        Line::from(""),
        divider(accent),
        Line::from(""),
    ]);

    // Guarda el total de líneas para que el app pueda calcular el scroll máximo
    app.about_content_lines.set(lines.len() as u16);

    let cl_lines = changelog_lines(theme, accent);

    // Layout de dos columnas — 55% about, 45% changelog
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let left = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.about_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(Line::from(vec![
                    Span::styled(
                        " About Questline  [↑↓] scroll  [R] Send Report  ",
                        Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "[Support] ",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ])),
        );
    f.render_widget(left, cols[0]);

    // El changelog usa el mismo scroll que el about — ambos se mueven juntos
    let right = Paragraph::new(cl_lines)
        .wrap(Wrap { trim: false })
        .scroll((app.about_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    " Version History ",
                    Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(right, cols[1]);
}
