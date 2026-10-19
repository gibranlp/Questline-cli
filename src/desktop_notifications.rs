use crate::{
    app::{App, NotificationKind},
    database::Database,
    screens::ActiveScreen,
    services::notifications::{self, NotificationIcon},
};
use anyhow::Result;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    path::Path,
    time::{Duration, Instant},
};

pub struct RewardSnapshot {
    xp: i64,
    level: i32,
    achievements: HashSet<String>,
}

impl RewardSnapshot {
    pub fn capture(db: &Database) -> Result<Self> {
        Ok(Self {
            xp: db.conn.query_row(
                "SELECT COALESCE(SUM(xp_gained),0) FROM xp_events",
                [],
                |r| r.get(0),
            )?,
            level: db.get_user()?.map(|u| u.level).unwrap_or(1),
            achievements: db
                .get_achievements()?
                .into_iter()
                .filter(|a| a.unlocked_at.is_some())
                .map(|a| a.id)
                .collect(),
        })
    }

    pub fn attach(&self, db: &Database, request: &Value, mut response: Value) -> Result<Value> {
        let now = Self::capture(db)?;
        let xp = (now.xp - self.xp).max(0);
        let achievements: Vec<_> = db
            .get_achievements()?
            .into_iter()
            .filter(|a| a.unlocked_at.is_some() && !self.achievements.contains(&a.id))
            .map(|a| json!({"id":a.id,"name":a.name,"description":a.description}))
            .collect();
        let mut alerts = Vec::new();
        let action = request["action"].as_str().unwrap_or("");
        if xp > 0
            && matches!(
                action,
                "complete_task" | "complete_sidequest" | "toggle_step" | "water_tree"
            )
        {
            let title = match action {
                "complete_sidequest" => "Sidequest completed",
                "toggle_step" => "Step completed",
                "water_tree" => "Evergrowth watered",
                _ => "Quest completed",
            };
            let label = request["id"]
                .as_str()
                .and_then(|id| {
                    uuid::Uuid::parse_str(id)
                        .ok()
                        .and_then(|id| db.get_task_by_id(id).ok().map(|t| t.title))
                        .or_else(|| {
                            db.get_rituals()
                                .ok()?
                                .into_iter()
                                .find(|r| r.id == id)
                                .map(|r| r.name)
                        })
                })
                .unwrap_or_default();
            alerts.push(json!({"title":title,"body":format!("{}{}+{} XP",label,if label.is_empty(){""}else{"\n"},xp),"icon":"task"}));
        }
        if action == "drink_water" && response["recorded"] == true {
            let count = response["count"].as_i64().unwrap_or(0);
            let target = response["target"].as_i64().unwrap_or(1);
            alerts.push(json!({"title":if count>=target {"Daily hydration goal reached!"} else {"Drink recorded"},"body":format!("{}/{} today{}",count,target,if xp>0 {format!(" · +{} XP",xp)}else{String::new()}),"icon":"hydration-progress"}));
        }
        if now.level > self.level {
            alerts.push(json!({"title":"Level up!","body":format!("Level {}. Your backlog has been warned.",now.level),"icon":"level"}));
        }
        for achievement in &achievements {
            alerts.push(json!({"title":"Achievement unlocked!","body":format!("{}\n{}",achievement["name"].as_str().unwrap_or(""),achievement["description"].as_str().unwrap_or("")),"icon":"achievement"}));
        }
        response["reward"] = json!({"xp_gained":xp,"level":now.level,"level_up":now.level>self.level,"achievements":achievements});
        response["desktop_notifications"] = json!(alerts);
        response["notifications_enabled"] = json!(enabled(db)?);
        Ok(response)
    }
}

fn enabled(db: &Database) -> Result<bool> {
    Ok(db.get_setting("external_notifications")?.as_deref() != Some("false"))
}

pub fn deliver_response(response: &Value) {
    if response["notifications_enabled"] != true {
        return;
    }
    if let Some(alerts) = response["desktop_notifications"].as_array() {
        for alert in alerts {
            let icon = match alert["icon"].as_str() {
                Some("level") => NotificationIcon::LevelUp,
                Some("achievement") => NotificationIcon::Achievement,
                _ => NotificationIcon::TaskCompleted,
            };
            if alert["icon"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("hydration"))
            {
                send_alert(alert);
            } else {
                notifications::send_system_notification_blocking(
                    alert["title"].as_str().unwrap_or("Questline"),
                    alert["body"].as_str().unwrap_or(""),
                    icon,
                );
            }
        }
    }
}

fn lock_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}

pub fn tui_presence(directory: &Path) -> Result<File> {
    let file = lock_file(&directory.join("desktop-tui.lock"))?;
    file.lock_shared()?;
    Ok(file)
}

fn spectrum_focus_active() -> bool {
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|p| std::path::PathBuf::from(p).join(".local/state"))
        });
    let Some(root) = root else {
        return false;
    };
    let state = std::fs::read(root.join("spectrumos/focus/state.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    state.is_some_and(|state| {
        let fresh =
            chrono::Utc::now().timestamp() as f64 - state["updated"].as_f64().unwrap_or(0.0) < 30.0;
        fresh
            && matches!(
                state["phase"].as_str(),
                Some("starting" | "work" | "paused")
            )
    })
}

fn chapter_cache(app: &mut App) {
    use crate::models::chapter::{ChapterObjectiveProgress, ChapterProgressData};
    let cache = app
        .db
        .get_setting("chapter_progress_cache")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    app.chapter_progress = cache.map(|value| {
        let objectives = crate::models::chapter::get_active_chapter()
            .map(|chapter| {
                chapter
                    .objectives
                    .iter()
                    .map(|definition| {
                        let found = value["objectives"].as_array().and_then(|items| {
                            items
                                .iter()
                                .find(|o| o["type"].as_str() == Some(definition.id))
                        });
                        ChapterObjectiveProgress {
                            id: definition.id.into(),
                            name: definition.name.into(),
                            current: found.and_then(|v| v["current"].as_u64()).unwrap_or(0),
                            target: found
                                .and_then(|v| v["target"].as_u64())
                                .unwrap_or(definition.target),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        ChapterProgressData {
            chapter_id: value["chapter_id"].as_str().unwrap_or("").into(),
            objectives,
            completed: value["completed"].as_bool().unwrap_or(false),
            completed_at: value["completed_at"].as_str().map(str::to_owned),
        }
    });
}

pub fn tick(app: &mut App, focused: bool) -> Result<Vec<Value>> {
    let mut alerts = Vec::new();
    if !enabled(&app.db)? {
        app.hydration_next_reminder_at = None;
        return Ok(alerts);
    }
    app.external_notifications = false;
    app.active_screen = ActiveScreen::Dashboard;
    alerts.extend(crate::desktop_hydration::tick(app, focused)?);
    if !focused {
        chapter_cache(app);
        app.tick_sprite_notifications();
        for notification in app.notifications.drain(..) {
            if matches!(notification.kind, NotificationKind::Swarm) {
                alerts.push(
                    json!({"title":notification.title,"body":notification.message,"icon":"swarm"}),
                );
            }
        }
    }
    Ok(alerts)
}

pub fn send_alert(alert: &Value) {
    let kind = alert["icon"].as_str().unwrap_or("swarm");
    let icon = if kind.starts_with("hydration") {
        NotificationIcon::Hydration
    } else {
        NotificationIcon::NotificationSwarm
    };
    #[cfg(target_os = "linux")]
    {
        let mut command = std::process::Command::new("notify-send");
        command
            .arg("--app-name=Questline")
            .arg("--urgency=normal")
            .arg(if alert["persistent"] == true {
                "--expire-time=0"
            } else {
                "--expire-time=8000"
            })
            .arg(format!("--hint=string:x-questline-kind:{}", kind));
        if let Some(token) = alert["reminder_token"].as_str() {
            command.arg(format!(
                "--hint=string:x-questline-reminder-token:{}",
                token
            ));
        }
        if let Some(path) = notifications::notification_icon_path(icon) {
            command.arg("--icon").arg(path);
        }
        command
            .arg(alert["title"].as_str().unwrap_or("Questline"))
            .arg(alert["body"].as_str().unwrap_or(""))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if command.status().is_ok_and(|status| status.success()) {
            return;
        }
    }
    notifications::send_system_notification_blocking(
        alert["title"].as_str().unwrap_or("Questline"),
        alert["body"].as_str().unwrap_or(""),
        icon,
    );
}

pub fn run(path: &Path) -> Result<()> {
    let directory = path.parent().expect("Questline database directory");
    let singleton = lock_file(&directory.join("desktop-notifications.lock"))?;
    if singleton.try_lock().is_err() {
        return Ok(());
    }
    let presence = lock_file(&directory.join("desktop-tui.lock"))?;
    let mut app:Option<App> = None;
    let mut loaded_epoch=None;
    loop {
        if presence.try_lock().is_ok() {
            let result = (|| -> Result<Vec<Value>> {
                let mut alerts = Vec::new();
                if let Some(current)=app.as_ref() {
                    if current.db.get_setting("shell_profile_epoch")? != loaded_epoch || current.db.get_user()?.map(|u|u.id)!=current.user.as_ref().map(|u|u.id) {app=None;}
                }
                if path.is_file() && app.is_none() {
                    let mut loaded = App::new_headless(path)?;
                    if loaded.user.is_some() {
                        loaded.notifications.clear();
                        loaded.last_sprite_notification_time = Some(Instant::now());
                        if let Some(reminder) = crate::desktop_hydration::resume(&mut loaded)? {
                            alerts.push(reminder);
                        }
                        loaded_epoch=loaded.db.get_setting("shell_profile_epoch")?;
                        app = Some(loaded);
                    }
                }
                if let Some(app) = app.as_mut() {
                    alerts.extend(tick(app, spectrum_focus_active())?);
                    Ok(alerts)
                } else {
                    Ok(alerts)
                }
            })();
            presence.unlock()?;
            for alert in result? {
                send_alert(&alert);
            }
        } else {
            app = None;
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ClassType, Project, Task, TaskPriority, User, XPEvent};
    use chrono::{Timelike, Utc};
    use uuid::Uuid;

    fn app() -> (App, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("questline-desktop-{}.db", Uuid::new_v4()));
        let app = App::new_headless(&path).unwrap();
        app.db
            .insert_user(&User {
                id: Uuid::new_v4(),
                username: "Notification test".into(),
                class: ClassType::MindSage,
                level: 1,
                xp: 0,
                created_at: Utc::now(),
                specialization: None,
            })
            .unwrap();
        app.db
            .set_setting("external_notifications", "true")
            .unwrap();
        (app, path)
    }
    fn xp(app: &App, amount: i32) {
        app.db
            .insert_xp_event(&XPEvent {
                id: Uuid::new_v4(),
                event_type: "Complete Quest".into(),
                xp_gained: amount,
                timestamp: Utc::now(),
            })
            .unwrap();
    }
    fn hydration(app: &mut App) {
        for (key, value) in [
            ("hydration_enabled", "true"),
            ("hydration_interval_mins", "30"),
            ("hydration_active_from", "0"),
            ("hydration_active_to", "24"),
        ] {
            app.db.set_setting(key, value).unwrap();
        }
        app.hydration_interval_mins = 30;
        app.hydration_next_reminder_at = Some(Instant::now() - Duration::from_secs(1));
        app.last_sprite_check_time = Some(Instant::now());
    }
    #[test]
    fn reward_receipts_use_recorded_xp_and_never_replay_old_rewards() {
        let (app, path) = app();
        xp(&app, 500);
        let snapshot = RewardSnapshot::capture(&app.db).unwrap();
        xp(&app, 27);
        let response = snapshot
            .attach(&app.db, &json!({"action":"complete_task"}), json!({}))
            .unwrap();
        assert_eq!(response["reward"]["xp_gained"], 27);
        assert!(
            response["desktop_notifications"][0]["body"]
                .as_str()
                .unwrap()
                .contains("+27 XP")
        );
        let next = RewardSnapshot::capture(&app.db)
            .unwrap()
            .attach(&app.db, &json!({"action":"complete_task"}), json!({}))
            .unwrap();
        assert!(next["desktop_notifications"].as_array().unwrap().is_empty());
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn receipts_detect_level_up_and_only_new_achievements() {
        let (app, path) = app();
        let snapshot = RewardSnapshot::capture(&app.db).unwrap();
        let mut user = app.db.get_user().unwrap().unwrap();
        user.level = 2;
        user.xp = 3;
        app.db.update_user(&user).unwrap();
        let achievement = app.db.get_achievements().unwrap()[0].clone();
        app.db.unlock_achievement(&achievement.id).unwrap();
        xp(&app, 220);
        let response = snapshot
            .attach(&app.db, &json!({"action":"complete_task"}), json!({}))
            .unwrap();
        assert_eq!(response["reward"]["level_up"], true);
        assert_eq!(response["reward"]["achievements"][0]["id"], achievement.id);
        assert_eq!(
            response["desktop_notifications"].as_array().unwrap().len(),
            3
        );
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn autosave_stays_quiet_and_disabled_notifications_stay_disabled() {
        let (app, path) = app();
        let snapshot = RewardSnapshot::capture(&app.db).unwrap();
        xp(&app, 5);
        app.db
            .set_setting("external_notifications", "false")
            .unwrap();
        let response = snapshot
            .attach(
                &app.db,
                &json!({"action":"create_scroll","autosave":true}),
                json!({}),
            )
            .unwrap();
        assert_eq!(response["reward"]["xp_gained"], 5);
        assert_eq!(response["notifications_enabled"], false);
        assert!(
            response["desktop_notifications"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn hydration_fires_once_without_recording_a_drink_or_xp() {
        let (mut app, path) = app();
        hydration(&mut app);
        let before = RewardSnapshot::capture(&app.db).unwrap();
        let count = app.db.hydration_get_today().unwrap().0;
        let alerts = tick(&mut app, false).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["icon"], "hydration");
        assert!(tick(&mut app, false).unwrap().is_empty());
        assert_eq!(app.db.hydration_get_today().unwrap().0, count);
        assert_eq!(RewardSnapshot::capture(&app.db).unwrap().xp, before.xp);
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn hydration_respects_focus_active_hours_and_notification_toggle() {
        let (mut app, path) = app();
        hydration(&mut app);
        app.db.set_setting("hydration_pause_focus", "true").unwrap();
        assert!(tick(&mut app, true).unwrap().is_empty());
        app.db
            .set_setting("external_notifications", "false")
            .unwrap();
        assert!(tick(&mut app, false).unwrap().is_empty());
        app.db
            .set_setting("external_notifications", "true")
            .unwrap();
        hydration(&mut app);
        let hour = chrono::Local::now().hour();
        app.db
            .set_setting("hydration_active_from", &((hour + 1) % 24).to_string())
            .unwrap();
        app.db
            .set_setting("hydration_active_to", &((hour + 2) % 24).to_string())
            .unwrap();
        assert!(tick(&mut app, false).unwrap().is_empty());
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn swarm_obeys_cooldown_and_completed_chapter_without_granting_xp() {
        let (mut app, path) = app();
        app.sprite_eligible_checks = 5;
        app.last_sprite_check_time = None;
        app.last_sprite_notification_time = None;
        let before = RewardSnapshot::capture(&app.db).unwrap();
        let alerts = tick(&mut app, false).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["icon"], "swarm");
        assert!(tick(&mut app, false).unwrap().is_empty());
        app.db
            .set_setting(
                "chapter_progress_cache",
                r#"{"chapter_id":"chapter_1","completed":true,"objectives":[]}"#,
            )
            .unwrap();
        app.last_sprite_check_time = None;
        app.last_sprite_notification_time = None;
        app.sprite_eligible_checks = 5;
        assert!(tick(&mut app, false).unwrap().is_empty());
        assert_eq!(RewardSnapshot::capture(&app.db).unwrap().xp, before.xp);
        drop(app);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn tui_presence_lock_excludes_background_sender_and_releases_on_exit() {
        let folder = std::env::temp_dir().join(format!("questline-presence-{}", Uuid::new_v4()));
        std::fs::create_dir(&folder).unwrap();
        let tui = tui_presence(&folder).unwrap();
        let sender = lock_file(&folder.join("desktop-tui.lock")).unwrap();
        assert!(sender.try_lock().is_err());
        drop(tui);
        assert!(sender.try_lock().is_ok());
        drop(sender);
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn completed_shell_task_reports_native_reward_and_duplicate_completion_fails() {
        let (app, path) = app();
        let now = Utc::now();
        let project = Uuid::new_v4();
        let task = Uuid::new_v4();
        app.db
            .insert_project(&Project {
                id: project,
                name: "Receipt campaign".into(),
                description: None,
                created_at: now,
                updated_at: now,
                archived: false,
                completed: false,
                owner_identity: None,
                owner_username: None,
                is_shared: false,
            })
            .unwrap();
        app.db
            .insert_task(&Task {
                id: task,
                project_id: Some(project),
                title: "Defeat the backlog".into(),
                description: None,
                due_date: None,
                set_date: None,
                completed: false,
                priority: TaskPriority::Medium,
                created_at: now,
                updated_at: now,
                owner_identity: None,
                owner_username: None,
                parent_task_id: None,
                xp_awarded: false,
                recurrence: None,
            })
            .unwrap();
        let before = RewardSnapshot::capture(&app.db).unwrap();
        let request = json!({"action":"complete_task","id":task});
        let response = crate::shell_api::run(&path, &request).unwrap();
        let actual = RewardSnapshot::capture(&app.db).unwrap().xp - before.xp;
        assert!(actual > 0);
        assert_eq!(response["reward"]["xp_gained"], actual);
        assert!(
            response["desktop_notifications"][0]["body"]
                .as_str()
                .unwrap()
                .contains("Defeat the backlog")
        );
        assert!(crate::shell_api::run(&path, &request).is_err());
        assert_eq!(
            RewardSnapshot::capture(&app.db).unwrap().xp - before.xp,
            actual
        );
        drop(app);
        let _ = std::fs::remove_file(path);
    }
}
