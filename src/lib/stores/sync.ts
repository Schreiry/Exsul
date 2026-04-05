import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { commands } from '$lib/tauri/commands';
import type { SyncPeer, WsPeerStatus, WsServerStatus, TrustedNode } from '$lib/tauri/types';

export const nodeId = writable<string>('');
export const syncPeers = writable<SyncPeer[]>([]);
export const syncStatus = writable<'idle' | 'syncing' | 'error' | 'success'>('idle');

// WebSocket live state
export const wsPeers = writable<WsPeerStatus[]>([]);
export const wsServerRunning = writable<boolean>(false);
export const wsPort = writable<number>(8765);

// Trusted nodes list
export const trustedNodes = writable<TrustedNode[]>([]);

// Status message for the sync modal
export const syncMessage = writable<string>('');

export async function loadSyncState() {
	const [id, peers] = await Promise.all([
		commands.getNodeId(),
		commands.getSyncState(),
	]);
	nodeId.set(id);
	syncPeers.set(peers);
}

export async function loadTrustedNodes() {
	try {
		const nodes = await commands.getTrustedNodes();
		trustedNodes.set(nodes);
	} catch (e) {
		console.error('Failed to load trusted nodes', e);
	}
}

export async function loadWsStatus() {
	try {
		const status: WsServerStatus = await commands.getWsStatus();
		wsServerRunning.set(status.running);
		wsPort.set(status.port);
		wsPeers.set(status.peers);
	} catch (e) {
		console.error('Failed to load WS status', e);
	}
}

/** One-shot sync with a peer by their Tailscale/LAN IP. */
export async function syncWithPeer(ip: string): Promise<number> {
	syncStatus.set('syncing');
	syncMessage.set('');
	try {
		const merged = await commands.wsConnectPeer(ip);
		syncStatus.set('success');
		syncMessage.set(`Синхронизировано: ${merged} событий`);
		await loadWsStatus();
		return merged;
	} catch (e: unknown) {
		const msg = e instanceof Error ? e.message : String(e);
		syncStatus.set('error');
		syncMessage.set(msg);
		throw e;
	}
}

/** Subscribe to live peer-update events emitted by the Rust WS layer. */
export function listenWsPeers() {
	return listen<WsPeerStatus[]>('sync://peers-updated', (event) => {
		wsPeers.set(event.payload);
	});
}
