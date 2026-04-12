<script lang="ts">
	import FlowerCard from '$lib/components/greenhouse/FlowerCard.svelte';
	import type { FlowerSort } from '$lib/tauri/types';

	interface Props {
		flowerSorts: FlowerSort[];
		onconfirm: (item: { item_id: string; quantity: number; unit_price: number }) => void;
		onclose: () => void;
	}

	let { flowerSorts, onconfirm, onclose }: Props = $props();

	let selectedSort = $state<FlowerSort | null>(null);
	let quantity = $state(1);
	let searchQuery = $state('');

	// Only show sorts with available stock
	const availableSorts = $derived(
		flowerSorts.filter((s) => {
			const q = searchQuery.toLowerCase();
			const matchesSearch = !q || s.name.toLowerCase().includes(q) || (s.variety?.toLowerCase().includes(q) ?? false);
			return matchesSearch && s.pkg_stock > 0;
		})
	);

	const unitPrice = $derived(selectedSort?.sell_price_stem ?? 0);
	const totalPrice = $derived(quantity * unitPrice);
	const maxQty = $derived(selectedSort?.pkg_stock ?? 0);
	const isOverStock = $derived(quantity > maxQty);

	function handleSelect(sort: FlowerSort) {
		selectedSort = sort;
		quantity = 1;
	}

	function handleConfirm() {
		if (!selectedSort || quantity < 1 || isOverStock) return;
		onconfirm({
			item_id: selectedSort.id,
			quantity,
			unit_price: unitPrice,
		});
	}

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdrop} role="dialog" aria-modal="true">
	<div class="modal-panel">
		<div class="modal-header">
			<h2>Добавить товар в заказ</h2>
			<button class="btn-close" type="button" onclick={onclose} aria-label="Закрыть">&#10005;</button>
		</div>

		<div class="modal-body">
			<!-- Search -->
			<div class="search-bar">
				<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round">
					<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
				</svg>
				<input type="text" bind:value={searchQuery} placeholder="Поиск по названию..." class="search-input" />
			</div>

			<!-- Cards grid -->
			<div class="cards-grid">
				{#if availableSorts.length === 0}
					<p class="empty-msg">Нет доступных товаров на складе</p>
				{:else}
					{#each availableSorts as sort (sort.id)}
						<FlowerCard
							{sort}
							compact
							selected={selectedSort?.id === sort.id}
							onclick={() => handleSelect(sort)}
						/>
					{/each}
				{/if}
			</div>

			<!-- Selection footer -->
			{#if selectedSort}
				<div class="selection-footer">
					<div class="selected-info">
						<span class="selected-name">{selectedSort.name}</span>
						{#if selectedSort.variety}
							<span class="selected-variety">{selectedSort.variety}</span>
						{/if}
						<span class="selected-stock">На складе: {selectedSort.pkg_stock} уп.</span>
					</div>

					<div class="qty-row">
						<label class="qty-label">Кол-во:</label>
						<div class="qty-stepper">
							<button type="button" class="qty-btn" onclick={() => { if (quantity > 1) quantity--; }}>&#8722;</button>
							<input type="number" min="1" max={maxQty} bind:value={quantity} class="qty-input" class:qty-err={isOverStock} />
							<button type="button" class="qty-btn" onclick={() => { if (quantity < maxQty) quantity++; }}>+</button>
						</div>

						<div class="price-info">
							<span class="price-unit">{unitPrice.toFixed(2)} / уп.</span>
							<span class="price-total">= {totalPrice.toFixed(2)}</span>
						</div>

						<button
							class="btn-add"
							type="button"
							onclick={handleConfirm}
							disabled={isOverStock || quantity < 1}
						>
							Добавить
						</button>
					</div>

					{#if isOverStock}
						<div class="stock-warning">
							<span>&#9888;</span> Недостаточно на складе (макс. {maxQty})
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(10px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2100;
		padding: 16px;
	}

	.modal-panel {
		background: var(--color-surface);
		border: 1px solid var(--glass-border);
		border-radius: 20px;
		width: 100%;
		max-width: 720px;
		max-height: 85vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		box-shadow: 0 32px 80px rgba(0, 0, 0, 0.5);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 18px 22px;
		border-bottom: 1px solid var(--glass-border);
		flex-shrink: 0;
	}

	.modal-header h2 {
		font-size: 1.05rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0;
	}

	.btn-close {
		background: none;
		border: none;
		color: var(--color-outline);
		font-size: 1rem;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 8px;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	.modal-body {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		flex: 1;
		padding: 16px 22px 22px;
		gap: 14px;
	}

	/* Search */
	.search-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 10px;
		padding: 8px 12px;
		color: var(--color-outline);
		flex-shrink: 0;
	}

	.search-input {
		flex: 1;
		background: none;
		border: none;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
	}
	.search-input::placeholder { color: var(--color-outline); opacity: 0.6; }

	/* Cards */
	.cards-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: 10px;
		overflow-y: auto;
		flex: 1;
		min-height: 120px;
		padding-right: 4px;
	}

	.empty-msg {
		grid-column: 1 / -1;
		text-align: center;
		color: var(--color-outline);
		font-size: 0.875rem;
		padding: 32px 0;
	}

	/* Selection footer */
	.selection-footer {
		flex-shrink: 0;
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 14px;
		padding: 14px 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.selected-info {
		display: flex;
		align-items: baseline;
		gap: 8px;
		flex-wrap: wrap;
	}

	.selected-name {
		font-weight: 600;
		font-size: 0.95rem;
		color: var(--color-on-surface);
	}

	.selected-variety {
		font-size: 0.8rem;
		color: var(--color-outline);
	}

	.selected-stock {
		font-size: 0.75rem;
		color: var(--color-primary);
		margin-left: auto;
		font-weight: 500;
	}

	.qty-row {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}

	.qty-label {
		font-size: 0.8rem;
		color: var(--color-outline);
		font-weight: 500;
	}

	.qty-stepper {
		display: flex;
		align-items: center;
		gap: 0;
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		overflow: hidden;
	}

	.qty-btn {
		width: 32px;
		height: 32px;
		background: none;
		border: none;
		color: var(--color-on-surface);
		font-size: 1rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.qty-btn:hover { background: var(--color-surface-container); }

	.qty-input {
		width: 48px;
		text-align: center;
		border: none;
		background: none;
		color: var(--color-on-surface);
		font-size: 0.9rem;
		font-weight: 600;
		font-family: inherit;
		outline: none;
		-moz-appearance: textfield;
	}
	.qty-input::-webkit-outer-spin-button,
	.qty-input::-webkit-inner-spin-button { -webkit-appearance: none; }
	.qty-input.qty-err { color: var(--color-alert-red, #ef4444); }

	.price-info {
		display: flex;
		align-items: baseline;
		gap: 6px;
		margin-left: auto;
	}

	.price-unit {
		font-size: 0.8rem;
		color: var(--color-outline);
	}

	.price-total {
		font-size: 1rem;
		font-weight: 700;
		color: var(--color-primary);
	}

	.btn-add {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		border: none;
		border-radius: 8px;
		padding: 8px 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
	}
	.btn-add:hover { opacity: 0.85; }
	.btn-add:disabled { opacity: 0.4; cursor: not-allowed; }

	.stock-warning {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--color-alert-red, #ef4444);
		background: rgba(239, 68, 68, 0.08);
		border: 1px solid var(--color-alert-red, #ef4444);
		border-radius: 8px;
		padding: 6px 10px;
	}

	/* Light mode */
	:global([data-theme="light"]) .modal-panel { background: var(--color-surface, #fafafa); }
</style>
