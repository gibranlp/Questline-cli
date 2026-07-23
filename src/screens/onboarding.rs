// ─────────────────────────────────────────────────────────────────────────────
// screens/onboarding.rs — la creación del personaje, donde el héroe elige su clase
// ─────────────────────────────────────────────────────────────────────────────

use crate::models::ClassType;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

// cuál sección tiene el foco — el nombre o la lista de clases, sencillo pero necesario
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingFocus {
    NameInput,
    ClassSelect,
}

// arma toda la pantalla de inicio — nombre, clase, detalle y footer; no manches cuántos widgets
pub fn draw(
    f: &mut Frame,
    username: &str,
    selected_idx: usize,
    focus: OnboardingFocus,
    classes: &[ClassType],
    error: Option<&str>,
) {
    let size = f.size();

    // el color del tema cambia según la clase seleccionada — puro dinamismo, se ve muy chido
    let highlighted_class = classes[selected_idx];
    let class_theme = Theme::for_class(highlighted_class);
    let highlight_color = class_theme.primary;

    // layout principal: header + nombre + error + panel de clase + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Name Input
            Constraint::Length(1), // Error message
            Constraint::Min(10),   // Class Selection panel
            Constraint::Length(3), // Help Info
        ])
        .split(size);

    // 1. Render Header
    let header = Paragraph::new("Choose Your Calling")
        .style(
            Style::default()
                .fg(class_theme.warning)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // campo de nombre: cambia el borde y el placeholder según si tiene foco o no
    let name_border_style = if focus == OnboardingFocus::NameInput {
        Style::default()
            .fg(highlight_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(class_theme.muted)
    };
    let name_text = if username.is_empty() {
        if focus == OnboardingFocus::NameInput {
            "_"
        } else {
            "Adventure awaits..."
        }
    } else {
        username
    };
    let name_p = Paragraph::new(name_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(name_border_style)
                .title(" Declare Your Name "),
        )
        .style(
            if username.is_empty() && focus != OnboardingFocus::NameInput {
                Style::default().fg(class_theme.muted)
            } else {
                Style::default().fg(Color::White)
            },
        );
    f.render_widget(name_p, chunks[1]);

    // 2b. Render error message (if any)
    let error_p = Paragraph::new(error.unwrap_or(""))
        .style(Style::default().fg(class_theme.danger).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(error_p, chunks[2]);

    // 3. Render Middle panels (Left: Class List, Right: Details)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[3]);

    // lista de clases: resalta la seleccionada con fondo de color — se siente como un RPG de verdad
    let list_border_style = if focus == OnboardingFocus::ClassSelect {
        Style::default()
            .fg(highlight_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(class_theme.muted)
    };

    let list_items: Vec<ListItem> = classes
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == selected_idx {
                Style::default()
                    .fg(Color::Black)
                    .bg(highlight_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(class_theme.text)
            };
            ListItem::new(format!("  {}  ", c.name())).style(style)
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(list_border_style)
            .title(" Select RPG Class "),
    );
    f.render_widget(list, body_chunks[0]);

    // panel derecho: lore, descripción, motto y el primer poder — solo muestra el nivel 1 por ahora
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(highlight_color))
        .title(format!(" {} Details ", highlighted_class.name()));

    let first_power = highlighted_class
        .powers()
        .into_iter()
        .next()
        .unwrap_or((1, "None", ""));

    let mut details_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("~ {} ~", highlighted_class.order()),
            Style::default()
                .fg(highlight_color)
                .add_modifier(Modifier::ITALIC | Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            highlighted_class.lore(),
            Style::default()
                .fg(highlight_color)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Description: ", Style::default().fg(class_theme.muted)),
            Span::styled(
                highlighted_class.description(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Motto: ", Style::default().fg(class_theme.muted)),
            Span::styled(
                format!("\"{}\"", highlighted_class.flavor()),
                Style::default()
                    .fg(highlight_color)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Starting Power (Level 1):",
            Style::default()
                .fg(class_theme.warning)
                .add_modifier(Modifier::UNDERLINED),
        )),
        Line::from(vec![
            Span::styled(
                format!("  {} - ", first_power.1),
                Style::default()
                    .fg(highlight_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(first_power.2, Style::default().fg(class_theme.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Class Passives (Always Active):",
            Style::default()
                .fg(class_theme.warning)
                .add_modifier(Modifier::UNDERLINED),
        )),
    ];
    for passive in highlighted_class.passive_description().split("  |  ") {
        details_text.push(Line::from(vec![
            Span::styled("  ✦ ", Style::default().fg(highlight_color)),
            Span::styled(passive.trim(), Style::default().fg(Color::White)),
        ]));
    }

    let details_p = Paragraph::new(details_text)
        .block(details_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(details_p, body_chunks[1]);

    // 4. Render Bottom Help
    let help_text = match focus {
        OnboardingFocus::NameInput => "Press [Tab] to switch to class selection.",
        OnboardingFocus::ClassSelect => {
            "Use [Up/Down] to navigate classes, [Enter] to embark."
        }
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(class_theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[4]);
}
