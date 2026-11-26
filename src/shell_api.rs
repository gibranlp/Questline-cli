use anyhow::{Context, Result, bail};
use chrono::{Local, NaiveDate, TimeZone, Utc};
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;

use crate::app::App;
use crate::models::{
    JournalEntry, LedgerEntry, LedgerEntryType, LedgerRecurrence, LedgerStatus, Note, RecurrenceType, Task, TaskPriority,
};
use crate::services::{TreasuryService, XPService};

fn field<'a>(request: &'a Value, name: &str) -> Result<&'a str> {
    request[name]
        .as_str()
        .with_context(|| format!("Missing {name}"))
}

fn task_recurrence(request: &Value) -> Result<Option<RecurrenceType>> {
    match request.get("recurrence") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value == "None" || value.is_empty() => Ok(None),
        Some(Value::String(value)) => RecurrenceType::from_str(value)
            .map(Some)
            .context("Choose a valid task recurrence"),
        _ => bail!("Choose a valid task recurrence"),
    }
}

fn text_field(request: &Value, name: &str, maximum: usize) -> Result<String> {
    let value = field(request, name)?.trim();
    if value.is_empty() || value.chars().count() > maximum {
        bail!("{name} must contain 1 to {maximum} characters");
    }
    Ok(value.to_string())
}

fn id_field(request: &Value, name: &str) -> Result<Uuid> {
    Ok(Uuid::parse_str(field(request, name)?)?)
}

fn due_date(request: &Value) -> Result<Option<chrono::DateTime<Utc>>> {
    date_field(request, "due_date")
}

fn date_field(request: &Value, name: &str) -> Result<Option<chrono::DateTime<Utc>>> {
    let value = request[name].as_str().unwrap_or("").trim();
    if value.is_empty() {
        return Ok(None);
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;
    let local = Local
        .from_local_datetime(&date.and_hms_opt(12, 0, 0).context("Invalid due date")?)
        .single()
        .context("Invalid local due date")?;
    Ok(Some(local.with_timezone(&Utc)))
}

fn optional_id(request: &Value, name: &str) -> Result<Option<Uuid>> {
    match request[name].as_str() {
        Some(value) if !value.is_empty() => Ok(Some(Uuid::parse_str(value)?)),
        _ if request[name].is_null() || request[name].as_str() == Some("") => Ok(None),
        _ => bail!("Invalid {name}"),
    }
}

fn apply_entry_fields(request: &Value, entry: &mut LedgerEntry, treasury: &TreasuryService) -> Result<()> {
    let desired = LedgerEntryType::parse(request["entry_type"].as_str().unwrap_or(entry.entry_type.as_str()))
        .context("Choose a valid transaction type")?;
    let changed_type = desired != entry.entry_type;
    entry.entry_type = desired;
    if request.get("category_id").is_some() || changed_type || entry.category_id.is_nil() {
        entry.category_id = match optional_id(request, "category_id")? {
            Some(id) => id,
            None => {
                let preferred = match desired {
                    LedgerEntryType::Income => "Other Income",
                    LedgerEntryType::Expense => "Other",
                    LedgerEntryType::Transfer => "Internal Transfer",
                    LedgerEntryType::Adjustment => "Other Adjustment",
                };
                treasury.categories_for_type(entry.campaign_id, desired)?.into_iter()
                    .find(|category| category.name == preferred).context("Default category not found")?.id
            }
        };
    }
    if request.get("account_id").is_some() || entry.account_id.is_none() {
        entry.account_id = optional_id(request, "account_id")?.or(
            treasury.accounts(entry.campaign_id)?.first().map(|account| account.id));
    }
    if request.get("transfer_account_id").is_some() || changed_type {
        entry.transfer_account_id = optional_id(request, "transfer_account_id")?;
    }
    if let Some(value) = request.get("description") {
        entry.description = value.as_str().context("Invalid description")?.to_string();
    }
    for name in ["vendor_source", "notes", "attachment_ref", "custom_recurrence"] {
        if let Some(value) = request.get(name) {
            let text = value.as_str().context("Invalid treasury text field")?.trim();
            if text.chars().count() > 100000 { bail!("Treasury text is too long"); }
            let value = if text.is_empty() { None } else { Some(text.to_string()) };
            match name {
                "vendor_source" => entry.vendor_source = value,
                "notes" => entry.notes = value,
                "attachment_ref" => entry.attachment_ref = value,
                _ => entry.custom_recurrence = value,
            }
        }
    }
    if request.get("related_task_id").is_some() {
        entry.related_task_id = optional_id(request, "related_task_id")?;
    }
    if request.get("recurrence").is_some() {
        entry.recurrence = LedgerRecurrence::parse(field(request, "recurrence")?)
            .context("Choose a valid recurrence")?;
    }
    if entry.recurrence != LedgerRecurrence::Custom { entry.custom_recurrence = None; }
    if request.get("due_date").is_some() { entry.due_date = date_field(request, "due_date")?; }
    if request.get("payment_date").is_some() { entry.payment_date = date_field(request, "payment_date")?; }
    if request.get("entry_date").is_some() {
        let date = date_field(request, "entry_date")?.context("Choose an entry date")?;
        if entry.created_at.with_timezone(&Local).date_naive() != date.with_timezone(&Local).date_naive() {
            entry.created_at = date;
        }
    }
    if entry.status == LedgerStatus::Paid && entry.payment_date.is_none() {
        entry.payment_date = Some(Utc::now());
    }
    Ok(())
}

fn local_campaign(app: &App, id: Uuid) -> Result<()> {
    let project = app
        .db
        .get_projects()?
        .into_iter()
        .find(|project| project.id == id)
        .context("Campaign not found")?;
    if project.archived || project.completed {
        bail!("Choose an active campaign");
    }
    if project.is_shared {
        bail!("Shared campaigns must be edited in Questline");
    }
    Ok(())
}

fn creation_id(request: &Value) -> Result<Uuid> {
    if request["autosave"].as_bool() == Some(true) && request["autosave_id"].is_string() {
        id_field(request, "autosave_id")
    } else {
        Ok(Uuid::new_v4())
    }
}

pub fn run(path: &Path, request: &Value) -> Result<Value> {
    if request["action"].as_str() == Some("capabilities") {
        return Ok(json!({"treasury_fields": true, "savings_accounts": true, "task_recurrence": true, "task_live_refresh": true,
            "treasury_defaults": {
                "accounts": crate::services::treasury::DEFAULT_ACCOUNTS.iter().map(|(name, kind)| json!({"name":name,"kind":kind.as_str()})).collect::<Vec<_>>(),
                "categories": crate::services::treasury::DEFAULT_CATEGORIES.iter().map(|(name, kind)| json!({"name":name,"entry_type":kind.as_str()})).collect::<Vec<_>>()
            }
        }));
    }
    if !path.is_file() {
        bail!("Open Questline and complete onboarding first");
    }
    let action=field(request,"action")?;
    {
        let db=crate::database::Database::new(path)?;
        let epoch=db.get_setting("shell_profile_epoch")?.unwrap_or_default();
        let key=format!("{}:{}",db.get_user()?.map(|u|u.id.to_string()).unwrap_or_default(),epoch);
        if !matches!(action,"drink_water"|"dismiss_hydration_reminder") && ((!epoch.is_empty() && request["profile_key"].as_str()!=Some(key.as_str())) || request["profile_key"].as_str().is_some_and(|v|!v.is_empty() && v!=key)) {
            bail!("The widget belongs to an older profile. Refresh Questline before saving.");
        }
    }
    let mut app = App::new_headless(path)?;
    if app.user.is_none() {
        bail!("Open Questline and complete onboarding first");
    }
    let action = field(request, "action")?;
    if action.starts_with("create_") && request["autosave"].as_bool() == Some(true)
        && request["autosave_id"].is_string() {
        let id = id_field(request, "autosave_id")?;
        let existing_campaign = match action {
            "create_task" | "create_step" => app.db.get_task_by_id(id).ok().and_then(|item| item.project_id),
            "create_scroll" => app.db.get_note_by_id(id).ok().and_then(|item| item.project_id),
            "create_chronicle" => app.db.get_journal_entries()?.into_iter().find(|item| item.id == id).map(|item| item.project_id),
            "create_entry" => TreasuryService::new(&app.db).get_entry(id)?.map(|item| item.campaign_id),
            _ => None,
        };
        let existing_sidequest = action == "create_sidequest" && app.db.get_rituals()?.iter().any(|item| item.id == id.to_string());
        if let Some(campaign_id) = existing_campaign {
            local_campaign(&app, campaign_id)?;
            let mut resumed = request.clone();
            resumed["action"] = Value::String(action.replacen("create_", "update_", 1));
            resumed["id"] = json!(id);
            return run(path, &resumed);
        }
        if existing_sidequest {
            let mut resumed = request.clone();
            resumed["action"] = Value::String(action.replacen("create_", "update_", 1));
            resumed["id"] = json!(id);
            return run(path, &resumed);
        }
    }
    let rewards = crate::desktop_notifications::RewardSnapshot::capture(&app.db)?;
    app.external_notifications = false;
    let result: Result<Value> = (|| {
    match action {
        "drink_water" => crate::desktop_hydration::drink(&mut app,request["reminder_token"].as_str().unwrap_or("")),
        "dismiss_hydration_reminder" => crate::desktop_hydration::dismiss(&app.db,request["reminder_token"].as_str().unwrap_or("")),
        "create_sidequest" | "update_sidequest" => {
            let name = text_field(request, "title", 200)?;
            let description = request["description"].as_str().unwrap_or("").trim().to_string();
            let frequency = field(request, "frequency")?;
            let daily_target = match frequency {
                "Daily" | "Weekdays" | "Weekly" | "Monthly" => 1,
                "2x Daily" => 2,
                "3x Daily" => 3,
                "5x Daily" => 5,
                _ => bail!("Unknown sidequest frequency"),
            };
            let reward_xp = request["reward_xp"].as_i64().context("Missing reward_xp")?;
            if !(0..=100).contains(&reward_xp) {
                bail!("Sidequest reward must be between 0 and 100 XP");
            }
            let now = Utc::now();
            let mut ritual = if action == "update_sidequest" {
                let id = field(request, "id")?;
                app.db.get_rituals()?.into_iter().find(|ritual| ritual.id == id).context("Sidequest not found")?
            } else {
                crate::models::Ritual {
                    id: creation_id(request)?.to_string(), name: String::new(), description: None,
                    frequency: String::new(), reward_xp: 0, daily_target: 1, created_at: now, updated_at: now,
                }
            };
            ritual.name = name;
            ritual.description = if description.is_empty() { None } else { Some(description) };
            ritual.frequency = frequency.to_string();
            ritual.daily_target = daily_target;
            ritual.reward_xp = reward_xp as i32;
            ritual.updated_at = now;
            if action == "update_sidequest" { app.db.update_ritual(&ritual)?; }
            else { app.db.insert_ritual(&ritual)?; }
            Ok(json!({ "id": ritual.id, "message": "Sidequest saved" }))
        }
        "complete_sidequest" => {
            let id = field(request, "id")?;
            let counts = app.db.get_ritual_day_counts(Local::now().date_naive())?;
            let (count, target) = counts.get(id).context("Sidequest not found")?;
            if count >= target { bail!("Sidequest is already complete today"); }
            app.complete_ritual(id)?;
            Ok(json!({ "id": id, "message": "Sidequest progress recorded" }))
        }
        "delete_sidequest" => {
            let id = field(request, "id")?;
            let rituals = app.db.get_rituals()?;
            if !rituals.iter().any(|ritual| ritual.id == id) { bail!("Sidequest not found"); }
            if rituals.len() <= 1 { bail!("Keep at least one active sidequest"); }
            app.db.delete_ritual(id)?;
            Ok(json!({ "id": id, "message": "Sidequest deleted" }))
        }
        "create_task" => {
            let project_id = id_field(request, "project_id")?;
            local_campaign(&app, project_id)?;
            let title = text_field(request, "title", 200)?;
            let description = request["description"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            let now = Utc::now();
            let task = Task {
                id: creation_id(request)?,
                project_id: Some(project_id),
                title,
                description: if description.is_empty() {
                    None
                } else {
                    Some(description)
                },
                due_date: due_date(request)?,
                set_date: None,
                completed: false,
                priority: TaskPriority::from_str(request["priority"].as_str().unwrap_or("Medium")),
                created_at: now,
                updated_at: now,
                owner_identity: Some(app.identity.public_key.clone()),
                owner_username: app.user.as_ref().map(|user| user.username.clone()),
                parent_task_id: None,
                xp_awarded: false,
                recurrence: task_recurrence(request)?,
            };
            app.db.insert_task(&task)?;
            if let Some(user) = app.user.as_mut() {
                XPService::new(&app.db).grant_xp(user, "Spawn Quest", 5)?;
            }
            Ok(json!({"id": task.id, "message": "Quest created"}))
        }
        "update_task" => {
            let mut task = app.db.get_task_by_id(id_field(request, "id")?)?;
            local_campaign(&app, task.project_id.context("Quest has no campaign")?)?;
            task.title = text_field(request, "title", 200)?;
            let description = request["description"].as_str().unwrap_or("").trim();
            task.description = if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            };
            task.due_date = due_date(request)?;
            task.priority =
                TaskPriority::from_str(request["priority"].as_str().unwrap_or("Medium"));
            if request.get("recurrence").is_some() {
                task.recurrence = task_recurrence(request)?;
            }
            app.db.update_task(&task)?;
            Ok(json!({"id": task.id, "message": "Quest updated"}))
        }
        "complete_task" => {
            let task = app.db.get_task_by_id(id_field(request, "id")?)?;
            local_campaign(&app, task.project_id.context("Quest has no campaign")?)?;
            if task.completed {
                bail!("Quest is already complete");
            }
            if app
                .db
                .get_tasks()?
                .iter()
                .any(|step| step.parent_task_id == Some(task.id) && !step.completed)
            {
                bail!("Complete the quest's steps before completing it");
            }
            let id = task.id;
            app.complete_dashboard_task(task)?;
            if !app.db.get_task_by_id(id)?.completed {
                bail!("Resolve the quest's blockers before completing it");
            }
            Ok(json!({"id": id, "message": "Quest completed"}))
        }
        "delete_task" => {
            let task = app.db.get_task_by_id(id_field(request, "id")?)?;
            if task.parent_task_id.is_some() {
                bail!("Use delete_step for a step");
            }
            local_campaign(&app, task.project_id.context("Quest has no campaign")?)?;
            let steps: Vec<Task> = app
                .db
                .get_tasks()?
                .into_iter()
                .filter(|step| step.parent_task_id == Some(task.id))
                .collect();
            for step in &steps {
                app.db.delete_task(step.id)?;
            }
            app.db.delete_task(task.id)?;
            Ok(json!({"message": "Quest and its steps deleted"}))
        }
        "delete_step" => {
            let step = app.db.get_task_by_id(id_field(request, "id")?)?;
            let parent_id = step.parent_task_id.context("Task is not a step")?;
            let parent = app.db.get_task_by_id(parent_id)?;
            if step.project_id != parent.project_id {
                bail!("Step and quest campaigns do not match");
            }
            local_campaign(&app, parent.project_id.context("Quest has no campaign")?)?;
            app.db.delete_task(step.id)?;
            Ok(json!({"id": parent_id, "message": "Step deleted"}))
        }
        "create_step" => {
            let parent = app.db.get_task_by_id(id_field(request, "parent_id")?)?;
            if parent.parent_task_id.is_some() || parent.completed {
                bail!("Choose an open parent quest");
            }
            let project_id = parent.project_id.context("Quest has no campaign")?;
            local_campaign(&app, project_id)?;
            let title = text_field(request, "title", 200)?;
            let description = request["description"].as_str().unwrap_or("").trim();
            let now = Utc::now();
            let step = Task {
                id: creation_id(request)?,
                project_id: Some(project_id),
                title,
                description: if description.is_empty() {
                    None
                } else {
                    Some(description.to_string())
                },
                due_date: due_date(request)?,
                set_date: None,
                completed: false,
                priority: TaskPriority::from_str(request["priority"].as_str().unwrap_or("Medium")),
                created_at: now,
                updated_at: now,
                owner_identity: Some(app.identity.public_key.clone()),
                owner_username: app.user.as_ref().map(|user| user.username.clone()),
                parent_task_id: Some(parent.id),
                xp_awarded: false,
                recurrence: None,
            };
            app.db.insert_task(&step)?;
            if let Some(user) = app.user.as_mut() {
                XPService::new(&app.db).grant_xp(user, "Spawn Step", 5)?;
            }
            Ok(json!({"id": step.id, "message": "Step created"}))
        }
        "update_step" => {
            let mut step = app.db.get_task_by_id(id_field(request, "id")?)?;
            let parent_id = step.parent_task_id.context("Task is not a step")?;
            let parent = app.db.get_task_by_id(parent_id)?;
            let project_id = parent.project_id.context("Quest has no campaign")?;
            if step.project_id != Some(project_id) {
                bail!("Step and quest campaigns do not match");
            }
            local_campaign(&app, project_id)?;
            step.title = text_field(request, "title", 200)?;
            let description = request["description"].as_str().unwrap_or("").trim();
            step.description = if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            };
            step.due_date = due_date(request)?;
            step.priority =
                TaskPriority::from_str(request["priority"].as_str().unwrap_or("Medium"));
            app.db.update_task(&step)?;
            Ok(json!({"id": step.id, "message": "Step updated"}))
        }
        "toggle_step" => {
            let mut step = app.db.get_task_by_id(id_field(request, "id")?)?;
            let parent_id = step.parent_task_id.context("Task is not a step")?;
            let parent = app.db.get_task_by_id(parent_id)?;
            if step.project_id != parent.project_id {
                bail!("Step and quest campaigns do not match");
            }
            local_campaign(&app, parent.project_id.context("Quest has no campaign")?)?;
            let was_completed = step.completed;
            let already_awarded = step.xp_awarded;
            step.completed = !was_completed;
            if step.completed {
                step.xp_awarded = true;
            }
            app.db.update_task(&step)?;
            if step.completed && !already_awarded {
                let high = step.priority == TaskPriority::High;
                app.grant_xp(
                    if high {
                        "Resolve Hero Step Quest"
                    } else {
                        "Complete Step Quest"
                    },
                    if high { 50 } else { 25 },
                )?;
                app.complete_productive_action()?;
                app.grow_tree(2)?;
                app.update_daily_adventure_progress("complete_tasks", 1)?;
            }
            Ok(
                json!({"id": parent_id, "message": if was_completed { "Step reopened" } else { "Step completed" }}),
            )
        }
        "create_chronicle" => {
            let project_id = id_field(request, "project_id")?;
            local_campaign(&app, project_id)?;
            let content = text_field(request, "content", crate::app::JOURNAL_ENTRY_CHAR_LIMIT)?;
            let now = Utc::now();
            let entry = JournalEntry {
                id: creation_id(request)?,
                project_id,
                entry_date: now.date_naive(),
                content,
                created_at: now,
                updated_at: now,
                visibility: "Private".to_string(),
                author_username: app.user.as_ref().context("Character not found")?.username.clone(),
            };
            app.db.insert_journal_entry(&entry)?;
            app.apply_class_passive("journal_create", 0)?;
            app.complete_productive_action()?;
            app.increment_quest_progress(60, 1)?;
            app.update_daily_adventure_progress("write_journal", 1)?;
            app.check_action_achievements()?;
            Ok(json!({"id": entry.id, "message": "Chronicle created"}))
        }
        "update_chronicle" => {
            let id = id_field(request, "id")?;
            let entry = app.db.get_journal_entries()?.into_iter()
                .find(|item| item.id == id).context("Chronicle not found")?;
            local_campaign(&app, entry.project_id)?;
            let content = text_field(request, "content", crate::app::JOURNAL_ENTRY_CHAR_LIMIT)?;
            app.db.update_journal_entry(id, &content)?;
            Ok(json!({"id": id, "message": "Chronicle updated"}))
        }
        "create_scroll" => {
            let project_id = id_field(request, "project_id")?;
            local_campaign(&app, project_id)?;
            let title = text_field(request, "title", 200)?;
            let content = text_field(request, "content", 100_000)?;
            let now = Utc::now();
            let note = Note {
                id: creation_id(request)?,
                project_id: Some(project_id),
                title,
                markdown_content: content,
                created_at: now,
                updated_at: now,
                sharing_permission: "collaborative".to_string(),
                codex_id: None,
                owner_identity: Some(app.identity.public_key.clone()),
            };
            app.db.insert_note(&note)?;
            app.grant_xp("Write New Scroll Note", 5)?;
            app.complete_productive_action()?;
            app.update_daily_adventure_progress("write_note", 1)?;
            Ok(json!({"id": note.id, "message": "Scroll created"}))
        }
        "update_scroll" => {
            let mut note = app.db.get_note_by_id(id_field(request, "id")?)?;
            local_campaign(&app, note.project_id.context("Scroll has no campaign")?)?;
            note.title = text_field(request, "title", 200)?;
            note.markdown_content = text_field(request, "content", 100_000)?;
            app.db.update_note(&note)?;
            if request["autosave"].as_bool() != Some(true) {
                app.grant_xp("Edit Scroll Note", 2)?;
            }
            Ok(json!({"id": note.id, "message": "Scroll updated"}))
        }
        "delete_scroll" => {
            let note = app.db.get_note_by_id(id_field(request, "id")?)?;
            if let Some(project_id) = note.project_id {
                local_campaign(&app, project_id)?;
            }
            app.db.delete_note(note.id)?;
            Ok(json!({"message": "Scroll deleted"}))
        }
        "treasury_snapshot" => {
            let campaign_id = id_field(request,"project_id")?;
            local_campaign(&app,campaign_id)?;
            let treasury = TreasuryService::new(&app.db);
            Ok(json!({"accounts":treasury.account_balances(campaign_id)?,
                "totals":crate::models::treasury::CampaignTotalsReport::from(treasury.calculate_campaign_totals(campaign_id)?)}))
        }
        "configure_savings" => {
            let treasury = TreasuryService::new(&app.db);
            let mut account = treasury.get_account(id_field(request,"id")?)?.context("Account not found")?;
            local_campaign(&app, account.campaign_id)?;
            if account.kind != crate::models::TreasuryAccountKind::Savings { bail!("Choose a savings account"); }
            if request["version"].as_i64() != Some(account.version) { bail!("Account changed; refresh before saving"); }
            account.savings_status = crate::models::SavingsStatus::parse(field(request,"savings_status")?).context("Choose Available or Invested")?;
            account.interest_rate_bps = crate::services::treasury::parse_interest_rate(field(request,"interest_rate")?)?;
            account.interest_period = crate::models::InterestPeriod::parse(field(request,"interest_period")?).context("Choose Monthly or Annual")?;
            let saved = treasury.update_account(account)?;
            Ok(json!({"id":saved.id,"message":"Savings settings saved; interest estimates do not change balances"}))
        }
        "create_entry" | "update_entry" => {
            let treasury = TreasuryService::new(&app.db);
            let updating = field(request, "action")? == "update_entry";
            let now = Utc::now();
            let mut entry = if updating {
                treasury.get_entry(id_field(request, "id")?)?.context("Treasury entry not found")?
            } else {
                let campaign_id = id_field(request, "project_id")?;
                local_campaign(&app, campaign_id)?;
                let campaign = treasury.ensure_campaign(campaign_id)?;
                LedgerEntry {
                    id: creation_id(request)?, campaign_id, title: String::new(),
                    description: String::new(), entry_type: LedgerEntryType::Expense,
                    category_id: Uuid::nil(), amount_minor: 0, currency_code: campaign.currency_code,
                    status: LedgerStatus::Planned, due_date: None, payment_date: None,
                    vendor_source: None, related_task_id: None, notes: None, attachment_ref: None,
                    recurrence: LedgerRecurrence::None, custom_recurrence: None,
                    version: 1, created_at: now, updated_at: now,
                    created_by_identity: Some(app.identity.public_key.clone()),
                    account_id: None, transfer_account_id: None,
                }
            };
            local_campaign(&app, entry.campaign_id)?;
            entry.title = text_field(request, "title", 200)?;
            entry.amount_minor = request["amount_minor"].as_i64().context("Missing amount")?;
            if entry.amount_minor <= 0 { bail!("Amount must be greater than zero"); }
            entry.status = LedgerStatus::parse(request["status"].as_str().unwrap_or("Planned"))
                .context("Choose a valid treasury status")?;
            let legacy_source = if entry.account_id.is_none() && updating {
                treasury.accounts(entry.campaign_id)?.into_iter().find(|account| account.is_default && account.name == "Spending").map(|account| account.id)
            } else { None };
            let previous_accounts = [entry.account_id.or(legacy_source), entry.transfer_account_id];
            apply_entry_fields(request, &mut entry, &treasury)?;
            if request["custom_accounts_only"].as_bool() == Some(true) {
                for id in [entry.account_id, entry.transfer_account_id].into_iter().flatten() {
                    let account = treasury.get_account(id)?.context("Account not found")?;
                    if account.is_default && !(updating && previous_accounts.contains(&Some(id))) {
                        bail!("Choose a user-created account");
                    }
                }
            }
            let saved = if updating { treasury.update_entry(entry)? } else { treasury.create_entry(entry)? };
            Ok(json!({"id": saved.id, "message": if updating {"Treasury entry updated"} else {"Treasury entry created"}}))
        }
        "water_tree" => {
            if let Err(message) = app.is_watering_allowed()? {
                bail!("{message}");
            }
            app.water_tree()?;
            let tree = app.db.get_zen_tree()?;
            Ok(
                json!({"message": "Evergrowth watered", "water_today": tree.water_today, "stage": tree.stage}),
            )
        }
        _ => bail!("Unknown shell action"),
    }
    })();
    rewards.attach(&app.db,request,result?)
}

pub fn render_tree(request: &Value) -> Result<Value> {
    use ratatui::style::Color;

    let rows = request["rows"]
        .as_u64()
        .context("Missing rows")?
        .clamp(8, 40) as usize;
    let cols = request["cols"]
        .as_u64()
        .context("Missing cols")?
        .clamp(20, 120) as usize;
    let seed = request["seed"].as_u64().context("Missing seed")?;
    let stage = request["stage"]
        .as_i64()
        .context("Missing stage")?
        .clamp(1, 7) as i32;
    let health = request["health"]
        .as_i64()
        .context("Missing health")?
        .clamp(0, 100) as i32;
    let lines =
        crate::services::bonsai::BonsaiGrid::generate(rows, cols, seed, stage, health).into_lines();
    let rows = lines
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| {
                    let color = match span.style.fg {
                        Some(Color::Rgb(red, green, blue)) => {
                            format!("#{red:02x}{green:02x}{blue:02x}")
                        }
                        _ => String::new(),
                    };
                    json!({ "text": span.content, "color": color })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    Ok(json!({ "rows": rows }))
}

pub fn hero_metadata(request: &Value) -> Result<Value> {
    use crate::models::user::{ClassType, User};

    let class = ClassType::from_str(field(request, "class")?).context("Unknown hero class")?;
    let level = request["level"].as_i64().context("Missing level")?;
    if !(1..=100).contains(&level) {
        bail!("Hero level must be between 1 and 100");
    }
    let user = User {
        id: Uuid::nil(), username: String::new(), class, level: level as i32, xp: 0,
        created_at: Utc::now(), specialization: None,
    };
    let powers = class.powers().into_iter().map(|(required_level, name, description)| {
        json!({ "level": required_level, "name": name, "description": description, "unlocked": i64::from(required_level) <= level })
    }).collect::<Vec<_>>();
    let specializations = class.specializations().into_iter().map(|(name, description)| {
        json!({ "name": name, "description": description })
    }).collect::<Vec<_>>();
    Ok(json!({ "title": user.title(), "xp_next_level": User::xp_for_next_level(user.level),
        "description": class.description(), "flavor": class.flavor(), "order": class.order(),
        "passive": class.passive_description(), "powers": powers, "specializations": specializations }))
}

#[cfg(test)]
mod hero_tests {
    use super::*;
    use crate::models::user::ClassType;

    #[test]
    fn hero_metadata_matches_every_class_and_level_boundary() {
        for class in [ClassType::CodeWarlock, ClassType::TaskPaladin, ClassType::MindSage,
            ClassType::SystemsArchitect, ClassType::TimeChronomancer, ClassType::ArchAccountant] {
            for level in [1, 5, 10, 50, 100] {
                let hero = hero_metadata(&json!({ "class": class.name(), "level": level })).unwrap();
                assert_eq!(hero["passive"], class.passive_description());
                assert_eq!(hero["powers"].as_array().unwrap().len(), class.powers().len());
                for power in hero["powers"].as_array().unwrap() {
                    assert_eq!(power["unlocked"], power["level"].as_i64().unwrap() <= level);
                }
                assert_eq!(hero["xp_next_level"], if level == 100 { 0 } else { 200 + level * level * 12 });
            }
        }
    }

    #[test]
    fn invalid_hero_metadata_is_rejected() {
        for request in [json!({}), json!({ "class": "Unknown", "level": 1 }),
            json!({ "class": "Code Warlock", "level": 0 }),
            json!({ "class": "Code Warlock", "level": 101 })] {
            assert!(hero_metadata(&request).is_err());
        }
    }
}
