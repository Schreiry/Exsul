<script lang="ts">
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { orders } from '$lib/stores/orders';
	import { commands } from '$lib/tauri/commands';
	import { inventory } from '$lib/stores/inventory';
	import type { Item, FlowerSort } from '$lib/tauri/types';

	interface Props {
		item: Item;
		onclose: () => void;
	}

	let { item, onclose }: Props = $props();

	// Find the associated flower sort (sort.id = item.category_id after bridge)
	const sort = $derived<FlowerSort | undefined>(
		$flowerSorts.find((s) => s.id === item.category_id || s.name === item.category)
	);

	// Active orders for linking
	const activeOrders = $derived(
		$orders.filter((o) => o.status === 'pending' || o.status === 'in_progress')
	);

	let packCount = $state(1);
	let selectedOrderId = $state('');
	let note = $state('');
	let loading = $state(false);
	let error = $state('');
	let success = $state(false);

	const effectiveFpp = $derived(
		sort?.flowers_per_pack_override && sort.flowers_per_pack_override > 0
			? sort.flowers_per_pack_override
			: Math.round($flowerConstants.flowers_per_pack)
	);

	const stemsNeeded = $derived(packCount * effectiveFpp);
	// Source of truth: item.current_stock is the actual stems in inventory
	const rawAvailable = $derived(item.current_stock);
	const stemsAfter = $derived(rawAvailable - stemsNeeded);
	const canPack = $derived(stemsNeeded <= rawAvailable && packCount > 0 && rawAvailable > 0);

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKey(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}

	async function handleConfirm() {
		if (!canPack || loading) return;
		loading = true;
		error = '';
		try {
			// 1. Deduct stems from the inventory item (source of truth)
			await commands.adjustStock({ item_id: item.id, delta: -stemsNeeded });

			// 2. If a flower sort is linked, increment its pkg_stock counter (raw managed via items)
			if (sort) {
				await commands.adjustFlowerStock(sort.id, 0, packCount);
			}

			// 3. Create pack assignment record
			const sortIdForAssignment = sort?.id ?? item.category_id ?? item.id;
			await commands.createPackAssignment({
				sort_id: sortIdForAssignment,
				order_id: selectedOrderId || undefined,
				pack_count: packCount,
				stems_per_pack: effectiveFpp,
				note: note.trim() || undefined,
			});

			// Reload stores
			await Promise.all([flowerSorts.load(), inventory.load()]);

			success = true;
			setTimeout(onclose, 900);
		} catch (e: unknown) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	// Load orders on mount
	$effect(() => {
		orders.load();
	});
</script>

<svelte:window onkeydown={handleKey} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={handleBackdrop}>
	<div class="modal" role="dialog" aria-modal="true" aria-label="Упаковка товара">

		<!-- Header -->
		<div class="modal-header">
			<div class="header-left">
				<span class="box-icon">📦</span>
				<div>
					<h2 class="modal-title">Упаковка</h2>
					<p class="modal-subtitle">{item.name}</p>
				</div>
			</div>
			<button class="close-btn" onclick={onclose} aria-label="Закрыть">✕</button>
		</div>

		<div class="modal-body">
			{#if !sort}
				<div class="no-sort-hint">
					ℹ Сорт не привязан — будет использован остаток товара как стебли.
					Для полной синхронизации укажите сорт в настройках товара.
				</div>
			{/if}
			<!-- Stock preview (always shown — item.current_stock is source of truth) -->
			<div class="stock-row">
				<div class="stock-item">
					<span class="stock-val color-raw">{rawAvailable}</span>
					<span class="stock-lbl">Стеблей в остатке</span>
				</div>
				<span class="stock-arrow">→</span>
				<div class="stock-item">
					<span class="stock-val" class:color-ok={stemsAfter >= 0} class:color-err={stemsAfter < 0}>
						{stemsAfter}
					</span>
					<span class="stock-lbl">Останется</span>
				</div>
				<span class="stock-arrow">+</span>
				<div class="stock-item">
					<span class="stock-val color-pkg">{(sort?.pkg_stock ?? 0) + (canPack ? packCount : 0)}</span>
					<span class="stock-lbl">Упаковок</span>
				</div>
			</div>

				<!-- Pack count -->
				<div class="field-group">
					<label class="field-label" for="pa-count">Количество упаковок</label>
					<div class="count-row">
						<button class="count-btn" onclick={() => (packCount = Math.max(1, packCount - 1))} disabled={packCount <= 1}>−</button>
						<input id="pa-count" class="count-input" type="number" min="1" bind:value={packCount} />
						<button class="count-btn" onclick={() => (packCount = packCount + 1)}>+</button>
					</div>
					<span class="field-hint">{packCount} × {effectiveFpp} = <strong>{stemsNeeded}</strong> стеблей</span>
				</div>

				<!-- Order link -->
				<div class="field-group">
					<label class="field-label" for="pa-order">Привязать к заказу (необязательно)</label>
					<select id="pa-order" class="field-select" bind:value={selectedOrderId}>
						<option value="">— Без заказа —</option>
						{#each activeOrders as order}
							<option value={order.id}>{order.customer_name} — #{order.id.slice(0, 8)}</option>
						{/each}
					</select>
				</div>

				<!-- Note -->
				<div class="field-group">
					<label class="field-label" for="pa-note">Заметка</label>
					<input id="pa-note" class="field-input" type="text" bind:value={note} placeholder="Для кого, особые условия…" />
				</div>

				{#if !canPack && packCount > 0}
					<div class="warn-box">
						⚠ Недостаточно стеблей: нужно {stemsNeeded}, есть {rawAvailable}
					</div>
				{/if}

			{#if error}
				<div class="error-box">{error}</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<button class="btn-ghost" onclick={onclose}>Отмена</button>
			<button
				class="btn-confirm"
				class:btn-success={success}
				onclick={handleConfirm}
				disabled={!canPack || loading || success}
			>
				{#if success}✓ Упаковано{:else if loading}…{:else}📦 Упаковать{/if}
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
		width: min(500px, calc(100vw - 32px));
		animation: modal-in 0.22s var(--ease-spring);
		overflow: hidden;
	}

	@keyframes modal-in {
		from { opacity: 0; transform: scale(0.96) translateY(8px); }
		to   { opacity: 1; transform: scale(1) translateY(0); }
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 20px 20px 16px;
		border-bottom: 1px solid var(--glass-border);
	}

	.header-left { display: flex; align-items: center; gap: 12px; }
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

	.modal-body {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		max-height: calc(100vh - 200px);
		overflow-y: auto;
	}

	.no-sort-hint {
		font-size: 0.85rem;
		color: #fbbf24;
		padding: 12px;
		background: rgba(251, 191, 36, 0.08);
		border-radius: 8px;
	}

	/* Stock row */
	.stock-row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 14px;
		padding: 14px;
		background: var(--color-surface-container);
		border-radius: 12px;
	}

	.stock-item { display: flex; flex-direction: column; align-items: center; gap: 4px; }

	.stock-val {
		font-size: 1.35rem;
		font-weight: 700;
		letter-spacing: -0.02em;
	}

	.stock-lbl {
		font-size: 0.62rem;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-outline);
	}

	.stock-arrow { font-size: 1.1rem; color: var(--color-outline); opacity: 0.5; }

	.color-raw { color: #60a5fa; }
	.color-pkg { color: #34d399; }
	.color-ok  { color: #34d399; }
	.color-err { color: #f87171; }

	/* Fields */
	.field-group { display: flex; flex-direction: column; gap: 6px; }

	.field-label {
		font-size: 0.72rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-weight: 500;
	}

	.field-hint {
		font-size: 0.75rem;
		color: var(--color-outline);
	}

	.field-hint strong { color: var(--color-primary); }

	.count-row { display: flex; align-items: center; gap: 8px; }

	.count-btn {
		width: 38px;
		height: 38px;
		border-radius: 9px;
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
		border-radius: 9px;
		padding: 9px;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-on-surface);
		font-family: inherit;
		outline: none;
	}

	.count-input:focus { border-color: var(--color-primary); }

	.field-select,
	.field-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 9px;
		padding: 9px 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.field-select:focus,
	.field-input:focus { border-color: var(--color-primary); }

	.warn-box {
		padding: 10px 14px;
		border-radius: 9px;
		background: rgba(251, 191, 36, 0.08);
		border: 1px solid rgba(251, 191, 36, 0.25);
		color: #fbbf24;
		font-size: 0.82rem;
	}

	.error-box {
		padding: 10px 14px;
		border-radius: 9px;
		background: rgba(248, 113, 113, 0.08);
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
		font-family: inherit;
		transition: background 0.15s;
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
		font-family: inherit;
		transition: background 0.15s, transform 0.1s var(--ease-spring);
	}

	.btn-confirm:hover:not(:disabled) {
		background: rgba(52, 211, 153, 0.2);
		transform: translateY(-1px);
	}

	.btn-confirm:disabled { opacity: 0.4; cursor: default; }
	.btn-confirm.btn-success { background: rgba(52, 211, 153, 0.2); }
</style>
