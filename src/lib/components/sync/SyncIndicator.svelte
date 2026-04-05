<script lang="ts">
	import { syncStatus, syncMessage, clearSyncStatus } from '$lib/stores/sync';

	let prevStatus = 'idle';
	let visible = $state(false);
	let dismissTimer: ReturnType<typeof setTimeout> | undefined;

	// Watch for status transitions
	$effect(() => {
		const status = $syncStatus;

		if (status === 'idle') {
			// Fade out immediately when explicitly cleared
			visible = false;
			prevStatus = 'idle';
			return;
		}

		visible = true;

		// Clear any pending dismiss timer when status changes
		clearTimeout(dismissTimer);

		if (status === 'success') {
			if (prevStatus !== 'success') {
				playChime();
			}
			dismissTimer = setTimeout(() => {
				clearSyncStatus();
			}, 3000);
		} else if (status === 'error') {
			dismissTimer = setTimeout(() => {
				clearSyncStatus();
			}, 5000);
		}

		prevStatus = status;
	});

	function playChime() {
		try {
			const ctx = new AudioContext();
			const osc = ctx.createOscillator();
			const gain = ctx.createGain();
			osc.connect(gain);
			gain.connect(ctx.destination);
			osc.type = 'sine';
			osc.frequency.setValueAtTime(880, ctx.currentTime);
			osc.frequency.exponentialRampToValueAtTime(1100, ctx.currentTime + 0.15);
			gain.gain.setValueAtTime(0.07, ctx.currentTime);
			gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.5);
			osc.start(ctx.currentTime);
			osc.stop(ctx.currentTime + 0.5);
		} catch {
			// AudioContext not available — silent fallback
		}
	}
</script>

{#if visible}
	<div
		class="sync-indicator glass-sm"
		class:syncing={$syncStatus === 'syncing'}
		class:success={$syncStatus === 'success'}
		class:error={$syncStatus === 'error'}
		role="status"
		aria-live="polite"
	>
		{#if $syncStatus === 'syncing'}
			<span class="dot pulse"></span>
			<span class="label">Синхронизация…</span>
		{:else if $syncStatus === 'success'}
			<span class="check">✓</span>
			<span class="label">Синхронизовано</span>
		{:else if $syncStatus === 'error'}
			<span class="x">✕</span>
			<span class="label">{$syncMessage || 'Ошибка синхронизации'}</span>
		{/if}
	</div>
{/if}

<style>
	.sync-indicator {
		position: fixed;
		top: 16px;
		right: 16px;
		z-index: 900;
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 7px 14px;
		font-size: 0.78rem;
		font-weight: 500;
		color: var(--color-on-surface);
		max-width: 260px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;

		animation: slide-in 0.2s var(--ease-spring) both;
	}

	@keyframes slide-in {
		from { opacity: 0; transform: translateY(-8px) scale(0.96); }
		to   { opacity: 1; transform: translateY(0)   scale(1); }
	}

	/* Status-specific accent colors */
	.sync-indicator.syncing {
		border-color: rgba(251, 191, 36, 0.30);
	}

	.sync-indicator.success {
		border-color: rgba(52, 211, 153, 0.30);
	}

	.sync-indicator.error {
		border-color: rgba(248, 113, 113, 0.30);
	}

	/* Dot */
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #fbbf24;
		flex-shrink: 0;
	}

	.dot.pulse {
		animation: pulse 1.2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; transform: scale(1); }
		50%       { opacity: 0.4; transform: scale(0.75); }
	}

	.check {
		color: #34d399;
		font-size: 0.85rem;
		flex-shrink: 0;
	}

	.x {
		color: #f87171;
		font-size: 0.85rem;
		flex-shrink: 0;
	}

	.label {
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
