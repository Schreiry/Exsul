<script lang="ts">
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { orders } from '$lib/stores/orders';
	import { commands } from '$lib/tauri/commands';
	import FlowerCard from '$lib/components/greenhouse/FlowerCard.svelte';
	import ContactPicker from '$lib/components/contacts/ContactPicker.svelte';
	import type { Contact, FlowerSort } from '$lib/tauri/types';

	interface Props {
		onclose: () => void;
		ondone?: () => void;
	}

	let { onclose, ondone }: Props = $props();

	// ── State ─────────────────────────────────────────────────────
	let selected = $state<FlowerSort | null>(null);
	let packCount = $state(1);
	let searchQuery = $state('');

	// Price per pack: manually overridable; null = use autoPricePerPack
	let pricePerPackOverride = $state<number | null>(null);

	// Order form (optional)
	let customerName = $state('');
	let customerPhone = $state('');
	let customerEmail = $state('');
	let deliveryAddress = $state('');
	let deadline = $state('');
	let notes = $state('');
	let cardColor = $state('');

	// Phase E6 — optional contact attachment for the created order.
	let selectedContact = $state<Contact | null>(null);
	let selectedLocationId = $state<string>('');

	async function handlePickContact(c: Contact) {
		selectedContact = c;
		customerName = c.name + (c.surname ? ` ${c.surname}` : '');
		customerEmail = c.email ?? customerEmail;
		customerPhone = c.phone ?? customerPhone;
		if (c.default_address && !deliveryAddress) deliveryAddress = c.default_address;
		try {
			const locs = await commands.getContactLocations(c.id);
			const def = locs.find((l) => l.is_default) ?? locs[0];
			if (def) {
				selectedLocationId = def.id;
				if (!deliveryAddress) deliveryAddress = def.address;
			}
		} catch (e) {
			console.warn('Failed to load contact locations:', e);
		}
	}

	function handleClearContact() {
		selectedContact = null;
		selectedLocationId = '';
	}

	let saving = $state(false);
	let error = $state('');
	let successMsg = $state('');

	// ── Derived ───────────────────────────────────────────────────
	const fpp = $derived(
		selected?.flowers_per_pack_override ?? $flowerConstants.flowers_per_pack ?? 10
	);
	const stemsNeeded = $derived(packCount * fpp);
	const stockOk = $derived(!selected || stemsNeeded <= selected.raw_stock);

	// Autofill price/pack from sort's per-stem price; fallback to global constant.
	const autoPricePerPack = $derived(
		selected && selected.sell_price_stem > 0
			? selected.sell_price_stem * fpp
			: ($flowerConstants.price_per_pack ?? 0)
	);
	const effectivePricePerPack = $derived(
		pricePerPackOverride !== null ? pricePerPackOverride : autoPricePerPack
	);
	const autoPrice = $derived(packCount * effectivePricePerPack);

	const visibleSorts = $derived(() => {
		const q = searchQuery.toLowerCase();
		return $flowerSorts.filter(
			(s) => !q || s.name.toLowerCase().includes(q) || (s.variety ?? '').toLowerCase().includes(q)
		);
	});

	// When selected sort changes, reset count and price override
	function selectSort(sort: FlowerSort) {
		if (sort.raw_stock === 0) return;
		selected = sort;
		packCount = 1;
		pricePerPackOverride = null;
		error = '';
	}

	async function handleConfirm() {
		if (!selected || !stockOk || packCount < 1) return;
		saving = true;
		error = '';
		try {
			const hasCustomer = customerName.trim().length > 0;
			const sortId = selected.id;

			// Single atomic Rust call: order (optional) + packaging + order_item +
			// pack_assignment + total_amount recompute, all in one SQLite
			// transaction. If anything fails, nothing is written — no more
			// "packaging_log present but order_item missing" half-states.
			await commands.packageFlowersWithOrder({
				sort_id: sortId,
				pack_count: packCount,
				price_per_pack: effectivePricePerPack,
				customer_name: hasCustomer ? customerName.trim() : undefined,
				customer_email: customerEmail.trim() || undefined,
				customer_phone: customerPhone.trim() || undefined,
				delivery_address: deliveryAddress.trim() || undefined,
				deadline: deadline || undefined,
				notes: notes.trim() || undefined,
				card_color: cardColor || undefined,
				contact_id: selectedContact?.id,
				contact_location_id: selectedLocationId || undefined,
			});

			// Refresh frontend stores so the warehouse + orders pages see the
			// new state without a manual reload.
			await flowerSorts.load();
			if (hasCustomer) await orders.load();

			successMsg = `✓ Упаковано ${packCount} уп. (${stemsNeeded} шт.)`;
			// Reset form
			selected = null;
			packCount = 1;
			pricePerPackOverride = null;
			customerName = ''; customerPhone = ''; customerEmail = '';
			deliveryAddress = ''; deadline = ''; notes = ''; cardColor = '';
			selectedContact = null; selectedLocationId = '';
			ondone?.();
			setTimeout(() => {
				successMsg = '';
				onclose();
			}, 1800);
		} catch (e) {
			error = String(e);
		} finally {
			saving = false;
		}
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
<div class="modal-backdrop" onclick={handleBackdrop} role="dialog" aria-modal="true" aria-label="Упаковка">
	<div class="modal-panel">

		<!-- ── Left 65%: material selection ───────────────────── -->
		<div class="panel-left">
			<div class="panel-header">
				<h2 class="panel-title">Выберите материал</h2>
				<input
					type="search"
					class="search-input"
					placeholder="Поиск…"
					bind:value={searchQuery}
				/>
			</div>

			{#if $flowerSorts.length === 0}
				<div class="empty-left">
					<p>Нет сырья в оранжерее</p>
					<p class="empty-sub">Сначала добавьте сырьё в Оранжерее</p>
				</div>
			{:else}
				<div class="sorts-grid">
					{#each visibleSorts() as sort (sort.id)}
						<div class="sort-card-wrap" class:unavailable={sort.raw_stock === 0}>
							<FlowerCard
								{sort}
								compact
								selected={selected?.id === sort.id}
								onclick={() => selectSort(sort)}
							/>
							{#if sort.raw_stock === 0}
								<div class="unavail-overlay">Нет в наличии</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- ── Right 35%: details + order ─────────────────────── -->
		<div class="panel-right">
			<button class="btn-close" type="button" onclick={onclose} aria-label="Закрыть">✕</button>

			{#if successMsg}
				<div class="success-banner">{successMsg}</div>
			{:else if !selected}
				<div class="right-empty">
					<svg viewBox="0 0 24 24" width="40" height="40" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" opacity="0.25">
						<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
					</svg>
					<p>Выберите сырьё слева</p>
				</div>
			{:else}
				<!-- Material info -->
				<div class="material-info">
					<p class="mat-name">{selected.name}</p>
					{#if selected.variety}<p class="mat-variety">{selected.variety}</p>{/if}
					<div class="mat-stocks">
						<span class="mat-stock-chip">
							<span class="chip-val">{selected.raw_stock}</span> шт. сырья
						</span>
						<span class="mat-stock-chip">
							<span class="chip-val">{selected.pkg_stock}</span> уп. готово
						</span>
					</div>
				</div>

				<div class="divider"></div>

				<!-- Pack count -->
				<div class="pack-section">
					<label class="field-label">Количество упаковок</label>
					<div class="stepper-row">
						<button type="button" class="stepper-btn" onclick={() => packCount = Math.max(1, packCount - 1)}>−</button>
						<input
							class="stepper-input"
							class:invalid={!stockOk}
							type="number"
							min="1"
							bind:value={packCount}
						/>
						<button type="button" class="stepper-btn" onclick={() => packCount += 1}>+</button>
					</div>

					<div class="stems-info" class:stems-err={!stockOk}>
						<span>{stemsNeeded} стеблей</span>
						<span class="stems-available">{!stockOk ? '⚠ недостаточно' : `из ${selected.raw_stock}`}</span>
					</div>

					<div class="price-row">
						<label class="field-label" for="price-per-pack-input">Цена за упаковку</label>
						<div class="price-input-wrap">
							<input
								id="price-per-pack-input"
								class="field-input price-input"
								type="number"
								min="0"
								step="0.01"
								value={effectivePricePerPack}
								oninput={(e) => {
									const v = (e.currentTarget as HTMLInputElement).value;
									pricePerPackOverride = v === '' ? null : Number(v);
								}}
							/>
							{#if pricePerPackOverride !== null && pricePerPackOverride !== autoPricePerPack}
								<button
									type="button"
									class="price-reset"
									title="Сбросить к авто"
									onclick={() => (pricePerPackOverride = null)}
								>↺</button>
							{/if}
						</div>
					</div>

					{#if autoPrice > 0}
						<p class="auto-price">Сумма: <strong>{autoPrice.toLocaleString()}</strong></p>
					{/if}
				</div>

				<div class="divider"></div>

				<!-- Optional order creation -->
				<details class="order-section">
					<summary class="order-summary">Создать заказ (необязательно)</summary>
					<div class="order-fields">
						<div class="field">
							<label class="field-label">Контакт</label>
							<ContactPicker
								selectedId={selectedContact?.id ?? ''}
								freeName={customerName}
								onselect={(c) => { void handlePickContact(c); }}
								onclear={handleClearContact}
								onfreeName={(v) => (customerName = v)}
							/>
						</div>
						<div class="field">
							<label class="field-label">Клиент *</label>
							<input class="field-input" type="text" bind:value={customerName} placeholder="Имя клиента" />
						</div>
						<div class="field">
							<label class="field-label">Телефон</label>
							<input class="field-input" type="tel" bind:value={customerPhone} placeholder="+7…" />
						</div>
						<div class="field">
							<label class="field-label">Email</label>
							<input class="field-input" type="email" bind:value={customerEmail} placeholder="email@…" />
						</div>
						<div class="field">
							<label class="field-label">Адрес доставки</label>
							<input class="field-input" type="text" bind:value={deliveryAddress} placeholder="Город, улица…" />
						</div>
						<div class="field">
							<label class="field-label">Срок</label>
							<input class="field-input" type="datetime-local" bind:value={deadline} />
						</div>
						<div class="field">
							<label class="field-label">Заметки</label>
							<textarea class="field-input" bind:value={notes} rows="2" placeholder="Дополнительно…"></textarea>
						</div>
						<div class="field">
							<label class="field-label" for="pack-card-color">Цвет карточки заказа</label>
							<div class="color-row">
								<input
									id="pack-card-color"
									class="color-swatch-input"
									type="color"
									value={cardColor || '#888888'}
									oninput={(e) => (cardColor = (e.currentTarget as HTMLInputElement).value)}
								/>
								<input
									class="field-input color-hex"
									type="text"
									placeholder="#rrggbb — авто"
									bind:value={cardColor}
								/>
								{#if cardColor}
									<button
										type="button"
										class="btn-reset-color"
										title="Авто"
										onclick={() => (cardColor = '')}
									>↺</button>
								{/if}
							</div>
						</div>
					</div>
				</details>

				{#if error}
					<p class="error-msg">{error}</p>
				{/if}

				<button
					type="button"
					class="btn-confirm"
					class:invalid={!stockOk}
					onclick={handleConfirm}
					disabled={saving || !stockOk || packCount < 1}
				>
					{saving ? '…' : !stockOk ? 'Нет стеблей' : `Упаковать ${packCount} уп.`}
				</button>
			{/if}
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
		max-width: 960px;
		height: min(90vh, 680px);
		background: var(--color-surface);
		border: 1px solid var(--glass-border);
		border-top-color: rgba(255,255,255,0.14);
		border-radius: 24px;
		overflow: hidden;
		box-shadow: 0 32px 80px rgba(0,0,0,0.55);
	}

	/* Left panel */
	.panel-left {
		flex: 1;
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--glass-border);
		overflow: hidden;
	}

	.panel-header {
		padding: 20px 20px 12px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		flex-shrink: 0;
		border-bottom: 1px solid var(--glass-border);
	}

	.panel-title {
		font-size: 1rem;
		font-weight: 700;
		margin: 0;
		color: var(--color-on-surface);
	}

	.search-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 7px 12px;
		font-size: 0.85rem;
		color: var(--color-on-surface);
		outline: none;
	}
	.search-input:focus { border-color: var(--color-primary); }

	.sorts-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 10px;
		padding: 14px;
		overflow-y: auto;
		flex: 1;
	}

	.sort-card-wrap {
		position: relative;
	}

	.unavailable { opacity: 0.45; pointer-events: none; }

	.unavail-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.72rem;
		color: var(--color-outline);
		background: rgba(0,0,0,0.2);
		border-radius: 16px;
		pointer-events: none;
	}

	.empty-left {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 6px;
		font-size: 0.9rem;
		color: var(--color-outline);
		text-align: center;
		padding: 24px;
	}

	.empty-sub { font-size: 0.78rem; }

	/* Right panel */
	.panel-right {
		width: 300px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		padding: 20px;
		gap: 14px;
		overflow-y: auto;
		position: relative;
	}

	.btn-close {
		position: absolute;
		top: 12px; right: 12px;
		background: none; border: none;
		color: var(--color-outline); font-size: 1rem;
		cursor: pointer; padding: 4px 8px; border-radius: 8px;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	.right-empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		color: var(--color-outline);
		font-size: 0.88rem;
		text-align: center;
	}

	.success-banner {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-primary);
		text-align: center;
	}

	/* Material info */
	.material-info { padding-top: 28px; }
	.mat-name { font-size: 1rem; font-weight: 700; margin: 0; color: var(--color-on-surface); }
	.mat-variety { font-size: 0.78rem; color: var(--color-outline); margin: 2px 0 8px; }
	.mat-stocks { display: flex; gap: 8px; flex-wrap: wrap; }
	.mat-stock-chip {
		font-size: 0.75rem;
		padding: 3px 8px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 20px;
		color: var(--color-outline);
	}
	.chip-val { font-weight: 700; color: var(--color-primary); }

	.divider { height: 1px; background: var(--glass-border); flex-shrink: 0; }

	/* Pack section */
	.pack-section { display: flex; flex-direction: column; gap: 8px; }
	.field-label { font-size: 0.72rem; color: var(--color-outline); font-weight: 500; }

	.stepper-row { display: flex; align-items: center; gap: 6px; }
	.stepper-btn {
		width: 32px; height: 32px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		font-size: 1.1rem;
		cursor: pointer;
		color: var(--color-on-surface);
		display: flex; align-items: center; justify-content: center;
		transition: background 0.1s;
		flex-shrink: 0;
	}
	.stepper-btn:hover { background: var(--glass-bg-hover); }

	.stepper-input {
		flex: 1;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 7px 10px;
		font-size: 1rem;
		font-weight: 600;
		text-align: center;
		color: var(--color-on-surface);
		outline: none;
		transition: border-color 0.15s;
	}
	.stepper-input.invalid { border-color: var(--color-alert-red); color: var(--color-alert-red); }
	.stepper-input:focus { border-color: var(--color-primary); }

	.stems-info {
		display: flex;
		justify-content: space-between;
		font-size: 0.78rem;
		color: var(--color-outline);
	}
	.stems-info.stems-err { color: var(--color-alert-red); }
	.stems-available { opacity: 0.7; }

	.auto-price { font-size: 0.82rem; color: var(--color-outline); margin: 0; }
	.auto-price strong { color: var(--color-primary); }

	.price-row { display: flex; flex-direction: column; gap: 3px; }
	.price-input-wrap { position: relative; display: flex; align-items: center; }
	.price-input { padding-right: 34px; font-weight: 600; }
	.price-reset {
		position: absolute;
		right: 6px;
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 6px;
		font-size: 0.95rem;
		line-height: 1;
	}
	.price-reset:hover { color: var(--color-primary); background: var(--glass-bg-hover); }

	/* Order section */
	.order-section { font-size: 0.85rem; }
	.order-summary {
		cursor: pointer;
		color: var(--color-outline);
		font-size: 0.78rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 4px 0;
		user-select: none;
	}
	.order-summary:hover { color: var(--color-primary); }
	.order-fields { display: flex; flex-direction: column; gap: 8px; margin-top: 10px; }
	.field { display: flex; flex-direction: column; gap: 3px; }
	.field-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 7px 10px;
		font-size: 0.82rem;
		color: var(--color-on-surface);
		outline: none;
		width: 100%;
		box-sizing: border-box;
		font-family: inherit;
	}
	.field-input:focus { border-color: var(--color-primary); }

	.error-msg { font-size: 0.8rem; color: var(--color-alert-red); margin: 0; }

	/* Card-color row (Phase D) */
	.color-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.color-swatch-input {
		width: 32px;
		height: 32px;
		padding: 0;
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		background: transparent;
		cursor: pointer;
		flex-shrink: 0;
	}
	.color-swatch-input::-webkit-color-swatch { border: none; border-radius: 6px; }
	.color-swatch-input::-webkit-color-swatch-wrapper { padding: 2px; border-radius: 8px; }
	.color-hex { flex: 1; }
	.btn-reset-color {
		background: none;
		border: none;
		color: var(--color-outline);
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 6px;
		font-size: 0.95rem;
		line-height: 1;
		flex-shrink: 0;
	}
	.btn-reset-color:hover { color: var(--color-primary); background: var(--glass-bg-hover); }

	/* Confirm button */
	.btn-confirm {
		width: 100%;
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		border: none;
		border-radius: 12px;
		padding: 12px;
		font-size: 0.95rem;
		font-weight: 700;
		cursor: pointer;
		transition: opacity 0.15s;
		margin-top: auto;
	}
	.btn-confirm:disabled { opacity: 0.45; cursor: not-allowed; }
	.btn-confirm.invalid { background: var(--color-alert-red); }

	/* Light mode */
	:global([data-theme="light"]) .modal-panel { background: var(--color-surface, #fafafa); }
	:global([data-theme="light"]) .field-input, :global([data-theme="light"]) .search-input {
		background: rgba(0,0,0,0.04);
		border-color: rgba(0,0,0,0.12);
	}
</style>
