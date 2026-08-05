use std::cmp::Ordering;

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{OptionalExtension, Row, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::database::Database;
use crate::models::{
    BudgetUsage, BudgetWarningLevel, CampaignTotals, CampaignTreasury, CategoryBudget,
    CategoryReport, CategoryTotals, Currency, LedgerCategory, LedgerEntry, LedgerEntryType,
    LedgerFilter, LedgerRecurrence, LedgerSort, LedgerStatus, MonthlySpending, TaskFinancials,
    TaskPaymentStatus, TreasuryReport,
};

pub const DEFAULT_CATEGORIES: [&str; 10] = [
    "Development",
    "Infrastructure",
    "Design",
    "Marketing",
    "Equipment",
    "Subscriptions",
    "Contractors",
    "Travel",
    "Administrative",
    "Other",
];

pub struct TreasuryService<'a> {
    db: &'a Database,
}

impl<'a> TreasuryService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Inicializa la tesorería y sus categorías de forma idempotente para evitar duplicados entre dispositivos.
    pub fn ensure_campaign(&self, campaign_id: Uuid) -> Result<CampaignTreasury> {
        self.ensure_campaign_exists(campaign_id)?;
        let now = Utc::now().to_rfc3339();
        let inserted = self.db.conn.execute(
            "INSERT OR IGNORE INTO campaign_treasury
             (campaign_id, overall_budget_minor, currency_code, large_expense_threshold_minor, version, created_at, updated_at)
             VALUES (?1, 0, 'USD', 100000, 1, ?2, ?2)",
            params![campaign_id.to_string(), now],
        )?;
        if inserted > 0 {
            self.db
                .log_change("campaign_treasury", &campaign_id.to_string(), "create")?;
        }
        for name in DEFAULT_CATEGORIES {
            let id = deterministic_category_id(campaign_id, name);
            let inserted = self.db.conn.execute(
                "INSERT OR IGNORE INTO ledger_categories
                 (id, campaign_id, name, is_default, version, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 1, 1, ?4, ?4)",
                params![id.to_string(), campaign_id.to_string(), name, now],
            )?;
            if inserted > 0 {
                self.db
                    .log_change("ledger_category", &id.to_string(), "create")?;
            }
        }
        self.get_campaign(campaign_id)?
            .ok_or_else(|| anyhow!("Treasury initialization did not persist"))
    }

    pub fn get_campaign(&self, campaign_id: Uuid) -> Result<Option<CampaignTreasury>> {
        self.db
            .conn
            .query_row(
                "SELECT campaign_id, overall_budget_minor, currency_code,
                        large_expense_threshold_minor, version, created_at, updated_at
                 FROM campaign_treasury WHERE campaign_id = ?1",
                params![campaign_id.to_string()],
                map_campaign,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_overall_budget(&self, campaign_id: Uuid, amount_minor: i64) -> Result<()> {
        if amount_minor < 0 {
            bail!("Budget cannot be negative");
        }
        let treasury = self.ensure_campaign(campaign_id)?;
        let currency = Currency::from_code_or_default(&treasury.currency_code);
        let now = Utc::now();
        self.db.conn.execute(
            "UPDATE campaign_treasury
             SET overall_budget_minor = ?1, version = version + 1, updated_at = ?2
             WHERE campaign_id = ?3",
            params![amount_minor, now.to_rfc3339(), campaign_id.to_string()],
        )?;
        self.record_history(
            campaign_id,
            "campaign_treasury",
            campaign_id,
            "budget_updated",
            treasury.version + 1,
        )?;
        self.db
            .log_change("campaign_treasury", &campaign_id.to_string(), "upsert")?;
        if treasury.overall_budget_minor != amount_minor {
            self.record_chronicle(
                campaign_id,
                &format!(
                    "Campaign budget changed from {} to {}.",
                    format_money(treasury.overall_budget_minor, currency),
                    format_money(amount_minor, currency)
                ),
            )?;
        }
        Ok(())
    }

    /// Divisa activa de la campaña, con USD como respaldo si la fila trae un código desconocido.
    pub fn campaign_currency(&self, campaign_id: Uuid) -> Result<Currency> {
        Ok(self
            .get_campaign(campaign_id)?
            .map(|treasury| Currency::from_code_or_default(&treasury.currency_code))
            .unwrap_or_default())
    }

    /// Cambia la divisa de trabajo de la campaña. No convierte importes: los números
    /// se mantienen y solo pasan a leerse en la nueva denominación, así que las filas
    /// existentes se reetiquetan para que los totales no mezclen códigos.
    pub fn set_currency(&self, campaign_id: Uuid, currency: Currency) -> Result<()> {
        let treasury = self.ensure_campaign(campaign_id)?;
        let previous = Currency::from_code_or_default(&treasury.currency_code);
        let now = Utc::now().to_rfc3339();
        self.db.conn.execute(
            "UPDATE campaign_treasury
             SET currency_code = ?1, version = version + 1, updated_at = ?2
             WHERE campaign_id = ?3",
            params![currency.code(), now, campaign_id.to_string()],
        )?;
        self.record_history(
            campaign_id,
            "campaign_treasury",
            campaign_id,
            "currency_updated",
            treasury.version + 1,
        )?;
        self.db
            .log_change("campaign_treasury", &campaign_id.to_string(), "upsert")?;

        for (table, key_column, entity_type) in [
            ("ledger_entries", "id", "ledger_entry"),
            ("task_financials", "task_id", "task_financials"),
        ] {
            let ids: Vec<String> = {
                let mut stmt = self.db.conn.prepare(&format!(
                    "SELECT {key_column} FROM {table}
                     WHERE campaign_id = ?1 AND currency_code <> ?2"
                ))?;
                stmt.query_map(params![campaign_id.to_string(), currency.code()], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
            };
            for id in ids {
                self.db.conn.execute(
                    &format!(
                        "UPDATE {table}
                         SET currency_code = ?1, version = version + 1, updated_at = ?2
                         WHERE {key_column} = ?3"
                    ),
                    params![currency.code(), now, id],
                )?;
                self.db.log_change(entity_type, &id, "update")?;
            }
        }

        if previous != currency || treasury.currency_code != currency.code() {
            self.record_chronicle(
                campaign_id,
                &format!(
                    "Campaign currency switched from {} to {} (amounts were relabeled, not converted).",
                    previous.code(),
                    currency.code()
                ),
            )?;
        }
        Ok(())
    }

    pub fn create_category(&self, campaign_id: Uuid, name: &str) -> Result<LedgerCategory> {
        self.ensure_campaign(campaign_id)?;
        let name = name.trim();
        if name.is_empty() {
            bail!("Category name cannot be empty");
        }
        let id = Uuid::new_v4();
        let now = Utc::now();
        self.db.conn.execute(
            "INSERT INTO ledger_categories
             (id, campaign_id, name, is_default, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, 1, ?4, ?4)",
            params![
                id.to_string(),
                campaign_id.to_string(),
                name,
                now.to_rfc3339()
            ],
        )?;
        self.record_history(campaign_id, "ledger_category", id, "created", 1)?;
        self.db
            .log_change("ledger_category", &id.to_string(), "create")?;
        self.get_category(id)?.context("Category was not persisted")
    }

    pub fn categories(&self, campaign_id: Uuid) -> Result<Vec<LedgerCategory>> {
        self.ensure_campaign(campaign_id)?;
        let mut stmt = self.db.conn.prepare(
            "SELECT id, campaign_id, name, is_default, version, created_at, updated_at
             FROM ledger_categories WHERE campaign_id = ?1
             ORDER BY is_default DESC, name COLLATE NOCASE",
        )?;
        stmt.query_map(params![campaign_id.to_string()], map_category)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn set_category_budget(
        &self,
        campaign_id: Uuid,
        category_id: Uuid,
        amount_minor: i64,
    ) -> Result<()> {
        if amount_minor < 0 {
            bail!("Category budget cannot be negative");
        }
        let category = self
            .get_category(category_id)?
            .filter(|category| category.campaign_id == campaign_id)
            .context("Category does not belong to this campaign")?;
        let previous = self.get_category_budget(category_id)?;
        let now = Utc::now();
        self.db.conn.execute(
            "INSERT INTO category_budgets
             (category_id, campaign_id, amount_minor, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)
             ON CONFLICT(category_id) DO UPDATE SET
                 amount_minor=excluded.amount_minor,
                 version=category_budgets.version + 1,
                 updated_at=excluded.updated_at",
            params![
                category_id.to_string(),
                campaign_id.to_string(),
                amount_minor,
                now.to_rfc3339()
            ],
        )?;
        let version = previous.as_ref().map_or(1, |budget| budget.version + 1);
        self.record_history(
            campaign_id,
            "category_budget",
            category_id,
            "budget_updated",
            version,
        )?;
        self.db
            .log_change("category_budget", &category_id.to_string(), "upsert")?;
        if previous.as_ref().map(|budget| budget.amount_minor) != Some(amount_minor) {
            self.record_chronicle(
                campaign_id,
                &format!(
                    "{} budget changed to {}.",
                    category.name,
                    format_money(amount_minor, self.campaign_currency(campaign_id)?)
                ),
            )?;
        }
        Ok(())
    }

    /// Crea un movimiento validado y conserva una instantánea auditable antes de anunciarlo al sincronizador.
    pub fn create_entry(&self, mut entry: LedgerEntry) -> Result<LedgerEntry> {
        self.ensure_campaign(entry.campaign_id)?;
        self.validate_entry(&entry)?;
        if entry.id.is_nil() {
            entry.id = Uuid::new_v4();
        }
        let now = Utc::now();
        entry.created_at = now;
        entry.updated_at = now;
        entry.version = 1;
        self.db.conn.execute(
            "INSERT INTO ledger_entries
             (id, campaign_id, title, description, entry_type, category_id, amount_minor,
              currency_code, status, due_date, payment_date, vendor_source, related_task_id,
              notes, attachment_ref, recurrence, custom_recurrence, version, created_at, updated_at,
              created_by_identity)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                     ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            entry_params(&entry),
        )?;
        self.record_history(entry.campaign_id, "ledger_entry", entry.id, "created", 1)?;
        self.db
            .log_change("ledger_entry", &entry.id.to_string(), "create")?;
        if entry.entry_type == LedgerEntryType::Income && entry.status != LedgerStatus::Planned {
            self.record_chronicle(
                entry.campaign_id,
                &format!("Funding received: {}.", entry.title),
            )?;
        }
        self.check_budget_events(entry.campaign_id, Some(&entry))?;
        Ok(entry)
    }

    /// Actualiza con control optimista de versión para que una edición obsoleta no sobrescriba otra más reciente.
    pub fn update_entry(&self, mut entry: LedgerEntry) -> Result<LedgerEntry> {
        self.validate_entry(&entry)?;
        let previous = self
            .get_entry(entry.id)?
            .context("Ledger entry not found")?;
        if previous.campaign_id != entry.campaign_id {
            bail!("A ledger entry cannot move between campaigns");
        }
        // La autoría es inmutable: editar el movimiento de otro no lo vuelve propio, y de
        // ahí depende que un Companion pueda seguir tocándolo.
        entry.created_by_identity = previous.created_by_identity.clone();
        let expected_version = entry.version;
        entry.version += 1;
        entry.updated_at = Utc::now();
        let changed = self.db.conn.execute(
            "UPDATE ledger_entries SET
                 title=?1, description=?2, entry_type=?3, category_id=?4, amount_minor=?5,
                 currency_code=?6, status=?7, due_date=?8, payment_date=?9, vendor_source=?10,
                 related_task_id=?11, notes=?12, attachment_ref=?13, recurrence=?14,
                 custom_recurrence=?15, version=?16, updated_at=?17
             WHERE id=?18 AND version=?19",
            params![
                entry.title,
                entry.description,
                entry.entry_type.as_str(),
                entry.category_id.to_string(),
                entry.amount_minor,
                entry.currency_code,
                entry.status.as_str(),
                entry.due_date.map(|date| date.to_rfc3339()),
                entry.payment_date.map(|date| date.to_rfc3339()),
                entry.vendor_source,
                entry.related_task_id.map(|id| id.to_string()),
                entry.notes,
                entry.attachment_ref,
                entry.recurrence.as_str(),
                entry.custom_recurrence,
                entry.version,
                entry.updated_at.to_rfc3339(),
                entry.id.to_string(),
                expected_version,
            ],
        )?;
        if changed == 0 {
            bail!("Ledger entry changed on another device; reload before editing");
        }
        self.record_history(
            entry.campaign_id,
            "ledger_entry",
            entry.id,
            "updated",
            entry.version,
        )?;
        self.db
            .log_change("ledger_entry", &entry.id.to_string(), "update")?;
        self.record_significant_transition(&previous, &entry)?;
        self.check_budget_events(entry.campaign_id, Some(&entry))?;
        Ok(entry)
    }

    pub fn delete_entry(&self, id: Uuid) -> Result<()> {
        let entry = self.get_entry(id)?.context("Ledger entry not found")?;
        self.db.create_revision(
            "ledger_entry",
            &id.to_string(),
            &serde_json::to_string(&entry)?,
        )?;
        self.record_history(
            entry.campaign_id,
            "ledger_entry",
            id,
            "deleted",
            entry.version + 1,
        )?;
        self.db.conn.execute(
            "DELETE FROM ledger_entries WHERE id = ?1",
            params![id.to_string()],
        )?;
        self.db
            .log_change("ledger_entry", &id.to_string(), "delete")?;
        Ok(())
    }

    pub fn approve_entry(&self, id: Uuid) -> Result<LedgerEntry> {
        let mut entry = self.get_entry(id)?.context("Ledger entry not found")?;
        if entry.status != LedgerStatus::Planned {
            bail!("Only planned entries can be approved");
        }
        entry.status = LedgerStatus::Approved;
        self.update_entry(entry)
    }

    pub fn mark_paid(&self, id: Uuid, payment_date: DateTime<Utc>) -> Result<LedgerEntry> {
        let mut entry = self.get_entry(id)?.context("Ledger entry not found")?;
        if entry.status == LedgerStatus::Cancelled {
            bail!("A cancelled entry cannot be paid");
        }
        entry.status = LedgerStatus::Paid;
        entry.payment_date = Some(payment_date);
        self.update_entry(entry)
    }

    pub fn get_entry(&self, id: Uuid) -> Result<Option<LedgerEntry>> {
        self.db
            .conn
            .query_row(
                &format!("{} WHERE id = ?1", ledger_select()),
                params![id.to_string()],
                map_entry,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn entries(
        &self,
        campaign_id: Uuid,
        filter: &LedgerFilter,
        sort: LedgerSort,
    ) -> Result<Vec<LedgerEntry>> {
        let mut stmt = self
            .db
            .conn
            .prepare(&format!("{} WHERE campaign_id = ?1", ledger_select()))?;
        let mut entries = stmt
            .query_map(params![campaign_id.to_string()], map_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        entries.retain(|entry| matches_filter(entry, filter));
        sort_entries(&mut entries, sort);
        Ok(entries)
    }

    /// Calcula el disponible únicamente con fondos confirmados y compromisos aprobados.
    pub fn calculate_campaign_totals(&self, campaign_id: Uuid) -> Result<CampaignTotals> {
        let treasury = self.ensure_campaign(campaign_id)?;
        let (income, paid, committed): (i64, i64, i64) = self.db.conn.query_row(
            "SELECT
                 COALESCE(SUM(CASE WHEN entry_type='Income' AND status IN ('Approved','Paid') THEN amount_minor ELSE 0 END), 0),
                 COALESCE(SUM(CASE WHEN entry_type='Expense' AND status='Paid' THEN amount_minor ELSE 0 END), 0),
                 COALESCE(SUM(CASE WHEN entry_type='Expense' AND status='Approved' THEN amount_minor ELSE 0 END), 0)
             FROM ledger_entries WHERE campaign_id = ?1 AND status != 'Cancelled'",
            params![campaign_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        Ok(CampaignTotals {
            budget_minor: treasury.overall_budget_minor,
            income_minor: income,
            paid_minor: paid,
            committed_minor: committed,
            available_minor: income - paid - committed,
        })
    }

    pub fn calculate_category_totals(&self, campaign_id: Uuid) -> Result<Vec<CategoryTotals>> {
        let categories = self.categories(campaign_id)?;
        categories
            .into_iter()
            .map(|category| {
                let spending = self.db.conn.query_row(
                    "SELECT COALESCE(SUM(amount_minor), 0) FROM ledger_entries
                     WHERE campaign_id=?1 AND category_id=?2 AND entry_type='Expense'
                       AND status IN ('Approved','Paid')",
                    params![campaign_id.to_string(), category.id.to_string()],
                    |row| row.get::<_, i64>(0),
                )?;
                let budget = self
                    .get_category_budget(category.id)?
                    .map(|item| item.amount_minor);
                Ok(CategoryTotals {
                    category,
                    budget_minor: budget,
                    spending_minor: spending,
                    remaining_minor: budget.map(|value| value - spending),
                })
            })
            .collect()
    }

    pub fn upcoming_payments(
        &self,
        campaign_id: Uuid,
        now: DateTime<Utc>,
        days: i64,
    ) -> Result<Vec<LedgerEntry>> {
        let until = now + Duration::days(days.max(0));
        let filter = LedgerFilter {
            status: Some(LedgerStatus::Approved),
            date_from: Some(now),
            date_to: Some(until),
            entry_type: Some(LedgerEntryType::Expense),
            ..Default::default()
        };
        self.entries(campaign_id, &filter, LedgerSort::DueDate)
    }

    pub fn overdue_payments(
        &self,
        campaign_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<LedgerEntry>> {
        let filter = LedgerFilter {
            status: Some(LedgerStatus::Approved),
            date_to: Some(now),
            entry_type: Some(LedgerEntryType::Expense),
            ..Default::default()
        };
        self.entries(campaign_id, &filter, LedgerSort::DueDate)
            .map(|entries| {
                entries
                    .into_iter()
                    .filter(|entry| entry.due_date.is_some_and(|due| due < now))
                    .collect()
            })
    }

    pub fn budget_usage(budget_minor: i64, used_minor: i64) -> BudgetUsage {
        let ratio = if budget_minor <= 0 {
            if used_minor > 0 { f64::INFINITY } else { 0.0 }
        } else {
            used_minor as f64 / budget_minor as f64
        };
        let warning = if used_minor > budget_minor {
            BudgetWarningLevel::Exceeded
        } else if ratio >= 1.0 {
            BudgetWarningLevel::Exhausted
        } else if ratio >= 0.9 {
            BudgetWarningLevel::NinetyPercent
        } else if ratio >= 0.8 {
            BudgetWarningLevel::EightyPercent
        } else {
            BudgetWarningLevel::Healthy
        };
        BudgetUsage {
            budget_minor,
            used_minor,
            ratio,
            warning,
        }
    }

    pub fn set_task_financials(&self, value: &TaskFinancials) -> Result<()> {
        for amount in [
            value.estimated_cost_minor,
            value.actual_cost_minor,
            value.billable_amount_minor,
        ]
        .into_iter()
        .flatten()
        {
            if amount < 0 {
                bail!("Task financial amounts cannot be negative");
            }
        }
        let now = Utc::now().to_rfc3339();
        self.db.conn.execute(
            "INSERT INTO task_financials
             (task_id, campaign_id, estimated_cost_minor, actual_cost_minor, billable_amount_minor,
              payment_status, currency_code, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?8)
             ON CONFLICT(task_id) DO UPDATE SET
               campaign_id=excluded.campaign_id, estimated_cost_minor=excluded.estimated_cost_minor,
               actual_cost_minor=excluded.actual_cost_minor, billable_amount_minor=excluded.billable_amount_minor,
               payment_status=excluded.payment_status, currency_code=excluded.currency_code,
               version=task_financials.version + 1, updated_at=excluded.updated_at",
            params![value.task_id.to_string(), value.campaign_id.to_string(), value.estimated_cost_minor,
                value.actual_cost_minor, value.billable_amount_minor,
                value.payment_status.map(|status| status.as_str()), value.currency_code, now],
        )?;
        self.db
            .log_change("task_financials", &value.task_id.to_string(), "upsert")?;
        Ok(())
    }

    pub fn get_task_financials(&self, task_id: Uuid) -> Result<Option<TaskFinancials>> {
        self.db.conn.query_row(
            "SELECT task_id, campaign_id, estimated_cost_minor, actual_cost_minor,
                    billable_amount_minor, payment_status, currency_code, version, created_at, updated_at
             FROM task_financials WHERE task_id=?1",
            params![task_id.to_string()],
            map_task_financials,
        ).optional().map_err(Into::into)
    }

    pub fn generate_report(&self, campaign_id: Uuid) -> Result<TreasuryReport> {
        let treasury = self.ensure_campaign(campaign_id)?;
        let totals = self.calculate_campaign_totals(campaign_id)?;
        let categories = self
            .calculate_category_totals(campaign_id)?
            .into_iter()
            .map(|item| CategoryReport {
                category_id: item.category.id,
                name: item.category.name,
                budget_minor: item.budget_minor,
                spending_minor: item.spending_minor,
                remaining_minor: item.remaining_minor,
            })
            .collect();
        let mut stmt = self.db.conn.prepare(
            "SELECT substr(COALESCE(payment_date, due_date, created_at), 1, 7), COALESCE(SUM(amount_minor), 0)
             FROM ledger_entries WHERE campaign_id=?1 AND entry_type='Expense' AND status='Paid'
             GROUP BY 1 ORDER BY 1",
        )?;
        let monthly_spending = stmt
            .query_map(params![campaign_id.to_string()], |row| {
                Ok(MonthlySpending {
                    month: row.get(0)?,
                    amount_minor: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let outstanding_payments = self.entries(
            campaign_id,
            &LedgerFilter {
                status: Some(LedgerStatus::Approved),
                entry_type: Some(LedgerEntryType::Expense),
                ..Default::default()
            },
            LedgerSort::DueDate,
        )?;
        Ok(TreasuryReport {
            campaign_id,
            generated_at: Utc::now(),
            currency_code: treasury.currency_code,
            totals: totals.into(),
            categories,
            monthly_spending,
            outstanding_payments,
        })
    }

    pub fn export_json(&self, campaign_id: Uuid) -> Result<String> {
        serde_json::to_string_pretty(&self.generate_report(campaign_id)?).map_err(Into::into)
    }

    pub fn export_csv(&self, campaign_id: Uuid) -> Result<String> {
        let entries = self.entries(campaign_id, &LedgerFilter::default(), LedgerSort::Newest)?;
        let mut output = String::from(
            "id,title,type,category_id,amount_minor,currency,status,due_date,payment_date,vendor_source,task_id,created_at,updated_at,recorded_by\n",
        );
        for entry in entries {
            let values = [
                entry.id.to_string(),
                entry.title,
                entry.entry_type.as_str().to_string(),
                entry.category_id.to_string(),
                entry.amount_minor.to_string(),
                entry.currency_code,
                entry.status.as_str().to_string(),
                entry.due_date.map(|v| v.to_rfc3339()).unwrap_or_default(),
                entry
                    .payment_date
                    .map(|v| v.to_rfc3339())
                    .unwrap_or_default(),
                entry.vendor_source.unwrap_or_default(),
                entry
                    .related_task_id
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
                entry.created_by_identity.unwrap_or_default(),
            ];
            output.push_str(
                &values
                    .into_iter()
                    .map(|value| csv_field(&value))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            output.push('\n');
        }
        Ok(output)
    }

    fn ensure_campaign_exists(&self, campaign_id: Uuid) -> Result<()> {
        let exists = self.db.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
            params![campaign_id.to_string()],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            bail!("Campaign does not exist");
        }
        Ok(())
    }

    fn validate_entry(&self, entry: &LedgerEntry) -> Result<()> {
        if entry.title.trim().is_empty() {
            bail!("Entry title cannot be empty");
        }
        if entry.amount_minor < 0 {
            bail!("Entry amount cannot be negative");
        }
        if entry.currency_code.trim().len() != 3 {
            bail!("Currency must use a three-letter code");
        }
        if entry.recurrence == LedgerRecurrence::Custom
            && entry
                .custom_recurrence
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            bail!("Custom recurrence requires a description");
        }
        if entry.status == LedgerStatus::Paid && entry.payment_date.is_none() {
            bail!("Paid entries require a payment date");
        }
        let category = self
            .get_category(entry.category_id)?
            .context("Category not found")?;
        if category.campaign_id != entry.campaign_id {
            bail!("Category does not belong to this campaign");
        }
        if let Some(task_id) = entry.related_task_id {
            let project_id: Option<String> = self
                .db
                .conn
                .query_row(
                    "SELECT project_id FROM tasks WHERE id=?1",
                    params![task_id.to_string()],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            if project_id.as_deref() != Some(entry.campaign_id.to_string().as_str()) {
                bail!("Related task does not belong to this campaign");
            }
        }
        Ok(())
    }

    fn get_category(&self, id: Uuid) -> Result<Option<LedgerCategory>> {
        self.db
            .conn
            .query_row(
                "SELECT id, campaign_id, name, is_default, version, created_at, updated_at
             FROM ledger_categories WHERE id=?1",
                params![id.to_string()],
                map_category,
            )
            .optional()
            .map_err(Into::into)
    }

    fn get_category_budget(&self, category_id: Uuid) -> Result<Option<CategoryBudget>> {
        self.db
            .conn
            .query_row(
                "SELECT category_id, campaign_id, amount_minor, version, created_at, updated_at
             FROM category_budgets WHERE category_id=?1",
                params![category_id.to_string()],
                map_category_budget,
            )
            .optional()
            .map_err(Into::into)
    }

    fn record_history(
        &self,
        campaign_id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        action: &str,
        version: i64,
    ) -> Result<()> {
        let snapshot = match entity_type {
            "ledger_entry" => self
                .get_entry(entity_id)?
                .and_then(|value| serde_json::to_string(&value).ok()),
            "ledger_category" => self
                .get_category(entity_id)?
                .and_then(|value| serde_json::to_string(&value).ok()),
            "category_budget" => self
                .get_category_budget(entity_id)?
                .and_then(|value| serde_json::to_string(&value).ok()),
            "campaign_treasury" => self
                .get_campaign(campaign_id)?
                .and_then(|value| serde_json::to_string(&value).ok()),
            _ => None,
        }
        .unwrap_or_else(|| "null".to_string());
        self.db.conn.execute(
            "INSERT INTO treasury_history (id, campaign_id, entity_type, entity_id, action, snapshot, version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![Uuid::new_v4().to_string(), campaign_id.to_string(), entity_type,
                entity_id.to_string(), action, snapshot, version, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn record_chronicle(&self, campaign_id: Uuid, content: &str) -> Result<()> {
        self.db.add_chronicle_message(
            &campaign_id.to_string(),
            "treasury",
            "Treasury",
            content,
            "treasury_event",
        )?;
        Ok(())
    }

    fn record_significant_transition(
        &self,
        previous: &LedgerEntry,
        current: &LedgerEntry,
    ) -> Result<()> {
        if previous.status != current.status {
            let message = match current.status {
                LedgerStatus::Approved if current.entry_type == LedgerEntryType::Expense => {
                    Some(format!("Expense approved: {}.", current.title))
                }
                LedgerStatus::Paid => Some(format!("Payment completed: {}.", current.title)),
                _ => None,
            };
            if let Some(message) = message {
                self.record_chronicle(current.campaign_id, &message)?;
            }
        }
        Ok(())
    }

    fn check_budget_events(&self, campaign_id: Uuid, entry: Option<&LedgerEntry>) -> Result<()> {
        let totals = self.calculate_campaign_totals(campaign_id)?;
        let usage = Self::budget_usage(
            totals.budget_minor,
            totals.paid_minor + totals.committed_minor,
        );
        if usage.warning == BudgetWarningLevel::Exceeded {
            let id = format!(
                "treasury:campaign_exceeded:{}:{}",
                campaign_id, totals.budget_minor
            );
            let exists = self.db.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM notifications WHERE id=?1)",
                params![id],
                |row| row.get::<_, bool>(0),
            )?;
            if !exists {
                self.db.create_notification_once(
                    &id,
                    "treasury_budget",
                    "Campaign budget exceeded",
                    "Paid and committed expenses exceed the campaign budget.",
                    Some(&campaign_id.to_string()),
                )?;
                self.record_chronicle(campaign_id, "Campaign budget exceeded.")?;
            }
        }
        if let Some(entry) = entry {
            let treasury = self.ensure_campaign(campaign_id)?;
            if entry.version == 1
                && entry.entry_type == LedgerEntryType::Expense
                && entry.amount_minor >= treasury.large_expense_threshold_minor
            {
                self.db.create_notification_once(
                    &format!("treasury:large:{}:{}", entry.id, entry.version),
                    "treasury_large_expense",
                    "Large expense recorded",
                    &entry.title,
                    Some(&entry.id.to_string()),
                )?;
            }
            if entry.version == 1
                && entry.entry_type == LedgerEntryType::Income
                && entry.status != LedgerStatus::Planned
            {
                self.db.create_notification_once(
                    &format!("treasury:income:{}:{}", entry.id, entry.version),
                    "treasury_income",
                    "Income received",
                    &entry.title,
                    Some(&entry.id.to_string()),
                )?;
            }
        }
        Ok(())
    }
}

fn ledger_select() -> &'static str {
    "SELECT id, campaign_id, title, description, entry_type, category_id, amount_minor,
            currency_code, status, due_date, payment_date, vendor_source, related_task_id,
            notes, attachment_ref, recurrence, custom_recurrence, version, created_at, updated_at,
            created_by_identity
     FROM ledger_entries"
}

fn map_campaign(row: &Row<'_>) -> rusqlite::Result<CampaignTreasury> {
    Ok(CampaignTreasury {
        campaign_id: parse_uuid(row, 0)?,
        overall_budget_minor: row.get(1)?,
        currency_code: row.get(2)?,
        large_expense_threshold_minor: row.get(3)?,
        version: row.get(4)?,
        created_at: parse_datetime(row, 5)?,
        updated_at: parse_datetime(row, 6)?,
    })
}

fn map_category(row: &Row<'_>) -> rusqlite::Result<LedgerCategory> {
    Ok(LedgerCategory {
        id: parse_uuid(row, 0)?,
        campaign_id: parse_uuid(row, 1)?,
        name: row.get(2)?,
        is_default: row.get::<_, i32>(3)? != 0,
        version: row.get(4)?,
        created_at: parse_datetime(row, 5)?,
        updated_at: parse_datetime(row, 6)?,
    })
}

fn map_category_budget(row: &Row<'_>) -> rusqlite::Result<CategoryBudget> {
    Ok(CategoryBudget {
        category_id: parse_uuid(row, 0)?,
        campaign_id: parse_uuid(row, 1)?,
        amount_minor: row.get(2)?,
        version: row.get(3)?,
        created_at: parse_datetime(row, 4)?,
        updated_at: parse_datetime(row, 5)?,
    })
}

fn map_entry(row: &Row<'_>) -> rusqlite::Result<LedgerEntry> {
    let entry_type: String = row.get(4)?;
    let status: String = row.get(8)?;
    let recurrence: String = row.get(15)?;
    Ok(LedgerEntry {
        id: parse_uuid(row, 0)?,
        campaign_id: parse_uuid(row, 1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        entry_type: LedgerEntryType::parse(&entry_type)
            .ok_or_else(|| conversion_error(4, "entry type"))?,
        category_id: parse_uuid(row, 5)?,
        amount_minor: row.get(6)?,
        currency_code: row.get(7)?,
        status: LedgerStatus::parse(&status).ok_or_else(|| conversion_error(8, "ledger status"))?,
        due_date: parse_optional_datetime(row, 9)?,
        payment_date: parse_optional_datetime(row, 10)?,
        vendor_source: row.get(11)?,
        related_task_id: parse_optional_uuid(row, 12)?,
        notes: row.get(13)?,
        attachment_ref: row.get(14)?,
        recurrence: LedgerRecurrence::parse(&recurrence)
            .ok_or_else(|| conversion_error(15, "recurrence"))?,
        custom_recurrence: row.get(16)?,
        version: row.get(17)?,
        created_at: parse_datetime(row, 18)?,
        updated_at: parse_datetime(row, 19)?,
        created_by_identity: row.get(20)?,
    })
}

fn map_task_financials(row: &Row<'_>) -> rusqlite::Result<TaskFinancials> {
    let payment: Option<String> = row.get(5)?;
    Ok(TaskFinancials {
        task_id: parse_uuid(row, 0)?,
        campaign_id: parse_uuid(row, 1)?,
        estimated_cost_minor: row.get(2)?,
        actual_cost_minor: row.get(3)?,
        billable_amount_minor: row.get(4)?,
        payment_status: payment.as_deref().and_then(TaskPaymentStatus::parse),
        currency_code: row.get(6)?,
        version: row.get(7)?,
        created_at: parse_datetime(row, 8)?,
        updated_at: parse_datetime(row, 9)?,
    })
}

fn parse_uuid(row: &Row<'_>, index: usize) -> rusqlite::Result<Uuid> {
    let value: String = row.get(index)?;
    Uuid::parse_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn parse_optional_uuid(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Uuid>> {
    row.get::<_, Option<String>>(index)?
        .map(|value| {
            Uuid::parse_str(&value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    index,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()
}

fn parse_datetime(row: &Row<'_>, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    let value: String = row.get(index)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn parse_optional_datetime(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<DateTime<Utc>>> {
    row.get::<_, Option<String>>(index)?
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        index,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
        })
        .transpose()
}

fn conversion_error(index: usize, field: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid {field}"),
        )),
    )
}

fn entry_params(entry: &LedgerEntry) -> rusqlite::ParamsFromIter<Vec<rusqlite::types::Value>> {
    use rusqlite::types::Value;
    params_from_values(vec![
        entry.id.to_string().into(),
        entry.campaign_id.to_string().into(),
        entry.title.clone().into(),
        entry.description.clone().into(),
        entry.entry_type.as_str().to_string().into(),
        entry.category_id.to_string().into(),
        entry.amount_minor.into(),
        entry.currency_code.clone().into(),
        entry.status.as_str().to_string().into(),
        entry
            .due_date
            .map(|v| v.to_rfc3339())
            .map_or(Value::Null, Into::into),
        entry
            .payment_date
            .map(|v| v.to_rfc3339())
            .map_or(Value::Null, Into::into),
        entry.vendor_source.clone().map_or(Value::Null, Into::into),
        entry
            .related_task_id
            .map(|v| v.to_string())
            .map_or(Value::Null, Into::into),
        entry.notes.clone().map_or(Value::Null, Into::into),
        entry.attachment_ref.clone().map_or(Value::Null, Into::into),
        entry.recurrence.as_str().to_string().into(),
        entry
            .custom_recurrence
            .clone()
            .map_or(Value::Null, Into::into),
        entry.version.into(),
        entry.created_at.to_rfc3339().into(),
        entry.updated_at.to_rfc3339().into(),
        entry
            .created_by_identity
            .clone()
            .map_or(Value::Null, Into::into),
    ])
}

fn params_from_values(
    values: Vec<rusqlite::types::Value>,
) -> rusqlite::ParamsFromIter<Vec<rusqlite::types::Value>> {
    rusqlite::params_from_iter(values)
}

fn matches_filter(entry: &LedgerEntry, filter: &LedgerFilter) -> bool {
    filter
        .category_id
        .is_none_or(|value| entry.category_id == value)
        && filter.status.is_none_or(|value| entry.status == value)
        && filter
            .entry_type
            .is_none_or(|value| entry.entry_type == value)
        && filter
            .task_id
            .is_none_or(|value| entry.related_task_id == Some(value))
        && filter
            .amount_min_minor
            .is_none_or(|value| entry.amount_minor >= value)
        && filter
            .amount_max_minor
            .is_none_or(|value| entry.amount_minor <= value)
        && filter.date_from.is_none_or(|value| {
            entry
                .due_date
                .or(entry.payment_date)
                .unwrap_or(entry.created_at)
                >= value
        })
        && filter.date_to.is_none_or(|value| {
            entry
                .due_date
                .or(entry.payment_date)
                .unwrap_or(entry.created_at)
                <= value
        })
        && filter.vendor.as_ref().is_none_or(|value| {
            entry
                .vendor_source
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&value.to_lowercase())
        })
        && filter.text.as_ref().is_none_or(|value| {
            let value = value.to_lowercase();
            entry.title.to_lowercase().contains(&value)
                || entry.description.to_lowercase().contains(&value)
                || entry
                    .notes
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&value)
                || entry
                    .vendor_source
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&value)
        })
}

fn sort_entries(entries: &mut [LedgerEntry], sort: LedgerSort) {
    entries.sort_by(|left, right| match sort {
        LedgerSort::Newest => right.created_at.cmp(&left.created_at),
        LedgerSort::Oldest => left.created_at.cmp(&right.created_at),
        LedgerSort::Largest => right.amount_minor.cmp(&left.amount_minor),
        LedgerSort::Smallest => left.amount_minor.cmp(&right.amount_minor),
        LedgerSort::DueDate => option_date_cmp(left.due_date, right.due_date),
        LedgerSort::PaymentDate => option_date_cmp(left.payment_date, right.payment_date),
    });
}

fn option_date_cmp(left: Option<DateTime<Utc>>, right: Option<DateTime<Utc>>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn deterministic_category_id(campaign_id: Uuid, name: &str) -> Uuid {
    let mut digest = Sha256::new();
    digest.update(campaign_id.as_bytes());
    digest.update(name.as_bytes());
    let hash = digest.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub fn format_minor(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let absolute = value.unsigned_abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}

/// Importe listo para pantalla: símbolo de la divisa y separadores de millares.
pub fn format_money(value_minor: i64, currency: Currency) -> String {
    let sign = if value_minor < 0 { "-" } else { "" };
    let absolute = value_minor.unsigned_abs();
    format!(
        "{sign}{}{}.{:02}",
        currency.symbol(),
        group_thousands(absolute / 100),
        absolute % 100
    )
}

fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub fn parse_minor(value: &str) -> Result<i64> {
    let value = value.trim().replace(',', "");
    if value.is_empty() || value.starts_with('-') {
        bail!("Amount must be zero or greater");
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("0");
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || fraction.len() > 2
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
    {
        bail!("Use an amount such as 1250.00");
    }
    let whole = whole.parse::<i64>()?;
    let cents = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>()? * 10,
        _ => fraction.parse::<i64>()?,
    };
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(cents))
        .context("Amount is too large")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Project;
    fn test_db() -> (std::path::PathBuf, Database, Uuid) {
        let file = std::env::temp_dir().join(format!("questline_treasury_{}.db", Uuid::new_v4()));
        let db = Database::new(&file).unwrap();
        let campaign_id = Uuid::new_v4();
        db.insert_project(&Project {
            id: campaign_id,
            name: "Test".into(),
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived: false,
            completed: false,
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        })
        .unwrap();
        (file, db, campaign_id)
    }

    fn entry(
        campaign_id: Uuid,
        category_id: Uuid,
        kind: LedgerEntryType,
        status: LedgerStatus,
        amount: i64,
    ) -> LedgerEntry {
        LedgerEntry {
            id: Uuid::new_v4(),
            campaign_id,
            title: "Entry".into(),
            description: String::new(),
            entry_type: kind,
            category_id,
            amount_minor: amount,
            currency_code: "USD".into(),
            status,
            due_date: None,
            payment_date: (status == LedgerStatus::Paid).then(Utc::now),
            vendor_source: None,
            related_task_id: None,
            notes: None,
            attachment_ref: None,
            recurrence: LedgerRecurrence::None,
            custom_recurrence: None,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by_identity: None,
        }
    }

    #[test]
    fn available_uses_confirmed_income_paid_and_committed() {
        let (_file, db, campaign_id) = test_db();
        let service = TreasuryService::new(&db);
        let category = service.categories(campaign_id).unwrap().remove(0);
        service
            .create_entry(entry(
                campaign_id,
                category.id,
                LedgerEntryType::Income,
                LedgerStatus::Paid,
                10_000,
            ))
            .unwrap();
        service
            .create_entry(entry(
                campaign_id,
                category.id,
                LedgerEntryType::Expense,
                LedgerStatus::Paid,
                2_500,
            ))
            .unwrap();
        service
            .create_entry(entry(
                campaign_id,
                category.id,
                LedgerEntryType::Expense,
                LedgerStatus::Approved,
                1_500,
            ))
            .unwrap();
        service
            .create_entry(entry(
                campaign_id,
                category.id,
                LedgerEntryType::Expense,
                LedgerStatus::Planned,
                8_000,
            ))
            .unwrap();
        let totals = service.calculate_campaign_totals(campaign_id).unwrap();
        assert_eq!(totals.available_minor, 6_000);
        assert_eq!(totals.paid_minor, 2_500);
        assert_eq!(totals.committed_minor, 1_500);
    }

    #[test]
    fn default_categories_are_idempotent() {
        let (_file, db, campaign_id) = test_db();
        let service = TreasuryService::new(&db);
        service.ensure_campaign(campaign_id).unwrap();
        service.ensure_campaign(campaign_id).unwrap();
        assert_eq!(
            service.categories(campaign_id).unwrap().len(),
            DEFAULT_CATEGORIES.len()
        );
    }

    #[test]
    fn budget_thresholds_are_exact() {
        assert_eq!(
            TreasuryService::budget_usage(10_000, 8_000).warning,
            BudgetWarningLevel::EightyPercent
        );
        assert_eq!(
            TreasuryService::budget_usage(10_000, 9_000).warning,
            BudgetWarningLevel::NinetyPercent
        );
        assert_eq!(
            TreasuryService::budget_usage(10_000, 10_000).warning,
            BudgetWarningLevel::Exhausted
        );
        assert_eq!(
            TreasuryService::budget_usage(10_000, 10_001).warning,
            BudgetWarningLevel::Exceeded
        );
    }

    #[test]
    fn currency_codes_round_trip() {
        assert_eq!(Currency::parse("mxn"), Some(Currency::Mxn));
        assert_eq!(Currency::parse(" USD "), Some(Currency::Usd));
        assert_eq!(Currency::parse("EUR"), None);
        assert_eq!(Currency::from_code_or_default("EUR"), Currency::Usd);
        for currency in Currency::ALL {
            assert_eq!(Currency::from_index(currency.index()), currency);
            assert_eq!(Currency::parse(currency.code()), Some(currency));
        }
    }

    #[test]
    fn money_is_formatted_with_symbol_and_groups() {
        assert_eq!(format_money(0, Currency::Usd), "US$0.00");
        assert_eq!(format_money(123_456_789, Currency::Mxn), "MX$1,234,567.89");
        assert_eq!(format_money(-99, Currency::Usd), "-US$0.99");
        assert_eq!(format_money(100_000, Currency::Mxn), "MX$1,000.00");
    }

    #[test]
    fn switching_currency_relabels_without_converting() {
        let (_file, db, campaign_id) = test_db();
        let service = TreasuryService::new(&db);
        let category = service.categories(campaign_id).unwrap().remove(0);
        let created = service
            .create_entry(entry(
                campaign_id,
                category.id,
                LedgerEntryType::Expense,
                LedgerStatus::Paid,
                2_500,
            ))
            .unwrap();
        assert_eq!(
            service.campaign_currency(campaign_id).unwrap(),
            Currency::Usd
        );

        service.set_currency(campaign_id, Currency::Mxn).unwrap();

        assert_eq!(
            service.campaign_currency(campaign_id).unwrap(),
            Currency::Mxn
        );
        let stored = service.get_entry(created.id).unwrap().unwrap();
        assert_eq!(stored.currency_code, "MXN");
        // El importe no se convierte: solo cambia la denominación con la que se lee.
        assert_eq!(stored.amount_minor, 2_500);
        assert!(stored.version > created.version);
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use crate::database::Database;

    /// Una base anterior a los permisos por rol no tiene `created_by_identity`. Al abrirla,
    /// la columna debe aparecer, la fila vieja seguir legible y su autoría quedar vacía —
    /// que es lo que deja esos movimientos en manos del Owner o del Steward.
    #[test]
    fn opening_a_pre_authorship_database_adds_the_column_and_keeps_the_rows() {
        let path = std::env::temp_dir().join(format!("questline_migration_{}.db", Uuid::new_v4()));
        let _ = std::fs::remove_file(&path);
        // Esquema tal como era antes de la autoría, con un movimiento ya asentado.
        let legacy = rusqlite::Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,
                     created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
                     archived INTEGER NOT NULL DEFAULT 0, completed INTEGER NOT NULL DEFAULT 0,
                     owner_identity TEXT, owner_username TEXT, is_shared INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE ledger_categories (id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL,
                     name TEXT NOT NULL, is_default INTEGER NOT NULL DEFAULT 0,
                     version INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL,
                     updated_at TEXT NOT NULL);
                 CREATE TABLE ledger_entries (id TEXT PRIMARY KEY, campaign_id TEXT NOT NULL,
                     title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
                     entry_type TEXT NOT NULL, category_id TEXT NOT NULL,
                     amount_minor INTEGER NOT NULL, currency_code TEXT NOT NULL DEFAULT 'USD',
                     status TEXT NOT NULL, due_date TEXT, payment_date TEXT, vendor_source TEXT,
                     related_task_id TEXT, notes TEXT, attachment_ref TEXT,
                     recurrence TEXT NOT NULL DEFAULT 'None', custom_recurrence TEXT,
                     version INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL,
                     updated_at TEXT NOT NULL);
                 INSERT INTO projects VALUES ('p1','Old Campaign',NULL,'2026-01-01T00:00:00+00:00',
                     '2026-01-01T00:00:00+00:00',0,0,NULL,NULL,0);
                 INSERT INTO ledger_categories VALUES ('cat1','p1','Other',1,1,
                     '2026-01-01T00:00:00+00:00','2026-01-01T00:00:00+00:00');
                 INSERT INTO ledger_entries VALUES ('e1','p1','Legacy expense','','Expense','cat1',
                     4200,'USD','Paid',NULL,'2026-01-02T00:00:00+00:00',NULL,NULL,NULL,NULL,
                     'None',NULL,1,'2026-01-02T00:00:00+00:00','2026-01-02T00:00:00+00:00');",
            )
            .unwrap();
        drop(legacy);

        // Abrir con la app actual dispara la migración.
        let db = Database::new(&path).unwrap();
        let has_column: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('ledger_entries')
                 WHERE name='created_by_identity'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_column, 1, "the migration must add created_by_identity");

        // La fila vieja se sigue leyendo por la ruta normal del servicio.
        let entry = TreasuryService::new(&db)
            .get_entry(Uuid::nil())
            .ok()
            .flatten();
        assert!(entry.is_none(), "a nil id must not resolve to a row");
        let (title, amount, author): (String, i64, Option<String>) = db
            .conn
            .query_row(
                "SELECT title, amount_minor, created_by_identity FROM ledger_entries WHERE id='e1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("the legacy row must survive the migration");
        assert_eq!(title, "Legacy expense");
        assert_eq!(amount, 4200);
        assert_eq!(author, None, "legacy rows have unknown authorship");

        // Y sin autor conocida, un Companion no puede tocarla.
        use crate::services::treasury_policy::{TreasuryAction, TreasuryRole, allows};
        assert!(!allows(
            TreasuryRole::Companion,
            TreasuryAction::EditEntry {
                mine: false,
                status: LedgerStatus::Paid
            }
        ));

        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
