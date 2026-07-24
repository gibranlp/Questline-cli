// ─────────────────────────────────────────────────────────────────────────────
// sync_engine.rs — el corazón del sync: push primero, pull después, nunca al revés
// ─────────────────────────────────────────────────────────────────────────────
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::database::Database;
use crate::models::{Note, Task};
use crate::services::Identity;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncLogEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: String,
    pub timestamp: String,
    pub content: Option<String>,
    #[serde(default)]
    pub device_id: String,
    // seq — cursor incremental del servidor para no descargar toda la historia en cada sync
    #[serde(default)]
    pub seq: i64,
}

pub trait CloudProvider {
    fn name(&self) -> &str;
    fn push(&self, public_key: &str, signature: &str, payload: &str) -> Result<()>;
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
}

use crate::services::ApiClient;

pub struct HttpCloudProvider {
    pub client: ApiClient,
}

impl CloudProvider for HttpCloudProvider {
    fn name(&self) -> &str {
        "Cloud Chronicle (HTTPS REST)"
    }

    fn push(&self, _public_key: &str, _signature: &str, payload: &str) -> Result<()> {
        self.client.send_request("POST", "sync/push", payload)?;
        Ok(())
    }

    fn pull(&self, _public_key: &str, _signature: &str, since_seq: i64) -> Result<String> {
        self.client.send_request("POST", &format!("sync/pull?since_seq={}", since_seq), "")
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

    // CRÍTICO: el orden push-antes-pull no es negociable — así los cambios locales no se pisan con lo remoto
    pub fn sync(&self) -> Result<(usize, usize, Vec<String>)> {
        let mut conflicts = Vec::new();
        let mut pushed_count = 0;
        let mut pulled_count = 0;

        let pending = self.db.get_pending_sync_logs()?;
        let mut local_payload = Vec::new();

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
                "note" => self
                    .db
                    .get_note_by_id(uuid)
                    .ok()
                    .and_then(|n| serde_json::to_string(&n).ok()),
                "project" => {
                    let mut stmt = self.db.conn.prepare("SELECT id, name, description, created_at, archived, completed, owner_identity, owner_username, is_shared FROM projects WHERE id = ?1")?;
                    stmt.query_row(params![entity_id], |row| {
                        let id_str: String = row.get(0)?;
                        let name: String = row.get(1)?;
                        let desc: Option<String> = row.get(2)?;
                        let created: String = row.get(3)?;
                        let archived: i32 = row.get(4)?;
                        let completed: i32 = row.get(5)?;
                        let owner_id: Option<String> = row.get(6)?;
                        let owner_name: Option<String> = row.get(7)?;
                        let is_shared: i32 = row.get(8)?;
                        Ok(serde_json::json!({
                            "id": id_str,
                            "name": name,
                            "description": desc,
                            "created_at": created,
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
                    let mut stmt = self.db.conn.prepare("SELECT id, project_id, entry_date, content, created_at, visibility, author_username FROM journal_entries WHERE id = ?1")?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let pid: String = row.get(1)?;
                        let date: String = row.get(2)?;
                        let content: String = row.get(3)?;
                        let created: String = row.get(4)?;
                        let visibility: String = row.get(5)?;
                        let author: String = row.get(6)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "project_id": pid,
                            "entry_date": date,
                            "content": content,
                            "created_at": created,
                            "visibility": visibility,
                            "author_username": author,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "milestone" => {
                    let mut stmt = self.db.conn.prepare("SELECT id, project_id, name, description, completed, xp_reward, created_at, tier, template_id FROM milestones WHERE id = ?1")?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let pid: String = row.get(1)?;
                        let name: String = row.get(2)?;
                        let desc: Option<String> = row.get(3)?;
                        let completed: i32 = row.get(4)?;
                        let xp: i32 = row.get(5)?;
                        let created: String = row.get(6)?;
                        let tier: i32 = row.get(7)?;
                        let template_id: String = row.get(8)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "project_id": pid,
                            "name": name,
                            "description": desc,
                            "completed": completed != 0,
                            "xp_reward": xp,
                            "created_at": created,
                            "tier": tier,
                            "template_id": template_id,
                        })
                        .to_string())
                    })
                    .ok()
                }
                "lore_unlock" => {
                    let mut stmt = self.db.conn.prepare("SELECT id, unlocked, unlocked_at FROM lore_library WHERE id = ?1")?;
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
                        "SELECT id, name, description, frequency, reward_xp, created_at, daily_target FROM rituals WHERE id = ?1",
                    )?;
                    stmt.query_row(params![entity_id], |row| {
                        let id: String = row.get(0)?;
                        let name: String = row.get(1)?;
                        let desc: Option<String> = row.get(2)?;
                        let freq: String = row.get(3)?;
                        let xp: i32 = row.get(4)?;
                        let created: String = row.get(5)?;
                        let daily_target: i32 = row.get(6)?;
                        Ok(serde_json::json!({
                            "id": id,
                            "name": name,
                            "description": desc,
                            "frequency": freq,
                            "reward_xp": xp,
                            "created_at": created,
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
                        Some(serde_json::json!({
                            "ritual_id": parts[0],
                            "completed_date": parts[1],
                        }).to_string())
                    } else {
                        None
                    }
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
                            }).to_string())
                        }).ok()
                    } else { None }
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
                            }).to_string())
                        }).ok()
                    } else { None }
                }
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
                        }).to_string())
                    }).ok()
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
                "zen_tree" => self.db.get_zen_tree().ok().and_then(|t| serde_json::to_string(&t).ok()),
                _ => None,
            };

            local_payload.push(SyncLogEntry {
                id: log_id,
                entity_type,
                entity_id,
                operation,
                timestamp,
                content,
                device_id: self.device_id.to_string(),
                seq: 0, // el servidor sobreescribe esto con el seq real al insertar
            });
        }

        // Heartbeat del dispositivo — lleva identidad del usuario para actualizar presencia en otros nodos
        {
            let now_str = Utc::now().to_rfc3339();
            let username = self.db.get_user()
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
            }).to_string();
            local_payload.push(SyncLogEntry {
                id: format!("device_heartbeat__{}__{}", self.device_id, Utc::now().format("%Y%m%d_%H%M")),
                entity_type: "device".to_string(),
                entity_id: self.device_id.to_string(),
                operation: "heartbeat".to_string(),
                timestamp: now_str,
                content: Some(device_info),
                device_id: self.device_id.to_string(),
                seq: 0,
            });
        }

        // Si falla el push, no jalamos nada — así no pisamos cambios que aún no subimos
        let pushed_ids: Vec<String> = if !local_payload.is_empty() {
            let serialized = serde_json::to_string(&local_payload)?;
            let signature = self.identity.sign(serialized.as_bytes())?;
            self.provider
                .push(&self.identity.public_key, &signature, &serialized)?;
            pushed_count = local_payload.len();
            local_payload.iter().map(|l| l.id.clone()).collect()
        } else {
            Vec::new()
        };

        // El cursor `since_seq` evita descargar toda la historia en cada sync
        let since_seq: i64 = self.db
            .get_setting("last_pull_seq")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let pulled_data = self.provider.pull(&self.identity.public_key, "", since_seq)?;
        let remote_logs: Vec<SyncLogEntry> = serde_json::from_str(&pulled_data)?;

        // Dedup por ID — si el servidor no retorna seq reales (todos llegan con seq=0), este set
        // evita reprocesar eventos que ya aplicamos en sesiones anteriores, sea cual sea el cursor
        let already_processed = self.db.load_processed_remote_ids().unwrap_or_default();
        let mut newly_processed_ids: Vec<String> = Vec::new();

        // Estrategia de conflictos: Latest Edit Wins, con la versión perdedora guardada en revisiones
        let mut max_seq: i64 = since_seq;
        for log in remote_logs {
            if log.seq > max_seq { max_seq = log.seq; }
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
                    // Los deletes (tombstones) siempre se aplican — no tienen conflicto posible
                    if log.operation == "delete" { true } else
                    { match self.db.get_task_by_id(ent_uuid) { Ok(local_task) => {
                        // El más reciente gana — sin democracia
                        let incoming_time = DateTime::parse_from_rfc3339(&log.timestamp)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or(DateTime::<Utc>::from(std::time::UNIX_EPOCH));

                        if incoming_time > local_task.updated_at {
                            if let Some(ref content) = log.content {
                                if let Ok(remote_task) = serde_json::from_str::<Task>(content) {
                                    if remote_task.title != local_task.title
                                        || remote_task.completed != local_task.completed
                                    {
                                        // Conflicto real — guardamos la versión local antes de pisarla
                                        if let Ok(local_json) = serde_json::to_string(&local_task) {
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
                                }
                            }
                            true
                        } else {
                            false
                        }
                    } _ => {
                        true
                    }}}
                }
                // Notas: mismo juego que tasks — timestamp gana, pero guardamos la versión local si hay conflicto
                "note" => {
                    if log.operation == "delete" { true } else {
                    match self.db.get_note_by_id(ent_uuid) { Ok(local_note) => {
                        let incoming_time = DateTime::parse_from_rfc3339(&log.timestamp)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or(DateTime::<Utc>::from(std::time::UNIX_EPOCH));
                        if incoming_time > local_note.updated_at {
                            if let Some(ref content) = log.content {
                                if let Ok(remote_note) = serde_json::from_str::<Note>(content) {
                                    if remote_note.title != local_note.title
                                        || remote_note.markdown_content
                                            != local_note.markdown_content
                                    {
                                        // Conflicto en nota — archivamos lo local antes de sobreescribir
                                        if let Ok(local_json) = serde_json::to_string(&local_note) {
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
                                }
                            }
                            true
                        } else {
                            false
                        }
                    } _ => {
                        true
                    }}}
                }
                // Proyectos: siempre aplicamos — INSERT OR REPLACE propaga renombres, archivado, etc.
                "project" => true,
                // Entradas de journal: siempre aceptamos — INSERT OR REPLACE en el pull maneja el upsert
                "journal_entry" => true,
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
                "ritual" => true,
                "ritual_history" => {
                    // Misma convención de formato compuesto que arriba — hay que partir el id
                    let parts: Vec<&str> = log.entity_id.splitn(2, "__").collect();
                    if parts.len() == 2 {
                        let exists = self.db.conn.query_row(
                            "SELECT count(*) FROM ritual_history WHERE ritual_id = ?1 AND completed_date = ?2",
                            params![parts[0], parts[1]],
                            |row| row.get::<_, i32>(0),
                        ).unwrap_or(0) > 0;
                        !exists
                    } else {
                        false
                    }
                }
                // Codex: siempre aplicamos — INSERT OR REPLACE propaga renombres y cambios de parent
                "codex" => true,
                // Las sesiones de focus son inmutables una vez completadas — nunca se actualizan
                "focus_session" => {
                    self.db.conn.query_row(
                        "SELECT count(*) FROM focus_sessions WHERE id = ?1",
                        params![log.entity_id],
                        |row| row.get::<_, i32>(0),
                    ).unwrap_or(0) == 0
                }
                "task_assignment" => {
                    true
                }
                "project_member" => {
                    true
                }
                // Los mensajes de la crónica son inmutables — nunca se editan, solo se insertan
                "chronicle_message" => {
                    self.db.conn.query_row(
                        "SELECT count(*) FROM chronicle_messages WHERE id = ?1",
                        params![log.entity_id],
                        |row| row.get::<_, i32>(0),
                    ).unwrap_or(0) == 0
                }
                // Lore: los desbloqueos no se revierten — solo aplicamos si el remoto dice desbloqueado y el local aún no
                "lore_unlock" => {
                    let remote_unlocked = log.content.as_ref()
                        .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                        .map(|v| v["unlocked"].as_bool().unwrap_or(false))
                        .unwrap_or(false);
                    if !remote_unlocked {
                        false
                    } else {
                        self.db.conn.query_row(
                            "SELECT unlocked FROM lore_library WHERE id = ?1",
                            params![log.entity_id],
                            |row| row.get::<_, i32>(0),
                        ).unwrap_or(1) == 0
                    }
                }
                // Árbol zen: latest-watered timestamp wins — es una sola fila global por usuario
                "zen_tree" => {
                    let incoming_time = DateTime::parse_from_rfc3339(&log.timestamp)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or(DateTime::<Utc>::from(std::time::UNIX_EPOCH));
                    match self.db.get_zen_tree() {
                        Ok(lt) => lt.last_watered.map(|lw| incoming_time > lw).unwrap_or(true),
                        Err(_) => true,
                    }
                }
                // Milestones: siempre aplicamos — INSERT OR REPLACE maneja create y update
                "milestone" => true,
                // Dispositivos: siempre aplicamos — upsert idempotente
                "device" => true,
                _ => false,
            };

            if is_newer {
                if let Some(ref content) = log.content {
                    match log.entity_type.as_str() {
                        "user" => {
                            if let Ok(u) = serde_json::from_str::<crate::models::User>(content) {
                                // Clear the table first to enforce singleton constraint and avoid duplicating users
                                let _ = self.db.conn.execute("DELETE FROM users", []);
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO users (id, username, class, level, xp, created_at, specialization) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
                                pulled_count += 1;
                            }
                        }
                        "task" => {
                            if log.operation == "delete" {
                                let _ = self.db.conn.execute(
                                    "DELETE FROM tasks WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(t) = serde_json::from_str::<Task>(content) {
                                // Triquete de completado: si ya está completa localmente, no la regresamos a incompleta
                                let local_task = self.db.get_task_by_id(ent_uuid).ok();
                                let local_completed = local_task
                                    .as_ref()
                                    .map(|lt| lt.completed)
                                    .unwrap_or(false);
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
                                        "INSERT OR REPLACE INTO tasks (id, project_id, title, description, due_date, set_date, completed, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded, recurrence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                                        let assigned = self.db.get_task_assignments(&t.id.to_string()).unwrap_or_default();
                                        if assigned.iter().any(|(id, _)| id == my_key) {
                                            if let Ok(Some(mut user)) = self.db.get_user() {
                                                let xp = if t.priority == crate::models::TaskPriority::High { 50 } else { 25 };
                                                let xp_svc = crate::services::XPService::new(self.db);
                                                let _ = xp_svc.grant_xp(&mut user, "Complete Shared Task Quest", xp);
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
                                let _ = self.db.conn.execute(
                                    "DELETE FROM notes WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(n) = serde_json::from_str::<Note>(content) {
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO notes (id, project_id, title, markdown_content, created_at, updated_at, sharing_permission, codex_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                    params![
                                        n.id.to_string(),
                                        n.project_id.map(|id| id.to_string()),
                                        n.title,
                                        n.markdown_content,
                                        n.created_at.to_rfc3339(),
                                        n.updated_at.to_rfc3339(),
                                        n.sharing_permission,
                                        n.codex_id.map(|id| id.to_string()),
                                    ],
                                );
                                pulled_count += 1;
                            }
                        }
                        "project" => {
                            if log.operation == "delete" {
                                let _ = self.db.conn.execute(
                                    "DELETE FROM projects WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(p) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = p["id"].as_str().unwrap_or_default();
                                let name = p["name"].as_str().unwrap_or_default();
                                let desc = p["description"].as_str();
                                let created = p["created_at"].as_str().unwrap_or_default();
                                let archived = p["archived"].as_bool().unwrap_or(false);
                                let completed = p["completed"].as_bool().unwrap_or(false);
                                let owner_id = p["owner_identity"].as_str();
                                let owner_name = p["owner_username"].as_str();
                                let is_shared = p["is_shared"].as_bool().unwrap_or(false);
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO projects (id, name, description, created_at, archived, completed, owner_identity, owner_username, is_shared) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                    params![
                                        id, name, desc, created,
                                        if archived { 1 } else { 0 },
                                        if completed { 1 } else { 0 },
                                        owner_id, owner_name,
                                        if is_shared { 1 } else { 0 }
                                    ],
                                );
                                // When a project is shared, ensure owner appears in project_members
                                if is_shared {
                                    let owner_id_str = p["owner_identity"].as_str().unwrap_or_default();
                                    let owner_name_str = p["owner_username"].as_str().unwrap_or_default();
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
                        "journal_entry" => {
                            if let Ok(j) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = j["id"].as_str().unwrap_or_default();
                                let pid = j["project_id"].as_str().unwrap_or_default();
                                let date = j["entry_date"].as_str().unwrap_or_default();
                                let body = j["content"].as_str().unwrap_or_default();
                                let created = j["created_at"].as_str().unwrap_or_default();
                                let visibility = j["visibility"].as_str().unwrap_or("Private");
                                let author = j["author_username"].as_str().unwrap_or("");

                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO journal_entries (id, project_id, entry_date, content, created_at, visibility, author_username) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                    params![id, pid, date, body, created, visibility, author],
                                );
                                pulled_count += 1;
                            }
                        }
                        "milestone" => {
                            if log.operation == "delete" {
                                let _ = self.db.conn.execute(
                                    "DELETE FROM milestones WHERE id = ?1",
                                    params![log.entity_id],
                                );
                                pulled_count += 1;
                            } else if let Ok(m) = serde_json::from_str::<serde_json::Value>(content) {
                                let id = m["id"].as_str().unwrap_or_default();
                                let pid = m["project_id"].as_str().unwrap_or_default();
                                let name = m["name"].as_str().unwrap_or_default();
                                let desc = m["description"].as_str();
                                let completed = m["completed"].as_bool().unwrap_or(false);
                                let xp = m["xp_reward"].as_i64().unwrap_or(0) as i32;
                                let created = m["created_at"].as_str().unwrap_or_default();
                                let tier = m["tier"].as_i64().unwrap_or(0) as i32;
                                let template_id = m["template_id"].as_str().unwrap_or("");

                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO milestones (id, project_id, name, description, completed, xp_reward, created_at, tier, template_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                    params![id, pid, name, desc, if completed { 1 } else { 0 }, xp, created, tier, template_id],
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
                                let daily_target = r["daily_target"].as_i64().unwrap_or(1) as i32;
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO rituals (id, name, description, frequency, reward_xp, created_at, daily_target) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                    params![id, name, desc, freq, xp, created, daily_target],
                                );
                                pulled_count += 1;
                            }
                        }
                        "ritual_history" => {
                            if let Ok(rh) = serde_json::from_str::<serde_json::Value>(content) {
                                let ritual_id = rh["ritual_id"].as_str().unwrap_or_default();
                                let completed_date = rh["completed_date"].as_str().unwrap_or_default();
                                let _ = self.db.conn.execute(
                                    "INSERT OR IGNORE INTO ritual_history (ritual_id, completed_date) VALUES (?1, ?2)",
                                    params![ritual_id, completed_date],
                                );
                                pulled_count += 1;
                            }
                        }
                        "codex" => {
                            if log.operation == "delete" {
                                let _ = self.db.conn.execute(
                                    "DELETE FROM codices WHERE id = ?1",
                                    params![log.entity_id],
                                );
                            } else if let Ok(c) = serde_json::from_str::<crate::models::Codex>(content) {
                                let _ = self.db.conn.execute(
                                    "INSERT OR REPLACE INTO codices (id, project_id, name, created_at, parent_codex_id, collapsed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                                    params![
                                        c.id.to_string(),
                                        c.project_id.to_string(),
                                        c.name,
                                        c.created_at.to_rfc3339(),
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
                            } else if let Ok(ta) = serde_json::from_str::<serde_json::Value>(content) {
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
                            } else if let Ok(pm) = serde_json::from_str::<serde_json::Value>(content) {
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
                                let sender_name = cm["sender_username"].as_str().unwrap_or_default();
                                let msg_content = cm["content"].as_str().unwrap_or_default();
                                let msg_type = cm["message_type"].as_str().unwrap_or("Text");
                                let timestamp = cm["timestamp"].as_str().unwrap_or_default();
                                let _ = self.db.conn.execute(
                                    "INSERT OR IGNORE INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                    params![id, project_id, sender_id, sender_name, msg_content, msg_type, timestamp],
                                );
                                pulled_count += 1;
                            }
                        }
                        // Árbol zen: aplicamos directamente sin pasar por update_zen_tree() para no
                        // disparar log_change() de nuevo y crear un loop de sync
                        "zen_tree" => {
                            if let Ok(t) = serde_json::from_str::<crate::models::ZenTree>(content) {
                                let local_water_today = self.db.get_zen_tree()
                                    .map(|lt| lt.water_today)
                                    .unwrap_or(0);
                                let _ = self.db.conn.execute(
                                    "UPDATE zen_tree SET growth = ?1, health = ?2, stage = ?3, last_watered = ?4, water_today = ?5, total_waterings = ?6 WHERE id = ?7",
                                    params![
                                        t.growth,
                                        t.health,
                                        t.stage,
                                        t.last_watered.map(|dt| dt.to_rfc3339()),
                                        t.water_today.max(local_water_today),
                                        t.total_waterings,
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
                                    let user_identity = v["user_identity"].as_str().unwrap_or_default();
                                    let username = v["username"].as_str().unwrap_or_default();
                                    if !user_identity.is_empty() && !username.is_empty() {
                                        let seen = last.unwrap_or_else(|| log.timestamp.as_str());
                                        let _ = self.db.update_presence(
                                            user_identity, username, true, seen, None, "Visible",
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
            // Marcar como procesado independientemente de si `is_newer` — queremos dedup siempre
            newly_processed_ids.push(log.id);
        }

        // Avanzamos el cursor — la próxima vez solo jalamos lo que llegó después de este punto
        if max_seq > since_seq {
            let _ = self.db.set_setting("last_pull_seq", &max_seq.to_string());
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

        Ok((pushed_count, pulled_count, conflicts))
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
        let setting_key = format!("task_notify:fellowship_created:{}", dedup_key);
        if self.db.get_setting(&setting_key).ok().flatten().is_some() {
            return;
        }
        let _ = self.db.set_setting(&setting_key, "1");
        let _ = self
            .db
            .create_notification(notif_type, title, content, Some(task_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::{Note, Task, TaskPriority};

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
        assert!(entries.is_ok(), "Failed to deserialize SyncLogEntry missing device_id field");
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].device_id, "", "Expected device_id to default to an empty string");
    }
}
