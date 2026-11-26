#[cfg(test)]
thread_local! {
    static TEST_STASH_MODE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub struct LegacyMusicTestGuard(Option<bool>);

#[cfg(test)]
impl Drop for LegacyMusicTestGuard {
    fn drop(&mut self) {
        TEST_STASH_MODE.set(self.0);
    }
}

#[cfg(test)]
pub fn legacy_music_test_mode() -> LegacyMusicTestGuard {
    LegacyMusicTestGuard(TEST_STASH_MODE.replace(Some(false)))
}

pub fn uses_stash() -> bool {
    #[cfg(test)]
    if let Some(mode) = TEST_STASH_MODE.get() {
        return mode;
    }
    stash_mode(
        std::env::var("SPECTRUM_STASH_PLAYER").ok().as_deref(),
        cfg!(target_os = "linux"),
    )
}

fn stash_mode(preference: Option<&str>, integration_available: bool) -> bool {
    match preference {
        Some("1") => true,
        Some("0") => false,
        _ => integration_available,
    }
}

pub fn is_stash_bus(name: &str) -> bool {
    name == "stash" || name.starts_with("stash.instance")
}

#[cfg(target_os = "linux")]
fn selected_player(finder: &mpris::PlayerFinder) -> Option<mpris::Player> {
    if uses_stash() {
        finder.find_all().ok()?.into_iter().find(|player| is_stash_bus(player.bus_name_trimmed()))
    } else {
        finder.find_active().ok()
    }
}

pub fn open_stash() -> Result<(), String> {
    std::process::Command::new("/usr/lib/spectrumos/stash-open")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|mut child| { std::thread::spawn(move || { let _ = child.wait(); }); })
        .map_err(|error| format!("Could not open Stash: {error}"))
}

#[cfg(target_os = "linux")]
pub fn stash_action(action: &str) -> Result<(), String> {
    let finder = mpris::PlayerFinder::new().map_err(|error| error.to_string())?;
    let player = finder.find_all().map_err(|error| error.to_string())?
        .into_iter().find(|player| is_stash_bus(player.bus_name_trimmed()))
        .ok_or_else(|| "Open Stash first with [o].".to_string())?;
    match action {
        "toggle" => player.play_pause(),
        "next" => player.next(),
        "previous" => player.previous(),
        "pause" => player.pause(),
        _ => return Err("Unknown Stash action".to_string()),
    }.map_err(|error| format!("Stash: {error}"))
}

#[cfg(not(target_os = "linux"))]
pub fn stash_action(_action: &str) -> Result<(), String> {
    Err("Stash controls require Linux.".to_string())
}

#[cfg(test)]
mod stash_tests {
    #[test]
    fn detects_spectrum_integration_without_a_launcher_flag() {
        assert!(super::stash_mode(None, true));
        assert!(!super::stash_mode(None, false));
        assert!(!super::stash_mode(Some("0"), true));
        assert!(super::stash_mode(Some("1"), false));
    }

    #[test]
    fn matches_only_stash_bus_names() {
        for name in ["stash", "stash.instance123"] { assert!(super::is_stash_bus(name)); }
        for name in ["spotify", "firefox", "questline", "stash-other", "notstash"] {
            assert!(!super::is_stash_bus(name));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// audio/mpris_player.rs — controla reproductores de música vía MPRIS (D-Bus)
// funciona con Spotify, VLC, Rhythmbox, y cualquier player compatible con MPRIS
// sin OAuth, sin cuenta, sin configuración — solo abre tu reproductor favorito
// solo disponible en Linux; macOS y Windows no tienen D-Bus
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub player: String,
    pub is_playing: bool,
}

#[cfg(target_os = "linux")]
pub fn get_now_playing() -> Option<NowPlaying> {
    use mpris::PlayerFinder;
    let finder = PlayerFinder::new().ok()?;
    let player = selected_player(&finder)?;
    let player_name = player.bus_name_trimmed().to_string();
    let metadata = player.get_metadata().ok()?;
    let status = player.get_playback_status().ok()?;

    let title = metadata.title().unwrap_or("Unknown Track").to_string();
    let artist = metadata
        .artists()
        .and_then(|a| a.first().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let is_playing = status == mpris::PlaybackStatus::Playing;

    Some(NowPlaying {
        title,
        artist,
        player: player_name,
        is_playing,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn get_now_playing() -> Option<NowPlaying> {
    None
}

#[cfg(target_os = "linux")]
pub fn play_pause() {
    use mpris::PlayerFinder;
    if let Ok(finder) = PlayerFinder::new() {
        if let Some(player) = selected_player(&finder) {
            let _ = player.play_pause();
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn play_pause() {}

#[cfg(target_os = "linux")]
pub fn pause() {
    use mpris::PlayerFinder;
    if let Ok(finder) = PlayerFinder::new() {
        if let Some(player) = selected_player(&finder) {
            let _ = player.pause();
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pause() {}

#[cfg(target_os = "linux")]
pub fn next_track() {
    use mpris::PlayerFinder;
    if let Ok(finder) = PlayerFinder::new() {
        if let Some(player) = selected_player(&finder) {
            let _ = player.next();
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn next_track() {}

#[cfg(target_os = "linux")]
pub fn prev_track() {
    use mpris::PlayerFinder;
    if let Ok(finder) = PlayerFinder::new() {
        if let Some(player) = selected_player(&finder) {
            let _ = player.previous();
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn prev_track() {}
