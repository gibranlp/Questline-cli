use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A named mind map inside a Campaign's Atlas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasChart {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[serde(default = "crate::models::default_sync_timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub owner_identity: Option<String>,
}

/// One independently-synchronised branch point in an Atlas Chart.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasNode {
    pub id: Uuid,
    pub chart_id: Uuid,
    /// Duplicated intentionally so encrypted sync can route a node without
    /// looking through a Chart that may not have arrived on this device yet.
    pub project_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub position: i64,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub marked: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub color: Option<u8>,
    pub created_at: DateTime<Utc>,
    #[serde(default = "crate::models::default_sync_timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub owner_identity: Option<String>,
}
