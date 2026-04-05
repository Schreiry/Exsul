import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type { SyncPeer } from '$lib/tauri/types';

export const nodeId = writable<string>('');
export const syncPeers = writable<SyncPeer[]>([]);
export const syncStatus = writable<'idle' | 'syncing' | 'error'>('idle');

export async function loadSyncState() {
	const [id, peers] = await Promise.all([
		commands.getNodeId(),
		commands.getSyncState(),
	]);
	nodeId.set(id);
	syncPeers.set(peers);
}
