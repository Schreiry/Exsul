<script lang="ts">
	import { inventory, categories as categoryStrings } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { t } from '$lib/stores/i18n';
	import { commands } from '$lib/tauri/commands';
	import LiquidGlassCard from '$lib/components/LiquidGlassCard.svelte';
	import type { CreateItemPayload } from '$lib/tauri/types';

	let showForm = $state(false);
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

	function handleImageDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		const file = e.dataTransfer?.files?.[0];
		if (!file || !file.type.startsWith('image/')) return;
		if (file.size > 5 * 1024 * 1024) { alert('Image must be under 5 MB'); return; }
		const reader = new FileReader();
		reader.onload = (ev) => {
			const dataUrl = ev.target?.result as string;
			previewUrl = dataUrl;
			const comma = dataUrl.indexOf(',');
			pendingImageBase64 = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
		};
		reader.readAsDataURL(file);
	}

	function handleImageClick() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = 'image/*';
		input.onchange = (e) => {
			const file = (e.target as HTMLInputElement).files?.[0];
			if (!file) return;
			if (file.size > 5 * 1024 * 1024) { alert('Image must be under 5 MB'); return; }
			const reader = new FileReader();
			reader.onload = (ev) => {
				const dataUrl = ev.target?.result as string;
				previewUrl = dataUrl;
				const comma = dataUrl.indexOf(',');
				pendingImageBase64 = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
			};
			reader.readAsDataURL(file);
		};
		input.click();
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
		<button class="btn-primary" onclick={() => (showForm = !showForm)}>
			{showForm ? $t('action_cancel') : $t('action_add_item')}
		</button>
	</div>

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
			<LiquidGlassCard {item} appDataDir={appDataDirPath} />
		{:else}
			<div class="empty-state">{$t('empty_no_items')}</div>
		{/each}
	</div>
</div>

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
		color: rgba(255, 255, 255, 0.92);
		margin: 0;
		letter-spacing: -0.02em;
	}

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
		letter-spacing: 0.01em;
	}

	.btn-primary:hover { opacity: 0.9; transform: translateY(-1px); }
	.btn-primary:active { transform: translateY(0); }
	.btn-primary:disabled { opacity: 0.35; cursor: not-allowed; transform: none; }

	/* ── Form ── */
	.item-form {
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(16px) saturate(160%);
		-webkit-backdrop-filter: blur(16px) saturate(160%);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-top-color: rgba(255, 255, 255, 0.14);
		border-radius: 16px;
		padding: 20px;
		margin-bottom: 24px;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.09),
			0 4px 20px rgba(0, 0, 0, 0.35);
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
		color: rgba(255, 255, 255, 0.4);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-weight: 500;
	}

	.form-field input,
	.form-field select {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.10);
		border-radius: 8px;
		padding: 8px 12px;
		color: rgba(255, 255, 255, 0.88);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition:
			border-color 0.3s cubic-bezier(0.2, 0.8, 0.2, 1),
			background 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
	}

	.form-field input:focus,
	.form-field select:focus {
		border-color: rgba(52, 211, 153, 0.5);
		background: rgba(255, 255, 255, 0.07);
	}

	/* ── Dropzone ── */
	.dropzone {
		border: 1px dashed rgba(255, 255, 255, 0.18);
		border-radius: 10px;
		padding: 20px;
		text-align: center;
		cursor: pointer;
		transition:
			border-color 0.3s var(--ease-spring),
			background 0.3s var(--ease-spring);
		min-height: 80px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.dropzone.active {
		border-color: rgba(52, 211, 153, 0.6);
		background: rgba(52, 211, 153, 0.06);
	}

	.dropzone-hint {
		font-size: 0.8rem;
		color: rgba(255, 255, 255, 0.35);
	}

	.preview-thumbnail {
		max-width: 100%;
		max-height: 120px;
		border-radius: 6px;
		object-fit: contain;
	}

	/* ── Filters ── */
	.filters {
		display: flex;
		gap: 10px;
		margin-bottom: 20px;
	}

	.search-input {
		flex: 1;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.09);
		border-radius: 10px;
		padding: 8px 14px;
		color: rgba(255, 255, 255, 0.88);
		font-size: 0.875rem;
		outline: none;
		transition: border-color 0.3s var(--ease-spring), background 0.3s;
		font-family: inherit;
	}

	.search-input:focus {
		border-color: rgba(52, 211, 153, 0.4);
		background: rgba(255, 255, 255, 0.06);
	}

	.category-filter {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.09);
		border-radius: 10px;
		padding: 8px 14px;
		color: rgba(255, 255, 255, 0.88);
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
		color: rgba(255, 255, 255, 0.25);
		font-size: 0.9rem;
	}
</style>
