// ─────────────────────────────────────────────────────────────────────────────
// audio/spectrum.rs — wrapper de fuente de audio que intercepta las muestras
// y calcula un espectro de frecuencias en tiempo real usando FFT
// ─────────────────────────────────────────────────────────────────────────────

use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};

pub const NUM_BARS: usize = 48;
pub const FFT_SIZE: usize = 1024;
pub const HOP_SIZE: usize = 256;
// decay rápido al subir (ataque instantáneo), lento al bajar (caída suave como VU meter)
pub const DECAY: f32 = 0.92;

pub type SpectrumData = Arc<Mutex<[f32; NUM_BARS]>>;

pub fn new_spectrum_data() -> SpectrumData {
    Arc::new(Mutex::new([0.0f32; NUM_BARS]))
}

// calcula el FFT sobre las primeras FFT_SIZE muestras del buffer mono y escribe el espectro
// — reutilizado tanto por SpectrumSource (rodio) como por el hilo de captura del sistema
pub fn update_from_mono(
    buffer: &[f32],
    fft: &Arc<dyn rustfft::Fft<f32>>,
    fft_buf: &mut Vec<Complex<f32>>,
    fft_scratch: &mut Vec<Complex<f32>>,
    spectrum: &SpectrumData,
    sample_rate: u32,
) {
    let sr = sample_rate as f32;

    // ventana Hanning para reducir spectral leakage en los bordes del buffer
    for (i, (&s, c)) in buffer[..FFT_SIZE].iter().zip(fft_buf.iter_mut()).enumerate() {
        let w = 0.5
            * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
        c.re = s * w;
        c.im = 0.0;
    }

    fft.process_with_scratch(fft_buf, fft_scratch);

    let bins = FFT_SIZE / 2;
    // rango audible de 30 Hz a 16 kHz en escala logarítmica — igual que en un analizador real
    let f_lo: f32 = 30.0;
    let f_hi: f32 = 16_000.0;
    let log_ratio = (f_hi / f_lo).ln();

    let mut new_bands = [0.0f32; NUM_BARS];
    for bar in 0..NUM_BARS {
        let t_lo = bar as f32 / NUM_BARS as f32;
        let t_hi = (bar + 1) as f32 / NUM_BARS as f32;
        let freq_lo = f_lo * (log_ratio * t_lo).exp();
        let freq_hi = f_lo * (log_ratio * t_hi).exp();

        let bin_lo = ((freq_lo / sr * FFT_SIZE as f32) as usize).max(1).min(bins - 1);
        let bin_hi = ((freq_hi / sr * FFT_SIZE as f32) as usize + 1)
            .max(bin_lo + 1)
            .min(bins);

        let count = (bin_hi - bin_lo) as f32;
        let sum_sq: f32 =
            fft_buf[bin_lo..bin_hi].iter().map(|c| c.norm_sqr()).sum::<f32>() / count;

        // escala dB perceptual: mapea -60 dB→0.0, 0 dB→1.0 para buena dinámica visual
        let amplitude = 2.0 * sum_sq.sqrt() / FFT_SIZE as f32;
        let db = 20.0 * amplitude.max(1e-10_f32).log10();
        new_bands[bar] = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    }

    // try_lock para no bloquear el hilo que llama si el UI está leyendo al mismo tiempo
    if let Ok(mut spec) = spectrum.try_lock() {
        for i in 0..NUM_BARS {
            if new_bands[i] > spec[i] {
                spec[i] = new_bands[i];
            } else {
                spec[i] = (spec[i] * DECAY).max(new_bands[i]);
            }
        }
    }
}

pub struct SpectrumSource {
    inner: Box<dyn rodio::Source<Item = f32> + Send + 'static>,
    buffer: Vec<f32>,
    spectrum: SpectrumData,
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_buf: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    sample_rate: u32,
    channels: u16,
    ch_acc: f32,
    ch_idx: u16,
}

impl SpectrumSource {
    pub fn new(
        inner: Box<dyn rodio::Source<Item = f32> + Send + 'static>,
        spectrum: SpectrumData,
    ) -> Self {
        let sample_rate = inner.sample_rate();
        let channels = inner.channels().max(1);
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();
        Self {
            inner,
            buffer: Vec::with_capacity(FFT_SIZE + HOP_SIZE),
            spectrum,
            fft,
            fft_buf: vec![Complex { re: 0.0, im: 0.0 }; FFT_SIZE],
            fft_scratch: vec![Complex { re: 0.0, im: 0.0 }; scratch_len],
            sample_rate,
            channels,
            ch_acc: 0.0,
            ch_idx: 0,
        }
    }

    fn compute_fft(&mut self) {
        let buffer = &self.buffer;
        let fft = &self.fft;
        let fft_buf = &mut self.fft_buf;
        let fft_scratch = &mut self.fft_scratch;
        let spectrum = &self.spectrum;
        let sample_rate = self.sample_rate;
        update_from_mono(buffer, fft, fft_buf, fft_scratch, spectrum, sample_rate);
    }
}

impl Iterator for SpectrumSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;

        // acumula samples del frame actual para promediar a mono antes del FFT
        self.ch_acc += sample;
        self.ch_idx += 1;
        if self.ch_idx >= self.channels {
            self.buffer.push(self.ch_acc / self.channels as f32);
            self.ch_acc = 0.0;
            self.ch_idx = 0;

            if self.buffer.len() >= FFT_SIZE {
                self.compute_fft();
                // ventana deslizante con 75% de overlap — más resolución temporal
                self.buffer.drain(..HOP_SIZE);
            }
        }

        Some(sample)
    }
}

impl rodio::Source for SpectrumSource {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}
