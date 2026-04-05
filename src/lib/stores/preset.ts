import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { AppPreset } from '$lib/tauri/types';

function createPresetStore() {
	const { subscribe, set } = writable<AppPreset>('balanced');

	return {
		subscribe,
		async load() {
			try {
				const p = await commands.getAppPreset();
				set(p as AppPreset);
			} catch {
				set('balanced');
			}
		},
		async switchTo(p: AppPreset) {
			await commands.setAppPreset(p);
			set(p);
		},
	};
}

export const preset = createPresetStore();
