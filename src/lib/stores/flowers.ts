import { writable, derived, get } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type {
	FlowerSort,
	FlowerConstants,
	HarvestLogEntry,
	PackageResult,
	UpdateFlowerSortPayload,
	CreateFlowerSortPayload,
} from '$lib/tauri/types';

// ── Flower sorts ──────────────────────────────────────────────

function createFlowerSortsStore() {
	const { subscribe, set, update } = writable<FlowerSort[]>([]);

	return {
		subscribe,
		async load() {
			try {
				const sorts = await commands.getFlowerSorts();
				set(sorts);
			} catch (e) {
				console.error('Failed to load flower sorts', e);
			}
		},
		async create(payload: CreateFlowerSortPayload): Promise<string> {
			const id = await commands.createFlowerSort(payload);
			await this.load();
			return id;
		},
		async updateSort(payload: UpdateFlowerSortPayload) {
			await commands.updateFlowerSort(payload);
			update((sorts) =>
				sorts.map((s) =>
					s.id === payload.id
						? {
								...s,
								name: payload.name ?? s.name,
								variety: payload.variety ?? s.variety,
								color_hex: payload.color_hex ?? s.color_hex,
								raw_stock: payload.raw_stock ?? s.raw_stock,
								pkg_stock: payload.pkg_stock ?? s.pkg_stock,
								purchase_price: payload.purchase_price ?? s.purchase_price,
								sell_price_stem: payload.sell_price_stem ?? s.sell_price_stem,
								flowers_per_pack_override:
									payload.flowers_per_pack_override ?? s.flowers_per_pack_override,
								description: payload.description ?? s.description,
								photo_path: payload.photo_path ?? s.photo_path,
							}
						: s
				)
			);
		},
		async remove(id: string) {
			await commands.deleteFlowerSort(id);
			update((sorts) => sorts.filter((s) => s.id !== id));
		},
		async adjustStock(id: string, rawDelta: number, pkgDelta: number) {
			await commands.adjustFlowerStock(id, rawDelta, pkgDelta);
			await this.load();
		},
		async packageFlowers(sortId: string, packCount: number): Promise<PackageResult> {
			const result = await commands.packageFlowers(sortId, packCount);
			update((sorts) =>
				sorts.map((s) =>
					s.id === sortId
						? { ...s, raw_stock: result.new_raw_stock, pkg_stock: result.new_pkg_stock }
						: s
				)
			);
			return result;
		},
		// ── Greenhouse-specific ──
		async logHarvest(
			sortId: string,
			delta: number,
			reason: 'manual' | 'correction',
			note?: string
		): Promise<void> {
			await commands.logGreenhouseHarvest(sortId, delta, reason, note);
			// Reload to get updated raw_stock + total_harvested from DB
			await this.load();
		},
		async getHarvestLog(sortId?: string, limit?: number): Promise<HarvestLogEntry[]> {
			return commands.getHarvestLog(sortId, limit);
		},
		async savePhoto(sortId: string, sourcePath: string): Promise<string> {
			const path = await commands.saveFlowerPhoto(sortId, sourcePath);
			update((sorts) =>
				sorts.map((s) => (s.id === sortId ? { ...s, photo_path: path } : s))
			);
			return path;
		},
	};
}

export const flowerSorts = createFlowerSortsStore();

// ── Aggregate stats ───────────────────────────────────────────

export const totalRawStems = derived(flowerSorts, ($sorts) =>
	$sorts.reduce((sum, s) => sum + s.raw_stock, 0)
);

export const totalPacks = derived(flowerSorts, ($sorts) =>
	$sorts.reduce((sum, s) => sum + s.pkg_stock, 0)
);

// Group sorts by species name
export const sortsBySpecies = derived(flowerSorts, ($sorts) => {
	const map = new Map<string, FlowerSort[]>();
	for (const s of $sorts) {
		const group = map.get(s.name) ?? [];
		group.push(s);
		map.set(s.name, group);
	}
	return map;
});

// ── Constants ─────────────────────────────────────────────────

const DEFAULT_CONSTANTS: FlowerConstants = {
	weight_per_flower: 0.05,
	flowers_per_pack: 10,
	price_per_pack: 500,
	price_per_flower: 50,
	pricing_mode: 'pack',
};

function createConstantsStore() {
	const { subscribe, set, update } = writable<FlowerConstants>(DEFAULT_CONSTANTS);

	return {
		subscribe,
		async load() {
			try {
				const c = await commands.getFlowerConstants();
				set(c);
			} catch {
				set(DEFAULT_CONSTANTS);
			}
		},
		async save(c: FlowerConstants) {
			await commands.setFlowerConstants(c);
			set(c);
		},
	};
}

export const flowerConstants = createConstantsStore();

// ── Derived financials (depend on constants + sorts) ──────────

export const flowerFinancials = derived(
	[flowerSorts, flowerConstants],
	([$sorts, $c]) => {
		const totalRaw = $sorts.reduce((sum, s) => sum + s.raw_stock, 0);
		const totalPkg = $sorts.reduce((sum, s) => sum + s.pkg_stock, 0);
		const totalWeight = totalRaw * $c.weight_per_flower;
		const potentialPacks = $c.flowers_per_pack > 0
			? Math.floor(totalRaw / $c.flowers_per_pack)
			: 0;

		// Per-sort value calculation:
		// Each pack contains (flowers_per_pack_override ?? global flowers_per_pack) stems.
		// Pack value = pkg_stock * fpp * sell_price_stem
		// Raw value = raw_stock * sell_price_stem
		let packValue = 0;
		let rawValue = 0;
		let totalPurchaseValue = 0;

		for (const s of $sorts) {
			const fpp = s.flowers_per_pack_override ?? $c.flowers_per_pack;
			packValue += s.pkg_stock * fpp * s.sell_price_stem;
			rawValue += s.raw_stock * s.sell_price_stem;
			totalPurchaseValue += s.raw_stock * (s.purchase_price ?? 0)
				+ s.pkg_stock * fpp * (s.purchase_price ?? 0);
		}

		return {
			totalRaw,
			totalPkg,
			packValue,
			rawValue,
			totalValue: packValue + rawValue,
			totalWeight,
			potentialPacks,
			totalPurchaseValue,
		};
	}
);
