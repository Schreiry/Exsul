import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { Category, CreateCategoryPayload, UpdateCategoryPayload } from '$lib/tauri/types';

function createCategoryStore() {
	const { subscribe, set } = writable<Category[]>([]);

	async function load() {
		try {
			set(await commands.getCategories());
		} catch (e) {
			console.error('Failed to load categories:', e);
		}
	}

	return {
		subscribe,
		load,
		async create(payload: CreateCategoryPayload): Promise<string> {
			const id = await commands.createCategory(payload);
			await load();
			return id;
		},
		async update(payload: UpdateCategoryPayload): Promise<void> {
			await commands.updateCategory(payload);
			await load();
		},
		async remove(id: string): Promise<void> {
			await commands.deleteCategory(id);
			await load();
		},
	};
}

export const categories = createCategoryStore();
