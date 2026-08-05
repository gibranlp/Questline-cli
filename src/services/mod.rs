// ─────────────────────────────────────────────────────────────────────────────
// services/mod.rs — re-exports de los servicios
// ─────────────────────────────────────────────────────────────────────────────
pub mod api_client;
pub mod bonsai;
pub mod config;
pub mod credential_vault;
pub mod encryption;
pub mod identity;
pub mod logger;
pub mod lore_manager;
pub mod notifications;
pub mod planner;
pub mod sync_engine;
pub mod task_notifications;
pub mod theme;
pub mod treasury;
pub mod treasury_notifications;
pub mod treasury_policy;
pub mod xp;

pub use api_client::ApiClient;
pub use config::Config;
pub use identity::Identity;
pub use logger::{init_panic_hook, log_structured};
pub use theme::ThemeService;
pub use treasury::TreasuryService;
pub use xp::XPService;
