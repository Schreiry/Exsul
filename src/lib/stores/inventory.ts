import { writable, derived } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { Item, CreateItemPayload, RecordSalePayload } from '$lib/tauri/types';

function createInventoryStore() {
	const { subscribe, set } = writable<Item[]>([]);

	async function load() {
		const items = await commands.getItems();
		set(items);
	}

	return {
		subscribe,
		load,

		async addItem(payload: CreateItemPayload): Promise<string> {
			const id = await commands.addItem(payload);
			await load();
			return id;
		},

		async recordSale(payload: RecordSalePayload) {
			await commands.recordSale(payload);
			await load();
		},

		async adjustStock(itemId: string, delta: number) {
			await commands.adjustStock({ item_id: itemId, delta });
			await load();
		},

		async changePrice(itemId: string, newPrice: number) {
			await commands.changePrice({ item_id: itemId, new_price: newPrice });
			await load();
		},

		async updateItem(itemId: string, updates: { name?: string; category?: string; production_cost?: number }) {
			await commands.updateItem({ item_id: itemId, ...updates });
			await load();
		},
	};
}

export const inventory = createInventoryStore();

export const totalStock = derived(inventory, ($items) =>
	$items.reduce((sum, item) => sum + item.current_stock, 0)
);

export const totalRevenue = derived(inventory, ($items) =>
	$items.reduce((sum, item) => sum + item.revenue, 0)
);

export const totalItems = derived(inventory, ($items) => $items.length);

export const categories = derived(inventory, ($items) =>
	[...new Set($items.map((item) => item.category))].sort()
);
