import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { AuditLog, AuditLogFilter } from '$lib/tauri/types';

function createAuditStore() {
	const { subscribe, set } = writable<AuditLog[]>([]);

	async function load(filter?: AuditLogFilter) {
		try {
			set(await commands.getAuditLogs(filter));
		} catch (e) {
			console.error('Failed to load audit logs:', e);
		}
	}

	return {
		subscribe,
		load,
	};
}

export const auditLog = createAuditStore();
