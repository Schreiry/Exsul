import { writable, get } from 'svelte/store';
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

// ── Singleton peer listener ───────────────────────────────────────────────────
// Lifted out of SyncModal lifecycle so it survives open/close cycles.
let _listenerInit = false;
let _peerUnlisten: (() => void) | undefined;

/** Call once from the app layout. Safe to call multiple times (no-op after first). */
export async function initSyncListener() {
	if (_listenerInit) return;
	_listenerInit = true;
	_peerUnlisten = await listen<WsPeerStatus[]>('sync://peers-updated', (event) => {
		wsPeers.set(event.payload);
	});
}

/** Reset UI-only state when the sync modal is opened, so stale messages don't show. */
export function resetSyncUi() {
	syncStatus.set('idle');
	syncMessage.set('');
}

/** Clear sync status back to idle (call after indicator auto-dismiss timeout). */
export function clearSyncStatus() {
	syncStatus.set('idle');
	syncMessage.set('');
}

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

/** @deprecated Use initSyncListener() in layout instead. Kept for compatibility. */
export function listenWsPeers() {
	return listen<WsPeerStatus[]>('sync://peers-updated', (event) => {
		wsPeers.set(event.payload);
	});
}
