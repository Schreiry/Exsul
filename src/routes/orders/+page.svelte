<script lang="ts">
	import { orders } from '$lib/stores/orders';
	import { inventory } from '$lib/stores/inventory';
	import { t } from '$lib/stores/i18n';
	import type { CreateOrderPayload, AddOrderItemPayload, OrderStatus } from '$lib/tauri/types';

	$effect(() => {
		orders.load();
	});

	type FilterTab = 'all' | OrderStatus;
	let activeTab = $state<FilterTab>('all');
	let showForm = $state(false);

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

	function statusToPercent(status: string): number {
		switch (status) {
			case 'pending': return 0;
			case 'in_progress': return 50;
			case 'completed': return 100;
			case 'cancelled': return 0;
			default: return 0;
		}
	}

	function percentToStatus(value: number): string {
		if (value <= 25) return 'pending';
		if (value <= 75) return 'in_progress';
		return 'completed';
	}

	function formatDeadline(deadline: string): string {
		const target = new Date(deadline).getTime();
		const now = Date.now();
		const diffDays = Math.round((target - now) / (1000 * 60 * 60 * 24));
		if (diffDays === 0) return $t('deadline_today');
		if (diffDays > 0) return $t('deadline_in_days', { n: diffDays });
		return $t('deadline_overdue', { n: Math.abs(diffDays) });
	}

	const statusColors: Record<string, string> = {
		pending: '#f59e0b',
		in_progress: '#3b82f6',
		completed: '#10b981',
		cancelled: '#6b7280',
	};

	async function handleProgressChange(orderId: string, e: Event) {
		const val = parseInt((e.currentTarget as HTMLInputElement).value, 10);
		const newStatus = percentToStatus(val);
		await orders.updateStatus(orderId, newStatus);
	}

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
					<div class="item-row">
						<span>{$inventory.find(it => it.id === item.item_id)?.name ?? item.item_id}</span>
						<span>x{item.quantity}</span>
						<span>{item.unit_price.toFixed(2)}</span>
						<button class="btn-ghost" onclick={() => removePendingItem(i)}>×</button>
					</div>
				{/each}
				<div class="add-item-row">
					<select bind:value={newItemRow.item_id}>
						<option value="">— {$t('label_name')} —</option>
						{#each $inventory as item}
							<option value={item.id}>{item.name}</option>
						{/each}
					</select>
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
				<div class="order-card">
					<div class="order-header">
						<div class="order-meta">
							<span class="customer-name">{order.customer_name}</span>
							{#if order.deadline}
								<span class="deadline" class:overdue={new Date(order.deadline) < new Date()}>
									{formatDeadline(order.deadline)}
								</span>
							{/if}
						</div>
						<div class="order-right">
							<span class="status-dot" style:background={statusColors[order.status]}></span>
							<span class="status-label">{$t('status_' + order.status)}</span>
							<span class="amount">
								{new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(order.total_amount)}
							</span>
						</div>
					</div>

					{#if order.status !== 'cancelled'}
						<div class="progress-row">
							<input
								type="range"
								min="0"
								max="100"
								value={statusToPercent(order.status)}
								oninput={(e) => handleProgressChange(order.id, e)}
								class="progress-slider"
								aria-label="{$t('label_progress')} {order.customer_name}"
							/>
						</div>
					{:else}
						<div class="cancelled-bar"></div>
					{/if}

					{#if order.notes}
						<p class="order-notes">{order.notes}</p>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

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
	.field select,
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
	.field select:focus,
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

	.add-item-row select { flex: 2; }
	.add-item-row input { flex: 1; }

	.add-item-row select,
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

	.deadline {
		font-size: 0.75rem;
		color: var(--color-primary);
		opacity: 0.8;
	}

	.deadline.overdue {
		color: #ef4444;
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
		margin-bottom: 8px;
	}

	.progress-slider {
		-webkit-appearance: none;
		appearance: none;
		width: 100%;
		height: 4px;
		border-radius: 2px;
		background: var(--color-outline-variant);
		cursor: pointer;
		outline: none;
	}

	.progress-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--color-primary);
		border: none;
		cursor: pointer;
		transition: transform 0.1s;
	}

	.progress-slider::-webkit-slider-thumb:hover {
		transform: scale(1.2);
	}

	.progress-slider::-moz-range-thumb {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--color-primary);
		border: none;
		cursor: pointer;
	}

	.cancelled-bar {
		height: 4px;
		background: var(--color-outline-variant);
		border-radius: 2px;
		opacity: 0.4;
		margin-bottom: 8px;
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

	@media (max-width: 600px) {
		.form-grid { grid-template-columns: 1fr; }
	}
</style>
