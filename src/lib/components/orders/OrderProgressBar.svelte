<script lang="ts">
	import type { OrderStatus } from '$lib/tauri/types';

	interface Props {
		status: OrderStatus;
		onchange?: (status: OrderStatus) => void;
		readonly?: boolean;
	}

	let { status, onchange, readonly = false }: Props = $props();

	const stages: { key: OrderStatus | 'delivered'; label: string }[] = [
		{ key: 'pending',     label: 'Ожидание' },
		{ key: 'in_progress', label: 'Сборка' },
		{ key: 'completed',   label: 'Готово' },
		{ key: 'delivered',   label: 'Доставлено' },
	];

	// completed fills all 4 stages (including "delivered" as visual endpoint)
	const stageIndex = $derived(() => {
		switch (status) {
			case 'pending':     return 0;
			case 'in_progress': return 1;
			case 'completed':   return 3;
			default:            return -1; // cancelled
		}
	});

	const isCancelled = $derived(status === 'cancelled');

	// Map stage index back to status
	function stageToStatus(idx: number): OrderStatus {
		switch (idx) {
			case 0: return 'pending';
			case 1: return 'in_progress';
			case 2: return 'completed';
			case 3: return 'completed'; // "delivered" is visual alias for completed
			default: return 'pending';
		}
	}

	function handleStageClick(idx: number) {
		if (readonly || isCancelled) return;
		const newStatus = stageToStatus(idx);
		if (newStatus !== status) {
			onchange?.(newStatus);
		}
	}
</script>

{#if isCancelled}
	<div class="cancelled-bar">
		<span class="cancelled-label">Отменён</span>
	</div>
{:else}
	<div class="progress-bar" role="group" aria-label="Статус заказа">
		{#each stages as stage, i}
			{@const active = i <= stageIndex()}
			{@const current = i === stageIndex()}
			<div class="stage-wrap">
				{#if i > 0}
					<div class="connector" class:filled={i <= stageIndex()}></div>
				{/if}
				<button
					type="button"
					class="stage-node"
					class:active
					class:current
					class:clickable={!readonly}
					onclick={() => handleStageClick(i)}
					aria-label={stage.label}
					aria-current={current ? 'step' : undefined}
					disabled={readonly}
				>
					{#if active}
						<svg viewBox="0 0 12 12" width="10" height="10" fill="currentColor">
							<circle cx="6" cy="6" r="6"/>
						</svg>
					{/if}
				</button>
				<span class="stage-label" class:active>{stage.label}</span>
			</div>
		{/each}
	</div>
{/if}

<style>
	.progress-bar {
		display: flex;
		align-items: center;
		gap: 0;
		width: 100%;
		position: relative;
	}

	.stage-wrap {
		display: flex;
		flex-direction: column;
		align-items: center;
		position: relative;
		flex: 1;
	}

	/* The horizontal connector line */
	.connector {
		position: absolute;
		top: 11px;
		right: 50%;
		width: 100%;
		height: 3px;
		background: var(--color-outline, rgba(255,255,255,0.15));
		border-radius: 1.5px;
		transform: translateX(-50%);
		z-index: 0;
		transition: background 0.5s ease-out, box-shadow 0.5s ease-out;
	}
	.connector.filled {
		background: var(--color-primary);
		box-shadow: 0 0 8px color-mix(in srgb, var(--color-primary) 35%, transparent);
	}

	/* Stage circle */
	.stage-node {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		border: 2px solid var(--color-outline, rgba(255,255,255,0.2));
		background: var(--color-surface);
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		z-index: 1;
		transition: border-color 0.3s, background 0.3s, transform 0.2s ease-out, box-shadow 0.3s;
		cursor: default;
		padding: 0;
	}

	.stage-node.active {
		border-color: var(--color-primary);
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		box-shadow: 0 0 6px color-mix(in srgb, var(--color-primary) 25%, transparent);
	}

	.stage-node.current {
		transform: scale(1.25);
		box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-primary) 20%, transparent),
		            0 0 12px color-mix(in srgb, var(--color-primary) 30%, transparent);
		animation: pulse-glow 2s ease-in-out infinite;
	}

	@keyframes pulse-glow {
		0%, 100% { box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-primary) 20%, transparent), 0 0 12px color-mix(in srgb, var(--color-primary) 30%, transparent); }
		50% { box-shadow: 0 0 0 6px color-mix(in srgb, var(--color-primary) 15%, transparent), 0 0 18px color-mix(in srgb, var(--color-primary) 20%, transparent); }
	}

	.stage-node.clickable { cursor: pointer; }
	.stage-node.clickable:hover {
		transform: scale(1.15);
		box-shadow: 0 0 10px color-mix(in srgb, var(--color-primary) 20%, transparent);
	}
	.stage-node.clickable.current:hover { transform: scale(1.3); }
	.stage-node:disabled { cursor: default; }

	.stage-label {
		font-size: 0.65rem;
		margin-top: 6px;
		color: var(--color-outline);
		white-space: nowrap;
		transition: color 0.3s;
		text-align: center;
	}
	.stage-label.active { color: var(--color-primary); font-weight: 500; }

	/* Cancelled */
	.cancelled-bar {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px 10px;
		background: rgba(107, 114, 128, 0.15);
		border-radius: 20px;
		width: fit-content;
	}
	.cancelled-label {
		font-size: 0.72rem;
		color: var(--color-outline);
		font-weight: 600;
		letter-spacing: 0.05em;
	}
</style>
