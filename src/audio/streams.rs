// ─────────────────────────────────────────────────────────────────────────────
// audio/streams.rs — manejo de streams de audio
// ─────────────────────────────────────────────────────────────────────────────
use crate::audio::generator::{AmbientRadio, BrownNoise, LoFiRadio, PinkNoise, WhiteNoise};
use rodio::Source;
use std::io::Cursor;

fn looping_asset_source(
    bytes: &'static [u8],
) -> Option<Box<dyn Source<Item = f32> + Send + 'static>> {
    let cursor = Cursor::new(bytes);
    let decoder = rodio::Decoder::new(cursor).ok()?;
    Some(Box::new(decoder.repeat_infinite().convert_samples()))
}

pub fn build_source(name: &str) -> Option<Box<dyn Source<Item = f32> + Send + 'static>> {
    match name {
        "White Noise" => Some(Box::new(WhiteNoise::new())),
        "Brown Noise" => Some(Box::new(BrownNoise::new())),
        "Pink Noise" => Some(Box::new(PinkNoise::new())),
        "Ocean Waves" => looping_asset_source(include_bytes!("../../assets/sounds/seawash.mp3")),
        "Rain Sounds" => looping_asset_source(include_bytes!("../../assets/sounds/rain.mp3")),
        "Forest Sounds" => looping_asset_source(include_bytes!("../../assets/sounds/forest.mp3")),
        "Ambient Radio" => Some(Box::new(AmbientRadio::new())),
        "LoFi Radio" => Some(Box::new(LoFiRadio::new())),
        _ => None,
    }
}
