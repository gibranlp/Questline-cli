// ─────────────────────────────────────────────────────────────────────────────
// audio/mod.rs — re-exports del módulo de audio
// ─────────────────────────────────────────────────────────────────────────────
pub mod capture;
pub mod generator;
pub mod player;
pub mod soundscapes;
pub mod mpris_player;
pub mod spectrum;
pub mod state;
pub mod streams;

pub use player::AudioPlayer;
pub use soundscapes::SOUNDSCAPES;
pub use state::PlaybackStatus;
