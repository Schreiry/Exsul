<script lang="ts">
	import { inventory } from '$lib/stores/inventory';
	import { commands } from '$lib/tauri/commands';
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { preset } from '$lib/stores/preset';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { formatDateTime } from '$lib/utils/time';
	import type { Item } from '$lib/tauri/types';

	interface Props {
		item: Item;
		onclose: () => void;
	}

	let { item, onclose }: Props = $props();

	let stemQty = $state(1);
	let packQty = $state(0);
	let note = $state('');
	let loading = $state(false);
	let error = $state('');
	let success = $state(false);

	// Find linked sort for pack info
	const sort = $derived(
		$flowerSorts.find((s) => s.id === item.category_id || s.name === item.category)
	);

	const effectiveFpp = $derived(
		sort?.flowers_per_pack_override && sort.flowers_per_pack_override > 0
			? sort.flowers_per_pack_override
			: Math.round($flowerConstants.flowers_per_pack)
	);

	// Total stems: direct stems + stems from packs
	const totalStems = $derived(stemQty + packQty * effectiveFpp);
	const totalAmount = $derived(totalStems * item.current_price);
	const stockAfter = $derived(item.current_stock - totalStems);
	const canSell = $derived(totalStems > 0 && totalStems <= item.current_stock);

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKey(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}

	function fmt(v: number) { return formatAmount(v, $globalCurrency); }

	async function handleSell() {
		if (!canSell || loading) return;
		loading = true;
		error = '';
		try {
			// Record sale for the total stems count
			await commands.recordSale({
				item_id: item.id,
				quantity: totalStems,
				sale_price: item.current_price,
			});

			// If packs were sold and a sort exists, update sort pkg_stock
			if (packQty > 0 && sort) {
				await commands.adjustFlowerStock(sort.id, 0, -packQty);
			}

			await inventory.load();
			if ($preset === 'flowers') await flowerSorts.load();

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
	<div class="modal" role="dialog" aria-modal="true" aria-label="Продажа">

		<div class="modal-header">
			<div class="header-left">
				<span class="icon">🛒</span>
				<div>
					<h2 class="modal-title">Продажа</h2>
					<p class="modal-subtitle">{item.name}</p>
				</div>
			</div>
			<button class="close-btn" onclick={onclose} aria-label="Закрыть">✕</button>
		</div>

		<div class="modal-body">
			<!-- Stock & price overview -->
			<div class="overview-row">
				<div class="ov-item">
					<span class="ov-val color-stock">{item.current_stock}</span>
					<span class="ov-lbl">Остаток</span>
				</div>
				<div class="ov-item">
					<span class="ov-val color-price">{fmt(item.current_price)}</span>
					<span class="ov-lbl">Цена</span>
				</div>
				<div class="ov-item">
					<span class="ov-val" class:color-ok={stockAfter >= 0} class:color-err={stockAfter < 0}>
						{stockAfter}
					</span>
					<span class="ov-lbl">Останется</span>
				</div>
			</div>

			<!-- Stem quantity -->
			<div class="field-group">
				<label class="field-label" for="qs-stems">
					{$preset === 'flowers' ? 'Количество стеблей' : 'Количество'}
				</label>
				<div class="count-row">
					<button class="count-btn" onclick={() => (stemQty = Math.max(0, stemQty - 1))}>−</button>
					<input id="qs-stems" class="count-input" type="number" min="0" bind:value={stemQty} />
					<button class="count-btn" onclick={() => (stemQty = stemQty + 1)}>+</button>
				</div>
			</div>

			<!-- Pack quantity (flowers only) -->
			{#if $preset === 'flowers'}
				<div class="field-group">
					<label class="field-label" for="qs-packs">
						Упаковок (по {effectiveFpp} шт.)
					</label>
					<div class="count-row">
						<button class="count-btn" onclick={() => (packQty = Math.max(0, packQty - 1))}>−</button>
						<input id="qs-packs" class="count-input" type="number" min="0" bind:value={packQty} />
						<button class="count-btn" onclick={() => (packQty = packQty + 1)}>+</button>
					</div>
				</div>
			{/if}

			<!-- Note -->
			<div class="field-group">
				<label class="field-label" for="qs-note">Кому / Заметка</label>
				<input id="qs-note" class="field-input" type="text" bind:value={note} placeholder="Клиент, особые условия…" />
			</div>

			<!-- Total summary -->
			{#if totalStems > 0}
				<div class="summary-box" class:summary-warn={!canSell}>
					<div class="summary-row">
						<span>Итого стеблей:</span>
						<strong>{totalStems}</strong>
					</div>
					<div class="summary-row">
						<span>Сумма:</span>
						<strong class="color-price">{fmt(totalAmount)}</strong>
					</div>
					{#if !canSell}
						<div class="warn-text">⚠ Недостаточно: нужно {totalStems}, есть {item.current_stock}</div>
					{/if}
				</div>
			{/if}

			{#if error}
				<div class="error-box">{error}</div>
			{/if}
		</div>

		<div class="modal-footer">
			<button class="btn-ghost" onclick={onclose}>Отмена</button>
			<button
				class="btn-confirm"
				class:btn-success={success}
				onclick={handleSell}
				disabled={!canSell || loading || success}
			>
				{#if success}✓ Продано{:else if loading}…{:else}🛒 Оформить продажу{/if}
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
		width: min(460px, calc(100vw - 32px));
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
		padding: 18px 20px 14px;
		border-bottom: 1px solid var(--glass-border);
	}

	.header-left { display: flex; align-items: center; gap: 12px; }
	.icon { font-size: 1.8rem; }
	.modal-title { font-size: 1.1rem; font-weight: 700; color: var(--color-on-surface); margin: 0; }
	.modal-subtitle { font-size: 0.8rem; color: var(--color-outline); margin: 2px 0 0; }

	.close-btn {
		width: 32px; height: 32px; border-radius: 50%;
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		color: var(--color-on-surface); cursor: pointer;
		display: flex; align-items: center; justify-content: center;
		font-size: 0.85rem; transition: background 0.15s; flex-shrink: 0;
	}
	.close-btn:hover { background: var(--glass-bg-hover); }

	.modal-body {
		padding: 18px 20px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.overview-row {
		display: flex;
		justify-content: space-around;
		padding: 12px;
		background: var(--color-surface-container);
		border-radius: 10px;
	}

	.ov-item { display: flex; flex-direction: column; align-items: center; gap: 3px; }
	.ov-val { font-size: 1.3rem; font-weight: 700; letter-spacing: -0.02em; }
	.ov-lbl { font-size: 0.62rem; text-transform: uppercase; letter-spacing: 0.07em; color: var(--color-outline); }

	.color-stock { color: #60a5fa; }
	.color-price { color: var(--color-primary); }
	.color-ok { color: #34d399; }
	.color-err { color: #f87171; }

	.field-group { display: flex; flex-direction: column; gap: 6px; }
	.field-label { font-size: 0.72rem; color: var(--color-outline); text-transform: uppercase; letter-spacing: 0.06em; font-weight: 500; }
	.count-row { display: flex; align-items: center; gap: 8px; }

	.count-btn {
		width: 36px; height: 36px; border-radius: 9px;
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		color: var(--color-on-surface); font-size: 1.1rem; cursor: pointer;
		display: flex; align-items: center; justify-content: center; flex-shrink: 0;
		transition: background 0.15s;
	}
	.count-btn:hover { background: var(--glass-bg-hover); }

	.count-input {
		flex: 1; text-align: center;
		background: var(--color-surface-container-high);
		border: 1px solid var(--glass-border); border-radius: 9px;
		padding: 8px; font-size: 1rem; font-weight: 600;
		color: var(--color-on-surface); font-family: inherit; outline: none;
	}
	.count-input:focus { border-color: var(--color-primary); }

	.field-input {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 9px; padding: 8px 12px;
		color: var(--color-on-surface); font-size: 0.875rem;
		font-family: inherit; outline: none; transition: border-color 0.15s;
	}
	.field-input:focus { border-color: var(--color-primary); }

	.summary-box {
		padding: 10px 14px; border-radius: 9px;
		background: rgba(52, 211, 153, 0.06);
		border: 1px solid rgba(52, 211, 153, 0.2);
		display: flex; flex-direction: column; gap: 5px;
	}
	.summary-box.summary-warn { background: rgba(248,113,113,0.06); border-color: rgba(248,113,113,0.2); }

	.summary-row { display: flex; justify-content: space-between; font-size: 0.875rem; color: var(--color-on-surface); }
	.warn-text { font-size: 0.78rem; color: #f87171; }

	.error-box { padding: 10px 14px; border-radius: 9px; background: rgba(248,113,113,0.08); border: 1px solid rgba(248,113,113,0.2); color: #f87171; font-size: 0.82rem; }

	.modal-footer {
		display: flex; justify-content: flex-end; gap: 10px;
		padding: 14px 20px; border-top: 1px solid var(--glass-border);
	}

	.btn-ghost {
		padding: 9px 20px; border-radius: 10px;
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		color: var(--color-on-surface); font-size: 0.875rem; cursor: pointer;
		font-family: inherit; transition: background 0.15s;
	}
	.btn-ghost:hover { background: var(--glass-bg-hover); }

	.btn-confirm {
		padding: 9px 24px; border-radius: 10px;
		background: rgba(52, 211, 153, 0.12); border: 1px solid rgba(52, 211, 153, 0.3);
		color: var(--color-primary); font-size: 0.875rem; font-weight: 600;
		cursor: pointer; font-family: inherit;
		transition: background 0.15s, transform 0.1s var(--ease-spring);
	}
	.btn-confirm:hover:not(:disabled) { background: rgba(52, 211, 153, 0.22); transform: translateY(-1px); }
	.btn-confirm:disabled { opacity: 0.4; cursor: default; }
	.btn-confirm.btn-success { background: rgba(52, 211, 153, 0.2); }
</style>
