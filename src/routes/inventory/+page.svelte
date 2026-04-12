<script lang="ts">
	import { inventory, categories as categoryStrings } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { preset } from '$lib/stores/preset';
	import { t } from '$lib/stores/i18n';
	import { commands } from '$lib/tauri/commands';
	import LiquidGlassCard from '$lib/components/LiquidGlassCard.svelte';
	import ItemDetailModal from '$lib/components/inventory/ItemDetailModal.svelte';
	import PackAssignmentModal from '$lib/components/flowers/PackAssignmentModal.svelte';
	import QuickSellModal from '$lib/components/inventory/QuickSellModal.svelte';
	import PackModal from '$lib/components/warehouse/PackModal.svelte';
	import type { CreateItemPayload, Item, Category, FlowerSort } from '$lib/tauri/types';

	let showForm = $state(false);
	let showCategoryManager = $state(false);
	let showSortManager = $state(false);
	let selectedItem = $state<Item | null>(null);
	let packingItem = $state<Item | null>(null);
	let sellingItem = $state<Item | null>(null);
	let packOpen = $state(false);

	// Load flower sorts + constants when in flowers preset
	$effect(() => {
		if ($preset === 'flowers') {
			flowerSorts.load();
			flowerConstants.load();
		}
	});

	// ── Warehouse view: filtered flower sorts ─────────────────
	const warehouseSorts = $derived.by(() => {
		if ($preset !== 'flowers') return [];
		let list = $flowerSorts;
		if (searchQuery) {
			const q = searchQuery.toLowerCase();
			list = list.filter(
				(s) => s.name.toLowerCase().includes(q) || (s.variety ?? '').toLowerCase().includes(q)
			);
		}
		if (filterCategory) {
			list = list.filter((s) => s.name === filterCategory);
		}
		return list;
	});

	const warehouseTotalPkg = $derived(warehouseSorts.reduce((s, f) => s + f.pkg_stock, 0));
	const warehouseTotalRaw = $derived(warehouseSorts.reduce((s, f) => s + f.raw_stock, 0));

	// ── Sort manager state ─────────────────────────────────────
	let newSortName = $state('');
	let newSortVariety = $state('');
	let newSortColor = $state('#f472b6');
	let newSortPurchasePrice = $state(0);
	let newSortSellPrice = $state(0);
	let editingSortId = $state<string | null>(null);
	let editingSort = $state<FlowerSort | null>(null);

	async function handleCreateSort() {
		if (!newSortName.trim()) return;
		await flowerSorts.create({
			name: newSortName.trim(),
			variety: newSortVariety.trim() || undefined,
			color_hex: newSortColor,
			purchase_price: newSortPurchasePrice || undefined,
			sell_price_stem: newSortSellPrice || undefined,
		});
		newSortName = '';
		newSortVariety = '';
		newSortColor = '#f472b6';
		newSortPurchasePrice = 0;
		newSortSellPrice = 0;
	}

	function startEditSort(sort: FlowerSort) {
		editingSortId = sort.id;
		editingSort = { ...sort };
	}

	async function handleSaveSort() {
		if (!editingSort) return;
		await flowerSorts.updateSort({
			id: editingSort.id,
			name: editingSort.name,
			variety: editingSort.variety,
			color_hex: editingSort.color_hex,
			purchase_price: editingSort.purchase_price,
			sell_price_stem: editingSort.sell_price_stem,
			flowers_per_pack_override: editingSort.flowers_per_pack_override ?? undefined,
		});
		editingSortId = null;
		editingSort = null;
	}

	async function handleDeleteSort(id: string) {
		await flowerSorts.remove(id);
	}

	let formData = $state<CreateItemPayload>({
		name: '',
		category: '',
		category_id: '',
		price: 0,
		production_cost: 0,
		initial_stock: 0,
	});

	// Image upload state
	let isDragOver = $state(false);
	let previewUrl = $state<string | null>(null);
	let pendingImageBase64 = $state<string | null>(null);

	// Resolved app data dir for serving images via asset protocol
	let appDataDirPath = $state('');
	$effect(() => {
		if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
			import('@tauri-apps/api/path').then(({ appDataDir }) =>
				appDataDir().then((dir) => { appDataDirPath = dir; })
			);
		}
	});

	let searchQuery = $state('');
	let filterCategory = $state('');
	let sortKey = $state<string | null>(null);
	let sortDir = $state<'asc' | 'desc'>('asc');
	let analyticsOpen = $state(typeof window !== 'undefined' && window.innerWidth >= 1024);

	function setSort(key: string) {
		if (sortKey === key) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	const filteredItems = $derived(
		$inventory.filter((item) => {
			const matchesSearch =
				!searchQuery ||
				item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
				item.category.toLowerCase().includes(searchQuery.toLowerCase());
			const matchesCategory = !filterCategory || item.category === filterCategory;
			return matchesSearch && matchesCategory;
		})
	);

	const sortedItems = $derived.by(() => {
		const base = filteredItems.slice();
		const key = sortKey;
		if (!key) return base;
		return base.sort((a, b) => {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const va = (a as any)[key], vb = (b as any)[key];
			const cmp =
				typeof va === 'string'
					? va.localeCompare(vb, ['ru', 'en'], { sensitivity: 'base' })
					: (va as number) - (vb as number);
			return sortDir === 'asc' ? cmp : -cmp;
		});
	});

	const analytics = $derived({
		totalItems: $inventory.length,
		totalStock: $inventory.reduce((s, i) => s + i.current_stock, 0),
		totalCost: $inventory.reduce((s, i) => s + i.production_cost * i.current_stock, 0),
		totalValue: $inventory.reduce((s, i) => s + i.current_price * i.current_stock, 0),
		avgMargin: (() => {
			const totalStock = $inventory.reduce((s, i) => s + i.current_stock, 0);
			if (totalStock === 0) return 0;
			const weightedMargin = $inventory.reduce((s, i) => {
				const margin = i.current_price > 0
					? (i.current_price - i.production_cost) / i.current_price
					: 0;
				return s + margin * i.current_stock;
			}, 0);
			return Math.round((weightedMargin / totalStock) * 100);
		})(),
		totalSold: $inventory.reduce((s, i) => s + i.sold_count, 0),
	});

	// ── Category manager state ─────────────────────────────────
	let newCatName = $state('');
	let newCatColor = $state('#34d399');
	let editingCatId = $state<string | null>(null);
	let editingCatName = $state('');
	let editingCatColor = $state('');

	async function handleCreateCategory() {
		if (!newCatName.trim()) return;
		await categories.create({ name: newCatName.trim(), color: newCatColor });
		newCatName = '';
		newCatColor = '#34d399';
	}

	function startEditCat(cat: Category) {
		editingCatId = cat.id;
		editingCatName = cat.name;
		editingCatColor = cat.color ?? '#34d399';
	}

	async function handleSaveCategory() {
		if (!editingCatId) return;
		await categories.update({ id: editingCatId, name: editingCatName.trim() || undefined, color: editingCatColor });
		editingCatId = null;
	}

	async function handleDeleteCategory(id: string) {
		await categories.remove(id);
	}

	// ── Item form ──────────────────────────────────────────────
	function handleImageDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		const file = e.dataTransfer?.files?.[0];
		if (!file || !file.type.startsWith('image/')) return;
		if (file.size > 5 * 1024 * 1024) return;
		readImageFile(file);
	}

	function handleImageClick() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = 'image/*';
		input.onchange = (e) => {
			const file = (e.target as HTMLInputElement).files?.[0];
			if (file && file.size <= 5 * 1024 * 1024) readImageFile(file);
		};
		input.click();
	}

	function readImageFile(file: File) {
		const reader = new FileReader();
		reader.onload = (ev) => {
			const dataUrl = ev.target?.result as string;
			previewUrl = dataUrl;
			const comma = dataUrl.indexOf(',');
			pendingImageBase64 = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
		};
		reader.readAsDataURL(file);
	}

	function onCategorySelect(e: Event) {
		const id = (e.currentTarget as HTMLSelectElement).value;
		formData.category_id = id;
		const cat = $categories.find((c) => c.id === id);
		formData.category = cat?.name ?? '';
	}

	async function handleDeleteAll() {
		const count = $inventory.length;
		if (count === 0) return;
		if (!confirm(`Удалить все ${count} товаров? Это действие необратимо.`)) return;
		await commands.deleteAllItems();
		await inventory.load();
	}

	async function handleSubmit() {
		if (!formData.name || formData.price <= 0) return;
		const newId = await inventory.addItem({
			name: formData.name,
			category: formData.category || undefined,
			category_id: formData.category_id || undefined,
			price: formData.price,
			production_cost: formData.production_cost || undefined,
			initial_stock: formData.initial_stock || undefined,
		});
		if (pendingImageBase64 && newId) {
			try {
				await commands.saveItemImage(newId, pendingImageBase64);
				await inventory.load();
			} catch (e) {
				console.error('Image upload failed:', e);
			}
		}
		formData = { name: '', category: '', category_id: '', price: 0, production_cost: 0, initial_stock: 0 };
		previewUrl = null;
		pendingImageBase64 = null;
		showForm = false;
	}
</script>

<div class="inventory-page">

{#if $preset === 'flowers'}
	<!-- ══ WAREHOUSE VIEW (flowers preset) ═══════════════════════ -->
	<div class="warehouse-view">

		<div class="wh-header">
			<div>
				<h1 class="wh-title">Склад</h1>
				<p class="wh-sub">Упакованные букеты</p>
			</div>
			<button type="button" class="btn-pack" onclick={() => (packOpen = true)}>
				<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>
				Упаковать
			</button>
		</div>

		<!-- KPI strip -->
		<div class="wh-kpi">
			<div class="wh-kpi-card">
				<span class="wh-kpi-val">{warehouseTotalPkg}</span>
				<span class="wh-kpi-label">Упаковок на складе</span>
			</div>
			<div class="wh-kpi-card">
				<span class="wh-kpi-val">{warehouseTotalRaw}</span>
				<span class="wh-kpi-label">Стеблей в наличии</span>
			</div>
			<div class="wh-kpi-card">
				<span class="wh-kpi-val">{$flowerSorts.length}</span>
				<span class="wh-kpi-label">Видов сырья</span>
			</div>
		</div>

		<!-- Cards showing pkg_stock -->
		{#if $flowerSorts.length === 0}
			<p class="wh-empty">Нет сырья. Добавьте его в <a href="/flowers">Оранжерее</a>.</p>
		{:else}
			<div class="wh-grid">
				{#each warehouseSorts as sort (sort.id)}
					<div class="wh-card">
						<div class="wh-card-name">{sort.name}</div>
						{#if sort.variety}<div class="wh-card-variety">{sort.variety}</div>{/if}
						<div class="wh-stocks">
							<div class="wh-stock">
								<span class="wh-stock-val">{sort.pkg_stock}</span>
								<span class="wh-stock-unit">уп.</span>
							</div>
							<div class="wh-stock muted">
								<span class="wh-stock-val">{sort.raw_stock}</span>
								<span class="wh-stock-unit">шт.</span>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}

	</div>

	{#if packOpen}
		<PackModal
			onclose={() => (packOpen = false)}
			ondone={() => { flowerSorts.load(); }}
		/>
	{/if}

{:else}
	<!-- ══ GENERIC INVENTORY VIEW ════════════════════════════════ -->
	<div class="header">
		<h1>{$t('page_inventory_title')}</h1>
		<div class="header-actions">
			<button
				class="btn-secondary"
				onclick={() => { showCategoryManager = !showCategoryManager; showSortManager = false; showForm = false; }}
			>
				{$t('action_manage_categories')}
			</button>
			{#if $inventory.length > 0}
				<button class="btn-danger" onclick={handleDeleteAll} title="Удалить все товары">
					🗑 Очистить
				</button>
			{/if}
			<button class="btn-primary" onclick={() => { showForm = !showForm; showCategoryManager = false; showSortManager = false; }}>
				{showForm ? $t('action_cancel') : $t('action_add_item')}
			</button>
		</div>
	</div>

	<!-- ── Category Manager ────────────────────────────────────── -->
	{#if showCategoryManager}
		<div class="cat-panel">
			<div class="cat-panel-title">{$t('action_manage_categories')}</div>

			{#if $categories.length === 0}
				<p class="empty-hint">{$t('empty_no_categories')}</p>
			{:else}
				<div class="cat-list">
					{#each $categories as cat (cat.id)}
						{#if editingCatId === cat.id}
							<!-- Inline edit row -->
							<div class="cat-row editing">
								<input type="color" class="cat-color-swatch" bind:value={editingCatColor} />
								<input class="cat-name-input" type="text" bind:value={editingCatName} />
								<button class="cat-action-btn save" onclick={handleSaveCategory}>✓</button>
								<button class="cat-action-btn" onclick={() => (editingCatId = null)}>✕</button>
							</div>
						{:else}
							<div class="cat-row">
								<span class="cat-color-dot" style:background={cat.color ?? 'var(--color-outline)'}></span>
								<span class="cat-name">{cat.name}</span>
								<button class="cat-action-btn" onclick={() => startEditCat(cat)} aria-label="Редактировать">✎</button>
								<button class="cat-action-btn danger" onclick={() => handleDeleteCategory(cat.id)} aria-label="Удалить">✕</button>
							</div>
						{/if}
					{/each}
				</div>
			{/if}

			<!-- Add new category form -->
			<div class="cat-add-form">
				<input type="color" class="cat-color-swatch" bind:value={newCatColor} title={$t('label_category_color')} />
				<input
					class="cat-name-input"
					type="text"
					placeholder={$t('action_new_category')}
					bind:value={newCatName}
					onkeydown={(e) => e.key === 'Enter' && handleCreateCategory()}
				/>
				<button class="btn-secondary" onclick={handleCreateCategory} disabled={!newCatName.trim()}>
					{$t('action_save')}
				</button>
			</div>
		</div>
	{/if}

	<!-- ── Sort Manager (generic preset only — flowers uses greenhouse page) -->
	{#if showSortManager}
		<div class="cat-panel">
			<div class="cat-panel-title">{$t('flowers_manage_sorts')}</div>
			{#if $flowerSorts.length === 0}
				<p class="empty-hint">{$t('empty_no_items')}</p>
			{:else}
				<div class="cat-list">
					{#each $flowerSorts as sort (sort.id)}
						{#if editingSortId === sort.id && editingSort}
							<div class="cat-row editing">
								<input type="color" class="cat-color-swatch" bind:value={editingSort.color_hex} />
								<input class="cat-name-input" type="text" bind:value={editingSort.name} placeholder="Вид" />
								<input class="cat-name-input sm" type="text" bind:value={editingSort.variety} placeholder="Сорт" />
								<button class="cat-action-btn save" onclick={handleSaveSort}>✓</button>
								<button class="cat-action-btn" onclick={() => { editingSortId = null; editingSort = null; }}>✕</button>
							</div>
						{:else}
							<div class="cat-row">
								<span class="cat-color-dot" style:background={sort.color_hex ?? '#888'}></span>
								<span class="cat-name">{sort.name}{sort.variety ? ` — ${sort.variety}` : ''}</span>
								<span class="sort-stock-badge">{sort.raw_stock} ст.</span>
								{#if sort.purchase_price > 0}
									<span class="sort-price-badge">{sort.purchase_price} ₽/шт</span>
								{/if}
								<button class="cat-action-btn" onclick={() => startEditSort(sort)}>✎</button>
								<button class="cat-action-btn danger" onclick={() => handleDeleteSort(sort.id)}>✕</button>
							</div>
						{/if}
					{/each}
				</div>
			{/if}
			<div class="cat-add-form">
				<input type="color" class="cat-color-swatch" bind:value={newSortColor} />
				<input class="cat-name-input" type="text" placeholder="Вид (Тюльпан)" bind:value={newSortName} />
				<input class="cat-name-input sm" type="text" placeholder="Сорт" bind:value={newSortVariety} />
				<button class="btn-secondary" onclick={handleCreateSort} disabled={!newSortName.trim()}>
					+ {$t('flowers_add_sort_label')}
				</button>
			</div>
		</div>
	{/if}

	<!-- ── Item creation form ──────────────────────────────────── -->
	{#if showForm}
		<form class="item-form" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
			<div class="form-grid">
				<div class="form-field">
					<label for="name">{$t('label_name')} *</label>
					<input id="name" type="text" bind:value={formData.name} required />
				</div>
				<div class="form-field">
					<label for="category">{$t('label_category')}</label>
					{#if $categories.length > 0}
						<select id="category" onchange={onCategorySelect}>
							<option value="">— {$t('label_category')} —</option>
							{#each $categories as cat}
								<option value={cat.id} selected={formData.category_id === cat.id}>{cat.name}</option>
							{/each}
						</select>
					{:else}
						<input id="category" type="text" bind:value={formData.category} placeholder="uncategorized" />
					{/if}
				</div>
				<div class="form-field">
					<label for="price">{$t('label_price')} *</label>
					<input id="price" type="number" step="0.01" min="0" bind:value={formData.price} required />
				</div>
				<div class="form-field">
					<label for="cost">{$t('label_cost')}</label>
					<input id="cost" type="number" step="0.01" min="0" bind:value={formData.production_cost} />
				</div>
				<div class="form-field">
					<label for="stock">{$t('label_stock')}</label>
					<input id="stock" type="number" min="0" bind:value={formData.initial_stock} />
				</div>
			</div>

			<div class="form-field">
				<label>{$t('label_image')}</label>
				<div
					class="dropzone"
					class:active={isDragOver}
					role="region"
					aria-label={$t('hint_drop_image')}
					tabindex="0"
					onclick={handleImageClick}
					onkeydown={(e) => e.key === 'Enter' && handleImageClick()}
					ondragover={(e) => { e.preventDefault(); isDragOver = true; }}
					ondragleave={() => (isDragOver = false)}
					ondrop={handleImageDrop}
				>
					{#if previewUrl}
						<img src={previewUrl} alt="Preview" class="preview-thumbnail" />
					{:else}
						<span class="dropzone-hint">{$t('hint_drop_image')}</span>
					{/if}
				</div>
			</div>

			<button type="submit" class="btn-primary" disabled={!formData.name || formData.price <= 0}>
				{$t('action_save')}
			</button>
		</form>
	{/if}

	<!-- ═══════════════════════════════════════════════════════════ -->
	<!-- GENERIC INVENTORY VIEW (non-flowers presets)               -->
	<!-- ═══════════════════════════════════════════════════════════ -->
		<!-- ── Analytics Panel ──────────────────────────────────────── -->
		<div class="analytics-toggle-row">
			<button class="analytics-toggle btn-secondary" onclick={() => (analyticsOpen = !analyticsOpen)}>
				{analyticsOpen ? '▲' : '▼'} Аналитика склада
			</button>
		</div>
		<div class="analytics-panel" class:analytics-open={analyticsOpen}>
			<div class="analytics-grid">
				<div class="analytics-kpi">
					<span class="analytics-val">{analytics.totalItems}</span>
					<span class="analytics-lbl">Товаров</span>
				</div>
				<div class="analytics-kpi">
					<span class="analytics-val">{analytics.totalStock}</span>
					<span class="analytics-lbl">На складе</span>
				</div>
				<div class="analytics-kpi">
					<span class="analytics-val">{analytics.totalCost.toFixed(0)}</span>
					<span class="analytics-lbl">Себест. (∑)</span>
				</div>
				<div class="analytics-kpi">
					<span class="analytics-val">{analytics.totalValue.toFixed(0)}</span>
					<span class="analytics-lbl">Стоимость (∑)</span>
				</div>
				<div class="analytics-kpi">
					<span class="analytics-val">{analytics.avgMargin}%</span>
					<span class="analytics-lbl">Ср. маржа</span>
				</div>
				<div class="analytics-kpi">
					<span class="analytics-val">{analytics.totalSold}</span>
					<span class="analytics-lbl">Продано</span>
				</div>
			</div>
		</div>

		<!-- ── Filters & Sort ──────────────────────────────────────── -->
		<div class="filters">
			<input
				type="text"
				class="search-input"
				placeholder={$t('hint_search')}
				bind:value={searchQuery}
			/>
			<select class="category-filter" bind:value={filterCategory}>
				<option value="">{$t('status_all')}</option>
				{#each $categoryStrings as cat}
					<option value={cat}>{cat}</option>
				{/each}
			</select>
			<div class="sort-pills">
				{#each [
					{ key: 'name',          label: 'А-Я' },
					{ key: 'current_price', label: $t('label_price') },
					{ key: 'sold_count',    label: '#' },
					{ key: 'current_stock', label: 'Ост.' },
				] as s}
					<button
						class="sort-pill"
						class:active={sortKey === s.key}
						onclick={() => setSort(s.key)}
					>
						{s.label}{sortKey === s.key ? (sortDir === 'asc' ? '↑' : '↓') : ''}
					</button>
				{/each}
			</div>
		</div>

		<div class="items-grid">
			{#each sortedItems as item (item.id)}
				<LiquidGlassCard
					{item}
					appDataDir={appDataDirPath}
					onclick={() => (selectedItem = item)}
					onpack={(it) => (packingItem = it)}
					onsell={(it) => (sellingItem = it)}
				/>
			{:else}
				<div class="empty-state">{$t('empty_no_items')}</div>
			{/each}
		</div>
{/if}<!-- close outer: flowers warehouse vs generic inventory -->
</div><!-- close inventory-page -->

<!-- Item detail modal -->
<ItemDetailModal item={selectedItem} onclose={() => (selectedItem = null)} appDataDir={appDataDirPath} />

<!-- Pack assignment modal (flowers preset) -->
{#if packingItem}
	<PackAssignmentModal item={packingItem} onclose={() => (packingItem = null)} />
{/if}

<!-- Quick sell modal -->
{#if sellingItem}
	<QuickSellModal item={sellingItem} onclose={() => (sellingItem = null)} />
{/if}

<style>
	.inventory-page { max-width: 1000px; margin: 0 auto; }

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 24px;
	}

	.header h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0;
		letter-spacing: -0.02em;
	}

	.header-actions { display: flex; gap: 10px; align-items: center; }

	.btn-primary {
		background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 70%, var(--color-secondary)));
		color: #0a0a0a;
		border: none;
		padding: 8px 20px;
		border-radius: 10px;
		font-weight: 700;
		cursor: pointer;
		font-size: 0.875rem;
		transition: opacity 0.2s var(--ease-spring), transform 0.2s var(--ease-spring);
	}

	.btn-primary:hover { opacity: 0.9; transform: translateY(-1px); }
	.btn-primary:active { transform: translateY(0); }
	.btn-primary:disabled { opacity: 0.35; cursor: not-allowed; transform: none; }

	.btn-secondary {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		padding: 8px 16px;
		border-radius: 10px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-secondary:hover { background: var(--glass-bg-hover); }
	.btn-secondary:disabled { opacity: 0.4; cursor: not-allowed; }

	.btn-danger {
		background: rgba(248, 113, 113, 0.1);
		border: 1px solid rgba(248, 113, 113, 0.3);
		color: #f87171;
		padding: 8px 16px;
		border-radius: 10px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s;
		font-family: inherit;
	}

	.btn-danger:hover { background: rgba(248, 113, 113, 0.2); }

	/* ── Category manager ── */
	.cat-panel {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 14px;
		padding: 18px;
		margin-bottom: 20px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.cat-panel-title {
		font-size: 0.72rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-outline);
	}

	.cat-list { display: flex; flex-direction: column; gap: 6px; }

	.cat-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 9px;
	}

	.cat-color-dot {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.cat-color-swatch {
		width: 28px;
		height: 28px;
		border: none;
		border-radius: 7px;
		cursor: pointer;
		background: transparent;
		flex-shrink: 0;
		padding: 0;
	}

	.cat-color-swatch::-webkit-color-swatch-wrapper { padding: 0; }
	.cat-color-swatch::-webkit-color-swatch { border-radius: 6px; border: 1px solid var(--glass-border); }

	.cat-name {
		flex: 1;
		font-size: 0.875rem;
		color: var(--color-on-surface);
	}

	.cat-name-input {
		flex: 1;
		background: var(--glass-bg-hover);
		border: 1px solid var(--color-primary);
		border-radius: 7px;
		padding: 5px 9px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
	}

	.cat-action-btn {
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		padding: 4px 7px;
		border-radius: 6px;
		font-size: 0.82rem;
		transition: color 0.15s, background 0.15s;
	}

	.cat-action-btn:hover { background: var(--glass-bg-hover); color: var(--color-on-surface); }
	.cat-action-btn.danger:hover { color: #f87171; background: rgba(248, 113, 113, 0.1); }
	.cat-action-btn.save { color: #34d399; }
	.cat-action-btn.save:hover { background: rgba(52, 211, 153, 0.1); }

	.cat-add-form {
		display: flex;
		align-items: center;
		gap: 8px;
		padding-top: 4px;
		border-top: 1px solid var(--glass-border);
	}

	.empty-hint { font-size: 0.82rem; color: var(--color-outline); font-style: italic; }

	.cat-name-input.sm { max-width: 120px; }

	.sort-stock-badge {
		font-size: 0.72rem;
		padding: 2px 8px;
		border-radius: 10px;
		background: rgba(96, 165, 250, 0.12);
		color: #60a5fa;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.sort-price-badge {
		font-size: 0.72rem;
		padding: 2px 8px;
		border-radius: 10px;
		background: rgba(52, 211, 153, 0.10);
		color: var(--color-primary);
		white-space: nowrap;
		flex-shrink: 0;
	}

	/* ── Item form ── */
	.item-form {
		background: var(--glass-bg);
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 16px;
		padding: 20px;
		margin-bottom: 24px;
		box-shadow: var(--glass-shadow);
	}

	.form-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: 12px;
		margin-bottom: 16px;
	}

	.form-field {
		display: flex;
		flex-direction: column;
		gap: 5px;
		margin-bottom: 12px;
	}

	.form-field label {
		font-size: 0.7rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-weight: 500;
	}

	.form-field input,
	.form-field select {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 8px 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.2s;
	}

	.form-field input:focus,
	.form-field select:focus { border-color: var(--color-primary); }

	/* ── Dropzone ── */
	.dropzone {
		border: 1px dashed var(--color-outline);
		border-radius: 10px;
		padding: 20px;
		text-align: center;
		cursor: pointer;
		transition: border-color 0.2s, background 0.2s;
		min-height: 80px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.dropzone.active { border-color: var(--color-primary); background: rgba(52, 211, 153, 0.05); }

	.dropzone-hint { font-size: 0.8rem; color: var(--color-outline); }

	.preview-thumbnail { max-width: 100%; max-height: 120px; border-radius: 6px; object-fit: contain; }

	/* ── Analytics Panel (Task 7) ── */
	.analytics-toggle-row {
		margin-bottom: 8px;
	}

	.analytics-toggle {
		font-size: 0.78rem;
		padding: 5px 12px;
	}

	.analytics-panel {
		overflow: hidden;
		max-height: 0;
		transition: max-height 320ms ease, opacity 320ms ease;
		opacity: 0;
		margin-bottom: 0;
	}

	.analytics-panel.analytics-open {
		max-height: 120px;
		opacity: 1;
		margin-bottom: 16px;
	}

	.analytics-grid {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 10px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 14px;
		padding: 14px 16px;
	}

	@media (max-width: 768px) {
		.analytics-grid { grid-template-columns: repeat(3, 1fr); }
	}

	.analytics-kpi {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.analytics-val {
		font-size: 1.05rem;
		font-weight: 700;
		color: var(--color-on-surface);
		letter-spacing: -0.02em;
	}

	.analytics-lbl {
		font-size: 0.62rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-outline);
	}

	/* ── Filters (Task 6) ── */
	.filters { display: flex; gap: 10px; margin-bottom: 20px; flex-wrap: wrap; align-items: center; }

	.search-input {
		flex: 0 0 280px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 8px 14px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		outline: none;
		transition: border-color 0.2s;
		font-family: inherit;
	}

	@media (max-width: 640px) {
		.search-input { flex: 1 1 100%; }
	}

	.search-input:focus { border-color: var(--color-primary); }

	.category-filter {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 8px 14px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		min-width: 160px;
		font-family: inherit;
		outline: none;
	}

	/* Sort pills */
	.sort-pills {
		display: flex;
		gap: 5px;
		flex-wrap: wrap;
		margin-left: auto;
	}

	.sort-pill {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		border-radius: 20px;
		padding: 5px 11px;
		font-size: 0.78rem;
		cursor: pointer;
		font-family: inherit;
		transition: background 0.15s, border-color 0.15s, color 0.15s;
		white-space: nowrap;
	}

	.sort-pill:hover { background: var(--glass-bg-hover); }

	.sort-pill.active {
		background: var(--color-primary);
		border-color: var(--color-primary);
		color: #0a0a0a;
		font-weight: 700;
	}

	/* ── Grid ── */
	.items-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 14px;
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 56px;
		color: var(--color-outline);
		font-size: 0.9rem;
	}

	/* ── New Warehouse View styles (in .warehouse-view component) ── */
	.warehouse-view { display: flex; flex-direction: column; gap: 20px; }
	.wh-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
	.wh-title { font-size: 1.8rem; font-weight: 800; letter-spacing: -0.03em; margin: 0; color: var(--color-on-surface); }
	.wh-sub { font-size: 0.82rem; color: var(--color-outline); margin: 2px 0 0; }
	.btn-pack {
		display: flex; align-items: center; gap: 7px;
		background: var(--color-primary); color: var(--color-on-primary, #fff);
		border: none; border-radius: 12px; padding: 10px 18px;
		font-size: 0.9rem; font-weight: 600; cursor: pointer; transition: opacity 0.15s;
	}
	.btn-pack:hover { opacity: 0.88; }
	.wh-kpi { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
	.wh-kpi-card {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 14px; padding: 14px 16px; display: flex; flex-direction: column; gap: 3px;
	}
	.wh-kpi-val { font-size: 1.7rem; font-weight: 700; color: var(--color-primary); line-height: 1; }
	.wh-kpi-label { font-size: 0.72rem; color: var(--color-outline); }
	.wh-empty { font-size: 0.88rem; color: var(--color-outline); margin: 0; }
	.wh-empty a { color: var(--color-primary); }
	.wh-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 12px; }
	.wh-card {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 14px; padding: 14px 16px; display: flex; flex-direction: column; gap: 6px;
	}
	.wh-card-name { font-size: 0.92rem; font-weight: 600; color: var(--color-on-surface); }
	.wh-card-variety { font-size: 0.75rem; color: var(--color-outline); }
	.wh-stocks { display: flex; gap: 12px; align-items: baseline; }
	.wh-stock { display: flex; align-items: baseline; gap: 3px; }
	.wh-stock.muted .wh-stock-val { font-size: 1rem; color: var(--color-outline); }
	.wh-stock-val { font-size: 1.5rem; font-weight: 700; color: var(--color-primary); line-height: 1; }
	.wh-stock-unit { font-size: 0.72rem; color: var(--color-outline); }
</style>
