//! Account state that previously bypassed the incremental outbox.
use super::Database;
use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension};

pub const RECORDS: &[(&str, &str, &[&str])] = &[
    ("favorite_category", "favorite_categories", &["id"]),
    ("favorite", "favorites", &["id"]),
    ("favorite_visit", "favorite_visits", &["favorite_id", "device_id"]),
    ("reflection", "reflections", &["created_date"]),
    ("trait", "traits", &["id"]),
    (
        "class_quest",
        "class_quests",
        &["class_name", "unlock_level"],
    ),
    ("legendary_title", "legendary_titles", &["title_id"]),
    ("relic", "relics", &["id"]),
    ("companion_lore", "companion_lore", &["id"]),
    ("great_chronicle", "great_chronicle", &["id"]),
    ("notification", "notifications", &["id"]),
    ("activity_log", "activity_log", &["id"]),
    (
        "message_reaction",
        "message_reactions",
        &["message_id", "user_identity", "emoji"],
    ),
];

pub fn definition(kind: &str) -> Option<(&'static str, &'static [&'static str])> {
    RECORDS
        .iter()
        .find(|(entity, _, _)| *entity == kind)
        .map(|(_, table, keys)| (*table, *keys))
}

fn id_expression(keys: &[&str], prefix: &str) -> String {
    keys.iter()
        .map(|key| format!("CAST({prefix}{key} AS TEXT)"))
        .collect::<Vec<_>>()
        .join(" || '__' || ")
}

pub fn install(conn: &Connection) -> Result<()> {
    // TEMP triggers and context belong to this connection, so one sync worker
    // cannot suppress change capture in the interactive/UI connection.
    conn.execute_batch("CREATE TEMP TABLE IF NOT EXISTS sync_context (enabled INTEGER NOT NULL); INSERT INTO sync_context SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM sync_context);")?;
    for (kind, table, keys) in RECORDS {
        for (action, prefix, operation) in [
            ("INSERT", "NEW.", "upsert"),
            ("UPDATE", "NEW.", "upsert"),
            ("DELETE", "OLD.", "delete"),
        ] {
            let id = id_expression(keys, prefix);
            let meaningful = match (*kind, action) {
                ("class_quest", "UPDATE") => {
                    " AND (NEW.status != OLD.status OR NEW.progress != OLD.progress)"
                }
                ("class_quest", "INSERT") => " AND (NEW.status != 'Locked' OR NEW.progress > 0)",
                ("legendary_title", "UPDATE") => {
                    " AND (NEW.unlocked != OLD.unlocked OR NEW.equipped != OLD.equipped)"
                }
                ("legendary_title" | "relic", "INSERT") => " AND NEW.unlocked != 0",
                ("relic", "UPDATE") => " AND NEW.unlocked != OLD.unlocked",
                _ => "",
            };
            conn.execute_batch(&format!("CREATE TEMP TRIGGER IF NOT EXISTS sync_capture_{kind}_{action}
                AFTER {action} ON main.{table} WHEN (SELECT enabled FROM sync_context)=1 {meaningful}
                BEGIN
                  INSERT INTO sync_log (id,entity_type,entity_id,operation,timestamp,synced)
                  VALUES (lower(hex(randomblob(16))),'{kind}',{id},'{operation}',strftime('%Y-%m-%dT%H:%M:%fZ','now'),0);
                  INSERT INTO sync_entity_versions (entity_type,entity_id,timestamp,event_id,deleted)
                  SELECT entity_type,entity_id,timestamp,id,{} FROM sync_log ORDER BY rowid DESC LIMIT 1
                  ON CONFLICT(entity_type,entity_id) DO UPDATE SET timestamp=excluded.timestamp,event_id=excluded.event_id,deleted=excluded.deleted;
                END;", i32::from(operation == "delete")))?;
        }
    }
    // Preferences are captured even when an action writes SQL directly.
    let allow = super::synced_setting_keys()
        .map(|key| format!("'{key}'"))
        .collect::<Vec<_>>()
        .join(",");
    let prefixes = super::SYNCED_PROGRESS_PREFIXES
        .iter()
        .map(|prefix| format!("NEW.key GLOB '{prefix}*'"))
        .collect::<Vec<_>>()
        .join(" OR ");
    for action in ["INSERT", "UPDATE"] {
        conn.execute_batch(&format!("CREATE TEMP TRIGGER IF NOT EXISTS sync_capture_setting_{action}
          AFTER {action} ON main.settings WHEN (SELECT enabled FROM sync_context)=1 AND (NEW.key IN ({allow}) OR {prefixes})
          BEGIN
            INSERT INTO sync_log (id,entity_type,entity_id,operation,timestamp,synced)
            VALUES (lower(hex(randomblob(16))),'setting',NEW.key,'upsert',strftime('%Y-%m-%dT%H:%M:%fZ','now'),0);
            INSERT INTO sync_entity_versions (entity_type,entity_id,timestamp,event_id,deleted)
            SELECT entity_type,entity_id,timestamp,id,0 FROM sync_log ORDER BY rowid DESC LIMIT 1
            ON CONFLICT(entity_type,entity_id) DO UPDATE SET timestamp=excluded.timestamp,event_id=excluded.event_id,deleted=0;
          END;"))?;
    }
    Ok(())
}

impl Database {
    pub fn record_sync_payload(&self, kind: &str, id: &str) -> Result<Option<String>> {
        let Some((table, keys)) = definition(kind) else {
            return Ok(None);
        };
        let columns = self
            .conn
            .prepare(&format!("PRAGMA table_info({table})"))?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let object = columns
            .iter()
            .map(|key| format!("'{key}',{key}"))
            .collect::<Vec<_>>()
            .join(",");
        let json: Option<String> = self
            .conn
            .query_row(
                &format!(
                    "SELECT json_object({object}) FROM {table} WHERE {}=?1",
                    id_expression(keys, "")
                ),
                [id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(json)
    }

    pub fn apply_sync_record(
        &self,
        kind: &str,
        id: &str,
        operation: &str,
        payload: Option<&str>,
    ) -> Result<()> {
        let (table, keys) =
            definition(kind).ok_or_else(|| anyhow::anyhow!("Unsupported entity {kind}"))?;
        if operation == "delete" {
            self.conn.execute(
                &format!("DELETE FROM {table} WHERE {}=?1", id_expression(keys, "")),
                [id],
            )?;
            return Ok(());
        }
        let value: serde_json::Value = serde_json::from_str(payload.unwrap_or("null"))?;
        crate::portals::validate_record(kind, &value)?;
        let object = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid {kind} payload"))?;
        let expected = keys
            .iter()
            .map(|key| match &value[*key] {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join("__");
        if expected != id {
            bail!("{kind} payload identity does not match envelope");
        }
        let existing = self
            .record_sync_payload(kind, id)?
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok());
        let mut value = value.clone();
        if let Some(local) = existing {
            for field in match kind {
                "favorite_visit" => &["count"][..],
                "class_quest" => &["progress"][..],
                "relic" | "legendary_title" => &["unlocked"][..],
                _ => &[][..],
            } {
                value[*field] = serde_json::json!(
                    local[*field]
                        .as_i64()
                        .unwrap_or(0)
                        .max(value[*field].as_i64().unwrap_or(0))
                );
            }
            if kind == "class_quest" {
                let rank = |v: &str| match v {
                    "Completed" => 3,
                    "Active" => 2,
                    "Available" => 1,
                    _ => 0,
                };
                if rank(local["status"].as_str().unwrap_or(""))
                    > rank(value["status"].as_str().unwrap_or(""))
                {
                    value["status"] = local["status"].clone();
                }
            }
        }
        let allowed = self
            .conn
            .prepare(&format!("PRAGMA table_info({table})"))?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let columns = object
            .keys()
            .filter(|key| allowed.contains(key))
            .cloned()
            .collect::<Vec<_>>();
        let params = columns
            .iter()
            .map(|key| match &value[key] {
                serde_json::Value::Null => rusqlite::types::Value::Null,
                serde_json::Value::Bool(v) => rusqlite::types::Value::Integer(i64::from(*v)),
                serde_json::Value::Number(v) => {
                    rusqlite::types::Value::Integer(v.as_i64().unwrap_or(0))
                }
                serde_json::Value::String(v) => rusqlite::types::Value::Text(v.clone()),
                v => rusqlite::types::Value::Text(v.to_string()),
            })
            .collect::<Vec<_>>();
        let update = columns
            .iter()
            .filter(|key| !keys.contains(&key.as_str()))
            .map(|key| format!("{key}=excluded.{key}"))
            .collect::<Vec<_>>()
            .join(",");
        let conflict = if update.is_empty() {
            "DO NOTHING".into()
        } else {
            format!("DO UPDATE SET {update}")
        };
        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT({}) {conflict}",
            columns.join(","),
            vec!["?"; columns.len()].join(","),
            keys.join(",")
        );
        self.conn
            .execute(&sql, rusqlite::params_from_iter(params))?;
        Ok(())
    }

    pub fn queue_record_sync(&self) -> Result<usize> {
        let mut count = 0;
        for (kind, table, keys) in RECORDS {
            let meaningful = match *kind {
                "class_quest" => " WHERE status!='Locked' OR progress>0",
                "relic" => " WHERE unlocked!=0",
                "legendary_title" => " WHERE unlocked!=0 OR equipped!=0",
                _ => "",
            };
            let mut stmt = self.conn.prepare(&format!(
                "SELECT {} FROM {table}{meaningful}",
                id_expression(keys, "")
            ))?;
            for id in stmt.query_map([], |r| r.get::<_, String>(0))? {
                self.queue_existing_state(kind, &id?, "upsert")?;
                count += 1;
            }
        }
        Ok(count)
    }
}
