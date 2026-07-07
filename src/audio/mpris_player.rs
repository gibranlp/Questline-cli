// ─────────────────────────────────────────────────────────────────────────────
// audio/mpris_player.rs — controla reproductores de música vía MPRIS (D-Bus)
// funciona con Spotify, VLC, Rhythmbox, y cualquier player compatible con MPRIS
// sin OAuth, sin cuenta, sin configuración — solo abre tu reproductor favorito
// ─────────────────────────────────────────────────────────────────────────────

use mpris::PlayerFinder;

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub player: String,
    pub is_playing: bool,
}

pub fn get_now_playing() -> Option<NowPlaying> {
    let finder = PlayerFinder::new().ok()?;
    let player = finder.find_active().ok()?;
    let player_name = player.bus_name_trimmed().to_string();
    let metadata = player.get_metadata().ok()?;
    let status = player.get_playback_status().ok()?;

    let title = metadata.title().unwrap_or("Unknown Track").to_string();
    let artist = metadata
        .artists()
        .and_then(|a| a.first().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let is_playing = status == mpris::PlaybackStatus::Playing;

    Some(NowPlaying { title, artist, player: player_name, is_playing })
}

pub fn play_pause() {
    if let Ok(finder) = PlayerFinder::new() {
        if let Ok(player) = finder.find_active() {
            let _ = player.play_pause();
        }
    }
}

pub fn next_track() {
    if let Ok(finder) = PlayerFinder::new() {
        if let Ok(player) = finder.find_active() {
            let _ = player.next();
        }
    }
}

pub fn prev_track() {
    if let Ok(finder) = PlayerFinder::new() {
        if let Ok(player) = finder.find_active() {
            let _ = player.previous();
        }
    }
}
