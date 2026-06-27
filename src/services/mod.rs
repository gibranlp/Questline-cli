// ─────────────────────────────────────────────────────────────────────────────
// services/mod.rs — re-exports de los servicios
// ─────────────────────────────────────────────────────────────────────────────
pub mod api_client;
pub mod config;
pub mod identity;
pub mod logger;
pub mod sync_engine;
pub mod theme;
pub mod xp;

pub use api_client::ApiClient;
pub use config::Config;
pub use identity::Identity;
pub use logger::{init_panic_hook, log_structured};
pub use theme::ThemeService;
pub use xp::XPService;
