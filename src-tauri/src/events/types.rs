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
