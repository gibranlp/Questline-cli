// ─────────────────────────────────────────────────────────────────────────────
// audio/streams.rs — manejo de streams de audio
// ─────────────────────────────────────────────────────────────────────────────
use crate::audio::generator::{
    AmbientRadio, BrownNoise, ForestSounds, LoFiRadio, OceanWaves, PinkNoise, RainSounds,
    WhiteNoise,
};
use rodio::Source;

pub fn build_source(name: &str) -> Option<Box<dyn Source<Item = f32> + Send + 'static>> {
    match name {
        "White Noise" => Some(Box::new(WhiteNoise::new())),
        "Brown Noise" => Some(Box::new(BrownNoise::new())),
        "Pink Noise" => Some(Box::new(PinkNoise::new())),
        "Ocean Waves" => Some(Box::new(OceanWaves::new())),
        "Rain Sounds" => Some(Box::new(RainSounds::new())),
        "Forest Sounds" => Some(Box::new(ForestSounds::new())),
        "Ambient Radio" => Some(Box::new(AmbientRadio::new())),
        "LoFi Radio" => Some(Box::new(LoFiRadio::new())),
        _ => None,
    }
}
