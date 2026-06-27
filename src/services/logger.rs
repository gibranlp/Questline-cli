// ─────────────────────────────────────────────────────────────────────────────
// services/logger.rs — logging estructurado a archivo JSON, con hook para panics
// ─────────────────────────────────────────────────────────────────────────────

use chrono::Utc;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Serialize)]
struct LogRecord {
    timestamp: String,
    level: String,
    component: String,
    message: String,
    details: Option<String>,
}

// Escribe una línea JSON al log — silencioso si falla para no matar el app por un log
pub fn log_structured(level: &str, component: &str, message: &str, details: Option<&str>) {
    if let Ok(storage_dir) = crate::storage::ensure_storage_dir_exists() {
        let log_path = storage_dir.join("questline.log");
        let record = LogRecord {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            component: component.to_string(),
            message: message.to_string(),
            details: details.map(|d| d.to_string()),
        };
        if let Ok(json_str) = serde_json::to_string(&record) {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = writeln!(file, "{}", json_str);
            }
        }
    }
}

// Órale, este hook atrapa los panics antes de que el proceso muera y los guarda en el log.
// También escribe un recovery_report.json para que el usuario sepa qué pasó.
pub fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic payload"
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "Unknown location".to_string());

        let details = format!("Location: {}", location);

        log_structured("CRITICAL", "crash_diagnostics", message, Some(&details));
        write_recovery_report(message, &location);

        // Imprime el crash en stderr para que el usuario vea qué pasó
        eprintln!(
            "\n================================================================================"
        );
        eprintln!("Questline has encountered a critical error (panic) and crashed.");
        eprintln!("Panic: {} at {}", message, location);
        eprintln!("Crash diagnostics logged to questline.log and a recovery report was generated.");
        eprintln!(
            "================================================================================\n"
        );
    }));
}

// Escribe el reporte de crash en JSON — chido tenerlo para debuggear después
fn write_recovery_report(panic_msg: &str, location: &str) {
    if let Ok(storage_dir) = crate::storage::ensure_storage_dir_exists() {
        let report_path = storage_dir.join("recovery_report.json");
        let report = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "status": "CRASHED",
            "error": panic_msg,
            "location": location,
            "recovery_action": "Verify database integrity and restore from backup if necessary.",
            "diagnostics": {
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "version": "1.0.1"
            }
        });
        if let Ok(json_str) = serde_json::to_string_pretty(&report) {
            let _ = std::fs::write(report_path, json_str);
        }
    }
}
