<script lang="ts">
	import { orders } from '$lib/stores/orders';
	import { inventory } from '$lib/stores/inventory';
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { contacts } from '$lib/stores/contacts';
	import { preset } from '$lib/stores/preset';
	import { t } from '$lib/stores/i18n';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { commands } from '$lib/tauri/commands';
	import { printSingleOrder, printOrdersRegistry, printContactsMatrix } from '$lib/utils/print';
	import OrderProgressBar from '$lib/components/orders/OrderProgressBar.svelte';
	import OrderDetailModal from '$lib/components/orders/OrderDetailModal.svelte';
	import AddItemModal from '$lib/components/orders/AddItemModal.svelte';
	import PrintAllOrdersDialog from '$lib/components/orders/PrintAllOrdersDialog.svelte';
	import ContactCard from '$lib/components/contacts/ContactCard.svelte';
	import ContactDetailModal from '$lib/components/contacts/ContactDetailModal.svelte';
	import ContactPicker from '$lib/components/contacts/ContactPicker.svelte';
	import { formatCountdown } from '$lib/utils/countdown';
	import type {
		Contact,
		CreateOrderPayload,
		AddOrderItemPayload,
		OrderStatus,
		Order,
		PackAssignment,
		PackagingLogEntry,
	} from '$lib/tauri/types';

	const isFlowers = $derived($preset === 'flowers');

	$effect(() => {
		orders.load();
		if (isFlowers) {
			flowerSorts.load();
			flowerConstants.load();
			contacts.load();
		}
	});

	type FilterTab = 'all' | OrderStatus;
	type GroupMode = 'status' | 'contact';
	let activeTab = $state<FilterTab>('all');
	let groupMode = $state<GroupMode>('status');
	let showForm = $state(false);
	let showAddItem = $state(false);
	let detailOrder = $state<Order | null>(null);
	let showPrintDialog = $state(false);
	let printMode = $state<'registry' | 'matrix'>('registry');
	let earliestDate = $state<string | null>(null);

	// Phase E4 — selected contact filter (left panel → right panel).
	let selectedContactId = $state<string>('');
	let contactDetailId = $state<string>('');
	let contactDetailAutoEdit = $state(false);
	let contactSearch = $state('');
	// Debounce contact search so every keystroke isn't a DB hit.
	let searchTimer: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		const q = contactSearch;
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => contacts.load(q || undefined), 220);
	});

	// Picker state for the create-order form (selected contact or free name).
	let selectedContact = $state<Contact | null>(null);
	let selectedLocationId = $state<string>('');

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
		card_color: '',
	});
	let deliveryAddress = $state('');

	// Inline order items being added while creating order
	let pendingItems = $state<Omit<AddOrderItemPayload, 'order_id'>[]>([]);

	// Filter pipeline: status tab → contact filter (Phase E4)
	let filteredOrders = $derived.by(() => {
		let list = activeTab === 'all' ? $orders : $orders.filter((o) => o.status === activeTab);
		if (selectedContactId) list = list.filter((o) => o.contact_id === selectedContactId);
		return list;
	});

	// Phase E8 — "By contact" grouping mode
	type ContactGroup = { contact: Contact | null; orders: Order[]; total: number };
	let groupedByContact = $derived.by<ContactGroup[]>(() => {
		if (groupMode !== 'contact') return [];
		const map = new Map<string, ContactGroup>();
		const unknownKey = '__none__';
		for (const o of filteredOrders) {
			const key = o.contact_id ?? unknownKey;
			if (!map.has(key)) {
				const c = o.contact_id ? $contacts.find((x) => x.id === o.contact_id) ?? null : null;
				map.set(key, { contact: c, orders: [], total: 0 });
			}
			const g = map.get(key)!;
			g.orders.push(o);
			g.total += o.total_amount;
		}
		return Array.from(map.values()).sort((a, b) => b.total - a.total);
	});

	// Resolved selected contact (for the right-panel header)
	let selectedContactRecord = $derived(
		selectedContactId ? $contacts.find((c) => c.id === selectedContactId) ?? null : null
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
			const sort = $flowerSorts.find((s) => s.id === item.sort_id || s.id === item.item_id);
			if (sort && item.quantity > sort.pkg_stock) {
				map.set(i, item.quantity - sort.pkg_stock);
			}
		}
		return map;
	});

	function handleAddItem(item: {
		item_id: string;
		sort_id?: string;
		quantity: number;
		unit_price: number;
		pack_count: number;
		stems_per_pack: number;
	}) {
		pendingItems = [...pendingItems, item];
		showAddItem = false;
	}

	function removePendingItem(index: number) {
		pendingItems = pendingItems.filter((_, i) => i !== index);
	}

	async function handleSubmit() {
		// Prefer the picker's selected contact over the free-form name.
		const name = selectedContact
			? selectedContact.name + (selectedContact.surname ? ` ${selectedContact.surname}` : '')
			: formData.customer_name;
		if (!name.trim()) return;
		const orderId = await orders.create({
			customer_name: name.trim(),
			customer_email:
				(selectedContact?.email ?? formData.customer_email) || undefined,
			customer_phone:
				(selectedContact?.phone ?? formData.customer_phone) || undefined,
			deadline: formData.deadline || undefined,
			notes: formData.notes || undefined,
			card_color: formData.card_color || undefined,
			contact_id: selectedContact?.id,
			contact_location_id: selectedLocationId || undefined,
		});

		// Combined update: delivery address + total pack_count_ordered (for flowers).
		// One call instead of two — keeps the event log clean and the modal's
		// "Заказано упаковок" block visible when items were added.
		const addr = deliveryAddress.trim();
		const totalPacks = isFlowers
			? pendingItems.reduce((s, it) => s + (it.pack_count ?? 0), 0)
			: 0;
		if (addr || totalPacks > 0) {
			await commands.updateOrderExtended(
				orderId,
				undefined,
				addr || undefined,
				undefined,
				totalPacks > 0 ? totalPacks : undefined
			);
		}

		for (const item of pendingItems) {
			await orders.addItem({ ...item, order_id: orderId });
			// For flowers, mirror the PackModal flow: create a pack_assignment so
			// the warehouse↔order chain is complete (Reserved column in print,
			// "Linked packs" block in the modal). Failure is non-fatal — the
			// order_item is already saved and the order remains usable.
			if (isFlowers && item.sort_id && item.pack_count && item.stems_per_pack) {
				try {
					await commands.createPackAssignment({
						sort_id: item.sort_id,
						order_id: orderId,
						pack_count: item.pack_count,
						stems_per_pack: item.stems_per_pack,
					});
				} catch (e) {
					console.warn('Failed to create pack assignment for order item:', e);
				}
			}
		}

		formData = { customer_name: '', customer_email: '', customer_phone: '', deadline: '', notes: '', card_color: '' };
		deliveryAddress = '';
		pendingItems = [];
		selectedContact = null;
		selectedLocationId = '';
		showForm = false;
	}

	// Resolve picker changes: when a contact is selected, prefill the
	// delivery-address from its default location so the form doesn't feel empty.
	async function handlePickContact(c: Contact) {
		selectedContact = c;
		formData.customer_name = c.name + (c.surname ? ` ${c.surname}` : '');
		formData.customer_email = c.email ?? '';
		formData.customer_phone = c.phone ?? '';
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

	// Fire the per-contact "print all their orders" action straight from the
	// card's hover-icon. Reuses the registry renderer so the output matches
	// what the "Печать всех заказов" button produces, just pre-filtered.
	async function printContactOrders(contactId: string) {
		try {
			const contactOrders = await commands.getOrdersForContact(contactId);
			if (contactOrders.length === 0) return;

			const packAssignmentsByOrder: Record<string, PackAssignment[]> = {};
			const packagingLogByOrder: Record<string, PackagingLogEntry[]> = {};
			if (isFlowers) {
				await Promise.all(
					contactOrders.map(async (o) => {
						const [pa, pl] = await Promise.all([
							commands.getPackAssignments(o.id).catch(() => []),
							commands.getPackagingLogByOrder(o.id).catch(() => []),
						]);
						packAssignmentsByOrder[o.id] = pa;
						packagingLogByOrder[o.id] = pl;
					})
				);
			}

			await printOrdersRegistry(
				contactOrders,
				(id) => orders.getItems(id),
				$flowerSorts,
				$inventory,
				$flowerConstants,
				$globalCurrency,
				$t,
				undefined,
				{ packAssignmentsByOrder, packagingLogByOrder }
			);
		} catch (e) {
			console.error('Failed to print contact orders:', e);
			alert(String(e));
		}
	}

	async function printPreorder(order: typeof filteredOrders[0]) {
		// For flowers mode, pull both pack_assignments (reservation side) and
		// packaging_log (production-audit side) so the printer can synthesize
		// a full row set even when order_items is empty/incomplete.
		const [items, packAssignments, packagingLog] = await Promise.all([
			orders.getItems(order.id),
			isFlowers
				? commands.getPackAssignments(order.id).catch((e) => {
						console.warn('Failed to load pack assignments for print:', e);
						return [];
					})
				: Promise.resolve(undefined),
			isFlowers
				? commands.getPackagingLogByOrder(order.id).catch((e) => {
						console.warn('Failed to load packaging log for print:', e);
						return [];
					})
				: Promise.resolve([]),
		]);
		printSingleOrder(
			order,
			items,
			$flowerSorts,
			$inventory,
			$flowerConstants,
			$globalCurrency,
			$t,
			{ packAssignments, packagingLog }
		);
	}

	async function handleDeleteOrder(order: Order) {
		const prompt =
			order.status === 'completed'
				? $t('confirm_delete_completed_order')
				: $t('confirm_delete_order').replace('{name}', order.customer_name);
		if (!confirm(prompt)) return;
		try {
			await orders.remove(order.id);
			if (detailOrder?.id === order.id) detailOrder = null;
		} catch (e) {
			console.error('Failed to delete order:', e);
			alert(String(e));
		}
	}

	async function handleClearAllOrders() {
		const confirmation = prompt($t('confirm_delete_all_orders'));
		if (confirmation !== 'DELETE') return;
		try {
			await orders.removeAll();
		} catch (e) {
			console.error('Failed to clear all orders:', e);
			alert(String(e));
		}
	}

	async function openPrintDialog(mode: 'registry' | 'matrix' = 'registry') {
		// Fetch the earliest order date lazily — the dialog uses it to seed
		// the "from" field and constrain the date picker. A single silent
		// failure should not block printing: fall back to `null`, which the
		// dialog treats as "no history yet".
		printMode = mode;
		try {
			earliestDate = await orders.getEarliestDate();
		} catch (e) {
			console.warn('Failed to fetch earliest order date:', e);
			earliestDate = null;
		}
		showPrintDialog = true;
	}

	async function confirmPrintAll(range: { dateFrom: string; dateTo: string }) {
		showPrintDialog = false;
		// Filter in local timezone. `created_at` is stored as ISO-8601 (UTC),
		// but the picker yields a YYYY-MM-DD date in the user's locale — we
		// treat the range as inclusive calendar days for the local user.
		const from = new Date(range.dateFrom + 'T00:00:00').getTime();
		const to = new Date(range.dateTo + 'T23:59:59.999').getTime();
		const subset = filteredOrders.filter((o) => {
			const ts = new Date(o.created_at).getTime();
			return ts >= from && ts <= to;
		});
		if (subset.length === 0) {
			// The dialog already warned about an empty range; nothing to print.
			return;
		}

		// Pre-fetch pack_assignments and packaging_log for every order in one
		// parallel burst. The print renderer uses packaging_log to reconstruct
		// rows when order_items is empty (legacy orders), and pack_assignments
		// to fill the "Reserved" column — without both the registry prints
		// blank cells the user can't interpret.
		const packAssignmentsByOrder: Record<string, PackAssignment[]> = {};
		const packagingLogByOrder: Record<string, PackagingLogEntry[]> = {};
		if (isFlowers) {
			await Promise.all(
				subset.map(async (o) => {
					const [pa, pl] = await Promise.all([
						commands.getPackAssignments(o.id).catch((e) => {
							console.warn('Failed to load pack assignments:', e);
							return [];
						}),
						commands.getPackagingLogByOrder(o.id).catch((e) => {
							console.warn('Failed to load packaging log:', e);
							return [];
						}),
					]);
					packAssignmentsByOrder[o.id] = pa;
					packagingLogByOrder[o.id] = pl;
				})
			);
		}

		if (printMode === 'matrix') {
			// Make sure the contact directory is loaded so matrix rows resolve
			// display names (surname + company) instead of raw customer_name.
			if ($contacts.length === 0) await contacts.load();
			await printContactsMatrix(
				subset,
				(id) => orders.getItems(id),
				$flowerSorts,
				$inventory,
				$flowerConstants,
				$contacts,
				$globalCurrency,
				$t,
				{ from: range.dateFrom, to: range.dateTo },
				{ packAssignmentsByOrder, packagingLogByOrder }
			);
			return;
		}

		await printOrdersRegistry(
			subset,
			(id) => orders.getItems(id),
			$flowerSorts,
			$inventory,
			$flowerConstants,
			$globalCurrency,
			$t,
			{ from: range.dateFrom, to: range.dateTo },
			{ packAssignmentsByOrder, packagingLogByOrder }
		);
	}

	const tabs: { key: FilterTab; label: string }[] = [
		{ key: 'all', label: $t('status_all') },
		{ key: 'pending', label: $t('status_pending') },
		{ key: 'in_progress', label: $t('status_in_progress') },
		{ key: 'completed', label: $t('status_completed') },
		{ key: 'cancelled', label: $t('status_cancelled') },
	];
</script>

<div class="page" class:two-panel={isFlowers}>
	{#if isFlowers}
		<!-- Left panel: Contacts directory (Phase E4) -->
		<aside class="contacts-panel">
			<div class="contacts-header">
				<h2>{$t('contacts_panel_title')}</h2>
				<button class="btn-ghost" type="button" onclick={async () => {
				// Create a blank contact, then open the modal pre-focused on the
				// name field so the user can start typing immediately. We used to
				// pre-fill "Новый контакт" and force the user to click Edit first —
				// that turned a one-step action into four clicks.
				const id = await contacts.create({ name: '' });
				contactDetailAutoEdit = true;
				contactDetailId = id;
			}} aria-label={$t('contacts_add')} title={$t('contacts_add')}>+</button>
			</div>
			<input
				type="search"
				class="contacts-search"
				placeholder={$t('contacts_search_placeholder')}
				bind:value={contactSearch}
			/>

			{#if $contacts.length === 0 && !contactSearch}
				<p class="empty-contacts">{$t('contacts_empty')}</p>
			{:else}
				<div class="contacts-grid">
					{#each $contacts as c (c.id)}
						<ContactCard
							contact={c}
							selected={selectedContactId === c.id}
							onclick={() => {
								if (selectedContactId === c.id) {
									contactDetailId = c.id;
								} else {
									selectedContactId = c.id;
								}
							}}
							onprint={() => { void printContactOrders(c.id); }}
						/>
					{/each}
				</div>
			{/if}
		</aside>
	{/if}

<div class="main-panel">
	<div class="page-header">
		<h1>{$t('page_orders_title')}</h1>
		<div class="page-header-actions">
			<button
				class="btn-secondary print-all-btn"
				onclick={() => openPrintDialog('registry')}
				disabled={$orders.length === 0}
				title={$t('action_print_all_orders')}
				aria-label={$t('action_print_all_orders')}
			>
				<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<polyline points="6 9 6 2 18 2 18 9"/>
					<path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>
					<rect x="6" y="14" width="12" height="8"/>
				</svg>
				<span>{$t('action_print_all_orders')}</span>
			</button>
			{#if isFlowers}
				<button
					class="btn-secondary print-all-btn"
					onclick={() => openPrintDialog('matrix')}
					disabled={$orders.length === 0}
					title={$t('print_matrix_button')}
					aria-label={$t('print_matrix_button')}
				>
					<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<rect x="3" y="3" width="7" height="7"/>
						<rect x="14" y="3" width="7" height="7"/>
						<rect x="3" y="14" width="7" height="7"/>
						<rect x="14" y="14" width="7" height="7"/>
					</svg>
					<span>{$t('print_matrix_button')}</span>
				</button>
			{/if}
			<button class="btn-primary" onclick={() => (showForm = !showForm)}>
				{showForm ? $t('action_cancel') : $t('action_create_order')}
			</button>
		</div>
	</div>

	{#if isFlowers && selectedContactRecord}
		<div class="contact-filter-banner">
			<span>{$t('contacts_filter_for', { name: selectedContactRecord.name })}</span>
			<button type="button" class="btn-ghost" onclick={() => (selectedContactId = '')}>
				{$t('contacts_filter_clear')}
			</button>
		</div>
	{/if}

	{#if showForm}
		<div class="form-card">
			<h2>{$t('action_create_order')}</h2>

			{#if isFlowers}
				<div class="form-picker-row">
					<span class="picker-label">{$t('contacts_panel_title')}</span>
					<ContactPicker
						selectedId={selectedContact?.id ?? ''}
						freeName={formData.customer_name}
						onselect={(c) => { void handlePickContact(c); }}
						onclear={handleClearContact}
						onfreeName={(v) => (formData.customer_name = v)}
					/>
				</div>
			{/if}

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
				<label class="field">
					<span>Адрес доставки</span>
					<div class="location-input-wrap">
						<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" class="location-icon">
							<path d="M21 10c0 7-9 13-9 13S3 17 3 10a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/>
						</svg>
						<input type="text" bind:value={deliveryAddress} placeholder="Город, улица, дом..." />
					</div>
				</label>
				<label class="field">
					<span>{$t('label_card_color')}</span>
					<div class="color-field-wrap">
						<input
							class="color-swatch-input"
							type="color"
							value={formData.card_color || '#888888'}
							oninput={(e) => (formData.card_color = (e.currentTarget as HTMLInputElement).value)}
						/>
						<input
							type="text"
							placeholder="#rrggbb"
							bind:value={formData.card_color}
						/>
						{#if formData.card_color}
							<button
								type="button"
								class="btn-ghost color-reset"
								onclick={() => (formData.card_color = '')}
								title={$t('action_reset_to_auto')}
							>↺</button>
						{/if}
					</div>
				</label>
			</div>
			<label class="field full-width">
				<span>{$t('label_notes')}</span>
				<textarea bind:value={formData.notes} rows="3"></textarea>
			</label>

			<div class="order-items-section">
				<div class="items-header">
					<h3>{$t('action_add_order_item')}</h3>
					<button class="btn-add-item" type="button" onclick={() => (showAddItem = true)}>
						<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
						Добавить товар
					</button>
				</div>

				{#if pendingItems.length === 0}
					<p class="items-empty">Товары ещё не добавлены</p>
				{:else}
					{#each pendingItems as item, i}
						<div class="item-row" class:item-shortage={shortages.has(i)}>
							<span class="item-name">{isFlowers
								? ($flowerSorts.find(s => s.id === item.sort_id || s.id === item.item_id)?.name ?? item.item_id)
								: ($inventory.find(it => it.id === item.item_id)?.name ?? item.item_id)
							}</span>
							<span class="item-qty">x{item.quantity}</span>
							<span class="item-price">{formatAmount(item.unit_price * item.quantity, $globalCurrency)}</span>
							<button class="btn-ghost btn-remove" onclick={() => removePendingItem(i)} aria-label="Удалить">&#10005;</button>
						</div>
						{#if shortages.has(i)}
							<div class="shortage-alert">
								<span class="shortage-icon">&#9888;</span>
								<span>{$t('shortage_alert')} — {$t('shortage_deficit', { n: shortages.get(i)! })}</span>
							</div>
						{/if}
					{/each}
				{/if}
			</div>

			<div class="form-actions">
				<button class="btn-ghost" onclick={() => (showForm = false)}>{$t('action_cancel')}</button>
				<button class="btn-primary" onclick={handleSubmit} disabled={!formData.customer_name.trim()}>
					{$t('action_save')}
				</button>
			</div>
		</div>
	{/if}

	<div class="toolbar-row">
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
		{#if isFlowers}
			<div class="group-toggle">
				<button
					class="tab"
					class:active={groupMode === 'status'}
					onclick={() => (groupMode = 'status')}
				>{$t('group_by_status')}</button>
				<button
					class="tab"
					class:active={groupMode === 'contact'}
					onclick={() => (groupMode = 'contact')}
				>{$t('group_by_contact')}</button>
			</div>
		{/if}
	</div>

	{#if filteredOrders.length === 0}
		<p class="empty">{$t('empty_no_orders')}</p>
	{:else if groupMode === 'contact'}
		<div class="contact-groups">
			{#each groupedByContact as group}
				<div class="contact-group">
					<div class="group-header">
						<span class="group-name">
							{group.contact?.name ?? '—'}
						</span>
						<span class="group-count">{group.orders.length}</span>
						<span class="group-total">{formatAmount(group.total, $globalCurrency)}</span>
					</div>
					<div class="order-list">
						{#each group.orders as order (order.id)}
							<div class="order-card" role="button" tabindex="0"
								class:has-color={!!order.card_color}
								style:--card-accent={order.card_color ?? 'transparent'}
								style:--card-tint={order.card_color ? `color-mix(in srgb, ${order.card_color} 8%, transparent)` : 'transparent'}
								onclick={() => (detailOrder = order)}
								onkeydown={(e) => e.key === 'Enter' && (detailOrder = order)}
							>
								<div class="order-header">
									<div class="order-meta">
										<span class="customer-name">{order.customer_name}</span>
									</div>
									<div class="order-right">
										<span class="status-dot" style:background={statusColors[order.status]}></span>
										<span class="amount">{formatAmount(order.total_amount, $globalCurrency)}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="order-list">
			{#each filteredOrders as order (order.id)}
				<div class="order-card" role="button" tabindex="0"
					class:has-color={!!order.card_color}
					style:--card-accent={order.card_color ?? 'transparent'}
					style:--card-tint={order.card_color ? `color-mix(in srgb, ${order.card_color} 8%, transparent)` : 'transparent'}
					onclick={() => (detailOrder = order)}
					onkeydown={(e) => e.key === 'Enter' && (detailOrder = order)}
				>
					<div class="order-header">
						<div class="order-meta">
							<span class="customer-name">{order.customer_name}</span>
							{#if order.delivery_address}
								<span class="order-location">
									<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M21 10c0 7-9 13-9 13S3 17 3 10a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>
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
								<span class="countdown" class:overdue={countdowns[order.id] === 'проср��чен'}>
									{countdowns[order.id]}
								</span>
							{/if}
						</div>
					{/if}

					<!-- Progress bar -->
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
						<span class="detail-hint">{$t('hint_click_details') ?? 'Нажмите для деталей'} &#8594;</span>
					</div>

					{#if order.notes}
						<p class="order-notes">{order.notes}</p>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div><!-- /main-panel -->
</div><!-- /page -->

{#if detailOrder}
	<OrderDetailModal
		bind:order={detailOrder}
		onclose={() => (detailOrder = null)}
	/>
{/if}

{#if contactDetailId}
	<ContactDetailModal
		contactId={contactDetailId}
		autoEdit={contactDetailAutoEdit}
		onclose={() => {
			contactDetailId = '';
			contactDetailAutoEdit = false;
		}}
		onselectorder={(o) => {
			contactDetailId = '';
			contactDetailAutoEdit = false;
			detailOrder = o;
		}}
	/>
{/if}

{#if showAddItem}
	<AddItemModal
		flowerSorts={$flowerSorts}
		onconfirm={handleAddItem}
		onclose={() => (showAddItem = false)}
	/>
{/if}

{#if showPrintDialog}
	<PrintAllOrdersDialog
		{earliestDate}
		onconfirm={confirmPrintAll}
		onclose={() => (showPrintDialog = false)}
	/>
{/if}

<style>
	.page { max-width: 900px; margin: 0 auto; }

	/* Two-panel layout for flowers mode (Phase E4).
	 * Desktop: contacts left / orders right.
	 * Mobile (< 1024px): single column, contacts panel above the orders
	 * list — acceptable first-iteration tradeoff. */
	.page.two-panel {
		max-width: 1280px;
		display: grid;
		grid-template-columns: minmax(260px, 320px) 1fr;
		gap: 24px;
		align-items: start;
	}
	@media (max-width: 1023px) {
		.page.two-panel { grid-template-columns: 1fr; }
	}
	.main-panel { min-width: 0; }

	.contacts-panel {
		position: sticky;
		top: 80px;
		max-height: calc(100vh - 100px);
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px;
		background: var(--color-surface-container);
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 16px;
		backdrop-filter: blur(12px);
	}
	@media (max-width: 1023px) {
		.contacts-panel {
			position: static;
			max-height: 340px;
		}
	}
	.contacts-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0 4px;
	}
	.contacts-header h2 {
		font-size: 0.82rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-outline);
		margin: 0;
	}
	.contacts-search {
		background: var(--glass-bg, var(--color-surface-container-high));
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 8px;
		padding: 8px 12px;
		font-size: 0.82rem;
		color: var(--color-on-surface);
		outline: none;
	}
	.contacts-search:focus { border-color: var(--color-primary); }
	.contacts-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
		gap: 8px;
	}
	.empty-contacts {
		text-align: center;
		font-size: 0.82rem;
		color: var(--color-outline);
		opacity: 0.6;
		padding: 18px 6px;
		margin: 0;
	}

	.contact-filter-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 8px 14px;
		margin-bottom: 16px;
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
		border: 1px solid color-mix(in srgb, var(--color-primary) 30%, transparent);
		border-radius: 10px;
		font-size: 0.85rem;
		color: var(--color-on-surface);
	}

	.toolbar-row {
		display: flex;
		gap: 10px;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		margin-bottom: 20px;
	}
	.group-toggle {
		display: flex;
		gap: 4px;
		background: var(--color-surface-container);
		padding: 4px;
		border-radius: 10px;
		border: 1px solid var(--color-outline-variant);
	}

	.contact-groups { display: flex; flex-direction: column; gap: 18px; }
	.contact-group { display: flex; flex-direction: column; gap: 8px; }
	.group-header {
		display: flex;
		align-items: baseline;
		gap: 10px;
		padding: 6px 4px;
		border-bottom: 1px solid var(--color-outline-variant);
	}
	.group-name { font-size: 0.95rem; font-weight: 700; color: var(--color-on-surface); }
	.group-count { font-size: 0.75rem; color: var(--color-outline); }
	.group-total {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--color-primary);
		margin-left: auto;
	}

	.form-picker-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-bottom: 14px;
	}
	.form-picker-row .picker-label {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-outline);
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 24px;
		gap: 12px;
	}

	.page-header-actions {
		display: flex;
		gap: 10px;
		align-items: center;
	}

	.print-all-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.print-all-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
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

	/* Location input with icon */
	.location-input-wrap {
		position: relative;
		display: flex;
		align-items: center;
	}
	.location-input-wrap .location-icon {
		position: absolute;
		left: 10px;
		color: var(--color-primary);
		pointer-events: none;
		flex-shrink: 0;
	}
	.location-input-wrap input {
		padding-left: 32px;
		width: 100%;
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding-top: 10px;
		padding-bottom: 10px;
		padding-right: 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}
	.location-input-wrap input:focus {
		border-color: var(--color-primary);
	}

	/* Order items */
	.order-items-section {
		margin-bottom: 16px;
	}

	.items-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 10px;
	}

	.items-header h3 {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0;
	}

	.btn-add-item {
		display: flex;
		align-items: center;
		gap: 6px;
		background: var(--color-surface-container-high);
		border: 1px dashed var(--color-outline-variant);
		border-radius: 8px;
		padding: 7px 14px;
		font-size: 0.8rem;
		font-weight: 500;
		color: var(--color-primary);
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
	}
	.btn-add-item:hover {
		background: color-mix(in srgb, var(--color-primary) 8%, transparent);
		border-color: var(--color-primary);
	}

	.items-empty {
		font-size: 0.8rem;
		color: var(--color-outline);
		opacity: 0.6;
		text-align: center;
		padding: 12px 0;
		margin: 0;
	}

	.item-row {
		display: flex;
		gap: 12px;
		align-items: center;
		padding: 8px 10px;
		border-bottom: 1px solid var(--color-outline-variant);
		font-size: 0.875rem;
		color: var(--color-on-surface);
		border-radius: 6px;
		transition: background 0.1s;
	}
	.item-row:hover { background: rgba(255,255,255,0.02); }

	.item-name { flex: 1; font-weight: 500; }
	.item-qty { color: var(--color-outline); font-size: 0.8rem; }
	.item-price { font-weight: 600; color: var(--color-primary); font-size: 0.85rem; }

	.btn-remove {
		font-size: 0.75rem;
		opacity: 0.4;
		padding: 2px 6px;
	}
	.btn-remove:hover { opacity: 1; color: var(--color-alert-red, #ef4444); }

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
		border-left: 4px solid transparent;
		border-radius: 12px;
		padding: 16px 20px;
		position: relative;
	}
	.order-card.has-color {
		border-left-color: var(--card-accent);
		background:
			linear-gradient(var(--card-tint), var(--card-tint)),
			var(--color-surface-container);
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

	/* Location chip — improved */
	.order-location {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font-size: 0.78rem;
		color: var(--color-on-surface);
		margin-top: 3px;
		padding: 3px 10px 3px 6px;
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
		border-radius: 20px;
		max-width: 280px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.order-location svg { flex-shrink: 0; color: var(--color-primary); }

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
		background: var(--color-surface-container-high, var(--glass-bg));
		color: var(--color-on-surface);
		border: 1px solid var(--color-outline-variant, var(--glass-border));
		border-radius: 8px;
		padding: 10px 16px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s, background 0.15s;
	}
	.btn-secondary:hover { background: var(--color-surface-container); }
	.btn-secondary:disabled { opacity: 0.4; cursor: not-allowed; }

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

	/* Color picker on create-order form */
	.color-field-wrap {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.color-swatch-input {
		width: 40px; height: 36px;
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 0;
		cursor: pointer;
		background: transparent;
		flex-shrink: 0;
	}
	.color-field-wrap input[type="text"] {
		flex: 1;
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 9px 12px;
		color: var(--color-on-surface);
		font-family: ui-monospace, monospace;
		font-size: 0.8rem;
		outline: none;
	}
	.color-reset {
		flex-shrink: 0;
		font-size: 0.9rem;
		padding: 6px 10px;
	}

	/* Shortage alerts */
	.item-shortage {
		border-color: var(--color-alert-red) !important;
		background: rgba(239, 68, 68, 0.04) !important;
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
</style>
