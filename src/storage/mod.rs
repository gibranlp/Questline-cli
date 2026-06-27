// ─────────────────────────────────────────────────────────────────────────────
// storage/mod.rs — resuelve la ruta del directorio de datos del usuario
// ─────────────────────────────────────────────────────────────────────────────
use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::path::PathBuf;

// Resolves and returns the path to the application's config directory (~/.config/questline/).
pub fn get_storage_dir() -> Result<PathBuf> {
    if let Some(base_dirs) = BaseDirs::new() {
        let mut config_dir = base_dirs.config_dir().to_path_buf();
        config_dir.push("questline");
        Ok(config_dir)
    } else {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
            .map_err(|_| anyhow!("Could not resolve user home directory"))?;
        let mut config_dir = home;
        config_dir.push(".config");
        config_dir.push("questline");
        Ok(config_dir)
    }
}

// Ensures that the storage directory exists on disk.
pub fn ensure_storage_dir_exists() -> Result<PathBuf> {
    let dir = get_storage_dir()?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}
