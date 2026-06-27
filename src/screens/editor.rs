// ─────────────────────────────────────────────────────────────────────────────
// screens/editor.rs — el editor de texto para notas y entradas de journal
// ─────────────────────────────────────────────────────────────────────────────

use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

// todo el estado del editor vive aquí — título, líneas, cursor y si estamos editando el título
#[derive(Debug, Clone)]
pub struct EditorState {
    pub title: String,
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub note_id: Option<uuid::Uuid>,
    pub project_id: uuid::Uuid,
    pub editing_title: bool,
    pub codex_id: Option<uuid::Uuid>,
}

// retrocede el índice hasta un límite de char válido en UTF-8 — sin esto explota con emojis
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

impl EditorState {
    // arranca el editor: split del contenido inicial en líneas, cursor al inicio en el título
    pub fn new(
        project_id: uuid::Uuid,
        note_id: Option<uuid::Uuid>,
        initial_title: String,
        initial_content: String,
    ) -> Self {
        let lines = if initial_content.is_empty() {
            vec![String::new()]
        } else {
            initial_content.lines().map(String::from).collect()
        };

        Self {
            title: initial_title,
            lines,
            cursor_x: 0,
            cursor_y: 0,
            note_id,
            project_id,
            editing_title: true,
            codex_id: None,
        }
    }

    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    // subir desde la primera línea regresa al título — detalle chido de UX
    pub fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line = &self.lines[self.cursor_y];
            self.cursor_x = floor_char_boundary(line, self.cursor_x.min(line.len()));
        } else {
            self.editing_title = true;
        }
    }

    pub fn move_down(&mut self) {
        if self.editing_title {
            self.editing_title = false;
            self.cursor_y = 0;
            let line = &self.lines[0];
            self.cursor_x = floor_char_boundary(line, self.cursor_x.min(line.len()));
        } else if self.cursor_y < self.lines.len() - 1 {
            self.cursor_y += 1;
            let line = &self.lines[self.cursor_y];
            self.cursor_x = floor_char_boundary(line, self.cursor_x.min(line.len()));
        }
    }

    // izquierda al inicio de línea salta a la línea anterior — comportamiento estilo terminal
    pub fn move_left(&mut self) {
        if self.cursor_x > 0 {
            let line = &self.lines[self.cursor_y];
            let prev_len = line[..self.cursor_x]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor_x -= prev_len;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
        }
    }

    pub fn move_right(&mut self) {
        let line = &self.lines[self.cursor_y];
        if self.cursor_x < line.len() {
            let ch_len = line[self.cursor_x..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor_x += ch_len;
        } else if self.cursor_y < self.lines.len() - 1 {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    // inserta en título (máx 50 chars) o en el cuerpo según editing_title — ojo con el límite
    pub fn insert_char(&mut self, c: char) {
        if self.editing_title {
            if self.title.len() < 50 {
                self.title.push(c);
            }
        } else {
            let line = &mut self.lines[self.cursor_y];
            let x = floor_char_boundary(line, self.cursor_x.min(line.len()));
            line.insert(x, c);
            self.cursor_x = x + c.len_utf8();
        }
    }

    // backspace: borra char anterior o mergea la línea actual con la de arriba — qué rollo el merge
    pub fn handle_backspace(&mut self) {
        if self.editing_title {
            self.title.pop();
        } else {
            if self.cursor_x > 0 {
                let line = &mut self.lines[self.cursor_y];
                let cursor = self.cursor_x.min(line.len());
                let prev_start = line[..cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                line.remove(prev_start);
                self.cursor_x = prev_start;
            } else if self.cursor_y > 0 {
                // Merge current line into previous line
                let current_line = self.lines.remove(self.cursor_y);
                self.cursor_y -= 1;
                let prev_line = &mut self.lines[self.cursor_y];
                self.cursor_x = prev_line.len();
                prev_line.push_str(&current_line);
            }
        }
    }

    // delete al final de línea también mergea con la siguiente — simétrico con backspace
    pub fn handle_delete(&mut self) {
        if !self.editing_title {
            let line = &mut self.lines[self.cursor_y];
            let x = floor_char_boundary(line, self.cursor_x.min(line.len()));
            if x < line.len() {
                line.remove(x);
            } else if self.cursor_y < self.lines.len() - 1 {
                // Merge next line into current line
                let next_line = self.lines.remove(self.cursor_y + 1);
                self.lines[self.cursor_y].push_str(&next_line);
            }
        }
    }

    // enter desde el título baja al cuerpo; en el cuerpo parte la línea en dos — split_off es lo chido aquí
    pub fn handle_enter(&mut self) {
        if self.editing_title {
            self.editing_title = false;
            self.cursor_y = 0;
            self.cursor_x = 0;
        } else {
            let line = &mut self.lines[self.cursor_y];
            let split_index = floor_char_boundary(line, self.cursor_x.min(line.len()));
            let remaining_content = line.split_off(split_index);
            self.lines.insert(self.cursor_y + 1, remaining_content);
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    pub fn handle_tab(&mut self) {
        if self.editing_title {
            self.editing_title = false;
            self.cursor_y = 0;
            self.cursor_x = 0;
        } else {
            let line = &mut self.lines[self.cursor_y];
            let x = floor_char_boundary(line, self.cursor_x.min(line.len()));
            line.insert_str(x, "    ");
            self.cursor_x = x + 4;
        }
    }
}

// pinta el editor: título arriba, cuerpo en medio con cursor visual, controles abajo
pub fn draw(f: &mut Frame, state: &EditorState, theme: &Theme) {
    let size = f.size();
    let accent_color = theme.primary;

    // Layout splits: Top Title Input, Middle Content Area, Bottom Status/Controls
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title input
            Constraint::Min(5),    // Body edit area
            Constraint::Length(3), // Controls/Help info
        ])
        .split(size);

    // 1. Render Title Input box
    let title_border_style = if state.editing_title {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let title_text = if state.editing_title {
        format!("{}_", state.title)
    } else if state.title.is_empty() {
        "Untitled Note".to_string()
    } else {
        state.title.clone()
    };
    let title_p = Paragraph::new(title_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(title_border_style)
            .title(" Scroll Title "),
    );
    f.render_widget(title_p, chunks[0]);

    // el cursor se dibuja manualmente: char bajo el cursor con fondo de color — no hay cursor nativo en ratatui
    let body_border_style = if !state.editing_title {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };

    let mut lines_spans = Vec::new();
    for (i, line) in state.lines.iter().enumerate() {
        if !state.editing_title && i == state.cursor_y {
            let x = floor_char_boundary(line, state.cursor_x.min(line.len()));
            let before = &line[..x];
            let (cursor_char, after): (&str, &str) = if x < line.len() {
                let ch_end = x + line[x..].chars().next().unwrap().len_utf8();
                (&line[x..ch_end], &line[ch_end..])
            } else {
                (" ", "")
            };

            lines_spans.push(Line::from(vec![
                Span::raw(before),
                Span::styled(
                    cursor_char,
                    Style::default().fg(Color::Black).bg(theme.selection),
                ),
                Span::raw(after),
            ]));
        } else {
            lines_spans.push(Line::from(line.as_str()));
        }
    }

    let body_p = Paragraph::new(lines_spans)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(body_border_style)
                .title(" Scroll Editor "),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(body_p, chunks[1]);

    // 3. Render Controls Status Bar
    let ctrl_text = vec![Line::from(vec![
        Span::styled(" Markdown Editor |  ", Style::default().fg(accent_color)),
        Span::styled(
            "Ctrl+S",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Save & Record | ", Style::default().fg(theme.muted)),
        Span::styled(
            "ESC",
            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Cancel | ", Style::default().fg(theme.muted)),
        Span::styled("Tab", Style::default().fg(accent_color)),
        Span::styled(" Tab Spacing / Switch Focus | ", Style::default().fg(theme.muted)),
        Span::styled("Shift+Tab", Style::default().fg(accent_color)),
        Span::styled(" Toggle Title/Editor", Style::default().fg(theme.muted)),
    ])];
    let ctrl_p = Paragraph::new(ctrl_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(ctrl_p, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_editor_typing() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(project_id, None, "".to_string(), "".to_string());

        assert!(editor.editing_title);
        editor.insert_char('M');
        editor.insert_char('y');
        editor.insert_char(' ');
        editor.insert_char('N');
        editor.insert_char('o');
        editor.insert_char('t');
        editor.insert_char('e');
        assert_eq!(editor.title, "My Note");

        editor.handle_enter();
        assert!(!editor.editing_title);
        assert_eq!(editor.cursor_y, 0);
        assert_eq!(editor.cursor_x, 0);

        editor.insert_char('H');
        editor.insert_char('i');
        assert_eq!(editor.lines[0], "Hi");

        editor.handle_tab();
        assert_eq!(editor.lines[0], "Hi    ");
        assert_eq!(editor.cursor_x, 6);

        editor.insert_char('Y');
        editor.cursor_x = 2;
        editor.handle_enter();
        assert_eq!(editor.lines[0], "Hi");
        assert_eq!(editor.lines[1], "    Y");
        assert_eq!(editor.cursor_y, 1);
        assert_eq!(editor.cursor_x, 0);

        editor.handle_backspace();
        assert_eq!(editor.lines[0], "Hi    Y");
        assert_eq!(editor.cursor_y, 0);
        assert_eq!(editor.cursor_x, 2);
    }
}
