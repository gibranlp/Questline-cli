// ─────────────────────────────────────────────────────────────────────────────
// audio/mod.rs — re-exports del módulo de audio
// ─────────────────────────────────────────────────────────────────────────────
pub mod generator;
pub mod player;
pub mod soundscapes;
pub mod state;
pub mod streams;

pub use player::AudioPlayer;
pub use soundscapes::SOUNDSCAPES;
pub use state::PlaybackStatus;
