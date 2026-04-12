import { invoke } from '@tauri-apps/api/core';
import type {
	AddOrderItemPayload,
	AddTrustedNodePayload,
	AdjustStockPayload,
	AppPreset,
	AppSetting,
	AuditLog,
	AuditLogFilter,
	Category,
	ChangePricePayload,
	CreateCategoryPayload,
	CreateFlowerSortPayload,
	CreateItemPayload,
	CreateOrderPayload,
	CreatePackAssignmentPayload,
	EventRecord,
	FlowerConstants,
	FlowerSort,
	HarvestLogEntry,
	Item,
	Order,
	OrderItem,
	OrderShortage,
	PackAssignment,
	PackageResult,
	PackagingLogEntry,
	PackStatus,
	PriceRecord,
	RecordSalePayload,
	SyncPeer,
	TrustedNode,
	UpdateCategoryPayload,
	UpdateFlowerSortPayload,
	UpdateItemPayload,
	VersionInfo,
	WsServerStatus,
} from './types';

// ============================================================
// Environment detection
// ============================================================

function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// ============================================================
// In-memory mock store for browser / vite dev mode
// ============================================================

interface MockStore {
	items: Item[];
	categories: Category[];
	orders: Order[];
	auditLogs: AuditLog[];
}

const mockStore: MockStore = {
	items: [],
	categories: [],
	orders: [],
	auditLogs: [],
};

function mockUuid(): string {
	return 'mock-' + Math.random().toString(36).slice(2, 10);
}

function nowIso(): string {
	return new Date().toISOString();
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function getMock<T>(cmd: string, args?: Record<string, unknown>): T {
	switch (cmd) {
		// Inventory
		case 'get_items':
			return mockStore.items as unknown as T;
		case 'add_item': {
			const p = (args?.payload as CreateItemPayload) ?? ({} as CreateItemPayload);
			const id = mockUuid();
			mockStore.items.push({
				id,
				name: p.name ?? '',
				category: p.category ?? 'uncategorized',
				category_id: p.category_id,
				initial_price: p.price ?? 0,
				current_price: p.price ?? 0,
				production_cost: p.production_cost ?? 0,
				current_stock: p.initial_stock ?? 0,
				sold_count: 0,
				revenue: 0,
				created_at: nowIso(),
				updated_at: nowIso(),
			});
			return id as unknown as T;
		}
		case 'update_item': {
			const p = args?.payload as UpdateItemPayload;
			const idx = mockStore.items.findIndex((i) => i.id === p?.item_id);
			if (idx >= 0 && p) {
				if (p.name !== undefined) mockStore.items[idx].name = p.name;
				if (p.category !== undefined) mockStore.items[idx].category = p.category;
				if (p.category_id !== undefined) mockStore.items[idx].category_id = p.category_id;
				if (p.production_cost !== undefined)
					mockStore.items[idx].production_cost = p.production_cost;
				mockStore.items[idx].updated_at = nowIso();
			}
			return undefined as unknown as T;
		}
		case 'get_item': {
			const item = mockStore.items.find((i) => i.id === args?.itemId) ?? null;
			return item as unknown as T;
		}
		case 'record_sale':
		case 'adjust_stock':
		case 'change_price':
			return undefined as unknown as T;
		case 'save_item_image':
			return 'images/mock.img' as unknown as T;

		// Events
		case 'get_events':
			return [] as unknown as T;
		case 'get_price_history':
			return [] as unknown as T;

		// Sync
		case 'get_sync_state':
			return [] as unknown as T;
		case 'merge_remote_events':
			return 0 as unknown as T;
		case 'get_node_id':
			return 'browser-mock-node' as unknown as T;

		// Backup
		case 'export_backup':
			return '/mock/backup.bak' as unknown as T;
		case 'import_backup':
			return undefined as unknown as T;

		// Categories
		case 'get_categories':
			return mockStore.categories as unknown as T;
		case 'create_category': {
			const p = args?.payload as CreateCategoryPayload;
			const id = mockUuid();
			mockStore.categories.push({
				id,
				name: p?.name ?? '',
				color: p?.color,
				icon: p?.icon,
				created_at: nowIso(),
			});
			return id as unknown as T;
		}
		case 'update_category': {
			const p = args?.payload as UpdateCategoryPayload;
			const idx = mockStore.categories.findIndex((c) => c.id === p?.id);
			if (idx >= 0 && p) {
				if (p.name !== undefined) mockStore.categories[idx].name = p.name;
				if (p.color !== undefined) mockStore.categories[idx].color = p.color;
				if (p.icon !== undefined) mockStore.categories[idx].icon = p.icon;
			}
			return undefined as unknown as T;
		}
		case 'delete_category': {
			mockStore.categories = mockStore.categories.filter((c) => c.id !== args?.id);
			return undefined as unknown as T;
		}

		// Orders
		case 'get_orders': {
			const sf = args?.statusFilter as string | undefined;
			const result = sf ? mockStore.orders.filter((o) => o.status === sf) : mockStore.orders;
			return result as unknown as T;
		}
		case 'create_order': {
			const p = args?.payload as CreateOrderPayload;
			const id = mockUuid();
			mockStore.orders.push({
				id,
				customer_name: p?.customer_name ?? '',
				customer_email: p?.customer_email,
				customer_phone: p?.customer_phone,
				deadline: p?.deadline,
				status: 'pending',
				total_amount: 0,
				notes: p?.notes,
				pack_count_ordered: 0,
				pack_count_ready: 0,
				deadline_confirmed: false,
				created_at: nowIso(),
				updated_at: nowIso(),
			});
			return id as unknown as T;
		}
		case 'update_order_status': {
			const idx = mockStore.orders.findIndex((o) => o.id === args?.orderId);
			if (idx >= 0) {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(mockStore.orders[idx] as any).status = args?.status;
				mockStore.orders[idx].updated_at = nowIso();
			}
			return undefined as unknown as T;
		}
		case 'get_order': {
			const order = mockStore.orders.find((o) => o.id === args?.orderId) ?? null;
			return order as unknown as T;
		}
		case 'add_order_item':
			return mockUuid() as unknown as T;
		case 'get_order_items':
			return [] as unknown as T;

		// Audit
		case 'get_audit_logs':
			return mockStore.auditLogs as unknown as T;

		default:
			return undefined as unknown as T;
	}
}

async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	if (isTauri()) {
		return invoke<T>(cmd, args);
	}
	return getMock<T>(cmd, args);
}

// ============================================================
// Command surface
// ============================================================

export const commands = {
	// Inventory
	addItem: (payload: CreateItemPayload) =>
		safeInvoke<string>('add_item', { payload }),
	updateItem: (payload: UpdateItemPayload) =>
		safeInvoke<void>('update_item', { payload }),
	getItems: () => safeInvoke<Item[]>('get_items'),
	getItem: (itemId: string) => safeInvoke<Item | null>('get_item', { itemId }),
	recordSale: (payload: RecordSalePayload) =>
		safeInvoke<void>('record_sale', { payload }),
	adjustStock: (payload: AdjustStockPayload) =>
		safeInvoke<void>('adjust_stock', { payload }),
	changePrice: (payload: ChangePricePayload) =>
		safeInvoke<void>('change_price', { payload }),
	saveItemImage: (itemId: string, base64Data: string) =>
		safeInvoke<string>('save_item_image', { itemId, base64Data }),

	// Events
	getEvents: (since?: string, limit?: number) =>
		safeInvoke<EventRecord[]>('get_events', { since, limit }),
	getPriceHistory: (itemId: string) =>
		safeInvoke<PriceRecord[]>('get_price_history', { itemId }),

	// Sync
	getSyncState: () => safeInvoke<SyncPeer[]>('get_sync_state'),
	mergeRemoteEvents: (events: EventRecord[]) =>
		safeInvoke<number>('merge_remote_events', { events }),
	getNodeId: () => safeInvoke<string>('get_node_id'),

	// Backup
	exportBackup: () => safeInvoke<string>('export_backup'),
	importBackup: (path: string) => safeInvoke<void>('import_backup', { path }),
	importBackupData: (data: number[]) => safeInvoke<void>('import_backup_data', { data }),

	// Categories
	createCategory: (payload: CreateCategoryPayload) =>
		safeInvoke<string>('create_category', { payload }),
	getCategories: () => safeInvoke<Category[]>('get_categories'),
	updateCategory: (payload: UpdateCategoryPayload) =>
		safeInvoke<void>('update_category', { payload }),
	deleteCategory: (id: string) => safeInvoke<void>('delete_category', { id }),

	// Orders
	createOrder: (payload: CreateOrderPayload) =>
		safeInvoke<string>('create_order', { payload }),
	updateOrderStatus: (orderId: string, status: string) =>
		safeInvoke<void>('update_order_status', { orderId, status }),
	getOrders: (statusFilter?: string) =>
		safeInvoke<Order[]>('get_orders', { statusFilter }),
	getOrder: (orderId: string) =>
		safeInvoke<Order | null>('get_order', { orderId }),
	addOrderItem: (payload: AddOrderItemPayload) =>
		safeInvoke<string>('add_order_item', { payload }),
	getOrderItems: (orderId: string) =>
		safeInvoke<OrderItem[]>('get_order_items', { orderId }),

	// Audit
	getAuditLogs: (filter?: AuditLogFilter) =>
		safeInvoke<AuditLog[]>('get_audit_logs', { filter }),

	// App Preset
	getAppPreset: () => safeInvoke<AppPreset>('get_app_preset'),
	setAppPreset: (preset: AppPreset) => safeInvoke<void>('set_app_preset', { preset }),

	// Trusted Nodes
	getTrustedNodes: () => safeInvoke<TrustedNode[]>('get_trusted_nodes'),
	addTrustedNode: (payload: AddTrustedNodePayload) =>
		safeInvoke<void>('add_trusted_node', { payload }),
	removeTrustedNode: (nodeId: string) =>
		safeInvoke<void>('remove_trusted_node', { nodeId }),

	// Flower Sorts
	getFlowerSorts: () => safeInvoke<FlowerSort[]>('get_flower_sorts'),
	createFlowerSort: (payload: CreateFlowerSortPayload) =>
		safeInvoke<string>('create_flower_sort', { payload }),
	updateFlowerSort: (payload: UpdateFlowerSortPayload) =>
		safeInvoke<void>('update_flower_sort', { payload }),
	deleteFlowerSort: (id: string) => safeInvoke<void>('delete_flower_sort', { id }),
	adjustFlowerStock: (id: string, rawDelta: number, pkgDelta: number) =>
		safeInvoke<void>('adjust_flower_stock', { id, rawDelta, pkgDelta }),

	// Flower Constants
	getFlowerConstants: () => safeInvoke<FlowerConstants>('get_flower_constants'),
	setFlowerConstants: (constants: FlowerConstants) =>
		safeInvoke<void>('set_flower_constants', { constants }),

	// Flower ERP — Packaging
	packageFlowers: (sortId: string, packCount: number) =>
		safeInvoke<PackageResult>('package_flowers', { sortId, packCount }),
	getPackagingLog: (limit?: number) =>
		safeInvoke<PackagingLogEntry[]>('get_packaging_log', { limit }),

	// WebSocket P2P
	startWsServer: () => safeInvoke<void>('start_ws_server'),
	wsConnectPeer: (targetIp: string) => safeInvoke<number>('ws_connect_peer', { targetIp }),
	getWsStatus: () => safeInvoke<WsServerStatus>('get_ws_status'),

	// App Version
	getAppVersion: () => safeInvoke<string>('get_app_version'),
	getVersionInfo: () => safeInvoke<VersionInfo>('get_version_info'),

	// Inventory — delete & duplicate
	deleteItem: (itemId: string) => safeInvoke<void>('delete_item', { itemId }),
	duplicateItem: (itemId: string) => safeInvoke<string>('duplicate_item', { itemId }),
	deleteAllItems: () => safeInvoke<number>('delete_all_items'),

	// Pack Assignments
	createPackAssignment: (payload: CreatePackAssignmentPayload) =>
		safeInvoke<string>('create_pack_assignment', { payload }),
	getPackAssignments: (orderId?: string) =>
		safeInvoke<PackAssignment[]>('get_pack_assignments', { orderId }),
	updatePackStatus: (id: string, status: PackStatus) =>
		safeInvoke<void>('update_pack_status', { id, status }),

	// Greenhouse
	saveFlowerPhoto: (sortId: string, sourcePath: string) =>
		safeInvoke<string>('save_flower_photo', { sortId, sourcePath }),
	logGreenhouseHarvest: (
		sortId: string,
		delta: number,
		reason: string,
		note?: string
	) => safeInvoke<void>('log_greenhouse_harvest', { sortId, delta, reason, note }),
	getHarvestLog: (sortId?: string, limit?: number) =>
		safeInvoke<HarvestLogEntry[]>('get_harvest_log', { sortId, limit }),

	// Orders extended
	updateOrderExtended: (
		orderId: string,
		customerCompany?: string,
		deliveryAddress?: string,
		deliveryNotes?: string,
		packCountOrdered?: number
	) =>
		safeInvoke<void>('update_order_extended', {
			orderId,
			customerCompany,
			deliveryAddress,
			deliveryNotes,
			packCountOrdered,
		}),
	confirmOrderDeadline: (orderId: string) =>
		safeInvoke<void>('confirm_order_deadline', { orderId }),
	getOverdueUnconfirmedOrders: () =>
		safeInvoke<Order[]>('get_overdue_unconfirmed_orders'),
	checkOrderShortages: () =>
		safeInvoke<OrderShortage[]>('check_order_shortages'),

	// App Settings
	getSetting: (key: string) => safeInvoke<string | null>('get_setting', { key }),
	setSetting: (key: string, value: string) =>
		safeInvoke<void>('set_setting', { key, value }),
	getAllSettings: () => safeInvoke<AppSetting[]>('get_all_settings'),
} as const;
