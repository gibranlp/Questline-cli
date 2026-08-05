use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Divisa con la que trabaja una campaña. No se convierte entre divisas:
/// el usuario elige la denominación y todos los importes se leen en ella.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    #[default]
    Usd,
    Mxn,
}

impl Currency {
    pub const ALL: [Self; 2] = [Self::Usd, Self::Mxn];

    pub fn code(self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Mxn => "MXN",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Usd => "US$",
            Self::Mxn => "MX$",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Usd => "USD · US Dollar",
            Self::Mxn => "MXN · Mexican Peso",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "USD" => Some(Self::Usd),
            "MXN" => Some(Self::Mxn),
            _ => None,
        }
    }

    /// Las filas heredadas o de otros dispositivos pueden traer códigos desconocidos:
    /// se muestran como USD en lugar de romper la pantalla.
    pub fn from_code_or_default(value: &str) -> Self {
        Self::parse(value).unwrap_or_default()
    }

    pub fn index(self) -> usize {
        match self {
            Self::Usd => 0,
            Self::Mxn => 1,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerEntryType {
    Income,
    Expense,
    Transfer,
    Adjustment,
}

impl LedgerEntryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "Income",
            Self::Expense => "Expense",
            Self::Transfer => "Transfer",
            Self::Adjustment => "Adjustment",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Income" => Some(Self::Income),
            "Expense" => Some(Self::Expense),
            "Transfer" => Some(Self::Transfer),
            "Adjustment" => Some(Self::Adjustment),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerStatus {
    Planned,
    Approved,
    Paid,
    Cancelled,
}

impl LedgerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::Approved => "Approved",
            Self::Paid => "Paid",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Planned" => Some(Self::Planned),
            "Approved" => Some(Self::Approved),
            "Paid" => Some(Self::Paid),
            "Cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Un movimiento sigue siendo del Companion que lo asentó mientras está en Planned.
    /// Al aprobarse, cancelarse o pagarse pasa a ser cosa del Owner o del Steward.
    pub fn is_open(self) -> bool {
        matches!(self, Self::Planned)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerRecurrence {
    None,
    Weekly,
    Monthly,
    Yearly,
    Custom,
}

impl LedgerRecurrence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
            Self::Custom => "Custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "None" => Some(Self::None),
            "Weekly" => Some(Self::Weekly),
            "Monthly" => Some(Self::Monthly),
            "Yearly" => Some(Self::Yearly),
            "Custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPaymentStatus {
    NotBillable,
    Unbilled,
    Invoiced,
    Paid,
}

impl TaskPaymentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotBillable => "NotBillable",
            Self::Unbilled => "Unbilled",
            Self::Invoiced => "Invoiced",
            Self::Paid => "Paid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "NotBillable" => Some(Self::NotBillable),
            "Unbilled" => Some(Self::Unbilled),
            "Invoiced" => Some(Self::Invoiced),
            "Paid" => Some(Self::Paid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignTreasury {
    pub campaign_id: Uuid,
    pub overall_budget_minor: i64,
    pub currency_code: String,
    pub large_expense_threshold_minor: i64,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerCategory {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub is_default: bool,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryBudget {
    pub category_id: Uuid,
    pub campaign_id: Uuid,
    pub amount_minor: i64,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub title: String,
    pub description: String,
    pub entry_type: LedgerEntryType,
    pub category_id: Uuid,
    pub amount_minor: i64,
    pub currency_code: String,
    pub status: LedgerStatus,
    pub due_date: Option<DateTime<Utc>>,
    pub payment_date: Option<DateTime<Utc>>,
    pub vendor_source: Option<String>,
    pub related_task_id: Option<Uuid>,
    pub notes: Option<String>,
    pub attachment_ref: Option<String>,
    pub recurrence: LedgerRecurrence,
    pub custom_recurrence: Option<String>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Companion Key de quien asentó el movimiento. Los eventos de clientes anteriores
    /// llegan sin este campo, así que se trata como autoría desconocida.
    #[serde(default)]
    pub created_by_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFinancials {
    pub task_id: Uuid,
    pub campaign_id: Uuid,
    pub estimated_cost_minor: Option<i64>,
    pub actual_cost_minor: Option<i64>,
    pub billable_amount_minor: Option<i64>,
    pub payment_status: Option<TaskPaymentStatus>,
    pub currency_code: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CampaignTotals {
    pub budget_minor: i64,
    pub income_minor: i64,
    pub paid_minor: i64,
    pub committed_minor: i64,
    pub available_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTotals {
    pub category: LedgerCategory,
    pub budget_minor: Option<i64>,
    pub spending_minor: i64,
    pub remaining_minor: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerSort {
    Newest,
    Oldest,
    Largest,
    Smallest,
    DueDate,
    PaymentDate,
}

#[derive(Debug, Clone, Default)]
pub struct LedgerFilter {
    pub category_id: Option<Uuid>,
    pub status: Option<LedgerStatus>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub vendor: Option<String>,
    pub task_id: Option<Uuid>,
    pub amount_min_minor: Option<i64>,
    pub amount_max_minor: Option<i64>,
    pub entry_type: Option<LedgerEntryType>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySpending {
    pub month: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryReport {
    pub campaign_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub currency_code: String,
    pub totals: CampaignTotalsReport,
    pub categories: Vec<CategoryReport>,
    pub monthly_spending: Vec<MonthlySpending>,
    pub outstanding_payments: Vec<LedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignTotalsReport {
    pub budget_minor: i64,
    pub income_minor: i64,
    pub paid_minor: i64,
    pub committed_minor: i64,
    pub available_minor: i64,
}

impl From<CampaignTotals> for CampaignTotalsReport {
    fn from(value: CampaignTotals) -> Self {
        Self {
            budget_minor: value.budget_minor,
            income_minor: value.income_minor,
            paid_minor: value.paid_minor,
            committed_minor: value.committed_minor,
            available_minor: value.available_minor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryReport {
    pub category_id: Uuid,
    pub name: String,
    pub budget_minor: Option<i64>,
    pub spending_minor: i64,
    pub remaining_minor: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetWarningLevel {
    Healthy,
    EightyPercent,
    NinetyPercent,
    Exhausted,
    Exceeded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetUsage {
    pub budget_minor: i64,
    pub used_minor: i64,
    pub ratio: f64,
    pub warning: BudgetWarningLevel,
}
