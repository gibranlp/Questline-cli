// ─────────────────────────────────────────────────────────────────────────────
// services/notifications.rs — despacha notificaciones al sistema operativo
// ─────────────────────────────────────────────────────────────────────────────

// Lanza una notificación del sistema nativa sin bloquear el hilo principal.
// urgent=true → critical/always-on-top en el daemon del OS (dunst, WinRT).
// Falla silenciosamente si el tool no está disponible.
pub fn send_system_notification(title: &str, message: &str, urgent: bool) {
    let title = strip_non_ascii(title);
    let message = strip_non_ascii(message);

    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            let urgency = if urgent { "critical" } else { "normal" };
            let _ = std::process::Command::new("notify-send")
                .arg("--app-name=Questline")
                .arg(format!("--urgency={}", urgency))
                .arg(&title)
                .arg(&message)
                .status();
        }

        #[cfg(target_os = "macos")]
        {
            // osascript no tiene niveles de urgencia; el subtítulo diferencia visualmente
            let subtitle = if urgent { "Questline - Urgent" } else { "Questline" };
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

        #[cfg(target_os = "windows")]
        {
            // WinRT toast — scenario="urgent" mantiene el toast hasta que el usuario lo descarte (Win11)
            let scenario_line = if urgent {
                "$x.DocumentElement.SetAttribute('scenario','urgent');"
            } else {
                ""
            };
            let ps = format!(
                concat!(
                    "[Windows.UI.Notifications.ToastNotificationManager,",
                    "Windows.UI.Notifications,ContentType=WindowsRuntime]|Out-Null;",
                    "$x=[Windows.UI.Notifications.ToastNotificationManager]",
                    "::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02);",
                    "{scenario}",
                    "$x.GetElementsByTagName('text')[0].InnerText={title};",
                    "$x.GetElementsByTagName('text')[1].InnerText={message};",
                    "[Windows.UI.Notifications.ToastNotificationManager]",
                    "::CreateToastNotifier('Questline')",
                    ".Show([Windows.UI.Notifications.ToastNotification]::new($x))"
                ),
                scenario = scenario_line,
                title = powershell_quote(&title),
                message = powershell_quote(&message),
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
                .status();
        }
    });
}

fn strip_non_ascii(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).collect()
}

#[cfg(target_os = "macos")]
fn applescript_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
fn powershell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
