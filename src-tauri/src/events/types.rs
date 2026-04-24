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
    // Extended fields (migration 011)
    pub customer_company: Option<String>,
    pub delivery_address: Option<String>,
    pub delivery_notes: Option<String>,
    pub pack_count_ordered: i32,
    pub pack_count_ready: i32,
    pub deadline_confirmed: bool,
    // Personalization (migration 016) — HEX color (#rrggbb) used as
    // left-edge accent and light background tint on the order card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_color: Option<String>,
    // Contact linkage (migration 017) — optional. Free-form customer_name/
    // email/phone stays as-is for legacy orders; contact_id is a soft FK
    // (no REFERENCES) so a contact that hasn't synced yet doesn't block the
    // order projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrderPayload {
    pub order_id: String,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_company: Option<String>,
    pub delivery_address: Option<String>,
    pub delivery_notes: Option<String>,
    pub deadline: Option<String>,
    pub notes: Option<String>,
    pub card_color: Option<String>,
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default)]
    pub contact_location_id: Option<String>,
    // Tri-state clears: when `true`, the corresponding field is set
    // to NULL in storage regardless of the Some/None value above.
    // This is how the UI says "remove the color" or "remove the
    // deadline" — Option::None alone means "don't touch this field".
    #[serde(default)]
    pub clear_card_color: bool,
    #[serde(default)]
    pub clear_deadline: bool,
    #[serde(default)]
    pub clear_contact: bool,
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
    #[serde(default)]
    pub pack_count: i32,
    #[serde(default)]
    pub stems_per_pack: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderPayload {
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub deadline: Option<String>,
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddOrderItemPayload {
    pub order_id: String,
    pub item_id: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub specifications: Option<serde_json::Value>,
    #[serde(default)]
    pub pack_count: Option<i32>,
    #[serde(default)]
    pub stems_per_pack: Option<i32>,
    #[serde(default)]
    pub sort_id: Option<String>,
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
    // Extended fields (migration 010)
    pub photo_path: Option<String>,
    pub description: Option<String>,
    pub total_harvested: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFlowerSortPayload {
    pub name: String,
    pub variety: Option<String>,
    pub color_hex: Option<String>,
    pub purchase_price: Option<f64>,
    pub sell_price_stem: Option<f64>,
    pub flowers_per_pack_override: Option<i32>,
    pub description: Option<String>,
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
    pub description: Option<String>,
    pub photo_path: Option<String>,
}

// ============================================================
// Flower ERP — Packaging
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagingLogEntry {
    pub id: String,
    pub sort_id: String,
    pub sort_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variety: Option<String>,
    pub pack_count: i32,
    pub stems_used: i32,
    // Derived fields — joined from flower_sorts so the frontend has
    // everything needed to render a print row without a second round-trip.
    #[serde(default)]
    pub stems_per_pack: i32,
    #[serde(default)]
    pub sell_price_stem: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub created_at: String,
}

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

/// Atomic payload for `package_flowers_with_order` — covers the full
/// warehouse→order chain (package, link, order_item, pack_assignment,
/// total recalculation) in a single transactional call.
/// Customer-side fields are optional: when none are set, the command
/// behaves as a plain packaging op with no order created.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageWithOrderPayload {
    pub sort_id: String,
    pub pack_count: i32,
    /// Price per pack. Frontend resolves this from the sort's sell_price_stem
    /// (or a manual override); backend uses it verbatim for the order_item.
    pub price_per_pack: f64,

    // Optional order/customer data. When `customer_name` is empty/None the
    // command skips the whole order-creation branch.
    #[serde(default)]
    pub customer_name: Option<String>,
    #[serde(default)]
    pub customer_email: Option<String>,
    #[serde(default)]
    pub customer_phone: Option<String>,
    #[serde(default)]
    pub delivery_address: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub card_color: Option<String>,
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default)]
    pub contact_location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageWithOrderResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub packaging_log_id: String,
    pub new_raw_stock: i32,
    pub new_pkg_stock: i32,
    pub stems_used: i32,
    pub packs_created: i32,
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

// ============================================================
// Greenhouse harvest log (migration 010)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestLogEntry {
    pub id: String,
    pub sort_id: String,
    pub sort_name: String,
    pub delta: i32,
    pub reason: String,
    pub note: Option<String>,
    pub created_at: String,
}

// ============================================================
// Order shortage (calculated on-the-fly, not stored in DB)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderShortage {
    pub order_id: String,
    pub customer_name: String,
    pub sort_id: String,
    pub sort_name: String,
    pub ordered_packs: i32,
    pub available_packs: i32,
    pub shortage: i32,
}

// Per-sort listing of orders that still need packs of that sort.
// Populated by a read-only aggregate query — not stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderWaitingForSort {
    pub order_id: String,
    pub customer_name: String,
    pub deadline: Option<String>,
    pub status: String,
    pub ordered_packs: i32,   // total pack_count across order_items for this sort
    pub reserved_packs: i32,  // pack_assignments.pack_count where status != 'delivered'
    pub shortage: i32,        // max(0, ordered_packs - reserved_packs)
    pub created_at: String,
}

// ============================================================
// App settings (migration 012)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
    pub value_type: String,
}

// ============================================================
// Contacts (migration 017) — Phase E
// ============================================================

/// A customer/contact record. `order_count` and `total_spent` are
/// aggregates computed on-the-fly by JOIN on orders; they are not
/// stored in the contacts table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub surname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
    pub photo_path: Option<String>,
    pub card_color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // Aggregates — populated in list/detail queries, default to 0 when
    // the contact has no orders yet.
    #[serde(default)]
    pub order_count: i64,
    #[serde(default)]
    pub total_spent: f64,
    // Default location shortcut — first row where is_default = 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactLocation {
    pub id: String,
    pub contact_id: String,
    pub label: Option<String>,
    pub address: String,
    pub is_default: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContactPayload {
    pub name: String,
    pub surname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
    pub card_color: Option<String>,
    /// Optional single default address created together with the contact.
    /// Nil → only the contacts row is created.
    pub default_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContactPayload {
    pub contact_id: String,
    pub name: Option<String>,
    pub surname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
    pub card_color: Option<String>,
    #[serde(default)]
    pub clear_card_color: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContactLocationPayload {
    pub contact_id: String,
    pub label: Option<String>,
    pub address: String,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContactLocationPayload {
    pub location_id: String,
    pub label: Option<String>,
    pub address: Option<String>,
}

// ============================================================
// Version info (extended)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub app_version: String,
    pub db_schema_version: i32,
    pub min_compatible_version: String,
}
