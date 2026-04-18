<script lang="ts">
	import { orders } from '$lib/stores/orders';
	import { flowerSorts, flowerConstants } from '$lib/stores/flowers';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { preset } from '$lib/stores/preset';
	import { inventory } from '$lib/stores/inventory';
	import { t } from '$lib/stores/i18n';
	import { formatCountdown } from '$lib/utils/countdown';
	import { printSingleOrder } from '$lib/utils/print';
	import { computeLine, resolveItemName } from '$lib/utils/orderLine';
	import OrderProgressBar from './OrderProgressBar.svelte';
	import FlowerCard from '$lib/components/greenhouse/FlowerCard.svelte';
	import type { Order, OrderItem, FlowerSort } from '$lib/tauri/types';

	interface Props {
		order: Order;
		onclose: () => void;
	}

	let { order = $bindable(), onclose }: Props = $props();

	let orderItems = $state<OrderItem[]>([]);
	let countdown = $state('');
	let timerInterval: ReturnType<typeof setInterval> | null = null;

	$effect(() => {
		orders.getItems(order.id).then((items) => (orderItems = items));
		if ($preset === 'flowers') flowerConstants.load();

		if (order.deadline) {
			function tick() { countdown = formatCountdown(order.deadline!); }
			tick();
			timerInterval = setInterval(tick, 60_000);
		}

		return () => {
			if (timerInterval) clearInterval(timerInterval);
		};
	});

	// Find the associated flower sorts. Prefer the explicit sort_id link
	// (reliable since migration 014); fall back to item_id for legacy rows.
	const linkedSorts = $derived(() => {
		if ($preset !== 'flowers') return [];
		const result: FlowerSort[] = [];
		for (const oi of orderItems) {
			const sort = $flowerSorts.find((s) => s.id === oi.sort_id || s.id === oi.item_id);
			if (sort && !result.find(s => s.id === sort.id)) result.push(sort);
		}
		return result;
	});

	// Line-by-line breakdown — uses the same computeLine as print.ts so the
	// modal and printed output agree to the cent.
	const lines = $derived.by(() => {
		if ($preset !== 'flowers') return [];
		return orderItems.map((oi) => {
			const sort = $flowerSorts.find((s) => s.id === oi.sort_id || s.id === oi.item_id);
			const { name, variety } = resolveItemName(oi, $flowerSorts, $inventory);
			const calc = computeLine(oi, sort, $flowerConstants);
			return { oi, sort, name, variety, calc };
		});
	});

	const totals = $derived.by(() => {
		let packs = 0;
		let stems = 0;
		let sum = 0;
		for (const l of lines) {
			packs += l.calc.packCount;
			stems += l.calc.packCount * l.calc.stemsPerPack;
			sum += l.calc.lineTotal;
		}
		return { packs, stems, sum };
	});

	const totalsMismatch = $derived(
		order.total_amount > 0 && Math.abs(totals.sum - order.total_amount) > 0.01
	);

	async function handleStatusChange(newStatus: string) {
		await orders.updateStatus(order.id, newStatus);
		// Update local binding
		order = { ...order, status: newStatus as Order['status'] };
	}

	async function printPreorder() {
		printSingleOrder(
			order,
			orderItems,
			$flowerSorts,
			$inventory,
			$flowerConstants,
			$globalCurrency,
			$t
		);
	}

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}

	const statusColors: Record<string, string> = {
		pending: '#f59e0b',
		in_progress: '#3b82f6',
		completed: '#10b981',
		cancelled: '#6b7280',
	};
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdrop} role="dialog" aria-modal="true">
	<div class="modal-panel">

		<!-- Header -->
		<div class="modal-header">
			<div class="header-left">
				<span class="status-dot" style:background={statusColors[order.status]}></span>
				<h2 class="order-customer">{order.customer_name}</h2>
			</div>
			<div class="header-right">
				<button class="btn-print" type="button" onclick={printPreorder}>
					<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><polyline points="6 9 6 2 18 2 18 9"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><rect x="6" y="14" width="12" height="8"/></svg>
					Печать
				</button>
				<button class="btn-close" type="button" onclick={onclose} aria-label="Закрыть">✕</button>
			</div>
		</div>

		<div class="modal-body">

			<!-- Left: customer details + materials -->
			<div class="col-left">
				<div class="section">
					<h3 class="section-title">Клиент</h3>
					<div class="detail-grid">
						<div class="detail-row">
							<span class="detail-label">Имя</span>
							<span class="detail-val">{order.customer_name}</span>
						</div>
						{#if order.customer_phone}
							<div class="detail-row">
								<span class="detail-label">Телефон</span>
								<span class="detail-val">{order.customer_phone}</span>
							</div>
						{/if}
						{#if order.customer_email}
							<div class="detail-row">
								<span class="detail-label">Email</span>
								<span class="detail-val">{order.customer_email}</span>
							</div>
						{/if}
						{#if order.customer_company}
							<div class="detail-row">
								<span class="detail-label">Компания</span>
								<span class="detail-val">{order.customer_company}</span>
							</div>
						{/if}
						{#if order.delivery_address}
							<div class="detail-row highlight">
								<span class="detail-label">Адрес</span>
								<span class="detail-val">{order.delivery_address}</span>
							</div>
						{/if}
						{#if order.delivery_notes}
							<div class="detail-row">
								<span class="detail-label">Доставка</span>
								<span class="detail-val">{order.delivery_notes}</span>
							</div>
						{/if}
					</div>
				</div>

				{#if order.notes}
					<div class="section">
						<h3 class="section-title">Заметки</h3>
						<p class="notes-text">{order.notes}</p>
					</div>
				{/if}

				<!-- Order items — sort, packs, stems, per-stem / per-pack price, line total -->
				{#if lines.length > 0}
					<div class="section">
						<h3 class="section-title">{$t('order_items_section_title')}</h3>
						<div class="items-table-wrap">
							<table class="items-table">
								<thead>
									<tr>
										<th class="it-sort">{$t('label_sort_col')}</th>
										<th class="it-num">{$t('label_pack_count')}</th>
										<th class="it-num">{$t('label_stems_per_pack')}</th>
										<th class="it-num">{$t('label_price_per_stem')}</th>
										<th class="it-num">{$t('label_price_per_pack')}</th>
										<th class="it-num">{$t('label_line_total')}</th>
									</tr>
								</thead>
								<tbody>
									{#each lines as l (l.oi.id)}
										<tr>
											<td class="it-sort">
												<span class="it-name">{l.name}</span>
												{#if l.variety}
													<span class="it-variety">{l.variety}</span>
												{/if}
											</td>
											<td class="it-num">{l.calc.packCount}</td>
											<td class="it-num">{l.calc.stemsPerPack}</td>
											<td class="it-num">{formatAmount(l.calc.pricePerStem, $globalCurrency)}</td>
											<td class="it-num">{formatAmount(l.calc.pricePerPack, $globalCurrency)}</td>
											<td class="it-num it-line-total">{formatAmount(l.calc.lineTotal, $globalCurrency)}</td>
										</tr>
									{/each}
								</tbody>
								<tfoot>
									<tr>
										<td class="it-sort it-foot-label">{$t('print_summary')}</td>
										<td class="it-num it-foot">{totals.packs}</td>
										<td class="it-num it-foot">{totals.stems}</td>
										<td class="it-num"></td>
										<td class="it-num"></td>
										<td class="it-num it-foot it-line-total" class:mismatch={totalsMismatch}>
											{formatAmount(totals.sum, $globalCurrency)}
										</td>
									</tr>
								</tfoot>
							</table>
						</div>
					</div>
				{/if}

				<!-- Linked flower sorts -->
				{#if linkedSorts().length > 0}
					<div class="section">
						<h3 class="section-title">Сырьё</h3>
						<div class="sorts-row">
							{#each linkedSorts() as sort (sort.id)}
								<FlowerCard {sort} compact />
							{/each}
						</div>
					</div>
				{/if}
			</div>

			<!-- Right: order meta + progress -->
			<div class="col-right">
				<!-- Deadline + countdown -->
				<div class="deadline-card" class:overdue={order.deadline && new Date(order.deadline) < new Date()}>
					<div class="deadline-label">Срок сдачи</div>
					{#if order.deadline}
						<div class="deadline-date">{new Date(order.deadline).toLocaleString('ru', { day:'2-digit', month:'short', hour:'2-digit', minute:'2-digit' })}</div>
						<div class="deadline-countdown" class:overdue={countdown === 'просрочен'}>{countdown}</div>
					{:else}
						<div class="deadline-date">Не указан</div>
					{/if}
				</div>

				<!-- Amount -->
				<div class="amount-card">
					<span class="amount-label">Сумма</span>
					<span class="amount-val">{formatAmount(order.total_amount, $globalCurrency)}</span>
				</div>

				<!-- Pack counts -->
				{#if order.pack_count_ordered > 0}
					<div class="packs-card">
						<div class="pack-row">
							<span class="pack-label">Заказано упаковок</span>
							<span class="pack-val">{order.pack_count_ordered}</span>
						</div>
						{#if order.pack_count_ready > 0}
							<div class="pack-row">
								<span class="pack-label">Готово упаковок</span>
								<span class="pack-val ready">{order.pack_count_ready}</span>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Progress bar -->
				<div class="section">
					<h3 class="section-title">Прогресс</h3>
					<OrderProgressBar
						status={order.status}
						onchange={handleStatusChange}
					/>
				</div>

				<!-- Created at -->
				<div class="meta-row">
					<span class="meta-label">Создан</span>
					<span class="meta-val">{new Date(order.created_at).toLocaleString('ru')}</span>
				</div>
				{#if order.updated_at !== order.created_at}
					<div class="meta-row">
						<span class="meta-label">Обновлён</span>
						<span class="meta-val">{new Date(order.updated_at).toLocaleString('ru')}</span>
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
		background: var(--color-surface);
		border: 1px solid var(--glass-border);
		border-top-color: rgba(255,255,255,0.14);
		border-radius: 24px;
		width: 100%;
		max-width: 820px;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		box-shadow: 0 32px 80px rgba(0,0,0,0.55);
	}

	/* Header */
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 20px 24px;
		border-bottom: 1px solid var(--glass-border);
		flex-shrink: 0;
	}

	.header-left { display: flex; align-items: center; gap: 10px; }
	.header-right { display: flex; align-items: center; gap: 8px; }

	.status-dot {
		width: 10px; height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.order-customer {
		font-size: 1.1rem; font-weight: 700;
		margin: 0; color: var(--color-on-surface);
	}

	.btn-print {
		display: flex; align-items: center; gap: 6px;
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 8px; padding: 6px 12px;
		font-size: 0.8rem; cursor: pointer;
		color: var(--color-on-surface); transition: background 0.1s;
	}
	.btn-print:hover { background: var(--glass-bg-hover); }

	.btn-close {
		background: none; border: none;
		color: var(--color-outline); font-size: 1rem;
		cursor: pointer; padding: 4px 8px; border-radius: 8px;
	}
	.btn-close:hover { color: var(--color-on-surface); }

	/* Body */
	.modal-body {
		display: grid;
		grid-template-columns: 1fr 280px;
		gap: 0;
		overflow: hidden;
		flex: 1;
	}

	.col-left {
		padding: 20px 24px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 16px;
		border-right: 1px solid var(--glass-border);
	}

	.col-right {
		padding: 20px 20px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	/* Sections */
	.section { display: flex; flex-direction: column; gap: 8px; }
	.section-title {
		font-size: 0.72rem; font-weight: 600;
		text-transform: uppercase; letter-spacing: 0.05em;
		color: var(--color-outline); margin: 0;
	}

	/* Detail grid */
	.detail-grid { display: flex; flex-direction: column; gap: 6px; }
	.detail-row { display: flex; gap: 10px; align-items: baseline; }
	.detail-row.highlight { padding: 6px 10px; background: color-mix(in srgb, var(--color-primary) 8%, transparent); border-radius: 8px; border-left: 3px solid var(--color-primary); }
	.detail-label { font-size: 0.75rem; color: var(--color-outline); min-width: 70px; flex-shrink: 0; }
	.detail-val { font-size: 0.88rem; color: var(--color-on-surface); }

	.notes-text { font-size: 0.85rem; color: var(--color-outline); margin: 0; font-style: italic; line-height: 1.5; }

	/* Sorts row */
	.sorts-row { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 10px; }

	/* Items table */
	.items-table-wrap {
		overflow-x: auto;
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		background: var(--glass-bg);
	}
	.items-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.82rem;
	}
	.items-table th,
	.items-table td {
		padding: 8px 10px;
		text-align: left;
		border-bottom: 1px solid var(--glass-border);
	}
	.items-table th {
		font-size: 0.68rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-outline);
		background: color-mix(in srgb, var(--color-on-surface) 3%, transparent);
	}
	.items-table tbody tr:last-child td { border-bottom: none; }
	.items-table .it-num { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
	.items-table .it-name { display: block; color: var(--color-on-surface); font-weight: 500; }
	.items-table .it-variety { display: block; color: var(--color-outline); font-size: 0.72rem; margin-top: 2px; }
	.items-table .it-line-total { font-weight: 600; color: var(--color-primary); }
	.items-table tfoot td {
		border-top: 1.5px solid var(--glass-border);
		border-bottom: none;
		font-weight: 600;
		background: color-mix(in srgb, var(--color-primary) 5%, transparent);
	}
	.items-table .it-foot-label { text-transform: uppercase; font-size: 0.7rem; letter-spacing: 0.04em; color: var(--color-outline); }
	.items-table .it-foot { color: var(--color-on-surface); }
	.items-table .mismatch {
		color: var(--color-alert-red, #ef4444);
		text-decoration: underline dotted;
	}

	/* Right column cards */
	.deadline-card {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.deadline-card.overdue { border-color: var(--color-alert-red); background: rgba(239,68,68,0.06); }

	.deadline-label { font-size: 0.7rem; color: var(--color-outline); text-transform: uppercase; letter-spacing: 0.04em; }
	.deadline-date { font-size: 0.95rem; font-weight: 600; color: var(--color-on-surface); }
	.deadline-countdown { font-size: 0.82rem; color: var(--color-primary); font-weight: 500; }
	.deadline-countdown.overdue { color: var(--color-alert-red); }

	.amount-card {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 12px 14px;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.amount-label { font-size: 0.75rem; color: var(--color-outline); }
	.amount-val { font-size: 1.2rem; font-weight: 700; color: var(--color-primary); }

	.packs-card {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 12px 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.pack-row { display: flex; justify-content: space-between; align-items: center; }
	.pack-label { font-size: 0.75rem; color: var(--color-outline); }
	.pack-val { font-size: 1rem; font-weight: 700; color: var(--color-on-surface); }
	.pack-val.ready { color: var(--color-primary); }

	.meta-row { display: flex; justify-content: space-between; }
	.meta-label { font-size: 0.72rem; color: var(--color-outline); }
	.meta-val { font-size: 0.75rem; color: var(--color-on-surface); opacity: 0.7; }

	/* Light mode */
	:global([data-theme="light"]) .modal-panel { background: var(--color-surface, #fafafa); }
</style>
