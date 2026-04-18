<script lang="ts">
	import { inventory } from '$lib/stores/inventory';
	import { t } from '$lib/stores/i18n';
	import { commands } from '$lib/tauri/commands';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { globalCurrency, itemCurrencies, formatAmount, setItemCurrency, CURRENCIES } from '$lib/stores/currency';
	import GlassDropdown from '$lib/components/common/GlassDropdown.svelte';
	import type { Item } from '$lib/tauri/types';

	interface Props {
		item: Item | null;
		onclose: () => void;
		appDataDir?: string;
	}

	let { item, onclose, appDataDir = '' }: Props = $props();

	let editMode = $state(false);
	let saving = $state(false);

	// Preset colors for card_color picker
	const PRESET_COLORS = [
		'#f472b6', '#fb923c', '#facc15', '#4ade80', '#34d399',
		'#22d3ee', '#60a5fa', '#a78bfa', '#e879f9', '#94a3b8',
		'#f87171', '#ffffff',
	];

	// Draft for edit mode — a flat copy of the item fields
	let draft = $state({
		name: '',
		category: '',
		production_cost: 0,
		new_price: 0,
		stock_delta: 0,
		sold_delta: 0,
		card_color: '',
	});

	// Image upload
	let isDragOver = $state(false);
	let previewUrl = $state<string | null>(null);
	let pendingImageBase64 = $state<string | null>(null);
	let selectedCurrency = $state('');

	// Reset draft whenever a new item is opened
	$effect(() => {
		if (item) {
			draft = {
				name: item.name,
				category: item.category,
				production_cost: item.production_cost,
				new_price: item.current_price,
				stock_delta: 0,
				sold_delta: 0,
				card_color: item.card_color ?? '',
			};
			editMode = false;
			previewUrl = null;
			pendingImageBase64 = null;
			selectedCurrency = itemCurrency;
		}
	});

	// Sync currency dropdown selection back to store
	$effect(() => {
		if (item && selectedCurrency && selectedCurrency !== itemCurrency) {
			setItemCurrency(item.id, selectedCurrency);
		}
	});

	function getImageSrc(path: string | null | undefined, baseDir: string): string | null {
		if (!path || !baseDir) return null;
		const base = baseDir.endsWith('\\') || baseDir.endsWith('/') ? baseDir : baseDir + '/';
		return convertFileSrc(base + path.replace(/\\/g, '/'));
	}

	let imageSrc = $derived(
		previewUrl ?? (item ? getImageSrc(item.image_path, appDataDir) : null)
	);

	let margin = $derived(
		item && item.production_cost > 0
			? ((item.current_price - item.production_cost) / item.current_price) * 100
			: null
	);

	// Resolved currency: per-item override → global fallback
	let itemCurrency = $derived(
		(item ? $itemCurrencies[item.id] : undefined) ?? $globalCurrency
	);

	function fmt(value: number): string {
		return formatAmount(value, itemCurrency);
	}

	function handleImageDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		const file = e.dataTransfer?.files?.[0];
		if (!file || !file.type.startsWith('image/')) return;
		if (file.size > 5 * 1024 * 1024) return;
		readFile(file);
	}

	function handleImageClick() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = 'image/*';
		input.onchange = (e) => {
			const file = (e.target as HTMLInputElement).files?.[0];
			if (file && file.size <= 5 * 1024 * 1024) readFile(file);
		};
		input.click();
	}

	function readFile(file: File) {
		const reader = new FileReader();
		reader.onload = (ev) => {
			const dataUrl = ev.target?.result as string;
			previewUrl = dataUrl;
			const comma = dataUrl.indexOf(',');
			pendingImageBase64 = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
		};
		reader.readAsDataURL(file);
	}

	async function handleSave() {
		if (!item) return;
		saving = true;
		try {
			// Update business metadata via store
			await inventory.updateItem(item.id, {
				name: draft.name || undefined,
				category: draft.category || undefined,
				production_cost: draft.production_cost,
			});

			// card_color is UI metadata — update directly via command
			if (draft.card_color !== (item.card_color ?? '')) {
				await commands.updateItem({ item_id: item.id, card_color: draft.card_color });
				await inventory.load();
			}

			// Price change
			if (draft.new_price !== item.current_price && draft.new_price > 0) {
				await inventory.changePrice(item.id, draft.new_price);
			}

			// Stock adjustment
			if (draft.stock_delta !== 0) {
				await inventory.adjustStock(item.id, draft.stock_delta);
			}

			// Sale recording
			if (draft.sold_delta > 0) {
				await commands.recordSale({ item_id: item.id, quantity: draft.sold_delta });
			}

			// Image upload
			if (pendingImageBase64) {
				await commands.saveItemImage(item.id, pendingImageBase64);
				await inventory.load();
			}

			editMode = false;
		} finally {
			saving = false;
		}
	}

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

{#if item}
	<!-- Backdrop -->
	<div
		class="backdrop"
		role="presentation"
		onclick={onclose}
		onkeydown={handleBackdropKeydown}
	></div>

	<!-- Modal — picks up a subtle tint from the item's card color. The
	     CSS variable is exposed so .has-color rules can use color-mix. -->
	<div
		class="modal"
		class:has-color={!!item.card_color}
		style={item.card_color ? `--card-color: ${item.card_color};` : ''}
		role="dialog"
		aria-modal="true"
		aria-label={item.name}
	>
		<!-- Header -->
		<div class="modal-header">
			<div class="header-left">
				{#if editMode}
					<input class="title-input" bind:value={draft.name} />
				{:else}
					<h2 class="modal-title">{item.name}</h2>
				{/if}
				{#if item.category && item.category !== 'uncategorized'}
					<span class="category-badge">{item.category}</span>
				{/if}
			</div>
			<div class="header-actions">
				{#if !editMode}
					<button class="btn-edit" onclick={() => (editMode = true)}>
						✎ {$t('label_edit_item')}
					</button>
				{/if}
				<button class="close-btn" onclick={onclose} aria-label="Закрыть">✕</button>
			</div>
		</div>

		<!-- Image -->
		{#if editMode}
			<button
				type="button"
				class="dropzone"
				class:drag-active={isDragOver}
				aria-label={$t('hint_drop_image')}
				onclick={handleImageClick}
				ondragover={(e) => { e.preventDefault(); isDragOver = true; }}
				ondragleave={() => (isDragOver = false)}
				ondrop={handleImageDrop}
			>
				{#if imageSrc}
					<img src={imageSrc} alt={item.name} class="preview-img" />
				{:else}
					<span class="drop-hint">{$t('hint_drop_image')}</span>
				{/if}
			</button>
		{:else if imageSrc}
			<div class="image-container">
				<img src={imageSrc} alt={item.name} class="item-image" />
			</div>
		{/if}

		<!-- Metrics (view mode) -->
		{#if !editMode}
			<div class="metrics-grid">
				<div class="metric">
					<span class="mlabel">{$t('table_header_price')}</span>
					<span class="mvalue color-price">{fmt(item.current_price)}</span>
				</div>
				<div class="metric">
					<span class="mlabel">{$t('table_header_stock')}</span>
					<span class="mvalue color-stock">{item.current_stock}</span>
				</div>
				<div class="metric">
					<span class="mlabel">{$t('table_header_sold')}</span>
					<span class="mvalue color-sold">{item.sold_count}</span>
				</div>
				<div class="metric">
					<span class="mlabel">{$t('table_header_revenue')}</span>
					<span class="mvalue color-revenue">{fmt(item.revenue)}</span>
				</div>
				{#if margin !== null}
					<div class="metric">
						<span class="mlabel">{$t('label_margin')}</span>
						<span class="mvalue color-margin">{margin.toFixed(1)}%</span>
					</div>
				{/if}
				{#if item.production_cost > 0}
					<div class="metric">
						<span class="mlabel">{$t('label_cost')}</span>
						<span class="mvalue color-cost">{fmt(item.production_cost)}</span>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Edit form -->
		{#if editMode}
			<div class="edit-form">
				<div class="field">
					<label class="field-label" for="edit-category">{$t('label_category')}</label>
					<input id="edit-category" class="field-input" type="text" bind:value={draft.category} />
				</div>
				<div class="field">
					<label class="field-label" for="edit-price">{$t('label_price')}</label>
					<input id="edit-price" class="field-input" type="number" step="0.01" min="0" bind:value={draft.new_price} />
				</div>
				<div class="field">
					<label class="field-label" for="edit-cost">{$t('label_cost')}</label>
					<input id="edit-cost" class="field-input" type="number" step="0.01" min="0" bind:value={draft.production_cost} />
				</div>
				<div class="field">
					<label class="field-label" for="edit-stock">{$t('label_add_stock')} (+/−)</label>
					<input id="edit-stock" class="field-input" type="number" bind:value={draft.stock_delta} />
					<span class="field-hint">Текущий остаток: {item.current_stock} шт.</span>
				</div>
				<div class="field">
					<label class="field-label" for="edit-sold">Продать (+)</label>
					<input id="edit-sold" class="field-input" type="number" min="0" bind:value={draft.sold_delta} />
				</div>
				<div class="field color-field">
					<span class="field-label">Цвет карточки</span>
					<div class="color-swatches">
						{#each PRESET_COLORS as color}
							<button
								class="color-swatch"
								class:active={draft.card_color === color}
								style:background={color}
								onclick={() => (draft.card_color = draft.card_color === color ? '' : color)}
								aria-label={color}
							></button>
						{/each}
						<input
							type="color"
							class="color-custom"
							bind:value={draft.card_color}
							title="Свой цвет"
						/>
					</div>
				</div>
				<div class="field">
					<label class="field-label" for="edit-currency">{$t('label_item_currency')}</label>
					<GlassDropdown
						items={CURRENCIES.map(c => ({ value: c.code, label: `${c.symbol} ${c.code} — ${c.name}` }))}
						bind:value={selectedCurrency}
						placeholder="— Currency —"
					/>
				</div>
			</div>

			<div class="edit-actions">
				<button class="btn-save" onclick={handleSave} disabled={saving}>
					{saving ? '…' : $t('action_save')}
				</button>
				<button class="btn-cancel" onclick={() => (editMode = false)}>
					{$t('action_cancel')}
				</button>
			</div>
		{/if}

		<!-- Order info placeholder -->
		{#if !editMode}
			<div class="order-section">
				<div class="section-label">{$t('label_order_info')}</div>
				<p class="order-hint">{$t('action_link_order')}</p>
			</div>
		{/if}
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(6px);
		z-index: 1100;
	}

	.modal {
		position: fixed;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 1200;
		width: min(560px, calc(100vw - 32px));
		max-height: calc(100vh - 80px);
		overflow-y: auto;
		border-radius: 20px;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 18px;

		background: rgba(14, 14, 18, 0.80);
		backdrop-filter: blur(28px) saturate(180%);
		-webkit-backdrop-filter: blur(28px) saturate(180%);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		box-shadow: var(--glass-shadow-hover);

		animation: modal-in 0.22s var(--ease-spring) both;
	}

	/* Subtle card-color wash for the edit modal. Kept light (≤18%) so
	   form labels/inputs stay legible; a delicate top-to-bottom fade
	   echoes the card's accent without overpowering the dialog. */
	.modal.has-color {
		background:
			linear-gradient(160deg,
				color-mix(in srgb, var(--card-color) 18%, rgba(14,14,18,0.80)) 0%,
				rgba(14,14,18,0.80) 65%);
		border-color: color-mix(in srgb, var(--card-color) 35%, var(--glass-border));
		box-shadow:
			var(--glass-shadow-hover),
			0 0 60px color-mix(in srgb, var(--card-color) 18%, transparent);
	}

	:global([data-theme="light"]) .modal.has-color {
		background:
			linear-gradient(160deg,
				color-mix(in srgb, var(--card-color) 16%, var(--color-surface, #fafafa)) 0%,
				var(--color-surface, #fafafa) 65%);
	}

	@keyframes modal-in {
		from { opacity: 0; transform: translate(-50%, -48%) scale(0.97); }
		to   { opacity: 1; transform: translate(-50%, -50%) scale(1); }
	}

	/* Header */
	.modal-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
		min-width: 0;
	}

	.modal-title {
		font-size: 1.15rem;
		font-weight: 700;
		color: var(--color-on-surface);
		margin: 0;
		letter-spacing: -0.02em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.title-input {
		font-size: 1.1rem;
		font-weight: 700;
		background: var(--glass-bg);
		border: 1px solid var(--color-primary);
		border-radius: 8px;
		padding: 6px 10px;
		color: var(--color-on-surface);
		outline: none;
		width: 100%;
	}

	.category-badge {
		font-size: 0.7rem;
		background: var(--glass-bg-hover);
		border: 1px solid var(--glass-border);
		padding: 3px 8px;
		border-radius: 6px;
		color: var(--color-outline);
		letter-spacing: 0.02em;
		align-self: flex-start;
	}

	.header-actions {
		display: flex;
		gap: 8px;
		align-items: center;
		flex-shrink: 0;
	}

	.btn-edit {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		color: var(--color-on-surface);
		font-size: 0.78rem;
		padding: 6px 12px;
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
		white-space: nowrap;
	}

	.btn-edit:hover { background: var(--glass-bg-hover); border-color: var(--color-outline); }

	.close-btn {
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		font-size: 1rem;
		padding: 4px 8px;
		border-radius: 8px;
		transition: color 0.15s, background 0.15s;
	}

	.close-btn:hover { background: var(--glass-bg-hover); color: var(--color-on-surface); }

	/* Image */
	.image-container {
		border-radius: 12px;
		overflow: hidden;
		max-height: 200px;
	}

	.item-image {
		width: 100%;
		height: 200px;
		object-fit: cover;
	}

	/* Dropzone */
	.dropzone {
		border: 1px dashed var(--color-outline);
		border-radius: 12px;
		padding: 24px;
		text-align: center;
		cursor: pointer;
		min-height: 100px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-color 0.15s, background 0.15s;
	}

	.dropzone.drag-active {
		border-color: var(--color-primary);
		background: rgba(52, 211, 153, 0.06);
	}

	.preview-img { max-height: 130px; border-radius: 8px; object-fit: contain; }

	.drop-hint { font-size: 0.8rem; color: var(--color-outline); }

	/* Metrics grid */
	.metrics-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 14px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 16px;
	}

	.metric { display: flex; flex-direction: column; gap: 3px; }

	.mlabel {
		font-size: 0.62rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 500;
	}

	.mvalue { font-size: 1rem; font-weight: 700; }

	.color-price   { color: var(--color-primary); }
	.color-stock   { color: #60a5fa; }
	.color-sold    { color: #34d399; }
	.color-revenue { color: #fbbf24; }
	.color-margin  { color: #a78bfa; }
	.color-cost    { color: var(--color-outline); }

	/* Edit form */
	.edit-form {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}

	.field { display: flex; flex-direction: column; gap: 4px; }

	.field-label {
		font-size: 0.68rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.field-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 8px 10px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.field-input:focus { border-color: var(--color-primary); }

	/* Edit actions */
	.edit-actions { display: flex; gap: 10px; }

	.btn-save {
		background: var(--color-primary);
		color: var(--color-on-primary);
		border: none;
		padding: 9px 22px;
		border-radius: 9px;
		font-weight: 700;
		cursor: pointer;
		font-size: 0.875rem;
		transition: opacity 0.15s;
	}

	.btn-save:hover:not(:disabled) { opacity: 0.88; }
	.btn-save:disabled { opacity: 0.5; cursor: not-allowed; }

	.btn-cancel {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		padding: 9px 22px;
		border-radius: 9px;
		cursor: pointer;
		font-size: 0.875rem;
		transition: background 0.15s;
	}

	.btn-cancel:hover { background: var(--glass-bg-hover); }

	/* Card color picker */
	.color-field { grid-column: 1 / -1; }

	.color-swatches {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
		align-items: center;
	}

	.color-swatch {
		width: 24px;
		height: 24px;
		border-radius: 6px;
		border: 2px solid transparent;
		cursor: pointer;
		transition: transform 0.15s, border-color 0.15s;
		padding: 0;
	}

	.color-swatch:hover { transform: scale(1.15); }
	.color-swatch.active { border-color: var(--color-on-surface); transform: scale(1.1); }

	.color-custom {
		width: 28px;
		height: 24px;
		border-radius: 6px;
		border: 1px solid var(--glass-border);
		cursor: pointer;
		background: transparent;
		padding: 0;
	}

	.color-custom::-webkit-color-swatch-wrapper { padding: 0; }
	.color-custom::-webkit-color-swatch { border-radius: 5px; border: none; }

	.field-hint {
		font-size: 0.65rem;
		color: var(--color-outline);
		font-style: italic;
	}

	/* Order section */
	.order-section {
		border-top: 1px solid var(--glass-border);
		padding-top: 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.section-label {
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-outline);
	}

	.order-hint {
		font-size: 0.8rem;
		color: var(--color-outline);
		font-style: italic;
	}
</style>
