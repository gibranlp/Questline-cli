// ─────────────────────────────────────────────────────────────────────────────
// services/lore_manager.rs — Descarga y cachea el lore desde questlinecli.com
//
// El lore ya no vive en el binario. Se descarga una vez por sesión y se persiste
// en disco (~/.questline/lore_cache.json). Si no hay red, se usa el caché local.
// Esto permite agregar nuevas entradas sin recompilar ni distribuir la app.
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LORE_URL:   &str = "https://questlinecli.com/data/lore.json";
const QUESTS_URL: &str = "https://questlinecli.com/data/quests.json";

// Tiempo máximo de vida del caché en segundos (1 hora)
const CACHE_TTL_SECS: i64 = 3_600;

// ── Estructuras del JSON ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreUnlock {
    #[serde(rename = "type")]
    pub unlock_type:  String,
    pub level:        Option<i32>,
    pub class:        Option<String>,
    pub milestone_id: Option<String>,
    pub chapter_id:   Option<String>,
    pub display:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreEntry {
    pub id:          String,
    pub category:    String,
    pub title:       String,
    pub content:     String,
    pub class_filter:Option<String>,
    pub unlock:      LoreUnlock,
    pub rarity:      Option<String>,
    pub sort_order:  i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    #[serde(rename = "type")]
    pub obj_type: String,
    pub target:   i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassQuest {
    pub class:          String,
    pub level:          i32,
    pub name:           String,
    pub description:    String,
    pub objective:      QuestObjective,
    pub lore_reward:    String,
    pub reward_lore_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoreFile {
    entries: Vec<LoreEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuestsFile {
    quests: Vec<ClassQuest>,
}

// Estructura que se guarda en disco como caché unificado
#[derive(Debug, Serialize, Deserialize)]
struct LoreCache {
    fetched_at: i64,  // unix timestamp
    lore:       Vec<LoreEntry>,
    quests:     Vec<ClassQuest>,
}

pub struct LoreManager {
    pub lore:   Vec<LoreEntry>,
    pub quests: Vec<ClassQuest>,
}

impl LoreManager {
    // Ruta del caché en disco — mismo directorio que la DB (~/.config/questline/)
    fn cache_path() -> PathBuf {
        crate::storage::get_storage_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("lore_cache.json")
    }

    // Lee el caché local; devuelve None si está expirado o no existe
    fn load_cache() -> Option<LoreCache> {
        let path = Self::cache_path();
        let data = std::fs::read_to_string(&path).ok()?;
        let cache: LoreCache = serde_json::from_str(&data).ok()?;

        let age = Utc::now().timestamp() - cache.fetched_at;
        if age > CACHE_TTL_SECS { return None; }

        Some(cache)
    }

    // Persiste el caché en disco — falla silenciosamente
    fn save_cache(lore: &[LoreEntry], quests: &[ClassQuest]) {
        let cache = LoreCache {
            fetched_at: Utc::now().timestamp(),
            lore:       lore.to_vec(),
            quests:     quests.to_vec(),
        };
        let path = Self::cache_path();
        if let Ok(json) = serde_json::to_string(&cache) {
            let _ = std::fs::write(&path, json);
        }
    }

    // Descarga un JSON desde la URL dada — timeout de 5 segundos para no bloquear el arranque
    fn fetch_json(url: &str) -> Result<String> {
        let resp = ureq::get(url)
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .context("HTTP request failed")?;
        resp.into_string().context("Failed to read response body")
    }

    // Intenta descargar lore y quests; si falla devuelve los valores del caché o vacío
    pub fn load() -> Self {
        // Primero intenta usar el caché vigente para no bloquear el arranque
        if let Some(cache) = Self::load_cache() {
            // Lanza una descarga en background para refrescar el caché (fire-and-forget)
            std::thread::spawn(|| {
                let _ = Self::fetch_and_save();
            });
            return Self { lore: cache.lore, quests: cache.quests };
        }

        // Sin caché válido: descarga bloqueante (solo ocurre la primera vez)
        match Self::fetch_and_save() {
            Ok((lore, quests)) => Self { lore, quests },
            Err(_) => {
                // Sin red y sin caché — inicia vacío; las tablas de la DB ya tienen INSERT OR IGNORE
                Self { lore: vec![], quests: vec![] }
            }
        }
    }

    // Descarga ambos archivos, guarda el caché y devuelve los datos
    fn fetch_and_save() -> Result<(Vec<LoreEntry>, Vec<ClassQuest>)> {
        let lore_json   = Self::fetch_json(LORE_URL)?;
        let quests_json = Self::fetch_json(QUESTS_URL)?;

        let lore_file:   LoreFile   = serde_json::from_str(&lore_json).context("Invalid lore.json")?;
        let quests_file: QuestsFile = serde_json::from_str(&quests_json).context("Invalid quests.json")?;

        Self::save_cache(&lore_file.entries, &quests_file.quests);

        Ok((lore_file.entries, quests_file.quests))
    }
}
