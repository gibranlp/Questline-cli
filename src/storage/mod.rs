// ─────────────────────────────────────────────────────────────────────────────
// storage/mod.rs — resuelve la ruta del directorio de datos del usuario
// ─────────────────────────────────────────────────────────────────────────────
use anyhow::{Result, anyhow};
use directories::BaseDirs;
use std::path::PathBuf;
use std::sync::OnceLock;

static ACTIVE_PROFILE: OnceLock<String> = OnceLock::new();

fn validate_profile_name(profile: &str) -> Result<String> {
    let profile = profile.trim();
    if profile.is_empty() || profile.len() > 32 {
        return Err(anyhow!(
            "Profile names must contain between 1 and 32 characters"
        ));
    }
    if !profile
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(anyhow!(
            "Profile names may contain only ASCII letters, numbers, '-' and '_'"
        ));
    }
    Ok(profile.to_ascii_lowercase())
}

/// Selects a process-wide test profile before any storage-backed service starts.
/// Ordinary launches never call this and retain the historical storage location.
pub fn set_profile(profile: &str) -> Result<()> {
    let profile = validate_profile_name(profile)?;
    if let Some(current) = ACTIVE_PROFILE.get() {
        if current != &profile {
            return Err(anyhow!("Questline profile is already set to '{current}'"));
        }
        return Ok(());
    }
    ACTIVE_PROFILE
        .set(profile)
        .map_err(|_| anyhow!("Could not select the Questline profile"))
}

pub fn active_profile() -> Option<&'static str> {
    ACTIVE_PROFILE.get().map(String::as_str)
}

fn with_active_profile(base: PathBuf) -> PathBuf {
    match active_profile() {
        Some(profile) => base.join("profiles").join(profile),
        None => base,
    }
}

// Resolves and returns the path to the application's config directory (~/.config/questline/).
pub fn get_storage_dir() -> Result<PathBuf> {
    if let Some(override_dir) = std::env::var_os("QUESTLINE_STORAGE_DIR") {
        let override_dir = PathBuf::from(override_dir);
        if !override_dir.is_absolute() {
            return Err(anyhow!("QUESTLINE_STORAGE_DIR must be an absolute path"));
        }
        return Ok(with_active_profile(override_dir));
    }

    if let Some(base_dirs) = BaseDirs::new() {
        let mut config_dir = base_dirs.config_dir().to_path_buf();
        config_dir.push("questline");
        Ok(with_active_profile(config_dir))
    } else {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
            .map_err(|_| anyhow!("Could not resolve user home directory"))?;
        let mut config_dir = home;
        config_dir.push(".config");
        config_dir.push("questline");
        Ok(with_active_profile(config_dir))
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

#[cfg(test)]
mod tests {
    use super::validate_profile_name;

    #[test]
    fn profile_names_are_safe_and_canonical() {
        assert_eq!(
            validate_profile_name("  Fellowship_A-1  ").unwrap(),
            "fellowship_a-1"
        );
        assert!(validate_profile_name("").is_err());
        assert!(validate_profile_name("../default").is_err());
        assert!(validate_profile_name("name with spaces").is_err());
        assert!(validate_profile_name(&"a".repeat(33)).is_err());
    }
}
