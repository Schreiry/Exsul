<script lang="ts">
	import { flowerSorts } from '$lib/stores/flowers';
	import { t } from '$lib/stores/i18n';
	import { commands } from '$lib/tauri/commands';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let name = $state('');
	let variety = $state('');
	let description = $state('');
	let purchasePrice = $state(0);
	let sellPriceStem = $state(0);
	let flowersPerPackOverride = $state<number | null>(null);
	let photoPath = $state<string | null>(null);
	let photoPreview = $state<string | null>(null);
	let saving = $state(false);
	let error = $state('');

	async function pickPhoto() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({
				filters: [{ name: 'Изображения', extensions: ['jpg', 'jpeg', 'png', 'webp', 'gif'] }],
				multiple: false,
			});
			if (typeof selected === 'string') {
				photoPath = selected;
				// Build a preview URL via Tauri asset protocol
				const { convertFileSrc } = await import('@tauri-apps/api/core');
				photoPreview = convertFileSrc(selected);
			}
		} catch {
			// Dialog cancelled or not in Tauri — ignore
		}
	}

	async function handleSubmit() {
		if (!name.trim()) { error = 'Введите название сырья'; return; }
		saving = true;
		error = '';
		try {
			const id = await flowerSorts.create({
				name: name.trim(),
				variety: variety.trim() || undefined,
				description: description.trim() || undefined,
				purchase_price: purchasePrice || undefined,
				sell_price_stem: sellPriceStem || undefined,
				flowers_per_pack_override: flowersPerPackOverride ?? undefined,
			});
			if (photoPath) {
				await flowerSorts.savePhoto(id, photoPath);
			}
			onclose();
		} catch (e) {
			error = String(e);
		} finally {
			saving = false;
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdropClick} role="dialog" aria-modal="true" aria-label="Добавить сырьё">
	<div class="modal-panel">
		<div class="modal-header">
			<h2 class="modal-title">Добавить сырьё</h2>
			<button class="btn-close" type="button" onclick={onclose} aria-label="Закрыть">✕</button>
		</div>

		<form class="modal-body" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>

			<!-- Photo picker -->
			<div class="photo-row">
				<button type="button" class="photo-btn" onclick={pickPhoto}>
					{#if photoPreview}
						<img src={photoPreview} alt="Preview" class="photo-preview" />
					{:else}
						<div class="photo-placeholder">
							<svg viewBox="0 0 24 24" width="28" height="28" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round">
								<rect x="3" y="3" width="18" height="18" rx="4"/>
								<circle cx="8.5" cy="8.5" r="1.5"/>
								<polyline points="21 15 16 10 5 21"/>
							</svg>
							<span>Фото</span>
						</div>
					{/if}
				</button>

				<div class="name-fields">
					<div class="field">
						<label class="field-label" for="sort-name">Название *</label>
						<input id="sort-name" class="field-input" type="text" bind:value={name}
							placeholder="Например: Роза, Тюльпан…" required autocomplete="off" />
					</div>
					<div class="field">
						<label class="field-label" for="sort-variety">Сорт</label>
						<input id="sort-variety" class="field-input" type="text" bind:value={variety}
							placeholder="Необязательно" autocomplete="off" />
					</div>
				</div>
			</div>

			<!-- Description -->
			<div class="field">
				<label class="field-label" for="sort-desc">Описание</label>
				<textarea id="sort-desc" class="field-input field-textarea" bind:value={description}
					placeholder="Дополнительные заметки…" rows="2"></textarea>
			</div>

			<!-- Pricing -->
			<div class="fields-row">
				<div class="field">
					<label class="field-label" for="buy-price">Закупочная цена</label>
					<input id="buy-price" class="field-input" type="number" min="0" step="0.01"
						bind:value={purchasePrice} placeholder="0" />
				</div>
				<div class="field">
					<label class="field-label" for="sell-price">Цена стебля</label>
					<input id="sell-price" class="field-input" type="number" min="0" step="0.01"
						bind:value={sellPriceStem} placeholder="0" />
				</div>
				<div class="field">
					<label class="field-label" for="fpp">Цветков в упаковке</label>
					<input id="fpp" class="field-input" type="number" min="1" step="1"
						bind:value={flowersPerPackOverride} placeholder="по умолч." />
				</div>
			</div>

			{#if error}
				<p class="error-msg">{error}</p>
			{/if}

			<div class="modal-footer">
				<button type="button" class="btn-secondary" onclick={onclose}>Отмена</button>
				<button type="submit" class="btn-primary" disabled={saving || !name.trim()}>
					{saving ? '…' : 'Добавить'}
				</button>
			</div>
		</form>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0,0,0,0.55);
		backdrop-filter: blur(6px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		padding: 16px;
	}

	.modal-panel {
		background: var(--color-surface, #0f0f0f);
		border: 1px solid var(--glass-border);
		border-top-color: rgba(255,255,255,0.14);
		border-radius: 20px;
		width: 100%;
		max-width: 520px;
		box-shadow: 0 24px 60px rgba(0,0,0,0.5);
		display: flex;
		flex-direction: column;
		max-height: 90vh;
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 20px 24px 0;
	}

	.modal-title {
		font-size: 1.1rem;
		font-weight: 700;
		margin: 0;
		color: var(--color-on-surface);
	}

	.btn-close {
		background: none;
		border: none;
		color: var(--color-outline);
		font-size: 1rem;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 8px;
		transition: color 0.1s;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	.modal-body {
		padding: 20px 24px 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		overflow-y: auto;
	}

	/* Photo + name row */
	.photo-row {
		display: flex;
		gap: 16px;
		align-items: flex-start;
	}

	.photo-btn {
		width: 90px;
		height: 90px;
		border-radius: 12px;
		border: 1.5px dashed var(--color-outline);
		background: var(--glass-bg);
		cursor: pointer;
		overflow: hidden;
		flex-shrink: 0;
		transition: border-color 0.15s;
		padding: 0;
	}
	.photo-btn:hover { border-color: var(--color-primary); }

	.photo-preview {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.photo-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 4px;
		color: var(--color-outline);
		font-size: 0.7rem;
	}

	.name-fields { flex: 1; display: flex; flex-direction: column; gap: 10px; }

	/* Fields */
	.field { display: flex; flex-direction: column; gap: 4px; }
	.fields-row { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }

	.field-label { font-size: 0.75rem; color: var(--color-outline); font-weight: 500; }

	.field-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 8px 12px;
		font-size: 0.9rem;
		color: var(--color-on-surface);
		outline: none;
		transition: border-color 0.15s;
		width: 100%;
		box-sizing: border-box;
	}
	.field-input:focus { border-color: var(--color-primary); }
	.field-textarea { resize: vertical; min-height: 56px; font-family: inherit; }

	.error-msg { color: var(--color-alert-red); font-size: 0.85rem; margin: 0; }

	.modal-footer {
		display: flex;
		gap: 10px;
		justify-content: flex-end;
		margin-top: 4px;
	}

	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		border: none;
		border-radius: 10px;
		padding: 9px 20px;
		font-size: 0.9rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
	}
	.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

	.btn-secondary {
		background: var(--glass-bg);
		color: var(--color-on-surface);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 9px 18px;
		font-size: 0.9rem;
		cursor: pointer;
		transition: background 0.1s;
	}
	.btn-secondary:hover { background: var(--glass-bg-hover); }

	/* Light mode */
	:global([data-theme="light"]) .modal-panel {
		background: var(--color-surface, #f8f8f8);
	}
	:global([data-theme="light"]) .field-input {
		background: rgba(0,0,0,0.03);
		border-color: rgba(0,0,0,0.12);
		color: var(--color-on-surface);
	}
	:global([data-theme="light"]) .field-input:focus {
		border-color: var(--color-primary);
	}
</style>
