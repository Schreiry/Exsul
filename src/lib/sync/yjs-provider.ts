import * as Y from 'yjs';
import { WebrtcProvider } from 'y-webrtc';
import { commands } from '$lib/tauri/commands';
import type { EventRecord } from '$lib/tauri/types';

let doc: Y.Doc | null = null;
let provider: WebrtcProvider | null = null;

export interface SyncContext {
	doc: Y.Doc;
	provider: WebrtcProvider;
}

export async function initSync(roomName?: string): Promise<SyncContext> {
	const nodeId = await commands.getNodeId();
	const room = roomName || `exsul-${nodeId.slice(0, 8)}`;

	doc = new Y.Doc();

	provider = new WebrtcProvider(room, doc, {
		// y-webrtc-signaling-eu.herokuapp.com was shut down (Heroku removed free tier).
		// Using only the official yjs.dev signaling server.
		signaling: ['wss://signaling.yjs.dev'],
		maxConns: 8,
	});

	const events = doc.getArray<Record<string, unknown>>('events');

	// When remote events arrive via CRDT, persist them locally
	events.observe(async (event) => {
		if (event.transaction.local) return;

		const remoteEvents: EventRecord[] = [];
		for (const item of event.changes.added) {
			for (const content of item.content.getContent()) {
				remoteEvents.push(content as EventRecord);
			}
		}

		if (remoteEvents.length > 0) {
			try {
				const merged = await commands.mergeRemoteEvents(remoteEvents);
				if (merged > 0) {
					console.log(`[sync] merged ${merged} remote events`);
				}
			} catch (err) {
				console.error('[sync] failed to merge remote events:', err);
			}
		}
	});

	console.log(`[sync] initialized, room=${room}, nodeId=${nodeId}`);
	return { doc, provider };
}

export function broadcastEvent(eventRecord: EventRecord) {
	if (!doc) return;
	const events = doc.getArray<Record<string, unknown>>('events');
	events.push([eventRecord as unknown as Record<string, unknown>]);
}

export function getConnectionStatus(): { connected: boolean; peers: number } {
	if (!provider) return { connected: false, peers: 0 };
	return {
		connected: provider.connected,
		peers: provider.awareness.getStates().size - 1,
	};
}

export function destroy() {
	provider?.destroy();
	doc?.destroy();
	provider = null;
	doc = null;
}
