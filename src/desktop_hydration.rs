use crate::{
    app::{App, ModalType},
    database::Database,
};
use anyhow::{Context, Result};
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
struct Reminder {
    token: String,
    day: String,
    count: i32,
    target: i32,
    pending: bool,
    acknowledged_at: Option<i64>,
}
fn path(db: &Database, suffix: &str) -> Result<PathBuf> {
    Ok(std::path::Path::new(
        db.conn
            .path()
            .context("Hydration requires a stored database")?,
    )
    .with_extension(suffix))
}
fn load(db: &Database) -> Result<Option<Reminder>> {
    let file = path(db, "hydration.json")?;
    if !file.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&std::fs::read(file)?)?))
}
fn save(db: &Database, state: &Reminder) -> Result<()> {
    use std::io::Write;
    let dest = path(db, "hydration.json")?;
    let temp = dest.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    file.write_all(&serde_json::to_vec(state)?)?;
    file.sync_all()?;
    std::fs::rename(temp, dest)?;
    Ok(())
}
fn lock(db: &Database) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path(db, "hydration.lock")?)?;
    file.lock()?;
    Ok(file)
}
fn alert(state: &Reminder) -> Value {
    json!({"title":"Hydration reminder","body":format!("{}/{} today\nYour hero runs on water, too.",state.count,state.target),"icon":"hydration","reminder_token":state.token,"persistent":true})
}
fn today() -> String {
    Local::now().date_naive().to_string()
}

pub fn resume(app: &mut App) -> Result<Option<Value>> {
    let _guard = lock(&app.db)?;
    app.reload_hydration_config();
    if app.db.get_setting("external_notifications")?.as_deref() == Some("false")
        || !app.hydration_enabled
    {
        return Ok(None);
    }
    if let Some(mut state) = load(&app.db)? {
        if state.pending && state.day == today() {
            app.reload_hydration_config();
            state.count = app.db.hydration_get_today()?.0;
            state.target = app.hydration_target.max(1);
            let value = alert(&state);
            save(&app.db, &state)?;
            return Ok(Some(value));
        }
    }
    Ok(None)
}

pub fn tick(app: &mut App, focused: bool) -> Result<Vec<Value>> {
    let _guard = lock(&app.db)?;
    let old_interval = app.hydration_interval_mins;
    app.reload_hydration_config();
    app.hydration_interval_mins = app.hydration_interval_mins.max(1);
    app.hydration_target = app.hydration_target.max(1);
    let count = app.db.hydration_get_today()?.0;
    if !app.hydration_enabled {
        app.hydration_next_reminder_at = None;
        return Ok(vec![]);
    }
    let previous = load(&app.db)?.filter(|state| state.day == today());
    if let Some(mut state) = previous
        .as_ref()
        .filter(|state| state.pending)
        .map(|state| Reminder {
            token: state.token.clone(),
            day: state.day.clone(),
            count: state.count,
            target: state.target,
            pending: true,
            acknowledged_at: state.acknowledged_at,
        })
    {
        if state.count != count || state.target != app.hydration_target {
            state.count = count;
            state.target = app.hydration_target;
            let value = alert(&state);
            save(&app.db, &state)?;
            return Ok(vec![value]);
        }
        return Ok(vec![]);
    }
    if !app.hydration_enabled
        || count >= app.hydration_target
        || (focused && app.hydration_pause_focus)
    {
        app.hydration_next_reminder_at = None;
        return Ok(vec![]);
    }
    let last_ack = previous.and_then(|state| state.acknowledged_at);
    let last_drink = app
        .db
        .hydration_last_drink_at()?
        .map(|date| date.timestamp());
    if let Some(last) = last_ack.into_iter().chain(last_drink).max() {
        let remaining =
            (last + app.hydration_interval_mins as i64 * 60 - Utc::now().timestamp()).max(0);
        app.hydration_next_reminder_at =
            Some(Instant::now() + Duration::from_secs(remaining as u64));
    } else if old_interval != app.hydration_interval_mins {
        app.hydration_next_reminder_at =
            Some(Instant::now() + Duration::from_secs(app.hydration_interval_mins as u64 * 60));
    }
    app.tick_hydration()?;
    if !matches!(app.modal_state, ModalType::HydrationReminder) {
        return Ok(vec![]);
    }
    app.modal_state = ModalType::None;
    app.hydration_next_reminder_at = None;
    let state = Reminder {
        token: uuid::Uuid::new_v4().to_string(),
        day: today(),
        count,
        target: app.hydration_target,
        pending: true,
        acknowledged_at: None,
    };
    let value = alert(&state);
    save(&app.db, &state)?;
    Ok(vec![value])
}

pub fn dismiss(db: &Database, token: &str) -> Result<Value> {
    let _guard = lock(db)?;
    if let Some(mut state) = load(db)? {
        if state.pending && (token.is_empty() || state.token == token) {
            state.pending = false;
            state.acknowledged_at = Some(Utc::now().timestamp());
            save(db, &state)?;
            return Ok(json!({"dismissed":true}));
        }
    }
    Ok(json!({"dismissed":false}))
}

pub fn drink(app: &mut App, token: &str) -> Result<Value> {
    let _guard = lock(&app.db)?;
    app.reload_hydration_config();
    app.hydration_interval_mins = app.hydration_interval_mins.max(1);
    app.hydration_target = app.hydration_target.max(1);
    let state = load(&app.db)?;
    let before = app.db.hydration_get_today()?.0;
    if !token.is_empty()
        && !state
            .as_ref()
            .is_some_and(|state| state.pending && state.token == token && state.day == today())
    {
        return Ok(
            json!({"recorded":false,"count":before,"target":app.hydration_target,"message":"This reminder has already been handled."}),
        );
    }
    app.record_hydration_drink(true)?;
    let after = app.db.hydration_get_today()?.0;
    let state = Reminder {
        token: state
            .map(|s| s.token)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        day: today(),
        count: after,
        target: app.hydration_target,
        pending: false,
        acknowledged_at: Some(Utc::now().timestamp()),
    };
    save(&app.db, &state)?;
    Ok(
        json!({"recorded":after>before,"count":after,"target":app.hydration_target,"message":format!("{}/{} today",after,app.hydration_target)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ClassType, User};
    use uuid::Uuid;
    fn app() -> (App, PathBuf) {
        let path = std::env::temp_dir().join(format!("questline-water-{}.db", Uuid::new_v4()));
        let mut app = App::new_headless(&path).unwrap();
        app.db
            .insert_user(&User {
                id: Uuid::new_v4(),
                username: "Water tester".into(),
                class: ClassType::MindSage,
                level: 1,
                xp: 0,
                created_at: Utc::now(),
                specialization: None,
            })
            .unwrap();
        app.reload_data().unwrap();
        for (key, value) in [
            ("external_notifications", "true"),
            ("hydration_enabled", "true"),
            ("hydration_interval_mins", "30"),
            ("hydration_active_from", "0"),
            ("hydration_active_to", "24"),
            ("hydration_target", "12"),
        ] {
            app.db.set_setting(key, value).unwrap();
        }
        app.external_notifications = false;
        app.active_screen = crate::screens::ActiveScreen::Dashboard;
        app.hydration_interval_mins = 30;
        app.hydration_next_reminder_at = Some(Instant::now() - Duration::from_secs(1));
        (app, path)
    }
    fn token(app: &mut App) -> String {
        tick(app, false).unwrap()[0]["reminder_token"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn cleanup(app: App, path: PathBuf) {
        for suffix in ["hydration.json", "hydration.lock"] {
            let _ = std::fs::remove_file(path.with_extension(suffix));
        }
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn reminder_includes_target_and_persists_without_repeating() {
        let (mut app, path) = app();
        let alerts = tick(&mut app, false).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["persistent"], true);
        assert!(alerts[0]["body"].as_str().unwrap().contains("0/12 today"));
        app.hydration_next_reminder_at = Some(Instant::now() - Duration::from_secs(1));
        assert!(tick(&mut app, false).unwrap().is_empty());
        assert_eq!(
            resume(&mut app).unwrap().unwrap()["reminder_token"],
            alerts[0]["reminder_token"]
        );
        cleanup(app, path);
    }
    #[test]
    fn dismissal_does_not_count_a_drink_and_waits_the_configured_interval() {
        let (mut app, path) = app();
        let token = token(&mut app);
        assert_eq!(dismiss(&app.db, &token).unwrap()["dismissed"], true);
        assert_eq!(app.db.hydration_get_today().unwrap().0, 0);
        assert!(tick(&mut app, false).unwrap().is_empty());
        assert!(
            app.hydration_next_reminder_at.unwrap() > Instant::now() + Duration::from_secs(1700)
        );
        cleanup(app, path);
    }
    #[test]
    fn drink_action_counts_once_for_each_reminder() {
        let (mut app, path) = app();
        let token = token(&mut app);
        let first = drink(&mut app, &token).unwrap();
        assert_eq!(first["recorded"], true);
        assert_eq!(first["count"], 1);
        assert_eq!(first["target"], 12);
        let again = drink(&mut app, &token).unwrap();
        assert_eq!(again["recorded"], false);
        assert_eq!(app.db.hydration_get_today().unwrap().0, 1);
        assert!(tick(&mut app, false).unwrap().is_empty());
        cleanup(app, path);
    }
    #[test]
    fn daily_target_uses_native_reward_once_and_stops_new_reminders() {
        let (mut app, path) = app();
        app.db.set_setting("hydration_target", "1").unwrap();
        let token = token(&mut app);
        let snapshot = crate::desktop_notifications::RewardSnapshot::capture(&app.db).unwrap();
        let request = json!({"action":"drink_water","reminder_token":token});
        let response = drink(&mut app, &token).unwrap();
        let receipt = snapshot.attach(&app.db, &request, response).unwrap();
        assert!(receipt["reward"]["xp_gained"].as_i64().unwrap() > 0);
        assert!(app.db.hydration_get_today().unwrap().1);
        let before: i64 = app
            .db
            .conn
            .query_row("SELECT SUM(xp_gained) FROM xp_events", [], |r| r.get(0))
            .unwrap();
        drink(&mut app, &token).unwrap();
        let after: i64 = app
            .db
            .conn
            .query_row("SELECT SUM(xp_gained) FROM xp_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
        app.hydration_next_reminder_at = Some(Instant::now() - Duration::from_secs(1));
        assert!(tick(&mut app, false).unwrap().is_empty());
        cleanup(app, path);
    }
    #[test]
    fn stale_reminders_cannot_count_todays_drinks_or_clear_a_new_reminder() {
        let (mut app, path) = app();
        let token = token(&mut app);
        let mut state = load(&app.db).unwrap().unwrap();
        state.day = "2000-01-01".into();
        save(&app.db, &state).unwrap();
        assert_eq!(drink(&mut app, &token).unwrap()["recorded"], false);
        app.hydration_next_reminder_at = Some(Instant::now() - Duration::from_secs(1));
        let new = token_for(&mut app);
        assert_ne!(new, token);
        assert_eq!(dismiss(&app.db, &token).unwrap()["dismissed"], false);
        assert!(load(&app.db).unwrap().unwrap().pending);
        cleanup(app, path);
    }
    fn token_for(app: &mut App) -> String {
        tick(app, false).unwrap()[0]["reminder_token"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    #[test]
    fn target_updates_refresh_the_existing_reminder_without_creating_another() {
        let (mut app, path) = app();
        let token = token(&mut app);
        app.db.set_setting("hydration_target", "8").unwrap();
        let update = tick(&mut app, false).unwrap();
        assert_eq!(update[0]["reminder_token"], token);
        assert!(update[0]["body"].as_str().unwrap().contains("0/8 today"));
        assert!(tick(&mut app, false).unwrap().is_empty());
        cleanup(app, path);
    }
}
