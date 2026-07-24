// ─────────────────────────────────────────────────────────────────────────────
// services/xp.rs — calcula cuánto XP gana el héroe por cada acción, con bonos por clase
// ─────────────────────────────────────────────────────────────────────────────

use crate::database::Database;
use crate::models::{ClassType, User, XPEvent};
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

// Servicio central de progresión — aquí viven los level-ups y el registro de eventos XP
pub struct XPService<'a> {
    db: &'a Database,
}

impl<'a> XPService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // Otorga XP al usuario, aplica bonos de especialización y clase, y detecta level-ups
    // Returns `true` if a level up occurred — el caller decide si mostrar la pantalla de festejo
    pub fn grant_xp(&self, user: &mut User, event_type: &str, xp_gained: i32) -> Result<bool> {
        let mut final_xp = xp_gained;

        // Si el héroe tiene especialización, revisa si el tipo de evento activa el bono del 10%
        if let Some(ref spec) = user.specialization {
            let is_matched = match spec.as_str() {
                // Especializaciones enfocadas en tareas — bonus por completar tasks o eventos Hero
                "Bug Hunter" | "Execution Knight" | "Insight Seeker" | "Process Optimizer"
                | "Temporal Ward" | "Audit Judge" => {
                    event_type.contains("Task") || event_type.contains("Hero")
                }
                // Especializaciones enfocadas en notas — bonus por crear scrolls o notes
                "Automation Mage" | "Momentum Crusader" | "Knowledge Keeper"
                | "Modular Designer" | "History Weaver" | "Ledger Overseer" => {
                    event_type.contains("Note") || event_type.contains("Scroll")
                }
                // Especializaciones de proyectos — bonus por todo lo relacionado con Projects
                "System Weaver"
                | "Guardian of Order"
                | "Cognitive Cartographer"
                | "Infrastructure Builder"
                | "Timeline Editor"
                | "Asset Growth Specialist" => event_type.contains("Project"),
                _ => false,
            };
            // Aplica el multiplicador de especialización — +10% si el evento hace match
            if is_matched {
                final_xp = (final_xp as f64 * 1.10).round() as i32;
            }
        }

        // Arch Accountant: +2 XP flat + 5% on all base (non-passive) events — no manches, qué bono más chido
        if user.class == ClassType::ArchAccountant && !event_type.starts_with("Passive:") {
            final_xp = (final_xp as f64 * 1.05).round() as i32 + 2;
        }

        let quest_bonus = self.class_quest_bonus_percent(user.class, event_type)?;
        if quest_bonus > 0 {
            final_xp = (final_xp as f64 * (1.0 + quest_bonus as f64 / 100.0)).round() as i32;
        }

        // Persiste el evento en la DB para que el historial de XP quede registrado
        let event = XPEvent {
            id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            xp_gained: final_xp,
            timestamp: Utc::now(),
        };
        self.db.insert_xp_event(&event)?;

        user.xp += final_xp;

        // Loop de level-up — puede subir múltiples niveles de un jalón si el XP es grande
        let mut leveled_up = false;
        loop {
            if user.level >= 100 {
                // Nivel 100 es el tope — después de ahí el XP no cuenta para nada
                break;
            }
            let needed = User::xp_for_next_level(user.level);
            if user.xp >= needed {
                user.xp -= needed;
                user.level += 1;
                leveled_up = true;
            } else {
                break;
            }
        }

        self.db.update_user(user)?;

        Ok(leveled_up)
    }

    fn class_quest_bonus_percent(&self, class: ClassType, event_type: &str) -> Result<i32> {
        if event_type.starts_with("Passive:") || event_type.contains("Class Quest") {
            return Ok(0);
        }

        let completed = |level: i32| -> Result<bool> {
            let key = format!("class_quest_completed:{}:{}", class_slug(class), level);
            Ok(self.db.get_setting(&key)?.as_deref() == Some("true"))
        };

        let is_task = event_type.contains("Task") || event_type.contains("Quest") || event_type.contains("Hero");
        let is_daily = event_type.contains("Daily Quest") || event_type.contains("Quest Chain");
        let is_focus = event_type.contains("Focus");
        let is_note = event_type.contains("Note") || event_type.contains("Scroll");
        let is_journal = event_type.contains("Journal");
        let is_sidequest = event_type.contains("Sidequest") || event_type.contains("Ritual");
        let is_milestone = event_type.contains("Milestone");
        let is_project = event_type.contains("Project") || event_type.contains("Campaign");
        let is_sync = event_type.contains("Sync");
        let is_memory = event_type.contains("Memory") || event_type.contains("Fragment");

        let mut percent = 0;
        if completed(10)? && is_task {
            percent += 5;
        }
        if completed(20)? && is_daily {
            percent += 5;
        }
        if completed(30)? && is_focus {
            percent += 5;
        }
        if completed(60)? && is_journal {
            percent += 5;
        }
        if completed(70)? && is_sidequest {
            percent += 5;
        }
        if completed(80)? && is_milestone {
            percent += 5;
        }
        if completed(90)? && is_project {
            percent += 10;
        }
        if completed(100)? {
            percent += 10;
        }

        if completed(40)? {
            let class_match = match class {
                ClassType::CodeWarlock => is_note || is_sync,
                ClassType::TaskPaladin => is_task,
                ClassType::MindSage => is_note || is_journal || is_memory,
                ClassType::SystemsArchitect => is_project || is_milestone,
                ClassType::TimeChronomancer => is_focus,
                ClassType::ArchAccountant => true,
            };
            if class_match {
                percent += 5;
            }
        }

        Ok(percent)
    }
}

fn class_slug(class: ClassType) -> &'static str {
    match class {
        ClassType::CodeWarlock => "code_warlock",
        ClassType::TaskPaladin => "task_paladin",
        ClassType::MindSage => "mind_sage",
        ClassType::SystemsArchitect => "systems_architect",
        ClassType::TimeChronomancer => "time_chronomancer",
        ClassType::ArchAccountant => "arch_accountant",
    }
}
