<script lang="ts">
	import { onMount } from 'svelte';
	import {
		flowerSorts,
		flowerConstants,
		flowerFinancials,
		totalRawStems,
		totalPacks,
		sortsBySpecies,
	} from '$lib/stores/flowers';
	import type { FlowerSort, FlowerConstants, UpdateFlowerSortPayload } from '$lib/tauri/types';

	// ── Local state ───────────────────────────────────────────
	let editingSort = $state<FlowerSort | null>(null);
	let showAddForm = $state(false);
	let newName = $state('');
	let newVariety = $state('');
	let newColorHex = $state('#34d399');
	let constantsDraft = $state<FlowerConstants>({ ...$flowerConstants });
	let constantsEditing = $state(false);
	let view = $state<'raw' | 'packaged' | 'all'>('all');

	// Keep constantsDraft in sync when store updates
	$effect(() => {
		constantsDraft = { ...$flowerConstants };
	});

	onMount(async () => {
		await flowerSorts.load();
		await flowerConstants.load();
	});

	// ── Derived ───────────────────────────────────────────────
	const visibleSorts = $derived.by(() => {
		if (view === 'raw') return $flowerSorts.filter((s) => s.raw_stock > 0);
		if (view === 'packaged') return $flowerSorts.filter((s) => s.pkg_stock > 0);
		return $flowerSorts;
	});

	// ── Actions ───────────────────────────────────────────────
	async function handleAddSort() {
		if (!newName.trim()) return;
		await flowerSorts.create(newName.trim(), newVariety.trim() || undefined, newColorHex);
		newName = '';
		newVariety = '';
		newColorHex = '#34d399';
		showAddForm = false;
	}

	async function handleUpdateSort() {
		if (!editingSort) return;
		await flowerSorts.updateSort({
			id: editingSort.id,
			name: editingSort.name,
			variety: editingSort.variety,
			color_hex: editingSort.color_hex,
		});
		editingSort = null;
	}

	async function handleAdjust(sort: FlowerSort, rawDelta: number, pkgDelta: number) {
		await flowerSorts.adjustStock(sort.id, rawDelta, pkgDelta);
	}

	async function handleDeleteSort(id: string) {
		await flowerSorts.remove(id);
	}

	async function handleSaveConstants() {
		await flowerConstants.save(constantsDraft);
		constantsEditing = false;
	}

	function fmtMoney(v: number) {
		return new Intl.NumberFormat('ru-RU', { style: 'currency', currency: 'RUB', maximumFractionDigits: 0 }).format(v);
	}

	function fmtWeight(v: number) {
		return v >= 1 ? `${v.toFixed(1)} кг` : `${(v * 1000).toFixed(0)} г`;
	}
</script>

<div class="page">

	<!-- ═══════════════════════════════════════════════════════ -->
	<!-- CONSTANTS ISLAND (detached, floats bottom-right)       -->
	<!-- ═══════════════════════════════════════════════════════ -->
	<div class="constants-island glass">
		<div class="island-header">
			<span class="island-title">Константы</span>
			{#if !constantsEditing}
				<button class="island-edit-btn" onclick={() => (constantsEditing = true)}>✎</button>
			{/if}
		</div>

		{#if constantsEditing}
			<div class="constants-grid editing">
				<label class="const-label">
					Вес/стебель (кг)
					<input class="glass-input" type="number" step="0.001" bind:value={constantsDraft.weight_per_flower} />
				</label>
				<label class="const-label">
					Стеблей/упак.
					<input class="glass-input" type="number" step="1" bind:value={constantsDraft.flowers_per_pack} />
				</label>
				<label class="const-label">
					Цена упак. (₽)
					<input class="glass-input" type="number" step="10" bind:value={constantsDraft.price_per_pack} />
				</label>
				<label class="const-label">
					Цена/стебель (₽)
					<input class="glass-input" type="number" step="1" bind:value={constantsDraft.price_per_flower} />
				</label>
			</div>
			<div class="island-actions">
				<button class="glass-btn accent" onclick={handleSaveConstants}>Сохранить</button>
				<button class="glass-btn" onclick={() => { constantsEditing = false; constantsDraft = { ...$flowerConstants }; }}>Отмена</button>
			</div>
		{:else}
			<div class="constants-grid">
				<div class="const-item">
					<span class="const-key">Вес/стебель</span>
					<span class="const-val badge purple">{$flowerConstants.weight_per_flower} кг</span>
				</div>
				<div class="const-item">
					<span class="const-key">Стеблей/уп.</span>
					<span class="const-val badge blue">{$flowerConstants.flowers_per_pack}</span>
				</div>
				<div class="const-item">
					<span class="const-key">Цена уп.</span>
					<span class="const-val badge green">{fmtMoney($flowerConstants.price_per_pack)}</span>
				</div>
				<div class="const-item">
					<span class="const-key">Цена/стеб.</span>
					<span class="const-val badge amber">{fmtMoney($flowerConstants.price_per_flower)}</span>
				</div>
			</div>
		{/if}
	</div>

	<!-- ═══════════════════════════════════════════════════════ -->
	<!-- HEADER + AGGREGATE STATS                                -->
	<!-- ═══════════════════════════════════════════════════════ -->
	<div class="page-header">
		<h1 class="page-title">Цветы</h1>
		<p class="page-sub">Склад и аналитика</p>
	</div>

	<!-- KPI strip -->
	<div class="kpi-strip">
		<div class="kpi glass">
			<span class="kpi-icon">🌸</span>
			<span class="kpi-val">{$totalRawStems}</span>
			<span class="kpi-label">Сырых стеблей</span>
		</div>
		<div class="kpi glass">
			<span class="kpi-icon">📦</span>
			<span class="kpi-val">{$totalPacks}</span>
			<span class="kpi-label">Упаковок готово</span>
		</div>
		<div class="kpi glass">
			<span class="kpi-icon">⚖️</span>
			<span class="kpi-val">{fmtWeight($flowerFinancials.totalWeight)}</span>
			<span class="kpi-label">Общий вес</span>
		</div>
		<div class="kpi glass accent">
			<span class="kpi-icon">💰</span>
			<span class="kpi-val">{fmtMoney($flowerFinancials.totalValue)}</span>
			<span class="kpi-label">Общая стоимость</span>
		</div>
		<div class="kpi glass">
			<span class="kpi-icon">🎁</span>
			<span class="kpi-val">{$flowerFinancials.potentialPacks}</span>
			<span class="kpi-label">Потенц. упаковок</span>
		</div>
	</div>

	<!-- ═══════════════════════════════════════════════════════ -->
	<!-- SPLIT-VIEW CONTROLS                                     -->
	<!-- ═══════════════════════════════════════════════════════ -->
	<div class="view-controls">
		<div class="view-tabs">
			{#each [['all', 'Все'], ['raw', 'Сырьё'], ['packaged', 'Упакованные']] as [v, label]}
				<button
					class="view-tab"
					class:active={view === v}
					onclick={() => (view = v as typeof view)}
				>
					{label}
				</button>
			{/each}
		</div>

		<button class="add-btn glass-btn accent" onclick={() => (showAddForm = !showAddForm)}>
			{showAddForm ? '✕ Отмена' : '+ Добавить сорт'}
		</button>
	</div>

	<!-- Add sort form -->
	{#if showAddForm}
		<div class="add-form glass">
			<div class="form-row">
				<input class="glass-input" type="text" placeholder="Вид (напр. Тюльпан)" bind:value={newName} />
				<input class="glass-input" type="text" placeholder="Сорт (напр. Queen of Night)" bind:value={newVariety} />
				<input type="color" class="color-picker" bind:value={newColorHex} title="Цвет" />
				<button class="glass-btn accent" onclick={handleAddSort}>Добавить</button>
			</div>
		</div>
	{/if}

	<!-- ═══════════════════════════════════════════════════════ -->
	<!-- SPLIT VIEW                                              -->
	<!-- ═══════════════════════════════════════════════════════ -->
	{#if view === 'all'}
		<div class="split-view">
			<!-- LEFT: Raw / Сырьё -->
			<div class="split-panel">
				<div class="panel-header">
					<span class="panel-badge raw">Сырьё (стебли)</span>
					<span class="panel-total">{$totalRawStems} шт</span>
				</div>
				<div class="sort-list">
					{#each $flowerSorts.filter((s) => s.raw_stock > 0) as sort (sort.id)}
						<div class="sort-card glass">
							<div class="sort-color" style="background: {sort.color_hex ?? '#888'}"></div>
							<div class="sort-info">
								<span class="sort-name">{sort.name}</span>
								{#if sort.variety}<span class="sort-variety">{sort.variety}</span>{/if}
							</div>
							<div class="sort-meta">
								<span class="stock-badge raw">{sort.raw_stock}</span>
								<div class="adj-btns">
									<button class="adj-btn" onclick={() => handleAdjust(sort, -1, 0)}>−</button>
									<button class="adj-btn pos" onclick={() => handleAdjust(sort, 1, 0)}>+</button>
								</div>
							</div>
						</div>
					{:else}
						<div class="empty-panel">Нет сырых стеблей</div>
					{/each}
				</div>
			</div>

			<!-- RIGHT: Packaged / Упакованные -->
			<div class="split-panel">
				<div class="panel-header">
					<span class="panel-badge pkg">Упакованные</span>
					<span class="panel-total">{$totalPacks} уп. · {fmtMoney($flowerFinancials.packValue)}</span>
				</div>
				<div class="sort-list">
					{#each $flowerSorts.filter((s) => s.pkg_stock > 0) as sort (sort.id)}
						<div class="sort-card glass">
							<div class="sort-color" style="background: {sort.color_hex ?? '#888'}"></div>
							<div class="sort-info">
								<span class="sort-name">{sort.name}</span>
								{#if sort.variety}<span class="sort-variety">{sort.variety}</span>{/if}
							</div>
							<div class="sort-meta">
								<span class="stock-badge pkg">{sort.pkg_stock}</span>
								<span class="sort-value">{fmtMoney(sort.pkg_stock * $flowerConstants.price_per_pack)}</span>
								<div class="adj-btns">
									<button class="adj-btn" onclick={() => handleAdjust(sort, 0, -1)}>−</button>
									<button class="adj-btn pos" onclick={() => handleAdjust(sort, 0, 1)}>+</button>
								</div>
							</div>
						</div>
					{:else}
						<div class="empty-panel">Нет упакованных</div>
					{/each}
				</div>
			</div>
		</div>
	{:else}
		<!-- ── Single-view (filtered) ── -->
		<div class="full-list">
			{#each visibleSorts as sort (sort.id)}
				{#if editingSort?.id === sort.id}
					<!-- Inline edit form -->
					<div class="sort-card glass editing-card">
						<div class="sort-color" style="background: {editingSort.color_hex ?? '#888'}"></div>
						<div class="edit-fields">
							<input class="glass-input sm" type="text" bind:value={editingSort.name} placeholder="Вид" />
							<input class="glass-input sm" type="text" bind:value={editingSort.variety} placeholder="Сорт" />
							<input type="color" class="color-picker sm" bind:value={editingSort.color_hex} />
						</div>
						<div class="edit-actions">
							<button class="glass-btn accent" onclick={handleUpdateSort}>✓</button>
							<button class="glass-btn" onclick={() => (editingSort = null)}>✕</button>
						</div>
					</div>
				{:else}
					<div class="sort-card glass">
						<div class="sort-color" style="background: {sort.color_hex ?? '#888'}"></div>
						<div class="sort-info">
							<span class="sort-name">{sort.name}</span>
							{#if sort.variety}<span class="sort-variety">{sort.variety}</span>{/if}
						</div>
						<div class="sort-meta">
							<span class="stock-badge raw" title="Сырьё">{sort.raw_stock} ст.</span>
							<span class="stock-badge pkg" title="Упаковок">{sort.pkg_stock} уп.</span>
							<div class="adj-btns">
								<button class="adj-btn" title="Сырьё -1" onclick={() => handleAdjust(sort, -1, 0)}>−R</button>
								<button class="adj-btn pos" title="Сырьё +1" onclick={() => handleAdjust(sort, 1, 0)}>+R</button>
								<button class="adj-btn" title="Упак. -1" onclick={() => handleAdjust(sort, 0, -1)}>−P</button>
								<button class="adj-btn pos" title="Упак. +1" onclick={() => handleAdjust(sort, 0, 1)}>+P</button>
							</div>
						</div>
						<div class="sort-actions">
							<button class="action-btn" onclick={() => (editingSort = { ...sort })}>✎</button>
							<button class="action-btn del" onclick={() => handleDeleteSort(sort.id)}>✕</button>
						</div>
					</div>
				{/if}
			{:else}
				<div class="empty-state">Нет цветов в этом разделе</div>
			{/each}
		</div>
	{/if}

	<!-- ═══════════════════════════════════════════════════════ -->
	<!-- CATEGORY BREAKDOWN (by species)                        -->
	<!-- ═══════════════════════════════════════════════════════ -->
	{#if $flowerSorts.length > 0}
		<div class="breakdown-section">
			<h2 class="breakdown-title">По видам</h2>
			<div class="breakdown-grid">
				{#each [...$sortsBySpecies.entries()] as [species, sorts]}
					{@const totalRaw = sorts.reduce((s, x) => s + x.raw_stock, 0)}
					{@const totalPkg = sorts.reduce((s, x) => s + x.pkg_stock, 0)}
					{#if totalRaw > 0 || totalPkg > 0}
						<div class="breakdown-card glass">
							<div class="breakdown-color" style="background: {sorts[0].color_hex ?? '#888'}"></div>
							<div class="breakdown-info">
								<span class="breakdown-name">{species}</span>
								<span class="breakdown-count">{sorts.length} {sorts.length === 1 ? 'сорт' : 'сорта'}</span>
							</div>
							<div class="breakdown-stocks">
								<span class="stock-badge raw">{totalRaw} ст.</span>
								<span class="stock-badge pkg">{totalPkg} уп.</span>
							</div>
						</div>
					{/if}
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.page {
		max-width: 1100px;
		margin: 0 auto;
		padding-bottom: 120px;
		position: relative;
	}

	/* ── Page header ─────────────────────────── */
	.page-header {
		margin-bottom: 20px;
	}

	.page-title {
		font-size: 1.8rem;
		font-weight: 700;
		color: var(--color-primary);
	}

	.page-sub {
		color: var(--color-outline);
		font-size: 0.85rem;
		margin-top: 2px;
	}

	/* ── Glass base ──────────────────────────── */
	.glass {
		background: var(--glass-bg);
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		box-shadow: var(--glass-shadow);
		border-radius: var(--glass-radius);
	}

	/* ── KPI strip ───────────────────────────── */
	.kpi-strip {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
		margin-bottom: 24px;
	}

	.kpi {
		flex: 1;
		min-width: 140px;
		padding: 16px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		text-align: center;
	}

	.kpi.accent {
		background: rgba(52, 211, 153, 0.08);
		border-color: rgba(52, 211, 153, 0.22);
	}

	.kpi-icon { font-size: 1.3rem; }
	.kpi-val  { font-size: 1.4rem; font-weight: 700; color: var(--color-on-surface); }
	.kpi-label { font-size: 0.72rem; color: var(--color-outline); }

	/* ── Constants island ────────────────────── */
	.constants-island {
		position: fixed;
		bottom: 90px;
		right: 20px;
		width: 240px;
		z-index: 800;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.island-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.island-title {
		font-size: 0.72rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-outline);
	}

	.island-edit-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--color-outline);
		font-size: 0.9rem;
		padding: 2px 6px;
		border-radius: 6px;
		transition: color 0.15s;
	}

	.island-edit-btn:hover { color: var(--color-primary); }

	.constants-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 6px;
	}

	.constants-grid.editing {
		grid-template-columns: 1fr;
	}

	.const-item {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.const-key {
		font-size: 0.65rem;
		color: var(--color-outline);
		white-space: nowrap;
	}

	.const-val {
		font-size: 0.82rem;
		font-weight: 600;
	}

	.const-label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 0.72rem;
		color: var(--color-outline);
	}

	.island-actions {
		display: flex;
		gap: 6px;
	}

	/* ── Badges ──────────────────────────────── */
	.badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: 6px;
		font-size: 0.78rem;
		font-weight: 600;
	}

	.badge.green {
		background: rgba(52, 211, 153, 0.12);
		color: #34d399;
	}

	.badge.blue {
		background: rgba(96, 165, 250, 0.12);
		color: #60a5fa;
	}

	.badge.purple {
		background: rgba(167, 139, 250, 0.12);
		color: #a78bfa;
	}

	.badge.amber {
		background: rgba(251, 191, 36, 0.12);
		color: #fbbf24;
	}

	/* ── View controls ───────────────────────── */
	.view-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 14px;
		flex-wrap: wrap;
		gap: 10px;
	}

	.view-tabs {
		display: flex;
		gap: 4px;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 4px;
	}

	.view-tab {
		background: none;
		border: none;
		color: var(--color-outline);
		font-size: 0.82rem;
		padding: 6px 14px;
		border-radius: 7px;
		cursor: pointer;
		transition: all 0.15s var(--ease-spring);
	}

	.view-tab.active {
		background: var(--color-primary);
		color: #000;
		font-weight: 600;
	}

	.add-btn { font-size: 0.82rem; }

	/* ── Add form ────────────────────────────── */
	.add-form {
		padding: 14px;
		margin-bottom: 16px;
	}

	.form-row {
		display: flex;
		gap: 8px;
		align-items: center;
		flex-wrap: wrap;
	}

	/* ── Split view ──────────────────────────── */
	.split-view {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
		margin-bottom: 32px;
	}

	@media (max-width: 640px) {
		.split-view { grid-template-columns: 1fr; }
	}

	.split-panel {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 2px;
	}

	.panel-badge {
		font-size: 0.72rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		padding: 3px 10px;
		border-radius: 6px;
	}

	.panel-badge.raw {
		background: rgba(251, 191, 36, 0.10);
		color: #fbbf24;
	}

	.panel-badge.pkg {
		background: rgba(52, 211, 153, 0.10);
		color: #34d399;
	}

	.panel-total {
		font-size: 0.78rem;
		color: var(--color-outline);
	}

	.sort-list, .full-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	/* ── Sort card ───────────────────────────── */
	.sort-card {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		transition: box-shadow 0.2s var(--ease-spring);
	}

	.sort-card:hover {
		box-shadow: var(--glass-shadow-hover);
	}

	.sort-color {
		width: 10px;
		height: 32px;
		border-radius: 5px;
		flex-shrink: 0;
	}

	.sort-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}

	.sort-name {
		font-size: 0.88rem;
		font-weight: 600;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sort-variety {
		font-size: 0.74rem;
		color: var(--color-outline);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sort-meta {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-shrink: 0;
	}

	.sort-value {
		font-size: 0.74rem;
		color: var(--color-primary);
	}

	.stock-badge {
		font-size: 0.72rem;
		font-weight: 700;
		padding: 3px 8px;
		border-radius: 6px;
		white-space: nowrap;
	}

	.stock-badge.raw {
		background: rgba(251, 191, 36, 0.12);
		color: #fbbf24;
		border: 1px solid rgba(251, 191, 36, 0.25);
	}

	.stock-badge.pkg {
		background: rgba(52, 211, 153, 0.12);
		color: #34d399;
		border: 1px solid rgba(52, 211, 153, 0.25);
	}

	/* ── Adjust buttons ──────────────────────── */
	.adj-btns {
		display: flex;
		gap: 2px;
	}

	.adj-btn {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 5px;
		color: var(--color-on-surface);
		font-size: 0.70rem;
		padding: 3px 7px;
		cursor: pointer;
		transition: background 0.12s;
	}

	.adj-btn:hover { background: rgba(255, 255, 255, 0.10); }

	.adj-btn.pos {
		background: rgba(52, 211, 153, 0.08);
		border-color: rgba(52, 211, 153, 0.20);
		color: #34d399;
	}

	.adj-btn.pos:hover { background: rgba(52, 211, 153, 0.16); }

	/* ── Sort actions ────────────────────────── */
	.sort-actions {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
	}

	.action-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--color-outline);
		font-size: 0.85rem;
		padding: 3px 7px;
		border-radius: 6px;
		transition: color 0.15s, background 0.15s;
	}

	.action-btn:hover { color: var(--color-on-surface); background: rgba(255,255,255,0.06); }
	.action-btn.del:hover { color: #f87171; background: rgba(248,113,113,0.08); }

	/* ── Edit card ───────────────────────────── */
	.editing-card { flex-wrap: wrap; gap: 8px; }

	.edit-fields {
		display: flex;
		gap: 6px;
		flex: 1;
		flex-wrap: wrap;
	}

	.edit-actions { display: flex; gap: 4px; }

	/* ── Inputs ──────────────────────────────── */
	.glass-input {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.10);
		border-radius: 8px;
		color: var(--color-on-surface);
		font-size: 0.85rem;
		padding: 7px 10px;
		outline: none;
		transition: border-color 0.15s;
		flex: 1;
		min-width: 80px;
	}

	.glass-input:focus { border-color: var(--color-primary); }
	.glass-input::placeholder { color: var(--color-outline); }
	.glass-input.sm { font-size: 0.80rem; padding: 5px 8px; }

	.color-picker {
		width: 36px;
		height: 34px;
		border: 1px solid rgba(255,255,255,0.10);
		border-radius: 8px;
		background: none;
		cursor: pointer;
		padding: 2px;
		flex-shrink: 0;
	}

	.color-picker.sm { width: 30px; height: 28px; }

	/* ── Buttons ─────────────────────────────── */
	.glass-btn {
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.10);
		border-radius: 9px;
		color: var(--color-on-surface);
		font-size: 0.82rem;
		padding: 8px 14px;
		cursor: pointer;
		transition: background 0.15s;
		white-space: nowrap;
	}

	.glass-btn:hover { background: rgba(255, 255, 255, 0.10); }

	.glass-btn.accent {
		background: rgba(52, 211, 153, 0.12);
		border-color: rgba(52, 211, 153, 0.30);
		color: var(--color-primary);
	}

	.glass-btn.accent:hover { background: rgba(52, 211, 153, 0.20); }

	/* ── Breakdown ───────────────────────────── */
	.breakdown-section { margin-top: 32px; }

	.breakdown-title {
		font-size: 0.90rem;
		font-weight: 600;
		color: var(--color-outline);
		margin-bottom: 12px;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.breakdown-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 10px;
	}

	.breakdown-card {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
	}

	.breakdown-color {
		width: 8px;
		height: 28px;
		border-radius: 4px;
		flex-shrink: 0;
	}

	.breakdown-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.breakdown-name { font-size: 0.85rem; font-weight: 600; }
	.breakdown-count { font-size: 0.72rem; color: var(--color-outline); }

	.breakdown-stocks { display: flex; gap: 4px; flex-wrap: wrap; }

	/* ── Empty states ────────────────────────── */
	.empty-panel, .empty-state {
		text-align: center;
		padding: 24px;
		color: var(--color-outline);
		font-size: 0.85rem;
		font-style: italic;
	}
</style>
