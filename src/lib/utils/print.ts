import type { Order, OrderItem, FlowerSort, FlowerConstants, Item } from '$lib/tauri/types';

type TranslateFn = (key: string) => string;
type GetItemsFn = (orderId: string) => Promise<OrderItem[]>;

// ────────────────────────────────────────────────────────────────
// Why an iframe, not document.body injection:
//
// Previous attempts injected the print container into the main
// document and used `@media print { body > * { display:none } }`
// to hide the app. This is fragile under SvelteKit + Tauri WebView2:
//   • `<div style="display:contents">` wraps the app and competes
//     with `display:none !important` in ways that differ across
//     engines/WebView2 versions.
//   • Svelte's component-scoped CSS can (and did) bleed into print.
//   • Fixed-positioned descendants (Dock, watermark) require extra
//     cascading rules.
//
// A hidden iframe with a self-contained HTML document eliminates
// all of these: the print renderer only sees our markup + styles,
// nothing from the host page.
// ────────────────────────────────────────────────────────────────

const PRINT_DOC_CSS = `
	*, *::before, *::after { box-sizing: border-box; }

	@page { margin: 14mm; }

	html, body {
		margin: 0;
		padding: 0;
		background: #ffffff;
		color: #111;
		font-family: 'Segoe UI', 'Helvetica Neue', Arial, sans-serif;
		font-size: 13pt;
		line-height: 1.4;
		-webkit-print-color-adjust: exact;
		print-color-adjust: exact;
	}

	.print-root { padding: 0; }

	.print-consolidated-header {
		border-bottom: 2.5px solid #111;
		padding-bottom: 8px;
		margin-bottom: 16px;
	}
	.print-consolidated-header h1 {
		margin: 0 0 4px;
		font-size: 22pt;
		font-weight: 700;
		letter-spacing: -0.01em;
	}
	.print-header-date {
		font-size: 10pt;
		color: #555;
	}

	.print-order {
		page-break-after: always;
		break-after: page;
		padding-bottom: 10px;
	}
	.print-order:last-of-type {
		page-break-after: auto;
		break-after: auto;
	}

	.print-order-header {
		margin-bottom: 14px;
		padding-bottom: 10px;
		border-bottom: 1px solid #999;
	}
	.print-customer {
		font-size: 20pt;
		font-weight: 700;
		margin: 0 0 8px;
		color: #000;
		letter-spacing: -0.01em;
	}
	.print-meta {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 4px 32px;
		margin: 0;
		font-size: 11pt;
	}
	.print-meta > div { display: flex; gap: 8px; }
	.print-meta dt { font-weight: 600; color: #444; margin: 0; }
	.print-meta dd { margin: 0; color: #111; }

	table.print-items {
		width: 100%;
		border-collapse: collapse;
		margin: 10px 0;
		font-size: 12pt;
	}
	table.print-items thead th {
		background: #e8e8e8;
		color: #000;
		font-weight: 700;
		text-align: left;
		padding: 9px 10px;
		border: 1px solid #777;
	}
	table.print-items tbody td {
		padding: 8px 10px;
		border: 1px solid #bbb;
		vertical-align: top;
	}
	table.print-items tbody tr:nth-child(even) td { background: #f6f6f6; }

	.c-num   { width: 28px; text-align: center; }
	.c-qty   { width: 70px; text-align: center; }
	.c-price,
	.c-total { text-align: right; white-space: nowrap; }
	.c-name  { font-weight: 600; }
	.c-total { font-weight: 700; }

	.print-order-subtotal {
		display: flex;
		justify-content: flex-end;
		align-items: baseline;
		gap: 14px;
		margin-top: 6px;
		padding: 8px 12px;
		border-top: 1px solid #999;
		font-size: 12pt;
	}
	.print-order-subtotal strong {
		font-weight: 700;
		min-width: 130px;
		text-align: right;
		font-size: 13pt;
	}

	.print-totals {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 10px;
		padding: 10px 14px;
		border-top: 2.5px solid #111;
		font-size: 15pt;
		font-weight: 700;
	}
	.tot-label { letter-spacing: 0.02em; }
	.tot-val   { min-width: 150px; text-align: right; }

	.print-notes {
		margin-top: 14px;
		padding: 10px 14px;
		background: #f3f3f3;
		border-left: 3px solid #777;
		font-size: 11pt;
		font-style: italic;
	}

	.print-grand-summary {
		margin-top: 18px;
		padding: 14px 16px;
		border: 2.5px solid #111;
		background: #fafafa;
	}
	.print-grand-summary h2 {
		margin: 0 0 10px;
		font-size: 16pt;
		font-weight: 700;
	}
	.print-grand-summary .grand-row {
		display: flex;
		justify-content: space-between;
		padding: 5px 0;
		font-size: 13pt;
	}
	.print-grand-summary .grand-row strong { font-weight: 700; }

	.print-breakdown { margin-top: 12px; }
	.print-breakdown thead th { font-size: 11pt; }
	.print-breakdown tbody td { font-size: 11pt; }

	.print-footer {
		margin-top: 16px;
		padding-top: 8px;
		border-top: 1px solid #bbb;
		font-size: 9pt;
		color: #666;
		text-align: right;
	}

	/* ── Registry (single-table across all orders) ─────────────── */
	.print-registry-header {
		margin-bottom: 12px;
		padding-bottom: 6px;
		border-bottom: 2.5px solid #111;
	}
	.print-registry-header h1 {
		margin: 0 0 4px;
		font-size: 20pt;
		font-weight: 700;
		letter-spacing: -0.01em;
	}
	.print-registry-range {
		font-size: 10pt;
		color: #555;
	}

	table.print-registry {
		width: 100%;
		border-collapse: collapse;
		font-size: 10pt;
		margin-top: 6px;
	}
	/* repeat header on every printed page */
	table.print-registry thead { display: table-header-group; }
	table.print-registry tfoot { display: table-footer-group; }
	table.print-registry thead th {
		background: #e8e8e8;
		color: #000;
		font-weight: 700;
		text-align: left;
		padding: 6px 8px;
		border: 1px solid #777;
		white-space: nowrap;
	}
	table.print-registry tbody td {
		padding: 5px 8px;
		border: 1px solid #bbb;
		vertical-align: top;
		word-break: break-word;
	}
	table.print-registry tbody tr { page-break-inside: avoid; break-inside: avoid; }
	table.print-registry tr.order-subtotal-row td {
		background: #f0f0f0;
		font-weight: 700;
		text-align: right;
		border-top: 1.5px solid #555;
	}
	table.print-registry .reg-num   { width: 28px; text-align: center; }
	table.print-registry .reg-date  { width: 88px; white-space: nowrap; }
	table.print-registry .reg-cust  { min-width: 140px; font-weight: 600; }
	table.print-registry .reg-sort  { min-width: 120px; }
	table.print-registry .reg-qty   { width: 56px; text-align: center; }
	table.print-registry .reg-price { width: 88px; text-align: right; white-space: nowrap; }
	table.print-registry .reg-total { width: 96px; text-align: right; font-weight: 700; white-space: nowrap; }
	table.print-registry .reg-status { width: 92px; white-space: nowrap; }
	table.print-registry .reg-empty  { color: #888; font-style: italic; text-align: center; }

	.print-registry-footer {
		margin-top: 10px;
		padding: 10px 14px;
		border-top: 2.5px solid #111;
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		font-size: 12pt;
	}
	.print-registry-footer .grand {
		font-size: 14pt;
		font-weight: 700;
	}
`;

function escapeHtml(s: string): string {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#39;');
}

function formatDateTime(iso?: string): string {
	if (!iso) return '';
	try {
		return new Date(iso).toLocaleString('ru-RU', {
			year: 'numeric',
			month: '2-digit',
			day: '2-digit',
			hour: '2-digit',
			minute: '2-digit',
		});
	} catch {
		return iso;
	}
}

function resolveItemName(
	item: OrderItem,
	flowerSorts: FlowerSort[],
	inventoryItems: Item[]
): { name: string; variety?: string } {
	// Prefer the explicit sort_id link (reliable since migration 014).
	// Fall back to item_id for legacy data where the link was never written.
	const sort = flowerSorts.find((s) => s.id === item.sort_id || s.id === item.item_id);
	if (sort) return { name: sort.name, variety: sort.variety };
	const inv = inventoryItems.find((i) => i.id === item.item_id);
	return { name: inv?.name ?? item.item_id };
}

interface LineCalc {
	packCount: number;
	stemsPerPack: number;
	pricePerPack: number;
	pricePerStem: number;
	lineTotal: number;
}

function computeLine(
	item: OrderItem,
	sort: FlowerSort | undefined,
	constants: FlowerConstants
): LineCalc {
	const fallbackStems =
		sort?.flowers_per_pack_override ?? constants.flowers_per_pack ?? 1;
	const stemsPerPack =
		item.stems_per_pack && item.stems_per_pack > 0 ? item.stems_per_pack : fallbackStems;
	const packCount =
		item.pack_count && item.pack_count > 0 ? item.pack_count : item.quantity;

	// unit_price is "price per pack" (AddItemModal stores pricePerPack).
	// Fallback: derive from sort.sell_price_stem * stemsPerPack when missing.
	let pricePerPack = item.unit_price;
	if ((!pricePerPack || pricePerPack <= 0) && sort) {
		pricePerPack = (sort.sell_price_stem ?? 0) * stemsPerPack;
	}
	const pricePerStem = stemsPerPack > 0 ? pricePerPack / stemsPerPack : 0;
	const lineTotal = packCount * pricePerPack;
	return { packCount, stemsPerPack, pricePerPack, pricePerStem, lineTotal };
}

function formatMoney(value: number, currencyCode: string): string {
	try {
		return new Intl.NumberFormat('ru-RU', {
			style: 'currency',
			currency: currencyCode,
			maximumFractionDigits: 2,
		}).format(value);
	} catch {
		return `${value.toFixed(2)} ${currencyCode}`;
	}
}

function renderOrderHeader(order: Order, t: TranslateFn): string {
	const deadline = order.deadline ? formatDateTime(order.deadline) : '';
	const created = formatDateTime(order.created_at);
	return `
		<div class="print-order-header">
			<h2 class="print-customer">${escapeHtml(order.customer_name)}</h2>
			<dl class="print-meta">
				${order.customer_phone ? `<div><dt>${t('label_customer_phone')}:</dt><dd>${escapeHtml(order.customer_phone)}</dd></div>` : ''}
				${order.customer_email ? `<div><dt>${t('label_customer_email')}:</dt><dd>${escapeHtml(order.customer_email)}</dd></div>` : ''}
				${order.customer_company ? `<div><dt>${t('order_customer_company')}:</dt><dd>${escapeHtml(order.customer_company)}</dd></div>` : ''}
				${order.delivery_address ? `<div><dt>${t('order_delivery_address')}:</dt><dd>${escapeHtml(order.delivery_address)}</dd></div>` : ''}
				${deadline ? `<div><dt>${t('label_deadline')}:</dt><dd>${escapeHtml(deadline)}</dd></div>` : ''}
				${created ? `<div><dt>${t('label_created_at') || 'Создан'}:</dt><dd>${escapeHtml(created)}</dd></div>` : ''}
			</dl>
		</div>
	`;
}

function renderItemsTable(
	items: OrderItem[],
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	currencyCode: string,
	t: TranslateFn
): { html: string; subtotal: number } {
	let subtotal = 0;
	const rows = items
		.map((it, idx) => {
			const sort = flowerSorts.find((s) => s.id === it.sort_id || s.id === it.item_id);
			const { name, variety } = resolveItemName(it, flowerSorts, inventoryItems);
			const calc = computeLine(it, sort, constants);
			subtotal += calc.lineTotal;
			const productLabel = variety
				? `${escapeHtml(name)} — ${escapeHtml(variety)}`
				: escapeHtml(name);
			return `
				<tr>
					<td class="c-num">${idx + 1}</td>
					<td class="c-name">${productLabel}</td>
					<td class="c-qty">${calc.packCount}</td>
					<td class="c-qty">${calc.stemsPerPack}</td>
					<td class="c-price">${formatMoney(calc.pricePerStem, currencyCode)}</td>
					<td class="c-price">${formatMoney(calc.pricePerPack, currencyCode)}</td>
					<td class="c-total">${formatMoney(calc.lineTotal, currencyCode)}</td>
				</tr>
			`;
		})
		.join('');

	const emptyRow =
		items.length === 0
			? `<tr><td colspan="7" style="text-align:center;color:#888;padding:18px;">—</td></tr>`
			: '';

	const html = `
		<table class="print-items">
			<thead>
				<tr>
					<th class="c-num">#</th>
					<th class="c-name">${t('label_product')}</th>
					<th class="c-qty">${t('label_pack_count')}</th>
					<th class="c-qty">${t('label_stems_per_pack')}</th>
					<th class="c-price">${t('label_price_per_stem')}</th>
					<th class="c-price">${t('label_price_per_pack')}</th>
					<th class="c-total">${t('print_summary')}</th>
				</tr>
			</thead>
			<tbody>${rows}${emptyRow}</tbody>
		</table>
	`;

	return { html, subtotal };
}

// ────────────────────────────────────────────────────────────────
// Iframe-based print driver.
// Builds a self-contained document, loads it into a hidden iframe,
// calls print() on the iframe's window, and cleans up afterwards.
// ────────────────────────────────────────────────────────────────
function buildPrintDocument(title: string, bodyHtml: string): string {
	return `<!doctype html>
<html lang="ru">
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
<style>${PRINT_DOC_CSS}</style>
</head>
<body>
<div class="print-root">${bodyHtml}</div>
</body>
</html>`;
}

function printViaIframe(title: string, bodyHtml: string): void {
	// Remove a stale iframe from a previous call if it's still around.
	const stale = document.getElementById('exsul-print-frame');
	if (stale?.parentNode) stale.parentNode.removeChild(stale);

	const frame = document.createElement('iframe');
	frame.id = 'exsul-print-frame';
	// Position off-screen but keep it rendered (display:none disables printing).
	frame.style.position = 'fixed';
	frame.style.right = '0';
	frame.style.bottom = '0';
	frame.style.width = '0';
	frame.style.height = '0';
	frame.style.border = '0';
	frame.style.visibility = 'hidden';
	frame.setAttribute('aria-hidden', 'true');
	frame.setAttribute('tabindex', '-1');

	let cleaned = false;
	const cleanup = () => {
		if (cleaned) return;
		cleaned = true;
		if (frame.parentNode) frame.parentNode.removeChild(frame);
	};

	const doPrint = () => {
		const win = frame.contentWindow;
		if (!win) {
			cleanup();
			return;
		}
		try {
			win.focus();
			// afterprint fires in the iframe's window, not the outer.
			win.addEventListener('afterprint', () => setTimeout(cleanup, 50), { once: true });
			win.print();
		} catch (e) {
			console.error('Print failed:', e);
			cleanup();
			return;
		}
		// Safety net — some Tauri/WebView2 builds never fire afterprint.
		setTimeout(cleanup, 15_000);
	};

	frame.addEventListener('load', () => {
		// Give the WebView one paint tick to finalize layout inside the iframe.
		setTimeout(doPrint, 50);
	});

	document.body.appendChild(frame);

	const doc = buildPrintDocument(title, bodyHtml);
	// Prefer direct document.write — in WebView2 this is more reliable than
	// srcdoc for complex documents and fires the load event predictably.
	const cw = frame.contentWindow;
	if (!cw) {
		cleanup();
		return;
	}
	const cd = cw.document;
	cd.open();
	cd.write(doc);
	cd.close();
}

export function printSingleOrder(
	order: Order,
	items: OrderItem[],
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	currencyCode: string,
	t: TranslateFn
): void {
	const header = renderOrderHeader(order, t);
	const { html, subtotal } = renderItemsTable(
		items,
		flowerSorts,
		inventoryItems,
		constants,
		currencyCode,
		t
	);
	const totalAmount = order.total_amount > 0 ? order.total_amount : subtotal;

	const body = `
		${header}
		${html}
		<div class="print-totals">
			<span class="tot-label">${t('print_summary')}</span>
			<span class="tot-val">${formatMoney(totalAmount, currencyCode)}</span>
		</div>
		${order.notes ? `<div class="print-notes"><strong>${t('label_notes')}:</strong> ${escapeHtml(order.notes)}</div>` : ''}
		<div class="print-footer">${t('print_date')}: ${formatDateTime(new Date().toISOString())}</div>
	`;

	const title = `${t('action_print_preorder')} — ${order.customer_name}`;
	printViaIframe(title, body);
}

export async function printAllOrders(
	ordersList: Order[],
	getItems: GetItemsFn,
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	currencyCode: string,
	t: TranslateFn
): Promise<void> {
	const sections: string[] = [];
	let grandTotal = 0;
	const sortBreakdown = new Map<
		string,
		{ packs: number; stems: number; total: number; name: string }
	>();

	for (let i = 0; i < ordersList.length; i++) {
		const order = ordersList[i];
		const items = await getItems(order.id);
		const header = renderOrderHeader(order, t);
		const { html, subtotal } = renderItemsTable(
			items,
			flowerSorts,
			inventoryItems,
			constants,
			currencyCode,
			t
		);
		const orderTotal = order.total_amount > 0 ? order.total_amount : subtotal;
		grandTotal += orderTotal;

		for (const it of items) {
			const sort = flowerSorts.find((s) => s.id === it.sort_id || s.id === it.item_id);
			const calc = computeLine(it, sort, constants);
			const key = it.sort_id ?? it.item_id;
			const label = sort
				? sort.variety
					? `${sort.name} — ${sort.variety}`
					: sort.name
				: key;
			const existing = sortBreakdown.get(key) ?? {
				packs: 0,
				stems: 0,
				total: 0,
				name: label,
			};
			existing.packs += calc.packCount;
			existing.stems += calc.packCount * calc.stemsPerPack;
			existing.total += calc.lineTotal;
			sortBreakdown.set(key, existing);
		}

		sections.push(`
			<section class="print-order">
				${header}
				${html}
				<div class="print-order-subtotal">
					<span>${t('print_summary')}:</span>
					<strong>${formatMoney(orderTotal, currencyCode)}</strong>
				</div>
			</section>
		`);
	}

	const breakdownRows = Array.from(sortBreakdown.values())
		.sort((a, b) => b.total - a.total)
		.map(
			(b) => `
				<tr>
					<td class="c-name">${escapeHtml(b.name)}</td>
					<td class="c-qty">${b.packs}</td>
					<td class="c-qty">${b.stems}</td>
					<td class="c-total">${formatMoney(b.total, currencyCode)}</td>
				</tr>
			`
		)
		.join('');

	const summary = `
		<section class="print-grand-summary">
			<h2>${t('print_consolidated_title')}</h2>
			<div class="grand-row">
				<span>${t('print_total_orders')}:</span>
				<strong>${ordersList.length}</strong>
			</div>
			<div class="grand-row">
				<span>${t('print_grand_total')}:</span>
				<strong>${formatMoney(grandTotal, currencyCode)}</strong>
			</div>
			${
				breakdownRows
					? `<table class="print-items print-breakdown">
							<thead>
								<tr>
									<th class="c-name">${t('label_product')}</th>
									<th class="c-qty">${t('label_pack_count')}</th>
									<th class="c-qty">${t('flowers_total_raw')}</th>
									<th class="c-total">${t('print_summary')}</th>
								</tr>
							</thead>
							<tbody>${breakdownRows}</tbody>
						</table>`
					: ''
			}
		</section>
	`;

	const footer = `<div class="print-footer">${t('print_date')}: ${formatDateTime(
		new Date().toISOString()
	)}</div>`;

	const top = `
		<div class="print-consolidated-header">
			<h1>${t('print_consolidated_title')}</h1>
			<div class="print-header-date">${t('print_date')}: ${formatDateTime(new Date().toISOString())}</div>
		</div>
	`;

	const body = top + sections.join('') + summary + footer;
	printViaIframe(t('print_consolidated_title'), body);
}

// ────────────────────────────────────────────────────────────────
// Registry print — one unified table across all selected orders.
//
// Unlike `printAllOrders` (which renders each order as its own
// section with a forced page-break), this emits a single HTML table
// where each row is an order-item. The "date / customer / status"
// cells span all rows for a given order (rowspan = items.length).
// The thead/tfoot use `display: table-*-group` so they repeat on
// every printed page — critical for readability in long registries.
// ────────────────────────────────────────────────────────────────
function formatDateShort(iso?: string): string {
	if (!iso) return '';
	try {
		return new Date(iso).toLocaleDateString('ru-RU', {
			year: '2-digit',
			month: '2-digit',
			day: '2-digit',
		});
	} catch {
		return iso;
	}
}

function translateStatus(status: string, t: TranslateFn): string {
	const key = `status_${status}`;
	const translated = t(key);
	// Fall back to the raw status if translation key is missing.
	return translated === key ? status : translated;
}

export async function printOrdersRegistry(
	ordersList: Order[],
	getItems: GetItemsFn,
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	currencyCode: string,
	t: TranslateFn,
	range?: { from: string; to: string }
): Promise<void> {
	// Pre-fetch all items in parallel, preserving input order.
	const pairs = await Promise.all(
		ordersList.map(async (o) => ({ order: o, items: await getItems(o.id) }))
	);

	const rows: string[] = [];
	let grandTotal = 0;
	let rowIdx = 0;

	for (const { order, items } of pairs) {
		const dateShort = escapeHtml(formatDateShort(order.created_at));
		const customer = escapeHtml(order.customer_name || '—');
		const statusLabel = escapeHtml(translateStatus(order.status, t));
		let orderSubtotal = 0;

		if (items.length === 0) {
			rowIdx++;
			rows.push(`
				<tr>
					<td class="reg-num">${rowIdx}</td>
					<td class="reg-date">${dateShort}</td>
					<td class="reg-cust">${customer}</td>
					<td class="reg-sort reg-empty">—</td>
					<td class="reg-qty">—</td>
					<td class="reg-qty">—</td>
					<td class="reg-price">—</td>
					<td class="reg-total">${formatMoney(0, currencyCode)}</td>
					<td class="reg-status">${statusLabel}</td>
				</tr>
			`);
			continue;
		}

		items.forEach((it, i) => {
			rowIdx++;
			const sort = flowerSorts.find((s) => s.id === it.sort_id || s.id === it.item_id);
			const { name, variety } = resolveItemName(it, flowerSorts, inventoryItems);
			const calc = computeLine(it, sort, constants);
			orderSubtotal += calc.lineTotal;
			const sortLabel = variety
				? `${escapeHtml(name)} — ${escapeHtml(variety)}`
				: escapeHtml(name);
			const isFirst = i === 0;
			const rowspan = items.length;
			rows.push(`
				<tr>
					<td class="reg-num">${rowIdx}</td>
					${isFirst ? `<td class="reg-date" rowspan="${rowspan}">${dateShort}</td>` : ''}
					${isFirst ? `<td class="reg-cust" rowspan="${rowspan}">${customer}</td>` : ''}
					<td class="reg-sort">${sortLabel}</td>
					<td class="reg-qty">${calc.packCount}</td>
					<td class="reg-qty">${calc.stemsPerPack}</td>
					<td class="reg-price">${formatMoney(calc.pricePerPack, currencyCode)}</td>
					<td class="reg-total">${formatMoney(calc.lineTotal, currencyCode)}</td>
					${isFirst ? `<td class="reg-status" rowspan="${rowspan}">${statusLabel}</td>` : ''}
				</tr>
			`);
		});

		const finalTotal = order.total_amount > 0 ? order.total_amount : orderSubtotal;
		grandTotal += finalTotal;
		rows.push(`
			<tr class="order-subtotal-row">
				<td colspan="7">${escapeHtml(t('print_summary'))}:</td>
				<td class="reg-total">${formatMoney(finalTotal, currencyCode)}</td>
				<td></td>
			</tr>
		`);
	}

	const rangeLine =
		range && (range.from || range.to)
			? `<div class="print-registry-range">${escapeHtml(t('print_registry_period'))}: ${escapeHtml(range.from)} — ${escapeHtml(range.to)}</div>`
			: '';

	const header = `
		<div class="print-registry-header">
			<h1>${escapeHtml(t('print_registry_title'))}</h1>
			${rangeLine}
			<div class="print-header-date">${escapeHtml(t('print_date'))}: ${escapeHtml(formatDateTime(new Date().toISOString()))}</div>
		</div>
	`;

	const tableHtml = `
		<table class="print-registry">
			<thead>
				<tr>
					<th class="reg-num">#</th>
					<th class="reg-date">${escapeHtml(t('label_order_date'))}</th>
					<th class="reg-cust">${escapeHtml(t('label_customer_name'))}</th>
					<th class="reg-sort">${escapeHtml(t('label_sort_col'))}</th>
					<th class="reg-qty">${escapeHtml(t('label_pack_count'))}</th>
					<th class="reg-qty">${escapeHtml(t('label_stems_per_pack'))}</th>
					<th class="reg-price">${escapeHtml(t('label_price_per_pack'))}</th>
					<th class="reg-total">${escapeHtml(t('print_summary'))}</th>
					<th class="reg-status">${escapeHtml(t('label_status_col'))}</th>
				</tr>
			</thead>
			<tbody>
				${
					rows.length === 0
						? `<tr><td colspan="9" class="reg-empty" style="padding:24px;">${escapeHtml(t('print_dialog_no_orders'))}</td></tr>`
						: rows.join('')
				}
			</tbody>
		</table>
	`;

	const totalsFooter = `
		<div class="print-registry-footer">
			<span>${escapeHtml(t('print_total_orders'))}: <strong>${ordersList.length}</strong></span>
			<span class="grand">${escapeHtml(t('print_registry_grand_total'))}: ${formatMoney(grandTotal, currencyCode)}</span>
		</div>
	`;

	const body = header + tableHtml + totalsFooter;
	printViaIframe(t('print_registry_title'), body);
}
