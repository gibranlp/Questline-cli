use crate::database::Database;
use crate::models::{AtlasChart, AtlasNode};
use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct AtlasService<'a> {
    db: &'a Database,
}

impl<'a> AtlasService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn charts(&self, project_id: Uuid) -> Result<Vec<AtlasChart>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, project_id, name, created_at, updated_at, owner_identity
             FROM atlas_charts WHERE project_id=?1 ORDER BY LOWER(name), created_at",
        )?;
        let rows = stmt.query_map(params![project_id.to_string()], chart_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn chart(&self, id: Uuid) -> Result<Option<AtlasChart>> {
        self.db
            .conn
            .query_row(
                "SELECT id, project_id, name, created_at, updated_at, owner_identity
             FROM atlas_charts WHERE id=?1",
                params![id.to_string()],
                chart_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn create_chart(
        &self,
        project_id: Uuid,
        name: &str,
        owner: Option<&str>,
    ) -> Result<AtlasChart> {
        let name = name.trim();
        if name.is_empty() {
            bail!("A Chart needs a name");
        }
        let now = Utc::now();
        let chart = AtlasChart {
            id: Uuid::new_v4(),
            project_id,
            name: name.to_string(),
            created_at: now,
            updated_at: now,
            owner_identity: owner.map(str::to_string),
        };
        let root = AtlasNode {
            id: Uuid::new_v4(),
            chart_id: chart.id,
            project_id,
            parent_id: None,
            title: name.to_string(),
            position: 0,
            collapsed: false,
            marked: false,
            hidden: false,
            color: None,
            created_at: now,
            updated_at: now,
            owner_identity: owner.map(str::to_string),
        };
        let tx = self.db.conn.unchecked_transaction()?;
        tx.execute("INSERT INTO atlas_charts (id,project_id,name,created_at,updated_at,owner_identity) VALUES (?1,?2,?3,?4,?5,?6)", params![chart.id.to_string(), project_id.to_string(), chart.name, now.to_rfc3339(), now.to_rfc3339(), chart.owner_identity])?;
        tx.execute("INSERT INTO atlas_nodes (id,chart_id,project_id,parent_id,title,position,collapsed,marked,created_at,updated_at,owner_identity) VALUES (?1,?2,?3,NULL,?4,0,0,0,?5,?6,?7)", params![root.id.to_string(), chart.id.to_string(), project_id.to_string(), root.title, now.to_rfc3339(), now.to_rfc3339(), root.owner_identity])?;
        Database::log_change_on(&tx, "atlas_chart", &chart.id.to_string(), "create")?;
        Database::log_change_on(&tx, "atlas_node", &root.id.to_string(), "create")?;
        tx.commit()?;
        Ok(chart)
    }

    pub fn rename_chart(&self, id: Uuid, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            bail!("A Chart needs a name");
        }
        let changed = self.db.conn.execute(
            "UPDATE atlas_charts SET name=?2,updated_at=?3 WHERE id=?1",
            params![id.to_string(), name, Utc::now().to_rfc3339()],
        )?;
        if changed == 0 {
            bail!("Chart no longer exists");
        }
        self.db.log_change("atlas_chart", &id.to_string(), "update")
    }

    pub fn delete_chart(&self, id: Uuid) -> Result<()> {
        let chart = self.chart(id)?.context("Chart no longer exists")?;
        let nodes = self.nodes(id)?;
        let tx = self.db.conn.unchecked_transaction()?;
        Database::create_revision_on(
            &tx,
            "atlas_chart",
            &id.to_string(),
            &serde_json::to_string(&chart)?,
        )?;
        for node in nodes {
            Database::create_revision_on(
                &tx,
                "atlas_node",
                &node.id.to_string(),
                &serde_json::to_string(&node)?,
            )?;
            Database::log_change_on(&tx, "atlas_node", &node.id.to_string(), "delete")?;
        }
        tx.execute(
            "DELETE FROM atlas_charts WHERE id=?1",
            params![id.to_string()],
        )?;
        Database::log_change_on(&tx, "atlas_chart", &id.to_string(), "delete")?;
        tx.commit()?;
        Ok(())
    }

    pub fn nodes(&self, chart_id: Uuid) -> Result<Vec<AtlasNode>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id,chart_id,project_id,parent_id,title,position,collapsed,marked,created_at,updated_at,owner_identity,hidden,color
             FROM atlas_nodes WHERE chart_id=?1 ORDER BY position,created_at,id",
        )?;
        let rows = stmt.query_map(params![chart_id.to_string()], node_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn node(&self, id: Uuid) -> Result<Option<AtlasNode>> {
        self.db.conn.query_row(
            "SELECT id,chart_id,project_id,parent_id,title,position,collapsed,marked,created_at,updated_at,owner_identity,hidden,color FROM atlas_nodes WHERE id=?1",
            params![id.to_string()], node_from_row,
        ).optional().map_err(Into::into)
    }

    pub fn add_node(
        &self,
        chart_id: Uuid,
        parent_id: Option<Uuid>,
        title: &str,
        owner: Option<&str>,
    ) -> Result<AtlasNode> {
        let chart = self.chart(chart_id)?.context("Chart no longer exists")?;
        let mut inherited_color = None;
        if let Some(parent) = parent_id {
            let p = self.node(parent)?.context("Parent node no longer exists")?;
            if p.chart_id != chart_id {
                bail!("Parent belongs to another Chart");
            }
            inherited_color = p.color;
        }
        let position = self.db.conn.query_row(
            "SELECT COALESCE(MAX(position),-1)+1 FROM atlas_nodes WHERE chart_id=?1 AND parent_id IS ?2",
            params![chart_id.to_string(), parent_id.map(|v| v.to_string())], |r| r.get(0),
        )?;
        let now = Utc::now();
        let node = AtlasNode {
            id: Uuid::new_v4(),
            chart_id,
            project_id: chart.project_id,
            parent_id,
            title: title.trim().to_string(),
            position,
            collapsed: false,
            marked: false,
            hidden: false,
            color: inherited_color,
            created_at: now,
            updated_at: now,
            owner_identity: owner.map(str::to_string),
        };
        if node.title.is_empty() {
            bail!("A branch needs a title");
        }
        self.upsert_node(&node, "create")?;
        Ok(node)
    }

    pub fn update_node(&self, node: &AtlasNode) -> Result<()> {
        self.upsert_node(node, "update")
    }

    pub fn color_branch(&self, id: Uuid, color: Option<u8>) -> Result<()> {
        if color.is_some_and(|value| value > 15) { bail!("Atlas color must be between 1 and 16"); }
        let node = self.node(id)?.context("Branch no longer exists")?;
        let all = self.nodes(node.chart_id)?;
        for target in std::iter::once(id).chain(descendant_ids(&all, id)) {
            if let Some(mut branch) = all.iter().find(|branch| branch.id == target).cloned() {
                branch.color = color;
                branch.updated_at = Utc::now();
                self.update_node(&branch)?;
            }
        }
        Ok(())
    }

    fn upsert_node(&self, node: &AtlasNode, operation: &str) -> Result<()> {
        self.db.conn.execute(
            "INSERT INTO atlas_nodes (id,chart_id,project_id,parent_id,title,position,collapsed,marked,created_at,updated_at,owner_identity,hidden,color)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(id) DO UPDATE SET chart_id=excluded.chart_id,project_id=excluded.project_id,parent_id=excluded.parent_id,title=excluded.title,position=excluded.position,collapsed=excluded.collapsed,marked=excluded.marked,updated_at=excluded.updated_at,owner_identity=excluded.owner_identity,hidden=excluded.hidden,color=excluded.color",
            params![node.id.to_string(),node.chart_id.to_string(),node.project_id.to_string(),node.parent_id.map(|v|v.to_string()),node.title,node.position,node.collapsed as i32,node.marked as i32,node.created_at.to_rfc3339(),node.updated_at.to_rfc3339(),node.owner_identity,node.hidden as i32,node.color],
        )?;
        self.db
            .log_change("atlas_node", &node.id.to_string(), operation)
    }

    pub fn delete_node(&self, id: Uuid) -> Result<()> {
        let node = self.node(id)?.context("Branch no longer exists")?;
        let all = self.nodes(node.chart_id)?;
        let descendants = descendant_ids(&all, id);
        let tx = self.db.conn.unchecked_transaction()?;
        for child in descendants.iter().rev().chain(std::iter::once(&id)) {
            if let Some(snapshot) = all.iter().find(|node| node.id == *child) {
                Database::create_revision_on(
                    &tx,
                    "atlas_node",
                    &child.to_string(),
                    &serde_json::to_string(snapshot)?,
                )?;
            }
            Database::log_change_on(&tx, "atlas_node", &child.to_string(), "delete")?;
        }
        tx.execute(
            "DELETE FROM atlas_nodes WHERE id=?1",
            params![id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn visible_nodes(&self, chart_id: Uuid) -> Result<Vec<(AtlasNode, usize)>> {
        self.visible_nodes_with_hidden(chart_id, false)
    }

    pub fn visible_nodes_with_hidden(&self, chart_id: Uuid, show_hidden: bool) -> Result<Vec<(AtlasNode, usize)>> {
        self.visible_nodes_for_view(chart_id, show_hidden, &std::collections::HashSet::new())
    }

    pub fn visible_nodes_for_view(&self, chart_id: Uuid, show_hidden: bool, expanded: &std::collections::HashSet<Uuid>) -> Result<Vec<(AtlasNode, usize)>> {
        let nodes = self.nodes(chart_id)?;
        Ok(flatten_visible(&nodes, true, show_hidden, expanded))
    }

    pub fn move_node(&self, id: Uuid, delta: i64) -> Result<()> {
        let node = self.node(id)?.context("Branch no longer exists")?;
        let mut siblings = self
            .nodes(node.chart_id)?
            .into_iter()
            .filter(|n| n.parent_id == node.parent_id)
            .collect::<Vec<_>>();
        siblings.sort_by_key(|n| (n.position, n.created_at, n.id));
        let Some(idx) = siblings.iter().position(|n| n.id == id) else {
            return Ok(());
        };
        let target = if delta < 0 {
            idx.checked_sub(1)
        } else if idx + 1 < siblings.len() {
            Some(idx + 1)
        } else {
            None
        };
        let Some(target) = target else {
            return Ok(());
        };
        let other_id = siblings[target].id;
        let tx = self.db.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE atlas_nodes SET position=?2,updated_at=?3 WHERE id=?1",
            params![
                id.to_string(),
                siblings[target].position,
                Utc::now().to_rfc3339()
            ],
        )?;
        tx.execute(
            "UPDATE atlas_nodes SET position=?2,updated_at=?3 WHERE id=?1",
            params![other_id.to_string(), node.position, Utc::now().to_rfc3339()],
        )?;
        Database::log_change_on(&tx, "atlas_node", &id.to_string(), "update")?;
        Database::log_change_on(&tx, "atlas_node", &other_id.to_string(), "update")?;
        tx.commit()?;
        Ok(())
    }

    pub fn export_markdown(&self, chart_id: Uuid) -> Result<String> {
        let chart = self.chart(chart_id)?.context("Chart no longer exists")?;
        let mut out = format!("# {}\n\n", chart.name);
        for (node, depth) in flatten(&self.nodes(chart_id)?, false) {
            writeln!(
                out,
                "{}- {}{}",
                "  ".repeat(depth),
                if node.marked { "[x] " } else { "" },
                node.title
            )?;
        }
        Ok(out)
    }

    pub fn export_text(&self, chart_id: Uuid) -> Result<String> {
        let mut out = String::new();
        for (node, depth) in flatten(&self.nodes(chart_id)?, false) {
            writeln!(
                out,
                "{}{}{}",
                "\t".repeat(depth),
                if node.marked { "✓ " } else { "" },
                node.title
            )?;
        }
        Ok(out)
    }

    pub fn export_html(&self, chart_id: Uuid) -> Result<String> {
        let chart = self.chart(chart_id)?.context("Chart no longer exists")?;
        let nodes = self.nodes(chart_id)?;
        let mut body = String::new();
        render_html_children(&mut body, &nodes, None);
        Ok(format!(
            r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>{}</title><style>body{{background:#09090b;color:#e4e4e7;font:16px system-ui;margin:3rem}}h1{{color:#f59e0b}}ul{{list-style:none;padding-left:2rem;border-left:1px solid #3f3f46}}li{{margin:.55rem 0}}li::before{{content:'◆';color:#a855f7;margin-right:.65rem}}.marked{{text-decoration:line-through;color:#a1a1aa}}</style></head><body><h1>{}</h1>{}</body></html>"#,
            html_escape(&chart.name),
            html_escape(&chart.name),
            body
        ))
    }

    /// Portable, dependency-free PDF export using PDF's built-in Helvetica.
    /// Non-ASCII glyphs are replaced in PDF only; HTML/Markdown/text retain Unicode.
    pub fn export_pdf(&self, chart_id: Uuid) -> Result<Vec<u8>> {
        let chart = self.chart(chart_id)?.context("Chart no longer exists")?;
        let mut lines = vec![(0usize, chart.name)];
        lines.extend(
            flatten(&self.nodes(chart_id)?, false)
                .into_iter()
                .map(|(n, d)| {
                    (
                        d + 1,
                        format!("{}{}", if n.marked { "[x] " } else { "" }, n.title),
                    )
                }),
        );
        Ok(simple_pdf(&lines))
    }

    pub fn export_all(&self, chart_id: Uuid, directory: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(directory)?;
        let chart = self.chart(chart_id)?.context("Chart no longer exists")?;
        let base = slug(&chart.name);
        let outputs = [
            directory.join(format!("{base}.md")),
            directory.join(format!("{base}.txt")),
            directory.join(format!("{base}.html")),
            directory.join(format!("{base}.pdf")),
        ];
        std::fs::write(&outputs[0], self.export_markdown(chart_id)?)?;
        std::fs::write(&outputs[1], self.export_text(chart_id)?)?;
        std::fs::write(&outputs[2], self.export_html(chart_id)?)?;
        std::fs::write(&outputs[3], self.export_pdf(chart_id)?)?;
        Ok(outputs.into_iter().collect())
    }
}

fn chart_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AtlasChart> {
    Ok(AtlasChart {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        project_id: parse_uuid(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        created_at: parse_time(row.get::<_, String>(3)?)?,
        updated_at: parse_time(row.get::<_, String>(4)?)?,
        owner_identity: row.get(5)?,
    })
}
fn node_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AtlasNode> {
    Ok(AtlasNode {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        chart_id: parse_uuid(row.get::<_, String>(1)?)?,
        project_id: parse_uuid(row.get::<_, String>(2)?)?,
        parent_id: row
            .get::<_, Option<String>>(3)?
            .map(parse_uuid)
            .transpose()?,
        title: row.get(4)?,
        position: row.get(5)?,
        collapsed: row.get::<_, i32>(6)? != 0,
        marked: row.get::<_, i32>(7)? != 0,
        hidden: row.get::<_, i32>(11)? != 0,
        color: row.get(12)?,
        created_at: parse_time(row.get::<_, String>(8)?)?,
        updated_at: parse_time(row.get::<_, String>(9)?)?,
        owner_identity: row.get(10)?,
    })
}
fn parse_uuid(s: String) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
fn parse_time(s: String) -> rusqlite::Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(&s)
        .map(|v| v.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn child_map(nodes: &[AtlasNode]) -> HashMap<Option<Uuid>, Vec<&AtlasNode>> {
    let mut m: HashMap<Option<Uuid>, Vec<&AtlasNode>> = HashMap::new();
    for n in nodes {
        m.entry(n.parent_id).or_default().push(n);
    }
    for v in m.values_mut() {
        v.sort_by_key(|n| (n.position, n.created_at, n.id));
    }
    m
}
fn flatten(nodes: &[AtlasNode], honor_collapsed: bool) -> Vec<(AtlasNode, usize)> {
    flatten_visible(nodes, honor_collapsed, true, &std::collections::HashSet::new())
}
fn flatten_visible(nodes: &[AtlasNode], honor_collapsed: bool, show_hidden: bool, expanded: &std::collections::HashSet<Uuid>) -> Vec<(AtlasNode, usize)> {
    fn walk(
        id: Option<Uuid>,
        d: usize,
        m: &HashMap<Option<Uuid>, Vec<&AtlasNode>>,
        out: &mut Vec<(AtlasNode, usize)>,
        honor: bool,
        show_hidden: bool,
        expanded: &std::collections::HashSet<Uuid>,
    ) {
        if let Some(children) = m.get(&id) {
            for n in children {
                if n.hidden && !show_hidden { continue; }
                let mut visible = (*n).clone();
                if expanded.contains(&n.id) { visible.collapsed = false; }
                out.push((visible, d));
                if !(honor && n.collapsed && !expanded.contains(&n.id)) {
                    walk(Some(n.id), d + 1, m, out, honor, show_hidden, expanded);
                }
            }
        }
    }
    let m = child_map(nodes);
    let mut out = Vec::new();
    walk(None, 0, &m, &mut out, honor_collapsed, show_hidden, expanded);
    out
}
fn descendant_ids(nodes: &[AtlasNode], root: Uuid) -> Vec<Uuid> {
    let m = child_map(nodes);
    fn walk(id: Uuid, m: &HashMap<Option<Uuid>, Vec<&AtlasNode>>, out: &mut Vec<Uuid>) {
        if let Some(c) = m.get(&Some(id)) {
            for n in c {
                out.push(n.id);
                walk(n.id, m, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &m, &mut out);
    out
}
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn render_html_children(out: &mut String, nodes: &[AtlasNode], parent: Option<Uuid>) {
    let m = child_map(nodes);
    fn walk(out: &mut String, id: Option<Uuid>, m: &HashMap<Option<Uuid>, Vec<&AtlasNode>>) {
        let Some(c) = m.get(&id) else { return };
        out.push_str("<ul>");
        for n in c {
            let class = if n.marked { " class=\"marked\"" } else { "" };
            let _ = write!(out, "<li{class}>{}", html_escape(&n.title));
            walk(out, Some(n.id), m);
            out.push_str("</li>");
        }
        out.push_str("</ul>");
    }
    walk(out, parent, &m)
}
fn slug(s: &str) -> String {
    let s = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "chart".into() } else { s }
}
fn pdf_text(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
fn simple_pdf(lines: &[(usize, String)]) -> Vec<u8> {
    const PER_PAGE: usize = 43;
    let page_count = lines.len().max(1).div_ceil(PER_PAGE);
    let font_id = 3 + page_count * 2;
    let kids = (0..page_count)
        .map(|i| format!("{} 0 R", 3 + i * 2))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>"),
    ];
    for page in 0..page_count {
        let content_id = 4 + page * 2;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 842] /Resources << /Font << /F1 {font_id} 0 R >> >> /Contents {content_id} 0 R >>"));
        let mut stream = String::from("BT /F1 11 Tf\n");
        for (row, (depth, value)) in lines
            .iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .enumerate()
        {
            let x = 42 + (*depth as i32 * 16).min(480);
            let y = 800 - row as i32 * 17;
            let _ = writeln!(stream, "1 0 0 1 {x} {y} Tm ({}) Tj", pdf_text(value));
        }
        stream.push_str("ET\n");
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}endstream",
            stream.len(),
            stream
        ));
    }
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_atlas_table_gains_color_and_hidden_columns() {
        let directory = std::env::temp_dir().join(format!("questline-atlas-upgrade-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("atlas.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE atlas_nodes (id TEXT PRIMARY KEY, chart_id TEXT NOT NULL, project_id TEXT NOT NULL, parent_id TEXT, title TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0, collapsed INTEGER NOT NULL DEFAULT 0, marked INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT '', owner_identity TEXT);").unwrap();
        drop(conn);
        let db = Database::new(&path).unwrap();
        for column in ["hidden", "color"] {
            let count: i64 = db.conn.query_row("SELECT count(*) FROM pragma_table_info('atlas_nodes') WHERE name=?1", [column], |row| row.get(0)).unwrap();
            assert_eq!(count, 1);
        }
        drop(db);
        let _ = std::fs::remove_dir_all(directory);
    }
    #[test]
    fn atlas_tree_and_exports_round_trip() {
        let path = std::env::temp_dir().join(format!("questline-atlas-{}.db", Uuid::new_v4()));
        let db = Database::new(&path).unwrap();
        let now = Utc::now();
        let p = crate::models::Project {
            id: Uuid::new_v4(),
            name: "Atlas test".into(),
            description: None,
            created_at: now,
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        };
        db.insert_project(&p).unwrap();
        let svc = AtlasService::new(&db);
        let chart = svc.create_chart(p.id, "Launch Plan", Some("hero")).unwrap();
        let root = svc.nodes(chart.id).unwrap()[0].clone();
        let child = svc
            .add_node(
                chart.id,
                Some(root.id),
                "Research & discovery",
                Some("hero"),
            )
            .unwrap();
        svc.add_node(chart.id, Some(child.id), "Interview users", Some("hero"))
            .unwrap();
        svc.color_branch(root.id, Some(15)).unwrap();
        assert!(svc.nodes(chart.id).unwrap().iter().all(|node| node.color == Some(15)));
        let mut child = svc.node(child.id).unwrap().unwrap();
        child.hidden = true;
        svc.update_node(&child).unwrap();
        assert_eq!(svc.visible_nodes(chart.id).unwrap().len(), 1);
        assert_eq!(svc.visible_nodes_with_hidden(chart.id, true).unwrap().len(), 3);
        svc.color_branch(root.id, Some(2)).unwrap();
        assert!(svc.nodes(chart.id).unwrap().iter().all(|node| node.color == Some(2)));
        let text = svc.export_text(chart.id).unwrap();
        assert!(text.contains("\tResearch & discovery"));
        assert!(
            svc.export_markdown(chart.id)
                .unwrap()
                .contains("  - Research & discovery")
        );
        assert!(
            svc.export_html(chart.id)
                .unwrap()
                .contains("Research &amp; discovery")
        );
        assert!(svc.export_pdf(chart.id).unwrap().starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn subtree_deletion_keeps_syncable_tombstone_snapshots() {
        let path = std::env::temp_dir().join(format!("questline-atlas-{}.db", Uuid::new_v4()));
        let db = Database::new(&path).unwrap();
        let now = Utc::now();
        let project = crate::models::Project {
            id: Uuid::new_v4(),
            name: "Delete test".into(),
            description: None,
            created_at: now,
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        };
        db.insert_project(&project).unwrap();
        let atlas = AtlasService::new(&db);
        let chart = atlas.create_chart(project.id, "Paths", None).unwrap();
        let root = atlas.nodes(chart.id).unwrap()[0].clone();
        let child = atlas
            .add_node(chart.id, Some(root.id), "Child", None)
            .unwrap();
        let grandchild = atlas
            .add_node(chart.id, Some(child.id), "Grandchild", None)
            .unwrap();
        atlas.delete_node(child.id).unwrap();
        assert_eq!(atlas.nodes(chart.id).unwrap().len(), 1);
        for id in [child.id, grandchild.id] {
            let snapshot: String = db.conn.query_row(
                "SELECT content FROM revisions WHERE entity_type='atlas_node' AND entity_id=?1 ORDER BY revision_number DESC LIMIT 1",
                params![id.to_string()], |row| row.get(0),
            ).unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&snapshot).unwrap()["project_id"],
                project.id.to_string()
            );
        }
    }

    #[test]
    fn pdf_export_paginates_large_charts() {
        let lines = (0..90)
            .map(|i| (i % 4, format!("Branch {i}")))
            .collect::<Vec<_>>();
        let pdf = String::from_utf8_lossy(&simple_pdf(&lines)).into_owned();
        assert!(pdf.contains("/Count 3"));
        assert!(pdf.contains("Branch 89"));
    }
}
