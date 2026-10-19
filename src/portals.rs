use crate::database::Database;
use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const UNCATEGORIZED: &str = "uncategorized";
const MAX_COUNT: i64 = 9_007_199_254_740_991;

fn text<'a>(value: &'a Value, key: &str, limit: usize) -> Result<&'a str> {
    let s = value[key]
        .as_str()
        .with_context(|| format!("Missing {key}"))?;
    if s.trim().is_empty() || s.chars().count() > limit || s.chars().any(char::is_control) {
        bail!("Invalid {key}");
    }
    Ok(s)
}

fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        bail!("Invalid portal identifier");
    }
    Ok(())
}

pub fn normalize_url(value: &str) -> Result<String> {
    if value.len() > 4096
        || value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        bail!("Use an HTTP or HTTPS URL without spaces or embedded credentials");
    }
    let parsed = url::Url::parse(value)?;
    if !value.contains("://")
        || !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        bail!("Use an HTTP or HTTPS URL without embedded credentials");
    }
    Ok(parsed.to_string())
}

pub fn validate_record(kind: &str, value: &Value) -> Result<()> {
    match kind {
        "favorite_category" => {
            let id = text(value, "id", 100)?;
            identifier(id)?;
            let name = text(value, "name", 80)?;
            if id == "top" || id == UNCATEGORIZED || name.eq_ignore_ascii_case("top") {
                bail!("Reserved category");
            }
        }
        "favorite" => {
            identifier(text(value, "id", 100)?)?;
            text(value, "title", 200)?;
            normalize_url(text(value, "url", 4096)?)?;
            identifier(text(value, "category", 100)?)?;
        }
        "favorite_visit" => {
            identifier(text(value, "favorite_id", 100)?)?;
            identifier(text(value, "device_id", 100)?)?;
            let count = value["count"].as_i64().context("Invalid visit count")?;
            if !(0..=MAX_COUNT).contains(&count) {
                bail!("Invalid visit count");
            }
        }
        _ => {}
    }
    Ok(())
}

fn stable_id(kind: &str, value: &str) -> String {
    format!("{kind}{:x}", Sha256::digest(value.as_bytes()))
}

fn items<'a>(state: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    let rows = state[key]
        .as_array()
        .with_context(|| format!("Invalid {key}"))?;
    if rows.len() > if key == "categories" { 300 } else { 10000 } {
        bail!("Too many {key}");
    }
    let mut ids = std::collections::HashSet::new();
    for row in rows {
        identifier(text(row, "id", 100)?)?;
        if !ids.insert(row["id"].as_str()) {
            bail!("Duplicate identifier");
        }
    }
    Ok(rows)
}

pub fn snapshot(db: &Database) -> Result<Value> {
    let read = db.savepoint("portal_snapshot")?;
    let user = db
        .get_user()?
        .context("Open Questline and complete onboarding first")?;
    let mut categories = vec![json!({"id":UNCATEGORIZED,"name":"Uncategorized"})];
    let mut stmt = db
        .conn
        .prepare("SELECT id,name FROM favorite_categories ORDER BY name COLLATE NOCASE,id")?;
    categories.extend(
        stmt.query_map([], |r| {
            Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?}))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    let mut stmt = db.conn.prepare("SELECT f.id,f.title,f.url,f.category,COALESCE((SELECT SUM(count) FROM favorite_visits v WHERE v.favorite_id=f.id),0) FROM favorites f ORDER BY f.title COLLATE NOCASE,f.id")?;
    let favorites = stmt.query_map([], |r| Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"url":r.get::<_,String>(2)?,"category":r.get::<_,String>(3)?,"clickCount":r.get::<_,i64>(4)?.min(MAX_COUNT)})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut versions = serde_json::Map::new();
    let mut stmt=db.conn.prepare("SELECT entity_type,entity_id,timestamp,event_id FROM sync_entity_versions WHERE entity_type IN ('favorite','favorite_category')")?;
    for entry in stmt.query_map([], |r| {
        Ok((
            format!("{}:{}", r.get::<_, String>(0)?, r.get::<_, String>(1)?),
            format!("{}:{}", r.get::<_, String>(2)?, r.get::<_, String>(3)?),
        ))
    })? {
        let (k, v) = entry?;
        versions.insert(k, json!(v));
    }
    read.commit()?;
    Ok(
        json!({"questlineAccount":user.id.to_string(),"categories":categories,"favorites":favorites,"_versions":versions}),
    )
}

fn ensure_unchanged(current: &Value, before: &Value, kind: &str, id: &str) -> Result<()> {
    let key = format!("{kind}:{id}");
    if current["_versions"][&key] != before["_versions"][&key] {
        bail!("This portal changed on another device. Refresh and retry your edit.");
    }
    Ok(())
}

fn upsert(db: &Database, kind: &str, row: &Value) -> Result<()> {
    validate_record(kind, row)?;
    let id = if kind == "favorite_visit" {
        format!(
            "{}__{}",
            row["favorite_id"].as_str().unwrap(),
            row["device_id"].as_str().unwrap()
        )
    } else {
        row["id"].as_str().unwrap().to_string()
    };
    db.apply_sync_record(kind, &id, "upsert", Some(&row.to_string()))
}

fn set_minimum_count(db: &Database, id: &str, count: i64) -> Result<()> {
    let others:i64=db.conn.query_row("SELECT COALESCE(SUM(count),0) FROM favorite_visits WHERE favorite_id=? AND device_id!='import'",[id],|r|r.get(0))?;
    let old: i64 = db
        .conn
        .query_row(
            "SELECT count FROM favorite_visits WHERE favorite_id=? AND device_id='import'",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0);
    let desired = count.saturating_sub(others).max(0).min(MAX_COUNT);
    if desired > old {
        upsert(
            db,
            "favorite_visit",
            &json!({"favorite_id":id,"device_id":"import","count":desired}),
        )?;
    }
    Ok(())
}

fn migrate(db: &Database, legacy: &Value) -> Result<()> {
    if db.get_setting("spectrum_favorites_import_v1")?.is_some() {
        return Ok(());
    }
    if !legacy["questlineAccount"].is_null() {
        return Ok(());
    }
    let current = snapshot(db)?;
    let mut category_ids = std::collections::HashMap::new();
    for category in items(legacy, "categories")? {
        let name = text(category, "name", 80)?;
        let old = text(category, "id", 100)?;
        let id = if old == UNCATEGORIZED || name.eq_ignore_ascii_case("Uncategorized") {
            UNCATEGORIZED.to_string()
        } else if let Some(existing) = current["categories"].as_array().unwrap().iter().find(|c| {
            c["name"]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case(name))
        }) {
            existing["id"].as_str().unwrap().into()
        } else {
            let id = stable_id("c", &name.to_lowercase());
            let known:Option<bool>=db.conn.query_row("SELECT deleted FROM sync_entity_versions WHERE entity_type='favorite_category' AND entity_id=?",[&id],|r|r.get(0)).optional()?;
            if known == Some(true) {
                UNCATEGORIZED.into()
            } else {
                if known.is_none() {
                    upsert(db, "favorite_category", &json!({"id":id,"name":name}))?;
                }
                id
            }
        };
        category_ids.insert(old.to_string(), id);
    }
    for favorite in items(legacy, "favorites")? {
        let url = normalize_url(text(favorite, "url", 4096)?)?;
        let existing: Option<String> = db
            .conn
            .query_row("SELECT id FROM favorites WHERE url=?", [&url], |r| r.get(0))
            .optional()?;
        let id = existing.clone().unwrap_or_else(|| stable_id("p", &url));
        // Una migración tardía no revive vínculos borrados en otro equipo.
        let deleted:bool=db.conn.query_row("SELECT EXISTS(SELECT 1 FROM sync_entity_versions WHERE entity_type='favorite' AND entity_id=? AND deleted=1)",[&id],|r|r.get(0))?;
        if deleted {
            continue;
        }
        let known: bool = db.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE id=?)",
            [&id],
            |r| r.get(0),
        )?;
        if existing.is_none() && !known {
            upsert(
                db,
                "favorite",
                &json!({"id":id,"title":text(favorite,"title",200)?,"url":url,"category":category_ids.get(favorite["category"].as_str().unwrap_or(UNCATEGORIZED)).map(String::as_str).unwrap_or(UNCATEGORIZED)}),
            )?;
        }
        set_minimum_count(
            db,
            &id,
            favorite["clickCount"]
                .as_i64()
                .unwrap_or(0)
                .clamp(0, MAX_COUNT),
        )?;
    }
    db.set_setting("spectrum_favorites_import_v1", "1")?;
    Ok(())
}

pub fn exchange(db: &Database, request: &Value) -> Result<Value> {
    let transaction = db.savepoint("portal_exchange")?;
    let mut current = snapshot(db)?;
    let mut deleted_categories = Vec::new();
    match request["action"].as_str().unwrap_or("snapshot") {
        "snapshot" => {
            if request["legacy"].is_object() {
                migrate(db, &request["legacy"])?;
            }
        }
        "apply" => {
            let before = &request["before"];
            let after = &request["after"];
            if request["account"] != current["questlineAccount"]
                || before["questlineAccount"] != current["questlineAccount"]
            {
                bail!("Questline profile changed. Refresh Favorites before editing.");
            }
            let device = match db.get_setting("device_id")? {
                Some(id) => id,
                None => {
                    let id = uuid::Uuid::new_v4().to_string();
                    db.set_setting("device_id", &id)?;
                    id
                }
            };
            for (key, kind) in [
                ("categories", "favorite_category"),
                ("favorites", "favorite"),
            ] {
                let old = items(before, key)?;
                let new = items(after, key)?;
                for row in new {
                    let id = text(row, "id", 100)?;
                    if kind == "favorite_category" && id == UNCATEGORIZED {
                        continue;
                    }
                    let mut row = row.clone();
                    if kind == "favorite" {
                        row["url"] = json!(normalize_url(text(&row, "url", 4096)?)?);
                        row.as_object_mut().unwrap().remove("clickCount");
                        row.as_object_mut().unwrap().retain(|k, _| {
                            ["id", "title", "url", "category"].contains(&k.as_str())
                        });
                    }
                    let previous = old.iter().find(|v| v["id"] == id);
                    let changed = previous.is_none_or(|v| {
                        if kind == "favorite" {
                            ["title", "url", "category"].iter().any(|k| v[k] != row[k])
                        } else {
                            v["name"] != row["name"]
                        }
                    });
                    if changed {
                        ensure_unchanged(&current, before, kind, id)?;
                        if kind == "favorite_category" {
                            let name = text(&row, "name", 80)?.to_lowercase();
                            if current["categories"].as_array().unwrap().iter().any(|c| {
                                c["id"] != id
                                    && c["name"].as_str().is_some_and(|v| v.to_lowercase() == name)
                            }) {
                                bail!("A category with this name already exists");
                            }
                        }
                        if kind == "favorite" {
                            let category = row["category"].as_str().context("Missing category")?;
                            if category != UNCATEGORIZED
                                && !db.conn.query_row(
                                    "SELECT EXISTS(SELECT 1 FROM favorite_categories WHERE id=?)",
                                    [category],
                                    |r| r.get::<_, bool>(0),
                                )?
                            {
                                bail!("This category no longer exists. Refresh and retry.");
                            }
                            let duplicate: bool = db.conn.query_row(
                                "SELECT EXISTS(SELECT 1 FROM favorites WHERE url=? AND id!=?)",
                                params![row["url"].as_str(), id],
                                |r| r.get(0),
                            )?;
                            if duplicate {
                                bail!("This URL is already in Portals");
                            }
                        }
                        upsert(db, kind, &row)?;
                    }
                    if kind == "favorite" {
                        let new_count = new.iter().find(|v| v["id"] == id).unwrap()["clickCount"]
                            .as_i64()
                            .unwrap_or(0)
                            .clamp(0, MAX_COUNT);
                        let old_count =
                            previous.and_then(|v| v["clickCount"].as_i64()).unwrap_or(0);
                        if new_count > old_count {
                            if request["countMode"] == "visit" {
                                // Cada equipo publica un contador creciente; los reintentos nunca suman dos veces.
                                let visits:i64=db.conn.query_row("SELECT count FROM favorite_visits WHERE favorite_id=? AND device_id=?",params![id,device],|r|r.get(0)).optional()?.unwrap_or(0);
                                if !current["favorites"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .any(|v| v["id"] == id)
                                {
                                    bail!("This favorite was deleted. Refresh before opening it.");
                                }
                                upsert(
                                    db,
                                    "favorite_visit",
                                    &json!({"favorite_id":id,"device_id":device,"count":visits.saturating_add(new_count-old_count).min(MAX_COUNT)}),
                                )?;
                            } else {
                                set_minimum_count(db, id, new_count)?;
                            }
                        }
                    }
                }
                for row in old
                    .iter()
                    .filter(|v| !new.iter().any(|n| n["id"] == v["id"]))
                {
                    let id = text(row, "id", 100)?;
                    if id == UNCATEGORIZED && kind == "favorite_category" {
                        bail!("Uncategorized cannot be deleted");
                    }
                    ensure_unchanged(&current, before, kind, id)?;
                    if kind == "favorite_category" {
                        deleted_categories.push(id.to_string());
                        continue;
                    }
                    db.apply_sync_record(kind, id, "delete", None)?;
                }
                current = snapshot(db)?;
            }
        }
        _ => bail!("Unknown Portals action"),
    }
    for id in deleted_categories {
        let nonempty: bool = db.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE category=?)",
            [&id],
            |r| r.get(0),
        )?;
        if nonempty {
            bail!("Move or delete all favorites before deleting this category");
        }
        db.apply_sync_record("favorite_category", &id, "delete", None)?;
    }
    let result = snapshot(db)?;
    transaction.commit()?;
    Ok(result)
}

pub fn run_json(path: &std::path::Path, request: &Value) -> Result<Value> {
    if !path.is_file() {
        bail!("Open Questline and complete onboarding first");
    }
    exchange(&Database::new(path)?, request)
}

pub fn open_url(value: &str) -> Result<()> {
    let url = normalize_url(value)?;
    #[cfg(target_os = "linux")]
    let mut opener = std::process::Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut opener = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut opener = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    opener
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn cli(path: &std::path::Path, args: &[String]) -> Result<()> {
    if args
        .first()
        .is_some_and(|v| matches!(v.as_str(), "help" | "--help" | "-h"))
    {
        println!(
            "Portals — synced bookmarks\n\nquestline portals list [category]\nquestline portals add <title> <url> [category]\nquestline portals edit <id> <title> <url> [category]\nquestline portals delete <id>\nquestline portals category <name>\nquestline portals rename <category> <name>\nquestline portals remove-category <category>\nquestline portals open <id>\n\nQuote titles and category names containing spaces. Changes save locally and queue for encrypted sync. Enable Auto Sync; run questline sync-background when the TUI is closed."
        );
        return Ok(());
    }
    if !path.is_file() {
        bail!("Open Questline and complete onboarding first");
    }
    let db = Database::new(path)?;
    let before = snapshot(&db)?;
    let mut after = before.clone();
    let category_id = |selector: &str| -> Result<String> {
        let matches = before["categories"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| {
                c["id"] == selector
                    || c["name"]
                        .as_str()
                        .is_some_and(|v| v.eq_ignore_ascii_case(selector))
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            bail!("Choose a unique category name or ID");
        }
        Ok(matches[0]["id"].as_str().unwrap().into())
    };
    let command = args.first().map(String::as_str).unwrap_or("list");
    let arg = |index: usize| {
        args.get(index)
            .map(String::as_str)
            .context("Missing argument; run questline portals help")
    };
    let mut visit = false;
    match command {
        "list" if args.len() <= 2 => {
            let category = args.get(1).map(|v| category_id(v)).transpose()?;
            println!("Portals");
            for c in before["categories"].as_array().unwrap() {
                if category.as_ref().is_some_and(|v| c["id"] != v.as_str()) {
                    continue;
                }
                println!(
                    "\n{} [{}]",
                    c["name"].as_str().unwrap(),
                    c["id"].as_str().unwrap()
                );
                for f in before["favorites"].as_array().unwrap().iter().filter(|f| {
                    f["category"] == c["id"]
                        || (c["id"] == UNCATEGORIZED
                            && !before["categories"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|c| c["id"] == f["category"]))
                }) {
                    println!(
                        "  {} · {} opens\n    {}\n    id: {}",
                        f["title"].as_str().unwrap(),
                        f["clickCount"],
                        f["url"].as_str().unwrap(),
                        f["id"].as_str().unwrap()
                    );
                }
            }
            return Ok(());
        }
        "add" if (3..=4).contains(&args.len()) => {
            let category = args
                .get(3)
                .map(|v| category_id(v))
                .transpose()?
                .unwrap_or(UNCATEGORIZED.into());
            after["favorites"].as_array_mut().unwrap().push(json!({"id":uuid::Uuid::new_v4().to_string(),"title":arg(1)?,"url":arg(2)?,"category":category,"clickCount":0}));
        }
        "edit" if (4..=5).contains(&args.len()) => {
            let id = arg(1)?;
            let row = after["favorites"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|v| v["id"] == id)
                .context("Portal not found")?;
            row["title"] = json!(arg(2)?);
            row["url"] = json!(arg(3)?);
            if let Some(category) = args.get(4) {
                row["category"] = json!(category_id(category)?);
            }
        }
        "delete" if args.len() == 2 => {
            let id = arg(1)?;
            if !after["favorites"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["id"] == id)
            {
                bail!("Portal not found");
            }
            after["favorites"]
                .as_array_mut()
                .unwrap()
                .retain(|v| v["id"] != id);
        }
        "category" if args.len() == 2 => {
            let name = arg(1)?;
            if before["categories"].as_array().unwrap().iter().any(|v| {
                v["name"]
                    .as_str()
                    .is_some_and(|v| v.eq_ignore_ascii_case(name))
            }) {
                bail!("Category already exists");
            }
            after["categories"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":uuid::Uuid::new_v4().to_string(),"name":name}));
        }
        "rename" if args.len() == 3 => {
            let id = category_id(arg(1)?)?;
            if id == UNCATEGORIZED {
                bail!("Uncategorized cannot be renamed");
            }
            after["categories"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|v| v["id"] == id)
                .unwrap()["name"] = json!(arg(2)?);
        }
        "remove-category" if args.len() == 2 => {
            let id = category_id(arg(1)?)?;
            after["categories"]
                .as_array_mut()
                .unwrap()
                .retain(|v| v["id"] != id);
        }
        "open" if args.len() == 2 => {
            let f = after["favorites"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|v| v["id"] == arg(1).unwrap())
                .context("Portal not found")?;
            open_url(f["url"].as_str().context("Missing URL")?)?;
            f["clickCount"] = json!(
                f["clickCount"]
                    .as_i64()
                    .unwrap_or(0)
                    .saturating_add(1)
                    .min(MAX_COUNT)
            );
            visit = true;
        }
        _ => bail!("Invalid Portals command; run questline portals help"),
    }
    exchange(
        &db,
        &json!({"action":"apply","account":before["questlineAccount"],"before":before,"after":after,"countMode":if visit {"visit"} else {"minimum"}}),
    )?;
    println!("Portals saved locally; changes queued for sync.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> Database {
        let db = Database::new(std::path::Path::new(":memory:")).unwrap();
        db.insert_user(&crate::models::User {
            id: uuid::Uuid::new_v4(),
            username: "Portal explorer".into(),
            class: crate::models::ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            specialization: None,
            created_at: chrono::Utc::now(),
        })
        .unwrap();
        db.set_setting("device_id", "test-device").unwrap();
        db
    }
    fn legacy() -> Value {
        json!({"version":1,"categories":[{"id":"uncategorized","name":"Uncategorized"},{"id":"work","name":"Work"}],"favorites":[{"id":"old","title":"Private work portal","url":"https://EXAMPLE.com/path?q=1#two","category":"work","clickCount":7}],"view":{"draft":{"url":"https://"}}})
    }
    fn imported(db: &Database) -> Value {
        exchange(db, &json!({"legacy":legacy()})).unwrap()
    }
    fn apply(db: &Database, before: &Value, after: &Value) -> Result<Value> {
        exchange(
            db,
            &json!({"action":"apply","account":before["questlineAccount"],"before":before,"after":after}),
        )
    }
    #[test]
    fn migration_is_idempotent_preserves_count_and_excludes_view() {
        let db = database();
        let first = imported(&db);
        let second = imported(&db);
        assert_eq!(first, second);
        assert_eq!(first["favorites"][0]["clickCount"], 7);
        assert_eq!(
            first["favorites"][0]["url"],
            "https://example.com/path?q=1#two"
        );
        assert!(first.get("view").is_none());
        assert!(db.pending_sync_count().unwrap() > 0);
    }
    #[test]
    fn stale_widget_cannot_undo_remote_edit_or_deletion() {
        let db = database();
        let first = imported(&db);
        let mut next = first.clone();
        next["favorites"][0]["title"] = json!("Changed elsewhere");
        let latest = apply(&db, &first, &next).unwrap();
        let mut stale = first.clone();
        stale["favorites"][0]["url"] = json!("https://another.example/");
        assert!(apply(&db, &first, &stale).is_err());
        let mut deleted = latest.clone();
        deleted["favorites"] = json!([]);
        apply(&db, &latest, &deleted).unwrap();
        assert!(apply(&db, &first, &stale).is_err());
        assert!(imported(&db)["favorites"].as_array().unwrap().is_empty());
    }
    #[test]
    fn failed_batch_rolls_back_entities_and_outbox() {
        let db = database();
        let first = imported(&db);
        let queued = db.pending_sync_count().unwrap();
        let mut next = first.clone();
        next["categories"][1]["name"] = json!("Renamed");
        next["favorites"][0]["url"] = json!("javascript:alert(1)");
        assert!(apply(&db, &first, &next).is_err());
        assert_eq!(snapshot(&db).unwrap(), first);
        assert_eq!(db.pending_sync_count().unwrap(), queued);
    }
    #[test]
    fn unrelated_stale_rows_are_not_republished() {
        let db = database();
        let first = imported(&db);
        let mut remote = first.clone();
        remote["favorites"][0]["title"] = json!("Remote title");
        apply(&db, &first, &remote).unwrap();
        let mut local = first.clone();
        local["categories"][1]["name"] = json!("Office");
        let saved = apply(&db, &first, &local).unwrap();
        assert_eq!(saved["favorites"][0]["title"], "Remote title");
        assert_eq!(saved["categories"][1]["name"], "Office");
    }
    #[test]
    fn categories_with_links_cannot_be_deleted_but_batch_can_delete_both() {
        let db = database();
        let first = imported(&db);
        let mut next = first.clone();
        next["categories"].as_array_mut().unwrap().pop();
        assert!(apply(&db, &first, &next).is_err());
        next["favorites"] = json!([]);
        assert!(apply(&db, &first, &next).is_ok());
    }
    #[test]
    fn visit_components_merge_monotonically_and_imports_do_not_inflate() {
        let db = database();
        let first = imported(&db);
        let id = first["favorites"][0]["id"].as_str().unwrap();
        for (device, count) in [("pc-a", 2), ("pc-b", 3), ("pc-a", 1)] {
            upsert(
                &db,
                "favorite_visit",
                &json!({"favorite_id":id,"device_id":device,"count":count}),
            )
            .unwrap();
        }
        assert_eq!(snapshot(&db).unwrap()["favorites"][0]["clickCount"], 12);
        set_minimum_count(&db, id, 8).unwrap();
        assert_eq!(snapshot(&db).unwrap()["favorites"][0]["clickCount"], 12);
    }
    #[test]
    fn account_switch_does_not_import_the_previous_accounts_cache() {
        let db = database();
        let mut old = legacy();
        old["questlineAccount"] = json!("previous-account");
        assert!(
            exchange(&db, &json!({"legacy":old})).unwrap()["favorites"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let first = snapshot(&db).unwrap();
        assert!(exchange(&db,&json!({"action":"apply","account":"previous-account","before":first,"after":legacy()})).is_err());
    }
    #[test]
    fn late_legacy_migration_preserves_known_remote_edits_and_tombstones() {
        let db = database();
        let first = imported(&db);
        let id = first["favorites"][0]["id"].as_str().unwrap();
        let category = first["categories"][1]["id"].as_str().unwrap();
        upsert(&db,"favorite",&json!({"id":id,"title":"Edited remotely","url":"https://new.example/","category":category})).unwrap();
        upsert(
            &db,
            "favorite_category",
            &json!({"id":category,"name":"Renamed remotely"}),
        )
        .unwrap();
        db.conn
            .execute(
                "DELETE FROM settings WHERE key='spectrum_favorites_import_v1'",
                [],
            )
            .unwrap();
        let next = imported(&db);
        assert_eq!(next["favorites"].as_array().unwrap().len(), 1);
        assert_eq!(next["favorites"][0]["title"], "Edited remotely");
        assert_eq!(next["categories"][1]["name"], "Renamed remotely");
        db.apply_sync_record("favorite", id, "delete", None)
            .unwrap();
        db.apply_sync_record("favorite_category", category, "delete", None)
            .unwrap();
        db.conn
            .execute(
                "DELETE FROM settings WHERE key='spectrum_favorites_import_v1'",
                [],
            )
            .unwrap();
        let final_state = imported(&db);
        assert!(final_state["favorites"].as_array().unwrap().is_empty());
        assert_eq!(final_state["categories"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn unsafe_urls_are_rejected() {
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://u:p@example.com",
            "https://example.com:99999",
            "https://exa mple.com",
            "https://example.com\\bad",
            "https://",
            "https:example.com",
        ] {
            assert!(normalize_url(url).is_err(), "{url}");
        }
    }
    #[test]
    fn profile_json_export_import_preserves_portals_and_visit_components() {
        let db = database();
        let first = imported(&db);
        let json = db.export_to_json().unwrap();
        let restored = database();
        restored.import_from_json(&json).unwrap();
        let result = snapshot(&restored).unwrap();
        assert_eq!(result["favorites"], first["favorites"]);
        assert_eq!(result["categories"], first["categories"]);
    }
    #[test]
    fn background_sync_respects_disabled_and_restore_hold() {
        let db = database();
        let config = crate::services::config::Config::default();
        assert!(crate::background_sync::allowed(&db, &config).unwrap());
        db.set_setting("sync_restore_hold", "1").unwrap();
        assert!(!crate::background_sync::allowed(&db, &config).unwrap());
        db.set_setting("sync_restore_hold", "0").unwrap();
        db.set_setting("auto_sync", "false").unwrap();
        assert!(!crate::background_sync::allowed(&db, &config).unwrap());
    }
}
