// ─────────────────────────────────────────────────────────────────────────────
// services/planner.rs — motor de planificación determinista del dashboard
// ─────────────────────────────────────────────────────────────────────────────

use chrono::{Duration, NaiveDate};

use crate::models::{Project, Task, TaskPriority};

// Resultado de puntuar una tarea: el puntaje, la razón legible y datos de progreso
pub struct ScoredTask {
    pub task: Task,
    pub reason: &'static str,
    pub project_name: String,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub est_minutes: u32,
}

// Resultado completo del motor de planificación para el dashboard
pub struct DashboardPlan {
    pub main_quest: Option<ScoredTask>,
    pub next_quest: Option<ScoredTask>,
    pub quick_wins: Vec<Task>,
    pub total_quest_count: usize,
    pub estimated_minutes: u32,
    pub guidance: &'static str,
}

// Estima la duración en minutos basándose en la cantidad de pasos (proxy de complejidad)
pub fn estimate_minutes(pending_steps: usize) -> u32 {
    match pending_steps {
        0 => 15,
        1..=2 => 30,
        3..=5 => 60,
        _ => 120,
    }
}

pub fn format_duration(minutes: u32) -> String {
    if minutes < 60 {
        format!("~{} min", minutes)
    } else if minutes == 60 {
        "~1 hr".to_string()
    } else {
        format!("~{:.1} hr", minutes as f32 / 60.0)
    }
}

// Asigna un puntaje de urgencia a la tarea según fecha límite y prioridad
fn score_task(task: &Task, today: NaiveDate) -> (i32, &'static str) {
    let mut score = 0i32;
    let mut reason = "Part of today's campaign.";

    if let Some(due) = task.due_date {
        let due_naive = due.date_naive();
        if due_naive < today {
            score += 100;
            reason = "Overdue. Resolve before it costs more.";
        } else if due_naive == today {
            score += 60;
            reason = "Due today. Complete before the day ends.";
        } else if due_naive == today + Duration::days(1) {
            score += 40;
            reason = "Due tomorrow. Act now to avoid the rush.";
        } else {
            let days = (due_naive - today).num_days();
            if days <= 3 {
                score += 25;
                reason = "Due within three days.";
            } else if days <= 7 {
                score += 10;
                reason = "Due this week.";
            }
        }
    }

    match task.priority {
        TaskPriority::High => {
            score += 30;
            if reason == "Part of today's campaign." {
                reason = "High priority. The realm demands action.";
            }
        }
        TaskPriority::Medium => {
            score += 10;
        }
        TaskPriority::Low => {}
    }

    (score, reason)
}

// Genera el plan del día: quest principal, siguiente quest, victorias rápidas y carga total
pub fn generate_plan(
    all_tasks: &[Task],
    projects: &[Project],
    today: NaiveDate,
    overdue_count: usize,
    streak: i32,
    tree_health: i32,
    daily_completed: usize,
    daily_total: usize,
) -> DashboardPlan {
    let get_project_name = |project_id: Option<uuid::Uuid>| -> String {
        project_id
            .and_then(|pid| projects.iter().find(|p| p.id == pid))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "General".to_string())
    };

    let parent_tasks: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| !t.completed && t.parent_task_id.is_none())
        .collect();

    let total_quest_count = parent_tasks.len();

    let mut scored: Vec<(i32, &'static str, &Task)> = parent_tasks
        .iter()
        .map(|t| {
            let (s, r) = score_task(t, today);
            (s, r, *t)
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let build_scored = |task: &Task, reason: &'static str| -> ScoredTask {
        let total_steps = all_tasks
            .iter()
            .filter(|t| t.parent_task_id == Some(task.id))
            .count();
        let completed_steps = all_tasks
            .iter()
            .filter(|t| t.parent_task_id == Some(task.id) && t.completed)
            .count();
        let pending_steps = total_steps - completed_steps;
        ScoredTask {
            task: task.clone(),
            reason,
            project_name: get_project_name(task.project_id),
            total_steps,
            completed_steps,
            est_minutes: estimate_minutes(pending_steps),
        }
    };

    let main_id = scored.first().map(|(_, _, t)| t.id);
    let next_id = scored.get(1).map(|(_, _, t)| t.id);

    let main_quest = scored.first().map(|(_, r, t)| build_scored(t, r));
    let next_quest = scored.get(1).map(|(_, r, t)| build_scored(t, r));

    // Victorias rápidas: tareas sin pasos pendientes, excluyendo las ya mostradas arriba
    let quick_wins: Vec<Task> = parent_tasks
        .iter()
        .filter(|t| {
            let has_pending_steps = all_tasks
                .iter()
                .any(|s| s.parent_task_id == Some(t.id) && !s.completed);
            !has_pending_steps
                && Some(t.id) != main_id
                && Some(t.id) != next_id
        })
        .take(5)
        .map(|t| (*t).clone())
        .collect();

    let estimated_minutes: u32 = parent_tasks
        .iter()
        .map(|t| {
            let pending = all_tasks
                .iter()
                .filter(|s| s.parent_task_id == Some(t.id) && !s.completed)
                .count();
            estimate_minutes(pending)
        })
        .sum();

    let guidance = choose_guidance(
        overdue_count,
        streak,
        tree_health,
        daily_completed,
        daily_total,
        total_quest_count,
    );

    DashboardPlan {
        main_quest,
        next_quest,
        quick_wins,
        total_quest_count,
        estimated_minutes,
        guidance,
    }
}

// Devuelve la tarea con mayor puntaje sin construir el plan completo — útil para atajos de teclado
pub fn find_main_quest(all_tasks: &[Task], today: NaiveDate) -> Option<Task> {
    let mut candidates: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| !t.completed && t.parent_task_id.is_none())
        .collect();
    candidates.sort_by(|a, b| {
        let (sa, _) = score_task(a, today);
        let (sb, _) = score_task(b, today);
        sb.cmp(&sa)
    });
    candidates.first().map(|t| (*t).clone())
}

fn choose_guidance(
    overdue: usize,
    streak: i32,
    tree_health: i32,
    daily_completed: usize,
    daily_total: usize,
    total_quests: usize,
) -> &'static str {
    if overdue > 1 {
        "Multiple overdue quests cast a shadow over the realm. Clear them first."
    } else if overdue == 1 {
        "One overdue quest threatens today's march. Resolve it before advancing."
    } else if tree_health < 40 {
        "The Evergrowth weakens. Complete quests to restore its vitality."
    } else if daily_total > 0 && daily_completed == daily_total {
        "All daily quests sealed. The realm grows stronger with every completed campaign."
    } else if streak >= 30 {
        "Thirty days of unbroken dedication. The Chronicle watches. Do not let the chain fall."
    } else if streak >= 7 {
        "The Chronicle records steady resolve. Continue the march."
    } else if total_quests == 0 {
        "The quest board stands empty. Create a campaign to begin the adventure."
    } else {
        "The Council recommends beginning with the highest-ranked quest. The realm awaits."
    }
}
