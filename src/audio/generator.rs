// ─────────────────────────────────────────────────────────────────────────────
// audio/generator.rs — genera soundscapes procedurales: lluvia, bosque, ruido blanco y más
// ─────────────────────────────────────────────────────────────────────────────

use rodio::Source;
use std::time::Duration;

// Generador de números pseudo-aleatorios para los soundscapes — sin rand crate, puro bit twiddling
#[derive(Clone)]
struct Xorshift32 {
    state: u32,
}

impl Xorshift32 {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 2463534242 } else { seed },
        }
    }

    fn next_f32(&mut self) -> f32 {
        let mut x = self.state;
        // tres XOR shifts y ya — es todo el "algoritmo", pero funciona chido
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        // Map to [-1.0, 1.0]
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

// Ruido blanco puro — valores aleatorios directos, suena como estática de TV pero chill
#[derive(Clone)]
pub struct WhiteNoise {
    rng: Xorshift32,
}

impl Default for WhiteNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl WhiteNoise {
    pub fn new() -> Self {
        Self {
            rng: Xorshift32::new(12345),
        }
    }
}

impl Iterator for WhiteNoise {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        Some(self.rng.next_f32() * 0.15) // 0.15 para que no reviente los oídos
    }
}

impl Source for WhiteNoise {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// Ruido café — el más relajante de los tres, grave y suave como el mar a lo lejos
#[derive(Clone)]
pub struct BrownNoise {
    rng: Xorshift32,
    current: f32,
}

impl Default for BrownNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl BrownNoise {
    pub fn new() -> Self {
        Self {
            rng: Xorshift32::new(54321),
            current: 0.0,
        }
    }
}

impl Iterator for BrownNoise {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let white = self.rng.next_f32();
        // integrador con fuga — acumula el ruido blanco pero lo va dejando escapar poco a poco
        self.current = (self.current + white * 0.08) * 0.985;
        Some(self.current * 0.3)
    }
}

impl Source for BrownNoise {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// Pink noise — entre el blanco y el café, filtra las frecuencias altas para sonar más natural
#[derive(Clone)]
pub struct PinkNoise {
    rng: Xorshift32,
    current: f32,
}

impl Default for PinkNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl PinkNoise {
    pub fn new() -> Self {
        Self {
            rng: Xorshift32::new(98765),
            current: 0.0,
        }
    }
}

impl Iterator for PinkNoise {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let white = self.rng.next_f32();
        // low-pass de primer orden: 88% valor anterior + 12% nuevo — sencillo pero efectivo
        self.current = 0.88 * self.current + 0.12 * white;
        Some(self.current * 0.22)
    }
}

impl Source for PinkNoise {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// OceanWaves — tres olas coseno desfasadas para que no suenen mecánicas, más espuma arriba del 85%
// Each swell runs at a different period so they rarely all peak at once — keeps it feeling real
#[derive(Clone)]
pub struct OceanWaves {
    rng: Xorshift32,
    brown: BrownNoise,
    phase1: f32, // primary swell  ~8 s
    phase2: f32, // secondary swell ~5.5 s
    phase3: f32, // tertiary swell  ~12 s
    foam: f32,   // decaying foam amplitude at crest
}

impl Default for OceanWaves {
    fn default() -> Self {
        Self::new()
    }
}

impl OceanWaves {
    pub fn new() -> Self {
        use std::f32::consts::PI;
        Self {
            rng: Xorshift32::new(34567),
            brown: BrownNoise::new(),
            phase1: 0.0,
            phase2: PI * 0.7,  // staggered so crests don't always coincide
            phase3: PI * 1.4,
            foam: 0.0,
        }
    }
}

impl Iterator for OceanWaves {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let pi2 = 2.0 * std::f32::consts::PI;

        // coseno a media ola: va de 0 en el valle a 1 en la cresta — envolvente perfecta
        let s1 = 0.5 - 0.5 * self.phase1.cos();
        let s2 = 0.5 - 0.5 * self.phase2.cos();
        let s3 = 0.5 - 0.5 * self.phase3.cos();
        // ola primaria domina (55%), las otras dan variación natural
        let combined = (s1 * 0.55 + s2 * 0.30 + s3 * 0.15).min(1.0);

        self.phase1 = (self.phase1 + pi2 / (44100.0 * 8.0)) % pi2;
        self.phase2 = (self.phase2 + pi2 / (44100.0 * 5.5)) % pi2;
        self.phase3 = (self.phase3 + pi2 / (44100.0 * 12.0)) % pi2;

        // espuma solo cuando la ola está casi en la cresta — se disipa lentamente
        if combined > 0.85 {
            let peak = (combined - 0.85) / 0.15;
            if peak > self.foam {
                self.foam = peak;
            }
        }
        let foam_noise = self.rng.next_f32() * self.foam * 0.12;
        self.foam *= 0.9996; // 0.9996 para que tarde como 2.5s en desaparecer

        let rumble = self.brown.next().unwrap();
        Some((rumble * combined + foam_noise) * 0.85)
    }
}

impl Source for OceanWaves {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// Lluvia — ruido rosa de fondo ("shhhh") + gotas individuales con decay exponencial
// Each drop fires at a random interval, decays fast, and varies in amplitude — no manches, quedó bien
#[derive(Clone)]
pub struct RainSounds {
    rng: Xorshift32,
    pink: PinkNoise,
    sample_counter: u32,
    next_drop_at: u32, // samples until next drop impact
    drop_env: f32,     // decaying amplitude of current drop
}

impl Default for RainSounds {
    fn default() -> Self {
        Self::new()
    }
}

impl RainSounds {
    pub fn new() -> Self {
        Self {
            rng: Xorshift32::new(88888),
            pink: PinkNoise::new(),
            sample_counter: 0,
            next_drop_at: 1323, // ~30 ms before first drop
            drop_env: 0.0,
        }
    }
}

impl Iterator for RainSounds {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        // ruido rosa es más brillante que el café — suena más al "shhhh" de la lluvia real
        let hiss = self.pink.next().unwrap();

        self.sample_counter = self.sample_counter.wrapping_add(1);
        if self.sample_counter >= self.next_drop_at {
            self.sample_counter = 0;
            // tamaño de gota aleatorio entre 0.4 y 1.0
            self.drop_env = 0.4 + self.rng.next_f32().abs() * 0.6;
            // Next drop in 15 ms–100 ms (lluvia moderada-fuerte)
            let r = self.rng.next_f32().abs();
            self.next_drop_at = 660 + (r * 3750.0) as u32;
        }

        let drop = if self.drop_env > 0.005 {
            let s = self.rng.next_f32() * self.drop_env;
            self.drop_env *= 0.994; // decae rápido — vida media ~115 muestras ≈ 2.6 ms
            s
        } else {
            self.drop_env = 0.0;
            0.0
        };

        // hiss domina (80%) y las gotas agregan textura (18%)
        Some((hiss * 0.80 + drop * 0.18) * 0.50)
    }
}

impl Source for RainSounds {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// ForestSounds — pues el más complejo del archivo: ruido de hojas + pájaros con frecuencia que sube/baja
// Chirp sequences are 2–4 swept-sine notes; timing driven entirely by sample counter, no rng modulo tricks
#[derive(Clone)]
pub struct ForestSounds {
    rng: Xorshift32,
    pink: PinkNoise,
    // Waiting state
    sample_counter: u32,
    next_chirp_at: u32,
    // Sequence state
    notes_left: u32,
    // Note playback state
    note_ticks_left: u32,
    note_ticks_total: u32,
    note_phase: f32,
    note_freq: f32,
    note_target_freq: f32,
    note_amp: f32,
    // Inter-note gap
    gap_ticks_left: u32,
}

impl Default for ForestSounds {
    fn default() -> Self {
        Self::new()
    }
}

impl ForestSounds {
    pub fn new() -> Self {
        Self {
            rng: Xorshift32::new(77777),
            pink: PinkNoise::new(),
            sample_counter: 0,
            next_chirp_at: 88200, // ~2 s initial delay before first chirp
            notes_left: 0,
            note_ticks_left: 0,
            note_ticks_total: 1,
            note_phase: 0.0,
            note_freq: 3000.0,
            note_target_freq: 3500.0,
            note_amp: 1.0,
            gap_ticks_left: 0,
        }
    }
}

impl Iterator for ForestSounds {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        // ruido rosa atenuado para simular el "shshshsh" de las hojas
        let rustle = self.pink.next().unwrap() * 0.30;

        let chirp = if self.note_ticks_left > 0 {
            // reproduciendo nota actual — frecuencia va barriendo de note_freq a note_target_freq
            self.note_ticks_left -= 1;
            let progress =
                1.0 - (self.note_ticks_left as f32 / self.note_ticks_total as f32);
            let freq =
                self.note_freq + (self.note_target_freq - self.note_freq) * progress;
            self.note_phase += 2.0 * std::f32::consts::PI * freq / 44100.0;
            if self.note_phase > 2.0 * std::f32::consts::PI {
                self.note_phase -= 2.0 * std::f32::consts::PI;
            }
            // envolvente seno: sube y baja suave, sin clicks al inicio o final
            let env = (progress * std::f32::consts::PI).sin();
            self.note_phase.sin() * self.note_amp * 0.10 * env
        } else if self.gap_ticks_left > 0 {
            // pausa entre notas del mismo pájaro
            self.gap_ticks_left -= 1;
            0.0
        } else if self.notes_left > 0 {
            // órale, siguiente nota de la secuencia — frecuencia y duración aleatorias
            self.notes_left -= 1;
            let dur_r = self.rng.next_f32().abs();
            self.note_ticks_total = 1760 + (dur_r * 2646.0) as u32; // 40–100 ms
            self.note_ticks_left = self.note_ticks_total;
            let freq_r = self.rng.next_f32().abs();
            self.note_freq = 2500.0 + freq_r * 4000.0; // rango 2.5–6.5 kHz, como pájaro chido
            // sweep random: puede subir o bajar
            let sweep_r = self.rng.next_f32();
            self.note_target_freq = self.note_freq + sweep_r * 1800.0;
            let amp_r = self.rng.next_f32().abs();
            self.note_amp = 0.5 + amp_r * 0.5;
            self.note_phase = 0.0;
            // pausa post-nota entre 10–40 ms para naturalidad
            let gap_r = self.rng.next_f32().abs();
            self.gap_ticks_left = 441 + (gap_r * 1323.0) as u32;
            0.0
        } else {
            // esperando la siguiente secuencia de trinos — entre 3 y 9 segundos
            self.sample_counter = self.sample_counter.wrapping_add(1);
            if self.sample_counter >= self.next_chirp_at {
                self.sample_counter = 0;
                let n_r = self.rng.next_f32().abs();
                self.notes_left = 2 + (n_r * 2.5) as u32; // 2–4 notas por secuencia
                let t_r = self.rng.next_f32().abs();
                self.next_chirp_at = 132300 + (t_r * 264600.0) as u32;
            }
            let _ = self.rng.next_f32(); // avanzar el rng aunque no se use el valor
            0.0
        };

        Some((rustle + chirp) * 0.50)
    }
}

impl Source for ForestSounds {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// AmbientRadio — acordes drone que se van crossfadeando cada 12 segundos, escala Am/Cmaj pentatónica
#[derive(Clone)]
pub struct AmbientRadio {
    rng: Xorshift32,
    frequencies: [f32; 5],
    amplitudes: [f32; 5],
    target_amplitudes: [f32; 5],
    phases: [f32; 5],
    ticks_to_chord_change: usize,
}

impl Default for AmbientRadio {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientRadio {
    pub fn new() -> Self {
        let rng = Xorshift32::new(99999);
        let mut s = Self {
            rng,
            frequencies: [110.0, 164.8, 220.0, 261.6, 329.6], // Am7 chord base
            amplitudes: [0.0; 5],
            target_amplitudes: [0.05, 0.05, 0.05, 0.05, 0.05],
            phases: [0.0; 5],
            ticks_to_chord_change: 0,
        };
        s.trigger_new_chords();
        s
    }

    // selecciona un acorde nuevo y pone las amplitudes target — el smoothing lo fade-in gradual
    fn trigger_new_chords(&mut self) {
        // 4 acordes en rotación: Am7, Cmaj7, Fmaj7, Em7 — todos en familia, nunca suenan chistoso
        let chord_idx = self.rng.state % 4;
        let scale_notes = match chord_idx {
            0 => [110.0, 164.8, 220.0, 261.6, 329.6], // Am7: A2, E3, A3, C4, E4
            1 => [130.8, 196.0, 261.6, 329.6, 392.0], // Cmaj7: C3, G3, C4, E4, G4
            2 => [174.6, 220.0, 349.2, 392.0, 440.0], // Fmaj7: F3, A3, F4, G4, A4
            _ => [164.8, 246.9, 329.6, 392.0, 493.9], // Em7: E3, B3, E4, G4, B4
        };

        for i in 0..5 {
            self.frequencies[i] = scale_notes[i];
            // Slow fade target
            self.target_amplitudes[i] = 0.015 + (self.rng.state % 25) as f32 * 0.001;
            let _ = self.rng.next_f32(); // advance
        }
        self.ticks_to_chord_change = 44100 * 12; // Change chord every 12 seconds
    }
}

impl Iterator for AmbientRadio {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.ticks_to_chord_change == 0 {
            self.trigger_new_chords();
        } else {
            self.ticks_to_chord_change -= 1;
        }

        let mut sample = 0.0;
        for i in 0..5 {
            // interpolación suave con factor 0.0001 — el crossfade tarda unos segundos, muy ambient
            self.amplitudes[i] += (self.target_amplitudes[i] - self.amplitudes[i]) * 0.0001;

            self.phases[i] += 2.0 * std::f32::consts::PI * self.frequencies[i] / 44100.0;
            if self.phases[i] > 2.0 * std::f32::consts::PI {
                self.phases[i] -= 2.0 * std::f32::consts::PI;
            }
            sample += self.phases[i].sin() * self.amplitudes[i];
        }

        Some(sample * 0.4)
    }
}

impl Source for AmbientRadio {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// LoFi Radio — qué rollo más chido: acordes de onda triangular + crackle de vinil + kick lento de 80 BPM
#[derive(Clone)]
pub struct LoFiRadio {
    rng: Xorshift32,
    frequencies: [f32; 4],
    amplitudes: [f32; 4],
    target_amplitudes: [f32; 4],
    phases: [f32; 4],
    ticks_to_chord_change: usize,

    // Drum components
    kick_phase: f32,
    ticks_to_next_beat: usize,

    // Vinyl crackle components
    crackle_decay: f32,
}

impl Default for LoFiRadio {
    fn default() -> Self {
        Self::new()
    }
}

impl LoFiRadio {
    pub fn new() -> Self {
        let rng = Xorshift32::new(11111);
        let mut s = Self {
            rng,
            frequencies: [110.0, 164.8, 220.0, 261.6],
            amplitudes: [0.0; 4],
            target_amplitudes: [0.03; 4],
            phases: [0.0; 4],
            ticks_to_chord_change: 0,
            kick_phase: 0.0,
            ticks_to_next_beat: 0,
            crackle_decay: 0.0,
        };
        s.trigger_new_chords();
        s
    }

    // misma idea que AmbientRadio pero acordes más íntimos y cambio cada 8s para el rollo lo-fi
    fn trigger_new_chords(&mut self) {
        let chord_idx = self.rng.state % 4;
        let scale_notes = match chord_idx {
            0 => [220.0, 261.6, 329.6, 392.0], // Am7: A3, C4, E4, G4
            1 => [146.8, 174.6, 220.0, 261.6], // Dm7: D3, F3, A3, C4
            2 => [164.8, 196.0, 246.9, 293.7], // Em7: E3, G3, B3, D4
            _ => [261.6, 329.6, 392.0, 440.0], // Cmaj7/6: C4, E4, G4, A4
        };

        for i in 0..4 {
            self.frequencies[i] = scale_notes[i];
            self.target_amplitudes[i] = 0.015 + (self.rng.state % 15) as f32 * 0.001;
            let _ = self.rng.next_f32();
        }
        self.ticks_to_chord_change = 44100 * 8;
    }
}

impl Iterator for LoFiRadio {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.ticks_to_chord_change == 0 {
            self.trigger_new_chords();
        } else {
            self.ticks_to_chord_change -= 1;
        }

        // onda triangular suena más redonda que la sinusoide pura, más lo-fi
        let mut chord_sample = 0.0;
        for i in 0..4 {
            self.amplitudes[i] += (self.target_amplitudes[i] - self.amplitudes[i]) * 0.0002;
            self.phases[i] += 2.0 * std::f32::consts::PI * self.frequencies[i] / 44100.0;
            if self.phases[i] > 2.0 * std::f32::consts::PI {
                self.phases[i] -= 2.0 * std::f32::consts::PI;
            }

            // fórmula de onda triangular: sube lineal, baja lineal, entre -1 y 1
            let phase_normalized = self.phases[i] / (2.0 * std::f32::consts::PI);
            let tri = if phase_normalized < 0.5 {
                4.0 * phase_normalized - 1.0
            } else {
                3.0 - 4.0 * phase_normalized
            };

            chord_sample += tri * self.amplitudes[i];
        }

        // kick grave tipo heartbeat — sweep de 45 Hz a 20 Hz mientras decae
        if self.ticks_to_next_beat == 0 {
            self.kick_phase = 1.0; // dispara el envelope del kick
            self.ticks_to_next_beat = 44100 * 3 / 2; // ~1.5s entre beats ≈ 80 BPM
        } else {
            self.ticks_to_next_beat -= 1;
        }

        let kick_sample = if self.kick_phase > 0.0 {
            // frecuencia baja conforme decae el kick — da ese "boom" característico
            let freq = 20.0 + 35.0 * self.kick_phase;
            let kick_osc_val = (self.ticks_to_next_beat as f32 * 2.0 * std::f32::consts::PI * freq
                / 44100.0)
                .sin();
            let sample = kick_osc_val * 0.16 * self.kick_phase;
            self.kick_phase *= 0.9997; // decay lento para que retumbe chido
            if self.kick_phase < 0.001 {
                self.kick_phase = 0.0;
            }
            sample
        } else {
            0.0
        };

        // crackle de vinil — dispara cuando el rng.state es múltiplo de 6000, básicamente al azar
        if self.rng.state.is_multiple_of(6000) {
            self.crackle_decay = 1.0;
        }
        let crackle = if self.crackle_decay > 0.0 {
            let val = self.rng.next_f32() * self.crackle_decay;
            self.crackle_decay *= 0.93; // decae muy rápido, solo es un "clic" pequeño
            val * 0.015
        } else {
            0.0
        };

        let _ = self.rng.next_f32(); // avanzar el rng para que el crackle no sea predecible

        Some((chord_sample + kick_sample + crackle) * 0.5)
    }
}

impl Source for LoFiRadio {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
