use std::{sync::mpsc::{self, Receiver, SyncSender}, time::{Duration, Instant}};
use serde_json::{Value, json};
use crossterm::event::KeyCode;

pub const RELEASES_URL: &str = "https://github.com/gibranlp/stash/releases/latest";

pub fn download_url() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "https://github.com/gibranlp/stash/releases/latest/download/stash-linux-x86_64",
        ("macos", "aarch64") => "https://github.com/gibranlp/stash/releases/latest/download/stash-macos-arm64",
        ("macos", "x86_64") => "https://github.com/gibranlp/stash/releases/latest/download/stash-macos-x86_64",
        ("windows", "x86_64") => "https://github.com/gibranlp/stash/releases/latest/download/stash-windows-x86_64.exe",
        _ => RELEASES_URL,
    }
}

fn executable(path: &std::path::Path) -> bool {
    if !path.is_file() { return false; }
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; path.metadata().is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0) }
    #[cfg(not(unix))]
    { true }
}

pub fn stash_binary() -> Option<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("STASH_BINARY") {
        let path = std::path::PathBuf::from(path);
        return executable(&path).then_some(path);
    }
    let name = if cfg!(target_os = "windows") { "stash.exe" } else { "stash" };
    let mut candidates: Vec<std::path::PathBuf> = std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).map(|path|path.join(name)).collect()).unwrap_or_default();
    if cfg!(unix) { candidates.extend(["/usr/bin/stash", "/usr/local/bin/stash"].map(std::path::PathBuf::from)); }
    candidates.into_iter().find(|path|executable(path))
}

pub fn installed() -> bool { stash_binary().is_some() }

pub struct Library {
    pub snapshot: Value,
    pub selected: usize,
    pub offset: usize,
    pub query: String,
    pub input: String,
    pub editing: bool,
    pub adding_folder: bool,
    pub folder_matches: Vec<String>,
    pub folder_match_selected: Option<usize>,
    pub error: String,
    request: Option<SyncSender<(u64, Value)>>,
    response: Option<Receiver<(u64, Result<Value, String>)>>,
    generation: u64,
    pending: bool,
    last_refresh: Instant,
}

impl Default for Library {
    fn default() -> Self {
        Self { snapshot: json!({}), selected: 0, offset: 0, query: String::new(), input: String::new(),
            editing: false, adding_folder: false, folder_matches: Vec::new(), folder_match_selected: None, error: String::new(), request: None, response: None,
            generation: 0, pending: false, last_refresh: Instant::now() - Duration::from_secs(5) }
    }
}

impl Library {
    pub fn poll(&mut self) {
        if !installed() { return; }
        if let Some(receiver) = &self.response {
            while let Ok((generation, result)) = receiver.try_recv() {
                if generation != self.generation { continue; }
                self.pending = false;
                match result {
                    Ok(value) => {
                        if let Some(error) = value["error"].as_str() { self.error = error.to_string(); }
                        else {
                            self.snapshot = value; self.error.clear();
                            let total = self.snapshot["total"].as_u64().unwrap_or(0) as usize;
                            self.selected = self.selected.min(total.saturating_sub(1));
                            if self.offset > self.selected { self.offset = self.selected / 64 * 64; }
                        }
                    },
                    Err(error) => self.error = error,
                }
            }
        }
        if !self.pending && self.last_refresh.elapsed() >= Duration::from_secs(1) { self.send("snapshot", json!({})); }
    }

    pub fn send(&mut self, action: &str, extra: Value) {
        if !installed() && self.request.is_none() { return; }
        if self.request.is_none() {
            let (tx, rx) = mpsc::sync_channel::<(u64, Value)>(8);
            let (result_tx, result_rx) = mpsc::channel();
            std::thread::spawn(move || {
                let mut last_start = Instant::now() - Duration::from_secs(10);
                while let Ok((generation, request)) = rx.recv() {
                    let result = exchange(&request).or_else(|first_error| {
                        if last_start.elapsed() < Duration::from_secs(5) { return Err(first_error); }
                        last_start = Instant::now();
                        start_engine()?;
                        for _ in 0..30 {
                            std::thread::sleep(Duration::from_millis(100));
                            if let Ok(value) = exchange(&request) { return Ok(value); }
                        }
                        Err("Stash could not start. Reopen the tab to retry.".to_string())
                    });
                    if result_tx.send((generation, result)).is_err() { break; }
                }
            });
            self.request = Some(tx);
            self.response = Some(result_rx);
        }
        let mut request = json!({"action":action,"query":self.query,"offset":self.offset});
        if let Some(extra) = extra.as_object() { for (key, value) in extra { request[key] = value.clone(); } }
        let generation = self.generation + 1;
        if self.request.as_ref().unwrap().try_send((generation, request)).is_ok() {
            self.generation = generation;
            self.pending = true;
            self.last_refresh = Instant::now();
        }
    }

    pub fn key(&mut self, key: KeyCode) -> bool {
        if !installed() && self.request.is_none() {
            return matches!(key, KeyCode::Enter | KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Char('d'|'g'|'f'|'r'|'o'|'p'|'n'|'b'|'s'|'j'|'k'|'h'|'l'|'/'|' '|'+'|'-'|'*'));
        }
        if self.editing {
            match key {
                KeyCode::Esc => self.editing = false,
                KeyCode::Tab if self.adding_folder => self.complete_folder(),
                KeyCode::Down if self.adding_folder && !self.folder_matches.is_empty() => self.cycle_folder(false),
                KeyCode::Up | KeyCode::BackTab if self.adding_folder && !self.folder_matches.is_empty() => self.cycle_folder(true),
                KeyCode::Backspace => { self.input.pop(); self.clear_folder_matches(); },
                KeyCode::Char(character) => { self.input.push(character); self.clear_folder_matches(); },
                KeyCode::Enter => {
                    self.editing = false;
                    if self.adding_folder {
                        let path = if self.input.starts_with("~/") || std::path::Path::new(&self.input).is_absolute() {
                            self.input.clone()
                        } else { std::env::current_dir().unwrap_or_default().join(&self.input).to_string_lossy().into_owned() };
                        self.send("add_folder", json!({"path":path}));
                    }
                    else { self.query = self.input.clone(); self.selected = 0; self.offset = 0; self.send("snapshot", json!({})); }
                },
                _ => {},
            }
            return true;
        }
        match key {
            KeyCode::Char('/') | KeyCode::Char('f') => {
                self.editing = true; self.adding_folder = key == KeyCode::Char('f');
                self.clear_folder_matches();
                self.input = if self.adding_folder { String::new() } else { self.query.clone() };
            },
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                if self.selected < self.offset { self.offset = self.selected / 64 * 64; self.send("snapshot", json!({})); }
            },
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.snapshot["total"].as_u64().unwrap_or(0) as usize;
                if self.selected + 1 < total { self.selected += 1; }
                if self.selected >= self.offset + 64 { self.offset = self.selected / 64 * 64; self.send("snapshot", json!({})); }
            },
            KeyCode::Enter => self.play_selected(),
            KeyCode::Char('p' | ' ') => self.send("toggle", json!({})),
            KeyCode::Char('n') => self.send("next", json!({})),
            KeyCode::Char('b') => self.send("previous", json!({})),
            KeyCode::Char('s') => self.send("pause", json!({})),
            KeyCode::Char('r' | 'o') => self.send("rescan", json!({})),
            KeyCode::Char('+' | '-') => {
                let volume = self.snapshot["volume"].as_u64().unwrap_or(75) as i64;
                self.send("volume", json!({"value":(volume + if key == KeyCode::Char('+') { 5 } else { -5 }).clamp(0,100)}));
            },
            KeyCode::Esc if !self.query.is_empty() => {
                self.query.clear(); self.selected = 0; self.offset = 0; self.send("snapshot", json!({}));
            },
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l' | '*') => {},
            _ => return false,
        }
        true
    }

    fn clear_folder_matches(&mut self) {
        self.folder_matches.clear();
        self.folder_match_selected = None;
    }

    fn complete_folder(&mut self) {
        if !self.folder_matches.is_empty() { self.cycle_folder(false); return; }
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let cwd = std::env::current_dir().unwrap_or_default();
        self.folder_matches = directory_matches(&self.input, &home, &cwd);
        self.folder_match_selected = None;
        if self.folder_matches.len() == 1 {
            self.input = self.folder_matches[0].clone();
            self.clear_folder_matches();
        } else if let Some(first) = self.folder_matches.first() {
            let common = self.folder_matches.iter().skip(1).fold(first.clone(), |prefix, path| {
                prefix.chars().zip(path.chars()).take_while(|(left,right)| left == right).map(|(character,_)|character).collect()
            });
            if !common.is_empty() { self.input = common; }
        }
    }

    fn cycle_folder(&mut self, reverse: bool) {
        let count = self.folder_matches.len();
        if count == 0 { return; }
        let index = match self.folder_match_selected {
            Some(index) if reverse => (index + count - 1) % count,
            Some(index) => (index + 1) % count,
            None if reverse => count - 1,
            None => 0,
        };
        self.folder_match_selected = Some(index);
        self.input = self.folder_matches[index].clone();
    }

    pub fn play_selected(&mut self) {
        if self.snapshot["offset"].as_u64().unwrap_or(0) as usize != self.offset { return; }
        if let Some(track) = self.snapshot["tracks"].get(self.selected.saturating_sub(self.offset)) {
            if let Some(path) = track["path"].as_str() { self.send("play", json!({"path":path})); }
        }
    }
}

fn directory_matches(input: &str, home: &std::path::Path, cwd: &std::path::Path) -> Vec<String> {
    let text = if input.is_empty() || input == "~" { "~/" } else { input };
    let (parent, prefix) = text.rsplit_once('/').unwrap_or(("", text));
    let base = if parent == "~" { home.to_path_buf() }
        else if let Some(relative) = parent.strip_prefix("~/") { home.join(relative) }
        else if parent.is_empty() && text.starts_with('/') { std::path::PathBuf::from("/") }
        else if std::path::Path::new(parent).is_absolute() { parent.into() }
        else { cwd.join(parent) };
    let Ok(entries) = std::fs::read_dir(&base) else { return Vec::new(); };
    let mut matches: Vec<String> = entries.filter_map(Result::ok).filter_map(|entry| {
        let name = entry.file_name().into_string().ok()?;
        if !name.starts_with(prefix) || (name.starts_with('.') && !prefix.starts_with('.')) || !entry.path().is_dir() { return None; }
        Some(if text.starts_with("~/") { format!("{parent}/{name}/") }
            else { base.join(name).to_string_lossy().into_owned() + "/" })
    }).collect();
    matches.sort();
    matches
}

#[cfg(target_os = "linux")]
fn exchange(request: &Value) -> Result<Value, String> {
    use std::{io::{BufRead, BufReader, Read, Write}, os::unix::net::UnixStream};
    let path = std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").ok_or("No desktop runtime directory")?).join("stash-player.sock");
    let mut stream = UnixStream::connect(path).map_err(|error| error.to_string())?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).map_err(|error| error.to_string())?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).map_err(|error| error.to_string())?;
    writeln!(stream, "{request}").map_err(|error| error.to_string())?;
    let mut line = String::new();
    BufReader::new(stream.take(1024*1024)).read_line(&mut line).map_err(|error| error.to_string())?;
    let result: Value = serde_json::from_str(&line).map_err(|error| error.to_string())?;
    Ok(result)
}

#[cfg(not(target_os = "linux"))]
fn exchange(_request: &Value) -> Result<Value, String> { Err("Stash integration requires Linux".to_string()) }

fn start_engine() -> Result<(), String> {
    let binary = stash_binary().ok_or_else(|| format!("Stash is not installed. Download it from {}", download_url()))?;
    let mut command = std::process::Command::new(binary);
    command.arg("--service").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(unix)]
    { use std::os::unix::process::CommandExt; command.process_group(0); }
    command.spawn().map(|mut child| { std::thread::spawn(move || { let _ = child.wait(); }); })
        .map_err(|error| format!("Could not start Stash: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Library, Receiver<(u64, Value)>) {
        let (tx, rx) = mpsc::sync_channel(8);
        let mut library = Library::default();
        library.request = Some(tx);
        library.snapshot = json!({"offset":0,"total":2,"volume":75,"tracks":[
            {"path":"/Music/first.wav","title":"First"},{"path":"/Music/second.wav","title":"Second"}]});
        (library, rx)
    }

    #[test]
    fn selected_track_search_and_folder_requests_use_the_shared_engine() {
        let (mut library, requests) = fixture();
        library.key(KeyCode::Down);
        library.key(KeyCode::Enter);
        assert_eq!(requests.recv().unwrap().1["path"], "/Music/second.wav");
        library.key(KeyCode::Char('/'));
        library.key(KeyCode::Char('F'));
        library.key(KeyCode::Enter);
        let request = requests.recv().unwrap().1;
        assert_eq!(request["query"], "F");
        assert_eq!(library.selected, 0);
        library.key(KeyCode::Char('f'));
        for character in "/Music/New Folder".chars() { library.key(KeyCode::Char(character)); }
        library.key(KeyCode::Enter);
        let request = requests.recv().unwrap().1;
        assert_eq!(request["action"], "add_folder");
        assert_eq!(request["path"], "/Music/New Folder");
    }

    #[test]
    fn pending_pages_cannot_play_a_track_from_the_previous_page() {
        let (mut library, requests) = fixture();
        library.offset = 64;
        library.selected = 64;
        library.play_selected();
        assert!(requests.try_recv().is_err());
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;

    #[test]
    fn directory_completion_handles_home_spaces_unicode_and_excludes_files() {
        let root = std::env::temp_dir().join(format!("questline-completion-{}", uuid::Uuid::new_v4()));
        for name in ["Music One", "Music Two", "Música", ".hidden"] { std::fs::create_dir_all(root.join(name)).unwrap(); }
        std::fs::write(root.join("Music file.mp3"), b"fixture").unwrap();
        assert_eq!(directory_matches("~/Music", &root, &root), vec!["~/Music One/", "~/Music Two/"]);
        assert_eq!(directory_matches("~/Mú", &root, &root), vec!["~/Música/"]);
        assert_eq!(directory_matches("~/.", &root, &root), vec!["~/.hidden/"]);
        assert!(!directory_matches("", &root, &root).iter().any(|path|path.contains(".hidden")));
        assert!(directory_matches("Music", &root, &root).iter().all(|path|std::path::Path::new(path).is_absolute()));
        assert!(directory_matches("~/missing/", &root, &root).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tab_expands_a_unique_directory_and_cycles_ambiguous_choices_without_submitting() {
        let root = std::env::temp_dir().join(format!("questline-completion-{}", uuid::Uuid::new_v4()));
        for name in ["Music One", "Music Two", "Videos"] { std::fs::create_dir_all(root.join(name)).unwrap(); }
        let mut library = Library::default();
        let (requests, received) = mpsc::sync_channel(8);
        library.request = Some(requests);
        library.key(KeyCode::Char('f'));
        library.input = root.join("Vid").to_string_lossy().into_owned();
        library.key(KeyCode::Tab);
        assert!(library.input.ends_with("/Videos/"));
        assert!(library.folder_matches.is_empty());
        assert!(library.editing);
        assert!(received.try_recv().is_err());
        library.input = root.join("Mu").to_string_lossy().into_owned();
        library.key(KeyCode::Tab);
        assert!(library.input.ends_with("/Music "));
        assert_eq!(library.folder_matches.len(),2);
        library.key(KeyCode::Tab);
        assert!(library.input.ends_with("/Music One/"));
        library.key(KeyCode::Down);
        assert!(library.input.ends_with("/Music Two/"));
        library.key(KeyCode::BackTab);
        assert!(library.input.ends_with("/Music One/"));
        library.key(KeyCode::Backspace);
        assert!(library.folder_matches.is_empty());
        library.key(KeyCode::Esc);
        assert!(!library.editing);
        assert!(received.try_recv().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod install_detection_tests {
    #[test]
    fn installation_requires_an_executable_file_and_download_targets_the_official_repo() {
        let root=std::env::temp_dir().join(format!("stash-detect-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let binary=root.join("stash");
        assert!(!super::executable(&binary));
        assert!(!super::executable(&root));
        std::fs::write(&binary,b"fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary,std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(!super::executable(&binary));
            std::fs::set_permissions(&binary,std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(super::executable(&binary));
        assert!(super::download_url().starts_with("https://github.com/gibranlp/stash/releases/latest"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
