import type { OrderItem, FlowerSort, FlowerConstants, Item } from '$lib/tauri/types';

export interface LineCalc {
	packCount: number;
	stemsPerPack: number;
	pricePerPack: number;
	pricePerStem: number;
	lineTotal: number;
}

export function resolveItemName(
	item: OrderItem,
	flowerSorts: FlowerSort[],
	inventoryItems: Item[]
): { name: string; variety?: string } {
	// Prefer the explicit sort_id link (reliable since migration 014).
	// Fall back to item_id for legacy data where the link was never written.
	const sort = flowerSorts.find((s) => s.id === item.sort_id || s.id === item.item_id);
	if (sort) return { name: sort.name, variety: sort.variety };
	const inv = inventoryItems.find((i) => i.id === item.item_id);
	return { name: inv?.name ?? item.item_id };
}

export function findSortForItem(
	item: OrderItem,
	flowerSorts: FlowerSort[]
): FlowerSort | undefined {
	return flowerSorts.find((s) => s.id === item.sort_id || s.id === item.item_id);
}

export function computeLine(
	item: OrderItem,
	sort: FlowerSort | undefined,
	constants: FlowerConstants
): LineCalc {
	const fallbackStems =
		sort?.flowers_per_pack_override ?? constants.flowers_per_pack ?? 1;
	const stemsPerPack =
		item.stems_per_pack && item.stems_per_pack > 0 ? item.stems_per_pack : fallbackStems;
	const packCount =
		item.pack_count && item.pack_count > 0 ? item.pack_count : item.quantity;

	// unit_price is "price per pack" (AddItemModal stores pricePerPack).
	// Fallback: derive from sort.sell_price_stem * stemsPerPack when missing.
	let pricePerPack = item.unit_price;
	if ((!pricePerPack || pricePerPack <= 0) && sort) {
		pricePerPack = (sort.sell_price_stem ?? 0) * stemsPerPack;
	}
	const pricePerStem = stemsPerPack > 0 ? pricePerPack / stemsPerPack : 0;
	const lineTotal = packCount * pricePerPack;
	return { packCount, stemsPerPack, pricePerPack, pricePerStem, lineTotal };
}
