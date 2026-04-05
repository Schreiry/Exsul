import { writable, derived, get } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { FlowerSort, FlowerConstants, UpdateFlowerSortPayload } from '$lib/tauri/types';

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
		async create(name: string, variety?: string, colorHex?: string) {
			const id = await commands.createFlowerSort({ name, variety, color_hex: colorHex });
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
		const packValue = totalPkg * $c.price_per_pack;
		const rawValue = totalRaw * $c.price_per_flower;
		const totalWeight = totalRaw * $c.weight_per_flower;
		const potentialPacks = $c.flowers_per_pack > 0
			? Math.floor(totalRaw / $c.flowers_per_pack)
			: 0;

		return {
			totalRaw,
			totalPkg,
			packValue,
			rawValue,
			totalValue: packValue + rawValue,
			totalWeight,
			potentialPacks,
		};
	}
);
