// ─────────────────────────────────────────────────────────────────────────────
// audio/state.rs — el estado del reproductor de audio (playing, paused, etc.)
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct AudioState {
    pub current_soundscape: String,
    pub status: PlaybackStatus,
    pub volume: f32, // 0.0 to 1.0
    pub local_music_folder: String,
}

impl Default for AudioState {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            current_soundscape: "Silent".to_string(),
            status: PlaybackStatus::Stopped,
            volume: 0.5,
            local_music_folder: String::new(),
        }
    }
}
