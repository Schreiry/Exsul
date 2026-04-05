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
}

export interface OrderItem {
	id: string;
	order_id: string;
	item_id: string;
	quantity: number;
	unit_price: number;
	specifications: Record<string, unknown>;
	created_at: string;
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
