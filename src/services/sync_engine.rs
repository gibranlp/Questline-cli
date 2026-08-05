// ─────────────────────────────────────────────────────────────────────────────
// sync_engine.rs — el corazón del sync: push primero, pull después, nunca al revés
// ─────────────────────────────────────────────────────────────────────────────
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::database::Database;
use crate::models::{Note, QuestStatus, Task};
use crate::services::Identity;

const PULL_PAGE_SIZE: usize = 1000;
const MAX_DRAIN_PAGES: usize = 100;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncLogEntry {
    #[serde(default)]
    pub version: u8,
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: String,
    pub timestamp: String,
    pub content: Option<String>,
    #[serde(default)]
    pub key_id: String,
    #[serde(default)]
    pub nonce: String,
    #[serde(default)]
    pub ciphertext: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub routing_id: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub author_public_key: String,
    #[serde(default)]
    pub event_signature: String,
    // seq — cursor incremental del servidor para no descargar toda la historia en cada sync
    #[serde(default)]
    pub seq: i64,
}

#[derive(Debug)]
struct PullPage {
    events: Vec<SyncLogEntry>,
    head_seq: i64,
    next_seq: i64,
    has_more: bool,
    metadata_supported: bool,
    signatures_required: bool,
}

#[derive(Deserialize)]
struct PullPageEnvelope {
    events: Vec<SyncLogEntry>,
    #[serde(default)]
    head_seq: i64,
    #[serde(default)]
    next_seq: i64,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    signatures_required: bool,
}

fn parse_pull_page(data: &str, since_seq: i64) -> Result<PullPage> {
    if let Ok(envelope) = serde_json::from_str::<PullPageEnvelope>(data) {
        let max_event_seq = envelope
            .events
            .iter()
            .map(|e| e.seq)
            .max()
            .unwrap_or(since_seq);
        let next_seq = envelope.next_seq.max(max_event_seq).max(since_seq);
        let head_seq = envelope.head_seq.max(next_seq);
        return Ok(PullPage {
            events: envelope.events,
            head_seq,
            next_seq,
            has_more: envelope.has_more,
            metadata_supported: true,
            signatures_required: envelope.signatures_required,
        });
    }

    let events: Vec<SyncLogEntry> = serde_json::from_str(data)?;
    let next_seq = events.iter().map(|e| e.seq).max().unwrap_or(since_seq);
    Ok(PullPage {
        has_more: events.len() >= PULL_PAGE_SIZE,
        head_seq: next_seq,
        next_seq,
        events,
        metadata_supported: false,
        signatures_required: false,
    })
}

pub trait CloudProvider {
    fn name(&self) -> &str;
    /// Production transports require sync-v2. Test providers may return decoded fixtures.
    fn requires_encryption(&self) -> bool {
        false
    }
    fn prepare_pull(&self, _db: &Database, _identity: &Identity) -> Result<()> {
        Ok(())
    }
    fn push(&self, public_key: &str, signature: &str, payload: &str) -> Result<()>;
    fn replace_snapshot(&self, public_key: &str, signature: &str, payload: &str) -> Result<()> {
        self.push(public_key, signature, payload)
    }
    fn pull(&self, public_key: &str, signature: &str, since_seq: i64) -> Result<String>;
}

// Simula el servidor en disco — para desarrollo y para que el sync funcione sin internet
pub struct FileCloudProvider {
    pub base_dir: PathBuf,
}

impl FileCloudProvider {
    pub fn new() -> Result<Self> {
        let storage_dir = crate::storage::get_storage_dir()?;
        let base_dir = storage_dir.join("cloud_chronicle");
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir })
    }

    fn user_log_file(&self, public_key: &str) -> PathBuf {
        // La llave pública en hex no tiene caracteres raros — sirve de nombre de archivo directo
        self.base_dir.join(format!("{}_logs.json", public_key))
    }
}

impl CloudProvider for FileCloudProvider {
    fn name(&self) -> &str {
        "Cloud Chronicle (File-Simulated)"
    }

    fn requires_encryption(&self) -> bool {
        true
    }

    // Nada se escribe sin firma criptográfica válida — el servidor de archivos también la exige
    fn push(&self, public_key: &str, signature: &str, payload: &str) -> Result<()> {
        let verified = Identity::verify(payload.as_bytes(), public_key, signature)?;
        if !verified {
            return Err(anyhow!(
                "Security Error: Signature verification failed for push"
            ));
        }

        let new_entries: Vec<SyncLogEntry> = serde_json::from_str(payload)?;
        let log_file = self.user_log_file(public_key);

        let mut existing_entries: Vec<SyncLogEntry> = if log_file.exists() {
            let data = std::fs::read_to_string(&log_file)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Dedup por id — el mismo evento no se guarda dos veces
        for entry in new_entries {
            if !existing_entries.iter().any(|e| e.id == entry.id) {
                existing_entries.push(entry);
            }
        }

        std::fs::write(&log_file, serde_json::to_string_pretty(&existing_entries)?)?;
        Ok(())
    }

    fn pull(&self, public_key: &str, _signature: &str, _since_seq: i64) -> Result<String> {
        let log_file = self.user_log_file(public_key);
        if log_file.exists() {
            let data = std::fs::read_to_string(&log_file)?;
            Ok(data)
        } else {
            Ok("[]".to_string())
        }
    }

    fn replace_snapshot(&self, public_key: &str, signature: &str, payload: &str) -> Result<()> {
        if !Identity::verify(payload.as_bytes(), public_key, signature)? {
            return Err(anyhow!(
                "Security Error: Signature verification failed for snapshot"
            ));
        }
        let entries: Vec<SyncLogEntry> = serde_json::from_str(payload)?;
        std::fs::write(
            self.user_log_file(public_key),
            serde_json::to_string_pretty(&entries)?,
        )?;
        Ok(())
    }
}

use crate::services::ApiClient;

fn total_user_progress_xp(user: &crate::models::User) -> i64 {
    let completed_levels: i64 = (1..user.level.max(1))
        .map(|level| crate::models::User::xp_for_next_level(level) as i64)
        .sum();
    completed_levels + user.xp.max(0) as i64
}

fn project_id_from_sync_content(
    entity_type: &str,
    entity_id: &str,
    content: &str,
) -> Option<String> {
    if entity_type == "project_key" {
        return None;
    }
    if entity_type == "project" {
        return serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|value| value["is_shared"].as_bool().filter(|shared| *shared))
            .map(|_| entity_id.to_string());
    }
    if entity_type == "project_member" {
        return entity_id
            .split_once("__")
            .map(|(project_id, _)| project_id.to_string());
    }
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|value| {
            value["project_id"]
                .as_str()
                .or_else(|| value["campaign_id"].as_str())
                .map(str::to_string)
        })
}

pub struct HttpCloudProvider {
    pub client: ApiClient,
}

impl CloudProvider for HttpCloudProvider {
    fn name(&self) -> &str {
        "Cloud Chronicle (HTTPS REST)"
    }

    fn requires_encryption(&self) -> bool {
        true
    }

    fn prepare_pull(&self, db: &Database, identity: &Identity) -> Result<()> {
        let response = self
            .client
            .send_request("GET", "project/key-envelopes", "")?;
        let envelopes: serde_json::Value = serde_json::from_str(&response)?;
        let Some(envelopes) = envelopes.as_array() else {
            return Err(anyhow!("invalid project key-envelope response"));
        };
        for envelope in envelopes {
            let old_route = envelope["old_routing_id"].as_str().unwrap_or_default();
            let new_route = envelope["new_routing_id"].as_str().unwrap_or_default();
            let sender_key = envelope["sender_encryption_key"]
                .as_str()
                .unwrap_or_default();
            let nonce = envelope["key_nonce"].as_str().unwrap_or_default();
            let ciphertext = envelope["key_ciphertext"].as_str().unwrap_or_default();
            if old_route.is_empty() || new_route.is_empty() {
                return Err(anyhow!("malformed project key rotation envelope"));
            }
            if db
                .get_project_encryption_key_by_routing_id(new_route)?
                .is_some()
            {
                continue;
            }
            let (project_id, _) = db
                .get_project_encryption_key_by_routing_id(old_route)?
                .ok_or_else(|| anyhow!("missing prior Fellowship key for route {old_route}"))?;
            let key = crate::services::encryption::unwrap_project_key(
                identity, sender_key, new_route, nonce, ciphertext,
            )?;
            db.save_project_encryption_key(&project_id, new_route, &key)?;
        }

        let response = self.client.send_request("GET", "project/revocations", "")?;
        let revocations: serde_json::Value = serde_json::from_str(&response)?;
        let Some(revocations) = revocations.as_array() else {
            return Err(anyhow!("invalid project revocation response"));
        };
        for revocation in revocations {
            let old_route = revocation["routing_id"].as_str().unwrap_or_default();
            let replacement_route = revocation["replacement_routing_id"]
                .as_str()
                .unwrap_or_default();
            if old_route.is_empty() || replacement_route.is_empty() {
                return Err(anyhow!("malformed project revocation"));
            }
            if db.is_project_route_revoked(old_route)? {
                continue;
            }
            if let Some(project_id) =
                db.apply_project_route_revocation(old_route, replacement_route)?
            {
                let _ = db.create_notification(
                    "fellowship_access_removed",
                    "Fellowship access removed",
                    "A shared campaign is now a private local copy. Future changes will not be sent to its former Fellowship.",
                    Some(&project_id),
                );
            }
        }
        Ok(())
    }

    fn push(&self, _public_key: &str, _signature: &str, payload: &str) -> Result<()> {
        self.client.send_request("POST", "sync/v2/push", payload)?;
        Ok(())
    }

    fn pull(&self, _public_key: &str, _signature: &str, since_seq: i64) -> Result<String> {
        self.client.send_request(
            "POST",
            &format!(
                "sync/v2/pull?since_seq={}&limit={}&include_meta=1",
                since_seq, PULL_PAGE_SIZE
            ),
            "",
        )
    }

    fn replace_snapshot(&self, _public_key: &str, _signature: &str, payload: &str) -> Result<()> {
        self.client
            .send_request("POST", "sync/v2/snapshot", payload)?;
        Ok(())
    }
}

pub struct SyncEngine<'a> {
    pub db: &'a Database,
    pub identity: &'a Identity,
    pub device_id: &'a str,
    pub provider: Box<dyn CloudProvider>,
}

impl<'a> SyncEngine<'a> {
    pub fn new(
        db: &'a Database,
        identity: &'a Identity,
        device_id: &'a str,
        server_url: Option<&str>,
    ) -> Result<Self> {
        let provider: Box<dyn CloudProvider> = if let Some(url) = server_url {
            let client = ApiClient::new(url, identity.clone(), device_id);
            Box::new(HttpCloudProvider { client })
        } else {
            Box::new(FileCloudProvider::new()?)
        };
        Ok(Self {
            db,
            identity,
            device_id,
            provider,
        })
    }

    fn save_local_revision_before_delete(&self, entity_type: &str, entity_id: &str) {
        match entity_type {
            "task" => {
                if let Ok(id) = Uuid::parse_str(entity_id) {
                    if let Ok(task) = self.db.get_task_by_id(id) {
                        if let Ok(content) = serde_json::to_string(&task) {
                            let _ = self.db.create_revision("task", entity_id, &content);
                        }
                    }
                }
            }
            "note" => {
                if let Ok(id) = Uuid::parse_str(entity_id) {
                    if let Ok(note) = self.db.get_note_by_id(id) {
                        if let Ok(content) = serde_json::to_string(&note) {
                            let _ = self.db.create_revision("note", entity_id, &content);
                        }
                    }
                }
            }
            "project" => {
                if let Ok(projects) = self.db.get_projects() {
                    if let Some(project) =
                        projects.into_iter().find(|p| p.id.to_string() == entity_id)
                    {
                        if let Ok(content) = serde_json::to_string(&project) {
                            let _ = self.db.create_revision("project", entity_id, &content);
                        }
                    }
                }
            }
            "milestone" => {
                let _ = self.db.conn.query_row(
                    "SELECT json_object(
                        'id', id,
                        'project_id', project_id,
                        'name', name,
                        'description', description,
                        'completed', completed != 0,
                        'xp_reward', xp_reward,
                        'created_at', created_at,
                        'tier', tier,
                        'template_id', template_id
                    ) FROM milestones WHERE id = ?1",
                    params![entity_id],
                    |row| {
                        let content: String = row.get(0)?;
                        let _ = self.db.create_revision("milestone", entity_id, &content);
                        Ok(())
                    },
                );
            }
            "codex" => {
                if let Ok(codex) = self.db.get_codex_by_id(entity_id) {
                    if let Ok(content) = serde_json::to_string(&codex) {
                        let _ = self.db.create_revision("codex", entity_id, &content);
                    }
                }
            }
            "ledger_entry" => {
                if let Ok(id) = Uuid::parse_str(entity_id) {
                    if let Ok(Some(entry)) =
                        crate::services::TreasuryService::new(self.db).get_entry(id)
                    {
                        if let Ok(content) = serde_json::to_string(&entry) {
                            let _ = self.db.create_revision("ledger_entry", entity_id, &content);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn task_payload_project_is_valid(
        &self,
        task: &Task,
        remote_task_projects: &std::collections::HashMap<Uuid, Option<Uuid>>,
    ) -> bool {
        let Some(parent_id) = task.parent_task_id else {
            return true;
        };

        if let Some(parent_project_id) = remote_task_projects.get(&parent_id) {
            return task.project_id == *parent_project_id;
        }

        self.db
            .get_task_by_id(parent_id)
            .map(|parent| task.project_id == parent.project_id)
            .unwrap_or(false)
    }

    fn timestamp_or_epoch(value: Option<&str>) -> DateTime<Utc> {
        value
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(crate::models::default_sync_timestamp)
    }

    /// Cuando una fila de tesorería no se puede aplicar casi siempre es orden de llegada:
    /// falta la campaña, la categoría o la tarea a la que apunta la llave foránea. Se
    /// reporta como conflicto y se rebobina el cursor para reintentar en el próximo sync,
    /// en lugar de descartar el movimiento en silencio y quedar divergentes para siempre.
    fn defer_treasury_event(
        log: &SyncLogEntry,
        stored: rusqlite::Result<usize>,
        conflicts: &mut Vec<String>,
        retry_from_seq: &mut Option<i64>,
    ) -> bool {
        let Err(error) = stored else {
            return false;
        };
        conflicts.push(format!(
            "Treasury {} {} could not be applied yet ({}); it will be retried on the next sync",
            log.entity_type, log.entity_id, error
        ));
        if log.seq > 0 {
            *retry_from_seq = Some(retry_from_seq.map_or(log.seq, |current| current.min(log.seq)));
        }
        true
    }

    fn incoming_entity_is_newer(&self, table: &str, log: &SyncLogEntry) -> bool {
        let incoming = if log.operation == "delete" {
            Self::timestamp_or_epoch(Some(&log.timestamp))
        } else {
            let value = log
                .content
                .as_ref()
                .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok());
            Self::timestamp_or_epoch(
                value
                    .as_ref()
                    .and_then(|value| value["updated_at"].as_str()),
            )
        };
        let id_column = match table {
            "campaign_treasury" => "campaign_id",
            "category_budgets" => "category_id",
            "task_financials" => "task_id",
            _ => "id",
        };
        let sql = format!("SELECT updated_at FROM {} WHERE {} = ?1", table, id_column);
        match self
            .db
            .conn
            .query_row(&sql, params![log.entity_id], |row| row.get::<_, String>(0))
        {
            Ok(local) => incoming > Self::timestamp_or_epoch(Some(&local)),
            Err(rusqlite::Error::QueryReturnedNoRows) => true,
            Err(_) => false,
        }
    }

    fn incoming_task_status_is_newer(&self, log: &SyncLogEntry) -> bool {
        let incoming = if log.operation == "delete" {
            Self::timestamp_or_epoch(Some(&log.timestamp))
        } else {
            let value = log
                .content
                .as_ref()
                .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok());
            Self::timestamp_or_epoch(
                value
                    .as_ref()
                    .and_then(|value| value["updated_at"].as_str()),
            )
        };
        match self.db.conn.query_row(
            "SELECT updated_at FROM task_statuses WHERE task_id = ?1",
            params![log.entity_id],
            |row| row.get::<_, String>(0),
        ) {
            Ok(local) => incoming > Self::timestamp_or_epoch(Some(&local)),
            Err(rusqlite::Error::QueryReturnedNoRows) => true,
            Err(_) => false,
        }
    }

    fn incoming_notification_state_is_newer(&self, log: &SyncLogEntry) -> bool {
        let value = log
            .content
            .as_ref()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok());
        let incoming = Self::timestamp_or_epoch(
            value
                .as_ref()
                .and_then(|value| value["updated_at"].as_str()),
        );
        match self.db.conn.query_row(
            "SELECT updated_at FROM notification_states WHERE notification_id = ?1",
            params![log.entity_id],
            |row| row.get::<_, String>(0),
        ) {
            Ok(local) => incoming > Self::timestamp_or_epoch(Some(&local)),
            Err(rusqlite::Error::QueryReturnedNoRows) => true,
            Err(_) => false,
        }
    }

    pub fn sync(&self) -> Result<(usize, usize, Vec<String>)> {
        self.provider.prepare_pull(self.db, self.identity)?;
        // Pull and drain remote history first, then push local changes. This keeps restore and
        // normal sync incremental: a device learns the server head before it publishes anything.
        let (mut pushed, mut pulled, mut conflicts, mut downloaded, mut has_more) =
            self.sync_once(false, true, false)?;
        let mut drain_pages = 0usize;

        while has_more && drain_pages < MAX_DRAIN_PAGES {
            let (more_pushed, more_pulled, more_conflicts, more_downloaded, more_has_more) =
                self.sync_once(false, true, false)?;
            pushed += more_pushed;
            pulled += more_pulled;
            downloaded += more_downloaded;
            conflicts.extend(more_conflicts);
            has_more = more_has_more && more_downloaded > 0;
            drain_pages += 1;
        }

        let (more_pushed, more_pulled, more_conflicts, more_downloaded, more_has_more) =
            self.sync_once(true, true, false)?;
        pushed += more_pushed;
        pulled += more_pulled;
        downloaded += more_downloaded;
        conflicts.extend(more_conflicts);
        has_more = more_has_more && more_downloaded > 0;

        while has_more && drain_pages < MAX_DRAIN_PAGES {
            let (more_pushed, more_pulled, more_conflicts, more_downloaded, more_has_more) =
                self.sync_once(false, true, false)?;
            pushed += more_pushed;
            pulled += more_pulled;
            downloaded += more_downloaded;
            conflicts.extend(more_conflicts);
            has_more = more_has_more && more_downloaded > 0;
            drain_pages += 1;
        }

        let _ = self
            .db
            .set_setting("last_sync_downloaded", &downloaded.to_string());
        Ok((pushed, pulled, conflicts))
    }

    /// Pushes pending local/full-state logs without pulling remote events.
    ///
    /// Cloud backup/export uses this so a known-good local snapshot can be uploaded even when the
    /// remote event stream still contains older destructive updates that must not be applied.
    pub fn push_pending_only(&self) -> Result<usize> {
        let (pushed, _, _, _, _) = self.sync_once(true, false, false)?;
        Ok(pushed)
    }

    /// Atomically replaces this account's remote encrypted history with the queued full snapshot.
    pub fn replace_with_pending_snapshot(&self) -> Result<usize> {
        let (pushed, _, _, _, _) = self.sync_once(true, false, true)?;
        Ok(pushed)
    }

    // One page of sync. `sync()` calls this in pull-before-push order; when `push_local` is true
    // this normally performs a small post-push pull so the cursor catches any concurrent events.
    fn sync_once(
        &self,
        push_local: bool,
        pull_remote: bool,
        replace_snapshot: bool,
    ) -> Result<(usize, usize, Vec<String>, usize, bool)> {
        let mut conflicts = Vec::new();
        let mut pushed_count = 0;
        let mut pulled_count = 0;

        let pending = if push_local {
            self.db.compact_pending_sync_logs()?;
            self.db.get_pending_sync_logs()?
        } else {
            Vec::new()
        };
        let mut local_payload = Vec::new();
        // Rol por campaña resuelto una sola vez — se consulta por cada evento pendiente.
        let mut observer_routes: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();

        for (log_id, entity_type, entity_id, operation, timestamp) in pending {
            let uuid = Uuid::parse_str(&entity_id).unwrap_or_default();
            let content = match entity_type.as_str() {
                "user" => self
                    .db
                    .get_user()
                    .ok()
                    .and_then(|u| u)
                    .and_then(|u| serde_json::to_string(&u).ok()),
                "task" => self
                    .db
                    .get_task_by_id(uuid)
                    .ok()
                    .and_then(|t| serde_json::to_string(&t).ok()),
                "campaign_treasury" => crate::services::TreasuryService::new(self.db)
                    .get_campaign(uuid)
                    .ok()
                    .flatten()
                    .and_then(|value| serde_json::to_string(&value).ok()),
                "ledger_entry" => crate::services::TreasuryService::new(self.db)
                    .get_entry(uuid)
                    .ok()
                    .flatten()
                    .and_then(|value| serde_json::to_string(&value).ok()),
                "task_financials" => crate::services::TreasuryService::new(self.db)
                    .get_task_financials(uuid)
                    .ok()
                    .flatten()
                    .and_then(|value| serde_json::to_string(&value).ok()),
                "ledger_category" => self
                    .db
                    .conn
                    .query_row(
                        "SELECT campaign_id FROM ledger_categories WHERE id=?1",
                        params![entity_id],
                        |row| row.get::<_, String>(0),
                    )
                    .ok()
                    .and_then(|campaign_id| Uuid::parse_str(&campaign_id).ok())
                    .and_then(|campaign_id| {
                        crate::services::TreasuryService::new(self.db)
                            .categories(campaign_id)
                            .ok()
                    })
                    .and_then(|categories| {
                        categories
                            .into_iter()
                            .find(|category| category.id.to_string() == entity_id)
                    })
                    .and_then(|category| serde_json::to_string(&category).ok()),
                "category_budget" => self
                    .db
                    .conn
                    .query_row(
                        "SELECT json_object('category_id', category_id, 'campaign_id', campaign_id,
                     'amount_minor', amount_minor, 'version', version, 'created_at', created_at,
                     'updated_at', updated_at) FROM category_budgets WHERE category_id=?1",
                        params![entity_id],
                        |row| row.get::<_, String>(0),
                    )
                    .ok(),
                "note" => self
                    .db
                    .get_note_by_id(uuid)
                    .ok()
                    .and_then(|n| serde_json::to_string(&n).ok()),
                "project" => {
                    let mut stmt = self.db.conn.prepare("SELECT id, name, description, created_at, updated_at, archived, completed, owner_identity, owner_username, is_shared FROM projects WHERE id = ?1")?;
                    stmt.query_row(params![entity_id], |row| {
                        let id_str: String = row.get(0)?;
                        let name: String = row.get(1)?;
                        let desc: Option<String> = row.get(2)?;
                        let created: String = row.get(3)?;
                        let updated: String = row.get(4)?;
                        let archived: i32 = row.get(5)?;
                        let completed: i32 = row.get(6)?;
                        let owner_id: Option<String> = row.get(7)?;
                        let owner_name: Option<String> = row.get(8)?;
                        let is_shared: i32 = row.get(9)?;
                        Ok(serde_json::json!({
                            "id": id_str,
                            "name": name,
                            "description": desc,
                            "created_at": created,
                            "updated_at": updated,
                            "archived": archived != 0,
                            "completed": completed != 0,
                            "owner_identity": owner_id,
                            "owner_username": owner_name,
                            "is_shared": is_shared != 0,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "journal_entry" => {
                    let mut stmt = self.db.conn.prepare("SELECT id, project_id, entry_date, content, created_at, updated_at, visibility, author_username FROM journal_entries WHERE id = ?1")?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let pid: String = row.get(1)?;
                        let date: String = row.get(2)?;
                        let content: String = row.get(3)?;
                        let created: String = row.get(4)?;
                        let updated: String = row.get(5)?;
                        let visibility: String = row.get(6)?;
                        let author: String = row.get(7)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "project_id": pid,
                            "entry_date": date,
                            "content": content,
                            "created_at": created,
                            "updated_at": updated,
                            "visibility": visibility,
                            "author_username": author,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "milestone" => {
                    let mut stmt = self.db.conn.prepare("SELECT id, project_id, name, description, completed, xp_reward, created_at, updated_at, tier, template_id FROM milestones WHERE id = ?1")?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let pid: String = row.get(1)?;
                        let name: String = row.get(2)?;
                        let desc: Option<String> = row.get(3)?;
                        let completed: i32 = row.get(4)?;
                        let xp: i32 = row.get(5)?;
                        let created: String = row.get(6)?;
                        let updated: String = row.get(7)?;
                        let tier: i32 = row.get(8)?;
                        let template_id: String = row.get(9)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "project_id": pid,
                            "name": name,
                            "description": desc,
                            "completed": completed != 0,
                            "xp_reward": xp,
                            "created_at": created,
                            "updated_at": updated,
                            "tier": tier,
                            "template_id": template_id,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "lore_unlock" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT id, unlocked, unlocked_at FROM lore_library WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let unlocked: i32 = row.get(1)?;
                        let unlocked_at: Option<String> = row.get(2)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "unlocked": unlocked != 0,
                            "unlocked_at": unlocked_at,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "achievement" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT id, name, description, unlocked_at FROM achievements WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let name: String = row.get(1)?;
                        let desc: String = row.get(2)?;
                        let unlocked: Option<String> = row.get(3)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "name": name,
                            "description": desc,
                            "unlocked_at": unlocked,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "ritual" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT id, name, description, frequency, reward_xp, created_at, updated_at, daily_target FROM rituals WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let name: String = row.get(1)?;
                        let desc: Option<String> = row.get(2)?;
                        let freq: String = row.get(3)?;
                        let xp: i32 = row.get(4)?;
                        let created: String = row.get(5)?;
                        let updated: String = row.get(6)?;
                        let daily_target: i32 = row.get(7)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "name": name,
                            "description": desc,
                            "frequency": freq,
                            "reward_xp": xp,
                            "created_at": created,
                            "updated_at": updated,
                            "daily_target": daily_target,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "ritual_history" => {
                    // Formato compuesto: "{ritual_id}__{completed_date}" — hay que partirlo
                    let parts: Vec<&str> = entity_id.splitn(2, "__").collect();
                    if parts.len() == 2 {
                        let row = self
                            .db
                            .conn
                            .query_row(
                                "SELECT completion_count, completed_at FROM ritual_history WHERE ritual_id = ?1 AND completed_date = ?2",
                                params![parts[0], parts[1]],
                                |row| Ok((row.get::<_, i32>(0)?, row.get::<_, Option<String>>(1)?)),
                            )
                            .ok();
                        let (completion_count, completed_at) = row.unwrap_or((1, None));
                        Some(
                            serde_json::json!({
                                "ritual_id": parts[0],
                                "completed_date": parts[1],
                                "completion_count": completion_count,
                                "completed_at": completed_at,
                            })
                            .to_string(),
                        )
                    } else {
                        None
                    }
                }
                "setting"
                    if crate::database::SYNCED_STREAK_SETTING_KEYS
                        .contains(&entity_id.as_str()) =>
                {
                    self.db.get_setting(&entity_id).ok().flatten().map(|value| {
                        serde_json::json!({
                            "key": entity_id,
                            "value": value,
                        })
                        .to_string()
                    })
                }
                "codex" => self
                    .db
                    .get_codex_by_id(&entity_id)
                    .ok()
                    .and_then(|c| serde_json::to_string(&c).ok()),
                "task_assignment" => {
                    // Formato compuesto: "task_id__user_identity"
                    let parts: Vec<&str> = entity_id.splitn(2, "__").collect();
                    if parts.len() == 2 {
                        let (tid, uid) = (parts[0], parts[1]);
                        let mut stmt = self.db.conn.prepare(
                            "SELECT ta.task_id, ta.user_identity, ta.user_username, t.project_id FROM task_assignments ta JOIN tasks t ON ta.task_id = t.id WHERE ta.task_id = ?1 AND ta.user_identity = ?2",
                        )?;
                        stmt.query_row(params![tid, uid], |row| {
                            let task_id: String = row.get(0)?;
                            let identity: String = row.get(1)?;
                            let username: String = row.get(2)?;
                            let project_id: Option<String> = row.get(3)?;
                            Ok(serde_json::json!({
                                "task_id": task_id,
                                "user_identity": identity,
                                "user_username": username,
                                "project_id": project_id,
                            })
                            .to_string())
                        })
                        .ok()
                    } else {
                        None
                    }
                }
                "task_status" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT task_id, project_id, status, changed_by_identity, changed_by_username, updated_at
                         FROM task_statuses WHERE task_id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        Ok(serde_json::json!({
                            "task_id": row.get::<_, String>(0)?,
                            "project_id": row.get::<_, String>(1)?,
                            "status": row.get::<_, String>(2)?,
                            "changed_by_identity": row.get::<_, String>(3)?,
                            "changed_by_username": row.get::<_, String>(4)?,
                            "updated_at": row.get::<_, String>(5)?,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "task_comment" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT id, task_id, project_id, author_identity, author_username, content,
                                mentioned_identities, created_at, updated_at, edited_at, deleted_at
                         FROM task_comments WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        Ok(serde_json::json!({
                            "id": row.get::<_, String>(0)?,
                            "task_id": row.get::<_, String>(1)?,
                            "project_id": row.get::<_, String>(2)?,
                            "author_identity": row.get::<_, String>(3)?,
                            "author_username": row.get::<_, String>(4)?,
                            "content": row.get::<_, String>(5)?,
                            "mentioned_identities": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(6)?).unwrap_or_else(|_| serde_json::json!([])),
                            "created_at": row.get::<_, String>(7)?,
                            "updated_at": row.get::<_, String>(8)?,
                            "edited_at": row.get::<_, Option<String>>(9)?,
                            "deleted_at": row.get::<_, Option<String>>(10)?,
                        }).to_string())
                    }).ok()
                }
                "task_dependency" => {
                    let parts: Vec<&str> = entity_id.splitn(2, "__").collect();
                    if parts.len() != 2 {
                        None
                    } else {
                        let mut stmt = self.db.conn.prepare(
                            "SELECT task_id, depends_on_task_id, project_id, created_by_identity,
                                    created_by_username, created_at
                             FROM task_dependencies WHERE task_id = ?1 AND depends_on_task_id = ?2",
                        )?;
                        stmt.query_row(params![parts[0], parts[1]], |row| {
                            Ok(serde_json::json!({
                                "task_id": row.get::<_, String>(0)?,
                                "depends_on_task_id": row.get::<_, String>(1)?,
                                "project_id": row.get::<_, String>(2)?,
                                "created_by_identity": row.get::<_, String>(3)?,
                                "created_by_username": row.get::<_, String>(4)?,
                                "created_at": row.get::<_, String>(5)?,
                            })
                            .to_string())
                        })
                        .ok()
                    }
                }
                "notification_state" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT notification_id, read, updated_at FROM notification_states WHERE notification_id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        Ok(serde_json::json!({
                            "notification_id": row.get::<_, String>(0)?,
                            "read": row.get::<_, i32>(1)? != 0,
                            "updated_at": row.get::<_, String>(2)?,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "project_member" => {
                    // Formato compuesto: "project_id__user_identity"
                    let parts: Vec<&str> = entity_id.splitn(2, "__").collect();
                    if parts.len() == 2 {
                        let (pid, uid) = (parts[0], parts[1]);
                        let mut stmt = self.db.conn.prepare(
                            "SELECT project_id, user_identity, user_username, role FROM project_members WHERE project_id = ?1 AND user_identity = ?2",
                        )?;
                        stmt.query_row(params![pid, uid], |row| {
                            let project_id: String = row.get(0)?;
                            let identity: String = row.get(1)?;
                            let username: String = row.get(2)?;
                            let role: String = row.get(3)?;
                            Ok(serde_json::json!({
                                "project_id": project_id,
                                "user_identity": identity,
                                "user_username": username,
                                "role": role,
                            })
                            .to_string())
                        })
                        .ok()
                    } else {
                        None
                    }
                }
                "project_key" => self
                    .db
                    .get_project_encryption_key(&entity_id)
                    .ok()
                    .flatten()
                    .map(|(routing_id, key)| {
                        serde_json::json!({
                            "project_id": entity_id,
                            "routing_id": routing_id,
                            "key_hex": key.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                        })
                        .to_string()
                    }),
                "chronicle_message" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT id, project_id, sender_identity, sender_username, content, message_type, timestamp FROM chronicle_messages WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let project_id: Option<String> = row.get(1)?;
                        let sender_id: String = row.get(2)?;
                        let sender_name: String = row.get(3)?;
                        let content: String = row.get(4)?;
                        let msg_type: String = row.get(5)?;
                        let timestamp: String = row.get(6)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "project_id": project_id,
                            "sender_identity": sender_id,
                            "sender_username": sender_name,
                            "content": content,
                            "message_type": msg_type,
                            "timestamp": timestamp,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "focus_session" => {
                    let mut stmt = self.db.conn.prepare(
                        "SELECT id, project_id, task_id, duration_mins, xp_gained, completed_at, soundscape, owner_identity FROM focus_sessions WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let proj: Option<String> = row.get(1)?;
                        let task: Option<String> = row.get(2)?;
                        let duration: i32 = row.get(3)?;
                        let xp: i32 = row.get(4)?;
                        let completed_at: String = row.get(5)?;
                        let soundscape: String = row.get(6)?;
                        let owner_identity: Option<String> = row.get(7)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "project_id": proj,
                            "task_id": task,
                            "duration_mins": duration,
                            "xp_gained": xp,
                            "completed_at": completed_at,
                            "soundscape": soundscape,
                            "owner_identity": owner_identity,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "zen_tree" => self
                    .db
                    .get_zen_tree()
                    .ok()
                    .and_then(|t| serde_json::to_string(&t).ok()),
                _ => None,
            };

            // Deletes no longer have a live row. Revisions retain the last complete
            // value, which lets us recover the Fellowship route for the tombstone.
            let plaintext = content.unwrap_or_else(|| {
                self.db
                    .conn
                    .query_row(
                        "SELECT content FROM revisions WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY revision_number DESC LIMIT 1",
                        params![entity_type, entity_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| "null".to_string())
            });
            let project_id = project_id_from_sync_content(&entity_type, &entity_id, &plaintext);
            let project_encryption = project_id.as_deref().and_then(|project_id| {
                if operation == "delete" {
                    self.db
                        .get_project_encryption_key_for_tombstone(project_id)
                        .ok()
                        .flatten()
                } else {
                    self.db
                        .get_project_encryption_key(project_id)
                        .ok()
                        .flatten()
                }
            });
            // El servidor rechaza con 403 cualquier escritura de un Observer sobre una ruta
            // compartida, y ese rechazo revierte el lote completo: un solo evento así
            // dejaría el sync atascado para todo lo demás. Se queda local y pendiente,
            // por si el rol cambia más adelante.
            if project_encryption.is_some()
                && entity_type != "chronicle_message"
                && project_id
                    .as_deref()
                    .and_then(|project_id| {
                        observer_routes
                            .entry(project_id.to_string())
                            .or_insert_with(|| {
                                self.db
                                    .get_member_role(project_id, &self.identity.public_key)
                                    .ok()
                                    .flatten()
                                    .as_deref()
                                    == Some("Observer")
                            })
                            .then_some(())
                    })
                    .is_some()
            {
                continue;
            }
            let (key_id, scope, routing_id) = match &project_encryption {
                Some((routing_id, _)) => ("project-v1", "project", routing_id.clone()),
                None => (
                    crate::services::encryption::KEY_ID,
                    "account",
                    String::new(),
                ),
            };
            let aad = if scope == "project" {
                crate::services::encryption::associated_data_for_route(
                    &log_id,
                    &entity_type,
                    &entity_id,
                    &operation,
                    &timestamp,
                    key_id,
                    scope,
                    &routing_id,
                )
            } else {
                crate::services::encryption::associated_data_for(
                    &log_id,
                    &entity_type,
                    &entity_id,
                    &operation,
                    &timestamp,
                    key_id,
                )
            };
            let (nonce, ciphertext) = match project_encryption {
                Some((_, key)) => {
                    crate::services::encryption::encrypt_with_project_key(&key, &plaintext, &aad)?
                }
                None => crate::services::encryption::encrypt(self.identity, &plaintext, &aad)?,
            };
            local_payload.push(SyncLogEntry {
                version: crate::services::encryption::SYNC_VERSION,
                id: log_id,
                entity_type,
                entity_id,
                operation,
                timestamp,
                content: None,
                key_id: key_id.to_string(),
                nonce,
                ciphertext,
                scope: scope.to_string(),
                routing_id,
                device_id: self.device_id.to_string(),
                author_public_key: self.identity.public_key.clone(),
                event_signature: String::new(),
                seq: 0, // el servidor sobreescribe esto con el seq real al insertar
            });
        }

        // Heartbeat del dispositivo — lleva identidad del usuario para actualizar presencia en otros nodos
        if push_local {
            let now_str = Utc::now().to_rfc3339();
            let username = self
                .db
                .get_user()
                .ok()
                .and_then(|u| u)
                .map(|u| u.username)
                .unwrap_or_else(|| "Unknown".to_string());
            let hostname = std::env::var("HOSTNAME")
                .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
                .unwrap_or_else(|_| "Unknown Node".to_string());
            let device_info = serde_json::json!({
                "device_id": self.device_id,
                "device_name": hostname,
                "last_sync": now_str,
                "user_identity": self.identity.public_key,
                "username": username,
            })
            .to_string();
            let heartbeat_id = format!(
                "device_heartbeat__{}__{}",
                self.device_id,
                Utc::now().format("%Y%m%d_%H%M")
            );
            let aad = crate::services::encryption::associated_data(
                &heartbeat_id,
                "device",
                self.device_id,
                "heartbeat",
                &now_str,
            );
            let (nonce, ciphertext) =
                crate::services::encryption::encrypt(self.identity, &device_info, &aad)?;
            local_payload.push(SyncLogEntry {
                version: crate::services::encryption::SYNC_VERSION,
                id: heartbeat_id,
                entity_type: "device".to_string(),
                entity_id: self.device_id.to_string(),
                operation: "heartbeat".to_string(),
                timestamp: now_str,
                content: None,
                key_id: crate::services::encryption::KEY_ID.to_string(),
                nonce,
                ciphertext,
                scope: "account".to_string(),
                routing_id: String::new(),
                device_id: self.device_id.to_string(),
                author_public_key: self.identity.public_key.clone(),
                event_signature: String::new(),
                seq: 0,
            });
        }

        for event in &mut local_payload {
            let version = event.version.to_string();
            let message = crate::services::encryption::event_signature_message(&[
                &version,
                &event.id,
                &event.entity_type,
                &event.entity_id,
                &event.operation,
                &event.timestamp,
                &event.key_id,
                &event.nonce,
                &event.ciphertext,
                &event.scope,
                &event.routing_id,
                &event.device_id,
                &event.author_public_key,
            ]);
            event.event_signature = self.identity.sign(&message)?;
        }

        // Si falla el push, no jalamos nada — así no pisamos cambios que aún no subimos
        let pushed_ids: Vec<String> = if !local_payload.is_empty() {
            let serialized = serde_json::to_string(&local_payload)?;
            let signature = self.identity.sign(serialized.as_bytes())?;
            if replace_snapshot {
                self.provider.replace_snapshot(
                    &self.identity.public_key,
                    &signature,
                    &serialized,
                )?;
            } else {
                self.provider
                    .push(&self.identity.public_key, &signature, &serialized)?;
            }
            pushed_count = local_payload.len();
            local_payload.iter().map(|l| l.id.clone()).collect()
        } else {
            Vec::new()
        };

        if !pull_remote {
            if !pushed_ids.is_empty() {
                self.db.mark_sync_logs_synced(&pushed_ids)?;
            }
            self.db.update_device_sync_time(self.device_id)?;
            return Ok((pushed_count, 0, Vec::new(), 0, false));
        }

        // El cursor `since_seq` evita descargar toda la historia en cada sync
        let since_seq: i64 = self
            .db
            .get_setting("last_pull_seq_v2")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let pulled_data = self
            .provider
            .pull(&self.identity.public_key, "", since_seq)?;
        let pull_page = parse_pull_page(&pulled_data, since_seq)?;
        let remote_downloaded = pull_page.events.len();
        let remote_head_seq = pull_page.head_seq;
        let remote_next_seq = pull_page.next_seq;
        let metadata_supported = pull_page.metadata_supported;
        let mut remote_has_more = pull_page.has_more;
        let mut remote_logs = pull_page.events;
        for log in &mut remote_logs {
            if !self.provider.requires_encryption() {
                continue;
            }
            if log.version != crate::services::encryption::SYNC_VERSION
                || !matches!(log.key_id.as_str(), "account-v1" | "project-v1")
                || log.nonce.is_empty()
                || log.ciphertext.is_empty()
            {
                return Err(anyhow!(
                    "server returned a non-encrypted or unsupported sync event {}",
                    log.id
                ));
            }
            let has_signature =
                log.author_public_key.len() == 64 && log.event_signature.len() == 128;
            if pull_page.signatures_required && !has_signature {
                return Err(anyhow!(
                    "server returned unsigned event {} after signature cutover",
                    log.id
                ));
            }
            if has_signature {
                let version = log.version.to_string();
                let signed = crate::services::encryption::event_signature_message(&[
                    &version,
                    &log.id,
                    &log.entity_type,
                    &log.entity_id,
                    &log.operation,
                    &log.timestamp,
                    &log.key_id,
                    &log.nonce,
                    &log.ciphertext,
                    &log.scope,
                    &log.routing_id,
                    &log.device_id,
                    &log.author_public_key,
                ]);
                if !Identity::verify(&signed, &log.author_public_key, &log.event_signature)? {
                    return Err(anyhow!(
                        "invalid durable signature on sync event {}",
                        log.id
                    ));
                }
            }
            let aad = if log.scope == "project" {
                crate::services::encryption::associated_data_for_route(
                    &log.id,
                    &log.entity_type,
                    &log.entity_id,
                    &log.operation,
                    &log.timestamp,
                    &log.key_id,
                    &log.scope,
                    &log.routing_id,
                )
            } else {
                crate::services::encryption::associated_data_for(
                    &log.id,
                    &log.entity_type,
                    &log.entity_id,
                    &log.operation,
                    &log.timestamp,
                    &log.key_id,
                )
            };
            log.content = Some(if log.key_id == "project-v1" {
                let (_, key) = self
                    .db
                    .get_project_encryption_key_by_routing_id(&log.routing_id)?
                    .ok_or_else(|| {
                        anyhow!("missing Fellowship key for routing id {}", log.routing_id)
                    })?;
                crate::services::encryption::decrypt_with_project_key(
                    &key,
                    &log.nonce,
                    &log.ciphertext,
                    &aad,
                )?
            } else {
                crate::services::encryption::decrypt(
                    self.identity,
                    &log.nonce,
                    &log.ciphertext,
                    &aad,
                )?
            });
        }

        // Dedup por ID — si el servidor no retorna seq reales (todos llegan con seq=0), este set
        // evita reprocesar eventos que ya aplicamos en sesiones anteriores, sea cual sea el cursor
        let already_processed = self.db.load_processed_remote_ids().unwrap_or_default();
        let mut newly_processed_ids: Vec<String> = Vec::new();
        let remote_task_projects = remote_logs
            .iter()
            .filter(|log| {
                !already_processed.contains(&log.id)
                    && log.device_id != self.device_id
                    && log.entity_type == "task"
                    && log.operation != "delete"
            })
            .filter_map(|log| {
                log.content
                    .as_ref()
                    .and_then(|content| serde_json::from_str::<Task>(content).ok())
            })
            .map(|task| (task.id, task.project_id))
            .collect::<std::collections::HashMap<_, _>>();

        let mut destructive_note_unlinks = 0usize;
        let mut destructive_task_unlinks = 0usize;
        let mut invalid_note_payloads = 0usize;
        let mut invalid_task_payloads = 0usize;
        for log in &remote_logs {
            if already_processed.contains(&log.id) || log.device_id == self.device_id {
                continue;
            }
            let Some(content) = log.content.as_ref() else {
                continue;
            };
            match log.entity_type.as_str() {
                "note" if log.operation != "delete" => {
                    if let Ok(remote_note) = serde_json::from_str::<Note>(content) {
                        if remote_note.project_id.is_none() {
                            invalid_note_payloads += 1;
                            if let Ok(local_note) = self.db.get_note_by_id(remote_note.id) {
                                if local_note.project_id.is_some()
                                    && remote_note.project_id.is_none()
                                {
                                    destructive_note_unlinks += 1;
                                }
                            }
                        }
                    }
                }
                "task" if log.operation != "delete" => {
                    if let Ok(remote_task) = serde_json::from_str::<Task>(content) {
                        if !self.task_payload_project_is_valid(&remote_task, &remote_task_projects)
                        {
                            invalid_task_payloads += 1;
                            if let Ok(local_task) = self.db.get_task_by_id(remote_task.id) {
                                if local_task.project_id.is_some() {
                                    destructive_task_unlinks += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if destructive_note_unlinks >= 5
            || destructive_task_unlinks >= 5
            || invalid_note_payloads >= 5
            || invalid_task_payloads >= 5
        {
            let quarantined_to = remote_next_seq.max(
                remote_logs
                    .iter()
                    .map(|log| log.seq)
                    .max()
                    .unwrap_or(since_seq),
            );
            let last_remote_head_seq = remote_head_seq.max(quarantined_to);
            let _ = self
                .db
                .set_setting("last_remote_head_seq", &last_remote_head_seq.to_string());
            let lag = last_remote_head_seq.saturating_sub(since_seq);
            let _ = self.db.set_setting("last_sync_lag", &lag.to_string());
            let _ = self.db.set_setting("sync_restore_hold", "1");
            let _ = self.db.set_setting("auto_sync", "false");
            let _ = self.db.set_setting(
                "last_quarantined_remote_page",
                &format!(
                    "Quarantined remote page at seq {}..{} because it contained {} invalid scrolls, {} invalid tasks, and would unlink {} scrolls and {} tasks. Cursor held at {}; reset cloud from a clean device.",
                    since_seq,
                    quarantined_to,
                    invalid_note_payloads,
                    invalid_task_payloads,
                    destructive_note_unlinks,
                    destructive_task_unlinks,
                    since_seq
                ),
            );
            conflicts.push(format!(
                "Remote sync page quarantined: {} invalid scrolls, {} invalid tasks",
                invalid_note_payloads, invalid_task_payloads
            ));
            return Ok((
                pushed_count,
                pulled_count,
                conflicts,
                remote_downloaded,
                remote_has_more && remote_downloaded > 0,
            ));
        }

        // Estrategia de conflictos: Latest Edit Wins, con la versión perdedora guardada en revisiones
        let mut max_seq: i64 = since_seq;
        let mut retry_from_seq: Option<i64> = None;
        for log in remote_logs {
            if log.seq > max_seq {
                max_seq = log.seq;
            }
            // Ignorar eventos que generamos nosotros mismos — ya los tenemos localmente
            if log.device_id == self.device_id {
                // Marcar como procesados para que no los replays si llegan de vuelta del server
                if !already_processed.contains(&log.id) {
                    newly_processed_ids.push(log.id);
                }
                continue;
            }
            // Saltar eventos que ya aplicamos en un sync anterior — end del ciclo de replay infinito
            if already_processed.contains(&log.id) {
                continue;
            }

            let ent_uuid = Uuid::parse_str(&log.entity_id).unwrap_or_default();
            let is_newer = match log.entity_type.as_str() {
                // Solo aceptamos usuario remoto si no existe localmente — restauración en dispositivo nuevo
                "user" => self.db.get_user().map(|u| u.is_none()).unwrap_or(true),
                "task" => {
                    if log.operation == "delete" {
                        match self.db.get_task_by_id(ent_uuid) {
                            Ok(local_task) => {
                                let incoming_time = DateTime::parse_from_rfc3339(&log.timestamp)
                                    .map(|d| d.with_timezone(&Utc))
                                    .unwrap_or(DateTime::<Utc>::from(std::time::UNIX_EPOCH));
                                incoming_time > local_task.updated_at
                            }
                            _ => true,
                        }
                    } else {
                        match self.db.get_task_by_id(ent_uuid) {
                            Ok(local_task) => {
                                if let Some(ref content) = log.content {
                                    if let Ok(mut remote_task) =
                                        serde_json::from_str::<Task>(content)
                                    {
                                        remote_task.normalize_schedule();
                                        if local_task.project_id.is_some()
                                            && remote_task.project_id.is_none()
                                        {
                                            false
                                        } else if remote_task.updated_at > local_task.updated_at {
                                            if remote_task.title != local_task.title
                                                || remote_task.completed != local_task.completed
                                                || remote_task.project_id != local_task.project_id
                                                || remote_task.parent_task_id
                                                    != local_task.parent_task_id
                                            {
                                                if let Ok(local_json) =
                                                    serde_json::to_string(&local_task)
                                                {
                                                    let _ = self.db.create_revision(
                                                        "task",
                                                        &log.entity_id,
                                                        &local_json,
                                                    );
                                                }
                                                conflicts.push(format!(
                                            "Task conflict: '{}' resolved using Latest Edit Wins",
                                            local_task.title
                                        ));
                                            }
                                            true
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                            _ => log
                                .content
                                .as_ref()
                                .and_then(|content| serde_json::from_str::<Task>(content).ok())
                                .map(|remote_task| {
                                    self.task_payload_project_is_valid(
                                        &remote_task,
                                        &remote_task_projects,
                                    )
                                })
                                .unwrap_or(false),
                        }
                    }
                }
                // Notas: mismo juego que tasks — timestamp gana, pero guardamos la versión local si hay conflicto
                "note" => {
                    if log.operation == "delete" {
                        match self.db.get_note_by_id(ent_uuid) {
                            Ok(local_note) => {
                                let incoming_time = DateTime::parse_from_rfc3339(&log.timestamp)
                                    .map(|d| d.with_timezone(&Utc))
                                    .unwrap_or(DateTime::<Utc>::from(std::time::UNIX_EPOCH));
                                incoming_time > local_note.updated_at
                            }
                            _ => true,
                        }
                    } else {
                        match self.db.get_note_by_id(ent_uuid) {
                            Ok(local_note) => {
                                if let Some(ref content) = log.content {
                                    if let Ok(remote_note) = serde_json::from_str::<Note>(content) {
                                        if (local_note.project_id.is_some()
                                            && remote_note.project_id.is_none())
                                            || (local_note.codex_id.is_some()
                                                && remote_note.codex_id.is_none())
                                        {
                                            false
                                        } else if remote_note.updated_at > local_note.updated_at {
                                            if remote_note.title != local_note.title
                                                || remote_note.markdown_content
                                                    != local_note.markdown_content
                                                || remote_note.codex_id != local_note.codex_id
                                                || remote_note.project_id != local_note.project_id
                                            {
                                                if let Ok(local_json) =
                                                    serde_json::to_string(&local_note)
                                                {
                                                    let _ = self.db.create_revision(
                                                        "note",
                                                        &log.entity_id,
                                                        &local_json,
                                                    );
                                                }
                                                conflicts.push(format!(
                                            "Note conflict: '{}' resolved using Latest Edit Wins",
                                            local_note.title
                                        ));
                                            }
                                            true
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                            _ => log
                                .content
                                .as_ref()
                                .and_then(|content| serde_json::from_str::<Note>(content).ok())
                                .map(|remote_note| remote_note.project_id.is_some())
                                .unwrap_or(false),
                        }
                    }
                }
                "project" => self.incoming_entity_is_newer("projects", &log),
                "journal_entry" => self.incoming_entity_is_newer("journal_entries", &log),
                // Logros: solo se sincronizan si no están ya desbloqueados — no se revierten
                "achievement" => {
                    let unlocked = self
                        .db
                        .get_achievements()
                        .map(|list| {
                            list.iter()
                                .any(|a| a.id == log.entity_id && a.unlocked_at.is_some())
                        })
                        .unwrap_or(false);
                    !unlocked
                }
                "ritual" => self.incoming_entity_is_newer("rituals", &log),
                "ritual_history" => true,
                "setting" => {
                    crate::database::SYNCED_STREAK_SETTING_KEYS.contains(&log.entity_id.as_str())
                }
                "codex" => self.incoming_entity_is_newer("codices", &log),
                // Las sesiones de focus son inmutables una vez completadas — nunca se actualizan
                "focus_session" => {
                    self.db
                        .conn
                        .query_row(
                            "SELECT count(*) FROM focus_sessions WHERE id = ?1",
                            params![log.entity_id],
                            |row| row.get::<_, i32>(0),
                        )
                        .unwrap_or(0)
                        == 0
                }
                "task_assignment" => true,
                "task_status" => self.incoming_task_status_is_newer(&log),
                "task_comment" => self.incoming_entity_is_newer("task_comments", &log),
                "task_dependency" => true,
                "notification_state" => self.incoming_notification_state_is_newer(&log),
                "project_member" => true,
                "project_key" => true,
                // Los mensajes de la crónica son inmutables — nunca se editan, solo se insertan
                "chronicle_message" => {
                    self.db
                        .conn
                        .query_row(
                            "SELECT count(*) FROM chronicle_messages WHERE id = ?1",
                            params![log.entity_id],
                            |row| row.get::<_, i32>(0),
                        )
                        .unwrap_or(0)
                        == 0
                }
                // Lore: los desbloqueos no se revierten — solo aplicamos si el remoto dice desbloqueado y el local aún no
                "lore_unlock" => {
                    let remote_unlocked = log
                        .content
                        .as_ref()
                        .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                        .map(|v| v["unlocked"].as_bool().unwrap_or(false))
                        .unwrap_or(false);
                    if !remote_unlocked {
                        false
                    } else {
                        self.db
                            .conn
                            .query_row(
                                "SELECT unlocked FROM lore_library WHERE id = ?1",
                                params![log.entity_id],
                                |row| row.get::<_, i32>(0),
                            )
                            .unwrap_or(1)
                            == 0
                    }
                }
                // Árbol zen: latest-watered timestamp wins — es una sola fila global por usuario
                "zen_tree" => {
                    if let Some(ref content) = log.content {
                        if let Ok(remote_tree) =
                            serde_json::from_str::<crate::models::ZenTree>(content)
                        {
                            match self.db.get_zen_tree() {
                                Ok(local_tree) => {
                                    remote_tree.growth > local_tree.growth
                                        || remote_tree.stage > local_tree.stage
                                        || remote_tree.total_waterings > local_tree.total_waterings
                                        || remote_tree
                                            .last_watered
                                            .zip(local_tree.last_watered)
                                            .map(|(remote, local)| remote > local)
                                            .unwrap_or(remote_tree.last_watered.is_some())
                                }
                                Err(_) => true,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                "milestone" => self.incoming_entity_is_newer("milestones", &log),
                "campaign_treasury" => self.incoming_entity_is_newer("campaign_treasury", &log),
                "ledger_entry" => self.incoming_entity_is_newer("ledger_entries", &log),
                "ledger_category" => self.incoming_entity_is_newer("ledger_categories", &log),
                "category_budget" => self.incoming_entity_is_newer("category_budgets", &log),
                "task_financials" => self.incoming_entity_is_newer("task_financials", &log),
                // Dispositivos: siempre aplicamos — upsert idempotente
                "device" => true,
                _ => false,
            };

            if is_newer {
                if log.operation == "delete" {
                    match log.entity_type.as_str() {
                        "task" => {
                            self.save_local_revision_before_delete("task", &log.entity_id);
                            let _ = self
                                .db
                                .conn
                                .execute("DELETE FROM tasks WHERE id = ?1", params![log.entity_id]);
                            pulled_count += 1;
                        }
                        "note" => {
                            self.save_local_revision_before_delete("note", &log.entity_id);
                            let _ = self
                                .db
                                .conn
                                .execute("DELETE FROM notes WHERE id = ?1", params![log.entity_id]);
                            pulled_count += 1;
                        }
                        "project" => {
                            self.save_local_revision_before_delete("project", &log.entity_id);
                            let _ = self.db.conn.execute(
                                "DELETE FROM projects WHERE id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "milestone" => {
                            self.save_local_revision_before_delete("milestone", &log.entity_id);
                            let _ = self.db.conn.execute(
                                "DELETE FROM milestones WHERE id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "codex" => {
                            self.save_local_revision_before_delete("codex", &log.entity_id);
                            let _ = self.db.conn.execute(
                                "DELETE FROM codices WHERE id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "ledger_entry" => {
                            self.save_local_revision_before_delete("ledger_entry", &log.entity_id);
                            let _ = self.db.conn.execute(
                                "DELETE FROM ledger_entries WHERE id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "task_assignment" => {
                            let parts: Vec<&str> = log.entity_id.splitn(2, "__").collect();
                            if parts.len() == 2 {
                                let task_title = uuid::Uuid::parse_str(parts[0])
                                    .ok()
                                    .and_then(|id| self.db.get_task_by_id(id).ok())
                                    .map(|task| task.title)
                                    .unwrap_or_else(|| "A shared Quest".to_string());
                                let _ = self.db.conn.execute(
                                    "DELETE FROM task_assignments WHERE task_id = ?1 AND user_identity = ?2",
                                    params![parts[0], parts[1]],
                                );
                                if parts[1] == self.identity.public_key.as_str() {
                                    self.create_task_fellowship_notification(
                                        &format!("task_unassigned:{}", log.id),
                                        "task_unassignment",
                                        "Released from Quest",
                                        &format!("You are no longer assigned to {}.", task_title),
                                        parts[0],
                                    );
                                }
                                pulled_count += 1;
                            }
                        }
                        "task_status" => {
                            let _ = self.db.conn.execute(
                                "DELETE FROM task_statuses WHERE task_id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "task_comment" => {
                            let _ = self.db.conn.execute(
                                "DELETE FROM task_comments WHERE id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "task_dependency" => {
                            let parts: Vec<&str> = log.entity_id.splitn(2, "__").collect();
                            if parts.len() == 2 {
                                let _ = self.db.conn.execute(
                                    "DELETE FROM task_dependencies WHERE task_id = ?1 AND depends_on_task_id = ?2",
                                    params![parts[0], parts[1]],
                                );
                                pulled_count += 1;
                            }
                        }
                        "notification_state" => {
                            let _ = self.db.conn.execute(
                                "DELETE FROM notification_states WHERE notification_id = ?1",
                                params![log.entity_id],
                            );
                            pulled_count += 1;
                        }
                        "project_member" => {
                            let parts: Vec<&str> = log.entity_id.splitn(2, "__").collect();
                            if parts.len() == 2 {
                                let _ = self.db.conn.execute(
                                    "DELETE FROM project_members WHERE project_id = ?1 AND user_identity = ?2",
                                    params![parts[0], parts[1]],
                                );
                                pulled_count += 1;
                            }
                        }
                        _ => {}
                    }
                    newly_processed_ids.push(log.id);
                    continue;
                }

                if let Some(ref content) = log.content {
                    if log.entity_type == "project_key" {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
                            if let (Some(project_id), Some(routing_id), Some(key_hex)) = (
                                value["project_id"].as_str(),
                                value["routing_id"].as_str(),
                                value["key_hex"].as_str(),
                            ) {
                                let decoded = (0..key_hex.len())
                                    .step_by(2)
                                    .map(|i| u8::from_str_radix(&key_hex[i..i + 2], 16))
                                    .collect::<std::result::Result<Vec<_>, _>>();
                                if let Ok(bytes) = decoded {
                                    if let Ok(key) = <[u8; 32]>::try_from(bytes) {
                                        if self
                                            .db
                                            .save_project_encryption_key_from_sync(
                                                project_id, routing_id, &key,
                                            )
                                            .is_ok()
                                        {
                                            pulled_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                        newly_processed_ids.push(log.id);
                        continue;
                    }
                    match log.entity_type.as_str() {
                        "user" => {
                            if let Ok(mut u) = serde_json::from_str::<crate::models::User>(content)
                            {
                                // Progression is monotonic. An older device snapshot must never
                                // reduce level/XP earned on another device.
                                let mut preserved_local_progress = false;
                                if let Ok(Some(local)) = self.db.get_user() {
                                    if total_user_progress_xp(&local) > total_user_progress_xp(&u) {
                                        u.level = local.level;
                                        u.xp = local.xp;
                                        preserved_local_progress = true;
                                    }
                                }
                                let _ = self.db.conn.execute(
                                    "DELETE FROM users WHERE id != ?1",
                                    params![u.id.to_string()],
                                );
                                let _ = self.db.conn.execute(
                                    "INSERT INTO users (id, username, class, level, xp, created_at, specialization)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                                     ON CONFLICT(id) DO UPDATE SET
                                         username=excluded.username, class=excluded.class,
                                         level=excluded.level, xp=excluded.xp,
                                         created_at=excluded.created_at,
                                         specialization=excluded.specialization",
                                    params![
                                        u.id.to_string(),
                                        u.username,
                                        u.class.name(),
                                        u.level,
                                        u.xp,
                                        u.created_at.to_rfc3339(),
                                        u.specialization
                                    ],
                                );
                                if preserved_local_progress {
                                    let _ = self.db.log_change("user", &u.id.to_string(), "upsert");
                                }
                                pulled_count += 1;
                            }
                        }
                        "task" => {
                            if log.operation == "delete" {
                                self.save_local_revision_before_delete("task", &log.entity_id);
                                let _ = self.db.conn.execute(
                                    "DELETE FROM tasks WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(mut t) = serde_json::from_str::<Task>(content) {
                                t.normalize_schedule();
                                // Triquete de completado: si ya está completa localmente, no la regresamos a incompleta
                                let local_task = self.db.get_task_by_id(ent_uuid).ok();
                                if let Some(local) = local_task.as_ref() {
                                    if local.project_id.is_some() && t.project_id.is_none() {
                                        t.project_id = local.project_id;
                                    }
                                }
                                if !self.task_payload_project_is_valid(&t, &remote_task_projects) {
                                    conflicts.push(format!(
                                        "Task '{}' rejected: missing project link",
                                        t.title
                                    ));
                                    let _ = self.db.set_setting("auto_sync", "false");
                                    let _ = self.db.set_setting("sync_restore_hold", "1");
                                    newly_processed_ids.push(log.id);
                                    continue;
                                }
                                let local_completed =
                                    local_task.as_ref().map(|lt| lt.completed).unwrap_or(false);
                                let was_incomplete_locally = !local_completed;
                                let assigned_to_me = self
                                    .db
                                    .get_task_assignments(&t.id.to_string())
                                    .unwrap_or_default()
                                    .iter()
                                    .any(|(id, _)| id == self.identity.public_key.as_str());
                                if local_completed && !t.completed {
                                    pulled_count += 1;
                                } else {
                                    // Bypass insert_task() para no disparar log_change de nuevo — evitamos el loop
                                    let _ = self.db.conn.execute(
                                        "INSERT INTO tasks (id, project_id, title, description, due_date, set_date, completed, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded, recurrence)
                                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                                         ON CONFLICT(id) DO UPDATE SET
                                             project_id=excluded.project_id, title=excluded.title,
                                             description=excluded.description, due_date=excluded.due_date,
                                             set_date=excluded.set_date, completed=excluded.completed,
                                             priority=excluded.priority, created_at=excluded.created_at,
                                             updated_at=excluded.updated_at, owner_identity=excluded.owner_identity,
                                             owner_username=excluded.owner_username,
                                             parent_task_id=excluded.parent_task_id,
                                             xp_awarded=excluded.xp_awarded, recurrence=excluded.recurrence",
                                        params![
                                            t.id.to_string(),
                                            t.project_id.map(|id| id.to_string()),
                                            t.title.clone(),
                                            t.description.clone(),
                                            t.due_date.map(|d| d.to_rfc3339()),
                                            t.set_date.map(|d| d.to_rfc3339()),
                                            if t.completed { 1 } else { 0 },
                                            t.priority.name(),
                                            t.created_at.to_rfc3339(),
                                            t.updated_at.to_rfc3339(),
                                            t.owner_identity.clone(),
                                            t.owner_username.clone(),
                                            t.parent_task_id.map(|id| id.to_string()),
                                            if t.xp_awarded { 1 } else { 0 },
                                            t.recurrence.map(|r| r.name()),
                                        ],
                                    );
                                    // XP para el usuario local si está asignado y la tarea acaba de completarse
                                    if t.completed && was_incomplete_locally {
                                        let my_key = self.identity.public_key.as_str();
                                        for dependent_id in self
                                            .db
                                            .get_tasks_blocked_by(&t.id.to_string())
                                            .unwrap_or_default()
                                        {
                                            let dependent_assigned_to_me = self
                                                .db
                                                .get_task_assignments(&dependent_id)
                                                .unwrap_or_default()
                                                .iter()
                                                .any(|(id, _)| id == my_key);
                                            if dependent_assigned_to_me {
                                                let dependent_title =
                                                    Uuid::parse_str(&dependent_id)
                                                        .ok()
                                                        .and_then(|id| {
                                                            self.db.get_task_by_id(id).ok()
                                                        })
                                                        .map(|task| task.title)
                                                        .unwrap_or_else(|| {
                                                            "A dependent Quest".to_string()
                                                        });
                                                self.create_task_fellowship_notification(
                                                    &format!(
                                                        "dependency_resolved:{}:{}",
                                                        log.id, dependent_id
                                                    ),
                                                    "dependency_resolved",
                                                    "The path has opened",
                                                    &format!(
                                                        "{} no longer blocks {}.",
                                                        t.title, dependent_title
                                                    ),
                                                    &dependent_id,
                                                );
                                                if let Some(project_id) = t.project_id {
                                                    let _ = self.db.log_activity(
                                                        Some(&project_id.to_string()),
                                                        "quest_dependency_resolved",
                                                        &format!(
                                                            "{} opened the path for {}.",
                                                            t.title, dependent_title
                                                        ),
                                                        &log.author_public_key,
                                                        t.owner_username
                                                            .as_deref()
                                                            .unwrap_or("Companion"),
                                                    );
                                                }
                                            }
                                        }
                                        let assigned = self
                                            .db
                                            .get_task_assignments(&t.id.to_string())
                                            .unwrap_or_default();
                                        if assigned.iter().any(|(id, _)| id == my_key) {
                                            if let Ok(Some(mut user)) = self.db.get_user() {
                                                let xp = if t.priority
                                                    == crate::models::TaskPriority::High
                                                {
                                                    50
                                                } else {
                                                    25
                                                };
                                                let xp_svc =
                                                    crate::services::XPService::new(self.db);
                                                let _ = xp_svc.grant_xp(
                                                    &mut user,
                                                    "Complete Shared Task Quest",
                                                    xp,
                                                );
                                            }
                                            self.create_task_fellowship_notification(
                                                &format!("task_completed:{}", log.id),
                                                "task_completed",
                                                "Assigned quest completed",
                                                &format!("{} was completed.", t.title),
                                                &t.id.to_string(),
                                            );
                                        }
                                    } else if assigned_to_me && log.operation == "update" {
                                        if let Some(local) = local_task {
                                            let meaningful_update = local.title != t.title
                                                || local.description != t.description
                                                || local.due_date != t.due_date
                                                || local.set_date != t.set_date;
                                            if meaningful_update {
                                                self.create_task_fellowship_notification(
                                                    &format!("task_updated:{}", log.id),
                                                    "task_updated",
                                                    "Assigned quest updated",
                                                    &format!("{} was updated.", t.title),
                                                    &t.id.to_string(),
                                                );
                                            }
                                        }
                                    }
                                    pulled_count += 1;
                                }
                            }
                        }
                        "note" => {
                            if log.operation == "delete" {
                                self.save_local_revision_before_delete("note", &log.entity_id);
                                let _ = self.db.conn.execute(
                                    "DELETE FROM notes WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(mut n) = serde_json::from_str::<Note>(content) {
                                if let Ok(local_note) = self.db.get_note_by_id(n.id) {
                                    if local_note.project_id.is_some() && n.project_id.is_none() {
                                        n.project_id = local_note.project_id;
                                    }
                                    if local_note.codex_id.is_some() && n.codex_id.is_none() {
                                        n.codex_id = local_note.codex_id;
                                    }
                                }
                                if n.project_id.is_none() {
                                    conflicts.push(format!(
                                        "Scroll '{}' rejected: missing project link",
                                        n.title
                                    ));
                                    let _ = self.db.set_setting("auto_sync", "false");
                                    let _ = self.db.set_setting("sync_restore_hold", "1");
                                    newly_processed_ids.push(log.id);
                                    continue;
                                }
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO notes (id, project_id, title, markdown_content, created_at, updated_at, sharing_permission, codex_id, owner_identity) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                    params![
                                        n.id.to_string(),
                                        n.project_id.map(|id| id.to_string()),
                                        n.title,
                                        n.markdown_content,
                                        n.created_at.to_rfc3339(),
                                        n.updated_at.to_rfc3339(),
                                        n.sharing_permission,
                                        n.codex_id.map(|id| id.to_string()),
                                        n.owner_identity,
                                    ],
                                );
                                pulled_count += 1;
                            }
                        }
                        "project" => {
                            if log.operation == "delete" {
                                self.save_local_revision_before_delete("project", &log.entity_id);
                                let _ = self.db.conn.execute(
                                    "DELETE FROM projects WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(p) = serde_json::from_str::<serde_json::Value>(content)
                            {
                                let id = p["id"].as_str().unwrap_or_default();
                                let name = p["name"].as_str().unwrap_or_default();
                                let desc = p["description"].as_str();
                                let created = p["created_at"].as_str().unwrap_or_default();
                                let updated = p["updated_at"].as_str().unwrap_or(created);
                                let archived = p["archived"].as_bool().unwrap_or(false);
                                let completed = p["completed"].as_bool().unwrap_or(false);
                                let owner_id = p["owner_identity"].as_str();
                                let owner_name = p["owner_username"].as_str();
                                let is_shared = p["is_shared"].as_bool().unwrap_or(false);
                                let local_is_shared = self
                                    .db
                                    .conn
                                    .query_row(
                                        "SELECT is_shared FROM projects WHERE id = ?1",
                                        params![id],
                                        |row| row.get::<_, i32>(0),
                                    )
                                    .unwrap_or(0)
                                    != 0;
                                let merged_is_shared = is_shared || local_is_shared;
                                let _ = self.db.conn.execute(
                                    "INSERT INTO projects (id, name, description, created_at, updated_at, archived, completed, owner_identity, owner_username, is_shared)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                                     ON CONFLICT(id) DO UPDATE SET
                                         name=excluded.name, description=excluded.description,
                                         created_at=excluded.created_at, updated_at=excluded.updated_at,
                                         archived=excluded.archived,
                                         completed=excluded.completed, owner_identity=excluded.owner_identity,
                                         owner_username=excluded.owner_username,
                                         is_shared=excluded.is_shared",
                                    params![
                                        id, name, desc, created, updated,
                                        if archived { 1 } else { 0 },
                                        if completed { 1 } else { 0 },
                                        owner_id, owner_name,
                                        if merged_is_shared { 1 } else { 0 }
                                    ],
                                );
                                // A snapshot deliberately sends the account-encrypted project key
                                // before project-v1 content. On an empty device it is staged in key
                                // history until this project row satisfies the active-key FK.
                                let _ = self.db.activate_staged_project_encryption_key(id);
                                // When a project is shared, ensure owner appears in project_members
                                if merged_is_shared {
                                    let owner_id_str =
                                        p["owner_identity"].as_str().unwrap_or_default();
                                    let owner_name_str =
                                        p["owner_username"].as_str().unwrap_or_default();
                                    if !owner_id_str.is_empty() {
                                        let _ = self.db.conn.execute(
                                            "INSERT OR IGNORE INTO project_members (project_id, user_identity, user_username, role) VALUES (?1, ?2, ?3, 'Owner')",
                                            params![id, owner_id_str, owner_name_str],
                                        );
                                    }
                                }
                                pulled_count += 1;
                            }
                        }
                        "campaign_treasury" => {
                            if let Ok(value) =
                                serde_json::from_str::<crate::models::CampaignTreasury>(content)
                            {
                                let stored = self.db.conn.execute(
                                    "INSERT INTO campaign_treasury
                                     (campaign_id, overall_budget_minor, currency_code, large_expense_threshold_minor,
                                      version, created_at, updated_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                                     ON CONFLICT(campaign_id) DO UPDATE SET
                                      overall_budget_minor=excluded.overall_budget_minor,
                                      currency_code=excluded.currency_code,
                                      large_expense_threshold_minor=excluded.large_expense_threshold_minor,
                                      version=excluded.version, updated_at=excluded.updated_at",
                                    params![value.campaign_id.to_string(), value.overall_budget_minor,
                                        value.currency_code, value.large_expense_threshold_minor, value.version,
                                        value.created_at.to_rfc3339(), value.updated_at.to_rfc3339()],
                                );
                                if Self::defer_treasury_event(
                                    &log,
                                    stored,
                                    &mut conflicts,
                                    &mut retry_from_seq,
                                ) {
                                    continue;
                                }
                                pulled_count += 1;
                            }
                        }
                        "ledger_category" => {
                            if let Ok(value) =
                                serde_json::from_str::<crate::models::LedgerCategory>(content)
                            {
                                let stored = self.db.conn.execute(
                                    "INSERT INTO ledger_categories
                                     (id, campaign_id, name, is_default, version, created_at, updated_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                                     ON CONFLICT(id) DO UPDATE SET name=excluded.name,
                                      is_default=excluded.is_default, version=excluded.version,
                                      updated_at=excluded.updated_at",
                                    params![value.id.to_string(), value.campaign_id.to_string(), value.name,
                                        value.is_default as i32, value.version, value.created_at.to_rfc3339(),
                                        value.updated_at.to_rfc3339()],
                                );
                                if Self::defer_treasury_event(
                                    &log,
                                    stored,
                                    &mut conflicts,
                                    &mut retry_from_seq,
                                ) {
                                    continue;
                                }
                                pulled_count += 1;
                            }
                        }
                        "category_budget" => {
                            if let Ok(value) =
                                serde_json::from_str::<crate::models::CategoryBudget>(content)
                            {
                                let stored = self.db.conn.execute(
                                    "INSERT INTO category_budgets
                                     (category_id, campaign_id, amount_minor, version, created_at, updated_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                                     ON CONFLICT(category_id) DO UPDATE SET amount_minor=excluded.amount_minor,
                                      version=excluded.version, updated_at=excluded.updated_at",
                                    params![value.category_id.to_string(), value.campaign_id.to_string(),
                                        value.amount_minor, value.version, value.created_at.to_rfc3339(),
                                        value.updated_at.to_rfc3339()],
                                );
                                if Self::defer_treasury_event(
                                    &log,
                                    stored,
                                    &mut conflicts,
                                    &mut retry_from_seq,
                                ) {
                                    continue;
                                }
                                pulled_count += 1;
                            }
                        }
                        "ledger_entry" => {
                            if let Ok(value) =
                                serde_json::from_str::<crate::models::LedgerEntry>(content)
                            {
                                let stored = self.db.conn.execute(
                                    "INSERT INTO ledger_entries
                                     (id, campaign_id, title, description, entry_type, category_id, amount_minor,
                                      currency_code, status, due_date, payment_date, vendor_source, related_task_id,
                                      notes, attachment_ref, recurrence, custom_recurrence, version, created_at, updated_at,
                                      created_by_identity)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                                             ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                                     ON CONFLICT(id) DO UPDATE SET title=excluded.title,
                                      description=excluded.description, entry_type=excluded.entry_type,
                                      category_id=excluded.category_id, amount_minor=excluded.amount_minor,
                                      currency_code=excluded.currency_code, status=excluded.status,
                                      due_date=excluded.due_date, payment_date=excluded.payment_date,
                                      vendor_source=excluded.vendor_source, related_task_id=excluded.related_task_id,
                                      notes=excluded.notes, attachment_ref=excluded.attachment_ref,
                                      recurrence=excluded.recurrence, custom_recurrence=excluded.custom_recurrence,
                                      version=excluded.version, updated_at=excluded.updated_at,
                                      created_by_identity=COALESCE(excluded.created_by_identity,
                                                                   ledger_entries.created_by_identity)",
                                    params![value.id.to_string(), value.campaign_id.to_string(), value.title,
                                        value.description, value.entry_type.as_str(), value.category_id.to_string(),
                                        value.amount_minor, value.currency_code, value.status.as_str(),
                                        value.due_date.map(|date| date.to_rfc3339()),
                                        value.payment_date.map(|date| date.to_rfc3339()), value.vendor_source,
                                        value.related_task_id.map(|id| id.to_string()), value.notes,
                                        value.attachment_ref, value.recurrence.as_str(), value.custom_recurrence,
                                        value.version, value.created_at.to_rfc3339(), value.updated_at.to_rfc3339(),
                                        value.created_by_identity],
                                );
                                if Self::defer_treasury_event(
                                    &log,
                                    stored,
                                    &mut conflicts,
                                    &mut retry_from_seq,
                                ) {
                                    continue;
                                }
                                pulled_count += 1;
                            }
                        }
                        "task_financials" => {
                            if let Ok(value) =
                                serde_json::from_str::<crate::models::TaskFinancials>(content)
                            {
                                let stored = self.db.conn.execute(
                                    "INSERT INTO task_financials
                                     (task_id, campaign_id, estimated_cost_minor, actual_cost_minor,
                                      billable_amount_minor, payment_status, currency_code, version, created_at, updated_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                                     ON CONFLICT(task_id) DO UPDATE SET
                                      estimated_cost_minor=excluded.estimated_cost_minor,
                                      actual_cost_minor=excluded.actual_cost_minor,
                                      billable_amount_minor=excluded.billable_amount_minor,
                                      payment_status=excluded.payment_status, currency_code=excluded.currency_code,
                                      version=excluded.version, updated_at=excluded.updated_at",
                                    params![value.task_id.to_string(), value.campaign_id.to_string(),
                                        value.estimated_cost_minor, value.actual_cost_minor,
                                        value.billable_amount_minor, value.payment_status.map(|status| status.as_str()),
                                        value.currency_code, value.version, value.created_at.to_rfc3339(),
                                        value.updated_at.to_rfc3339()],
                                );
                                if Self::defer_treasury_event(
                                    &log,
                                    stored,
                                    &mut conflicts,
                                    &mut retry_from_seq,
                                ) {
                                    continue;
                                }
                                pulled_count += 1;
                            }
                        }
                        "journal_entry" => {
                            if let Ok(j) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = j["id"].as_str().unwrap_or_default();
                                let pid = j["project_id"].as_str().unwrap_or_default();
                                let date = j["entry_date"].as_str().unwrap_or_default();
                                let body = j["content"].as_str().unwrap_or_default();
                                let created = j["created_at"].as_str().unwrap_or_default();
                                let updated = j["updated_at"].as_str().unwrap_or(created);
                                let visibility = j["visibility"].as_str().unwrap_or("Private");
                                let author = j["author_username"].as_str().unwrap_or("");

                                let _ = self.db.conn.execute(
                                    "INSERT INTO journal_entries (id, project_id, entry_date, content, created_at, updated_at, visibility, author_username)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                                     ON CONFLICT(id) DO UPDATE SET
                                        project_id=excluded.project_id, entry_date=excluded.entry_date,
                                        content=excluded.content, created_at=excluded.created_at,
                                        updated_at=excluded.updated_at, visibility=excluded.visibility,
                                        author_username=excluded.author_username",
                                    params![id, pid, date, body, created, updated, visibility, author],
                                );
                                pulled_count += 1;
                            }
                        }
                        "milestone" => {
                            if log.operation == "delete" {
                                self.save_local_revision_before_delete("milestone", &log.entity_id);
                                let _ = self.db.conn.execute(
                                    "DELETE FROM milestones WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(m) = serde_json::from_str::<serde_json::Value>(content)
                            {
                                let id = m["id"].as_str().unwrap_or_default();
                                let pid = m["project_id"].as_str().unwrap_or_default();
                                let name = m["name"].as_str().unwrap_or_default();
                                let desc = m["description"].as_str();
                                let completed = m["completed"].as_bool().unwrap_or(false);
                                let xp = m["xp_reward"].as_i64().unwrap_or(0) as i32;
                                let created = m["created_at"].as_str().unwrap_or_default();
                                let updated = m["updated_at"].as_str().unwrap_or(created);
                                let tier = m["tier"].as_i64().unwrap_or(0) as i32;
                                let template_id = m["template_id"].as_str().unwrap_or("");

                                let _ = self.db.conn.execute(
                                    "INSERT INTO milestones (id, project_id, name, description, completed, xp_reward, created_at, updated_at, tier, template_id)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                                     ON CONFLICT(id) DO UPDATE SET
                                        project_id=excluded.project_id, name=excluded.name,
                                        description=excluded.description, completed=excluded.completed,
                                        xp_reward=excluded.xp_reward, created_at=excluded.created_at,
                                        updated_at=excluded.updated_at, tier=excluded.tier,
                                        template_id=excluded.template_id",
                                    params![id, pid, name, desc, if completed { 1 } else { 0 }, xp, created, updated, tier, template_id],
                                );
                                pulled_count += 1;
                            }
                        }
                        // Lore: solo desbloqueamos, nunca volvemos a bloquear — condición en el UPDATE
                        "lore_unlock" => {
                            if let Ok(l) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = l["id"].as_str().unwrap_or_default();
                                let unlocked = l["unlocked"].as_bool().unwrap_or(false);
                                let unlocked_at = l["unlocked_at"].as_str();
                                if unlocked {
                                    let _ = self.db.conn.execute(
                                        "UPDATE lore_library SET unlocked = 1, unlocked_at = ?1 WHERE id = ?2 AND unlocked = 0",
                                        params![unlocked_at, id],
                                    );
                                }
                                pulled_count += 1;
                            }
                        }
                        // Logros: solo actualizamos si unlocked_at es NULL — no tocamos desbloques previos
                        "achievement" => {
                            if let Ok(a) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = a["id"].as_str().unwrap_or_default();
                                let unlocked = a["unlocked_at"].as_str();
                                let _ = self.db.conn.execute(
                                    "UPDATE achievements SET unlocked_at = ?1 WHERE id = ?2 AND unlocked_at IS NULL",
                                    params![unlocked, id],
                                );
                                pulled_count += 1;
                            }
                        }
                        "ritual" => {
                            if let Ok(r) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = r["id"].as_str().unwrap_or_default();
                                let name = r["name"].as_str().unwrap_or_default();
                                let desc = r["description"].as_str();
                                let freq = r["frequency"].as_str().unwrap_or("Daily");
                                let xp = r["reward_xp"].as_i64().unwrap_or(0) as i32;
                                let created = r["created_at"].as_str().unwrap_or_default();
                                let updated = r["updated_at"].as_str().unwrap_or(created);
                                let daily_target = r["daily_target"].as_i64().unwrap_or(1) as i32;
                                let _ = self.db.conn.execute(
                                    "INSERT INTO rituals (id, name, description, frequency, reward_xp, created_at, updated_at, daily_target)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                                     ON CONFLICT(id) DO UPDATE SET
                                        name=excluded.name, description=excluded.description,
                                        frequency=excluded.frequency, reward_xp=excluded.reward_xp,
                                        created_at=excluded.created_at, updated_at=excluded.updated_at,
                                        daily_target=excluded.daily_target",
                                    params![id, name, desc, freq, xp, created, updated, daily_target],
                                );
                                pulled_count += 1;
                            }
                        }
                        "ritual_history" => {
                            if let Ok(rh) = serde_json::from_str::<serde_json::Value>(content) {
                                let ritual_id = rh["ritual_id"].as_str().unwrap_or_default();
                                let completed_date =
                                    rh["completed_date"].as_str().unwrap_or_default();
                                let completion_count =
                                    rh["completion_count"].as_i64().unwrap_or(1) as i32;
                                let completed_at = rh["completed_at"].as_str();
                                let _ = self.db.conn.execute(
                                    "INSERT INTO ritual_history (ritual_id, completed_date, completion_count, completed_at) VALUES (?1, ?2, ?3, ?4)
                                     ON CONFLICT(ritual_id, completed_date) DO UPDATE SET
                                        completion_count = MAX(completion_count, excluded.completion_count),
                                        completed_at = COALESCE(excluded.completed_at, completed_at)",
                                    params![ritual_id, completed_date, completion_count, completed_at],
                                );
                                pulled_count += 1;
                            }
                        }
                        "setting" => {
                            if let Ok(setting) = serde_json::from_str::<serde_json::Value>(content)
                            {
                                let key = setting["key"].as_str().unwrap_or_default();
                                let value = setting["value"].as_str().unwrap_or_default();
                                if key == log.entity_id
                                    && crate::database::SYNCED_STREAK_SETTING_KEYS.contains(&key)
                                {
                                    let _ = self.db.set_setting(key, value);
                                    pulled_count += 1;
                                }
                            }
                        }
                        "codex" => {
                            if log.operation == "delete" {
                                self.save_local_revision_before_delete("codex", &log.entity_id);
                                let _ = self.db.conn.execute(
                                    "DELETE FROM codices WHERE id = ?1",
                                    params![log.entity_id],
                                );
                            } else if let Ok(c) =
                                serde_json::from_str::<crate::models::Codex>(content)
                            {
                                let _ = self.db.conn.execute(
                                    "INSERT INTO codices (id, project_id, name, created_at, updated_at, parent_codex_id, collapsed)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                                     ON CONFLICT(id) DO UPDATE SET
                                         project_id=excluded.project_id, name=excluded.name,
                                         created_at=excluded.created_at, updated_at=excluded.updated_at,
                                         parent_codex_id=excluded.parent_codex_id,
                                         collapsed=excluded.collapsed",
                                    params![
                                        c.id.to_string(),
                                        c.project_id.to_string(),
                                        c.name,
                                        c.created_at.to_rfc3339(),
                                        c.updated_at.to_rfc3339(),
                                        c.parent_codex_id.map(|id| id.to_string()),
                                        c.collapsed as i32,
                                    ],
                                );
                            }
                            pulled_count += 1;
                        }
                        "focus_session" => {
                            if let Ok(fs) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = fs["id"].as_str().unwrap_or_default();
                                let proj = fs["project_id"].as_str();
                                let task = fs["task_id"].as_str();
                                let duration = fs["duration_mins"].as_i64().unwrap_or(0) as i32;
                                let xp = fs["xp_gained"].as_i64().unwrap_or(0) as i32;
                                let completed_at = fs["completed_at"].as_str().unwrap_or_default();
                                let soundscape = fs["soundscape"].as_str().unwrap_or("Silent");
                                let owner_identity = fs["owner_identity"].as_str();
                                let _ = self.db.conn.execute(
                                    "INSERT OR IGNORE INTO focus_sessions (id, project_id, task_id, duration_mins, xp_gained, completed_at, soundscape, owner_identity) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                    params![id, proj, task, duration, xp, completed_at, soundscape, owner_identity],
                                );
                                pulled_count += 1;
                            }
                        }
                        "task_assignment" => {
                            if log.operation == "delete" {
                                let parts: Vec<&str> = log.entity_id.splitn(2, "__").collect();
                                if parts.len() == 2 {
                                    let _ = self.db.conn.execute(
                                        "DELETE FROM task_assignments WHERE task_id = ?1 AND user_identity = ?2",
                                        params![parts[0], parts[1]],
                                    );
                                    pulled_count += 1;
                                }
                            } else if let Ok(ta) =
                                serde_json::from_str::<serde_json::Value>(content)
                            {
                                let task_id = ta["task_id"].as_str().unwrap_or_default();
                                let identity = ta["user_identity"].as_str().unwrap_or_default();
                                let username = ta["user_username"].as_str().unwrap_or_default();
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO task_assignments (task_id, user_identity, user_username) VALUES (?1, ?2, ?3)",
                                    params![task_id, identity, username],
                                );
                                if identity == self.identity.public_key {
                                    let task_title = Uuid::parse_str(task_id)
                                        .ok()
                                        .and_then(|id| self.db.get_task_by_id(id).ok())
                                        .map(|t| t.title)
                                        .unwrap_or_else(|| "A quest".to_string());
                                    self.create_task_fellowship_notification(
                                        &format!("task_assigned:{}", log.id),
                                        "task_assignment",
                                        "Quest assigned",
                                        &format!("{} was assigned to you.", task_title),
                                        task_id,
                                    );
                                }
                                pulled_count += 1;
                            }
                        }
                        "task_status" => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
                                let task_id = value["task_id"].as_str().unwrap_or_default();
                                let project_id = value["project_id"].as_str().unwrap_or_default();
                                let status = value["status"].as_str().unwrap_or("Backlog");
                                let actor_id =
                                    value["changed_by_identity"].as_str().unwrap_or_default();
                                let actor_name =
                                    value["changed_by_username"].as_str().unwrap_or("Companion");
                                let updated_at = value["updated_at"].as_str().unwrap_or_default();
                                if !task_id.is_empty() && !project_id.is_empty() {
                                    let _ = self.db.conn.execute(
                                        "INSERT INTO task_statuses (task_id, project_id, status, changed_by_identity, changed_by_username, updated_at)
                                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                                         ON CONFLICT(task_id) DO UPDATE SET project_id=excluded.project_id, status=excluded.status,
                                             changed_by_identity=excluded.changed_by_identity, changed_by_username=excluded.changed_by_username,
                                             updated_at=excluded.updated_at",
                                        params![task_id, project_id, status, actor_id, actor_name, updated_at],
                                    );
                                    if actor_id != self.identity.public_key.as_str() {
                                        let quest_title = uuid::Uuid::parse_str(task_id)
                                            .ok()
                                            .and_then(|id| self.db.get_task_by_id(id).ok())
                                            .map(|task| task.title)
                                            .unwrap_or_else(|| "A shared Quest".to_string());
                                        let stance = QuestStatus::from_str(status).display_name();
                                        let description =
                                            format!("{} entered stance: {}.", quest_title, stance);
                                        let _ = self.db.log_activity(
                                            Some(project_id),
                                            "quest_status_changed",
                                            &description,
                                            actor_id,
                                            actor_name,
                                        );

                                        let assigned_to_me = self
                                            .db
                                            .get_task_assignments(task_id)
                                            .unwrap_or_default()
                                            .iter()
                                            .any(|(identity, _)| {
                                                identity == self.identity.public_key.as_str()
                                            });
                                        if assigned_to_me {
                                            self.create_task_fellowship_notification(
                                                &format!("task_status:{}", log.id),
                                                "quest_status",
                                                "Council decree",
                                                &format!("{} is now {}.", quest_title, stance),
                                                task_id,
                                            );
                                        }
                                    }
                                    pulled_count += 1;
                                }
                            }
                        }
                        "task_dependency" => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
                                let task_id = value["task_id"].as_str().unwrap_or_default();
                                let blocker_id =
                                    value["depends_on_task_id"].as_str().unwrap_or_default();
                                let project_id = value["project_id"].as_str().unwrap_or_default();
                                let actor_id =
                                    value["created_by_identity"].as_str().unwrap_or_default();
                                let actor_name =
                                    value["created_by_username"].as_str().unwrap_or("Companion");
                                if !task_id.is_empty()
                                    && !blocker_id.is_empty()
                                    && task_id != blocker_id
                                    && !project_id.is_empty()
                                {
                                    if actor_id != log.author_public_key {
                                        conflicts.push(format!(
                                            "Quest dependency {} rejected: creator does not match its signed Companion Key",
                                            log.entity_id
                                        ));
                                    } else {
                                        match self.db.add_task_dependency(
                                            task_id, blocker_id, project_id, actor_id, actor_name,
                                        ) {
                                            Ok(()) => {
                                                let _ = self.db.conn.execute(
                                                    "UPDATE sync_log SET synced = 1 WHERE entity_type = 'task_dependency' AND entity_id = ?1",
                                                    params![log.entity_id],
                                                );
                                                pulled_count += 1;
                                            }
                                            Err(error) => conflicts.push(format!(
                                                "Quest dependency {} rejected: {}",
                                                log.entity_id, error
                                            )),
                                        }
                                    }
                                }
                            }
                        }
                        "task_comment" => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = value["id"].as_str().unwrap_or_default();
                                let task_id = value["task_id"].as_str().unwrap_or_default();
                                let project_id = value["project_id"].as_str().unwrap_or_default();
                                let author_id =
                                    value["author_identity"].as_str().unwrap_or_default();
                                let author_name =
                                    value["author_username"].as_str().unwrap_or("Companion");
                                let body = value["content"].as_str().unwrap_or_default();
                                let mentions = value["mentioned_identities"].to_string();
                                let created_at = value["created_at"].as_str().unwrap_or_default();
                                let updated_at = value["updated_at"].as_str().unwrap_or_default();
                                let edited_at = value["edited_at"].as_str();
                                let deleted_at = value["deleted_at"].as_str();
                                if author_id != log.author_public_key {
                                    conflicts.push(format!(
                                        "Quest Council message {} rejected: author does not match its signed Companion Key",
                                        id
                                    ));
                                } else if !id.is_empty()
                                    && !task_id.is_empty()
                                    && !project_id.is_empty()
                                {
                                    let stored = self.db.conn.execute(
                                        "INSERT INTO task_comments (id, task_id, project_id, author_identity, author_username, content, mentioned_identities, created_at, updated_at, edited_at, deleted_at)
                                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                                         ON CONFLICT(id) DO UPDATE SET content=excluded.content,
                                             mentioned_identities=excluded.mentioned_identities, updated_at=excluded.updated_at,
                                             edited_at=excluded.edited_at, deleted_at=excluded.deleted_at",
                                        params![id, task_id, project_id, author_id, author_name, body, mentions, created_at, updated_at, edited_at, deleted_at],
                                    );
                                    if let Err(error) = stored {
                                        conflicts.push(format!(
                                            "Quest Council message {} could not be stored: {}",
                                            id, error
                                        ));
                                        if log.seq > 0 {
                                            retry_from_seq =
                                                Some(retry_from_seq.map_or(log.seq, |current| {
                                                    current.min(log.seq)
                                                }));
                                        }
                                        continue;
                                    }
                                    if author_id != self.identity.public_key.as_str() {
                                        let description = if deleted_at.is_some() {
                                            "withdrew a Quest Council message."
                                        } else if edited_at.is_some() {
                                            "revised a Quest Council message."
                                        } else {
                                            "convened the Quest Council."
                                        };
                                        let _ = self.db.log_activity(
                                            Some(project_id),
                                            "quest_comment_added",
                                            description,
                                            author_id,
                                            author_name,
                                        );
                                    }
                                    if deleted_at.is_none()
                                        && author_id != self.identity.public_key.as_str()
                                        && value["mentioned_identities"].as_array().is_some_and(
                                            |ids| {
                                                ids.iter().any(|identity| {
                                                    identity.as_str()
                                                        == Some(self.identity.public_key.as_str())
                                                })
                                            },
                                        )
                                    {
                                        self.create_task_fellowship_notification(
                                            &format!("task_comment_mention:{}", log.id),
                                            "mention",
                                            "The Council calls your name",
                                            &format!(
                                                "{} mentioned you in a Quest Council.",
                                                author_name
                                            ),
                                            task_id,
                                        );
                                    }
                                    pulled_count += 1;
                                }
                            }
                        }
                        "notification_state" => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
                                let notification_id =
                                    value["notification_id"].as_str().unwrap_or_default();
                                let read = value["read"].as_bool().unwrap_or(false);
                                let updated_at = value["updated_at"].as_str().unwrap_or_default();
                                if !notification_id.is_empty() {
                                    let _ = self.db.conn.execute(
                                        "INSERT INTO notification_states (notification_id, read, updated_at)
                                         VALUES (?1, ?2, ?3)
                                         ON CONFLICT(notification_id) DO UPDATE SET read=excluded.read, updated_at=excluded.updated_at",
                                        params![notification_id, read as i32, updated_at],
                                    );
                                    let _ = self.db.conn.execute(
                                        "UPDATE notifications SET read = ?1 WHERE id = ?2",
                                        params![read as i32, notification_id],
                                    );
                                    pulled_count += 1;
                                }
                            }
                        }
                        "project_member" => {
                            if log.operation == "delete" {
                                let parts: Vec<&str> = log.entity_id.splitn(2, "__").collect();
                                if parts.len() == 2 {
                                    let _ = self.db.conn.execute(
                                        "DELETE FROM project_members WHERE project_id = ?1 AND user_identity = ?2",
                                        params![parts[0], parts[1]],
                                    );
                                    pulled_count += 1;
                                }
                            } else if let Ok(pm) =
                                serde_json::from_str::<serde_json::Value>(content)
                            {
                                let project_id = pm["project_id"].as_str().unwrap_or_default();
                                let identity = pm["user_identity"].as_str().unwrap_or_default();
                                let username = pm["user_username"].as_str().unwrap_or_default();
                                let role = pm["role"].as_str().unwrap_or("Member");
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO project_members (project_id, user_identity, user_username, role) VALUES (?1, ?2, ?3, ?4)",
                                    params![project_id, identity, username, role],
                                );
                                pulled_count += 1;
                            }
                        }
                        "chronicle_message" => {
                            if let Ok(cm) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = cm["id"].as_str().unwrap_or_default();
                                let project_id = cm["project_id"].as_str();
                                let sender_id = cm["sender_identity"].as_str().unwrap_or_default();
                                let sender_name =
                                    cm["sender_username"].as_str().unwrap_or_default();
                                let msg_content = cm["content"].as_str().unwrap_or_default();
                                let msg_type = cm["message_type"].as_str().unwrap_or("Text");
                                let timestamp = cm["timestamp"].as_str().unwrap_or_default();
                                let _ = self.db.conn.execute(
                                    "INSERT OR IGNORE INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                    params![id, project_id, sender_id, sender_name, msg_content, msg_type, timestamp],
                                );
                                let my_username = self
                                    .db
                                    .get_user()
                                    .ok()
                                    .flatten()
                                    .map(|user| user.username)
                                    .unwrap_or_default();
                                let mentions_me = !my_username.is_empty()
                                    && msg_content.split_whitespace().any(|word| {
                                        word.strip_prefix('@').is_some_and(|name| {
                                            name.trim_matches(|character: char| {
                                                !character.is_alphanumeric() && character != '_'
                                            })
                                            .eq_ignore_ascii_case(&my_username)
                                        })
                                    });
                                if sender_id != self.identity.public_key.as_str() && mentions_me {
                                    if let Some(project_id) = project_id {
                                        self.create_task_fellowship_notification(
                                            &format!("chronicle_mention:{}", log.id),
                                            "chronicle_mention",
                                            "Summoned to the Chronicle",
                                            &format!(
                                                "{} mentioned you in a Campaign Chronicle.",
                                                sender_name
                                            ),
                                            project_id,
                                        );
                                    }
                                }
                                pulled_count += 1;
                            }
                        }
                        // Árbol zen: aplicamos directamente sin pasar por update_zen_tree() para no
                        // disparar log_change() de nuevo y crear un loop de sync
                        "zen_tree" => {
                            if let Ok(t) = serde_json::from_str::<crate::models::ZenTree>(content) {
                                let local_tree = self.db.get_zen_tree().ok();
                                let local_growth =
                                    local_tree.as_ref().map(|lt| lt.growth).unwrap_or(0);
                                let local_health =
                                    local_tree.as_ref().map(|lt| lt.health).unwrap_or(0);
                                let local_stage =
                                    local_tree.as_ref().map(|lt| lt.stage).unwrap_or(1);
                                let local_water_today =
                                    local_tree.as_ref().map(|lt| lt.water_today).unwrap_or(0);
                                let local_total_waterings = local_tree
                                    .as_ref()
                                    .map(|lt| lt.total_waterings)
                                    .unwrap_or(0);
                                let _ = self.db.conn.execute(
                                    "UPDATE zen_tree SET growth = ?1, health = ?2, stage = ?3, last_watered = ?4, water_today = ?5, total_waterings = ?6 WHERE id = ?7",
                                    params![
                                        t.growth.max(local_growth),
                                        t.health.max(local_health),
                                        t.stage.max(local_stage),
                                        t.last_watered.map(|dt| dt.to_rfc3339()),
                                        t.water_today.max(local_water_today),
                                        t.total_waterings.max(local_total_waterings),
                                        t.id.to_string(),
                                    ],
                                );
                                pulled_count += 1;
                            }
                        }
                        // Dispositivos remotos: upsert device + actualiza presencia del usuario en ese nodo
                        "device" => {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
                                let did = v["device_id"].as_str().unwrap_or_default();
                                let dname = v["device_name"].as_str().unwrap_or("Unknown Node");
                                let last = v["last_sync"].as_str();
                                if !did.is_empty() {
                                    let _ = self.db.upsert_remote_device(did, dname, last);
                                    // Si el heartbeat trae identidad del usuario, actualizamos su presencia
                                    let user_identity =
                                        v["user_identity"].as_str().unwrap_or_default();
                                    let username = v["username"].as_str().unwrap_or_default();
                                    if !user_identity.is_empty() && !username.is_empty() {
                                        let seen = last.unwrap_or_else(|| log.timestamp.as_str());
                                        let _ = self.db.update_presence(
                                            user_identity,
                                            username,
                                            true,
                                            seen,
                                            None,
                                            "Visible",
                                        );
                                    }
                                    pulled_count += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if is_newer
                && matches!(
                    log.entity_type.as_str(),
                    "campaign_treasury"
                        | "ledger_category"
                        | "category_budget"
                        | "ledger_entry"
                        | "task_financials"
                )
            {
                let snapshot = log.content.as_deref().unwrap_or("null");
                if let Some(campaign_id) =
                    project_id_from_sync_content(&log.entity_type, &log.entity_id, snapshot)
                {
                    let version = serde_json::from_str::<serde_json::Value>(snapshot)
                        .ok()
                        .and_then(|value| value["version"].as_i64())
                        .unwrap_or(1);
                    let _ = self.db.conn.execute(
                        "INSERT INTO treasury_history
                         (id, campaign_id, entity_type, entity_id, action, snapshot, version, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![Uuid::new_v4().to_string(), campaign_id, log.entity_type,
                            log.entity_id, format!("remote_{}", log.operation), snapshot,
                            version, log.timestamp],
                    );
                }
            }
            // Marcar como procesado independientemente de si `is_newer` — queremos dedup siempre
            newly_processed_ids.push(log.id);
        }

        // Avanzamos el cursor — la próxima vez solo jalamos lo que llegó después de este punto
        max_seq = max_seq.max(remote_next_seq);
        if let Some(retry_seq) = retry_from_seq {
            max_seq = max_seq.min(retry_seq.saturating_sub(1));
        }
        if max_seq > since_seq {
            let _ = self
                .db
                .set_setting("last_pull_seq_v2", &max_seq.to_string());
        }
        let last_remote_head_seq = remote_head_seq.max(max_seq);
        let _ = self
            .db
            .set_setting("last_remote_head_seq", &last_remote_head_seq.to_string());
        let lag = last_remote_head_seq.saturating_sub(max_seq);
        let _ = self.db.set_setting("last_sync_lag", &lag.to_string());
        if !metadata_supported {
            remote_has_more = remote_downloaded >= PULL_PAGE_SIZE && max_seq > since_seq;
        }

        // Persistir IDs de eventos aplicados — end del loop infinito de replay
        let _ = self.db.mark_remote_events_processed(&newly_processed_ids);

        // Solo marcamos como synced DESPUÉS de que el pull también termine — si pull falla, re-intentamos todo
        if !pushed_ids.is_empty() {
            self.db.mark_sync_logs_synced(&pushed_ids)?;
        }

        self.db.update_device_sync_time(self.device_id)?;

        // Limpiar entradas antiguas de dedup (>90 días) — housekeeping silencioso
        let _ = self.db.cleanup_processed_remote_events(90);

        Ok((
            pushed_count,
            pulled_count,
            conflicts,
            remote_downloaded,
            remote_has_more,
        ))
    }

    /// Registra una notificación de Fellowship una sola vez por evento remoto aplicado.
    fn create_task_fellowship_notification(
        &self,
        dedup_key: &str,
        notif_type: &str,
        title: &str,
        content: &str,
        task_id: &str,
    ) {
        let notification_id = format!("fellowship:{}", dedup_key);
        let _ = self.db.create_notification_once(
            &notification_id,
            notif_type,
            title,
            content,
            Some(task_id),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::{Codex, Note, Project, Ritual, Task, TaskPriority, ZenTree};
    use chrono::{Duration, Local, TimeZone};

    struct StaticCloudProvider {
        pull_payload: String,
    }

    impl CloudProvider for StaticCloudProvider {
        fn name(&self) -> &str {
            "Static Test Provider"
        }

        fn push(&self, _public_key: &str, _signature: &str, _payload: &str) -> Result<()> {
            Ok(())
        }

        fn pull(&self, _public_key: &str, _signature: &str, _since_seq: i64) -> Result<String> {
            Ok(self.pull_payload.clone())
        }
    }

    fn test_identity() -> Identity {
        let mut rng = rand::thread_rng();
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();
        Identity {
            user_uuid: Uuid::new_v4(),
            public_key: verifying_key
                .to_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect(),
            secret_key: signing_key
                .to_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    // Verifica que la criptografía ed25519 funcione bien — firma válida pasa, mensaje alterado falla
    #[test]
    fn test_identity_key_generation_and_sign_verify() {
        let mut rng = rand::thread_rng();
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();

        let secret_hex: String = signing_key
            .to_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        let public_hex: String = verifying_key
            .to_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        let identity = Identity {
            user_uuid: Uuid::new_v4(),
            public_key: public_hex.clone(),
            secret_key: secret_hex.clone(),
            created_at: Utc::now().to_rfc3339(),
        };

        let msg = b"Questline sync chronicle validation message";
        let sig_hex = identity.sign(msg).expect("Failed to sign message");

        let is_valid =
            Identity::verify(msg, &public_hex, &sig_hex).expect("Failed to verify signature");
        assert!(is_valid, "Cryptographic signature validation failed");

        let is_invalid = Identity::verify(b"tampered message", &public_hex, &sig_hex)
            .expect("Failed to check invalid signature");
        assert!(
            !is_invalid,
            "Cryptographic signature accepted tampered message"
        );
    }

    // Checa que el DB registre cambios y guarde revisiones — crítico para que sync no pierda datos
    #[test]
    fn test_database_change_tracking_and_revisions() {
        let temp_db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_sync.db");
        if temp_db_path.exists() {
            let _ = std::fs::remove_file(&temp_db_path);
        }

        let db = Database::new(&temp_db_path).expect("Failed to create test DB");

        let task_id = Uuid::new_v4();
        let task = Task {
            id: task_id,
            project_id: None,
            title: "Test Sync Task".to_string(),
            description: Some("Description for testing sync".to_string()),
            due_date: None,
            set_date: None,
            completed: false,
            priority: TaskPriority::High,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            owner_identity: None,
            owner_username: None,
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        };

        db.insert_task(&task).expect("Failed to insert task");

        let pending_logs = db
            .get_pending_sync_logs()
            .expect("Failed to get pending sync logs");
        assert!(
            !pending_logs.is_empty(),
            "Database did not create a sync log entry on task insert"
        );
        assert_eq!(pending_logs[0].1, "task", "Sync log entity_type mismatch");
        assert_eq!(
            pending_logs[0].2,
            task_id.to_string(),
            "Sync log entity_id mismatch"
        );
        assert_eq!(pending_logs[0].3, "create", "Sync log operation mismatch");

        let note_id = Uuid::new_v4();
        let note = Note {
            id: note_id,
            project_id: None,
            title: "Test Note Scroll".to_string(),
            markdown_content: "Content version 1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            sharing_permission: "read_only".to_string(),
            codex_id: None,
            owner_identity: None,
        };

        db.insert_note(&note).expect("Failed to insert note");

        let note_revisions = db
            .get_revisions("note", &note_id.to_string())
            .expect("Failed to get revisions");
        assert_eq!(
            note_revisions.len(),
            1,
            "Database did not archive note version 1 snapshot"
        );

        let mut updated_note = note.clone();
        updated_note.markdown_content = "Content version 2".to_string();
        db.update_note(&updated_note)
            .expect("Failed to update note");

        let updated_revisions = db
            .get_revisions("note", &note_id.to_string())
            .expect("Failed to get updated revisions");
        assert_eq!(
            updated_revisions.len(),
            2,
            "Database did not archive note version 2 snapshot"
        );
        assert_eq!(
            updated_revisions[0].0, 2,
            "Revision number increment failed"
        );

        let _ = std::fs::remove_file(&temp_db_path);
    }

    // Backward compat: logs viejos sin device_id deben deserializar bien con el default vacío
    #[test]
    fn test_sync_log_entry_deserialization_without_device_id() {
        let json_data = r#"[
            {
                "id": "event-1",
                "entity_type": "task",
                "entity_id": "task-uuid-1",
                "operation": "create",
                "timestamp": "2026-06-21T12:00:00Z",
                "content": "some task content"
            }
        ]"#;

        let entries: Result<Vec<SyncLogEntry>, _> = serde_json::from_str(json_data);
        assert!(
            entries.is_ok(),
            "Failed to deserialize SyncLogEntry missing device_id field"
        );
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].device_id, "",
            "Expected device_id to default to an empty string"
        );
    }

    #[test]
    fn test_pull_page_parses_legacy_array_response() {
        let json_data = r#"[
            {
                "id": "event-1",
                "entity_type": "task",
                "entity_id": "task-uuid-1",
                "operation": "update",
                "timestamp": "2026-06-21T12:00:00Z",
                "content": "{}",
                "device_id": "device-a",
                "seq": 42
            }
        ]"#;

        let page = parse_pull_page(json_data, 10).expect("legacy pull page should parse");
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.next_seq, 42);
        assert_eq!(page.head_seq, 42);
        assert!(!page.has_more);
        assert!(!page.metadata_supported);
    }

    #[test]
    fn test_pull_page_parses_metadata_response() {
        let json_data = r#"{
            "events": [
                {
                    "id": "event-1",
                    "entity_type": "task",
                    "entity_id": "task-uuid-1",
                    "operation": "update",
                    "timestamp": "2026-06-21T12:00:00Z",
                    "content": "{}",
                    "device_id": "device-a",
                    "seq": 42
                }
            ],
            "head_seq": 100,
            "next_seq": 42,
            "has_more": true
        }"#;

        let page = parse_pull_page(json_data, 10).expect("metadata pull page should parse");
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.next_seq, 42);
        assert_eq!(page.head_seq, 100);
        assert!(page.has_more);
        assert!(page.metadata_supported);
    }

    #[test]
    fn user_progress_comparison_includes_completed_levels() {
        let now = Utc::now();
        let mut lower = crate::models::User {
            id: Uuid::new_v4(),
            username: "Hero".into(),
            class: crate::models::ClassType::CodeWarlock,
            level: 19,
            xp: 1872,
            created_at: now,
            specialization: None,
        };
        let mut higher = lower.clone();
        higher.xp = 2019;
        assert!(total_user_progress_xp(&higher) > total_user_progress_xp(&lower));

        lower.level = 20;
        lower.xp = 0;
        assert!(total_user_progress_xp(&lower) > total_user_progress_xp(&higher));
    }

    #[test]
    fn test_remote_task_delete_older_than_local_edit_is_skipped() {
        let temp_db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_stale_task_delete.db");
        if temp_db_path.exists() {
            let _ = std::fs::remove_file(&temp_db_path);
        }

        let db = Database::new(&temp_db_path).expect("Failed to create test DB");
        let task_id = Uuid::new_v4();
        let updated_at = Utc::now();
        let task = Task {
            id: task_id,
            project_id: None,
            title: "Keep newer local task".to_string(),
            description: None,
            due_date: None,
            set_date: None,
            completed: false,
            priority: TaskPriority::Medium,
            created_at: updated_at,
            updated_at,
            owner_identity: None,
            owner_username: None,
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        };
        db.insert_task(&task).expect("Failed to insert task");

        let remote_delete = serde_json::json!({
            "events": [{
                "id": "remote-delete-task",
                "entity_type": "task",
                "entity_id": task_id.to_string(),
                "operation": "delete",
                "timestamp": (updated_at - Duration::minutes(5)).to_rfc3339(),
                "content": null,
                "device_id": "other-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        let sync_engine = SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: remote_delete,
            }),
        };

        sync_engine.sync().expect("sync should complete");
        assert!(
            db.get_task_by_id(task_id).is_ok(),
            "stale remote delete removed a newer local task"
        );

        let _ = std::fs::remove_file(&temp_db_path);
    }

    #[test]
    fn legacy_remote_set_date_is_imported_as_due_date() {
        let temp_db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_legacy_set_date_sync.db");
        let _ = std::fs::remove_file(&temp_db_path);
        let db = Database::new(&temp_db_path).expect("Failed to create test DB");
        let scheduled = Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap();
        let legacy_task = Task {
            id: Uuid::new_v4(),
            project_id: None,
            title: "Legacy yearly quest".to_string(),
            description: None,
            due_date: None,
            set_date: Some(scheduled),
            completed: false,
            priority: TaskPriority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            owner_identity: None,
            owner_username: None,
            parent_task_id: None,
            xp_awarded: false,
            recurrence: Some(crate::models::RecurrenceType::Yearly),
        };
        let payload = serde_json::json!({
            "events": [{
                "id": "legacy-set-date-task",
                "entity_type": "task",
                "entity_id": legacy_task.id.to_string(),
                "operation": "create",
                "timestamp": legacy_task.updated_at.to_rfc3339(),
                "content": serde_json::to_string(&legacy_task).unwrap(),
                "device_id": "older-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: payload,
            }),
        }
        .sync()
        .unwrap();

        let saved = db.get_task_by_id(legacy_task.id).unwrap();
        assert_eq!(saved.due_date, Some(scheduled));
        assert_eq!(saved.set_date, None);

        drop(db);
        let _ = std::fs::remove_file(&temp_db_path);
    }

    #[test]
    fn project_conflicts_use_updated_at_instead_of_pull_order() {
        let temp_db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_project_lww.db");
        let _ = std::fs::remove_file(&temp_db_path);
        let db = Database::new(&temp_db_path).expect("Failed to create test DB");
        let now = Utc::now();
        let project_id = Uuid::new_v4();
        let local = Project {
            id: project_id,
            name: "Newer local campaign".to_string(),
            description: None,
            created_at: now - Duration::days(10),
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        };
        db.insert_project(&local).unwrap();

        let mut older_remote = local.clone();
        older_remote.name = "Stale remote campaign".to_string();
        older_remote.updated_at = now - Duration::hours(1);
        let old_payload = serde_json::json!({
            "events": [{
                "id": "remote-project-old",
                "entity_type": "project",
                "entity_id": project_id.to_string(),
                "operation": "update",
                "timestamp": older_remote.updated_at.to_rfc3339(),
                "content": serde_json::to_string(&older_remote).unwrap(),
                "device_id": "other-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: old_payload,
            }),
        }
        .sync()
        .unwrap();
        assert_eq!(db.get_projects().unwrap()[0].name, "Newer local campaign");

        let mut newer_remote = local;
        newer_remote.name = "Newest remote campaign".to_string();
        newer_remote.updated_at = now + Duration::hours(1);
        let new_payload = serde_json::json!({
            "events": [{
                "id": "remote-project-new",
                "entity_type": "project",
                "entity_id": project_id.to_string(),
                "operation": "update",
                "timestamp": newer_remote.updated_at.to_rfc3339(),
                "content": serde_json::to_string(&newer_remote).unwrap(),
                "device_id": "other-device",
                "seq": 2
            }],
            "head_seq": 2,
            "next_seq": 2,
            "has_more": false
        })
        .to_string();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: new_payload,
            }),
        }
        .sync()
        .unwrap();
        assert_eq!(db.get_projects().unwrap()[0].name, "Newest remote campaign");

        drop(db);
        let _ = std::fs::remove_file(&temp_db_path);
    }

    #[test]
    fn remote_ritual_update_preserves_completion_history() {
        let temp_db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_ritual_history_sync.db");
        let _ = std::fs::remove_file(&temp_db_path);
        let db = Database::new(&temp_db_path).expect("Failed to create test DB");
        let now = Utc::now();
        let today = Local::now().date_naive();
        let ritual = Ritual {
            id: "sync-history-ritual".to_string(),
            name: "Original ritual".to_string(),
            description: None,
            frequency: "Daily".to_string(),
            reward_xp: 20,
            daily_target: 1,
            created_at: now,
            updated_at: now,
        };
        db.insert_ritual(&ritual).unwrap();
        db.complete_ritual(&ritual.id, today, Local::now()).unwrap();

        let mut remote = ritual;
        remote.name = "Updated on another device".to_string();
        remote.updated_at = now + Duration::hours(1);
        let payload = serde_json::json!({
            "events": [{
                "id": "remote-ritual-update",
                "entity_type": "ritual",
                "entity_id": remote.id.clone(),
                "operation": "update",
                "timestamp": remote.updated_at.to_rfc3339(),
                "content": serde_json::to_string(&remote).unwrap(),
                "device_id": "other-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: payload,
            }),
        }
        .sync()
        .unwrap();

        let saved = db
            .get_rituals()
            .unwrap()
            .into_iter()
            .find(|item| item.id == "sync-history-ritual")
            .unwrap();
        assert_eq!(saved.name, "Updated on another device");
        assert_eq!(
            db.get_ritual_day_counts(today)
                .unwrap()
                .get("sync-history-ritual"),
            Some(&(1, 1))
        );

        drop(db);
        let _ = std::fs::remove_file(&temp_db_path);
    }

    #[test]
    fn test_contentless_remote_note_delete_applies_with_revision() {
        let temp_db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_contentless_note_delete.db");
        if temp_db_path.exists() {
            let _ = std::fs::remove_file(&temp_db_path);
        }

        let db = Database::new(&temp_db_path).expect("Failed to create test DB");
        let note_id = Uuid::new_v4();
        let created_at = Utc::now() - Duration::minutes(10);
        let note = Note {
            id: note_id,
            project_id: None,
            title: "Delete via tombstone".to_string(),
            markdown_content: "Recoverable content".to_string(),
            created_at,
            updated_at: created_at,
            sharing_permission: "collaborative".to_string(),
            codex_id: None,
            owner_identity: None,
        };
        db.insert_note(&note).expect("Failed to insert note");

        let remote_delete = serde_json::json!({
            "events": [{
                "id": "remote-delete-note",
                "entity_type": "note",
                "entity_id": note_id.to_string(),
                "operation": "delete",
                "timestamp": Utc::now().to_rfc3339(),
                "content": null,
                "device_id": "other-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        let sync_engine = SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: remote_delete,
            }),
        };

        sync_engine.sync().expect("sync should complete");
        assert!(
            db.get_note_by_id(note_id).is_err(),
            "contentless remote delete did not remove the note"
        );
        let revisions = db
            .get_revisions("note", &note_id.to_string())
            .expect("Failed to read note revisions");
        assert!(
            revisions
                .iter()
                .any(|(_, content, _)| content.contains("Recoverable content")),
            "remote delete did not leave a recoverable note revision"
        );

        let _ = std::fs::remove_file(&temp_db_path);
    }

    #[test]
    fn test_export_import_preserves_note_codex_and_zen_tree() {
        let source_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_export_source.db");
        let restore_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_export_restore.db");
        let _ = std::fs::remove_file(&source_path);
        let _ = std::fs::remove_file(&restore_path);

        let source = Database::new(&source_path).expect("Failed to create source DB");
        let project_id = Uuid::new_v4();
        let codex_id = Uuid::new_v4();
        let note_id = Uuid::new_v4();
        let now = Utc::now();

        source
            .insert_project(&Project {
                id: project_id,
                name: "Gibranlp".to_string(),
                description: None,
                created_at: now,
                updated_at: now,
                archived: false,
                completed: false,
                owner_identity: None,
                owner_username: None,
                is_shared: false,
            })
            .expect("Failed to insert project");
        source
            .insert_codex(&Codex {
                id: codex_id,
                project_id,
                name: "Software".to_string(),
                created_at: now,
                updated_at: now,
                parent_codex_id: None,
                collapsed: false,
            })
            .expect("Failed to insert codex");
        source
            .insert_note(&Note {
                id: note_id,
                project_id: Some(project_id),
                title: "Grouped Scroll".to_string(),
                markdown_content: "Must stay inside Software".to_string(),
                created_at: now,
                updated_at: now,
                sharing_permission: "collaborative".to_string(),
                codex_id: Some(codex_id),
                owner_identity: None,
            })
            .expect("Failed to insert note");
        source
            .update_zen_tree(&ZenTree {
                id: source.get_zen_tree().unwrap().id,
                growth: 272,
                health: 88,
                stage: 4,
                last_watered: Some(now),
                water_today: 1,
                total_waterings: 12,
            })
            .expect("Failed to update tree");

        let json = source.export_to_json().expect("Failed to export source");
        let restored = Database::new(&restore_path).expect("Failed to create restore DB");
        restored.import_from_json(&json).expect("Failed to import");

        let restored_note = restored
            .get_note_by_id(note_id)
            .expect("Missing restored note");
        assert_eq!(restored_note.project_id, Some(project_id));
        assert_eq!(restored_note.codex_id, Some(codex_id));
        let restored_tree = restored.get_zen_tree().expect("Missing restored tree");
        assert_eq!(restored_tree.growth, 272);
        assert_eq!(restored_tree.stage, 4);
        assert_eq!(restored_tree.total_waterings, 12);

        let _ = std::fs::remove_file(&source_path);
        let _ = std::fs::remove_file(&restore_path);
    }

    #[test]
    fn remote_parent_upserts_do_not_unlink_children() {
        let db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_questline_parent_upsert.db");
        let _ = std::fs::remove_file(&db_path);
        let db = Database::new(&db_path).expect("Failed to create test DB");
        let now = Utc::now();
        let project_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let codex_id = Uuid::new_v4();
        let note_id = Uuid::new_v4();

        let project = Project {
            id: project_id,
            name: "Protected project".into(),
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
        let parent = Task {
            id: parent_id,
            project_id: Some(project_id),
            title: "Parent".into(),
            description: None,
            due_date: None,
            set_date: None,
            completed: false,
            priority: TaskPriority::Medium,
            created_at: now,
            updated_at: now + Duration::minutes(1),
            owner_identity: None,
            owner_username: None,
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        };
        db.insert_task(&parent).unwrap();
        let mut child = parent.clone();
        child.id = child_id;
        child.title = "Child".into();
        child.parent_task_id = Some(parent_id);
        db.insert_task(&child).unwrap();
        let codex = Codex {
            id: codex_id,
            project_id,
            name: "Codex".into(),
            created_at: now,
            updated_at: now,
            parent_codex_id: None,
            collapsed: false,
        };
        db.insert_codex(&codex).unwrap();
        db.insert_note(&Note {
            id: note_id,
            project_id: Some(project_id),
            title: "Scroll".into(),
            markdown_content: "content".into(),
            created_at: now,
            updated_at: now,
            sharing_permission: "collaborative".into(),
            codex_id: Some(codex_id),
            owner_identity: None,
        })
        .unwrap();

        let pull_payload = serde_json::json!({
            "events": [
                {
                    "id": "remote-project-upsert",
                    "entity_type": "project",
                    "entity_id": project_id.to_string(),
                    "operation": "upsert",
                    "timestamp": (now + Duration::minutes(2)).to_rfc3339(),
                    "content": serde_json::to_string(&project).unwrap(),
                    "device_id": "other-device",
                    "seq": 1
                },
                {
                    "id": "remote-task-upsert",
                    "entity_type": "task",
                    "entity_id": parent_id.to_string(),
                    "operation": "upsert",
                    "timestamp": (now + Duration::minutes(2)).to_rfc3339(),
                    "content": serde_json::to_string(&parent).unwrap(),
                    "device_id": "other-device",
                    "seq": 2
                },
                {
                    "id": "remote-codex-upsert",
                    "entity_type": "codex",
                    "entity_id": codex_id.to_string(),
                    "operation": "upsert",
                    "timestamp": (now + Duration::minutes(2)).to_rfc3339(),
                    "content": serde_json::to_string(&codex).unwrap(),
                    "device_id": "other-device",
                    "seq": 3
                }
            ],
            "head_seq": 3,
            "next_seq": 3,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider { pull_payload }),
        }
        .sync()
        .expect("sync should complete");

        assert_eq!(
            db.get_task_by_id(child_id).unwrap().parent_task_id,
            Some(parent_id)
        );
        let restored_note = db.get_note_by_id(note_id).unwrap();
        assert_eq!(restored_note.project_id, Some(project_id));
        assert_eq!(restored_note.codex_id, Some(codex_id));

        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn remote_notification_state_updates_matching_local_notice_without_content() {
        let db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_notification_state_sync.db");
        let _ = std::fs::remove_file(&db_path);
        let db = Database::new(&db_path).unwrap();
        let notice_id = "fellowship:task_comment_mention:event-9";
        db.create_notification_once(
            notice_id,
            "mention",
            "Private local title",
            "Private decrypted text",
            Some("task-9"),
        )
        .unwrap();
        let updated_at = (Utc::now() + Duration::minutes(1)).to_rfc3339();
        let pull_payload = serde_json::json!({
            "events": [{
                "id": "remote-notification-read",
                "entity_type": "notification_state",
                "entity_id": notice_id,
                "operation": "upsert",
                "timestamp": updated_at,
                "content": serde_json::json!({
                    "notification_id": notice_id,
                    "read": true,
                    "updated_at": updated_at
                }).to_string(),
                "device_id": "other-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        let identity = test_identity();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider { pull_payload }),
        }
        .sync()
        .unwrap();
        let notices = db.get_notifications().unwrap();
        assert!(notices[0].5);
        assert_eq!(notices[0].2, "Private local title");
        assert_eq!(notices[0].3, "Private decrypted text");
        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn remote_council_message_is_stored_and_creates_one_mention_notice() {
        let db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_remote_council_mention_sync.db");
        let _ = std::fs::remove_file(&db_path);
        let db = Database::new(&db_path).unwrap();
        let now = Utc::now();
        let project_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let local_identity = test_identity();
        let remote_identity = test_identity();
        db.insert_project(&Project {
            id: project_id,
            name: "Shared Council".into(),
            description: None,
            created_at: now,
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: Some(remote_identity.public_key.clone()),
            owner_username: Some("Aria".into()),
            is_shared: true,
        })
        .unwrap();
        db.insert_task(&Task {
            id: task_id,
            project_id: Some(project_id),
            title: "Review the map".into(),
            description: None,
            due_date: None,
            set_date: None,
            completed: false,
            priority: TaskPriority::Medium,
            created_at: now,
            updated_at: now,
            owner_identity: Some(remote_identity.public_key.clone()),
            owner_username: Some("Aria".into()),
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        })
        .unwrap();
        let comment_id = Uuid::new_v4().to_string();
        let event_time = (now + Duration::minutes(1)).to_rfc3339();
        let content = serde_json::json!({
            "id": comment_id,
            "task_id": task_id.to_string(),
            "project_id": project_id.to_string(),
            "author_identity": remote_identity.public_key.clone(),
            "author_username": "Aria",
            "content": "@Ren the map is ready",
            "mentioned_identities": [local_identity.public_key.clone()],
            "created_at": event_time,
            "updated_at": event_time,
            "edited_at": null,
            "deleted_at": null
        });
        let payload = serde_json::json!({
            "events": [{
                "id": "remote-council-event",
                "entity_type": "task_comment",
                "entity_id": comment_id,
                "operation": "create",
                "timestamp": event_time,
                "content": content.to_string(),
                "device_id": "profile-a-device",
                "author_public_key": content["author_identity"],
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();

        SyncEngine {
            db: &db,
            identity: &local_identity,
            device_id: "profile-b-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: payload.clone(),
            }),
        }
        .sync()
        .unwrap();
        assert_eq!(db.get_task_comments(&task_id.to_string()).unwrap().len(), 1);
        let notices = db.get_notifications().unwrap();
        assert_eq!(
            notices
                .iter()
                .filter(|notice| notice.1 == "mention")
                .count(),
            1
        );

        SyncEngine {
            db: &db,
            identity: &local_identity,
            device_id: "profile-b-device",
            provider: Box::new(StaticCloudProvider {
                pull_payload: payload,
            }),
        }
        .sync()
        .unwrap();
        assert_eq!(db.get_notifications().unwrap().len(), 1);
        assert_eq!(
            db.get_activity_log_for_project(&project_id.to_string(), 5)
                .unwrap()
                .len(),
            1
        );
        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn remote_blocker_completion_creates_one_dependency_resolution_notice() {
        let db_path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_remote_dependency_resolution.db");
        let _ = std::fs::remove_file(&db_path);
        let db = Database::new(&db_path).unwrap();
        let identity = test_identity();
        let project_id = Uuid::new_v4();
        let blocker_id = Uuid::new_v4();
        let dependent_id = Uuid::new_v4();
        let now = Utc::now();
        db.insert_project(&Project {
            id: project_id,
            name: "Dependency Sync".into(),
            description: None,
            created_at: now,
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        })
        .unwrap();
        let make_task = |id, title: &str| Task {
            id,
            project_id: Some(project_id),
            title: title.into(),
            description: None,
            due_date: None,
            set_date: None,
            completed: false,
            priority: TaskPriority::Medium,
            created_at: now,
            updated_at: now,
            owner_identity: None,
            owner_username: Some("Remote Companion".into()),
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        };
        let blocker = make_task(blocker_id, "Open the gate");
        let dependent = make_task(dependent_id, "Cross the gate");
        db.insert_task(&blocker).unwrap();
        db.insert_task(&dependent).unwrap();
        db.add_task_dependency(
            &dependent_id.to_string(),
            &blocker_id.to_string(),
            &project_id.to_string(),
            "remote-author",
            "Remote Companion",
        )
        .unwrap();
        db.assign_task(
            &dependent_id.to_string(),
            &identity.public_key,
            "Local Hero",
        )
        .unwrap();
        let mut completed = blocker.clone();
        completed.completed = true;
        completed.updated_at = now + Duration::minutes(1);
        let pull_payload = serde_json::json!({
            "events": [{
                "id": "remote-blocker-completed",
                "entity_type": "task",
                "entity_id": blocker_id.to_string(),
                "operation": "update",
                "timestamp": completed.updated_at.to_rfc3339(),
                "content": serde_json::to_string(&completed).unwrap(),
                "device_id": "other-device",
                "seq": 1
            }],
            "head_seq": 1,
            "next_seq": 1,
            "has_more": false
        })
        .to_string();
        SyncEngine {
            db: &db,
            identity: &identity,
            device_id: "local-device",
            provider: Box::new(StaticCloudProvider { pull_payload }),
        }
        .sync()
        .unwrap();

        let notices = db.get_notifications().unwrap();
        assert_eq!(
            notices
                .iter()
                .filter(|notice| notice.1 == "dependency_resolved")
                .count(),
            1
        );
        assert!(
            notices
                .iter()
                .any(|notice| notice.3.contains("Cross the gate"))
        );
        drop(db);
        let _ = std::fs::remove_file(&db_path);
    }

    /// Servidor de prueba que sí ejerce la ruta cifrada real: guarda los sobres tal
    /// como los manda el cliente y los devuelve con el mismo formato que sync/v2/pull.
    #[derive(Clone, Default)]
    struct SharedEventLog(std::sync::Arc<std::sync::Mutex<Vec<SyncLogEntry>>>);

    struct RecordingProvider {
        events: SharedEventLog,
        serve: bool,
        // El servidor real solo replica eventos de ámbito project a otras cuentas.
        project_scope_only: bool,
        // Simula un padre que todavía no llega (paginación, borrado local, evento perdido).
        withhold: Option<&'static str>,
    }

    impl CloudProvider for RecordingProvider {
        fn name(&self) -> &str {
            "Recording Encrypted Provider"
        }

        fn requires_encryption(&self) -> bool {
            true
        }

        fn push(&self, _public_key: &str, _signature: &str, payload: &str) -> Result<()> {
            let mut parsed: Vec<SyncLogEntry> = serde_json::from_str(payload)?;
            let mut events = self.events.0.lock().unwrap();
            for event in &mut parsed {
                event.seq = events.len() as i64 + 1;
                events.push(event.clone());
            }
            Ok(())
        }

        fn pull(&self, _public_key: &str, _signature: &str, since_seq: i64) -> Result<String> {
            let events = self.events.0.lock().unwrap();
            let served = if self.serve {
                events
                    .iter()
                    .filter(|event| event.seq > since_seq)
                    .filter(|event| !self.project_scope_only || event.scope == "project")
                    .filter(|event| Some(event.entity_type.as_str()) != self.withhold)
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let head = events.len() as i64;
            Ok(serde_json::json!({
                "events": served,
                "head_seq": head,
                "next_seq": head,
                "has_more": false,
                "signatures_required": true,
            })
            .to_string())
        }
    }

    fn treasury_test_db(name: &str) -> (std::path::PathBuf, Database) {
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("{name}.db"));
        let _ = std::fs::remove_file(&path);
        let db = Database::new(&path).unwrap();
        (path, db)
    }

    fn shared_campaign_project(id: Uuid, owner: &Identity, is_shared: bool) -> Project {
        let now = Utc::now();
        Project {
            id,
            name: "Ledger Campaign".into(),
            description: None,
            created_at: now,
            updated_at: now,
            archived: false,
            completed: false,
            owner_identity: Some(owner.public_key.clone()),
            owner_username: Some("Aria".into()),
            is_shared,
        }
    }

    fn campaign_task(id: Uuid, project_id: Uuid, owner: &Identity) -> Task {
        let now = Utc::now();
        Task {
            id,
            project_id: Some(project_id),
            title: "Commission the smith".into(),
            description: None,
            due_date: None,
            set_date: None,
            completed: false,
            priority: TaskPriority::Medium,
            created_at: now,
            updated_at: now,
            owner_identity: Some(owner.public_key.clone()),
            owner_username: Some("Aria".into()),
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        }
    }

    /// Siembra tesorería completa: divisa, categoría propia, presupuestos, movimiento y finanzas de tarea.
    fn seed_treasury(
        db: &Database,
        campaign_id: Uuid,
        task_id: Uuid,
        author: &str,
    ) -> (Uuid, Uuid) {
        let service = crate::services::TreasuryService::new(db);
        service.ensure_campaign(campaign_id).unwrap();
        service
            .set_currency(campaign_id, crate::models::Currency::Mxn)
            .unwrap();
        service.set_overall_budget(campaign_id, 5_000_00).unwrap();
        let category = service.create_category(campaign_id, "Smithing").unwrap();
        service
            .set_category_budget(campaign_id, category.id, 1_200_00)
            .unwrap();
        let now = Utc::now();
        let entry = service
            .create_entry(crate::models::LedgerEntry {
                id: Uuid::new_v4(),
                campaign_id,
                title: "Dwarven anvil".into(),
                description: "Special order".into(),
                entry_type: crate::models::LedgerEntryType::Expense,
                category_id: category.id,
                amount_minor: 742_55,
                currency_code: "MXN".into(),
                status: crate::models::LedgerStatus::Paid,
                due_date: None,
                payment_date: Some(now),
                vendor_source: Some("Erebor Forge".into()),
                related_task_id: Some(task_id),
                notes: Some("Paid in pesos".into()),
                attachment_ref: None,
                recurrence: crate::models::LedgerRecurrence::None,
                custom_recurrence: None,
                version: 0,
                created_at: now,
                updated_at: now,
                created_by_identity: Some(author.to_string()),
            })
            .unwrap();
        service
            .set_task_financials(&crate::models::TaskFinancials {
                task_id,
                campaign_id,
                estimated_cost_minor: Some(800_00),
                actual_cost_minor: Some(742_55),
                billable_amount_minor: Some(900_00),
                payment_status: Some(crate::models::TaskPaymentStatus::Invoiced),
                currency_code: "MXN".into(),
                version: 1,
                created_at: now,
                updated_at: now,
            })
            .unwrap();
        (category.id, entry.id)
    }

    fn assert_treasury_matches(
        db: &Database,
        campaign_id: Uuid,
        category_id: Uuid,
        entry_id: Uuid,
        author: &str,
    ) {
        let service = crate::services::TreasuryService::new(db);
        let treasury = service
            .get_campaign(campaign_id)
            .unwrap()
            .expect("campaign treasury did not sync");
        assert_eq!(treasury.currency_code, "MXN");
        assert_eq!(treasury.overall_budget_minor, 5_000_00);

        let categories = service.categories(campaign_id).unwrap();
        assert!(
            categories
                .iter()
                .any(|item| item.id == category_id && item.name == "Smithing" && !item.is_default),
            "custom ledger category did not sync"
        );
        assert_eq!(
            categories.len(),
            crate::services::treasury::DEFAULT_CATEGORIES.len() + 1
        );

        let budgeted = service
            .calculate_category_totals(campaign_id)
            .unwrap()
            .into_iter()
            .find(|item| item.category.id == category_id)
            .expect("category totals missing");
        assert_eq!(budgeted.budget_minor, Some(1_200_00));

        let entry = service
            .get_entry(entry_id)
            .unwrap()
            .expect("ledger entry did not sync");
        assert_eq!(entry.title, "Dwarven anvil");
        assert_eq!(entry.amount_minor, 742_55);
        assert_eq!(entry.currency_code, "MXN");
        assert_eq!(entry.vendor_source.as_deref(), Some("Erebor Forge"));
        assert_eq!(entry.notes.as_deref(), Some("Paid in pesos"));
        assert_eq!(entry.status, crate::models::LedgerStatus::Paid);
        assert!(entry.payment_date.is_some());
        // La autoría viaja: de ella depende que un Companion pueda tocar su movimiento.
        assert_eq!(entry.created_by_identity.as_deref(), Some(author));

        let financials = service
            .get_task_financials(entry.related_task_id.unwrap())
            .unwrap()
            .expect("task financials did not sync");
        assert_eq!(financials.estimated_cost_minor, Some(800_00));
        assert_eq!(financials.actual_cost_minor, Some(742_55));
        assert_eq!(financials.billable_amount_minor, Some(900_00));
        assert_eq!(financials.currency_code, "MXN");
        assert_eq!(
            financials.payment_status,
            Some(crate::models::TaskPaymentStatus::Invoiced)
        );
    }

    /// Campaña personal: la tesorería viaja cifrada con la llave de cuenta y llega
    /// completa al segundo dispositivo de la misma identidad.
    #[test]
    fn treasury_round_trips_through_account_encrypted_sync() {
        let identity = test_identity();
        let campaign_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let (path_a, db_a) = treasury_test_db("treasury_sync_account_device_a");
        let (path_b, db_b) = treasury_test_db("treasury_sync_account_device_b");
        db_a.insert_project(&shared_campaign_project(campaign_id, &identity, false))
            .unwrap();
        db_a.insert_task(&campaign_task(task_id, campaign_id, &identity))
            .unwrap();
        let (category_id, entry_id) =
            seed_treasury(&db_a, campaign_id, task_id, &identity.public_key);

        let server = SharedEventLog::default();
        SyncEngine {
            db: &db_a,
            identity: &identity,
            device_id: "device-a",
            provider: Box::new(RecordingProvider {
                events: server.clone(),
                serve: false,
                project_scope_only: false,
                withhold: None,
            }),
        }
        .sync()
        .unwrap();

        let treasury_types = [
            "campaign_treasury",
            "ledger_category",
            "category_budget",
            "ledger_entry",
            "task_financials",
        ];
        let raw = server.0.lock().unwrap().clone();
        for entity_type in treasury_types {
            let events = raw
                .iter()
                .filter(|event| event.entity_type == entity_type)
                .collect::<Vec<_>>();
            assert!(!events.is_empty(), "{entity_type} never reached the server");
            for event in events {
                assert_eq!(event.version, crate::services::encryption::SYNC_VERSION);
                assert_eq!(event.key_id, "account-v1");
                assert_eq!(event.scope, "account");
                assert!(event.content.is_none(), "{entity_type} pushed plaintext");
                assert!(!event.nonce.is_empty() && !event.ciphertext.is_empty());
                assert_eq!(event.event_signature.len(), 128);
            }
        }
        // Ningún dato sensible de la tesorería puede aparecer legible en el sobre.
        let wire = serde_json::to_string(&raw).unwrap();
        for secret in ["Dwarven anvil", "Erebor Forge", "Smithing", "Paid in pesos"] {
            assert!(
                !wire.contains(secret),
                "{secret} leaked in cleartext to the server"
            );
        }

        SyncEngine {
            db: &db_b,
            identity: &identity,
            device_id: "device-b",
            provider: Box::new(RecordingProvider {
                events: server.clone(),
                serve: true,
                project_scope_only: false,
                withhold: None,
            }),
        }
        .sync()
        .unwrap();

        assert_treasury_matches(
            &db_b,
            campaign_id,
            category_id,
            entry_id,
            &identity.public_key,
        );
        drop(db_a);
        drop(db_b);
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);
    }

    /// Un cambio de divisa aislado —sin ningún otro movimiento después— tiene que
    /// viajar solo: la campaña y cada fila reetiquetada deben llegar al otro equipo.
    #[test]
    fn currency_switch_alone_syncs_to_the_other_device() {
        let identity = test_identity();
        let campaign_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let (path_a, db_a) = treasury_test_db("treasury_sync_currency_only_a");
        let (path_b, db_b) = treasury_test_db("treasury_sync_currency_only_b");
        db_a.insert_project(&shared_campaign_project(campaign_id, &identity, false))
            .unwrap();
        db_a.insert_task(&campaign_task(task_id, campaign_id, &identity))
            .unwrap();
        let (_, entry_id) = seed_treasury(&db_a, campaign_id, task_id, &identity.public_key);

        let server = SharedEventLog::default();
        let sync = |db: &Database, device: &str, serve: bool| {
            SyncEngine {
                db,
                identity: &identity,
                device_id: device,
                provider: Box::new(RecordingProvider {
                    events: server.clone(),
                    serve,
                    project_scope_only: false,
                    withhold: None,
                }),
            }
            .sync()
            .unwrap();
        };

        sync(&db_a, "device-a", false);
        sync(&db_b, "device-b", true);
        let service_b = crate::services::TreasuryService::new(&db_b);
        assert_eq!(
            service_b.campaign_currency(campaign_id).unwrap(),
            crate::models::Currency::Mxn
        );

        // Único cambio de esta ronda: volver a USD. Nada más toca la tesorería.
        let before = server.0.lock().unwrap().len();
        crate::services::TreasuryService::new(&db_a)
            .set_currency(campaign_id, crate::models::Currency::Usd)
            .unwrap();
        sync(&db_a, "device-a", false);
        let pushed = server.0.lock().unwrap()[before..]
            .iter()
            .map(|event| event.entity_type.clone())
            .collect::<Vec<_>>();
        assert!(
            pushed.iter().any(|kind| kind == "campaign_treasury"),
            "the currency change alone was never pushed: {pushed:?}"
        );
        assert!(
            pushed.iter().any(|kind| kind == "ledger_entry"),
            "relabeled ledger rows were not pushed: {pushed:?}"
        );
        assert!(
            pushed.iter().any(|kind| kind == "task_financials"),
            "relabeled task financials were not pushed: {pushed:?}"
        );

        sync(&db_b, "device-b", true);
        assert_eq!(
            service_b.campaign_currency(campaign_id).unwrap(),
            crate::models::Currency::Usd
        );
        let entry = service_b.get_entry(entry_id).unwrap().unwrap();
        assert_eq!(entry.currency_code, "USD");
        // Reetiquetado, nunca convertido.
        assert_eq!(entry.amount_minor, 742_55);
        assert_eq!(
            service_b
                .get_task_financials(task_id)
                .unwrap()
                .unwrap()
                .currency_code,
            "USD"
        );
        drop(db_a);
        drop(db_b);
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);
    }

    /// El servidor devuelve 403 y revierte el lote entero si un Observer escribe en una
    /// ruta compartida. Sus eventos de tesorería no deben salir, o el sync se atasca.
    #[test]
    fn observer_treasury_edits_never_reach_a_shared_route() {
        let observer = test_identity();
        let owner = test_identity();
        let campaign_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let routing_id = Uuid::new_v4().to_string();
        let (path, db) = treasury_test_db("treasury_sync_observer_guard");
        db.insert_project(&shared_campaign_project(campaign_id, &owner, true))
            .unwrap();
        db.save_project_encryption_key_from_sync(
            &campaign_id.to_string(),
            &routing_id,
            &[13u8; 32],
        )
        .unwrap();
        db.conn
            .execute(
                "INSERT INTO project_members (project_id, user_identity, user_username, role)
                 VALUES (?1, ?2, 'Watcher', 'Observer')",
                params![campaign_id.to_string(), observer.public_key],
            )
            .unwrap();
        db.insert_task(&campaign_task(task_id, campaign_id, &owner))
            .unwrap();
        seed_treasury(&db, campaign_id, task_id, &observer.public_key);

        let server = SharedEventLog::default();
        SyncEngine {
            db: &db,
            identity: &observer,
            device_id: "observer-device",
            provider: Box::new(RecordingProvider {
                events: server.clone(),
                serve: false,
                project_scope_only: false,
                withhold: None,
            }),
        }
        .sync()
        .unwrap();

        let leaked = server
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.scope == "project")
            // El servidor sí acepta mensajes de crónica de un Observer; nada más.
            .filter(|event| event.entity_type != "chronicle_message")
            .map(|event| event.entity_type.clone())
            .collect::<Vec<_>>();
        assert!(
            leaked.is_empty(),
            "an Observer pushed shared-route events and would 403 the whole batch: {leaked:?}"
        );
        // Siguen pendientes en local: si el rol cambia a Companion, subirán entonces.
        assert!(!db.get_pending_sync_logs().unwrap().is_empty());
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    /// Un movimiento que llega antes que su tarea no se puede insertar por la llave
    /// foránea. No debe perderse: se reporta y se reintenta cuando la tarea ya existe.
    #[test]
    fn treasury_event_that_cannot_apply_yet_is_retried_not_dropped() {
        let identity = test_identity();
        let campaign_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let (path_a, db_a) = treasury_test_db("treasury_sync_retry_source");
        let (path_b, db_b) = treasury_test_db("treasury_sync_retry_target");
        db_a.insert_project(&shared_campaign_project(campaign_id, &identity, false))
            .unwrap();
        db_a.insert_task(&campaign_task(task_id, campaign_id, &identity))
            .unwrap();
        let (category_id, entry_id) =
            seed_treasury(&db_a, campaign_id, task_id, &identity.public_key);

        let server = SharedEventLog::default();
        SyncEngine {
            db: &db_a,
            identity: &identity,
            device_id: "device-a",
            provider: Box::new(RecordingProvider {
                events: server.clone(),
                serve: false,
                project_scope_only: false,
                withhold: None,
            }),
        }
        .sync()
        .unwrap();

        // El destino ya tiene la campaña, pero su tarea aún no llega: el asiento ligado
        // a ella y sus finanzas violan la llave foránea en este primer intento.
        db_b.insert_project(&shared_campaign_project(campaign_id, &identity, false))
            .unwrap();
        let pull = |db: &Database, withhold: Option<&'static str>| -> Vec<String> {
            SyncEngine {
                db,
                identity: &identity,
                device_id: "device-b",
                provider: Box::new(RecordingProvider {
                    events: server.clone(),
                    serve: true,
                    project_scope_only: false,
                    withhold,
                }),
            }
            .sync()
            .unwrap()
            .2
        };

        let conflicts = pull(&db_b, Some("task"));
        let service_b = crate::services::TreasuryService::new(&db_b);
        assert!(
            conflicts
                .iter()
                .any(|line| line.contains("could not be applied yet")),
            "a blocked treasury row must be reported, got {conflicts:?}"
        );
        assert!(
            service_b.get_entry(entry_id).unwrap().is_none(),
            "the entry cannot exist before its task"
        );

        // Llega la tarea que faltaba: el reintento tiene que completar la tesorería.
        db_b.insert_task(&campaign_task(task_id, campaign_id, &identity))
            .unwrap();
        pull(&db_b, None);
        assert_treasury_matches(
            &db_b,
            campaign_id,
            category_id,
            entry_id,
            &identity.public_key,
        );
        drop(db_a);
        drop(db_b);
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);
    }

    /// Campaña compartida: la tesorería se cifra con la llave de la Fellowship y otro
    /// miembro la reconstruye; sin esa llave el sobre es indescifrable.
    #[test]
    fn treasury_round_trips_through_fellowship_project_encryption() {
        let owner = test_identity();
        let companion = test_identity();
        let campaign_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let routing_id = Uuid::new_v4().to_string();
        let project_key = [11u8; 32];
        let (path_a, db_a) = treasury_test_db("treasury_sync_project_owner");
        let (path_b, db_b) = treasury_test_db("treasury_sync_project_companion");

        for db in [&db_a, &db_b] {
            db.insert_project(&shared_campaign_project(campaign_id, &owner, true))
                .unwrap();
            db.save_project_encryption_key_from_sync(
                &campaign_id.to_string(),
                &routing_id,
                &project_key,
            )
            .unwrap();
        }
        db_a.insert_task(&campaign_task(task_id, campaign_id, &owner))
            .unwrap();
        db_b.insert_task(&campaign_task(task_id, campaign_id, &owner))
            .unwrap();
        let (category_id, entry_id) = seed_treasury(&db_a, campaign_id, task_id, &owner.public_key);

        let server = SharedEventLog::default();
        SyncEngine {
            db: &db_a,
            identity: &owner,
            device_id: "owner-device",
            provider: Box::new(RecordingProvider {
                events: server.clone(),
                serve: false,
                project_scope_only: false,
                withhold: None,
            }),
        }
        .sync()
        .unwrap();

        let raw = server.0.lock().unwrap().clone();
        let treasury_events = raw
            .iter()
            .filter(|event| {
                matches!(
                    event.entity_type.as_str(),
                    "campaign_treasury"
                        | "ledger_category"
                        | "category_budget"
                        | "ledger_entry"
                        | "task_financials"
                )
            })
            .collect::<Vec<_>>();
        assert!(!treasury_events.is_empty());
        for event in &treasury_events {
            assert_eq!(event.key_id, "project-v1");
            assert_eq!(event.scope, "project");
            assert_eq!(event.routing_id, routing_id);
            assert!(event.content.is_none());
        }

        // Sin la llave de la Fellowship el sobre no se abre.
        let sample = treasury_events
            .iter()
            .find(|event| event.entity_type == "ledger_entry")
            .unwrap();
        let aad = crate::services::encryption::associated_data_for_route(
            &sample.id,
            &sample.entity_type,
            &sample.entity_id,
            &sample.operation,
            &sample.timestamp,
            &sample.key_id,
            &sample.scope,
            &sample.routing_id,
        );
        assert!(
            crate::services::encryption::decrypt_with_project_key(
                &[12u8; 32],
                &sample.nonce,
                &sample.ciphertext,
                &aad,
            )
            .is_err(),
            "a wrong Fellowship key must not open a treasury event"
        );
        assert!(
            crate::services::encryption::decrypt_with_project_key(
                &project_key,
                &sample.nonce,
                &sample.ciphertext,
                &aad,
            )
            .is_ok()
        );

        SyncEngine {
            db: &db_b,
            identity: &companion,
            device_id: "companion-device",
            provider: Box::new(RecordingProvider {
                events: server.clone(),
                serve: true,
                project_scope_only: true,
                withhold: None,
            }),
        }
        .sync()
        .unwrap();

        assert_treasury_matches(&db_b, campaign_id, category_id, entry_id, &owner.public_key);
        drop(db_a);
        drop(db_b);
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);
    }
}
