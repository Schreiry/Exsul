<script lang="ts">
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { t } from '$lib/stores/i18n';
	import type { FlowerSort } from '$lib/tauri/types';

	interface Props {
		sort: FlowerSort;
		onclose: () => void;
	}

	let { sort, onclose }: Props = $props();

	let packCount = $state(1);
	let loading = $state(false);
	let error = $state('');
	let success = $state(false);

	const effectiveFpp = $derived(
		sort.flowers_per_pack_override && sort.flowers_per_pack_override > 0
			? sort.flowers_per_pack_override
			: Math.round($flowerConstants.flowers_per_pack)
	);

	const stemsNeeded = $derived(packCount * effectiveFpp);
	const canPackage = $derived(stemsNeeded <= sort.raw_stock && packCount > 0);

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKey(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}

	async function handleConfirm() {
		if (!canPackage || loading) return;
		loading = true;
		error = '';
		try {
			await flowerSorts.packageFlowers(sort.id, packCount);
			success = true;
			setTimeout(onclose, 800);
		} catch (e: unknown) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}
</script>

<svelte:window onkeydown={handleKey} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={handleBackdrop}>
	<div class="modal" role="dialog" aria-modal="true" aria-label={$t('flowers_pack_modal_title')}>

		<!-- Header -->
		<div class="modal-header">
			<div class="header-left">
				<span class="box-icon">📦</span>
				<div>
					<h2 class="modal-title">{$t('flowers_pack_modal_title')}</h2>
					<p class="modal-subtitle">{sort.name}{sort.variety ? ` — ${sort.variety}` : ''}</p>
				</div>
			</div>
			<button class="close-btn" onclick={onclose} aria-label="Закрыть">✕</button>
		</div>

		<!-- Body -->
		<div class="modal-body">
			<!-- Current stock -->
			<div class="stock-row">
				<div class="stock-item">
					<span class="stock-val color-raw">{sort.raw_stock}</span>
					<span class="stock-lbl">{$t('flowers_raw_stock')}</span>
				</div>
				<div class="stock-divider">→</div>
				<div class="stock-item">
					<span class="stock-val color-pkg">{sort.pkg_stock}</span>
					<span class="stock-lbl">{$t('flowers_total_packs')}</span>
				</div>
				<div class="stock-divider">+</div>
				<div class="stock-item">
					<span class="stock-val color-potential">{effectiveFpp}</span>
					<span class="stock-lbl">{$t('flowers_per_pack')}</span>
				</div>
			</div>

			<!-- Pack count input -->
			<div class="input-group">
				<label class="input-label" for="pack-count">{$t('flowers_pack_count')}</label>
				<div class="count-row">
					<button
						class="count-btn"
						onclick={() => packCount = Math.max(1, packCount - 1)}
						disabled={packCount <= 1}
					>−</button>
					<input
						id="pack-count"
						type="number"
						min="1"
						bind:value={packCount}
						class="count-input"
					/>
					<button
						class="count-btn"
						onclick={() => packCount = packCount + 1}
					>+</button>
				</div>
			</div>

			<!-- Math preview -->
			<div class="math-preview" class:math-error={!canPackage}>
				<span class="math-formula">
					{packCount} × {effectiveFpp} = <strong>{stemsNeeded}</strong> стеблей
				</span>
				{#if !canPackage && packCount > 0}
					<span class="math-warning">
						⚠ {$t('flowers_pack_insufficient', { needed: String(stemsNeeded), has: String(sort.raw_stock) })}
					</span>
				{:else if canPackage}
					<span class="math-ok">
						✓ Остаток: {sort.raw_stock - stemsNeeded} стеблей
					</span>
				{/if}
			</div>

			<!-- Error -->
			{#if error}
				<div class="error-msg">{error}</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<button class="btn-ghost" onclick={onclose}>{$t('action_cancel')}</button>
			<button
				class="btn-confirm"
				class:btn-success={success}
				onclick={handleConfirm}
				disabled={!canPackage || loading || success}
			>
				{#if success}
					✓ Упаковано
				{:else if loading}
					…
				{:else}
					📦 {$t('flowers_pack_confirm')}
				{/if}
			</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(6px);
		-webkit-backdrop-filter: blur(6px);
		z-index: 1100;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal {
		background: rgba(14, 14, 18, 0.88);
		backdrop-filter: blur(28px) saturate(180%);
		-webkit-backdrop-filter: blur(28px) saturate(180%);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 20px;
		box-shadow: var(--glass-shadow-hover);
		width: min(480px, calc(100vw - 32px));
		animation: modal-in 0.22s var(--ease-spring);
		overflow: hidden;
	}

	@keyframes modal-in {
		from { opacity: 0; transform: scale(0.96) translateY(8px); }
		to   { opacity: 1; transform: scale(1) translateY(0); }
	}

	/* Header */
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 20px 20px 16px;
		border-bottom: 1px solid var(--glass-border);
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.box-icon { font-size: 1.8rem; }

	.modal-title {
		font-size: 1.1rem;
		font-weight: 700;
		color: var(--color-on-surface);
		margin: 0;
	}

	.modal-subtitle {
		font-size: 0.8rem;
		color: var(--color-outline);
		margin: 2px 0 0;
	}

	.close-btn {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.85rem;
		transition: background 0.15s;
		flex-shrink: 0;
	}

	.close-btn:hover { background: var(--glass-bg-hover); }

	/* Body */
	.modal-body {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 18px;
	}

	/* Stock row */
	.stock-row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 16px;
		padding: 14px;
		background: var(--color-surface-container);
		border-radius: 12px;
	}

	.stock-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
	}

	.stock-val {
		font-size: 1.4rem;
		font-weight: 700;
		letter-spacing: -0.02em;
	}

	.stock-lbl {
		font-size: 0.65rem;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-outline);
	}

	.stock-divider {
		font-size: 1.2rem;
		color: var(--color-outline);
		opacity: 0.5;
	}

	/* Count input */
	.input-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.input-label {
		font-size: 0.82rem;
		font-weight: 500;
		color: var(--color-on-surface);
		opacity: 0.8;
	}

	.count-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.count-btn {
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		font-size: 1.1rem;
		cursor: pointer;
		transition: background 0.15s;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.count-btn:hover:not(:disabled) { background: var(--glass-bg-hover); }
	.count-btn:disabled { opacity: 0.35; cursor: default; }

	.count-input {
		flex: 1;
		text-align: center;
		background: var(--color-surface-container-high);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 10px;
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--color-on-surface);
		font-family: inherit;
		outline: none;
	}

	.count-input:focus { border-color: var(--color-primary); }

	/* Math preview */
	.math-preview {
		padding: 12px 14px;
		border-radius: 10px;
		background: rgba(52, 211, 153, 0.06);
		border: 1px solid rgba(52, 211, 153, 0.15);
		display: flex;
		flex-direction: column;
		gap: 4px;
		transition: background 0.2s, border-color 0.2s;
	}

	.math-preview.math-error {
		background: rgba(248, 113, 113, 0.06);
		border-color: rgba(248, 113, 113, 0.2);
	}

	.math-formula {
		font-size: 0.9rem;
		color: var(--color-on-surface);
	}

	.math-formula strong { color: var(--color-primary); }

	.math-warning {
		font-size: 0.78rem;
		color: #f87171;
	}

	.math-ok {
		font-size: 0.78rem;
		color: #34d399;
	}

	.error-msg {
		padding: 10px 14px;
		border-radius: 8px;
		background: rgba(248, 113, 113, 0.1);
		border: 1px solid rgba(248, 113, 113, 0.2);
		color: #f87171;
		font-size: 0.82rem;
	}

	/* Footer */
	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
		padding: 16px 20px;
		border-top: 1px solid var(--glass-border);
	}

	.btn-ghost {
		padding: 9px 20px;
		border-radius: 10px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s;
		font-family: inherit;
	}

	.btn-ghost:hover { background: var(--glass-bg-hover); }

	.btn-confirm {
		padding: 9px 24px;
		border-radius: 10px;
		background: rgba(52, 211, 153, 0.12);
		border: 1px solid rgba(52, 211, 153, 0.3);
		color: var(--color-primary);
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.15s, transform 0.1s var(--ease-spring);
		font-family: inherit;
	}

	.btn-confirm:hover:not(:disabled) {
		background: rgba(52, 211, 153, 0.2);
		transform: translateY(-1px);
	}

	.btn-confirm:disabled { opacity: 0.4; cursor: default; }

	.btn-confirm.btn-success {
		background: rgba(52, 211, 153, 0.2);
		color: #34d399;
	}

	/* Color tokens */
	.color-raw      { color: #60a5fa; }
	.color-pkg      { color: #34d399; }
	.color-potential { color: #fbbf24; }
</style>
