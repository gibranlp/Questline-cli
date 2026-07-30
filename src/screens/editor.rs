// ─────────────────────────────────────────────────────────────────────────────
// screens/editor.rs — vim-mode note editor
// ─────────────────────────────────────────────────────────────────────────────

use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

// ── Mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual {
        anchor_y: usize,
        anchor_x: usize,
        line_mode: bool,
    },
}

impl EditorMode {
    pub fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual {
                line_mode: false, ..
            } => "VISUAL",
            EditorMode::Visual {
                line_mode: true, ..
            } => "V-LINE",
        }
    }
}

// ── Snapshot for undo/redo ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EditorSnapshot {
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EditorState {
    pub title: String,
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub note_id: Option<uuid::Uuid>,
    pub project_id: uuid::Uuid,
    pub editing_title: bool,
    /// Quick notes opened from the dashboard/palette choose their campaign in-place.
    pub quick_note: bool,
    pub editing_project: bool,
    pub project_choices: Vec<(uuid::Uuid, String)>,
    pub project_choice_idx: usize,
    pub saved_title: String,
    pub saved_content: String,
    pub saved_project_id: uuid::Uuid,
    pub confirm_close: bool,
    pub autosave_timer: std::time::Instant,
    pub last_autosaved_at: Option<std::time::Instant>,
    pub codex_id: Option<uuid::Uuid>,
    // vim
    pub mode: EditorMode,
    pub yank_register: String,
    pub yank_is_line: bool,
    pub undo_history: Vec<EditorSnapshot>,
    pub redo_history: Vec<EditorSnapshot>,
    pub pending_cmd: String,
    pub show_help: bool,
    pub scroll_offset: usize,
}

// ── UTF-8 helpers ─────────────────────────────────────────────────────────────

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

// Byte index of the end of the char that starts at `idx`
fn char_end(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    idx + s[idx..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// ── EditorState impl ──────────────────────────────────────────────────────────

impl EditorState {
    pub fn new(
        project_id: uuid::Uuid,
        note_id: Option<uuid::Uuid>,
        initial_title: String,
        initial_content: String,
    ) -> Self {
        let saved_title = initial_title.clone();
        let lines = if initial_content.is_empty() {
            vec![String::new()]
        } else {
            initial_content.lines().map(String::from).collect()
        };
        let saved_content = lines.join("\n");
        // Existing notes open in Normal mode in the body; new notes start in title
        let editing_title = note_id.is_none();
        let mode = if note_id.is_some() {
            EditorMode::Normal
        } else {
            EditorMode::Insert
        };
        Self {
            title: initial_title,
            lines,
            cursor_x: 0,
            cursor_y: 0,
            note_id,
            project_id,
            editing_title,
            quick_note: false,
            editing_project: false,
            project_choices: Vec::new(),
            project_choice_idx: 0,
            saved_title,
            saved_content,
            saved_project_id: project_id,
            confirm_close: false,
            autosave_timer: std::time::Instant::now(),
            last_autosaved_at: None,
            codex_id: None,
            mode,
            yank_register: String::new(),
            yank_is_line: false,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            pending_cmd: String::new(),
            show_help: false,
            scroll_offset: 0,
        }
    }

    pub fn new_quick_note(
        project_choices: Vec<(uuid::Uuid, String)>,
        selected_project_id: Option<uuid::Uuid>,
    ) -> Option<Self> {
        let project_choice_idx = selected_project_id
            .and_then(|id| project_choices.iter().position(|(pid, _)| *pid == id))
            .unwrap_or(0);
        let project_id = project_choices.get(project_choice_idx)?.0;
        let mut state = Self::new(project_id, None, String::new(), String::new());
        state.quick_note = true;
        state.project_choices = project_choices;
        state.project_choice_idx = project_choice_idx;
        Some(state)
    }

    pub fn selected_project_name(&self) -> &str {
        self.project_choices
            .get(self.project_choice_idx)
            .map(|(_, name)| name.as_str())
            .unwrap_or("No active campaign")
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.title != self.saved_title
            || self.get_content() != self.saved_content
            || self.project_id != self.saved_project_id
    }

    pub fn mark_saved(&mut self, note_id: uuid::Uuid) {
        self.note_id = Some(note_id);
        self.saved_title = self.title.clone();
        self.saved_content = self.get_content();
        self.saved_project_id = self.project_id;
        self.confirm_close = false;
        self.autosave_timer = std::time::Instant::now();
    }

    pub fn autosave_due(&self, interval: std::time::Duration) -> bool {
        self.has_unsaved_changes()
            && !self.confirm_close
            && self.autosave_timer.elapsed() >= interval
    }

    pub fn record_autosaved(&mut self) {
        self.last_autosaved_at = Some(std::time::Instant::now());
    }

    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    // ── Undo / redo ───────────────────────────────────────────────────────────

    pub fn push_undo(&mut self) {
        if self.undo_history.len() >= 100 {
            self.undo_history.remove(0);
        }
        self.undo_history.push(EditorSnapshot {
            lines: self.lines.clone(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
        });
        self.redo_history.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snap) = self.undo_history.pop() {
            let cur = EditorSnapshot {
                lines: self.lines.clone(),
                cursor_x: self.cursor_x,
                cursor_y: self.cursor_y,
            };
            if self.redo_history.len() >= 100 {
                self.redo_history.remove(0);
            }
            self.redo_history.push(cur);
            self.lines = snap.lines;
            self.cursor_x = snap.cursor_x;
            self.cursor_y = snap.cursor_y;
            self.mode = EditorMode::Normal;
            self.clamp_cursor();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snap) = self.redo_history.pop() {
            let cur = EditorSnapshot {
                lines: self.lines.clone(),
                cursor_x: self.cursor_x,
                cursor_y: self.cursor_y,
            };
            if self.undo_history.len() >= 100 {
                self.undo_history.remove(0);
            }
            self.undo_history.push(cur);
            self.lines = snap.lines;
            self.cursor_x = snap.cursor_x;
            self.cursor_y = snap.cursor_y;
            self.clamp_cursor();
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor_y = self.cursor_y.min(self.lines.len().saturating_sub(1));
        let len = self.lines[self.cursor_y].len();
        self.cursor_x = floor_char_boundary(&self.lines[self.cursor_y], self.cursor_x.min(len));
    }

    // In Normal mode the cursor can't sit past the last character
    pub fn clamp_to_normal(&mut self) {
        if self.editing_title {
            return;
        }
        let line = &self.lines[self.cursor_y];
        if line.is_empty() {
            self.cursor_x = 0;
        } else {
            let last = line.char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
            if self.cursor_x > last {
                self.cursor_x = last;
            } else {
                self.cursor_x = floor_char_boundary(line, self.cursor_x);
            }
        }
    }

    // ── Normal-mode motions ───────────────────────────────────────────────────

    pub fn normal_h(&mut self) {
        if self.cursor_x == 0 {
            return;
        }
        let line = &self.lines[self.cursor_y];
        self.cursor_x = line[..self.cursor_x]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    pub fn normal_l(&mut self) {
        let line = &self.lines[self.cursor_y];
        if line.is_empty() {
            return;
        }
        let next = char_end(line, self.cursor_x);
        if next < line.len() {
            self.cursor_x = next;
        }
        // already on last char — stay
    }

    pub fn normal_j(&mut self) {
        if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            let line = &self.lines[self.cursor_y];
            self.cursor_x = floor_char_boundary(line, self.cursor_x.min(line.len()));
            self.clamp_to_normal();
        }
    }

    pub fn normal_k(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line = &self.lines[self.cursor_y];
            self.cursor_x = floor_char_boundary(line, self.cursor_x.min(line.len()));
            self.clamp_to_normal();
        }
    }

    pub fn goto_line_start(&mut self) {
        self.cursor_x = 0;
    }

    pub fn goto_line_end(&mut self) {
        let line = &self.lines[self.cursor_y];
        self.cursor_x = if line.is_empty() {
            0
        } else {
            line.char_indices().next_back().map(|(i, _)| i).unwrap_or(0)
        };
    }

    pub fn goto_file_start(&mut self) {
        self.cursor_y = 0;
        self.cursor_x = 0;
        self.editing_title = false;
    }

    pub fn goto_file_end(&mut self) {
        self.cursor_y = self.lines.len().saturating_sub(1);
        self.goto_line_end();
        self.editing_title = false;
    }

    // w: jump to start of next word
    pub fn word_forward(&mut self) {
        let mut y = self.cursor_y;
        let mut x = self.cursor_x;
        let line = &self.lines[y];

        if x < line.len() {
            let ch = line[x..].chars().next().unwrap();
            let cur_is_word = is_word_char(ch);
            // Skip current group
            while x < line.len() {
                let c = line[x..].chars().next().unwrap();
                if is_word_char(c) != cur_is_word {
                    break;
                }
                x = char_end(line, x);
            }
            // Skip whitespace
            while x < line.len() && line[x..].chars().next().unwrap().is_whitespace() {
                x = char_end(line, x);
            }
            if x < line.len() {
                self.cursor_x = x;
                return;
            }
        }

        if y + 1 < self.lines.len() {
            y += 1;
            x = 0;
            let line = &self.lines[y];
            while x < line.len() && line[x..].chars().next().unwrap().is_whitespace() {
                x = char_end(line, x);
            }
            self.cursor_y = y;
            self.cursor_x = x;
        }
    }

    // b: jump to start of current/previous word
    pub fn word_backward(&mut self) {
        let mut y = self.cursor_y;
        let mut x = self.cursor_x;

        if x == 0 {
            if y == 0 {
                return;
            }
            y -= 1;
            x = self.lines[y].len();
        }

        let line = &self.lines[y];
        // Step one char back
        x = line[..x]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Skip whitespace backwards
        while x > 0
            && line[x..]
                .chars()
                .next()
                .map(|c| c.is_whitespace())
                .unwrap_or(false)
        {
            x = line[..x]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }

        let ch = line[x..].chars().next().unwrap_or(' ');
        let cur_is_word = is_word_char(ch);
        while x > 0 {
            let prev = line[..x]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            let c = line[prev..].chars().next().unwrap();
            if is_word_char(c) != cur_is_word {
                break;
            }
            x = prev;
        }

        self.cursor_y = y;
        self.cursor_x = x;
    }

    // e: jump to end of current/next word
    pub fn word_end(&mut self) {
        let mut y = self.cursor_y;
        let mut x = self.cursor_x;
        let line = &self.lines[y];

        // Move one forward first (skip current char)
        if x < line.len() {
            x = char_end(line, x);
        }

        // Skip whitespace
        while x < line.len() && line[x..].chars().next().unwrap().is_whitespace() {
            x = char_end(line, x);
        }

        if x < line.len() {
            let ch = line[x..].chars().next().unwrap();
            let cur_is_word = is_word_char(ch);
            while char_end(line, x) < line.len() {
                let c = line[char_end(line, x)..].chars().next().unwrap();
                if is_word_char(c) != cur_is_word {
                    break;
                }
                x = char_end(line, x);
            }
            self.cursor_x = x;
            self.cursor_y = y;
            return;
        }

        if y + 1 < self.lines.len() {
            y += 1;
            x = 0;
            let line = &self.lines[y];
            while x < line.len() && line[x..].chars().next().unwrap().is_whitespace() {
                x = char_end(line, x);
            }
            if x < line.len() {
                let ch = line[x..].chars().next().unwrap();
                let cur_is_word = is_word_char(ch);
                while char_end(line, x) < line.len() {
                    let c = line[char_end(line, x)..].chars().next().unwrap();
                    if is_word_char(c) != cur_is_word {
                        break;
                    }
                    x = char_end(line, x);
                }
            }
            self.cursor_y = y;
            self.cursor_x = x;
        }
    }

    pub fn set_yank_register(&mut self, text: String, is_line: bool) {
        self.yank_register = text;
        self.yank_is_line = is_line;
        let _ = crate::services::identity::copy_to_clipboard(&self.yank_register);
    }

    // ── Normal-mode edits ─────────────────────────────────────────────────────

    // x: delete char under cursor
    pub fn delete_char(&mut self) {
        if self.editing_title {
            return;
        }
        let (x, len) = {
            let line = &self.lines[self.cursor_y];
            if line.is_empty() {
                return;
            }
            let x = floor_char_boundary(line, self.cursor_x.min(line.len().saturating_sub(1)));
            let len = char_end(line, x) - x;
            (x, len)
        };
        let text = self.lines[self.cursor_y][x..x + len].to_string();
        self.set_yank_register(text, false);
        self.lines[self.cursor_y].drain(x..x + len);
        self.cursor_x = x;
        self.clamp_to_normal();
    }

    // X: delete char before cursor
    pub fn delete_char_before(&mut self) {
        if self.editing_title || self.cursor_x == 0 {
            return;
        }
        let prev = {
            let line = &self.lines[self.cursor_y];
            line[..self.cursor_x]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        let text = self.lines[self.cursor_y][prev..self.cursor_x].to_string();
        self.set_yank_register(text, false);
        self.lines[self.cursor_y].drain(prev..self.cursor_x);
        self.cursor_x = prev;
        self.clamp_to_normal();
    }

    // dd: delete line
    pub fn delete_line(&mut self) {
        if self.editing_title {
            return;
        }
        self.set_yank_register(self.lines[self.cursor_y].clone(), true);
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.cursor_x = 0;
        } else {
            self.lines.remove(self.cursor_y);
            if self.cursor_y >= self.lines.len() {
                self.cursor_y = self.lines.len() - 1;
            }
            self.clamp_to_normal();
        }
    }

    // D: delete to end of line
    pub fn delete_to_end(&mut self) {
        if self.editing_title {
            return;
        }
        let x = {
            let line = &self.lines[self.cursor_y];
            floor_char_boundary(line, self.cursor_x.min(line.len()))
        };
        let text = self.lines[self.cursor_y][x..].to_string();
        self.set_yank_register(text, false);
        self.lines[self.cursor_y].truncate(x);
        self.clamp_to_normal();
    }

    // yy: yank line
    pub fn yank_line(&mut self) {
        if self.editing_title {
            return;
        }
        self.set_yank_register(self.lines[self.cursor_y].clone(), true);
    }

    // p: paste after
    pub fn paste_after(&mut self) {
        if self.editing_title {
            return;
        }
        if self.yank_is_line {
            let new_line = self.yank_register.clone();
            let at = (self.cursor_y + 1).min(self.lines.len());
            self.lines.insert(at, new_line);
            self.cursor_y = at;
            self.cursor_x = 0;
        } else if !self.yank_register.is_empty() {
            let reg = self.yank_register.clone();
            let line = &mut self.lines[self.cursor_y];
            let x = if line.is_empty() {
                0
            } else {
                char_end(
                    line,
                    floor_char_boundary(line, self.cursor_x.min(line.len().saturating_sub(1))),
                )
            };
            line.insert_str(x, &reg);
            self.cursor_x =
                floor_char_boundary(&self.lines[self.cursor_y], x + reg.len().saturating_sub(1));
            self.clamp_to_normal();
        }
    }

    // P: paste before
    pub fn paste_before(&mut self) {
        if self.editing_title {
            return;
        }
        if self.yank_is_line {
            let new_line = self.yank_register.clone();
            self.lines.insert(self.cursor_y, new_line);
            self.cursor_x = 0;
        } else if !self.yank_register.is_empty() {
            let reg = self.yank_register.clone();
            let line = &mut self.lines[self.cursor_y];
            let x = floor_char_boundary(line, self.cursor_x.min(line.len()));
            line.insert_str(x, &reg);
            self.cursor_x =
                floor_char_boundary(&self.lines[self.cursor_y], x + reg.len().saturating_sub(1));
            self.clamp_to_normal();
        }
    }

    // r{c}: replace char under cursor
    pub fn replace_char(&mut self, c: char) {
        if self.editing_title {
            return;
        }
        let line = &mut self.lines[self.cursor_y];
        if line.is_empty() {
            line.push(c);
            return;
        }
        let x = floor_char_boundary(line, self.cursor_x.min(line.len().saturating_sub(1)));
        let len = char_end(line, x) - x;
        line.drain(x..x + len);
        line.insert(x, c);
        self.cursor_x = x;
    }

    // ── Inner-word helpers ────────────────────────────────────────────────────

    fn inner_word_range(&self) -> (usize, usize) {
        let line = &self.lines[self.cursor_y];
        if line.is_empty() {
            return (0, 0);
        }
        let x = floor_char_boundary(line, self.cursor_x.min(line.len().saturating_sub(1)));
        let ch = line[x..].chars().next().unwrap_or(' ');
        let cur_is_word = is_word_char(ch);
        let mut start = x;
        while start > 0 {
            let prev = line[..start]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            if is_word_char(line[prev..].chars().next().unwrap()) != cur_is_word {
                break;
            }
            start = prev;
        }
        let mut end = x;
        while end < line.len() {
            let c = line[end..].chars().next().unwrap();
            if is_word_char(c) != cur_is_word {
                break;
            }
            end = char_end(line, end);
        }
        (start, end)
    }

    pub fn delete_inner_word(&mut self) {
        if self.editing_title {
            return;
        }
        let (s, e) = self.inner_word_range();
        self.set_yank_register(self.lines[self.cursor_y][s..e].to_string(), false);
        self.lines[self.cursor_y].drain(s..e);
        self.cursor_x = s;
        self.clamp_to_normal();
    }

    pub fn delete_word(&mut self) {
        if self.editing_title {
            return;
        }
        let x_start = self.cursor_x;
        let y_start = self.cursor_y;
        self.word_forward();
        let (x_end, y_end) = (self.cursor_x, self.cursor_y);
        if y_start == y_end {
            let s =
                floor_char_boundary(&self.lines[y_start], x_start.min(self.lines[y_start].len()));
            let e = floor_char_boundary(&self.lines[y_start], x_end.min(self.lines[y_start].len()));
            self.set_yank_register(self.lines[y_start][s..e].to_string(), false);
            self.lines[y_start].drain(s..e);
            self.cursor_x = s;
            self.cursor_y = y_start;
        } else {
            let s =
                floor_char_boundary(&self.lines[y_start], x_start.min(self.lines[y_start].len()));
            self.lines[y_start].truncate(s);
            self.cursor_x = s;
            self.cursor_y = y_start;
        }
        self.clamp_to_normal();
    }

    // ── Enter/exit modes ──────────────────────────────────────────────────────

    pub fn enter_insert(&mut self) {
        self.editing_title = false;
        self.mode = EditorMode::Insert;
    }

    pub fn enter_insert_after(&mut self) {
        self.editing_title = false;
        let line = &self.lines[self.cursor_y];
        if !line.is_empty() && self.cursor_x < line.len() {
            self.cursor_x = char_end(line, self.cursor_x);
        }
        self.mode = EditorMode::Insert;
    }

    pub fn enter_insert_line_start(&mut self) {
        self.editing_title = false;
        self.cursor_x = 0;
        self.mode = EditorMode::Insert;
    }

    pub fn enter_insert_line_end(&mut self) {
        self.editing_title = false;
        self.cursor_x = self.lines[self.cursor_y].len();
        self.mode = EditorMode::Insert;
    }

    pub fn open_line_below(&mut self) {
        self.editing_title = false;
        let insert_at = self.cursor_y + 1;
        self.lines.insert(insert_at, String::new());
        self.cursor_y = insert_at;
        self.cursor_x = 0;
        self.mode = EditorMode::Insert;
    }

    pub fn open_line_above(&mut self) {
        self.editing_title = false;
        self.lines.insert(self.cursor_y, String::new());
        self.cursor_x = 0;
        self.mode = EditorMode::Insert;
    }

    pub fn change_line(&mut self) {
        if self.editing_title {
            return;
        }
        self.set_yank_register(self.lines[self.cursor_y].clone(), false);
        self.lines[self.cursor_y].clear();
        self.cursor_x = 0;
        self.mode = EditorMode::Insert;
    }

    pub fn change_to_end(&mut self) {
        self.delete_to_end();
        self.mode = EditorMode::Insert;
    }
    pub fn change_word(&mut self) {
        self.delete_word();
        self.mode = EditorMode::Insert;
    }
    pub fn change_inner_word(&mut self) {
        self.delete_inner_word();
        self.mode = EditorMode::Insert;
    }

    pub fn leave_insert(&mut self) {
        self.mode = EditorMode::Normal;
        self.clamp_to_normal();
    }

    // ── Visual mode ───────────────────────────────────────────────────────────

    pub fn enter_visual_char(&mut self) {
        if self.editing_title {
            return;
        }
        let (y, x) = (self.cursor_y, self.cursor_x);
        self.mode = EditorMode::Visual {
            anchor_y: y,
            anchor_x: x,
            line_mode: false,
        };
    }

    pub fn enter_visual_line(&mut self) {
        if self.editing_title {
            return;
        }
        let y = self.cursor_y;
        self.mode = EditorMode::Visual {
            anchor_y: y,
            anchor_x: 0,
            line_mode: true,
        };
    }

    // Normalized selection bounds: (start_y, start_x, end_y, end_x, line_mode)
    pub fn visual_range(&self) -> Option<(usize, usize, usize, usize, bool)> {
        if let EditorMode::Visual {
            anchor_y,
            anchor_x,
            line_mode,
        } = &self.mode
        {
            let (sy, sx, ey, ex) = if (*anchor_y, *anchor_x) <= (self.cursor_y, self.cursor_x) {
                (*anchor_y, *anchor_x, self.cursor_y, self.cursor_x)
            } else {
                (self.cursor_y, self.cursor_x, *anchor_y, *anchor_x)
            };
            Some((sy, sx, ey, ex, *line_mode))
        } else {
            None
        }
    }

    pub fn get_visual_text(&self) -> String {
        let Some((sy, sx, ey, ex, line_mode)) = self.visual_range() else {
            return String::new();
        };
        if line_mode {
            return self.lines[sy..=ey].join("\n");
        }
        if sy == ey {
            let line = &self.lines[sy];
            let s = floor_char_boundary(line, sx.min(line.len()));
            let e = char_end(
                line,
                floor_char_boundary(line, ex.min(line.len().saturating_sub(1))),
            );
            return line[s..e].to_string();
        }
        let mut out = String::new();
        let first = &self.lines[sy];
        let s = floor_char_boundary(first, sx.min(first.len()));
        out.push_str(&first[s..]);
        for y in sy + 1..ey {
            out.push('\n');
            out.push_str(&self.lines[y]);
        }
        out.push('\n');
        let last = &self.lines[ey];
        let e = char_end(
            last,
            floor_char_boundary(last, ex.min(last.len().saturating_sub(1))),
        );
        out.push_str(&last[..e]);
        out
    }

    pub fn yank_visual(&mut self) {
        let is_line = matches!(
            self.mode,
            EditorMode::Visual {
                line_mode: true,
                ..
            }
        );
        self.set_yank_register(self.get_visual_text(), is_line);
        self.mode = EditorMode::Normal;
    }

    pub fn delete_visual(&mut self) {
        let Some((sy, sx, ey, ex, line_mode)) = self.visual_range() else {
            return;
        };
        self.set_yank_register(self.get_visual_text(), line_mode);

        if line_mode {
            for _ in sy..=ey {
                self.lines.remove(sy);
            }
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }
            self.cursor_y = sy.min(self.lines.len() - 1);
            self.cursor_x = 0;
        } else if sy == ey {
            let line = &mut self.lines[sy];
            let s = floor_char_boundary(line, sx.min(line.len()));
            let e = char_end(
                line,
                floor_char_boundary(line, ex.min(line.len().saturating_sub(1))),
            );
            line.drain(s..e);
            self.cursor_y = sy;
            self.cursor_x = s;
        } else {
            let head_keep = floor_char_boundary(&self.lines[sy], sx.min(self.lines[sy].len()));
            let tail_start = char_end(
                &self.lines[ey],
                floor_char_boundary(
                    &self.lines[ey],
                    ex.min(self.lines[ey].len().saturating_sub(1)),
                ),
            );
            let tail = self.lines[ey][tail_start..].to_string();
            self.lines[sy].truncate(head_keep);
            self.lines[sy].push_str(&tail);
            for _ in sy + 1..=ey {
                self.lines.remove(sy + 1);
            }
            self.cursor_y = sy;
            self.cursor_x = head_keep;
        }
        self.mode = EditorMode::Normal;
        self.clamp_to_normal();
    }

    pub fn change_visual(&mut self) {
        self.delete_visual();
        self.mode = EditorMode::Insert;
    }

    // ── Insert-mode editing (unchanged) ──────────────────────────────────────

    pub fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line = &self.lines[self.cursor_y];
            self.cursor_x = floor_char_boundary(line, self.cursor_x.min(line.len()));
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

    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.editing_title {
            let remaining = 50usize.saturating_sub(self.title.chars().count());
            self.title
                .extend(text.replace(['\r', '\n'], " ").chars().take(remaining));
            self.autosave_timer = std::time::Instant::now();
            return;
        }

        self.push_undo();
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let parts: Vec<&str> = normalized.split('\n').collect();
        let x = floor_char_boundary(
            &self.lines[self.cursor_y],
            self.cursor_x.min(self.lines[self.cursor_y].len()),
        );
        let suffix = self.lines[self.cursor_y].split_off(x);
        self.lines[self.cursor_y].push_str(parts[0]);

        if parts.len() == 1 {
            self.cursor_x = self.lines[self.cursor_y].len();
            self.lines[self.cursor_y].push_str(&suffix);
        } else {
            let insert_at = self.cursor_y + 1;
            for (offset, part) in parts[1..parts.len() - 1].iter().enumerate() {
                self.lines.insert(insert_at + offset, (*part).to_string());
            }
            let mut last_line = parts.last().copied().unwrap_or_default().to_string();
            self.cursor_y += parts.len() - 1;
            self.cursor_x = last_line.len();
            last_line.push_str(&suffix);
            self.lines.insert(self.cursor_y, last_line);
        }
        self.mode = EditorMode::Insert;
        self.autosave_timer = std::time::Instant::now();
    }

    pub fn handle_backspace(&mut self) {
        if self.editing_title {
            self.title.pop();
        } else if self.cursor_x > 0 {
            let line = &mut self.lines[self.cursor_y];
            let cursor = self.cursor_x.min(line.len());
            let prev = line[..cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            line.remove(prev);
            self.cursor_x = prev;
        } else if self.cursor_y > 0 {
            let cur = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            let prev_line = &mut self.lines[self.cursor_y];
            self.cursor_x = prev_line.len();
            prev_line.push_str(&cur);
        }
    }

    pub fn handle_ctrl_backspace(&mut self) {
        if self.editing_title {
            let cursor = self.title.len();
            delete_word_before_cursor(&mut self.title, cursor);
        } else if self.cursor_x > 0 {
            let line = &mut self.lines[self.cursor_y];
            self.cursor_x = delete_word_before_cursor(line, self.cursor_x);
        } else if self.cursor_y > 0 {
            let cur = self.lines.remove(self.cursor_y);
            self.cursor_y -= 1;
            let prev_line = &mut self.lines[self.cursor_y];
            self.cursor_x = prev_line.len();
            prev_line.push_str(&cur);
        }
    }

    pub fn handle_delete(&mut self) {
        if self.editing_title {
            return;
        }
        let line = &mut self.lines[self.cursor_y];
        let x = floor_char_boundary(line, self.cursor_x.min(line.len()));
        if x < line.len() {
            line.remove(x);
        } else if self.cursor_y < self.lines.len() - 1 {
            let next = self.lines.remove(self.cursor_y + 1);
            self.lines[self.cursor_y].push_str(&next);
        }
    }

    pub fn handle_enter(&mut self) {
        if self.editing_title {
            self.editing_title = false;
            self.cursor_y = 0;
            self.cursor_x = 0;
        } else {
            let line = &mut self.lines[self.cursor_y];
            let split = floor_char_boundary(line, self.cursor_x.min(line.len()));
            let rest = line.split_off(split);
            self.lines.insert(self.cursor_y + 1, rest);
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

fn delete_word_before_cursor(input: &mut String, cursor: usize) -> usize {
    let cursor = cursor.min(input.len());
    if cursor == 0 {
        return 0;
    }

    let mut start = cursor;
    while start > 0 {
        let (idx, ch) = input[..start].char_indices().next_back().unwrap();
        if !ch.is_whitespace() {
            break;
        }
        start = idx;
    }
    while start > 0 {
        let (idx, ch) = input[..start].char_indices().next_back().unwrap();
        if ch.is_whitespace() {
            break;
        }
        start = idx;
    }

    input.drain(start..cursor);
    start
}

// ── Rendering helpers ─────────────────────────────────────────────────────────

// Returns (sel_start_byte, sel_end_byte_exclusive) for `line_i` in visual mode
fn line_sel_range(line_i: usize, line: &str, state: &EditorState) -> Option<(usize, usize)> {
    let (sy, sx, ey, ex, line_mode) = state.visual_range()?;
    if line_i < sy || line_i > ey {
        return None;
    }

    if line_mode {
        let end = if line.is_empty() { 1 } else { line.len() };
        return Some((0, end));
    }

    let start = if line_i > sy {
        0
    } else {
        floor_char_boundary(line, sx.min(line.len()))
    };
    let end = if line_i < ey {
        if line.is_empty() { 1 } else { line.len() }
    } else {
        // inclusive end — include the char at ex
        let clamped = floor_char_boundary(line, ex.min(line.len().saturating_sub(1)));
        if line.is_empty() {
            1
        } else {
            char_end(line, clamped)
        }
    };

    Some((start, end.max(start + if line.is_empty() { 1 } else { 0 })))
}

// Build styled spans for one editor body line
pub(crate) fn render_body_line<'a>(
    line: &'a str,
    line_i: usize,
    state: &EditorState,
    theme: &Theme,
) -> Line<'a> {
    let is_cursor_line = !state.editing_title && !state.editing_project && line_i == state.cursor_y;
    let sel = line_sel_range(line_i, line, state);

    if let Some((sel_s, sel_e)) = sel {
        // Visual selection — split into before / selected / after
        let sel_s = sel_s.min(line.len());
        let before = &line[..sel_s];

        let (in_sel, after): (String, &str) = if sel_e <= line.len() {
            (line[sel_s..sel_e].to_string(), &line[sel_e..])
        } else {
            // sel_e > line.len() means empty line or end of content → show one space as cursor
            let text = if line.len() > sel_s {
                line[sel_s..].to_string()
            } else {
                " ".to_string()
            };
            (text, "")
        };

        let sel_style = Style::default().fg(Color::Black).bg(theme.selection);
        let mut spans: Vec<Span<'a>> = Vec::new();
        if !before.is_empty() {
            spans.push(Span::raw(before));
        }
        spans.push(Span::styled(in_sel, sel_style));
        if !after.is_empty() {
            spans.push(Span::raw(after));
        }
        Line::from(spans)
    } else if is_cursor_line && state.mode == EditorMode::Insert {
        // Inside text, highlight the character under the cursor without adding
        // another terminal cell (which would look like a space in the word).
        // At end-of-line, use a visible glyph because styled trailing spaces are
        // commonly discarded by terminals.
        let x = floor_char_boundary(line, state.cursor_x.min(line.len()));
        let before = &line[..x];
        let (cursor_ch, after, cursor_style): (String, &str, Style) = if x < line.len() {
            let end = char_end(line, x);
            (
                line[x..end].to_string(),
                &line[end..],
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "│".to_string(),
                "",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )
        };
        let mut spans: Vec<Span<'a>> = Vec::new();
        if !before.is_empty() {
            spans.push(Span::raw(before));
        }
        spans.push(Span::styled(cursor_ch, cursor_style));
        if !after.is_empty() {
            spans.push(Span::raw(after));
        }
        Line::from(spans)
    } else if is_cursor_line {
        // Normal mode keeps the Vim-style block cursor over the current char.
        let x = floor_char_boundary(line, state.cursor_x.min(line.len()));
        let before = &line[..x];
        let (cursor_ch, after): (String, &str) = if x < line.len() {
            let end = char_end(line, x);
            (line[x..end].to_string(), &line[end..])
        } else {
            ("█".to_string(), "")
        };
        let cur_style = if x < line.len() {
            Style::default().fg(Color::Black).bg(theme.selection)
        } else {
            Style::default()
                .fg(theme.selection)
                .add_modifier(Modifier::BOLD)
        };
        let mut spans: Vec<Span<'a>> = Vec::new();
        if !before.is_empty() {
            spans.push(Span::raw(before));
        }
        spans.push(Span::styled(cursor_ch, cur_style));
        if !after.is_empty() {
            spans.push(Span::raw(after));
        }
        Line::from(spans)
    } else {
        Line::from(line)
    }
}

fn cursor_visual_row(state: &EditorState, width: u16) -> usize {
    if width == 0 {
        return 0;
    }

    let mut cursor_lines: Vec<Line<'static>> = state.lines[..state.cursor_y]
        .iter()
        .cloned()
        .map(Line::from)
        .collect();
    let line = &state.lines[state.cursor_y];
    let x = floor_char_boundary(line, state.cursor_x.min(line.len()));
    let cursor_line = if state.mode == EditorMode::Insert && x < line.len() {
        line[..char_end(line, x)].to_string()
    } else if state.mode == EditorMode::Insert {
        format!("{}│", &line[..x])
    } else if x < line.len() {
        line[..char_end(line, x)].to_string()
    } else {
        format!("{line}█")
    };
    cursor_lines.push(Line::from(cursor_line));

    Paragraph::new(cursor_lines)
        .wrap(Wrap { trim: false })
        .line_count(width)
        .saturating_sub(1)
}

// ── draw ──────────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, state: &mut EditorState, theme: &Theme) {
    draw_in_area(f, state, theme, f.size());
}

pub fn draw_in_area(f: &mut Frame, state: &mut EditorState, theme: &Theme, size: Rect) {
    let accent = theme.primary;

    if state.quick_note {
        f.render_widget(Clear, size);
        f.render_widget(
            Block::default()
                .style(Style::default().bg(theme.background))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(accent))
                .title(Span::styled(
                    " Quick Scroll ",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                )),
            size,
        );
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(if state.quick_note {
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // Campaign
                Constraint::Min(5),    // Body
                Constraint::Length(3), // Status bar
            ]
        } else {
            vec![
                Constraint::Length(3), // Title
                Constraint::Min(5),    // Body
                Constraint::Length(3), // Status bar
            ]
        })
        .split(size);

    // ── Title ─────────────────────────────────────────────────────────────────
    let title_border_style = if state.editing_title {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let title_text = if state.editing_title {
        state.title.clone()
    } else if state.title.is_empty() {
        "Untitled Scroll".to_string()
    } else {
        state.title.clone()
    };
    let title_block_label = if state.editing_title {
        if state.quick_note {
            " Scroll Title  Tab/Enter: Campaign "
        } else {
            " Scroll Title  Tab/Enter: Body "
        }
    } else {
        " Scroll Title  Shift+Tab: edit "
    };
    f.render_widget(
        Paragraph::new(title_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(title_border_style)
                .title(title_block_label),
        ),
        chunks[0],
    );

    let body_idx = if state.quick_note { 2 } else { 1 };
    let status_idx = if state.quick_note { 3 } else { 2 };

    // ── Campaign ──────────────────────────────────────────────────────────────
    if state.quick_note {
        let project_style = if state.editing_project {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border)
        };
        let project_text = if state.editing_project {
            format!("◀  {}  ▶", state.selected_project_name())
        } else {
            state.selected_project_name().to_string()
        };
        let project_label = if state.editing_project {
            " Campaign  ↑/↓: choose  Enter: Editor "
        } else {
            " Campaign  Shift+Tab: change "
        };
        f.render_widget(
            Paragraph::new(project_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(project_style)
                    .title(project_label),
            ),
            chunks[1],
        );
    }

    // ── Body ──────────────────────────────────────────────────────────────────
    let body_height = chunks[body_idx].height.saturating_sub(2) as usize;
    let body_width = chunks[body_idx].width.saturating_sub(2);

    // Paragraph wraps long logical lines into several terminal rows. Track the
    // cursor in those visual rows so pasted text cannot continue below the box.
    if !state.editing_title && !state.editing_project && body_height > 0 && body_width > 0 {
        let cursor_row = cursor_visual_row(state, body_width);
        // Keep one spare row when possible. Word wrapping can move the current
        // word down one row after the text following the cursor is considered.
        let visible_cursor_height = body_height.saturating_sub(1).max(1);
        if cursor_row < state.scroll_offset {
            state.scroll_offset = cursor_row;
        } else if cursor_row >= state.scroll_offset + visible_cursor_height {
            state.scroll_offset = cursor_row + 1 - visible_cursor_height;
        }
    }

    let body_border_style = if !state.editing_title && !state.editing_project {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };

    let lines_to_render: Vec<Line> = state
        .lines
        .iter()
        .enumerate()
        .map(|(line_i, line)| render_body_line(line, line_i, state, theme))
        .collect();

    f.render_widget(
        Paragraph::new(lines_to_render)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(body_border_style)
                    .title(" Scroll Editor "),
            )
            .wrap(Wrap { trim: false })
            .scroll((state.scroll_offset.min(u16::MAX as usize) as u16, 0)),
        chunks[body_idx],
    );

    // ── Status bar ────────────────────────────────────────────────────────────
    let mode_str = state.mode.label();
    let pending = &state.pending_cmd;
    let pos_str = format!(" {}:{} ", state.cursor_y + 1, state.cursor_x + 1);

    let mode_style = match &state.mode {
        EditorMode::Normal => Style::default()
            .fg(Color::Black)
            .bg(theme.primary)
            .add_modifier(Modifier::BOLD),
        EditorMode::Insert => Style::default()
            .fg(Color::Black)
            .bg(theme.success)
            .add_modifier(Modifier::BOLD),
        EditorMode::Visual { .. } => Style::default()
            .fg(Color::Black)
            .bg(theme.warning)
            .add_modifier(Modifier::BOLD),
    };

    let mut status_spans = vec![
        Span::styled(format!(" {} ", mode_str), mode_style),
        Span::styled("  ", Style::default()),
    ];
    if state
        .last_autosaved_at
        .map(|saved_at| saved_at.elapsed().as_secs() < 4)
        .unwrap_or(false)
    {
        status_spans.push(Span::styled(
            "✓ Autosaved  ",
            Style::default().fg(theme.muted),
        ));
    }
    if !pending.is_empty() {
        status_spans.push(Span::styled(
            format!(" {pending}_ "),
            Style::default().fg(theme.warning),
        ));
        status_spans.push(Span::styled("  ", Style::default()));
    }
    if state.editing_title {
        let esc_label = if state.note_id.is_some() {
            " Body"
        } else {
            " Cancel"
        };
        status_spans.extend([
            Span::styled(
                "Ctrl+S",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save", Style::default().fg(theme.muted)),
            Span::styled("  Enter", Style::default().fg(accent)),
            Span::styled(" Body", Style::default().fg(theme.muted)),
            Span::styled("  Esc", Style::default().fg(accent)),
            Span::styled(esc_label, Style::default().fg(theme.muted)),
        ]);
    } else if state.editing_project {
        status_spans.extend([
            Span::styled(
                "Ctrl+S",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save", Style::default().fg(theme.muted)),
            Span::styled("  ↑/↓", Style::default().fg(accent)),
            Span::styled(" Campaign", Style::default().fg(theme.muted)),
            Span::styled("  Enter", Style::default().fg(accent)),
            Span::styled(" Editor", Style::default().fg(theme.muted)),
            Span::styled("  Esc", Style::default().fg(accent)),
            Span::styled(" Title", Style::default().fg(theme.muted)),
        ]);
    } else {
        let (esc_action, esc_label) = match state.mode {
            EditorMode::Normal => (" Close", "Esc"),
            EditorMode::Insert | EditorMode::Visual { .. } => (" Normal", "Esc"),
        };
        status_spans.extend([
            Span::styled(
                "Ctrl+S",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save", Style::default().fg(theme.muted)),
            Span::styled(format!("  {esc_label}"), Style::default().fg(accent)),
            Span::styled(esc_action, Style::default().fg(theme.muted)),
            Span::styled("  ?", Style::default().fg(accent)),
            Span::styled(" Help", Style::default().fg(theme.muted)),
            Span::styled(pos_str, Style::default().fg(theme.muted)),
        ]);
    }

    f.render_widget(
        Paragraph::new(Line::from(status_spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        ),
        chunks[status_idx],
    );

    // The body already renders an inline cursor in `render_body_line`. Drawing a
    // second terminal cursor there drifts onto another visual row when long or
    // pasted text wraps. Only the title uses the terminal cursor.
    if !state.show_help {
        if state.editing_title {
            let col = state.title.chars().count() as u16;
            f.set_cursor(
                (chunks[0].x + 1 + col).min(chunks[0].x + chunks[0].width.saturating_sub(2)),
                chunks[0].y + 1,
            );
        }
    }

    // ── Help popup ────────────────────────────────────────────────────────────
    if state.show_help {
        draw_help_popup(f, size, theme);
    }
    if state.confirm_close {
        draw_unsaved_changes_popup(f, size, theme);
    }
}

fn draw_unsaved_changes_popup(f: &mut Frame, area: Rect, theme: &Theme) {
    let popup_w = 58u16.min(area.width);
    let popup_h = 7u16.min(area.height);
    let popup_area = Rect {
        x: area.x + area.width.saturating_sub(popup_w) / 2,
        y: area.y + area.height.saturating_sub(popup_h) / 2,
        width: popup_w,
        height: popup_h,
    };
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "This scroll has unsaved changes.",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[S/Enter]", Style::default().fg(theme.success)),
                Span::raw(" Save & Close   "),
                Span::styled("[D]", Style::default().fg(theme.warning)),
                Span::raw(" Discard   "),
                Span::styled("[Esc]", Style::default().fg(theme.primary)),
                Span::raw(" Keep Editing"),
            ]),
        ])
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .style(Style::default().bg(theme.background))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(" Unsaved Scroll "),
        ),
        popup_area,
    );
}

fn draw_help_popup(f: &mut Frame, area: Rect, theme: &Theme) {
    let popup_w: u16 = 52;
    let popup_h: u16 = 27;
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect {
        x,
        y,
        width: popup_w.min(area.width),
        height: popup_h.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let accent = theme.primary;
    let head = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(Color::White);
    let muted = Style::default().fg(theme.muted);

    let kd = |k: &'static str, d: &'static str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<18}", k), key),
            Span::styled(d, desc),
        ])
    };

    let lines: Vec<Line> = vec![
        Line::from(Span::styled(" MOTION", head)),
        kd("h j k l", "← ↓ ↑ →"),
        kd("w / b / e", "next / prev / end word"),
        kd("0  $", "line start / end"),
        kd("gg  G", "file start / end"),
        Line::from(""),
        Line::from(Span::styled(" INSERT", head)),
        kd("i / a", "insert before / after cursor"),
        kd("I / A", "insert at line start / end"),
        kd("o / O", "new line below / above"),
        Line::from(""),
        Line::from(Span::styled(" EDIT  (Normal mode)", head)),
        kd("x / X", "delete char fwd / bwd"),
        kd("r{c}", "replace char with {c}"),
        kd("dd  D", "delete line / to end"),
        kd("dw  diw", "delete word / inner word"),
        kd("cc  cw  ciw  C", "change variants"),
        kd("yy  p  P", "yank line / paste ↓ / ↑"),
        kd("u  Ctrl+R", "undo / redo"),
        Line::from(""),
        Line::from(Span::styled(" VISUAL", head)),
        kd("v", "char select"),
        kd("V", "line select"),
        kd("y  d  c", "yank / delete / change"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ?", key),
            Span::styled("  toggle help", muted),
            Span::styled("    Ctrl+S", key),
            Span::styled("  save", muted),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent))
                .title(" Vim Keys "),
        ),
        popup_area,
    );
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

    #[test]
    fn bulk_paste_inserts_unicode_paragraphs_in_one_edit() {
        let project_id = Uuid::new_v4();
        let mut editor =
            EditorState::new(project_id, None, String::new(), "Antes después".to_string());
        editor.editing_title = false;
        editor.cursor_x = "Antes ".len();

        editor.insert_text("información\r\nPhishing:\nAtaque DoS");

        assert_eq!(
            editor.lines,
            vec![
                "Antes información".to_string(),
                "Phishing:".to_string(),
                "Ataque DoSdespués".to_string(),
            ]
        );
        assert_eq!(editor.cursor_y, 2);
        assert_eq!(editor.cursor_x, "Ataque DoS".len());
        assert_eq!(editor.undo_history.len(), 1);
        assert_eq!(editor.mode, EditorMode::Insert);
    }

    #[test]
    fn quick_note_uses_active_campaign_and_keeps_all_choices() {
        let first_id = Uuid::new_v4();
        let active_id = Uuid::new_v4();
        let choices = vec![
            (first_id, "First Campaign".to_string()),
            (active_id, "Active Campaign".to_string()),
        ];

        let editor = EditorState::new_quick_note(choices, Some(active_id)).unwrap();

        assert!(editor.quick_note);
        assert!(editor.editing_title);
        assert_eq!(editor.project_id, active_id);
        assert_eq!(editor.project_choice_idx, 1);
        assert_eq!(editor.selected_project_name(), "Active Campaign");
    }

    #[test]
    fn quick_note_requires_an_active_campaign() {
        assert!(EditorState::new_quick_note(Vec::new(), None).is_none());
    }

    #[test]
    fn insert_cursor_is_visible_on_empty_lines_and_between_text() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(project_id, None, String::new(), String::new());
        editor.editing_title = false;
        editor.mode = EditorMode::Insert;
        let theme = Theme::default_theme();

        let empty_cursor = render_body_line("", 0, &editor, &theme);
        let empty_text: String = empty_cursor
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(empty_text, "│");

        editor.lines[0] = "pasted text".to_string();
        editor.cursor_x = "pasted ".len();
        let text_cursor = render_body_line(&editor.lines[0], 0, &editor, &theme);
        let rendered: String = text_cursor
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(rendered, "pasted text");
        assert_eq!(text_cursor.spans[1].content.as_ref(), "t");
        assert_eq!(text_cursor.spans[1].style.bg, Some(theme.success));
    }

    #[test]
    fn cursor_visual_row_counts_wrapped_rows_and_newlines() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(
            project_id,
            None,
            String::new(),
            "first\nabcdefghij".to_string(),
        );
        editor.editing_title = false;
        editor.mode = EditorMode::Insert;
        editor.cursor_y = 1;
        editor.cursor_x = editor.lines[1].len();

        // "first" occupies row 0. Ten characters plus the insert cursor wrap
        // across three rows at width 5, placing the cursor on global row 3.
        assert_eq!(cursor_visual_row(&editor, 5), 3);
    }

    #[test]
    fn editor_scrolls_when_wrapped_cursor_reaches_below_the_panel() {
        let project_id = Uuid::new_v4();
        let content = "wrapped text ".repeat(40);
        let mut editor = EditorState::new(project_id, None, String::new(), content.clone());
        editor.editing_title = false;
        editor.mode = EditorMode::Insert;
        editor.cursor_x = content.len();
        let theme = Theme::default_theme();
        let backend = ratatui::backend::TestBackend::new(30, 14);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| draw(frame, &mut editor, &theme))
            .unwrap();

        assert!(editor.scroll_offset > 0);
    }

    #[test]
    fn unsaved_changes_clear_after_marking_the_note_saved() {
        let project_id = Uuid::new_v4();
        let mut editor =
            EditorState::new(project_id, None, "Draft".to_string(), "Body".to_string());
        assert!(!editor.has_unsaved_changes());

        editor.title.push_str(" changed");
        assert!(editor.has_unsaved_changes());

        let note_id = Uuid::new_v4();
        editor.mark_saved(note_id);
        assert!(!editor.has_unsaved_changes());
        assert_eq!(editor.note_id, Some(note_id));

        editor.lines.push("Another line".to_string());
        assert!(editor.has_unsaved_changes());
    }

    #[test]
    fn autosave_runs_only_for_changed_notes_after_the_interval() {
        let project_id = Uuid::new_v4();
        let mut editor =
            EditorState::new(project_id, None, "Draft".to_string(), "Body".to_string());
        let interval = std::time::Duration::from_secs(5);
        assert!(!editor.autosave_due(interval));

        editor.title.push('!');
        assert!(!editor.autosave_due(interval));
        editor.autosave_timer = std::time::Instant::now() - std::time::Duration::from_secs(6);
        assert!(editor.autosave_due(interval));

        editor.mark_saved(Uuid::new_v4());
        editor.record_autosaved();
        assert!(!editor.autosave_due(interval));
        assert!(editor.last_autosaved_at.is_some());
    }

    #[test]
    fn test_ctrl_backspace_deletes_previous_word() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(
            project_id,
            Some(Uuid::new_v4()),
            "Title".to_string(),
            "alpha beta gamma".to_string(),
        );
        editor.mode = EditorMode::Insert;
        editor.cursor_x = "alpha beta ".len();

        editor.handle_ctrl_backspace();

        assert_eq!(editor.lines[0], "alpha gamma");
        assert_eq!(editor.cursor_x, "alpha ".len());
    }

    #[test]
    fn test_vim_undo_redo() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(
            project_id,
            None,
            "Title".to_string(),
            "hello world".to_string(),
        );
        editor.editing_title = false;

        editor.push_undo();
        editor.lines[0] = "goodbye world".to_string();

        editor.undo();
        assert_eq!(editor.lines[0], "hello world");

        editor.redo();
        assert_eq!(editor.lines[0], "goodbye world");
    }

    #[test]
    fn test_up_on_first_body_line_stays_in_body() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(
            project_id,
            Some(Uuid::new_v4()),
            "Title".to_string(),
            "alpha\nbeta".to_string(),
        );
        editor.editing_title = false;
        editor.cursor_y = 0;

        editor.normal_k();
        assert!(!editor.editing_title);
        assert_eq!(editor.cursor_y, 0);

        editor.mode = EditorMode::Insert;
        editor.move_up();
        assert!(!editor.editing_title);
        assert_eq!(editor.cursor_y, 0);
    }

    #[test]
    fn test_visual_mode_line_navigation_stays_in_body() {
        let project_id = Uuid::new_v4();
        let mut editor = EditorState::new(
            project_id,
            Some(Uuid::new_v4()),
            "Title".to_string(),
            "alpha\nbeta\ngamma".to_string(),
        );
        editor.editing_title = false;
        editor.enter_visual_char();

        editor.normal_j();
        assert!(!editor.editing_title);
        assert_eq!(editor.cursor_y, 1);

        editor.normal_k();
        assert!(!editor.editing_title);
        assert_eq!(editor.cursor_y, 0);

        editor.normal_k();
        assert!(!editor.editing_title);
        assert_eq!(editor.cursor_y, 0);
        assert!(matches!(editor.mode, EditorMode::Visual { .. }));
    }
}
