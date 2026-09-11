// ─────────────────────────────────────────────────────────────────────────────
// services/planner.rs — motor de planificación determinista del dashboard
// ─────────────────────────────────────────────────────────────────────────────

use chrono::NaiveDate;
use std::collections::HashSet;

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

// Asigna un puntaje de urgencia a la tarea según fecha límite y prioridad.
//
// El due date se pondera en tramos de 15 puntos (0, 15, 30, 45, 60, 75, 90, 105) y
// la prioridad aporta como máximo 14 puntos (High). Como 14 < 15, la prioridad nunca
// puede saltar un tramo de fecha completo: una tarea High priority sin fecha (o con
// vencimiento a más de 30 días) jamás superará a una tarea con vencimiento real más
// cercano, sin importar su prioridad. El tiempo manda; la prioridad solo desempata
// dentro de un mismo horizonte de urgencia.
fn score_task(task: &Task, today: NaiveDate) -> (i32, &'static str) {
    let mut score = 0i32;
    let mut reason = "No due date. Ranked by priority alone.";

    if let Some(due) = task.due_date {
        let due_naive = due.date_naive();
        let days = (due_naive - today).num_days();

        if days < 0 {
            score += 105;
            reason = "Overdue. Resolve before it costs more.";
        } else if days == 0 {
            score += 90;
            reason = "Due today. Complete before the day ends.";
        } else if days == 1 {
            score += 75;
            reason = "Due tomorrow. Act now to avoid the rush.";
        } else if days <= 3 {
            score += 60;
            reason = "Due within three days.";
        } else if days <= 7 {
            score += 45;
            reason = "Due this week.";
        } else if days <= 14 {
            score += 30;
            reason = "Due within two weeks.";
        } else if days <= 30 {
            score += 15;
            reason = "Due within the month.";
        } else {
            reason = "Due far in the future. Not urgent yet.";
        }
    }

    match task.priority {
        TaskPriority::High => {
            score += 14;
            // Only override the reason when there's truly no due date to explain
            // the ranking — a far-future due date also leaves score at 14 here,
            // but should keep its own "not urgent yet" reason, not this one.
            if task.due_date.is_none() {
                reason = "High priority. The realm demands action.";
            }
        }
        TaskPriority::Medium => {
            score += 5;
        }
        TaskPriority::Low => {}
    }

    (score, reason)
}

// Genera el plan del día: quest principal, siguiente quest, victorias rápidas y carga total.
// horizon_days viene del Oath Calendar ("Show quests up to") — None significa "All" (sin límite).
pub fn generate_plan(
    all_tasks: &[Task],
    projects: &[Project],
    today: NaiveDate,
    overdue_count: usize,
    streak: i32,
    tree_health: i32,
    daily_completed: usize,
    daily_total: usize,
    horizon_days: Option<i64>,
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
    let parents_with_open_steps: HashSet<uuid::Uuid> = all_tasks
        .iter()
        .filter(|s| !s.completed)
        .filter_map(|s| s.parent_task_id)
        .collect();

    let total_quest_count = parent_tasks.len();

    // El héroe se agobia si "lo que hay que hacer ahora" incluye pendientes de dentro de
    // un mes. Main Quest, Next Quest y Quick Wins solo compiten entre tareas sin fecha o
    // que vencen dentro del horizonte configurado en el Oath Calendar — el resto vive en
    // el backlog, no en la primera línea del Command Center.
    let within_planning_horizon = |t: &Task| -> bool {
        match horizon_days {
            None => true,
            Some(limit) => t
                .due_date
                .map(|due| (due.date_naive() - today).num_days() <= limit)
                .unwrap_or(true),
        }
    };

    let mut scored: Vec<(i32, &'static str, &Task)> = parent_tasks
        .iter()
        .filter(|t| within_planning_horizon(t))
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

    // Quick Wins: tasks without pending steps, inside the planning horizon, excluding Main/Next.
    // Sort before taking five so the Command Center always shows the nearest deadlines instead of
    // whichever rows SQLite happened to return first. Undated work follows all dated work.
    let mut quick_win_candidates: Vec<&Task> = parent_tasks
        .iter()
        .filter(|t| {
            within_planning_horizon(t)
                && !parents_with_open_steps.contains(&t.id)
                && Some(t.id) != main_id
                && Some(t.id) != next_id
        })
        .copied()
        .collect();
    quick_win_candidates.sort_by(|a, b| {
        (a.due_date.is_none(), a.due_date)
            .cmp(&(b.due_date.is_none(), b.due_date))
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| a.created_at.cmp(&b.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    let quick_wins: Vec<Task> = quick_win_candidates
        .into_iter()
        .take(5)
        .cloned()
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

// Devuelve la tarea con mayor puntaje sin construir el plan completo — útil para atajos de teclado.
// Respeta el mismo horizon_days que generate_plan, o el atajo [o] saltaría a una tarea distinta
// de la que el Command Center realmente muestra como Main Quest.
pub fn find_main_quest(
    all_tasks: &[Task],
    today: NaiveDate,
    horizon_days: Option<i64>,
) -> Option<Task> {
    let mut candidates: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| {
            !t.completed
                && t.parent_task_id.is_none()
                && match horizon_days {
                    None => true,
                    Some(limit) => t
                        .due_date
                        .map(|due| (due.date_naive() - today).num_days() <= limit)
                        .unwrap_or(true),
                }
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_task(title: &str, priority: TaskPriority, due_in_days: Option<i64>) -> Task {
        let now = Utc::now();
        Task {
            id: uuid::Uuid::new_v4(),
            project_id: None,
            title: title.to_string(),
            description: None,
            due_date: due_in_days.map(|days| {
                Utc.from_utc_datetime(
                    &(chrono::Local::now().date_naive() + chrono::Duration::days(days))
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                )
            }),
            set_date: None,
            completed: false,
            priority,
            created_at: now,
            updated_at: now,
            owner_identity: None,
            owner_username: None,
            parent_task_id: None,
            xp_awarded: false,
            recurrence: None,
        }
    }

    // No usamos Utc::now() como referencia porque due_in_days ancla al día local de "ahora" —
    // así que "today" para la prueba tiene que ser exactamente ese mismo día local.
    fn today() -> NaiveDate {
        chrono::Local::now().date_naive()
    }

    #[test]
    fn a_task_due_within_two_weeks_can_become_main_quest() {
        let tasks = vec![make_task("Due in 14 days", TaskPriority::Low, Some(14))];
        let plan = generate_plan(&tasks, &[], today(), 0, 0, 100, 0, 0, Some(14));
        assert_eq!(
            plan.main_quest.map(|s| s.task.title),
            Some("Due in 14 days".to_string()),
            "a task due exactly 14 days out is still within the planning horizon"
        );
    }

    #[test]
    fn a_task_due_beyond_two_weeks_never_becomes_main_next_or_a_quick_win() {
        // Alta prioridad y sin competencia — si el horizonte no filtrara, ganaría fácil.
        let tasks = vec![make_task("Due in 15 days", TaskPriority::High, Some(15))];
        let plan = generate_plan(&tasks, &[], today(), 0, 0, 100, 0, 0, Some(14));
        assert!(
            plan.main_quest.is_none(),
            "a task 15 days out must not be crowned Main Quest"
        );
        assert!(plan.next_quest.is_none());
        assert!(
            plan.quick_wins.is_empty(),
            "the same task must not appear as a Quick Win either"
        );
    }

    #[test]
    fn horizon_days_none_means_all_and_disables_the_filter() {
        // "All" en el Oath Calendar: la tarea de dentro de un año vuelve a competir normal.
        let tasks = vec![make_task("Due in a year", TaskPriority::Low, Some(365))];
        let plan = generate_plan(&tasks, &[], today(), 0, 0, 100, 0, 0, None);
        assert_eq!(
            plan.main_quest.map(|s| s.task.title),
            Some("Due in a year".to_string()),
            "horizon_days: None must lift the planning-horizon filter entirely"
        );
    }

    #[test]
    fn an_undated_task_is_not_treated_as_far_away() {
        let tasks = vec![make_task("Someday, no rush", TaskPriority::Low, None)];
        let plan = generate_plan(&tasks, &[], today(), 0, 0, 100, 0, 0, Some(14));
        assert_eq!(
            plan.main_quest.map(|s| s.task.title),
            Some("Someday, no rush".to_string()),
            "an undated task competes normally, unlike one due a month out"
        );
    }

    #[test]
    fn an_overdue_task_always_outranks_the_horizon_filter() {
        let tasks = vec![
            make_task("Overdue", TaskPriority::Low, Some(-3)),
            make_task("Far away but high priority", TaskPriority::High, Some(45)),
        ];
        let plan = generate_plan(&tasks, &[], today(), 1, 0, 100, 0, 0, Some(14));
        assert_eq!(plan.main_quest.map(|s| s.task.title), Some("Overdue".to_string()));
    }

    #[test]
    fn find_main_quest_agrees_with_generate_plan() {
        let tasks = vec![
            make_task("Due in 20 days, high priority", TaskPriority::High, Some(20)),
            make_task("Due in 5 days, low priority", TaskPriority::Low, Some(5)),
        ];
        let plan = generate_plan(&tasks, &[], today(), 0, 0, 100, 0, 0, Some(14));
        let shortcut = find_main_quest(&tasks, today(), Some(14));
        assert_eq!(
            plan.main_quest.map(|s| s.task.id),
            shortcut.map(|t| t.id),
            "the [o] shortcut must jump to the same task the dashboard displays as Main Quest"
        );
    }

    #[test]
    fn quick_wins_are_sorted_by_due_date_with_undated_tasks_last() {
        let tasks = vec![
            make_task("Main", TaskPriority::Medium, Some(-2)),
            make_task("Next", TaskPriority::Medium, Some(-1)),
            make_task("Due in ten days", TaskPriority::Medium, Some(10)),
            make_task("Undated", TaskPriority::High, None),
            make_task("Due in thirteen days", TaskPriority::Medium, Some(13)),
            make_task("Due in eleven days", TaskPriority::Medium, Some(11)),
        ];

        let plan = generate_plan(&tasks, &[], today(), 2, 0, 100, 0, 0, Some(14));
        let titles: Vec<&str> = plan.quick_wins.iter().map(|task| task.title.as_str()).collect();

        assert_eq!(
            titles,
            vec![
                "Due in ten days",
                "Due in eleven days",
                "Due in thirteen days",
                "Undated",
            ]
        );
    }
}
