use crate::{
    database::Database,
    services::{
        config::Config,
        identity::{Identity, get_local_device_name},
        sync_engine::SyncEngine,
    },
};
use anyhow::{Context, Result, bail};
use rusqlite::OptionalExtension;
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};

fn private_lock(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}

pub fn cloud_enabled(db: &Database, config: &Config) -> Result<bool> {
    Ok(db
        .get_setting("cloud_sync_enabled")?
        .map(|v| v == "true")
        .unwrap_or(config.sync_enabled))
}

pub fn allowed(db: &Database, config: &Config) -> Result<bool> {
    Ok(cloud_enabled(db, config)?
        && db.get_user()?.is_some()
        && db.get_setting("sync_restore_hold")?.as_deref() != Some("1")
        && db
            .get_setting("auto_sync")?
            .map(|v| v == "true")
            .unwrap_or(config.auto_sync))
}

pub fn sync_once(path: &Path) -> Result<()> {
    sync_mode(path, false)
}
pub fn sync_now(path: &Path) -> Result<()> {
    sync_mode(path, true)
}
fn sync_mode(path: &Path, manual: bool) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let db = Database::new(path)?;
    let config = Config::load()?;
    if manual {
        if !cloud_enabled(&db, &config)? {
            bail!("Enable Cloud Sync before syncing manually");
        }
        if db.get_setting("sync_restore_hold")?.as_deref() == Some("1") {
            bail!("This profile is on a recovery hold. Restore the profile before syncing.");
        }
    } else if !allowed(&db, &config)? {
        return Ok(());
    }
    let startup_lock = db.lock_sync()?;
    let user = db.get_user()?.context("No local profile")?;
    let identity = Identity::load_for_database(&db, Some(user.id))?;
    let name = get_local_device_name();
    if db
        .get_setting("device_bound_name")?
        .is_some_and(|bound| bound != name)
    {
        bail!(
            "This profile was copied from another PC. Open Questline to bind this device before syncing."
        );
    }
    let device = match db.get_setting("device_id")? {
        Some(id) => id,
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            db.set_setting("device_id", &id)?;
            id
        }
    };
    let previous_name: Option<String> = db
        .conn
        .query_row(
            "SELECT device_name FROM devices WHERE device_id=?",
            [&device],
            |r| r.get(0),
        )
        .optional()?;
    if previous_name.is_some_and(|bound| bound != name) {
        bail!("Open Questline to bind this copied profile to the current PC before syncing.");
    }
    if db.get_setting("device_bound_name")?.is_none() {
        db.set_setting("device_bound_name", &name)?;
    }
    if db.get_setting("_profile_identity")?.is_none() {
        db.set_setting("_profile_identity", &serde_json::to_string(&identity)?)?;
    }
    db.conn.execute("INSERT OR IGNORE INTO devices (device_id,device_name,created_at,last_sync) VALUES (?,?,?,NULL)",rusqlite::params![device,name,chrono::Utc::now().to_rfc3339()])?;
    drop(startup_lock);
    let engine = SyncEngine::new(&db, &identity, &device, Some(&config.server_url))?;
    let result = engine.sync();
    if let Ok(_status_lock) = db.lock_sync() {
        let still_current = db
            .get_setting("_profile_identity")?
            .and_then(|s| serde_json::from_str::<Identity>(&s).ok())
            .is_some_and(|v| v.public_key == identity.public_key);
        if still_current {
            db.set_setting(
                "background_sync_error",
                &result
                    .as_ref()
                    .err()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )?;
            if result.is_ok() {
                db.set_setting("background_sync_at", &chrono::Utc::now().to_rfc3339())?;
            }
        }
    }
    match result {
        Ok((_, _, conflicts)) => {
            if !conflicts.is_empty() {
                eprintln!(
                    "Questline sync: {} event(s) need attention",
                    conflicts.len()
                );
            }
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub fn run(path: &Path) -> Result<()> {
    let directory = path.parent().context("Missing profile directory")?;
    std::fs::create_dir_all(directory)?;
    let singleton = private_lock(&directory.join("background-sync.lock"))?;
    if singleton.try_lock().is_err() {
        return Ok(());
    }
    let presence = private_lock(&directory.join("desktop-tui.lock"))?;
    let mut failures = 0u32;
    loop {
        if presence.try_lock().is_ok() {
            presence.unlock()?;
            match sync_once(path) {
                Ok(()) => failures = 0,
                Err(error) => {
                    failures = failures.saturating_add(1);
                    eprintln!("Questline background sync: {error}");
                }
            }
        }
        std::thread::sleep(Duration::from_secs(
            (5u64.saturating_mul(1 << failures.min(6))).min(300),
        ));
    }
}
