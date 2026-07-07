// ─────────────────────────────────────────────────────────────────────────────
// audio/soundscapes.rs — definiciones de los soundscapes disponibles
// ─────────────────────────────────────────────────────────────────────────────
pub struct SoundscapeInfo {
    pub name: &'static str,
    pub symbol: &'static str,
    pub description: &'static str,
    pub bonus: &'static str,
}

pub const SOUNDSCAPES: [SoundscapeInfo; 9] = [
    SoundscapeInfo {
        name: "Media Player",
        symbol: "",
        description: "Control any MPRIS-compatible player: Spotify, VLC, Rhythmbox, and more.",
        bonus: "+5% Focus XP — syncs with your listening mood",
    },
    SoundscapeInfo {
        name: "Music For Programming",
        symbol: "",
        description: "Focus mixes dynamically streamed from musicforprogramming.net.",
        bonus: "+15% Focus XP bonus",
    },
    SoundscapeInfo {
        name: "LoFi Radio",
        symbol: "",
        description: "Soft generative chords with a low-passed beat.",
        bonus: "None",
    },
    SoundscapeInfo {
        name: "Local Folder",
        symbol: "",
        description: "Plays audio tracks sequentially from a local directory (press 'f' to set path).",
        bonus: "None",
    },
    SoundscapeInfo {
        name: "Forest Sounds",
        symbol: "",
        description: "Wind rustles with procedurally generated bird calls.",
        bonus: "+1 Tree Growth on completed Focus sessions",
    },
    SoundscapeInfo {
        name: "Rain Sounds",
        symbol: "",
        description: "Deep low rumbling combined with raindrop crackles.",
        bonus: "Auto-waters Tree after completed Focus sessions",
    },
    SoundscapeInfo {
        name: "Ocean Waves",
        symbol: "",
        description: "Waves swelling and breaking in a slow, natural envelope.",
        bonus: "+5 Focus XP bonus",
    },
    SoundscapeInfo {
        name: "White Noise",
        symbol: "",
        description: "Comfortable, uniform frequency spectrum static wash.",
        bonus: "+10% Focus Duration for stats",
    },
    SoundscapeInfo {
        name: "Silent",
        symbol: "",
        description: "Silence for deep focus without atmospheric audio.",
        bonus: "Silent Monk Achievement progression",
    },
];
