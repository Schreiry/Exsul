<script lang="ts">
	import { flowerSorts } from '$lib/stores/flowers';
	import type { FlowerSort } from '$lib/tauri/types';

	interface Props {
		sorts: FlowerSort[];
	}

	let { sorts }: Props = $props();

	let editingId = $state<string | null>(null);
	let draft = $state<Partial<FlowerSort>>({});

	function startEdit(sort: FlowerSort) {
		editingId = sort.id;
		draft = {
			name: sort.name,
			variety: sort.variety,
			purchase_price: sort.purchase_price,
			sell_price_stem: sort.sell_price_stem,
			flowers_per_pack_override: sort.flowers_per_pack_override,
		};
	}

	async function saveEdit(id: string) {
		await flowerSorts.updateSort({
			id,
			name: (draft.name as string) || undefined,
			variety: (draft.variety as string) || undefined,
			purchase_price: draft.purchase_price as number,
			sell_price_stem: draft.sell_price_stem as number,
			flowers_per_pack_override: draft.flowers_per_pack_override as number | undefined,
		});
		editingId = null;
	}

	async function removeSortConfirm(sort: FlowerSort) {
		if (!confirm(`Удалить "${sort.name}"?`)) return;
		await flowerSorts.remove(sort.id);
	}
</script>

<div class="sort-manager">
	<h3 class="panel-title">Управление сортами</h3>
	{#if sorts.length === 0}
		<p class="empty">Сорты не добавлены</p>
	{:else}
		<div class="sort-list">
			{#each sorts as sort (sort.id)}
				{#if editingId === sort.id}
					<div class="sort-row editing">
						<div class="edit-fields">
							<input class="cell-input" type="text" bind:value={draft.name} placeholder="Название" />
							<input class="cell-input" type="text" bind:value={draft.variety} placeholder="Сорт" />
							<input class="cell-input num" type="number" bind:value={draft.purchase_price} placeholder="Закупка" min="0" step="0.01" />
							<input class="cell-input num" type="number" bind:value={draft.sell_price_stem} placeholder="Продажа/шт." min="0" step="0.01" />
							<input class="cell-input num" type="number" bind:value={draft.flowers_per_pack_override} placeholder="Шт./уп." min="1" />
						</div>
						<div class="row-actions">
							<button type="button" class="btn-save" onclick={() => saveEdit(sort.id)}>✓</button>
							<button type="button" class="btn-cancel" onclick={() => (editingId = null)}>✕</button>
						</div>
					</div>
				{:else}
					<div class="sort-row">
						<div class="sort-info">
							<span class="sort-name">{sort.name}</span>
							{#if sort.variety}<span class="sort-variety">{sort.variety}</span>{/if}
						</div>
						<div class="sort-meta">
							<span class="meta-chip">{sort.raw_stock} шт.</span>
							{#if sort.sell_price_stem > 0}
								<span class="meta-chip">{sort.sell_price_stem}/шт.</span>
							{/if}
						</div>
						<div class="row-actions">
							<button type="button" class="btn-edit" onclick={() => startEdit(sort)} title="Редактировать">
								<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
							</button>
							<button type="button" class="btn-del" onclick={() => removeSortConfirm(sort)} title="Удалить">
								<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/><path d="M9 6V4h6v2"/></svg>
							</button>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style>
	.sort-manager {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 14px;
		padding: 16px;
		margin-bottom: 20px;
	}

	.panel-title {
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-outline);
		margin: 0 0 12px;
	}

	.empty { font-size: 0.85rem; color: var(--color-outline); margin: 0; }

	.sort-list { display: flex; flex-direction: column; gap: 4px; }

	.sort-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: 10px;
		background: transparent;
		transition: background 0.1s;
	}
	.sort-row:hover { background: var(--glass-bg-hover); }
	.sort-row.editing { background: var(--glass-bg-hover); flex-wrap: wrap; }

	.sort-info { flex: 1; display: flex; flex-direction: column; gap: 1px; min-width: 0; }
	.sort-name { font-size: 0.88rem; font-weight: 600; color: var(--color-on-surface); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
	.sort-variety { font-size: 0.72rem; color: var(--color-outline); }

	.sort-meta { display: flex; gap: 6px; flex-shrink: 0; }
	.meta-chip { font-size: 0.7rem; padding: 2px 6px; background: var(--glass-bg); border: 1px solid var(--glass-border); border-radius: 20px; color: var(--color-outline); }

	.row-actions { display: flex; gap: 4px; flex-shrink: 0; }

	.btn-edit, .btn-del, .btn-save, .btn-cancel {
		background: none; border: none; cursor: pointer;
		padding: 4px 6px; border-radius: 6px; display: flex; align-items: center;
		transition: background 0.1s, color 0.1s;
	}
	.btn-edit { color: var(--color-outline); }
	.btn-edit:hover { color: var(--color-primary); background: var(--glass-bg); }
	.btn-del { color: var(--color-outline); }
	.btn-del:hover { color: var(--color-alert-red); background: rgba(239,68,68,0.1); }
	.btn-save { color: var(--color-primary); }
	.btn-cancel { color: var(--color-outline); }

	.edit-fields {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		flex: 1;
	}

	.cell-input {
		background: var(--color-surface);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 5px 8px;
		font-size: 0.82rem;
		color: var(--color-on-surface);
		outline: none;
		min-width: 80px;
	}
	.cell-input:focus { border-color: var(--color-primary); }
	.cell-input.num { max-width: 90px; }
</style>
