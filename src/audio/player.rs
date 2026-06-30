// ─────────────────────────────────────────────────────────────────────────────
// audio/player.rs — controla la reproducción de música y soundscapes con MPRIS
// ─────────────────────────────────────────────────────────────────────────────

use crate::audio::state::{AudioState, PlaybackStatus};
use crate::audio::streams::build_source;
use rodio::{OutputStream, Sink};
use std::sync::{Arc, Mutex};
#[cfg(not(target_os = "windows"))]
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    handle: Option<rodio::OutputStreamHandle>,
    sink: Arc<Mutex<Option<Sink>>>,
    state: Arc<Mutex<AudioState>>,
    cinematic_sink: Arc<Mutex<Option<Sink>>>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    // inicializa el sistema de audio — si falla, el app sigue en modo silencioso, no truena
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(AudioState::new()));

        let mut _stream = None;
        let mut handle = None;
        let mut sink = None;

        match OutputStream::try_default() { Ok((s, h)) => {
            match Sink::try_new(&h) { Ok(sk) => {
                _stream = Some(s);
                handle = Some(h);
                // volumen default al 50%, ni muy alto ni muy bajo
                sk.set_volume(0.5);
                sink = Some(sk);
                crate::services::log_structured(
                    "INFO",
                    "audio_init",
                    "Audio output stream initialized successfully.",
                    None,
                );
            } _ => {
                crate::services::log_structured(
                    "ERROR",
                    "audio_init",
                    "Failed to create audio playback sink.",
                    None,
                );
            }}
        } _ => {
            crate::services::log_structured(
                "WARNING",
                "audio_init",
                "Failed to initialize default audio output stream. Running in silent mode.",
                None,
            );
        }}

        let player = Self {
            _stream,
            handle,
            sink: Arc::new(Mutex::new(sink)),
            state,
            cinematic_sink: Arc::new(Mutex::new(None)),
        };
        #[cfg(not(target_os = "windows"))]
        player.init_mpris();
        player
    }

    // reproduce efectos de sonido en su propio sink para no interrumpir el soundscape principal
    pub fn play_effect_bytes(&self, bytes: &'static [u8]) {
        if let Some(ref handle) = self.handle {
            let handle = handle.clone();
            // efectos al 20% del volumen maestro — pues no queremos que asusten al usuario
            let volume = self.get_state().volume * 0.20;
            std::thread::spawn(move || {
                let cursor = std::io::Cursor::new(bytes);
                let source = match rodio::Decoder::new(cursor) {
                    Ok(s) => s,
                    Err(e) => {
                        crate::services::log_structured(
                            "ERROR",
                            "audio_effect",
                            "Failed to decode embedded sound bytes",
                            Some(&e.to_string()),
                        );
                        return;
                    }
                };
                
                let sink = match Sink::try_new(&handle) {
                    Ok(sk) => sk,
                    Err(e) => {
                        crate::services::log_structured(
                            "ERROR",
                            "audio_effect",
                            "Failed to create sink for sound effect",
                            Some(&e.to_string()),
                        );
                        return;
                    }
                };
                
                sink.set_volume(volume);
                sink.append(source);
                sink.sleep_until_end();
            });
        }
    }

    pub fn play_task_creation(&self) {
        self.play_effect_bytes(include_bytes!("../../assets/sounds/taskcreation.ogg"));
    }

    pub fn play_task_complete(&self) {
        self.play_effect_bytes(include_bytes!("../../assets/sounds/focus-task-complete.ogg"));
    }

    // dispara el audio cinemático en su propio sink para que suene encima del soundscape
    pub fn play_cinematic(&self) {
        let Some(ref handle) = self.handle else { return };
        let sink_arc = self.cinematic_sink.clone();
        let volume = self.get_state().volume * 0.20;
        let handle = handle.clone();
        std::thread::spawn(move || {
            let cursor = std::io::Cursor::new(
                include_bytes!("../../assets/sounds/Cinematic Epic.mp3"),
            );
            let source = match rodio::Decoder::new(cursor) {
                Ok(s) => s,
                Err(_) => return,
            };
            let sk = match Sink::try_new(&handle) {
                Ok(s) => s,
                Err(_) => return,
            };
            sk.set_volume(volume);
            sk.append(source);
            sk.play();
            let mut guard = sink_arc.lock().unwrap();
            *guard = Some(sk);
        });
    }

    pub fn stop_cinematic(&self) {
        let mut guard = self.cinematic_sink.lock().unwrap();
        if let Some(ref sk) = *guard {
            sk.stop();
        }
        *guard = None;
    }

    pub fn play(&self, soundscape_name: &str) {
        trigger_playback(soundscape_name, &self.state, &self.sink, &self.handle);
    }

    pub fn init_soundscape(&self, soundscape_name: &str) {
        let mut state = self.state.lock().unwrap();
        state.current_soundscape = soundscape_name.to_string();
        state.status = PlaybackStatus::Stopped;
    }

    // toggle entre playing/paused — si está tocando lo pausa, si está pausado lo reanuda
    pub fn pause(&self) {
        let mut state = self.state.lock().unwrap();
        let sink_guard = self.sink.lock().unwrap();
        if state.status == PlaybackStatus::Playing {
            state.status = PlaybackStatus::Paused;
            if let Some(ref sk) = *sink_guard {
                sk.pause();
            }
        } else if state.status == PlaybackStatus::Paused {
            state.status = PlaybackStatus::Playing;
            if let Some(ref sk) = *sink_guard {
                sk.play();
            }
        }
    }

    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        state.status = PlaybackStatus::Stopped;
        let sink_guard = self.sink.lock().unwrap();
        if let Some(ref sk) = *sink_guard {
            sk.stop();
        }
    }

    // actualiza volumen en tiempo real sin reiniciar el stream — clampea entre 0 y 1
    pub fn set_volume(&self, vol: f32) {
        let mut state = self.state.lock().unwrap();
        let clamped = vol.clamp(0.0, 1.0);
        state.volume = clamped;
        let sink_guard = self.sink.lock().unwrap();
        if let Some(ref sk) = *sink_guard {
            sk.set_volume(clamped);
        }
    }

    pub fn set_local_music_folder(&self, folder: &str) {
        let mut state = self.state.lock().unwrap();
        state.local_music_folder = folder.to_string();
    }

    pub fn volume_up(&self) {
        let current = self.get_state().volume;
        self.set_volume(current + 0.05);
    }

    pub fn volume_down(&self) {
        let current = self.get_state().volume;
        self.set_volume(current - 0.05);
    }

    pub fn get_state(&self) -> AudioState {
        self.state.lock().unwrap().clone()
    }

    // integración con MPRIS para que los controles de media del sistema funcionen con Questline
    #[cfg(not(target_os = "windows"))]
    fn init_mpris(&self) {
        let state_clone = self.state.clone();
        let sink_clone = self.sink.clone();
        let handle_clone = self.handle.clone();

        // corre en su propio thread — el loop de MPRIS no debe bloquear el UI
        std::thread::spawn(move || {
            let mut controls = match MediaControls::new(PlatformConfig {
                dbus_name: "questline",
                display_name: "Questline",
                hwnd: None,
            }) {
                Ok(c) => c,
                Err(e) => {
                    crate::services::log_structured(
                        "ERROR",
                        "mpris",
                        "Failed to initialize MPRIS media controls",
                        Some(&format!("{:?}", e)),
                    );
                    return;
                }
            };

            let (tx, rx) = std::sync::mpsc::channel();

            if let Err(e) = controls.attach(move |event| {
                let _ = tx.send(event);
            }) {
                crate::services::log_structured(
                    "ERROR",
                    "mpris",
                    "Failed to attach MPRIS event listener",
                    Some(&format!("{:?}", e)),
                );
                return;
            }

            let mut last_status = PlaybackStatus::Stopped;
            let mut last_title = String::new();

            loop {
                // procesa eventos de teclas de media (play/pause/next del teclado o applet)
                while let Ok(event) = rx.try_recv() {
                    match event {
                        MediaControlEvent::Play => {
                            let mut state = state_clone.lock().unwrap();
                            if state.status == PlaybackStatus::Paused {
                                state.status = PlaybackStatus::Playing;
                                let sink_guard = sink_clone.lock().unwrap();
                                if let Some(ref sk) = *sink_guard {
                                    sk.play();
                                }
                            }
                        }
                        MediaControlEvent::Pause => {
                            let mut state = state_clone.lock().unwrap();
                            if state.status == PlaybackStatus::Playing {
                                state.status = PlaybackStatus::Paused;
                                let sink_guard = sink_clone.lock().unwrap();
                                if let Some(ref sk) = *sink_guard {
                                    sk.pause();
                                }
                            }
                        }
                        MediaControlEvent::Toggle => {
                            let mut state = state_clone.lock().unwrap();
                            let sink_guard = sink_clone.lock().unwrap();
                            if state.status == PlaybackStatus::Playing {
                                state.status = PlaybackStatus::Paused;
                                if let Some(ref sk) = *sink_guard {
                                    sk.pause();
                                }
                            } else if state.status == PlaybackStatus::Paused {
                                state.status = PlaybackStatus::Playing;
                                if let Some(ref sk) = *sink_guard {
                                    sk.play();
                                }
                            }
                        }
                        MediaControlEvent::Stop => {
                            let mut state = state_clone.lock().unwrap();
                            state.status = PlaybackStatus::Stopped;
                            let sink_guard = sink_clone.lock().unwrap();
                            if let Some(ref sk) = *sink_guard {
                                sk.stop();
                            }
                        }
                        // Next/Previous solo aplican a fuentes con múltiples tracks — los soundscapes los ignoran
                        MediaControlEvent::Next => {
                            let current = state_clone.lock().unwrap().current_soundscape.clone();
                            if current == "Local Folder" || current.starts_with("Local:") {
                                trigger_local_folder(state_clone.clone(), sink_clone.clone(), handle_clone.clone());
                            } else if current == "Music For Programming" || current.starts_with("MFP:") {
                                trigger_music_for_programming(state_clone.clone(), sink_clone.clone(), handle_clone.clone());
                            }
                        }
                        MediaControlEvent::Previous => {
                            let current = state_clone.lock().unwrap().current_soundscape.clone();
                            if current == "Local Folder" || current.starts_with("Local:") {
                                trigger_local_folder(state_clone.clone(), sink_clone.clone(), handle_clone.clone());
                            } else if current == "Music For Programming" || current.starts_with("MFP:") {
                                trigger_music_for_programming(state_clone.clone(), sink_clone.clone(), handle_clone.clone());
                            }
                        }
                        _ => {}
                    }
                }

                // sincroniza el estado interno con lo que ve el OS — solo cuando cambia algo
                let (status, current_title) = {
                    let state = state_clone.lock().unwrap();
                    (state.status, state.current_soundscape.clone())
                };

                if status != last_status {
                    let playback = match status {
                        PlaybackStatus::Playing => MediaPlayback::Playing { progress: None },
                        PlaybackStatus::Paused => MediaPlayback::Paused { progress: None },
                        PlaybackStatus::Stopped => MediaPlayback::Stopped,
                    };
                    let _ = controls.set_playback(playback);
                    last_status = status;
                }

                if current_title != last_title {
                    // actualiza el título en el applet de media para que se vea bonito
                    let _ = controls.set_metadata(MediaMetadata {
                        title: Some(&current_title),
                        artist: Some("Questline"),
                        album: Some("Atmospheres"),
                        ..Default::default()
                    });
                    last_title = current_title;
                }

                // poll cada 150ms — suficiente responsividad sin quemar CPU
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        });
    }
}

// despacha la reproducción al handler correcto según el tipo de soundscape
fn trigger_playback(
    soundscape_name: &str,
    state_clone: &Arc<Mutex<AudioState>>,
    sink_clone: &Arc<Mutex<Option<Sink>>>,
    handle_clone: &Option<rodio::OutputStreamHandle>,
) {
    let mut state = state_clone.lock().unwrap();
    state.current_soundscape = soundscape_name.to_string();

    // "Silent" y "None" son casos especiales — simplemente para todo
    if soundscape_name == "Silent" || soundscape_name == "None" {
        state.status = PlaybackStatus::Stopped;
        let sink_guard = sink_clone.lock().unwrap();
        if let Some(ref sk) = *sink_guard {
            sk.stop();
        }
        return;
    }

    state.status = PlaybackStatus::Playing;

    let sink_guard = sink_clone.lock().unwrap();
    if let Some(ref sk) = *sink_guard {
        sk.stop(); // detener lo que estaba antes para no mezclar
    }

    if soundscape_name == "Music For Programming" {
        // drop explícito antes de spawn para no tener deadlock en el otro thread
        drop(sink_guard);
        drop(state);
        trigger_music_for_programming(state_clone.clone(), sink_clone.clone(), handle_clone.clone());
        return;
    }

    if soundscape_name == "Local Folder" {
        drop(sink_guard);
        drop(state);
        trigger_local_folder(state_clone.clone(), sink_clone.clone(), handle_clone.clone());
        return;
    }

    if let Some(src) = build_source(soundscape_name) {
        if let Some(ref sk) = *sink_guard {
            sk.append(src);
            sk.play();
        }
    }
}

// baja un episodio aleatorio de musicforprogramming.net, lo cachea y lo reproduce
fn trigger_music_for_programming(
    state_clone: Arc<Mutex<AudioState>>,
    sink_clone: Arc<Mutex<Option<Sink>>>,
    handle_clone: Option<rodio::OutputStreamHandle>,
) {
    std::thread::spawn(move || {
        // todo esto corre en background — el UI no espera, la descarga puede tardar
        let rss_url = "https://musicforprogramming.net/rss.xml";
        let body = match ureq::get(rss_url).timeout(std::time::Duration::from_secs(10)).call() {
            Ok(resp) => match resp.into_string() {
                Ok(s) => s,
                Err(_) => return,
            },
            Err(_) => return,
        };

        let mut urls = Vec::new();
        for line in body.lines() {
            if let Some(start) = line.find("<enclosure url=\"") {
                let rest = &line[start + 16..];
                if let Some(end) = rest.find("\"") {
                    let url = &rest[..end];
                    urls.push(url.to_string());
                }
            }
        }

        if urls.is_empty() {
            return;
        }

        // usa el timestamp como semilla para elegir episodio — aleatorio sin importar rand
        let index = match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(d) => (d.as_millis() as usize) % urls.len(),
            Err(_) => 0,
        };
        let mp3_url = urls[index].clone();

        let display_name = if let Some(filename) = mp3_url.split('/').last() {
            if let Some(name_without_ext) = filename.strip_suffix(".mp3") {
                name_without_ext.replace('_', " ").replace('-', " : ")
            } else {
                "Music For Programming".to_string()
            }
        } else {
            "Music For Programming".to_string()
        };

        {
            let mut state = state_clone.lock().unwrap();
            if state.current_soundscape == "Music For Programming" {
                state.current_soundscape = "MFP: Loading mix...".to_string();
            } else {
                return;
            }
        }

        let storage_dir = match crate::storage::get_storage_dir() {
            Ok(d) => d,
            Err(_) => return,
        };
        let temp_file_path = storage_dir.join("music_for_programming.mp3");

        let resp = match ureq::get(&mp3_url).timeout(std::time::Duration::from_secs(30)).call() {
            Ok(r) => r,
            Err(_) => return,
        };

        let mut file = match std::fs::File::create(&temp_file_path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let mut reader = resp.into_reader();
        if std::io::copy(&mut reader, &mut file).is_err() {
            return;
        }

        {
            // check: si el usuario cambió de soundscape mientras descargaba, cancelar y limpiar
            let state = state_clone.lock().unwrap();
            if !state.current_soundscape.starts_with("MFP:") && state.current_soundscape != "Music For Programming" {
                let _ = std::fs::remove_file(&temp_file_path);
                return;
            }
        }

        if let Some(ref handle) = handle_clone {
            let file = match std::fs::File::open(&temp_file_path) {
                Ok(f) => f,
                Err(_) => return,
            };
            let source = match rodio::Decoder::new(file) {
                Ok(src) => src,
                Err(_) => return,
            };

            let sink = match Sink::try_new(handle) {
                Ok(sk) => sk,
                Err(_) => return,
            };

            let volume = state_clone.lock().unwrap().volume;
            sink.set_volume(volume);
            sink.pause();
            sink.append(source);
            sink.play();

            {
                let mut state = state_clone.lock().unwrap();
                state.current_soundscape = display_name;
            }

            let mut sink_guard = sink_clone.lock().unwrap();
            *sink_guard = Some(sink);
        }
    });
}

// carga todos los archivos de audio de la carpeta local, los baraja y los encola en el sink
fn trigger_local_folder(
    state_clone: Arc<Mutex<AudioState>>,
    sink_clone: Arc<Mutex<Option<Sink>>>,
    handle_clone: Option<rodio::OutputStreamHandle>,
) {
    // lee el path desde el state — nunca tocamos la DB desde el thread de audio
    let folder_path = {
        let state = state_clone.lock().unwrap();
        state.local_music_folder.clone()
    };

    std::thread::spawn(move || {
        if folder_path.trim().is_empty() {
            let mut state = state_clone.lock().unwrap();
            state.current_soundscape = "Local Music: Folder not configured (press 'f')".to_string();
            return;
        }

        // expandir "~" manualmente — std::path no lo hace por sí solo en Rust
        let mut path_str = folder_path;
        if path_str.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                if path_str == "~" {
                    path_str = home;
                } else if path_str.starts_with("~/") {
                    path_str = path_str.replacen('~', &home, 1);
                }
            }
        }

        let dir_path = std::path::Path::new(&path_str);
        if !dir_path.exists() || !dir_path.is_dir() {
            let mut state = state_clone.lock().unwrap();
            state.current_soundscape = "Local Music: Path does not exist".to_string();
            return;
        }

        let entries = match std::fs::read_dir(dir_path) {
            Ok(e) => e,
            Err(e) => {
                crate::services::log_structured(
                    "ERROR",
                    "local_music",
                    "Failed to read local music directory",
                    Some(&format!("Path: '{}', Error: {}", path_str, e)),
                );
                let mut state = state_clone.lock().unwrap();
                state.current_soundscape = "Local Music: Read error".to_string();
                return;
            }
        };

        // filtra solo formatos que rodio puede decodear — nada de .m4a o .aac por ahorita
        let mut audio_files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "mp3" || ext_lower == "ogg" || ext_lower == "wav" || ext_lower == "flac" {
                        audio_files.push(path);
                    }
                }
            }
        }

        if audio_files.is_empty() {
            let mut state = state_clone.lock().unwrap();
            state.current_soundscape = "Local Music: No audio files found".to_string();
            return;
        }

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        audio_files.shuffle(&mut rng);

        if let Some(ref handle) = handle_clone {
            let sink = match Sink::try_new(handle) {
                Ok(sk) => sk,
                Err(_) => return,
            };

            // poner volumen ANTES de append — rodio default es 1.0, no queremos susto al inicio
            let volume = state_clone.lock().unwrap().volume;
            sink.set_volume(volume);
            sink.pause();

            let mut play_count = 0;
            let mut first_file_name = String::new();

            for path in audio_files {
                let file = match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let source = match rodio::Decoder::new(file) {
                    Ok(src) => src,
                    Err(_) => continue,
                };

                if play_count == 0 {
                    first_file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Local Track")
                        .to_string();
                }

                sink.append(source);
                play_count += 1;
            }

            if play_count == 0 {
                let mut state = state_clone.lock().unwrap();
                state.current_soundscape = "Local Music: Failed to decode tracks".to_string();
                return;
            }

            sink.play();

            {
                let mut state = state_clone.lock().unwrap();
                state.current_soundscape = format!("Local: {}", first_file_name);
            }

            let mut sink_guard = sink_clone.lock().unwrap();
            *sink_guard = Some(sink);
        }
    });
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_sound_decoding() {
        let cursor1 = std::io::Cursor::new(include_bytes!("../../assets/sounds/taskcreation.ogg"));
        let _decoder1 = rodio::Decoder::new(cursor1).expect("Failed to decode embedded taskcreation.ogg bytes");

        let cursor2 = std::io::Cursor::new(include_bytes!("../../assets/sounds/focus-task-complete.ogg"));
        let _decoder2 = rodio::Decoder::new(cursor2).expect("Failed to decode embedded focus-task-complete.ogg bytes");
    }
}
