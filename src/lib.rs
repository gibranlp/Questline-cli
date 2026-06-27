// ─────────────────────────────────────────────────────────────────────────────
// lib.rs — el punto de entrada de la librería, re-exporta todos los módulos
// ─────────────────────────────────────────────────────────────────────────────
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::should_implement_trait,
    clippy::needless_range_loop,
    clippy::unnecessary_sort_by,
    clippy::redundant_pattern_matching,
    clippy::if_same_then_else
)]

pub mod app;
pub mod audio;
pub mod database;
pub mod milestone_templates;
pub mod models;
pub mod screens;
pub mod services;
pub mod storage;
pub mod theme;
pub mod ui;
