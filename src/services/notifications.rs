// ─────────────────────────────────────────────────────────────────────────────
// services/notifications.rs — despacha notificaciones al sistema operativo
// ─────────────────────────────────────────────────────────────────────────────

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationIcon {
    Tasks,
    TaskDue,
    TaskOverdue,
    TaskCompleted,
    TaskRecurring,
    TaskHighPriority,
    TaskDailySummary,
    TaskIdle,
    Fellowship,
    NotificationSwarm,
    LevelUp,
    Achievement,
    Hydration,
    Sync,
    JournalNotes,
    Rewards,
    DailyAdventure,
    Evergrowth,
    Info,
    Warning,
    Focus,
}

impl NotificationIcon {
    fn file_name(self) -> &'static str {
        match self {
            Self::Tasks => "tasks.png",
            Self::TaskDue => "task_due.png",
            Self::TaskOverdue => "task_overdue.png",
            Self::TaskCompleted => "task_completed.png",
            Self::TaskRecurring => "task_recurring.png",
            Self::TaskHighPriority => "task_high_priority.png",
            Self::TaskDailySummary => "task_daily_summary.png",
            Self::TaskIdle => "task_idle.png",
            Self::Fellowship => "fellowship.png",
            Self::NotificationSwarm => "notification_swarm.png",
            Self::LevelUp => "level_up.png",
            Self::Achievement => "achievement.png",
            Self::Hydration => "hydration.png",
            Self::Sync => "sync.png",
            Self::JournalNotes => "journal_notes.png",
            Self::Rewards => "rewards.png",
            Self::DailyAdventure => "daily_adventure.png",
            Self::Evergrowth => "evergrowth.png",
            Self::Info => "info.png",
            Self::Warning => "warning.png",
            Self::Focus => "focus.png",
        }
    }
}

// Lanza una notificación nativa con el icono informativo predeterminado.
pub fn send_system_notification(title: &str, message: &str, urgent: bool) {
    send_system_notification_with_icon(title, message, urgent, NotificationIcon::Info);
}

// Despacha una notificación del sistema sin bloquear y adjunta un icono absoluto cuando la plataforma lo permite.
pub fn send_system_notification_with_icon(
    title: &str,
    message: &str,
    urgent: bool,
    icon: NotificationIcon,
) {
    let title = strip_non_ascii(title);
    let message = strip_non_ascii(message);
    let icon_path = notification_icon_path(icon);

    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            send_linux_notification(&title, &message, urgent, icon_path.as_ref());
        }

        #[cfg(target_os = "macos")]
        {
            // AppleScript no permite elegir iconos por notificación; terminal-notifier sí cuando está instalado.
            let subtitle = if urgent {
                "Questline - Urgent"
            } else {
                "Questline"
            };
            if !send_with_terminal_notifier(&title, &message, subtitle, icon_path.as_ref()) {
                let script = format!(
                    "display notification {} with title {} subtitle {}",
                    applescript_quote(&message),
                    applescript_quote(&title),
                    applescript_quote(subtitle),
                );
                let _ = std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .status();
            }
        }

        #[cfg(target_os = "windows")]
        {
            // WinRT ToastImageAndText02 muestra un icono local cuando Windows acepta la ruta del archivo.
            let scenario_line = if urgent {
                "$x.DocumentElement.SetAttribute('scenario','urgent');"
            } else {
                ""
            };
            let template = if icon_path.is_some() {
                "ToastImageAndText02"
            } else {
                "ToastText02"
            };
            let image_line = icon_path
                .as_ref()
                .map(|path| {
                    format!(
                        "$x.GetElementsByTagName('image')[0].SetAttribute('src',{});",
                        powershell_quote(&windows_file_uri(path))
                    )
                })
                .unwrap_or_default();
            let ps = format!(
                concat!(
                    "[Windows.UI.Notifications.ToastNotificationManager,",
                    "Windows.UI.Notifications,ContentType=WindowsRuntime]|Out-Null;",
                    "$x=[Windows.UI.Notifications.ToastNotificationManager]",
                    "::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::{template});",
                    "{scenario}",
                    "{image}",
                    "$x.GetElementsByTagName('text')[0].InnerText={title};",
                    "$x.GetElementsByTagName('text')[1].InnerText={message};",
                    "[Windows.UI.Notifications.ToastNotificationManager]",
                    "::CreateToastNotifier('Questline')",
                    ".Show([Windows.UI.Notifications.ToastNotification]::new($x))"
                ),
                template = template,
                scenario = scenario_line,
                image = image_line,
                title = powershell_quote(&title),
                message = powershell_quote(&message),
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .status();
        }
    });
}

#[cfg(target_os = "macos")]
fn send_with_terminal_notifier(
    title: &str,
    message: &str,
    subtitle: &str,
    icon_path: Option<&PathBuf>,
) -> bool {
    for command in terminal_notifier_candidates() {
        let mut cmd = std::process::Command::new(&command);
        cmd.arg("-title")
            .arg(title)
            .arg("-subtitle")
            .arg(subtitle)
            .arg("-message")
            .arg(message)
            .arg("-group")
            .arg("questline");
        if let Some(path) = icon_path {
            cmd.arg("-appIcon").arg(path);
        }
        let status = cmd.status();
        if status.map(|s| s.success()).unwrap_or(false) {
            return true;
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn terminal_notifier_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("terminal-notifier")];
    candidates.push(PathBuf::from("/opt/homebrew/bin/terminal-notifier"));
    candidates.push(PathBuf::from("/usr/local/bin/terminal-notifier"));
    candidates
}

#[cfg(target_os = "linux")]
fn send_linux_notification(title: &str, message: &str, urgent: bool, icon_path: Option<&PathBuf>) {
    // notify-send talks to the Freedesktop notification portal/server and works with
    // Dunst, Mako, SwayNC, GNOME Shell, KDE Plasma, Xfce, Cinnamon, and similar desktops.
    if send_with_notify_send(title, message, urgent, icon_path) {
        return;
    }

    // Dunst users may have dunstify even when notify-send/libnotify is not installed.
    if send_with_dunstify(title, message, urgent, icon_path) {
        return;
    }

    // gdbus is commonly present on GNOME/GTK systems and calls the same notification API directly.
    if send_with_gdbus(title, message, urgent, icon_path) {
        return;
    }

    // KDE and GTK fallbacks. These are less featureful but still give the user a visible alert.
    if send_with_kdialog(title, message) {
        return;
    }
    let _ = send_with_zenity(title, message, icon_path);
}

#[cfg(target_os = "linux")]
fn command_succeeded(cmd: &mut std::process::Command) -> bool {
    cmd.status().map(|status| status.success()).unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn send_with_notify_send(
    title: &str,
    message: &str,
    urgent: bool,
    icon_path: Option<&PathBuf>,
) -> bool {
    let urgency = if urgent { "critical" } else { "normal" };
    let mut cmd = std::process::Command::new("notify-send");
    cmd.arg("--app-name=Questline")
        .arg(format!("--urgency={}", urgency));
    if let Some(path) = icon_path {
        cmd.arg("--icon").arg(path);
    }
    command_succeeded(cmd.arg(title).arg(message))
}

#[cfg(target_os = "linux")]
fn send_with_dunstify(
    title: &str,
    message: &str,
    urgent: bool,
    icon_path: Option<&PathBuf>,
) -> bool {
    let urgency = if urgent { "critical" } else { "normal" };
    let mut cmd = std::process::Command::new("dunstify");
    cmd.arg("--appname")
        .arg("Questline")
        .arg("--urgency")
        .arg(urgency);
    if let Some(path) = icon_path {
        cmd.arg("--icon").arg(path);
    }
    command_succeeded(cmd.arg(title).arg(message))
}

#[cfg(target_os = "linux")]
fn send_with_gdbus(title: &str, message: &str, urgent: bool, icon_path: Option<&PathBuf>) -> bool {
    let hints = format!("{{'urgency': <{}>}}", if urgent { 2 } else { 1 });
    let timeout_ms = if urgent { "0" } else { "6000" };
    let icon = icon_path
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "questline".to_string());

    command_succeeded(
        std::process::Command::new("gdbus")
            .arg("call")
            .arg("--session")
            .arg("--dest")
            .arg("org.freedesktop.Notifications")
            .arg("--object-path")
            .arg("/org/freedesktop/Notifications")
            .arg("--method")
            .arg("org.freedesktop.Notifications.Notify")
            .arg("Questline")
            .arg("0")
            .arg(icon)
            .arg(title)
            .arg(message)
            .arg("[]")
            .arg(hints)
            .arg(timeout_ms),
    )
}

#[cfg(target_os = "linux")]
fn send_with_kdialog(title: &str, message: &str) -> bool {
    command_succeeded(
        std::process::Command::new("kdialog")
            .arg("--title")
            .arg(title)
            .arg("--passivepopup")
            .arg(message)
            .arg("6"),
    )
}

#[cfg(target_os = "linux")]
fn send_with_zenity(title: &str, message: &str, icon_path: Option<&PathBuf>) -> bool {
    let mut cmd = std::process::Command::new("zenity");
    cmd.arg("--notification")
        .arg("--title")
        .arg(title)
        .arg("--text")
        .arg(message);
    if let Some(path) = icon_path {
        cmd.arg("--window-icon").arg(path);
    }
    command_succeeded(&mut cmd)
}

fn notification_icon_path(icon: NotificationIcon) -> Option<PathBuf> {
    let relative = PathBuf::from("assets")
        .join("icons")
        .join("notifications")
        .join(icon.file_name());

    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(&relative));
            candidates.push(dir.join("..").join(&relative));
            candidates.push(
                dir.join("..")
                    .join("share")
                    .join("questline")
                    .join(&relative),
            );
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(&relative));
    }

    candidates.into_iter().find(|path| path.exists())
}

fn strip_non_ascii(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii() && !c.is_ascii_control())
        .collect()
}

#[cfg(target_os = "macos")]
fn applescript_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
fn powershell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[cfg(target_os = "windows")]
fn windows_file_uri(path: &std::path::Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.starts_with("//") {
        format!("file:{}", normalized)
    } else {
        format!("file:///{}", normalized)
    }
}
