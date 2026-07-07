// ─────────────────────────────────────────────────────────────────────────────
// audio/capture.rs — captura el audio del sistema vía monitor de PulseAudio/PipeWire
// permite mostrar el espectro de reproductores externos (MPRIS, Spotify, VLC, etc.)
// usa pacat que viene incluido con pipewire-pulse o pulseaudio en Linux
// ─────────────────────────────────────────────────────────────────────────────

use crate::audio::spectrum::{update_from_mono, SpectrumData, FFT_SIZE, HOP_SIZE};
use rustfft::{num_complex::Complex, FftPlanner};
use std::io::Read;

const CAPTURE_RATE: u32 = 44100;
// chunk pequeño para latencia baja — cada lectura de pacat tarda ~12ms
const CAPTURE_CHUNK: usize = 512;

// inicia la captura del monitor de audio del sistema en un hilo de fondo
// escribe al mismo SpectrumData que usa SpectrumSource — se complementan sin conflictos
pub fn start_system_capture(spectrum: SpectrumData) {
    std::thread::spawn(move || {
        // pacat es parte de pipewire-pulse o pulseaudio — en Arch Linux siempre presente
        let mut child = match std::process::Command::new("pacat")
            .args([
                "--record",
                "--device=@DEFAULT_MONITOR@",
                "--format=float32le",
                "--rate=44100",
                "--channels=1",
                "--latency-msec=30",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                crate::services::log_structured(
                    "INFO",
                    "capture",
                    "pacat no disponible, captura del sistema desactivada",
                    None,
                );
                return;
            }
        };

        let mut stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();
        let mut fft_buf = vec![Complex { re: 0.0f32, im: 0.0f32 }; FFT_SIZE];
        let mut fft_scratch = vec![Complex { re: 0.0f32, im: 0.0f32 }; scratch_len];

        let mut acc_buf: Vec<f32> = Vec::with_capacity(FFT_SIZE + HOP_SIZE);
        let mut raw = vec![0u8; CAPTURE_CHUNK * 4]; // 4 bytes por muestra f32

        loop {
            if stdout.read_exact(&mut raw).is_err() {
                break; // pacat terminó o el sistema de audio se desconectó
            }

            // convierte bytes F32LE a f32 — pacat ya entrega el formato correcto
            for chunk in raw.chunks_exact(4) {
                let bytes: [u8; 4] = chunk.try_into().unwrap();
                acc_buf.push(f32::from_le_bytes(bytes));
            }

            // procesa en ventanas de FFT_SIZE con salto de HOP_SIZE (75% overlap)
            while acc_buf.len() >= FFT_SIZE {
                update_from_mono(
                    &acc_buf,
                    &fft,
                    &mut fft_buf,
                    &mut fft_scratch,
                    &spectrum,
                    CAPTURE_RATE,
                );
                acc_buf.drain(..HOP_SIZE);
            }
        }

        // cuando pacat muere, desvanece el espectro suavemente en vez de congelarlo
        if let Ok(mut spec) = spectrum.try_lock() {
            for v in spec.iter_mut() {
                *v *= 0.0;
            }
        }

        let _ = child.kill();
    });
}
