<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { flowerSorts } from '$lib/stores/flowers';
	import { showDetailedPricing } from '$lib/stores/appSettings';
	import { commands } from '$lib/tauri/commands';
	import { t } from '$lib/stores/i18n';
	import { formatCountdown } from '$lib/utils/countdown';
	import type { FlowerSort, HarvestLogEntry, OrderWaitingForSort } from '$lib/tauri/types';

	interface Props {
		sort: FlowerSort;
		onclose: () => void;
	}

	let { sort = $bindable(), onclose }: Props = $props();

	// Edit mode state
	let editing = $state(false);
	let editName = $state(sort.name);
	let editVariety = $state(sort.variety ?? '');
	let editDescription = $state(sort.description ?? '');
	let editPurchasePrice = $state(sort.purchase_price);
	let editSellPrice = $state(sort.sell_price_stem);
	let editFpp = $state<number | null>(sort.flowers_per_pack_override ?? null);
	let editColorHex = $state<string | null>(sort.color_hex ?? null);
	let editSaving = $state(false);

	const PRESET_COLORS = ['#f472b6','#fb923c','#facc15','#4ade80','#34d399','#22d3ee','#60a5fa','#a78bfa','#e879f9','#94a3b8','#f87171','#ffffff'];

	// Harvest log
	let harvestLog = $state<HarvestLogEntry[]>([]);
	let logLoading = $state(true);

	// Orders that still need packs of this sort (pending/in_progress with
	// order_items referencing sort.id). Driven by a single read query on mount —
	// reservations are operator-driven and don't change while the modal is open.
	let waitingOrders = $state<OrderWaitingForSort[]>([]);
	let waitingLoading = $state(true);

	// Add harvest state
	let harvestDelta = $state(0);
	let harvestReason = $state<'manual' | 'correction'>('manual');
	let harvestNote = $state('');
	let harvestSaving = $state(false);
	let harvestSuccess = $state(false);

	let appDataDir = $state('');

	function resolvePhotoSrc(photoPath: string | null | undefined, baseDir: string): string | null {
		if (!photoPath) return null;
		if (photoPath.includes(':') || photoPath.startsWith('/')) {
			return convertFileSrc(photoPath);
		}
		if (!baseDir) return null;
		const base = baseDir.endsWith('\\') || baseDir.endsWith('/') ? baseDir : baseDir + '/';
		return convertFileSrc(base + photoPath.replace(/\\/g, '/'));
	}

	const photoSrc = $derived(resolvePhotoSrc(sort.photo_path, appDataDir));

	$effect(() => {
		import('@tauri-apps/api/path').then(({ appDataDir: getDir }) =>
			getDir().then((dir) => { appDataDir = dir; })
		).catch(() => {});
	});

	$effect(() => {
		flowerSorts.getHarvestLog(sort.id, 90).then((log) => {
			harvestLog = log;
			logLoading = false;
		});
	});

	// Reload waiting orders whenever the underlying sort changes (e.g. after
	// the user deletes a reservation elsewhere and the parent rebinds).
	$effect(() => {
		waitingLoading = true;
		commands
			.getOrdersWaitingForSort(sort.id)
			.then((rows) => {
				waitingOrders = rows ?? [];
			})
			.catch((e) => {
				console.warn('Failed to load waiting orders:', e);
				waitingOrders = [];
			})
			.finally(() => {
				waitingLoading = false;
			});
	});

	const totalOrderedByWaiting = $derived(
		waitingOrders.reduce((s, o) => s + o.ordered_packs, 0)
	);
	const totalReservedByWaiting = $derived(
		waitingOrders.reduce((s, o) => s + o.reserved_packs, 0)
	);
	const totalShortage = $derived(
		waitingOrders.reduce((s, o) => s + o.shortage, 0)
	);

	function formatDeadline(deadline: string | null | undefined): string {
		if (!deadline) return '—';
		try {
			return new Date(deadline).toLocaleString('ru', {
				day: '2-digit',
				month: 'short',
				hour: '2-digit',
				minute: '2-digit',
			});
		} catch {
			return deadline;
		}
	}

	async function handleSaveEdit() {
		editSaving = true;
		try {
			await flowerSorts.updateSort({
				id: sort.id,
				name: editName.trim() || undefined,
				variety: editVariety.trim() || undefined,
				description: editDescription.trim() || undefined,
				purchase_price: editPurchasePrice,
				sell_price_stem: editSellPrice,
				flowers_per_pack_override: editFpp ?? undefined,
				color_hex: editColorHex || undefined,
			});
			editing = false;
		} finally {
			editSaving = false;
		}
	}

	async function handleAddHarvest() {
		if (harvestDelta <= 0) return;
		harvestSaving = true;
		try {
			await flowerSorts.logHarvest(sort.id, harvestDelta, harvestReason, harvestNote || undefined);
			harvestLog = await flowerSorts.getHarvestLog(sort.id, 90);
			harvestDelta = 0;
			harvestNote = '';
			harvestSuccess = true;
			setTimeout(() => (harvestSuccess = false), 2000);
		} finally {
			harvestSaving = false;
		}
	}

	async function handleDelete() {
		if (!confirm(`Удалить "${sort.name}"? Это действие нельзя отменить.`)) return;
		await flowerSorts.remove(sort.id);
		onclose();
	}

	async function pickNewPhoto() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({
				filters: [{ name: 'Изображения', extensions: ['jpg','jpeg','png','webp'] }],
				multiple: false,
			});
			if (typeof selected === 'string') {
				await flowerSorts.savePhoto(sort.id, selected);
			}
		} catch { /* cancelled */ }
	}

	// Mini harvest chart — group by date, last 30 days
	const chartData = $derived(() => {
		const days: Map<string, number> = new Map();
		const now = Date.now();
		for (let i = 29; i >= 0; i--) {
			const d = new Date(now - i * 86400000);
			days.set(d.toISOString().slice(0, 10), 0);
		}
		for (const e of harvestLog) {
			const day = e.created_at.slice(0, 10);
			if (days.has(day)) days.set(day, (days.get(day) ?? 0) + Math.max(0, e.delta));
		}
		return Array.from(days.values());
	});

	const chartMax = $derived(() => Math.max(1, ...chartData()));

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdropClick} role="dialog" aria-modal="true">
	<!-- Panel picks up a subtle tint from the sort's assigned color so
	     the modal feels visually tied to the card the user clicked. -->
	<div
		class="modal-panel"
		class:has-color={!!sort.color_hex}
		style={sort.color_hex ? `--card-color: ${sort.color_hex};` : ''}
	>

		<!-- Left panel: photo + info / edit form -->
		<div class="panel-left">
			<div class="photo-container">
				{#if photoSrc}
					<img src={photoSrc} alt={sort.name} class="photo-img" />
				{:else}
					<div class="photo-empty">
						<svg viewBox="0 0 24 24" width="48" height="48" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" opacity="0.3">
							<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12"/>
							<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(72 12 12)"/>
							<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(144 12 12)"/>
							<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(216 12 12)"/>
							<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(288 12 12)"/>
							<circle cx="12" cy="12" r="2"/>
						</svg>
					</div>
				{/if}
				<button class="photo-edit-btn" type="button" onclick={pickNewPhoto} title="Изменить фото">
					<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
				</button>
			</div>

			{#if !editing}
				<div class="info-section">
					<div class="info-name-row">
						<div>
							<h2 class="info-name">{sort.name}</h2>
							{#if sort.variety}<p class="info-variety">{sort.variety}</p>{/if}
						</div>
						<button class="btn-icon" type="button" onclick={() => { editing = true; }} title="Редактировать">
							<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
						</button>
					</div>
					{#if sort.description}
						<p class="info-desc">{sort.description}</p>
					{/if}
					<div class="info-prices">
						{#if sort.purchase_price > 0 && $showDetailedPricing}
							<span class="price-chip">Закупка: {sort.purchase_price}</span>
						{/if}
						{#if sort.sell_price_stem > 0}
							<span class="price-chip primary">Продажа: {sort.sell_price_stem}/шт.</span>
						{/if}
					</div>
				</div>
			{:else}
				<!-- Edit form -->
				<div class="edit-form">
					<div class="field">
						<label class="field-label">Название</label>
						<input class="field-input" type="text" bind:value={editName} />
					</div>
					<div class="field">
						<label class="field-label">Сорт</label>
						<input class="field-input" type="text" bind:value={editVariety} placeholder="необязательно" />
					</div>
					<div class="field">
						<label class="field-label">Описание</label>
						<textarea class="field-input" bind:value={editDescription} rows="2"></textarea>
					</div>
					<div class="fields-row">
						<div class="field">
							<label class="field-label">Закупка</label>
							<input class="field-input" type="number" bind:value={editPurchasePrice} min="0" step="0.01" />
						</div>
						<div class="field">
							<label class="field-label">Цена/шт.</label>
							<input class="field-input" type="number" bind:value={editSellPrice} min="0" step="0.01" />
						</div>
					</div>
					<div class="field">
						<label class="field-label">Цветков в упаковке</label>
						<input class="field-input" type="number" bind:value={editFpp} min="1" placeholder="по умолч." />
					</div>
					<div class="field">
						<span class="field-label">Цвет карточки</span>
						<div class="color-swatches">
							{#each PRESET_COLORS as c}
								<button
									type="button"
									class="color-swatch"
									class:active={editColorHex === c}
									style:background={c}
									onclick={() => (editColorHex = editColorHex === c ? null : c)}
									aria-label="Цвет {c}"
								></button>
							{/each}
							<label class="color-custom" title="Свой цвет">
								<input
									type="color"
									value={editColorHex ?? '#6b7280'}
									oninput={(e) => (editColorHex = e.currentTarget.value)}
								/>
								<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M12 3v1m0 16v1m8.66-13.66l-.71.71M4.05 19.95l-.71.71M21 12h-1M4 12H3m16.66 7.66l-.71-.71M4.05 4.05l-.71-.71"/></svg>
							</label>
						</div>
					</div>
					<div class="edit-actions">
						<button type="button" class="btn-secondary" onclick={() => (editing = false)}>Отмена</button>
						<button type="button" class="btn-primary" onclick={handleSaveEdit} disabled={editSaving}>
							{editSaving ? '…' : 'Сохранить'}
						</button>
					</div>
				</div>
			{/if}

			<button type="button" class="btn-danger" onclick={handleDelete}>Удалить сорт</button>
		</div>

		<!-- Right panel: stock + harvest + chart -->
		<div class="panel-right">
			<button class="btn-close" type="button" onclick={onclose} aria-label="Закрыть">✕</button>

			<!-- Stock KPIs -->
			<div class="stock-grid">
				<div class="stock-cell">
					<span class="stock-label">В наличии</span>
					<span class="stock-val">{sort.raw_stock}</span>
					<span class="stock-unit">шт.</span>
				</div>
				<div class="stock-cell">
					<span class="stock-label">Упаковано</span>
					<span class="stock-val">{sort.pkg_stock}</span>
					<span class="stock-unit">уп.</span>
				</div>
				<div class="stock-cell">
					<span class="stock-label">Всего собрано</span>
					<span class="stock-val">{sort.total_harvested}</span>
					<span class="stock-unit">шт.</span>
				</div>
			</div>

			<!-- Add harvest -->
			<div class="harvest-section">
				<h3 class="section-title">Добавить в наличие</h3>
				<div class="harvest-row">
					<input
						class="field-input harvest-input"
						type="number"
						min="1"
						step="1"
						bind:value={harvestDelta}
						placeholder="0"
					/>
					<select class="field-input harvest-select" bind:value={harvestReason}>
						<option value="manual">Ручное добавление</option>
						<option value="correction">Коррекция</option>
					</select>
				</div>
				<input
					class="field-input"
					type="text"
					bind:value={harvestNote}
					placeholder="Заметка (необязательно)"
				/>
				<button
					type="button"
					class="btn-primary harvest-btn"
					onclick={handleAddHarvest}
					disabled={harvestSaving || harvestDelta <= 0}
				>
					{#if harvestSuccess}
						✓ Добавлено
					{:else}
						{harvestSaving ? '…' : '+ Добавить стебли'}
					{/if}
				</button>
			</div>

			<!-- Orders waiting for this sort — driven by get_orders_waiting_for_sort.
			     Shown above the chart because it's the more actionable panel: the
			     operator usually opens the modal to decide what to pack next. -->
			{#if !waitingLoading && waitingOrders.length > 0}
				<div class="waiting-section">
					<div class="waiting-header">
						<h3 class="section-title">{$t('section_waiting_orders') ?? 'Заказы, ожидающие сорт'}</h3>
						<div class="waiting-totals">
							<span class="wt-chip">{waitingOrders.length} {waitingOrders.length === 1 ? 'заказ' : 'заказ(ов)'}</span>
							<span class="wt-chip wt-ordered">{totalOrderedByWaiting} {$t('label_pack_count') ?? 'уп.'}</span>
							<span class="wt-chip wt-reserved" title={$t('label_assigned_packs')}>↺ {totalReservedByWaiting}</span>
							{#if totalShortage > 0}
								<span class="wt-chip wt-shortage" title={$t('label_deficit')}>⚠ {totalShortage}</span>
							{/if}
						</div>
					</div>
					<div class="waiting-list">
						{#each waitingOrders as o (o.order_id)}
							<div class="waiting-row" class:wr-shortage={o.shortage > 0}>
								<div class="wr-head">
									<span class="wr-customer">{o.customer_name}</span>
									<span class="wr-status wr-s-{o.status}">{$t('status_' + o.status) ?? o.status}</span>
								</div>
								<div class="wr-meta">
									<span class="wr-deadline" class:wr-overdue={o.deadline && new Date(o.deadline) < new Date()}>
										<svg viewBox="0 0 24 24" width="10" height="10" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
										{formatDeadline(o.deadline)}
										{#if o.deadline}
											<span class="wr-countdown">{formatCountdown(o.deadline)}</span>
										{/if}
									</span>
								</div>
								<div class="wr-numbers">
									<span class="wr-num">
										<span class="wr-num-label">{$t('label_pack_count') ?? 'Заказано'}</span>
										<span class="wr-num-val">{o.ordered_packs}</span>
									</span>
									<span class="wr-num">
										<span class="wr-num-label">{$t('label_assigned_packs')}</span>
										<span class="wr-num-val wr-num-reserved">{o.reserved_packs}</span>
									</span>
									{#if o.shortage > 0}
										<span class="wr-num">
											<span class="wr-num-label">{$t('label_deficit')}</span>
											<span class="wr-num-val wr-num-shortage">{o.shortage}</span>
										</span>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Mini chart: last 30 days -->
			<div class="chart-section">
				<h3 class="section-title">Сбор за 30 дней</h3>
				{#if logLoading}
					<p class="chart-empty">Загрузка…</p>
				{:else if harvestLog.length === 0}
					<p class="chart-empty">Нет данных</p>
				{:else}
					<div class="mini-chart" aria-hidden="true">
						{#each chartData() as val}
							<div
								class="chart-bar"
								style:height="{Math.max(2, (val / chartMax()) * 100)}%"
								title="{val} шт."
							></div>
						{/each}
					</div>
				{/if}
			</div>
		</div>

	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0,0,0,0.6);
		backdrop-filter: blur(8px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
		padding: 16px;
	}

	.modal-panel {
		display: flex;
		width: 100%;
		max-width: 820px;
		max-height: 90vh;
		background: var(--color-surface);
		border: 1px solid var(--glass-border);
		border-top-color: rgba(255,255,255,0.14);
		border-radius: 24px;
		overflow: hidden;
		box-shadow: 0 32px 80px rgba(0,0,0,0.55);
	}

	/* Light tint for the detail panel — mirrors the card's accent without
	   obscuring the photo, form fields or harvest log. */
	.modal-panel.has-color {
		background:
			linear-gradient(160deg,
				color-mix(in srgb, var(--card-color) 16%, var(--color-surface)) 0%,
				var(--color-surface) 60%);
		border-color: color-mix(in srgb, var(--card-color) 32%, var(--glass-border));
		box-shadow:
			0 32px 80px rgba(0,0,0,0.55),
			0 0 50px color-mix(in srgb, var(--card-color) 16%, transparent);
	}

	/* Left panel */
	.panel-left {
		width: 280px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--glass-border);
		overflow-y: auto;
	}

	.photo-container {
		position: relative;
		width: 100%;
		aspect-ratio: 4/3;
		background: color-mix(in srgb, var(--color-primary) 8%, transparent);
		flex-shrink: 0;
	}

	.photo-img { width: 100%; height: 100%; object-fit: cover; display: block; }

	.photo-empty {
		width: 100%; height: 100%;
		display: flex; align-items: center; justify-content: center;
		color: var(--color-primary);
	}

	.photo-edit-btn {
		position: absolute;
		bottom: 8px; right: 8px;
		background: rgba(0,0,0,0.55);
		border: none;
		border-radius: 8px;
		padding: 6px 8px;
		cursor: pointer;
		color: #fff;
		display: flex;
		align-items: center;
		transition: background 0.15s;
	}
	.photo-edit-btn:hover { background: rgba(0,0,0,0.75); }

	.info-section { padding: 16px; flex: 1; }
	.info-name-row { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; }
	.info-name { font-size: 1.1rem; font-weight: 700; margin: 0; color: var(--color-on-surface); }
	.info-variety { font-size: 0.8rem; color: var(--color-outline); margin: 2px 0 0; }
	.info-desc { font-size: 0.82rem; color: var(--color-outline); margin: 8px 0 0; line-height: 1.5; }
	.info-prices { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
	.price-chip {
		font-size: 0.72rem; padding: 3px 8px;
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 20px; color: var(--color-on-surface);
	}
	.price-chip.primary { border-color: var(--color-primary); color: var(--color-primary); }

	.btn-icon {
		background: none; border: none; cursor: pointer;
		color: var(--color-outline); padding: 4px; border-radius: 6px;
		transition: color 0.1s;
	}
	.btn-icon:hover { color: var(--color-on-surface); }

	/* Edit form */
	.edit-form { padding: 16px; display: flex; flex-direction: column; gap: 10px; flex: 1; }
	.field { display: flex; flex-direction: column; gap: 3px; }
	.fields-row { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
	.field-label { font-size: 0.72rem; color: var(--color-outline); }
	.field-input {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 8px; padding: 7px 10px; font-size: 0.85rem;
		color: var(--color-on-surface); outline: none; width: 100%; box-sizing: border-box;
	}
	.field-input:focus { border-color: var(--color-primary); }
	.edit-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 4px; }

	.btn-danger {
		margin: 0 16px 16px;
		background: none;
		border: 1px solid var(--color-alert-red, #ef4444);
		color: var(--color-alert-red, #ef4444);
		border-radius: 10px;
		padding: 8px 14px;
		font-size: 0.82rem;
		cursor: pointer;
		transition: background 0.15s;
	}
	.btn-danger:hover { background: rgba(239,68,68,0.1); }

	/* Right panel */
	.panel-right {
		flex: 1;
		display: flex;
		flex-direction: column;
		padding: 24px;
		gap: 20px;
		overflow-y: auto;
		position: relative;
	}

	.btn-close {
		position: absolute;
		top: 16px; right: 16px;
		background: none; border: none;
		color: var(--color-outline); font-size: 1rem;
		cursor: pointer; padding: 4px 8px; border-radius: 8px;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	/* Stock KPIs */
	.stock-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 10px;
		padding-top: 4px;
	}

	.stock-cell {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 12px;
		display: flex; flex-direction: column; gap: 2px;
	}

	.stock-label { font-size: 0.7rem; color: var(--color-outline); }
	.stock-val { font-size: 1.6rem; font-weight: 700; color: var(--color-primary); line-height: 1.1; }
	.stock-unit { font-size: 0.72rem; color: var(--color-outline); }

	/* Harvest */
	.harvest-section { display: flex; flex-direction: column; gap: 10px; }
	.section-title { font-size: 0.85rem; font-weight: 600; margin: 0; color: var(--color-on-surface); opacity: 0.7; text-transform: uppercase; letter-spacing: 0.04em; }

	.harvest-row { display: grid; grid-template-columns: 1fr 2fr; gap: 8px; }
	.harvest-input { text-align: center; }
	.harvest-select { appearance: none; }

	.harvest-btn {
		width: 100%;
		padding: 10px;
		font-size: 0.9rem;
		transition: background 0.15s, opacity 0.15s;
	}

	/* Waiting orders */
	.waiting-section { display: flex; flex-direction: column; gap: 10px; }
	.waiting-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 8px;
	}
	.waiting-totals { display: flex; gap: 6px; flex-wrap: wrap; }
	.wt-chip {
		font-size: 0.7rem;
		padding: 2px 8px;
		border-radius: 20px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-outline);
		font-variant-numeric: tabular-nums;
	}
	.wt-chip.wt-ordered { color: var(--color-on-surface); }
	.wt-chip.wt-reserved {
		color: #f59e0b;
		border-color: color-mix(in srgb, #f59e0b 40%, transparent);
	}
	.wt-chip.wt-shortage {
		color: var(--color-alert-red, #ef4444);
		border-color: color-mix(in srgb, var(--color-alert-red, #ef4444) 40%, transparent);
		background: color-mix(in srgb, var(--color-alert-red, #ef4444) 8%, transparent);
	}

	.waiting-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.waiting-row {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		transition: border-color 0.15s, background 0.15s;
	}
	.waiting-row.wr-shortage {
		border-color: color-mix(in srgb, var(--color-alert-red, #ef4444) 40%, transparent);
		background: color-mix(in srgb, var(--color-alert-red, #ef4444) 5%, var(--glass-bg));
	}
	.wr-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
	.wr-customer { font-size: 0.9rem; font-weight: 600; color: var(--color-on-surface); }
	.wr-status {
		font-size: 0.66rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 2px 7px;
		border-radius: 999px;
		border: 1px solid var(--glass-border);
	}
	.wr-s-pending { color: #f59e0b; border-color: color-mix(in srgb, #f59e0b 35%, transparent); }
	.wr-s-in_progress { color: #3b82f6; border-color: color-mix(in srgb, #3b82f6 35%, transparent); }
	.wr-meta { display: flex; align-items: center; gap: 10px; font-size: 0.78rem; color: var(--color-outline); }
	.wr-deadline { display: inline-flex; align-items: center; gap: 5px; }
	.wr-deadline.wr-overdue { color: var(--color-alert-red, #ef4444); }
	.wr-countdown {
		padding: 1px 7px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
		color: var(--color-primary);
		font-weight: 500;
		font-size: 0.72rem;
	}
	.wr-numbers { display: flex; gap: 10px; flex-wrap: wrap; }
	.wr-num {
		display: inline-flex;
		flex-direction: column;
		background: color-mix(in srgb, var(--color-primary) 5%, transparent);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 5px 10px;
		min-width: 68px;
	}
	.wr-num-label {
		font-size: 0.62rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-outline);
	}
	.wr-num-val {
		font-size: 0.95rem;
		font-weight: 700;
		color: var(--color-on-surface);
		font-variant-numeric: tabular-nums;
	}
	.wr-num-val.wr-num-reserved { color: #f59e0b; }
	.wr-num-val.wr-num-shortage { color: var(--color-alert-red, #ef4444); }

	/* Chart */
	.chart-section { display: flex; flex-direction: column; gap: 10px; }
	.chart-empty { font-size: 0.82rem; color: var(--color-outline); margin: 0; }

	.mini-chart {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 64px;
	}

	.chart-bar {
		flex: 1;
		background: var(--color-primary);
		border-radius: 2px 2px 0 0;
		opacity: 0.7;
		min-height: 2px;
		transition: height 0.3s var(--ease-spring);
	}

	/* Shared buttons */
	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		border: none; border-radius: 10px;
		padding: 8px 16px; font-size: 0.88rem; font-weight: 600;
		cursor: pointer; transition: opacity 0.15s;
	}
	.btn-primary:disabled { opacity: 0.45; cursor: not-allowed; }

	.btn-secondary {
		background: var(--glass-bg); color: var(--color-on-surface);
		border: 1px solid var(--glass-border); border-radius: 10px;
		padding: 8px 14px; font-size: 0.88rem; cursor: pointer;
	}
	.btn-secondary:hover { background: var(--glass-bg-hover); }

	/* Color swatches */
	.color-swatches {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
		align-items: center;
	}
	.color-swatch {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;
		padding: 0;
	}
	.color-swatch:hover { transform: scale(1.18); }
	.color-swatch.active {
		border-color: var(--color-on-surface);
		box-shadow: 0 0 0 2px var(--color-surface), 0 0 8px currentColor;
	}
	.color-custom {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 1.5px dashed var(--color-outline);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		position: relative;
		overflow: hidden;
		color: var(--color-outline);
	}
	.color-custom:hover { border-color: var(--color-primary); }
	.color-custom input[type="color"] {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		opacity: 0;
		cursor: pointer;
		border: none;
		padding: 0;
	}

	/* Light mode */
	:global([data-theme="light"]) .modal-panel { background: var(--color-surface, #fafafa); }
	:global([data-theme="light"]) .field-input { background: rgba(0,0,0,0.04); border-color: rgba(0,0,0,0.12); }
</style>
