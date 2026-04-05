<script lang="ts">
	import { inventory, categories as categoryStrings } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { t } from '$lib/stores/i18n';
	import { commands } from '$lib/tauri/commands';
	import LiquidGlassCard from '$lib/components/LiquidGlassCard.svelte';
	import ItemDetailModal from '$lib/components/inventory/ItemDetailModal.svelte';
	import type { CreateItemPayload, Item, Category } from '$lib/tauri/types';

	let showForm = $state(false);
	let showCategoryManager = $state(false);
	let selectedItem = $state<Item | null>(null);

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

	let filteredItems = $derived(
		$inventory.filter((item) => {
			const matchesSearch =
				!searchQuery ||
				item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
				item.category.toLowerCase().includes(searchQuery.toLowerCase());
			const matchesCategory = !filterCategory || item.category === filterCategory;
			return matchesSearch && matchesCategory;
		})
	);

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
	<div class="header">
		<h1>{$t('page_inventory_title')}</h1>
		<div class="header-actions">
			<button
				class="btn-secondary"
				onclick={() => { showCategoryManager = !showCategoryManager; showForm = false; }}
			>
				{$t('action_manage_categories')}
			</button>
			<button class="btn-primary" onclick={() => { showForm = !showForm; showCategoryManager = false; }}>
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
	</div>

	<div class="items-grid">
		{#each filteredItems as item (item.id)}
			<LiquidGlassCard {item} appDataDir={appDataDirPath} onclick={() => (selectedItem = item)} />
		{:else}
			<div class="empty-state">{$t('empty_no_items')}</div>
		{/each}
	</div>
</div>

<!-- Item detail modal -->
<ItemDetailModal item={selectedItem} onclose={() => (selectedItem = null)} appDataDir={appDataDirPath} />

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

	/* ── Filters ── */
	.filters { display: flex; gap: 10px; margin-bottom: 20px; }

	.search-input {
		flex: 1;
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
</style>
