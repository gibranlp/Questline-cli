use crate::{database::Database, portals, theme::Theme};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct PortalState {
    pub data: Value,
    pub selected: usize,
    pub category: usize,
    pub search: String,
    pub message: String,
    pub editor: Option<Editor>,
    refreshed: Option<Instant>,
}

pub struct Editor {
    kind: &'static str,
    id: String,
    fields: Vec<String>,
    field: usize,
    before: Value,
}

impl PortalState {
    pub fn paste(&mut self, text: &str) {
        if let Some(editor) = &mut self.editor {
            editor.fields[editor.field].extend(text.chars().filter(|c| !c.is_control()).take(4096));
        }
    }

    pub fn refresh(&mut self, db: &Database, force: bool) -> Result<()> {
        if !force
            && self
                .refreshed
                .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
        {
            return Ok(());
        }
        match portals::snapshot(db) {
            Ok(data) => self.data = data,
            Err(error) => {
                self.message = error.to_string();
                self.data = json!({"categories":[{"id":"uncategorized","name":"Uncategorized"}],"favorites":[]});
            }
        }
        self.refreshed = Some(Instant::now());
        self.category = self
            .category
            .min(self.data["categories"].as_array().map_or(0, Vec::len));
        self.selected = self.selected.min(self.links().len().saturating_sub(1));
        Ok(())
    }
    fn category_id(&self) -> &str {
        if self.category == 0 {
            "top"
        } else {
            self.data["categories"][self.category - 1]["id"]
                .as_str()
                .unwrap_or("uncategorized")
        }
    }
    fn category_name(&self, id: &str) -> &str {
        self.data["categories"]
            .as_array()
            .and_then(|rows| rows.iter().find(|v| v["id"] == id))
            .and_then(|v| v["name"].as_str())
            .unwrap_or("Uncategorized")
    }
    fn links(&self) -> Vec<Value> {
        let category = self.category_id();
        let query = self.search.to_lowercase();
        let mut rows = self.data["favorites"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        rows.retain(|v| {
            let name = self.category_name(v["category"].as_str().unwrap_or(""));
            if !query.is_empty() {
                format!(
                    "{} {} {name}",
                    v["title"].as_str().unwrap_or(""),
                    v["url"].as_str().unwrap_or("")
                )
                .to_lowercase()
                .contains(&query)
            } else {
                category == "top"
                    || v["category"] == category
                    || (category == "uncategorized" && name == "Uncategorized")
            }
        });
        rows.sort_by(|a, b| {
            b["clickCount"]
                .as_i64()
                .cmp(&a["clickCount"].as_i64())
                .then_with(|| {
                    a["title"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(&b["title"].as_str().unwrap_or("").to_lowercase())
                })
        });
        if category == "top" && query.is_empty() {
            rows.truncate(24);
        }
        rows
    }
    fn apply(&mut self, db: &Database, before: &Value, after: Value, visit: bool) -> Result<()> {
        self.data = portals::exchange(
            db,
            &json!({"action":"apply","account":before["questlineAccount"],"before":before,"after":after,"countMode":if visit {"visit"} else {"minimum"}}),
        )?;
        self.message = "Saved locally; queued for encrypted sync.".into();
        self.refreshed = Some(Instant::now());
        self.selected = self.selected.min(self.links().len().saturating_sub(1));
        Ok(())
    }
    pub fn key(&mut self, db: &Database, key: KeyEvent) -> Result<(bool, bool)> {
        self.refresh(db, false)?;
        self.message.clear();
        if let Some(mut editor) = self.editor.take() {
            match key.code {
                KeyCode::Esc => return Ok((true, false)),
                KeyCode::Tab => editor.field = (editor.field + 1) % editor.fields.len(),
                KeyCode::BackTab => {
                    editor.field = (editor.field + editor.fields.len() - 1) % editor.fields.len()
                }
                KeyCode::Backspace => {
                    editor.fields[editor.field].pop();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    editor.fields[editor.field].push(c)
                }
                KeyCode::Enter => {
                    if editor.kind == "search" {
                        self.search = editor.fields[0].clone();
                        self.selected = 0;
                        return Ok((true, false));
                    }
                    let before = editor.before.clone();
                    let mut after = before.clone();
                    let row = if editor.kind == "favorite" {
                        let category = before["categories"].as_array().unwrap().iter().find(|c| {
                            c["name"]
                                .as_str()
                                .is_some_and(|n| n.eq_ignore_ascii_case(editor.fields[2].trim()))
                                || c["id"] == editor.fields[2]
                        });
                        if let Some(c) = category {
                            json!({"id":editor.id,"title":editor.fields[0],"url":editor.fields[1],"category":c["id"],"clickCount":0})
                        } else {
                            self.message = "Enter an existing category name.".into();
                            self.editor = Some(editor);
                            return Ok((true, false));
                        }
                    } else {
                        json!({"id":editor.id,"name":editor.fields[0]})
                    };
                    let collection = if editor.kind == "favorite" {
                        "favorites"
                    } else {
                        "categories"
                    };
                    let rows = after[collection].as_array_mut().unwrap();
                    if let Some(existing) = rows.iter_mut().find(|v| v["id"] == editor.id) {
                        let count = existing.get("clickCount").cloned();
                        *existing = row;
                        if let Some(count) = count {
                            existing["clickCount"] = count;
                        }
                    } else {
                        rows.push(row);
                    }
                    match self.apply(db, &before, after, false) {
                        Ok(()) => return Ok((true, true)),
                        Err(error) => self.message = error.to_string(),
                    }
                }
                _ => {}
            }
            self.editor = Some(editor);
            return Ok((true, false));
        }
        if !matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Enter
                | KeyCode::Delete
                | KeyCode::Esc
                | KeyCode::Char('j' | 'k' | '/' | 'n' | 'e' | 'c' | 'r' | 'd' | 'D')
        ) {
            return Ok((false, false));
        }
        let before = match portals::snapshot(db) {
            Ok(value) => value,
            Err(error) => {
                self.message = error.to_string();
                return Ok((true, false));
            }
        };
        let result = (|| -> Result<(bool, bool)> {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected = (self.selected + 1).min(self.links().len().saturating_sub(1))
                }
                KeyCode::Left => {
                    self.category = self.category.saturating_sub(1);
                    self.selected = 0;
                }
                KeyCode::Right => {
                    self.category = (self.category + 1)
                        .min(self.data["categories"].as_array().map_or(0, Vec::len));
                    self.selected = 0;
                }
                KeyCode::Char('/') => {
                    self.editor = Some(Editor {
                        kind: "search",
                        id: String::new(),
                        fields: vec![self.search.clone()],
                        field: 0,
                        before,
                    })
                }
                KeyCode::Char('n') | KeyCode::Char('e') => {
                    let existing = if key.code == KeyCode::Char('e') {
                        Some(
                            self.links()
                                .get(self.selected)
                                .cloned()
                                .context("Select a portal first")?,
                        )
                    } else {
                        None
                    };
                    let id = existing
                        .as_ref()
                        .and_then(|v| v["id"].as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let fields = if let Some(v) = existing {
                        vec![
                            v["title"].as_str().unwrap_or("").into(),
                            v["url"].as_str().unwrap_or("").into(),
                            self.category_name(v["category"].as_str().unwrap_or(""))
                                .into(),
                        ]
                    } else {
                        vec![
                            String::new(),
                            String::new(),
                            if self.category == 0 {
                                "Uncategorized".into()
                            } else {
                                self.category_name(self.category_id()).into()
                            },
                        ]
                    };
                    self.editor = Some(Editor {
                        kind: "favorite",
                        id,
                        fields,
                        field: 0,
                        before,
                    });
                }
                KeyCode::Char('c') | KeyCode::Char('r') => {
                    let (id, name) = if key.code == KeyCode::Char('r') {
                        if matches!(self.category_id(), "top" | "uncategorized") {
                            anyhow::bail!("Select a custom category first");
                        }
                        (
                            self.category_id().to_string(),
                            self.category_name(self.category_id()).to_string(),
                        )
                    } else {
                        (uuid::Uuid::new_v4().to_string(), String::new())
                    };
                    self.editor = Some(Editor {
                        kind: "favorite_category",
                        id,
                        fields: vec![name],
                        field: 0,
                        before,
                    });
                }
                KeyCode::Delete | KeyCode::Char('d') => {
                    let link = self
                        .links()
                        .get(self.selected)
                        .cloned()
                        .context("Select a portal first")?;
                    let mut after = before.clone();
                    after["favorites"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|v| v["id"] != link["id"]);
                    self.apply(db, &before, after, false)?;
                    return Ok((true, true));
                }
                KeyCode::Char('D') => {
                    if matches!(self.category_id(), "top" | "uncategorized") {
                        anyhow::bail!("Select an empty custom category first");
                    }
                    let mut after = before.clone();
                    after["categories"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|v| v["id"] != self.category_id());
                    self.apply(db, &before, after, false)?;
                    self.category = 0;
                    return Ok((true, true));
                }
                KeyCode::Enter => {
                    let link = self
                        .links()
                        .get(self.selected)
                        .cloned()
                        .context("Select a portal first")?;
                    portals::open_url(link["url"].as_str().context("Missing URL")?)?;
                    let mut after = before.clone();
                    let row = after["favorites"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|v| v["id"] == link["id"])
                        .context("This portal was deleted; refresh first")?;
                    row["clickCount"] = json!(
                        row["clickCount"]
                            .as_i64()
                            .unwrap_or(0)
                            .saturating_add(1)
                            .min(9_007_199_254_740_991)
                    );
                    self.apply(db, &before, after, true)?;
                    return Ok((true, true));
                }
                KeyCode::Esc if !self.search.is_empty() => {
                    self.search.clear();
                    self.selected = 0;
                }
                _ => return Ok((false, false)),
            }
            Ok((true, false))
        })();
        match result {
            Ok(v) => Ok(v),
            Err(error) => {
                self.message = error.to_string();
                Ok((true, false))
            }
        }
    }
}

pub fn draw(frame: &mut Frame, state: &PortalState, theme: &Theme, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(if state.editor.is_some() { 8 } else { 3 }),
        ])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(layout[0]);
    let mut categories = vec![ListItem::new("TOP")];
    categories.extend(
        state.data["categories"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| ListItem::new(c["name"].as_str().unwrap_or(""))),
    );
    let mut category = ListState::default().with_selected(Some(state.category));
    frame.render_stateful_widget(
        List::new(categories)
            .block(Block::default().borders(Borders::ALL).title(" Categories "))
            .highlight_style(theme.primary_selected_style()),
        columns[0],
        &mut category,
    );
    let links = state.links();
    let rows: Vec<ListItem> = links
        .iter()
        .map(|v| {
            ListItem::new(vec![
                Line::from(format!(
                    "{} · {} opens",
                    v["title"].as_str().unwrap_or(""),
                    v["clickCount"]
                )),
                Line::from(v["url"].as_str().unwrap_or("")),
            ])
        })
        .collect();
    let mut selected = ListState::default().with_selected(if rows.is_empty() {
        None
    } else {
        Some(state.selected)
    });
    frame.render_stateful_widget(
        List::new(rows)
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Portals — synced favorites {} ",
                if state.search.is_empty() {
                    String::new()
                } else {
                    format!("/{}", state.search)
                }
            )))
            .highlight_style(theme.primary_selected_style())
            .highlight_symbol("› "),
        columns[1],
        &mut selected,
    );
    if let Some(editor) = &state.editor {
        let names = if editor.kind == "favorite" {
            vec!["Title", "URL", "Category"]
        } else if editor.kind == "search" {
            vec!["Search"]
        } else {
            vec!["Category name"]
        };
        let mut lines = vec![Line::from("Tab: next field · Enter: save · Esc: cancel")];
        for (i, value) in editor.fields.iter().enumerate() {
            let line = Line::from(format!(
                "{} {}: {}",
                if i == editor.field { "›" } else { " " },
                names[i],
                value
            ));
            lines.push(if i == editor.field {
                line.style(
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                line
            });
        }
        lines.push(Line::from(state.message.as_str()));
        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Edit portal "),
                )
                .wrap(Wrap { trim: false }),
            layout[1],
        );
    } else {
        frame.render_widget(Paragraph::new(vec![Line::from("←/→ category · ↑/↓ select · Enter open · n add · e edit · d delete · c category · r rename · D delete category · / search"),Line::from(state.message.as_str())]).block(Block::default().borders(Borders::ALL)).wrap(Wrap{trim:false}),layout[1]);
    }
}
