// ─────────────────────────────────────────────────────────────────────────────
// services/config.rs — carga y guarda la configuración del app en TOML
// ─────────────────────────────────────────────────────────────────────────────

use crate::storage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub sync_enabled: bool,
    pub auto_sync: bool,
    pub sync_interval_minutes: u64,
    // "none" | "achievements" | "everything"
    #[serde(default = "Config::default_share_level")]
    pub chronicle_share_level: String,
}

impl Config {
    fn default_share_level() -> String {
        "everything".to_string()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: "https://questlinecli.com/api".to_string(),
            sync_enabled: true,
            auto_sync: true,
            sync_interval_minutes: 5,
            chronicle_share_level: Self::default_share_level(),
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        let storage_dir = storage::get_storage_dir()?;
        let target_path = storage_dir.join("config.toml");
        let serialized = toml::to_string_pretty(self)?;
        std::fs::write(target_path, serialized)?;
        Ok(())
    }

    // No manches, tres lugares donde puede vivir el config: storage dir, directorio actual, o nada.
    // Si no encuentra nada, escribe el default solito y ya — no hay excusa para andar sin config.
    pub fn load() -> Result<Self> {
        let storage_dir = storage::get_storage_dir()?;
        let target_path = storage_dir.join("config.toml");

        if target_path.exists() {
            let content = std::fs::read_to_string(&target_path)?;
            if let Ok(config) = toml::from_str::<Config>(&content) {
                return Ok(config);
            }
        }

        // Checa el directorio actual por si el config está ahí
        let local_path = PathBuf::from("config.toml");
        if local_path.exists() {
            let content = std::fs::read_to_string(&local_path)?;
            if let Ok(config) = toml::from_str::<Config>(&content) {
                // Lo copia al storage para que persista aunque corras el app desde otro lugar
                if let Some(parent) = target_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&target_path, &content);
                return Ok(config);
            }
        }

        // Si no encontró nada, escribe la config por default y listo
        let default_config = Config::default();
        if let Some(parent) = target_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let serialized = toml::to_string_pretty(&default_config)?;
        std::fs::write(&target_path, serialized)?;

        Ok(default_config)
    }
}
