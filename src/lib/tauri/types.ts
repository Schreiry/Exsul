// ============================================================
// Core item types
// ============================================================

export interface Item {
	id: string;
	name: string;
	category: string;
	initial_price: number;
	current_price: number;
	production_cost: number;
	current_stock: number;
	sold_count: number;
	revenue: number;
	created_at: string;
	updated_at: string;
	category_id?: string;
	image_path?: string;
	card_color?: string;
}

export interface EventRecord {
	id: number | null;
	aggregate_id: string;
	aggregate_type: string;
	event_type: string;
	data: Record<string, unknown>;
	hlc_timestamp: string;
	node_id: string;
	version: number;
	created_at: string | null;
}

export interface PriceRecord {
	id: number;
	item_id: string;
	price: number;
	effective_at: string;
	event_id: number;
	created_at: string;
}

export interface SyncPeer {
	peer_node_id: string;
	last_hlc: string;
	last_event_id: number;
	last_synced_at: string;
}

// ============================================================
// Item payloads
// ============================================================

export interface CreateItemPayload {
	name: string;
	category?: string;
	category_id?: string;
	price: number;
	production_cost?: number;
	initial_stock?: number;
}

export interface UpdateItemPayload {
	item_id: string;
	name?: string;
	category?: string;
	category_id?: string;
	production_cost?: number;
	card_color?: string;
}

export interface RecordSalePayload {
	item_id: string;
	quantity: number;
	sale_price?: number;
}

export interface AdjustStockPayload {
	item_id: string;
	delta: number;
}

export interface ChangePricePayload {
	item_id: string;
	new_price: number;
}

// ============================================================
// Categories
// ============================================================

export interface Category {
	id: string;
	name: string;
	color?: string;
	icon?: string;
	created_at: string;
}

export interface CreateCategoryPayload {
	name: string;
	color?: string;
	icon?: string;
}

export interface UpdateCategoryPayload {
	id: string;
	name?: string;
	color?: string;
	icon?: string;
}

// ============================================================
// Orders
// ============================================================

export type OrderStatus = 'pending' | 'in_progress' | 'completed' | 'cancelled';

export interface Order {
	id: string;
	customer_name: string;
	customer_email?: string;
	customer_phone?: string;
	deadline?: string;
	status: OrderStatus;
	total_amount: number;
	notes?: string;
	created_at: string;
	updated_at: string;
	// Extended fields (migration 011)
	customer_company?: string;
	delivery_address?: string;
	delivery_notes?: string;
	pack_count_ordered: number;
	pack_count_ready: number;
	deadline_confirmed: boolean;
}

export interface OrderItem {
	id: string;
	order_id: string;
	item_id: string;
	quantity: number;
	unit_price: number;
	specifications: Record<string, unknown>;
	created_at: string;
	pack_count?: number;
	stems_per_pack?: number;
	sort_id?: string;
}

export interface CreateOrderPayload {
	customer_name: string;
	customer_email?: string;
	customer_phone?: string;
	deadline?: string;
	notes?: string;
}

export interface AddOrderItemPayload {
	order_id: string;
	item_id: string;
	quantity: number;
	unit_price: number;
	specifications?: Record<string, unknown>;
	pack_count?: number;
	stems_per_pack?: number;
	sort_id?: string;
}

// ============================================================
// Audit Logs
// ============================================================

export interface AuditLog {
	id: number;
	timestamp: string;
	user_id: string;
	action: string;
	payload: Record<string, unknown>;
	ip_address?: string;
	session_id?: string;
}

export interface AuditLogFilter {
	user_id?: string;
	action?: string;
	since?: string;
	until?: string;
	limit?: number;
}

// ============================================================
// App Preset
// ============================================================

export type AppPreset = 'flowers' | 'ochokochi' | 'balanced';

// ============================================================
// Trusted Nodes
// ============================================================

export interface TrustedNode {
	node_id: string;
	alias?: string;
	ip_hint?: string;
	added_at: string;
}

export interface AddTrustedNodePayload {
	node_id: string;
	alias?: string;
	ip_hint?: string;
}

// ============================================================
// Flower Sorts
// ============================================================

export interface FlowerSort {
	id: string;
	name: string;
	variety?: string;
	color_hex?: string;
	raw_stock: number;
	pkg_stock: number;
	purchase_price: number;
	sell_price_stem: number;
	flowers_per_pack_override?: number;
	created_at: string;
	updated_at: string;
	// Extended fields (migration 010)
	photo_path?: string;
	description?: string;
	total_harvested: number;
}

export interface CreateFlowerSortPayload {
	name: string;
	variety?: string;
	color_hex?: string;
	purchase_price?: number;
	sell_price_stem?: number;
	flowers_per_pack_override?: number;
	description?: string;
}

export interface UpdateFlowerSortPayload {
	id: string;
	name?: string;
	variety?: string;
	color_hex?: string;
	raw_stock?: number;
	pkg_stock?: number;
	purchase_price?: number;
	sell_price_stem?: number;
	flowers_per_pack_override?: number;
	description?: string;
	photo_path?: string;
}

export interface PackageResult {
	sort_id: string;
	new_raw_stock: number;
	new_pkg_stock: number;
	stems_used: number;
	packs_created: number;
}

export interface PackagingLogEntry {
	id: string;
	sort_id: string;
	sort_name: string;
	variety?: string;
	pack_count: number;
	stems_used: number;
	stems_per_pack: number;
	sell_price_stem: number;
	order_id?: string;
	created_at: string;
}

// ============================================================
// Flower Constants
// ============================================================

export type PricingMode = 'pack' | 'stem' | 'mixed';

export interface FlowerConstants {
	weight_per_flower: number;
	flowers_per_pack: number;
	price_per_pack: number;
	price_per_flower: number;
	pricing_mode: PricingMode;
}

// ============================================================
// WebSocket / P2P
// ============================================================

export type WsPeerState = 'connected' | 'connecting' | 'disconnected' | 'rejected';

export interface WsPeerStatus {
	node_id: string;
	alias?: string;
	ip: string;
	state: WsPeerState;
	last_sync?: string;
	events_merged: number;
	app_version?: string;
}

export interface WsServerStatus {
	running: boolean;
	port: number;
	peers: WsPeerStatus[];
}

// ============================================================
// Pack Assignments
// ============================================================

export type PackStatus = 'prepared' | 'loaded' | 'delivered';

export interface PackAssignment {
	id: string;
	sort_id: string;
	order_id?: string;
	pack_count: number;
	stems_per_pack: number;
	status: PackStatus;
	note?: string;
	created_at: string;
}

export interface CreatePackAssignmentPayload {
	sort_id: string;
	order_id?: string;
	pack_count: number;
	stems_per_pack: number;
	note?: string;
}

// ============================================================
// Greenhouse harvest log (migration 010)
// ============================================================

export type HarvestReason = 'manual' | 'packaged' | 'correction';

export interface HarvestLogEntry {
	id: string;
	sort_id: string;
	sort_name: string;
	delta: number;
	reason: HarvestReason;
	note?: string;
	created_at: string;
}

// ============================================================
// Order shortage (calculated on-the-fly)
// ============================================================

export interface OrderShortage {
	order_id: string;
	customer_name: string;
	sort_id: string;
	sort_name: string;
	ordered_packs: number;
	available_packs: number;
	shortage: number;
}

// ============================================================
// App settings (migration 012)
// ============================================================

export type SettingValueType = 'string' | 'number' | 'bool' | 'json';

export interface AppSetting {
	key: string;
	value: string;
	value_type: SettingValueType;
}

// ============================================================
// Version info
// ============================================================

export interface VersionInfo {
	app_version: string;
	db_schema_version: number;
	min_compatible_version: string;
}
