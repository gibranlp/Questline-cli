//! A restored profile is built off to the side and activated only after validation.
use crate::{
    database::Database,
    services::{ApiClient, Identity, profile_backup, sync_engine::SyncEngine},
};
use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct PreparedRestore {
    pub db: Database,
    pub identity: Identity,
    path: PathBuf,
    // Keep the active profile exclusively owned until activation or cancellation.
    _lock: Option<std::fs::File>,
    active_guard: Option<(Database, i64)>,
}

impl Drop for PreparedRestore {
    fn drop(&mut self) {
        // SQLite still has its file open here; Unix permits cleanup. On other
        // platforms a leftover staging file is harmless and can be removed later.
        for path in [
            self.path.clone(),
            self.path.with_extension("sync.lock"),
            PathBuf::from(format!("{}-wal", self.path.display())),
            PathBuf::from(format!("{}-shm", self.path.display())),
        ] {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn decode_transfer_code(code: &str, fallback_user: Uuid) -> Result<Identity> {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = STANDARD
        .decode(compact)
        .context("Invalid transfer code encoding")?;
    let (user_uuid, secret) = match bytes.len() {
        48 => (Uuid::from_slice(&bytes[..16])?, &bytes[16..]),
        32 => (fallback_user, bytes.as_slice()),
        _ => bail!("Transfer code must contain a 32-byte key or a UUID and 32-byte key"),
    };
    let key = SigningKey::from_bytes(secret.try_into()?);
    Ok(Identity {
        user_uuid,
        public_key: hex(&key.verifying_key().to_bytes()),
        secret_key: hex(secret),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn prepare_json(active: &Database, json: &str) -> Result<PreparedRestore> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    for table in ["users", "projects", "tasks", "notes", "settings"] {
        if !value[table].is_array() {
            bail!("Profile backup is missing its {table} table");
        }
    }
    let active_path = active.path()?;
    let guard = Database::new(&active_path)?;
    let lock = guard.lock_sync()?;
    let version = guard
        .conn
        .query_row("PRAGMA data_version", [], |r| r.get(0))?;
    let path = active_path.with_file_name(format!(".questline-restore-{}.db", Uuid::new_v4()));
    let db = Database::new(&path)?;
    let fallback = Identity::load_for_database(active, active.get_user()?.map(|user| user.id))?;
    let mut prepared = PreparedRestore {
        db,
        identity: fallback,
        path,
        _lock: lock,
        active_guard: Some((guard, version)),
    };
    prepared.db.import_from_json(json)?;
    if let Some(json) = prepared.db.get_setting("_profile_identity")? {
        prepared.identity = serde_json::from_str(&json)?;
    }
    prepared.identity.validate()?;
    if let Some(user) = prepared.db.get_user()? {
        prepared.identity.user_uuid = user.id;
    }
    Ok(prepared)
}

pub fn prepare_cloud(
    active_path: &Path,
    mut identity: Identity,
    device: &str,
    server: &str,
) -> Result<PreparedRestore> {
    let active = Database::new(active_path)?;
    let lock = active.lock_sync()?;
    let data_version: i64 = active
        .conn
        .query_row("PRAGMA data_version", [], |r| r.get(0))?;
    let path = active_path.with_file_name(format!(".questline-restore-{}.db", Uuid::new_v4()));
    let db = Database::new(&path)?;
    let mut prepared = PreparedRestore {
        db,
        identity: identity.clone(),
        path,
        _lock: lock,
        active_guard: Some((active, data_version)),
    };
    prepared.db.set_setting("device_id", device)?;
    let client = ApiClient::new(server, identity.clone(), device);
    let head: serde_json::Value =
        serde_json::from_str(&client.send_request("GET", "sync/v2/head", "")?)?;
    let current_generation = head["generation"].as_i64();
    match client.send_request("GET", "profile/v2/backup/latest", "") {
        Ok(raw) => {
            let envelope = serde_json::from_str::<profile_backup::Envelope>(&raw)?;
            let json = profile_backup::open(&envelope, &identity)?;
            // A reset stream is a complete replacement. An older backup must
            // not reintroduce rows intentionally omitted from that replacement.
            if envelope.generation == current_generation
                || (envelope.generation.is_none() && current_generation == Some(0))
            {
                prepared.db.import_from_json(&json)?;
                if let Some(generation) = current_generation {
                    prepared
                        .db
                        .set_setting("sync_history_generation_v2", &generation.to_string())?;
                }
            }
        }
        // Older servers/accounts may have only the incremental encrypted stream.
        Err(error)
            if error
                .downcast_ref::<crate::services::api_client::HttpError>()
                .is_some_and(|error| error.status == 404) => {}
        Err(error) => return Err(error.context("Could not download encrypted profile backup")),
    }
    // A unique recovery device must see events authored by this same machine.
    // Its ID is temporary; activation preserves the real destination device ID.
    let recovery_device = Uuid::new_v4().to_string();
    SyncEngine::new(&prepared.db, &identity, &recovery_device, Some(server))?.pull_only()?;
    let user = prepared
        .db
        .get_user()?
        .context("No recoverable profile was found for this identity")?;
    identity.user_uuid = user.id;
    prepared.identity = identity;
    if !Database::verify_db_backup(&prepared.path)? {
        bail!("Restored profile failed validation");
    }
    Ok(prepared)
}

/// Install data and identity in one transaction. Checking the original connection
/// under a write lock prevents another process's edits during download being lost.
pub fn activate(active: &mut Database, prepared: &PreparedRestore) -> Result<()> {
    let device = active.get_setting("device_id")?.unwrap_or_else(||Uuid::new_v4().to_string());
    prepared.db.set_setting("device_id", &device)?;
    let name=active.get_setting("device_bound_name")?.unwrap_or_else(crate::services::identity::get_local_device_name);
    prepared.db.set_setting("device_bound_name", &name)?;
    for key in [
        "server_url",
        "cloud_sync_enabled",
        "local_music_folder",
        "last_music_source",
        "last_music_volume",
    ] {
        if let Some(value) = active.get_setting(key)? {
            prepared.db.set_setting(key, &value)?;
        } else {
            prepared
                .db
                .conn
                .execute("DELETE FROM settings WHERE key=?1", [key])?;
        }
    }
    prepared.db.set_setting("shell_profile_epoch",&Uuid::new_v4().to_string())?;
    prepared.db.set_setting(
        "_profile_identity",
        &serde_json::to_string(&prepared.identity)?,
    )?;
    prepared.db.set_setting("auto_sync", "false")?;
    // Pulled state is already on the server. A restored backup can still carry
    // data absent from the stream; queue it explicitly when resuming, not on exit.
    prepared.db.set_setting("sync_restore_hold", "0")?;
    prepared.db.queue_full_state_sync()?;
    let backup_path = active
        .path()?
        .with_file_name(format!("questline_backup_prerestore_{}.db", Uuid::new_v4()));
    active.backup_to(&backup_path)?;
    active.conn.execute(
        "ATTACH DATABASE ?1 AS restored_profile",
        [prepared.path.to_string_lossy().as_ref()],
    )?;
    active.conn.execute_batch("PRAGMA foreign_keys=OFF")?;
    let result = (|| -> Result<()> {
        let tx = active
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some((guard, before)) = &prepared.active_guard {
            let now: i64 = guard
                .conn
                .query_row("PRAGMA data_version", [], |r| r.get(0))?;
            if now != *before {
                bail!(
                    "Local data changed while the profile downloaded. Restore canceled; retry with the latest local state saved."
                );
            }
        }
        tx.execute("UPDATE sync_context SET enabled=0", [])?;
        let tables = tx.prepare("SELECT name FROM main.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")?
            .query_map([], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for table in tables {
            if !table
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                bail!("Invalid local table name");
            }
            tx.execute(&format!("DELETE FROM main.{table}"), [])?;
            tx.execute(
                &format!("INSERT INTO main.{table} SELECT * FROM restored_profile.{table}"),
                [],
            )?;
        }
        let invalid: i64 =
            tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })?;
        if invalid != 0 {
            bail!("Restored profile has invalid relationships");
        }
        tx.execute("UPDATE sync_context SET enabled=1", [])?;
        tx.commit()?;
        Ok(())
    })();
    let foreign_keys = active.conn.execute_batch("PRAGMA foreign_keys=ON");
    let detach = active
        .conn
        .execute_batch("DETACH DATABASE restored_profile");
    result?;
    foreign_keys?;
    detach?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_replaces_data_and_identity_together_and_keeps_destination_device() {
        let path = std::env::temp_dir().join(format!("questline-transfer-{}.db", Uuid::new_v4()));
        let mut active = Database::new(&path).unwrap();
        active.set_setting("device_id", "destination").unwrap();
        active
            .set_setting("old-account-data", "must disappear")
            .unwrap();
        let stage_path = path.with_extension("stage.db");
        let staged = Database::new(&stage_path).unwrap();
        staged.set_setting("restored-data", "present").unwrap();
        let identity = decode_transfer_code(&STANDARD.encode([1u8; 32]), Uuid::new_v4()).unwrap();
        let prepared = PreparedRestore {
            db: staged,
            identity: identity.clone(),
            path: stage_path,
            _lock: active.lock_sync().unwrap(),
            active_guard: None,
        };
        activate(&mut active, &prepared).unwrap();
        assert!(active.get_setting("old-account-data").unwrap().is_none());
        assert_eq!(
            active.get_setting("restored-data").unwrap().as_deref(),
            Some("present")
        );
        assert_eq!(
            active.get_setting("device_id").unwrap().as_deref(),
            Some("destination")
        );
        let persisted: Identity =
            serde_json::from_str(&active.get_setting("_profile_identity").unwrap().unwrap())
                .unwrap();
        assert_eq!(persisted.public_key, identity.public_key);
        drop(prepared);
        drop(active);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn invalid_transfer_code_is_rejected_before_any_profile_changes() {
        assert!(decode_transfer_code("bad code", Uuid::new_v4()).is_err());
        assert!(decode_transfer_code(&STANDARD.encode([1u8; 33]), Uuid::new_v4()).is_err());
    }

    #[test]
    fn concurrent_local_edit_cancels_activation_without_losing_either_profile() {
        let directory =
            std::env::temp_dir().join(format!("questline-activation-race-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("active.db");
        let mut active = Database::new(&path).unwrap();
        let identity = decode_transfer_code(&STANDARD.encode([2u8; 32]), Uuid::new_v4()).unwrap();
        active
            .set_setting(
                "_profile_identity",
                &serde_json::to_string(&identity).unwrap(),
            )
            .unwrap();
        active.set_setting("existing-data", "kept").unwrap();
        let source = Database::new(Path::new(":memory:")).unwrap();
        source
            .set_setting("replacement-data", "staged only")
            .unwrap();
        let prepared = prepare_json(&active, &source.export_to_json().unwrap()).unwrap();
        let concurrent = Database::new(&path).unwrap();
        concurrent
            .set_setting("edit-during-download", "also kept")
            .unwrap();
        assert!(
            activate(&mut active, &prepared)
                .unwrap_err()
                .to_string()
                .contains("Local data changed")
        );
        assert_eq!(
            active.get_setting("existing-data").unwrap().as_deref(),
            Some("kept")
        );
        assert_eq!(
            active
                .get_setting("edit-during-download")
                .unwrap()
                .as_deref(),
            Some("also kept")
        );
        assert!(active.get_setting("replacement-data").unwrap().is_none());
        assert!(
            prepared
                .db
                .get_setting("replacement-data")
                .unwrap()
                .is_some()
        );
        drop(concurrent);
        drop(prepared);
        drop(active);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "requires disposable local PHP/MariaDB integration server"]
    fn encrypted_cloud_transfer_preserves_source_and_replaces_destination_only_at_activation() {
        let server = std::env::var("QUESTLINE_TEST_API_URL")
            .expect("Set QUESTLINE_TEST_API_URL to a disposable localhost API");
        assert!(server.starts_with("http://127.0.0.1:") || server.starts_with("http://localhost:"));
        let directory =
            std::env::temp_dir().join(format!("questline-cloud-transfer-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let source = Database::new(&directory.join("source.db")).unwrap();
        let mut destination = Database::new(&directory.join("destination.db")).unwrap();
        let identity =
            decode_transfer_code(&STANDARD.encode(rand::random::<[u8; 32]>()), Uuid::new_v4())
                .unwrap();
        let now = chrono::Utc::now();
        let source_device = Uuid::new_v4().to_string();
        let destination_device = Uuid::new_v4().to_string();
        source
            .insert_user(&crate::models::User {
                id: identity.user_uuid,
                username: "Transferred Hero".into(),
                class: crate::models::ClassType::CodeWarlock,
                level: 2,
                xp: 23,
                created_at: now,
                specialization: None,
            })
            .unwrap();
        source
            .set_setting(
                "_profile_identity",
                &serde_json::to_string(&identity).unwrap(),
            )
            .unwrap();
        source
            .insert_reflection(&crate::models::DailyReflection {
                created_date: now.date_naive(),
                what_went_well: "Cloud transfer".into(),
                what_can_improve: "Nothing lost".into(),
            })
            .unwrap();
        source.unlock_trait("cloud-transfer-test").unwrap();
        destination
            .set_setting("device_id", &destination_device)
            .unwrap();
        destination
            .set_setting("private-old-account-data", "must disappear")
            .unwrap();
        SyncEngine::new(&source, &identity, &source_device, Some(&server))
            .unwrap()
            .sync()
            .unwrap();
        profile_backup::upload(&source, &identity, &source_device, &server).unwrap();
        let prepared = prepare_cloud(
            &destination.path().unwrap(),
            identity.clone(),
            &destination_device,
            &server,
        )
        .unwrap();
        assert_eq!(
            destination
                .get_setting("private-old-account-data")
                .unwrap()
                .as_deref(),
            Some("must disappear")
        );
        assert!(
            destination.get_user().unwrap().is_none(),
            "download must not activate prematurely"
        );
        activate(&mut destination, &prepared).unwrap();
        assert!(
            destination
                .get_setting("private-old-account-data")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            destination.get_user().unwrap().unwrap().username,
            "Transferred Hero"
        );
        assert_eq!(destination.get_user().unwrap().unwrap().xp, 23);
        assert_eq!(
            destination.get_unlocked_traits().unwrap(),
            vec!["cloud-transfer-test"]
        );
        assert_eq!(destination.get_reflections().unwrap().len(), 1);
        assert_eq!(
            destination.get_setting("device_id").unwrap().as_deref(),
            Some(destination_device.as_str())
        );
        drop(prepared);
        // Pending restored state can be safely published with the destination ID.
        SyncEngine::new(&destination, &identity, &destination_device, Some(&server))
            .unwrap()
            .sync()
            .unwrap();
        assert!(destination.get_pending_sync_logs().unwrap().is_empty());
        // Reset while the independent backup still contains a deleted trait.
        // Recovery must use the new stream, never resurrect the stale backup row.
        source.conn.execute("DELETE FROM traits", []).unwrap();
        source.queue_full_state_sync().unwrap();
        let engine = SyncEngine::new(&source, &identity, &source_device, Some(&server)).unwrap();
        engine.replace_with_pending_snapshot().unwrap();
        assert!(engine.verify_remote_has_events().unwrap() > 0);
        let restored = prepare_cloud(
            &destination.path().unwrap(),
            identity.clone(),
            &destination_device,
            &server,
        )
        .unwrap();
        assert!(restored.db.get_unlocked_traits().unwrap().is_empty());
        activate(&mut destination, &restored).unwrap();
        assert!(destination.get_unlocked_traits().unwrap().is_empty());
        drop(restored);

        drop(destination);
        drop(source);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
