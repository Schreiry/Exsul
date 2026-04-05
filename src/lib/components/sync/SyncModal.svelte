<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import {
		nodeId,
		syncStatus,
		syncMessage,
		wsPeers,
		wsServerRunning,
		trustedNodes,
		loadSyncState,
		loadTrustedNodes,
		loadWsStatus,
		syncWithPeer,
		listenWsPeers,
	} from '$lib/stores/sync';
	import { commands } from '$lib/tauri/commands';
	import type { TrustedNode } from '$lib/tauri/types';

	interface Props {
		open: boolean;
		onclose: () => void;
	}

	let { open, onclose }: Props = $props();

	// ── local form state ─────────────────────────────────────
	let targetIp = $state('100.92.43.117'); // dgsurface default
	let newNodeId = $state('');
	let newAlias = $state('');
	let newIpHint = $state('');
	let addNodeError = $state('');
	let copied = $state(false);

	// ── lifecycle ─────────────────────────────────────────────
	let unlisten: (() => void) | undefined;

	onMount(async () => {
		await Promise.all([loadSyncState(), loadTrustedNodes(), loadWsStatus()]);
		const un = await listenWsPeers();
		unlisten = un;
	});

	onDestroy(() => {
		unlisten?.();
	});

	// ── actions ───────────────────────────────────────────────
	async function handleSync() {
		if (!targetIp.trim()) return;
		await syncWithPeer(targetIp.trim());
		await loadWsStatus();
	}

	async function handleAddNode() {
		addNodeError = '';
		if (!newNodeId.trim()) {
			addNodeError = 'ID узла обязателен';
			return;
		}
		try {
			await commands.addTrustedNode({
				node_id: newNodeId.trim(),
				alias: newAlias.trim() || undefined,
				ip_hint: newIpHint.trim() || undefined,
			});
			newNodeId = '';
			newAlias = '';
			newIpHint = '';
			await loadTrustedNodes();
		} catch (e: unknown) {
			addNodeError = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleRemoveNode(node_id: string) {
		await commands.removeTrustedNode(node_id);
		await loadTrustedNodes();
	}

	async function copyNodeId() {
		if (!$nodeId) return;
		await navigator.clipboard.writeText($nodeId);
		copied = true;
		setTimeout(() => (copied = false), 1800);
	}

	function getStateColor(state: string) {
		switch (state) {
			case 'connected': return '#34d399';
			case 'connecting': return '#fbbf24';
			case 'rejected': return '#f87171';
			default: return '#6b7280';
		}
	}

	function getStateLabel(state: string) {
		switch (state) {
			case 'connected': return 'Подключён';
			case 'connecting': return 'Соединение…';
			case 'rejected': return 'Отклонён';
			default: return 'Отключён';
		}
	}
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="backdrop"
		role="presentation"
		onclick={onclose}
		onkeydown={(e) => e.key === 'Escape' && onclose()}
	></div>

	<!-- Modal -->
	<div class="modal glass" role="dialog" aria-modal="true" aria-label="Синхронизация">
		<!-- Header -->
		<div class="modal-header">
			<div class="header-left">
				<span class="header-icon">⟳</span>
				<span class="header-title">Синхронизация</span>
			</div>
			<button class="close-btn" onclick={onclose} aria-label="Закрыть">✕</button>
		</div>

		<!-- Node identity -->
		<section class="section">
			<div class="section-label">Этот узел</div>
			<div class="node-id-row">
				<code class="node-id">{$nodeId || '—'}</code>
				<button class="copy-btn glass-btn" onclick={copyNodeId}>
					{copied ? '✓ Скопировано' : 'Копировать'}
				</button>
			</div>
			<div class="server-status">
				<span
					class="dot"
					style="background: {$wsServerRunning ? '#34d399' : '#6b7280'}"
				></span>
				<span>
					Сервер P2P {$wsServerRunning ? `запущен на порту 8765` : 'не запущен'}
				</span>
			</div>
		</section>

		<!-- Quick sync -->
		<section class="section">
			<div class="section-label">Синхронизировать с узлом</div>
			<div class="input-row">
				<input
					class="glass-input"
					type="text"
					placeholder="IP-адрес (напр. 100.92.43.117)"
					bind:value={targetIp}
					onkeydown={(e) => e.key === 'Enter' && handleSync()}
				/>
				<button
					class="sync-btn glass-btn accent"
					onclick={handleSync}
					disabled={$syncStatus === 'syncing'}
				>
					{$syncStatus === 'syncing' ? '…' : 'Синхр.'}
				</button>
			</div>
			{#if $syncMessage}
				<div
					class="sync-message"
					class:success={$syncStatus === 'success'}
					class:error={$syncStatus === 'error'}
				>
					{$syncMessage}
				</div>
			{/if}
		</section>

		<!-- Live peers -->
		{#if $wsPeers.length > 0}
			<section class="section">
				<div class="section-label">Активные соединения</div>
				<div class="peers-list">
					{#each $wsPeers as peer (peer.ip)}
						<div class="peer-row glass-card">
							<div class="peer-info">
								<span class="peer-alias">{peer.alias ?? peer.node_id.slice(0, 8) + '…'}</span>
								<span class="peer-ip">{peer.ip}</span>
							</div>
							<div class="peer-right">
								<span class="state-badge" style="background: {getStateColor(peer.state)}1a; color: {getStateColor(peer.state)}; border-color: {getStateColor(peer.state)}40">
									{getStateLabel(peer.state)}
								</span>
								{#if peer.last_sync}
									<span class="peer-sync-time">+{peer.events_merged} событий</span>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		<!-- Trusted nodes management -->
		<section class="section">
			<div class="section-label">Доверенные узлы</div>

			{#if $trustedNodes.length > 0}
				<div class="trusted-list">
					{#each $trustedNodes as node (node.node_id)}
						<div class="trusted-row glass-card">
							<div class="trusted-info">
								<span class="trusted-alias">{node.alias ?? 'Узел'}</span>
								<code class="trusted-id">{node.node_id.slice(0, 16)}…</code>
								{#if node.ip_hint}
									<span class="trusted-ip">{node.ip_hint}</span>
								{/if}
							</div>
							<button
								class="remove-btn"
								onclick={() => handleRemoveNode(node.node_id)}
								aria-label="Удалить"
							>✕</button>
						</div>
					{/each}
				</div>
			{:else}
				<p class="empty-hint">Нет доверенных узлов. Добавьте ID второго ПК ниже.</p>
			{/if}

			<!-- Add trusted node form -->
			<div class="add-node-form glass-card">
				<div class="form-title">Добавить узел</div>
				<input
					class="glass-input"
					type="text"
					placeholder="Node ID (UUID)"
					bind:value={newNodeId}
				/>
				<div class="form-row">
					<input
						class="glass-input"
						type="text"
						placeholder="Псевдоним (напр. dgsurface)"
						bind:value={newAlias}
					/>
					<input
						class="glass-input"
						type="text"
						placeholder="IP (Tailscale)"
						bind:value={newIpHint}
					/>
				</div>
				{#if addNodeError}
					<div class="error-text">{addNodeError}</div>
				{/if}
				<button class="glass-btn accent" onclick={handleAddNode}>
					Добавить в доверенные
				</button>
			</div>
		</section>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(4px);
		z-index: 1100;
	}

	.modal {
		position: fixed;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 1200;
		width: min(520px, calc(100vw - 32px));
		max-height: calc(100vh - 80px);
		overflow-y: auto;
		border-radius: 20px;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;

		background: rgba(14, 14, 18, 0.72);
		backdrop-filter: blur(24px) saturate(180%);
		-webkit-backdrop-filter: blur(24px) saturate(180%);
		border: 1px solid rgba(255, 255, 255, 0.10);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.12),
			0 24px 64px rgba(0, 0, 0, 0.6),
			0 8px 24px rgba(0, 0, 0, 0.3);
	}

	/* ── Header ──────────────────────────────── */
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.header-icon {
		font-size: 1.2rem;
		color: var(--color-primary);
	}

	.header-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-on-surface);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		font-size: 1rem;
		padding: 4px 8px;
		border-radius: 8px;
		transition: color 0.15s, background 0.15s;
	}

	.close-btn:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-on-surface);
	}

	/* ── Section ─────────────────────────────── */
	.section {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.section-label {
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-outline);
	}

	/* ── Node ID ─────────────────────────────── */
	.node-id-row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.node-id {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 0.78rem;
		color: var(--color-primary);
		background: rgba(52, 211, 153, 0.06);
		border: 1px solid rgba(52, 211, 153, 0.18);
		border-radius: 8px;
		padding: 8px 12px;
		word-break: break-all;
	}

	.copy-btn {
		flex-shrink: 0;
		font-size: 0.78rem;
	}

	.server-status {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.78rem;
		color: var(--color-outline);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		display: inline-block;
		flex-shrink: 0;
	}

	/* ── Inputs / Buttons ────────────────────── */
	.glass-input {
		flex: 1;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.10);
		border-radius: 10px;
		color: var(--color-on-surface);
		font-size: 0.85rem;
		padding: 9px 12px;
		outline: none;
		transition: border-color 0.15s;
		width: 100%;
	}

	.glass-input:focus {
		border-color: var(--color-primary);
	}

	.glass-input::placeholder {
		color: var(--color-outline);
	}

	.glass-btn {
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.10);
		border-radius: 10px;
		color: var(--color-on-surface);
		font-size: 0.82rem;
		padding: 8px 14px;
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
		white-space: nowrap;
	}

	.glass-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.10);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.glass-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.glass-btn.accent {
		background: rgba(52, 211, 153, 0.12);
		border-color: rgba(52, 211, 153, 0.30);
		color: var(--color-primary);
	}

	.glass-btn.accent:hover:not(:disabled) {
		background: rgba(52, 211, 153, 0.20);
	}

	.input-row {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.sync-btn {
		flex-shrink: 0;
	}

	/* ── Status messages ─────────────────────── */
	.sync-message {
		font-size: 0.80rem;
		padding: 8px 12px;
		border-radius: 8px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-outline);
	}

	.sync-message.success {
		background: rgba(52, 211, 153, 0.08);
		border-color: rgba(52, 211, 153, 0.25);
		color: #34d399;
	}

	.sync-message.error {
		background: rgba(248, 113, 113, 0.08);
		border-color: rgba(248, 113, 113, 0.25);
		color: #f87171;
	}

	/* ── Peer rows ───────────────────────────── */
	.glass-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 12px 14px;
	}

	.peers-list,
	.trusted-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.peer-row,
	.trusted-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	.peer-info,
	.trusted-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.peer-alias,
	.trusted-alias {
		font-size: 0.85rem;
		font-weight: 500;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.peer-ip,
	.trusted-ip {
		font-size: 0.75rem;
		color: var(--color-outline);
		font-family: var(--font-mono);
	}

	.trusted-id {
		font-size: 0.72rem;
		color: var(--color-primary);
		font-family: var(--font-mono);
	}

	.peer-right {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 4px;
		flex-shrink: 0;
	}

	.state-badge {
		font-size: 0.72rem;
		font-weight: 600;
		padding: 3px 8px;
		border-radius: 6px;
		border: 1px solid;
		white-space: nowrap;
	}

	.peer-sync-time {
		font-size: 0.70rem;
		color: var(--color-outline);
	}

	.remove-btn {
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		font-size: 0.85rem;
		padding: 4px 8px;
		border-radius: 6px;
		transition: color 0.15s, background 0.15s;
		flex-shrink: 0;
	}

	.remove-btn:hover {
		color: #f87171;
		background: rgba(248, 113, 113, 0.08);
	}

	/* ── Add node form ───────────────────────── */
	.add-node-form {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-top: 4px;
	}

	.form-title {
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--color-outline);
	}

	.form-row {
		display: flex;
		gap: 8px;
	}

	.error-text {
		font-size: 0.78rem;
		color: #f87171;
	}

	.empty-hint {
		font-size: 0.82rem;
		color: var(--color-outline);
		font-style: italic;
	}
</style>
