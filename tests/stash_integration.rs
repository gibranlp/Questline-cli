use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use questline::{app::{App, ModalType}, screens::{self, ActiveScreen}, theme::Theme};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn stash_screen_replaces_local_music_and_keeps_focus_sources() {
    if !questline::audio::mpris_player::uses_stash() { return; }
    let root = std::env::temp_dir().join(format!("questline-stash-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let mut app = App::new_headless(&root.join("profile.db")).unwrap();
    app.active_screen = ActiveScreen::Soundscapes;
    app.modal_state = ModalType::None;
    app.stash_library.snapshot = serde_json::json!({"offset":0,"total":2,"tracks":[
        {"title":"First song","path":"/Music/first.wav"},{"title":"Second song","path":"/Music/second.wav"}]});
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| {
        let regions = screens::soundscapes::draw(frame, &app, &Theme::default_theme(), frame.size());
        assert_eq!(regions.item_count, 2);
        assert!(regions.local_tracks.is_none());
        app.hit_regions.soundscapes = Some(regions);
    }).unwrap();
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Stash"));
    assert!(text.contains("First song"));
    assert!(text.contains("add folder"));
    assert!(!text.contains("Local Library"));
    for key in [KeyCode::Down, KeyCode::Char('/'), KeyCode::Char('S'), KeyCode::Esc] {
        app.handle_key_event(KeyEvent::new(key, if key == KeyCode::Char('S') { KeyModifiers::SHIFT } else { KeyModifiers::NONE })).unwrap();
        assert_eq!(app.modal_state, ModalType::None);
        assert_eq!(app.audio_player.get_state().status, questline::audio::PlaybackStatus::Stopped);
    }
    assert_eq!(app.stash_library.selected, 1);
    assert_eq!(app.stash_library.input, "S");
    std::fs::create_dir_all(root.join("Music With Spaces")).unwrap();
    app.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE)).unwrap();
    app.stash_library.input = root.join("Music W").to_string_lossy().into_owned();
    app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)).unwrap();
    assert_eq!(app.active_screen, ActiveScreen::Soundscapes);
    assert!(app.stash_library.input.ends_with("/Music With Spaces/"));
    assert!(app.stash_library.editing);
    terminal.draw(|frame| { screens::soundscapes::draw(frame, &app, &Theme::default_theme(), frame.size()); }).unwrap();
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Tab: complete"));
    app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).unwrap();
    assert!(questline::audio::SOUNDSCAPES.iter().any(|source| source.name == "Rain Sounds"));
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires private MPRIS fixture bus"]
fn controls_stash_while_another_player_is_active() {
    let track = questline::audio::mpris_player::get_now_playing().unwrap();
    assert_eq!(track.player, "stash");
    assert_eq!(track.title, "Stash fixture");
    for action in ["toggle", "next", "previous", "pause"] {
        questline::audio::mpris_player::stash_action(action).unwrap();
    }
}

#[test]
#[ignore = "requires private session and temporary Stash music fixture"]
fn library_starts_a_background_engine_and_plays_without_another_window() {
    use std::time::{Duration, Instant};
    let mut library = questline::audio::stash_library::Library::default();
    let deadline = Instant::now() + Duration::from_secs(15);
    while library.snapshot["total"].as_u64().unwrap_or(0) < 2 {
        library.poll();
        assert!(Instant::now() < deadline, "Stash library failed: {}", library.error);
        std::thread::sleep(Duration::from_millis(40));
    }
    let pid = library.snapshot["pid"].as_u64().unwrap();
    library.key(KeyCode::Enter);
    let deadline = Instant::now() + Duration::from_secs(8);
    while library.snapshot["status"].as_str() != Some("Playing") {
        library.poll();
        assert!(Instant::now() < deadline, "Playback did not start: {}", library.error);
        std::thread::sleep(Duration::from_millis(40));
    }
    assert_eq!(library.snapshot["pid"].as_u64(), Some(pid));
    library.key(KeyCode::Char('s'));
    let deadline = Instant::now() + Duration::from_secs(5);
    while library.snapshot["status"].as_str() != Some("Paused") {
        library.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(40));
    }
}

#[test]
#[ignore = "requires a missing STASH_BINARY override and fake browser"]
fn missing_stash_shows_github_download_and_opens_the_verified_link() {
    use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
    assert!(!questline::audio::stash_library::installed());
    let root = std::env::temp_dir().join(format!("questline-stash-download-{}",uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let mut app = App::new_headless(&root.join("profile.db")).unwrap();
    app.active_screen=ActiveScreen::Soundscapes;app.modal_state=ModalType::None;
    app.stash_library.poll();
    assert!(app.stash_library.error.is_empty());
    let mut terminal=Terminal::new(TestBackend::new(110,24)).unwrap();
    terminal.draw(|frame| {
        let regions=screens::soundscapes::draw(frame,&app,&Theme::default_theme(),frame.size());
        assert_eq!(regions.item_count,0);assert!(regions.folder_picker.is_some());
        app.hit_regions.soundscapes=Some(regions);
    }).unwrap();
    let text:String=terminal.backend().buffer().content.iter().map(|cell|cell.symbol()).collect();
    assert!(text.contains("Stash is not installed"));
    assert!(text.contains("Download Stash"));
    assert!(text.contains("github.com/gibranlp/stash/releases/latest/download/stash-linux-x86_64"));
    assert!(!text.contains("Starting Stash"));
    for key in [KeyCode::Enter,KeyCode::Char('d'),KeyCode::Char('g')] {
        app.handle_key_event(KeyEvent::new(key,KeyModifiers::NONE)).unwrap();
    }
    let button=app.hit_regions.soundscapes.unwrap().folder_picker.unwrap();
    app.handle_mouse_event(MouseEvent { kind:MouseEventKind::Down(MouseButton::Left),column:button.x+2,row:button.y,modifiers:KeyModifiers::NONE }).unwrap();
    drop(app);std::fs::remove_dir_all(root).unwrap();
}
