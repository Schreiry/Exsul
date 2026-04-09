use serde::{Deserialize, Serialize};

// ============================================================
// Core event sourcing types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: Option<i64>,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub hlc_timestamp: String,
    pub node_id: String,
    pub version: i64,
    pub created_at: Option<String>,
}

// ============================================================
// Item payloads
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateItemPayload {
    pub name: String,
    pub category: Option<String>,
    pub category_id: Option<String>,
    pub price: f64,
    pub production_cost: Option<f64>,
    pub initial_stock: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItemPayload {
    pub item_id: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub category_id: Option<String>,
    pub production_cost: Option<f64>,
    pub card_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSalePayload {
    pub item_id: String,
    pub quantity: i32,
    pub sale_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustStockPayload {
    pub item_id: String,
    pub delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePricePayload {
    pub item_id: String,
    pub new_price: f64,
}

// ============================================================
// Materialized projections
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub category: String,
    pub initial_price: f64,
    pub current_price: f64,
    pub production_cost: f64,
    pub current_stock: i32,
    pub sold_count: i32,
    pub revenue: f64,
    pub created_at: String,
    pub updated_at: String,
    pub category_id: Option<String>,
    pub image_path: Option<String>,
    pub card_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRecord {
    pub id: i64,
    pub item_id: String,
    pub price: f64,
    pub effective_at: String,
    pub event_id: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeer {
    pub peer_node_id: String,
    pub last_hlc: String,
    pub last_event_id: i64,
    pub last_synced_at: String,
}

// ============================================================
// Categories
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryPayload {
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryPayload {
    pub id: String,
    pub name: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

// ============================================================
// Orders
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub deadline: Option<String>,
    pub status: String,
    pub total_amount: f64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: String,
    pub order_id: String,
    pub item_id: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub specifications: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderPayload {
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub deadline: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddOrderItemPayload {
    pub order_id: String,
    pub item_id: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub specifications: Option<serde_json::Value>,
}

// ============================================================
// Audit Logs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub timestamp: String,
    pub user_id: String,
    pub action: String,
    pub payload: serde_json::Value,
    pub ip_address: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogFilter {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<i64>,
}

// ============================================================
// App Preset
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppPreset {
    Flowers,
    Ochokochi,
    Balanced,
}

impl AppPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppPreset::Flowers => "flowers",
            AppPreset::Ochokochi => "ochokochi",
            AppPreset::Balanced => "balanced",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "flowers" => AppPreset::Flowers,
            "ochokochi" => AppPreset::Ochokochi,
            _ => AppPreset::Balanced,
        }
    }
}

// ============================================================
// Trusted Nodes (P2P whitelist)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedNode {
    pub node_id: String,
    pub alias: Option<String>,
    pub ip_hint: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTrustedNodePayload {
    pub node_id: String,
    pub alias: Option<String>,
    pub ip_hint: Option<String>,
}

// ============================================================
// Flower Sorts
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowerSort {
    pub id: String,
    pub name: String,
    pub variety: Option<String>,
    pub color_hex: Option<String>,
    pub raw_stock: i32,
    pub pkg_stock: i32,
    pub purchase_price: f64,
    pub sell_price_stem: f64,
    pub flowers_per_pack_override: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFlowerSortPayload {
    pub name: String,
    pub variety: Option<String>,
    pub color_hex: Option<String>,
    pub purchase_price: Option<f64>,
    pub sell_price_stem: Option<f64>,
    pub flowers_per_pack_override: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFlowerSortPayload {
    pub id: String,
    pub name: Option<String>,
    pub variety: Option<String>,
    pub color_hex: Option<String>,
    pub raw_stock: Option<i32>,
    pub pkg_stock: Option<i32>,
    pub purchase_price: Option<f64>,
    pub sell_price_stem: Option<f64>,
    pub flowers_per_pack_override: Option<i32>,
}

// ============================================================
// Flower ERP — Packaging
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageResult {
    pub sort_id: String,
    pub new_raw_stock: i32,
    pub new_pkg_stock: i32,
    pub stems_used: i32,
    pub packs_created: i32,
}

// ============================================================
// Flower Constants
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowerConstants {
    pub weight_per_flower: f64,
    pub flowers_per_pack: f64,
    pub price_per_pack: f64,
    pub price_per_flower: f64,
    #[serde(default = "default_pricing_mode")]
    pub pricing_mode: String,
}

fn default_pricing_mode() -> String {
    "pack".to_string()
}

// ============================================================
// Pack Assignments (Task 9)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackAssignment {
    pub id: String,
    pub sort_id: String,
    pub order_id: Option<String>,
    pub pack_count: i32,
    pub stems_per_pack: i32,
    pub status: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePackAssignmentPayload {
    pub sort_id: String,
    pub order_id: Option<String>,
    pub pack_count: i32,
    pub stems_per_pack: i32,
    pub note: Option<String>,
}

// ============================================================
// WebSocket / P2P status
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WsPeerState {
    Connected,
    Connecting,
    Disconnected,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsPeerStatus {
    pub node_id: String,
    pub alias: Option<String>,
    pub ip: String,
    pub state: WsPeerState,
    pub last_sync: Option<String>,
    pub events_merged: u32,
    pub app_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsServerStatus {
    pub running: bool,
    pub port: u16,
    pub peers: Vec<WsPeerStatus>,
}
