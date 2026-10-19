use crate::{
    background_sync,
    database::Database,
    services::{Identity, config::Config, profile_backup, profile_transfer},
};
use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rusqlite::{Connection, MAIN_DB, OpenFlags};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

fn profile(db: &Database) -> Result<Value> {
    let user = db.get_user()?;
    Ok(
        json!({"account":user.as_ref().map(|u|u.id.to_string()),"name":user.as_ref().map(|u|u.username.clone()),"tasks":db.conn.query_row("SELECT COUNT(*) FROM tasks",[],|r|r.get::<_,i64>(0))?,"notes":db.conn.query_row("SELECT COUNT(*) FROM notes",[],|r|r.get::<_,i64>(0))?,"favorites":db.conn.query_row("SELECT COUNT(*) FROM favorites",[],|r|r.get::<_,i64>(0))?}),
    )
}

pub fn status(path: &Path) -> Result<Value> {
    let config = Config::load()?;
    if !path.is_file() {
        return Ok(
            json!({"available":false,"account":null,"cloud_enabled":config.sync_enabled,"auto_sync":config.auto_sync,"queued":0,"deferred":0,"last_success":null,"error":"","server":config.server_url}),
        );
    }
    let db = Database::new(path)?;
    let last_success = [
        db.get_setting("background_sync_at")?,
        db.get_setting("last_sync")?,
    ]
    .into_iter()
    .flatten()
    .filter_map(|v| chrono::DateTime::parse_from_rfc3339(&v).ok())
    .max()
    .map(|v| v.to_rfc3339());
    Ok(
        json!({"available":db.get_user()?.is_some(),"account":profile(&db)?["account"],"profile_key":format!("{}:{}",db.get_user()?.map(|u|u.id.to_string()).unwrap_or_default(),db.get_setting("shell_profile_epoch")?.unwrap_or_default()),"cloud_enabled":background_sync::cloud_enabled(&db,&config)?,"auto_sync":db.get_setting("auto_sync")?.map(|v|v=="true").unwrap_or(config.auto_sync),"queued":db.pending_sync_count()?,"deferred":db.conn.query_row("SELECT COUNT(*) FROM sync_deferred_events",[],|r|r.get::<_,i64>(0))?,"quarantined":db.conn.query_row("SELECT COUNT(*) FROM sync_quarantine",[],|r|r.get::<_,i64>(0))?,"last_success":last_success,"error":db.get_setting("background_sync_error")?.unwrap_or_default(),"recovery_hold":db.get_setting("sync_restore_hold")?.as_deref()==Some("1"),"server":config.server_url,"devices":db.get_devices()?.into_iter().map(|(id,name,_,last)|json!({"id":id,"name":name,"last_sync":last})).collect::<Vec<_>>()}),
    )
}

fn account_matches(db: &Database, request: &Value) -> Result<()> {
    if !request
        .as_object()
        .is_some_and(|v| v.contains_key("account"))
        || request["account"] != profile(db)?["account"]
    {
        bail!("The active profile changed. Refresh Sync and retry.");
    }
    Ok(())
}

fn path_field(request: &Value) -> Result<PathBuf> {
    let p = PathBuf::from(request["path"].as_str().context("Choose a file")?);
    if !p.is_absolute() {
        bail!("Choose an absolute file path");
    }
    Ok(p)
}

fn idle_profile(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let lock = options.open(
        path.parent()
            .context("Missing profile directory")?
            .join("desktop-tui.lock"),
    )?;
    lock.try_lock().map_err(|_|anyhow::anyhow!("Close the Questline terminal before restoring or pruning; retry if a reminder is being saved."))?;
    Ok(lock)
}

fn backup_path(path: &Path) -> Result<PathBuf> {
    let directory = path
        .parent()
        .context("Missing profile directory")?
        .join("backups");
    std::fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(directory.join(format!(
        "questline-{}-{}.db",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        Uuid::new_v4()
    )))
}

pub fn perform(path: &Path, request: &Value) -> Result<Value> {
    let action = request["action"].as_str().context("Missing action")?;
    if action == "status" {
        return Ok(json!({"ok":true,"status":status(path)?}));
    }
    if !path.is_file() && action != "set_cloud_sync" {
        bail!("Create or restore a Questline profile first");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = Database::new(path)?;
    account_matches(&db, request)?;
    match action {
        "set_auto_sync" | "set_cloud_sync" => {
            let enabled = request["enabled"]
                .as_bool()
                .context("Choose enabled or disabled")?;
            if action == "set_auto_sync"
                && enabled
                && db.get_setting("sync_restore_hold")?.as_deref() == Some("1")
            {
                bail!("This profile is on a recovery hold. Restore it before enabling Auto Sync.");
            }
            db.set_setting(
                if action == "set_auto_sync" {
                    "auto_sync"
                } else {
                    "cloud_sync_enabled"
                },
                if enabled { "true" } else { "false" },
            )?;
            Ok(json!({"ok":true,"message":"Sync preference saved","status":status(path)?}))
        }
        "sync_now" => {
            background_sync::sync_now(path)?;
            Ok(json!({"ok":true,"message":"Sync completed","status":status(path)?}))
        }
        "backup" | "export_profile" => {
            let _lock = db.lock_sync()?;
            let identity = Identity::load_for_database(&db, db.get_user()?.map(|u| u.id))?;
            identity.validate()?;
            db.set_setting("_profile_identity", &serde_json::to_string(&identity)?)?;
            let destination = if action == "export_profile" {
                path_field(request)?
            } else {
                backup_path(path)?
            };
            let reserved = [
                path.to_path_buf(),
                PathBuf::from(format!("{}-wal", path.display())),
                PathBuf::from(format!("{}-shm", path.display())),
                path.with_file_name("identity.key"),
                path.with_file_name("config.toml"),
            ];
            if reserved.iter().any(|p| {
                p == &destination
                    || (p.canonicalize().is_ok()
                        && p.canonicalize().ok() == destination.canonicalize().ok())
            }) {
                bail!("Choose a file other than the active database");
            }
            if action == "export_profile" {
                db.write_json_export(&destination)?;
            } else {
                db.backup_to(&destination)?;
            }
            drop(_lock);
            let config = Config::load()?;
            let mut result = json!({"ok":true,"path":destination,"message":if action=="backup" {"Local backup saved"} else {"Profile exported"},"cloud_saved":false});
            if background_sync::cloud_enabled(&db, &config)? {
                let device = db
                    .get_setting("device_id")?
                    .context("Open Questline once to register this PC")?;
                match profile_backup::upload(&db, &identity, &device, &config.server_url) {
                    Ok(()) => {
                        result["cloud_saved"] = json!(true);
                        result["message"] = json!("Local copy and encrypted cloud backup saved");
                        if action == "export_profile" {
                            let mut bytes = identity.user_uuid.as_bytes().to_vec();
                            for pair in identity.secret_key.as_bytes().chunks_exact(2) {
                                bytes.push(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?);
                            }
                            result["transfer_code"] = json!(STANDARD.encode(bytes));
                        }
                    }
                    Err(error) => {
                        result["warning"] = json!(format!(
                            "Local file is safe. Cloud backup failed: {error}. Use the exported file for recovery until a cloud backup succeeds."
                        ))
                    }
                }
            }
            result["status"] = status(path)?;
            Ok(result)
        }
        _ => bail!("Unknown Sync action"),
    }
}

fn prepare_file(active: &Database, file: &Path) -> Result<profile_transfer::PreparedRestore> {
    if file.canonicalize()? == active.path()?.canonicalize()? {
        bail!("Choose a saved backup, not the active database");
    }
    if std::fs::metadata(file)?.len() > 512 * 1024 * 1024 {
        bail!("Backup exceeds the 512 MiB restore limit");
    }
    let mut header = [0u8; 16];
    {
        use std::io::Read;
        let mut f = File::open(file)?;
        let _ = f.read(&mut header)?;
    }
    if &header == b"SQLite format 3\0" {
        let staged = active
            .path()?
            .with_file_name(format!(".backup-preview-{}.db", Uuid::new_v4()));
        let mut private = OpenOptions::new();
        private.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            private.mode(0o600);
        }
        drop(private.open(&staged)?);
        let result = (|| -> Result<profile_transfer::PreparedRestore> {
            let source = Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            source.backup(MAIN_DB, &staged, None)?;
            if !Database::verify_db_backup(&staged)? {
                bail!("Backup integrity or foreign-key validation failed");
            }
            let copied = Database::new(&staged)?;
            profile_transfer::prepare_json(active, &copied.export_to_json()?)
        })();
        for p in [
            staged.clone(),
            PathBuf::from(format!("{}-wal", staged.display())),
            PathBuf::from(format!("{}-shm", staged.display())),
        ] {
            let _ = std::fs::remove_file(p);
        }
        result
    } else {
        profile_transfer::prepare_json(active, &std::fs::read_to_string(file)?)
    }
}

fn emit(value: &Value) -> Result<()> {
    println!("{value}");
    std::io::stdout().flush()?;
    Ok(())
}
fn read_request(input: &mut impl BufRead) -> Result<Value> {
    let mut raw = Vec::new();
    {
        use std::io::Read;
        input.take(64 * 1024 + 1).read_until(b'\n', &mut raw)?;
    }
    if raw.is_empty() {
        bail!("Sync request canceled");
    }
    if raw.len() > 64 * 1024 {
        bail!("Sync request exceeds 64 KiB");
    }
    Ok(serde_json::from_slice(&raw)?)
}

fn interactive(path: &Path, request: &Value, input: &mut impl BufRead) -> Result<Value> {
    let action = request["action"].as_str().context("Missing action")?;
    if !matches!(
        action,
        "restore_identity" | "restore_backup" | "restore_cloud" | "prune_tasks"
    ) {
        return perform(path, request);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut active = Database::new(path)?;
    account_matches(&active, request)?;
    let _idle = idle_profile(path)?;
    if action == "prune_tasks" {
        let _lock = active.lock_sync()?;
        let days = request["days"].as_i64().unwrap_or(30);
        let ids = active.prunable_task_ids(days)?;
        let preview = json!({"ok":true,"phase":"preview","token":Uuid::new_v4().to_string(),"kind":action,"count":ids.len(),"days":days,"message":"Remove old completed private quests and steps. Recent or unfinished branches and shared campaigns are preserved. A local backup will be saved before pruning."});
        emit(&preview)?;
        let decision = read_request(input)?;
        if decision["confirm"] != preview["token"] {
            return Ok(json!({"ok":true,"cancelled":true,"message":"Pruning canceled"}));
        }
        account_matches(&active, request)?;
        let backup = backup_path(path)?;
        active.backup_to(&backup)?;
        active.conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> Result<usize> {
            if active.prunable_task_ids(days)? != ids {
                bail!("Tasks changed during the preview. Review the new preview before pruning.");
            }
            active.prune_completed_tasks(days)
        })();
        match result {
            Ok(count) => {
                active.conn.execute_batch("COMMIT")?;
                Ok(
                    json!({"ok":true,"message":format!("Pruned {count} completed quests and steps"),"backup":backup,"status":status(path)?}),
                )
            }
            Err(error) => {
                active.conn.execute_batch("ROLLBACK")?;
                Err(error)
            }
        }
    } else {
        let prepared = if action == "restore_identity" || action == "restore_cloud" {
            let identity = if action == "restore_cloud" {
                Identity::load_for_database(&active, active.get_user()?.map(|u| u.id))?
            } else {
                profile_transfer::decode_transfer_code(
                    request["transfer_code"]
                        .as_str()
                        .context("Enter a Transfer Code")?,
                    active
                        .get_user()?
                        .map(|u| u.id)
                        .unwrap_or_else(Uuid::new_v4),
                )?
            };
            let config = Config::load()?;
            if !background_sync::cloud_enabled(&active, &config)? {
                bail!("Enable Cloud Sync before restoring from the cloud");
            }
            let device = active
                .get_setting("device_id")?
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            profile_transfer::prepare_cloud(path, identity, &device, &config.server_url)?
        } else {
            prepare_file(&active, &path_field(request)?)?
        };
        let preview = json!({"ok":true,"phase":"preview","token":Uuid::new_v4().to_string(),"kind":action,"incoming":profile(&prepared.db)?,"current":profile(&active)?,"message":"Replace this PC's local profile with the previewed profile. A pre-restore backup is saved automatically. Auto Sync stays off until you review the result."});
        emit(&preview)?;
        let decision = read_request(input)?;
        if decision["confirm"] != preview["token"] {
            return Ok(json!({"ok":true,"cancelled":true,"message":"Restore canceled"}));
        }
        account_matches(&active, request)?;
        profile_transfer::activate(&mut active, &prepared)?;
        let warning = prepared
            .identity
            .save_compatibility_copy(path.parent().context("Missing profile directory")?)
            .err()
            .map(|error| {
                format!(
                    "Profile restored; compatibility identity file could not be updated: {error}"
                )
            });
        Ok(
            json!({"ok":true,"profile_replaced":true,"message":"Profile restored. Review your data, then enable Auto Sync.","warning":warning,"status":status(path)?}),
        )
    }
}

pub fn run_stream(path: &Path) -> Result<()> {
    let mut input = std::io::stdin().lock();
    let result = (|| {
        let request = read_request(&mut input)?;
        interactive(path, &request, &mut input)
    })();
    match result {
        Ok(value) => emit(&value),
        Err(error) => emit(&json!({"ok":false,"error":error.to_string()})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, Database) {
        let directory =
            std::env::temp_dir().join(format!("questline-sync-control-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("questline.db");
        let db = Database::new(&path).unwrap();
        let identity =
            profile_transfer::decode_transfer_code(&STANDARD.encode([7u8; 32]), Uuid::new_v4())
                .unwrap();
        db.insert_user(&crate::models::User {
            id: identity.user_uuid,
            username: "Sync explorer".into(),
            class: crate::models::ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            specialization: None,
            created_at: chrono::Utc::now(),
        })
        .unwrap();
        db.set_setting(
            "_profile_identity",
            &serde_json::to_string(&identity).unwrap(),
        )
        .unwrap();
        db.set_setting("device_id", "fixture-device").unwrap();
        db.set_setting("cloud_sync_enabled", "false").unwrap();
        (path, db)
    }
    #[test]
    fn sync_preferences_are_local_persistent_and_account_guarded() {
        let (path, db) = fixture();
        let account = profile(&db).unwrap()["account"].clone();
        assert!(
            perform(
                &path,
                &json!({"action":"set_auto_sync","account":"stale","enabled":true})
            )
            .is_err()
        );
        perform(
            &path,
            &json!({"action":"set_auto_sync","account":account,"enabled":false}),
        )
        .unwrap();
        assert!(!status(&path).unwrap()["auto_sync"].as_bool().unwrap());
        perform(
            &path,
            &json!({"action":"set_cloud_sync","account":account,"enabled":true}),
        )
        .unwrap();
        assert!(status(&path).unwrap()["cloud_enabled"].as_bool().unwrap());
        let count:i64=db.conn.query_row("SELECT COUNT(*) FROM sync_log WHERE entity_type='setting' AND entity_id IN ('auto_sync','cloud_sync_enabled')",[],|r|r.get(0)).unwrap();
        assert_eq!(count, 0);
    }
    #[test]
    fn recovery_hold_cannot_be_enabled_away() {
        let (path, db) = fixture();
        db.set_setting("sync_restore_hold", "1").unwrap();
        assert!(perform(&path,&json!({"action":"set_auto_sync","account":profile(&db).unwrap()["account"],"enabled":true})).is_err());
    }
    #[test]
    fn local_export_backup_and_reserved_paths_are_safe() {
        let (path, db) = fixture();
        let account = profile(&db).unwrap()["account"].clone();
        let output = path.with_file_name("profile.json");
        let result = perform(
            &path,
            &json!({"action":"export_profile","account":account,"path":output}),
        )
        .unwrap();
        assert_eq!(result["cloud_saved"], false);
        assert!(result.get("transfer_code").is_none());
        assert!(output.is_file());
        assert!(
            perform(
                &path,
                &json!({"action":"export_profile","account":account,"path":path})
            )
            .is_err()
        );
        let result = perform(&path, &json!({"action":"backup","account":account})).unwrap();
        assert!(Database::verify_db_backup(Path::new(result["path"].as_str().unwrap())).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(output).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn local_sqlite_restore_reads_a_copy_and_keeps_destination_device() {
        let (path, db) = fixture();
        let backup = path.with_file_name("saved.db");
        db.backup_to(&backup).unwrap();
        let original = std::fs::read(&backup).unwrap();
        let prepared = prepare_file(&db, &backup).unwrap();
        assert_eq!(
            profile(&prepared.db).unwrap()["account"],
            profile(&db).unwrap()["account"]
        );
        drop(prepared);
        assert_eq!(std::fs::read(backup).unwrap(), original);
    }
    #[test]
    fn pruning_retains_unfinished_recent_and_shared_branches() {
        let (_, db) = fixture();
        db.conn.execute("INSERT INTO projects(id,name,created_at,is_shared) VALUES ('shared','Shared','2000-01-01T00:00:00Z',1)",[]).unwrap();
        for (id, parent, complete, date, project) in [
            ("old", None, 1, "2000-01-01T00:00:00Z", None),
            ("old-step", Some("old"), 1, "2000-01-01T00:00:00Z", None),
            ("protected", None, 1, "2000-01-01T00:00:00Z", None),
            (
                "unfinished",
                Some("protected"),
                0,
                "2000-01-01T00:00:00Z",
                None,
            ),
            ("recent-parent", None, 1, "2000-01-01T00:00:00Z", None),
            (
                "recent-step",
                Some("recent-parent"),
                1,
                "2999-01-01T00:00:00Z",
                None,
            ),
            (
                "shared-task",
                None,
                1,
                "2000-01-01T00:00:00Z",
                Some("shared"),
            ),
        ] {
            db.conn.execute("INSERT INTO tasks(id,title,parent_task_id,completed,created_at,updated_at,project_id) VALUES (?1,?1,?2,?3,?4,?4,?5)",rusqlite::params![id,parent,complete,date,project]).unwrap();
        }
        assert_eq!(db.count_prunable_tasks(30).unwrap(), 2);
        assert_eq!(db.prune_completed_tasks(30).unwrap(), 2);
        assert_eq!(
            db.conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            db.conn
                .query_row(
                    "SELECT COUNT(*) FROM sync_log WHERE entity_type='task' AND operation='delete'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
    }
    #[test]
    fn profile_replacement_invalidates_stale_shell_actions() {
        let (path, db) = fixture();
        db.set_setting("shell_profile_epoch", "new-profile")
            .unwrap();
        let error=crate::shell_api::run(&path,&json!({"action":"create_scroll","profile_key":"old-profile","title":"Never publish this old draft"})).unwrap_err();
        assert!(error.to_string().contains("older profile"));
    }
    #[test]
    fn status_uses_the_latest_successful_sync_without_leaking_keys() {
        let (path, db) = fixture();
        db.set_setting("background_sync_at", "2026-10-18T00:00:00Z")
            .unwrap();
        db.set_setting("last_sync", "2026-10-19T00:00:00Z").unwrap();
        let state = status(&path).unwrap();
        assert!(
            state["last_success"]
                .as_str()
                .unwrap()
                .starts_with("2026-10-19")
        );
        assert!(!state.to_string().contains("secret_key"));
    }
    #[test]
    fn running_tui_observes_widget_sync_preferences() {
        let (path, db) = fixture();
        let mut app = crate::app::App::new_headless(&path).unwrap();
        db.set_setting("cloud_sync_enabled", "true").unwrap();
        db.set_setting("auto_sync", "false").unwrap();
        app.last_sync_settings_check = None;
        app.tick_auto_sync().unwrap();
        assert!(app.config.sync_enabled);
        assert!(!app.auto_sync);
        db.set_setting("cloud_sync_enabled", "false").unwrap();
        db.set_setting("auto_sync", "true").unwrap();
        app.last_sync_settings_check = None;
        app.tick_auto_sync().unwrap();
        assert!(!app.config.sync_enabled);
        assert!(app.auto_sync);
        assert!(!app.sync_in_progress);
    }
}
