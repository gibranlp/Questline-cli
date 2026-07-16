// ─────────────────────────────────────────────────────────────────────────────
// services/mod.rs — re-exports de los servicios
// ─────────────────────────────────────────────────────────────────────────────
pub mod api_client;
pub mod bonsai;
pub mod config;
pub mod identity;
pub mod logger;
pub mod lore_manager;
pub mod planner;
pub mod sync_engine;
pub mod theme;
pub mod xp;

pub use api_client::ApiClient;
pub use config::Config;
pub use identity::Identity;
pub use logger::{init_panic_hook, log_structured};
pub use theme::ThemeService;
pub use xp::XPService;
