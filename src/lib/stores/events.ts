import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { EventRecord } from '$lib/tauri/types';

function createEventStore() {
	const { subscribe, set } = writable<EventRecord[]>([]);

	return {
		subscribe,

		async load(since?: string, limit?: number) {
			const events = await commands.getEvents(since, limit);
			set(events);
		},
	};
}

export const eventLog = createEventStore();
