<script lang="ts">
	import { orders } from '$lib/stores/orders';
	import { inventory } from '$lib/stores/inventory';
	import { flowerSorts } from '$lib/stores/flowers';
	import { preset } from '$lib/stores/preset';
	import { t } from '$lib/stores/i18n';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import GlassDropdown from '$lib/components/common/GlassDropdown.svelte';
	import OrderProgressBar from '$lib/components/orders/OrderProgressBar.svelte';
	import OrderDetailModal from '$lib/components/orders/OrderDetailModal.svelte';
	import { formatCountdown } from '$lib/utils/countdown';
	import type { CreateOrderPayload, AddOrderItemPayload, OrderStatus, OrderItem, Order } from '$lib/tauri/types';

	const isFlowers = $derived($preset === 'flowers');

	$effect(() => {
		orders.load();
		if (isFlowers) flowerSorts.load();
	});

	type FilterTab = 'all' | OrderStatus;
	let activeTab = $state<FilterTab>('all');
	let showForm = $state(false);
	let detailOrder = $state<Order | null>(null);

	// Live countdown — one shared timer updates all cards each minute
	let countdowns = $state<Record<string, string>>({});
	$effect(() => {
		function updateCountdowns() {
			const next: Record<string, string> = {};
			for (const o of $orders) {
				if (o.deadline) next[o.id] = formatCountdown(o.deadline);
			}
			countdowns = next;
		}
		updateCountdowns();
		const id = setInterval(updateCountdowns, 60_000);
		return () => clearInterval(id);
	});

	let formData = $state<CreateOrderPayload>({
		customer_name: '',
		customer_email: '',
		customer_phone: '',
		deadline: '',
		notes: '',
	});

	// Inline order items being added while creating order
	let pendingItems = $state<Omit<AddOrderItemPayload, 'order_id'>[]>([]);
	let newItemRow = $state({ item_id: '', quantity: 1, unit_price: 0 });

	let filteredOrders = $derived(
		activeTab === 'all' ? $orders : $orders.filter((o) => o.status === activeTab)
	);


	const statusColors: Record<string, string> = {
		pending: '#f59e0b',
		in_progress: '#3b82f6',
		completed: '#10b981',
		cancelled: '#6b7280',
	};

	// Shortage detection for flowers preset
	const shortages = $derived.by(() => {
		if (!isFlowers) return new Map<number, number>();
		const map = new Map<number, number>();
		for (let i = 0; i < pendingItems.length; i++) {
			const item = pendingItems[i];
			const sort = $flowerSorts.find((s) => s.id === item.item_id);
			if (sort && item.quantity > sort.pkg_stock) {
				map.set(i, item.quantity - sort.pkg_stock);
			}
		}
		return map;
	});

	function addPendingItem() {
		if (!newItemRow.item_id) return;
		pendingItems = [...pendingItems, { ...newItemRow }];
		newItemRow = { item_id: '', quantity: 1, unit_price: 0 };
	}

	function removePendingItem(index: number) {
		pendingItems = pendingItems.filter((_, i) => i !== index);
	}

	async function handleSubmit() {
		if (!formData.customer_name.trim()) return;
		const orderId = await orders.create({
			customer_name: formData.customer_name,
			customer_email: formData.customer_email || undefined,
			customer_phone: formData.customer_phone || undefined,
			deadline: formData.deadline || undefined,
			notes: formData.notes || undefined,
		});
		for (const item of pendingItems) {
			await orders.addItem({ ...item, order_id: orderId });
		}
		formData = { customer_name: '', customer_email: '', customer_phone: '', deadline: '', notes: '' };
		pendingItems = [];
		showForm = false;
	}

	async function printPreorder(order: typeof filteredOrders[0]) {
		const items = await orders.getItems(order.id);
		const el = document.createElement('div');
		el.className = 'print-preorder';

		const itemRows = items.map((it: OrderItem) => {
			const name = isFlowers
				? ($flowerSorts.find(s => s.id === it.item_id)?.name ?? it.item_id)
				: ($inventory.find(inv => inv.id === it.item_id)?.name ?? it.item_id);
			const lineTotal = it.quantity * it.unit_price;
			return `<tr><td>${name}</td><td>${it.quantity}</td><td>${formatAmount(it.unit_price, $globalCurrency)}</td><td>${formatAmount(lineTotal, $globalCurrency)}</td></tr>`;
		}).join('');

		el.innerHTML = `
			<h1>${$t('action_print_preorder')}</h1>
			<p><strong>${$t('label_customer_name')}:</strong> ${order.customer_name}</p>
			${order.customer_phone ? `<p><strong>${$t('label_customer_phone')}:</strong> ${order.customer_phone}</p>` : ''}
			${order.customer_email ? `<p><strong>${$t('label_customer_email')}:</strong> ${order.customer_email}</p>` : ''}
			${order.deadline ? `<p><strong>${$t('label_deadline')}:</strong> ${new Date(order.deadline).toLocaleString()}</p>` : ''}
			<table>
				<thead><tr><th>${$t('label_name')}</th><th>Qty</th><th>Price</th><th>Total</th></tr></thead>
				<tbody>${itemRows}</tbody>
			</table>
			<p class="grand-total"><strong>Total: ${formatAmount(order.total_amount, $globalCurrency)}</strong></p>
			${order.notes ? `<p><em>${order.notes}</em></p>` : ''}
		`;

		document.body.appendChild(el);
		window.print();
		document.body.removeChild(el);
	}

	const tabs: { key: FilterTab; label: string }[] = [
		{ key: 'all', label: $t('status_all') },
		{ key: 'pending', label: $t('status_pending') },
		{ key: 'in_progress', label: $t('status_in_progress') },
		{ key: 'completed', label: $t('status_completed') },
		{ key: 'cancelled', label: $t('status_cancelled') },
	];
</script>

<div class="page">
	<div class="page-header">
		<h1>{$t('page_orders_title')}</h1>
		<button class="btn-primary" onclick={() => (showForm = !showForm)}>
			{showForm ? $t('action_cancel') : $t('action_create_order')}
		</button>
	</div>

	{#if showForm}
		<div class="form-card">
			<h2>{$t('action_create_order')}</h2>
			<div class="form-grid">
				<label class="field">
					<span>{$t('label_customer_name')} *</span>
					<input type="text" bind:value={formData.customer_name} required />
				</label>
				<label class="field">
					<span>{$t('label_customer_email')}</span>
					<input type="email" bind:value={formData.customer_email} />
				</label>
				<label class="field">
					<span>{$t('label_customer_phone')}</span>
					<input type="tel" bind:value={formData.customer_phone} />
				</label>
				<label class="field">
					<span>{$t('label_deadline')}</span>
					<input type="datetime-local" bind:value={formData.deadline} />
				</label>
			</div>
			<label class="field full-width">
				<span>{$t('label_notes')}</span>
				<textarea bind:value={formData.notes} rows="3"></textarea>
			</label>

			<div class="order-items-section">
				<h3>{$t('action_add_order_item')}</h3>
				{#each pendingItems as item, i}
					<div class="item-row" class:item-shortage={shortages.has(i)}>
						<span>{isFlowers
							? ($flowerSorts.find(s => s.id === item.item_id)?.name ?? item.item_id)
							: ($inventory.find(it => it.id === item.item_id)?.name ?? item.item_id)
						}</span>
						<span>x{item.quantity}</span>
						<span>{item.unit_price.toFixed(2)}</span>
						<button class="btn-ghost" onclick={() => removePendingItem(i)}>×</button>
					</div>
					{#if shortages.has(i)}
						<div class="shortage-alert">
							<span class="shortage-icon">⚠</span>
							<span>{$t('shortage_alert')} — {$t('shortage_deficit', { n: shortages.get(i)! })}</span>
						</div>
					{/if}
				{/each}
				<div class="add-item-row">
					<div class="add-item-dropdown">
						<GlassDropdown
							items={isFlowers
								? $flowerSorts.map(s => ({ value: s.id, label: s.name + (s.variety ? ` — ${s.variety}` : '') }))
								: $inventory.map(it => ({ value: it.id, label: it.name }))}
							bind:value={newItemRow.item_id}
							placeholder="— {$t('label_name')} —"
						/>
					</div>
					<input type="number" min="1" bind:value={newItemRow.quantity} placeholder="Qty" />
					<input type="number" min="0" step="0.01" bind:value={newItemRow.unit_price} placeholder="Price" />
					<button class="btn-secondary" onclick={addPendingItem}>+</button>
				</div>
			</div>

			<div class="form-actions">
				<button class="btn-ghost" onclick={() => (showForm = false)}>{$t('action_cancel')}</button>
				<button class="btn-primary" onclick={handleSubmit} disabled={!formData.customer_name.trim()}>
					{$t('action_save')}
				</button>
			</div>
		</div>
	{/if}

	<div class="tabs">
		{#each tabs as tab}
			<button
				class="tab"
				class:active={activeTab === tab.key}
				onclick={() => (activeTab = tab.key)}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	{#if filteredOrders.length === 0}
		<p class="empty">{$t('empty_no_orders')}</p>
	{:else}
		<div class="order-list">
			{#each filteredOrders as order (order.id)}
				<div class="order-card" role="button" tabindex="0"
					onclick={() => (detailOrder = order)}
					onkeydown={(e) => e.key === 'Enter' && (detailOrder = order)}
				>
					<div class="order-header">
						<div class="order-meta">
							<span class="customer-name">{order.customer_name}</span>
							{#if order.delivery_address}
								<span class="order-location">
									<svg viewBox="0 0 24 24" width="11" height="11" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M21 10c0 7-9 13-9 13S3 17 3 10a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>
									{order.delivery_address}
								</span>
							{/if}
						</div>
						<div class="order-right">
							<span class="status-dot" style:background={statusColors[order.status]}></span>
							<span class="status-label">{$t('status_' + order.status)}</span>
							<span class="amount">{formatAmount(order.total_amount, $globalCurrency)}</span>
						</div>
					</div>

					<!-- Deadline + live countdown -->
					{#if order.deadline}
						<div class="deadline-row" class:overdue={new Date(order.deadline) < new Date()}>
							<svg viewBox="0 0 24 24" width="11" height="11" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
							<span class="deadline-date">{new Date(order.deadline).toLocaleString('ru', {day:'2-digit',month:'short',hour:'2-digit',minute:'2-digit'})}</span>
							{#if countdowns[order.id]}
								<span class="countdown" class:overdue={countdowns[order.id] === 'просрочен'}>
									{countdowns[order.id]}
								</span>
							{/if}
						</div>
					{/if}

					<!-- Progress bar (replaces slider) -->
					<div class="progress-row" role="presentation" onclick={(e) => e.stopPropagation()}>
						<OrderProgressBar
							status={order.status}
							onchange={(s) => orders.updateStatus(order.id, s)}
						/>
					</div>

					<div class="order-actions" role="presentation" onclick={(e) => e.stopPropagation()}>
						<button class="btn-ghost btn-print" onclick={() => printPreorder(order)}>
							<svg viewBox="0 0 24 24" width="13" height="13" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><polyline points="6 9 6 2 18 2 18 9"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><rect x="6" y="14" width="12" height="8"/></svg>
							{$t('action_print_preorder')}
						</button>
						<span class="detail-hint">Нажмите для деталей →</span>
					</div>

					{#if order.notes}
						<p class="order-notes">{order.notes}</p>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

{#if detailOrder}
	<OrderDetailModal
		bind:order={detailOrder}
		onclose={() => (detailOrder = null)}
	/>
{/if}

<style>
	.page { max-width: 900px; margin: 0 auto; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 24px;
	}

	h1 {
		font-size: 1.75rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0;
	}

	.form-card {
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
		padding: 24px;
		margin-bottom: 24px;
	}

	.form-card h2 {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 16px;
	}

	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
		margin-bottom: 16px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.field span {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-on-surface);
		opacity: 0.7;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.field input,
	.field textarea {
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 10px 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.field input:focus,
	.field textarea:focus {
		border-color: var(--color-primary);
	}

	.field textarea { resize: vertical; }
	.full-width { margin-bottom: 16px; }

	.order-items-section {
		margin-bottom: 16px;
	}

	.order-items-section h3 {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 10px;
	}

	.item-row {
		display: flex;
		gap: 12px;
		align-items: center;
		padding: 6px 0;
		border-bottom: 1px solid var(--color-outline-variant);
		font-size: 0.875rem;
		color: var(--color-on-surface);
	}

	.item-row span:first-child { flex: 1; }

	.add-item-row {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-top: 10px;
	}

	.add-item-dropdown { flex: 2; }
	.add-item-row input { flex: 1; }

	.add-item-row input {
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 8px 10px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: 12px;
		margin-top: 8px;
	}

	.tabs {
		display: flex;
		gap: 4px;
		margin-bottom: 20px;
		background: var(--color-surface-container);
		padding: 4px;
		border-radius: 10px;
		border: 1px solid var(--color-outline-variant);
		width: fit-content;
	}

	.tab {
		background: none;
		border: none;
		padding: 6px 16px;
		border-radius: 8px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s, color 0.15s;
		opacity: 0.6;
	}

	.tab.active {
		background: var(--color-surface-container-high);
		opacity: 1;
		color: var(--color-primary);
		font-weight: 600;
	}

	.empty {
		color: var(--color-on-surface);
		opacity: 0.5;
		text-align: center;
		padding: 40px 0;
	}

	.order-list { display: flex; flex-direction: column; gap: 12px; }

	.order-card {
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
		padding: 16px 20px;
	}

	.order-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 12px;
	}

	.order-meta { display: flex; flex-direction: column; gap: 4px; }

	.customer-name {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-on-surface);
	}

	/* Deadline row with live countdown */
	.deadline-row {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 0.75rem;
		color: var(--color-outline);
		margin-bottom: 6px;
	}
	.deadline-row.overdue { color: var(--color-alert-red, #ef4444); }
	.deadline-date { font-weight: 500; color: var(--color-on-surface); opacity: 0.8; }
	.countdown { color: var(--color-primary); font-weight: 500; }
	.countdown.overdue { color: var(--color-alert-red, #ef4444); }

	/* Location chip */
	.order-location {
		display: flex; align-items: center; gap: 3px;
		font-size: 0.72rem; color: var(--color-outline);
		margin-top: 2px;
		white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 220px;
	}

	/* Detail hint */
	.detail-hint {
		font-size: 0.7rem; color: var(--color-outline); opacity: 0.6; margin-left: auto;
	}

	.order-right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.status-label {
		font-size: 0.75rem;
		color: var(--color-on-surface);
		opacity: 0.7;
	}

	.amount {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-primary);
	}

	.progress-row {
		margin: 8px 0;
		padding: 4px 0;
	}

	/* Clickable card */
	.order-card {
		cursor: pointer;
		transition: transform 0.12s var(--ease-spring), box-shadow 0.12s;
	}
	.order-card:hover {
		transform: translateY(-1px);
		box-shadow: 0 6px 20px rgba(0,0,0,0.2);
	}

	.order-notes {
		font-size: 0.8rem;
		color: var(--color-on-surface);
		opacity: 0.6;
		margin: 4px 0 0;
	}

	/* Buttons */
	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary);
		border: none;
		border-radius: 8px;
		padding: 10px 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-primary:hover { opacity: 0.85; }
	.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

	.btn-secondary {
		background: var(--color-surface-container-high);
		color: var(--color-on-surface);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 8px 14px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.btn-ghost {
		background: none;
		border: none;
		color: var(--color-on-surface);
		opacity: 0.6;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.btn-ghost:hover { opacity: 1; background: var(--color-surface-container-high); }

	/* Shortage alerts */
	.item-shortage {
		border-color: var(--color-alert-red) !important;
	}

	.shortage-alert {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		margin-bottom: 4px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--color-alert-red);
		border-radius: 6px;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-alert-red);
	}

	.shortage-icon {
		font-size: 0.9rem;
	}

	/* Order actions */
	.order-actions {
		display: flex;
		gap: 8px;
		margin-bottom: 4px;
	}

	.btn-print {
		font-size: 0.75rem;
	}

	@media (max-width: 600px) {
		.form-grid { grid-template-columns: 1fr; }
	}

	/* Print styles */
	@media print {
		.page, .tabs, .order-list, .page-header, .form-card { display: none !important; }
		:global(.dock-container), :global(.logo-watermark), :global(.sync-indicator) { display: none !important; }

		:global(.print-preorder) {
			display: block !important;
			position: fixed;
			inset: 0;
			background: white;
			color: black;
			padding: 40px;
			font-family: serif;
			font-size: 14px;
			z-index: 99999;
		}
		:global(.print-preorder h1) { font-size: 20px; margin-bottom: 16px; }
		:global(.print-preorder table) { width: 100%; border-collapse: collapse; margin: 16px 0; }
		:global(.print-preorder th),
		:global(.print-preorder td) { border: 1px solid #333; padding: 6px 10px; text-align: left; }
		:global(.print-preorder .grand-total) { font-size: 16px; margin-top: 12px; }
	}
</style>
