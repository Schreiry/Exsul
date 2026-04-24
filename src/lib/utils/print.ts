import type {
	Order,
	OrderItem,
	FlowerSort,
	FlowerConstants,
	Item,
	PackAssignment,
	PackagingLogEntry,
	Contact,
} from '$lib/tauri/types';
import { computeLine, resolveItemName } from '$lib/utils/orderLine';

type TranslateFn = (key: string) => string;
type GetItemsFn = (orderId: string) => Promise<OrderItem[]>;

export interface PrintSingleOptions {
	packAssignments?: PackAssignment[];
	packagingLog?: PackagingLogEntry[];
}

export interface PrintMultiOptions {
	packAssignmentsByOrder?: Record<string, PackAssignment[]>;
	packagingLogByOrder?: Record<string, PackagingLogEntry[]>;
}

// ────────────────────────────────────────────────────────────────
// Normalized print row. The print layout — both single-order and
// registry — renders one of these per line. Rows are built by
// merging three sources, in this order of preference:
//
//   1. order_items    — primary source when present (carries the
//                       authoritative unit_price)
//   2. packaging_log  — primary source when order_items is empty
//                       (reconstructs sort/packs/stems/price from
//                       the warehouse event log and flower_sorts)
//   3. order snapshot — last-resort fallback (pack_count_ordered
//                       + total_amount) so the sheet is never blank
//
// This is what lets old/broken orders still print something useful,
// and what makes the "warehouse→order chain" visible in one place.
// ────────────────────────────────────────────────────────────────
interface PrintRow {
	sortId: string | null;
	sortName: string;
	variety?: string;
	packCount: number;
	stemsPerPack: number;
	pricePerPack: number;
	pricePerStem: number;
	lineTotal: number;
	reservedPacks: number;
	source: 'order_item' | 'packaging_log' | 'legacy';
	packagedAt?: string;
}

function buildPrintRows(
	order: Order,
	items: OrderItem[],
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	packagingLog: PackagingLogEntry[] = [],
	packAssignments: PackAssignment[] = []
): PrintRow[] {
	const reservedBySort = new Map<string, number>();
	for (const a of packAssignments) {
		reservedBySort.set(a.sort_id, (reservedBySort.get(a.sort_id) ?? 0) + a.pack_count);
	}

	// Path 1: authoritative order_items. We still enrich each row with
	// packaging_log info (so missing unit_price can fall back to the sort's
	// sell_price_stem via computeLine, and reserved pulls from assignments).
	if (items.length > 0) {
		return items.map((it) => {
			const sort = flowerSorts.find((s) => s.id === it.sort_id || s.id === it.item_id);
			const { name, variety } = resolveItemName(it, flowerSorts, inventoryItems);
			const calc = computeLine(it, sort, constants);
			const sortId = it.sort_id ?? sort?.id ?? null;
			return {
				sortId,
				sortName: name,
				variety,
				packCount: calc.packCount,
				stemsPerPack: calc.stemsPerPack,
				pricePerPack: calc.pricePerPack,
				pricePerStem: calc.pricePerStem,
				lineTotal: calc.lineTotal,
				reservedPacks: sortId ? (reservedBySort.get(sortId) ?? 0) : 0,
				source: 'order_item',
			};
		});
	}

	// Path 2: reconstruct from packaging_log. The warehouse always writes
	// one row here per packaging event, so this is effectively the
	// production audit trail — far more reliable than whatever ended up
	// in order_items for legacy orders.
	if (packagingLog.length > 0) {
		return packagingLog.map((pl) => {
			const sort = flowerSorts.find((s) => s.id === pl.sort_id);
			const fallbackStems =
				sort?.flowers_per_pack_override ?? constants.flowers_per_pack ?? 1;
			const stemsPerPack = pl.stems_per_pack > 0 ? pl.stems_per_pack : fallbackStems;
			const pricePerStem = pl.sell_price_stem > 0
				? pl.sell_price_stem
				: (sort?.sell_price_stem ?? 0);
			const pricePerPack = pricePerStem * stemsPerPack;
			const lineTotal = pl.pack_count * pricePerPack;
			return {
				sortId: pl.sort_id,
				sortName: pl.sort_name || sort?.name || '—',
				variety: pl.variety ?? sort?.variety,
				packCount: pl.pack_count,
				stemsPerPack,
				pricePerPack,
				pricePerStem,
				lineTotal,
				reservedPacks: reservedBySort.get(pl.sort_id) ?? 0,
				source: 'packaging_log',
				packagedAt: pl.created_at,
			};
		});
	}

	// Path 3: absolute fallback. Nothing in order_items, nothing in
	// packaging_log — but the order itself may still carry a summary
	// (pack_count_ordered / total_amount). Render a single grey row so
	// the sheet doesn't look broken.
	if (order.pack_count_ordered > 0 || order.total_amount > 0) {
		const packs = order.pack_count_ordered ?? 0;
		const total = order.total_amount ?? 0;
		return [
			{
				sortId: null,
				sortName: '—',
				packCount: packs,
				stemsPerPack: 0,
				pricePerPack: packs > 0 ? total / packs : 0,
				pricePerStem: 0,
				lineTotal: total,
				reservedPacks: 0,
				source: 'legacy',
			},
		];
	}
	return [];
}

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
		font-size: 14pt;
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
		gap: 6px 32px;
		margin: 0;
		font-size: 12pt;
	}
	.print-meta > div { display: flex; gap: 8px; }
	.print-meta dt { font-weight: 600; color: #444; margin: 0; }
	.print-meta dd { margin: 0; color: #111; }

	table.print-items {
		width: 100%;
		border-collapse: collapse;
		margin: 12px 0;
		font-size: 13pt;
	}
	table.print-items thead th {
		background: #e8e8e8;
		color: #000;
		font-weight: 700;
		text-align: left;
		padding: 11px 12px;
		border: 1px solid #777;
	}
	table.print-items tbody td {
		padding: 10px 12px;
		border: 1px solid #bbb;
		vertical-align: top;
	}
	table.print-items tbody tr:nth-child(even) td { background: #f6f6f6; }

	.c-num   { width: 32px; text-align: center; }
	.c-qty   { width: 78px; text-align: center; }
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
		padding: 12px 16px;
		border-top: 2.5px solid #111;
		font-size: 16pt;
		font-weight: 700;
	}
	.tot-label { letter-spacing: 0.02em; }
	.tot-val   { min-width: 160px; text-align: right; }

	.print-totals-grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0;
		margin-top: 12px;
		border-top: 2.5px solid #111;
		border-bottom: 2.5px solid #111;
	}
	.print-totals-grid .tot-cell {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		padding: 12px 16px;
		border-right: 1px solid #bbb;
		font-size: 13pt;
	}
	.print-totals-grid .tot-cell:last-child { border-right: 0; }
	.print-totals-grid .tot-cell .tot-label {
		font-size: 10.5pt;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #555;
		font-weight: 600;
	}
	.print-totals-grid .tot-cell .tot-val {
		font-size: 17pt;
		font-weight: 700;
		color: #111;
		min-width: 0;
		text-align: left;
	}
	.print-totals-grid .tot-cell-grand { background: #f6f6f6; }
	.print-totals-grid .tot-cell-grand .tot-val { font-size: 19pt; }

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

	.print-warehouse {
		margin-top: 14px;
		padding: 10px 14px;
		border: 1px solid #999;
		background: #fafafa;
	}
	.print-warehouse h3 {
		margin: 0 0 8px;
		font-size: 12pt;
		font-weight: 700;
		letter-spacing: 0.02em;
		text-transform: uppercase;
		color: #333;
	}
	table.print-warehouse-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 10.5pt;
	}
	table.print-warehouse-table th,
	table.print-warehouse-table td {
		padding: 5px 8px;
		border: 1px solid #bbb;
		text-align: right;
		white-space: nowrap;
	}
	table.print-warehouse-table th {
		background: #e8e8e8;
		color: #000;
		font-weight: 700;
	}
	table.print-warehouse-table td.wh-name,
	table.print-warehouse-table th.wh-name {
		text-align: left;
		white-space: normal;
	}
	table.print-warehouse-table tr.wh-deficit td { background: #fde0e0; }
	table.print-warehouse-table tr.wh-deficit td.wh-deficit-cell {
		color: #b91c1c;
		font-weight: 700;
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
		font-size: 11.5pt;
		margin-top: 8px;
	}
	/* repeat header on every printed page */
	table.print-registry thead { display: table-header-group; }
	table.print-registry tfoot { display: table-footer-group; }
	table.print-registry thead th {
		background: #e8e8e8;
		color: #000;
		font-weight: 700;
		text-align: left;
		padding: 8px 10px;
		border: 1px solid #777;
		white-space: nowrap;
	}
	table.print-registry tbody td {
		padding: 7px 10px;
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
	table.print-registry .reg-num   { width: 32px; text-align: center; }
	table.print-registry .reg-date  { width: 96px; white-space: nowrap; }
	table.print-registry .reg-cust  { min-width: 150px; font-weight: 600; }
	table.print-registry .reg-sort  { min-width: 130px; }
	table.print-registry .reg-qty   { width: 64px; text-align: center; }
	table.print-registry .reg-price { width: 96px; text-align: right; white-space: nowrap; }
	table.print-registry .reg-total { width: 106px; text-align: right; font-weight: 700; white-space: nowrap; }
	table.print-registry .reg-empty  { color: #888; font-style: italic; text-align: center; }

	.print-registry-footer {
		margin-top: 12px;
		padding: 12px 16px;
		border-top: 2.5px solid #111;
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 12px 24px;
		align-items: baseline;
		font-size: 13pt;
	}
	.print-registry-footer .grand {
		font-size: 15pt;
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
	rows: PrintRow[],
	currencyCode: string,
	t: TranslateFn
): { html: string; subtotal: number } {
	let subtotal = 0;
	const bodyRows = rows
		.map((r, idx) => {
			subtotal += r.lineTotal;
			const productLabel = r.variety
				? `${escapeHtml(r.sortName)} — ${escapeHtml(r.variety)}`
				: escapeHtml(r.sortName);
			const sourceTag = r.source === 'packaging_log'
				? ` <span style="color:#888;font-size:9pt;font-weight:400;">${escapeHtml(
						t('print_row_from_packaging') || 'из упаковки'
					)}</span>`
				: r.source === 'legacy'
					? ` <span style="color:#b91c1c;font-size:9pt;font-weight:400;">${escapeHtml(
							t('print_row_legacy') || 'архивный заказ'
						)}</span>`
					: '';
			return `
				<tr>
					<td class="c-num">${idx + 1}</td>
					<td class="c-name">${productLabel}${sourceTag}</td>
					<td class="c-qty">${r.packCount || '—'}</td>
					<td class="c-qty">${r.stemsPerPack || '—'}</td>
					<td class="c-price">${r.pricePerStem > 0 ? formatMoney(r.pricePerStem, currencyCode) : '—'}</td>
					<td class="c-price">${r.pricePerPack > 0 ? formatMoney(r.pricePerPack, currencyCode) : '—'}</td>
					<td class="c-total">${formatMoney(r.lineTotal, currencyCode)}</td>
				</tr>
			`;
		})
		.join('');

	const emptyRow =
		rows.length === 0
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
			<tbody>${bodyRows}${emptyRow}</tbody>
		</table>
	`;

	return { html, subtotal };
}

// ────────────────────────────────────────────────────────────────
// Warehouse/greenhouse state block — prints the raw stock, pkg
// stock, reserved packs (aggregated from pack_assignments) and
// the deficit vs. the ordered pack count. Rendered only when the
// caller passes pack assignments (flowers mode); otherwise the
// block is omitted entirely.
// ────────────────────────────────────────────────────────────────
function renderWarehouseBlock(
	printRows: PrintRow[],
	flowerSorts: FlowerSort[],
	packAssignments: PackAssignment[] | undefined,
	t: TranslateFn
): string {
	if (!packAssignments) return '';

	type Row = {
		name: string;
		variety: string | null;
		rawStock: number;
		pkgStock: number;
		reserved: number;
		needed: number;
		deficit: number;
		statusCounts: Record<string, number>;
	};
	const rows: Row[] = [];

	// Aggregate by sort — one warehouse row per unique sort, even if the
	// order has multiple print rows for it (e.g. split packaging events).
	const bySort = new Map<string, { name: string; variety?: string; needed: number }>();
	for (const r of printRows) {
		if (!r.sortId) continue;
		const existing = bySort.get(r.sortId);
		if (existing) {
			existing.needed += r.packCount;
		} else {
			bySort.set(r.sortId, {
				name: r.sortName,
				variety: r.variety,
				needed: r.packCount,
			});
		}
	}

	for (const [sortId, agg] of bySort.entries()) {
		const sort = flowerSorts.find((s) => s.id === sortId);
		const linked = packAssignments.filter((a) => a.sort_id === sortId);
		const reserved = linked.reduce((sum, a) => sum + a.pack_count, 0);
		const statusCounts: Record<string, number> = { prepared: 0, loaded: 0, delivered: 0 };
		for (const a of linked) {
			statusCounts[a.status] = (statusCounts[a.status] ?? 0) + a.pack_count;
		}
		rows.push({
			name: agg.name,
			variety: agg.variety ?? null,
			rawStock: sort?.raw_stock ?? 0,
			pkgStock: sort?.pkg_stock ?? 0,
			reserved,
			needed: agg.needed,
			deficit: Math.max(0, agg.needed - reserved),
			statusCounts,
		});
	}

	if (rows.length === 0) return '';

	const body = rows
		.map((r) => {
			const productLabel = r.variety
				? `${escapeHtml(r.name)} — ${escapeHtml(r.variety)}`
				: escapeHtml(r.name);
			const statusDetail =
				r.reserved > 0
					? ` <span style="color:#555;font-size:9.5pt;">(${escapeHtml(t('pack_status_prepared'))} ${r.statusCounts.prepared ?? 0}, ${escapeHtml(t('pack_status_loaded'))} ${r.statusCounts.loaded ?? 0}, ${escapeHtml(t('pack_status_delivered'))} ${r.statusCounts.delivered ?? 0})</span>`
					: '';
			return `
				<tr class="${r.deficit > 0 ? 'wh-deficit' : ''}">
					<td class="wh-name">${productLabel}</td>
					<td>${r.rawStock}</td>
					<td>${r.pkgStock}</td>
					<td>${r.reserved}${statusDetail}</td>
					<td>${r.needed}</td>
					<td class="wh-deficit-cell">${r.deficit}</td>
				</tr>
			`;
		})
		.join('');

	return `
		<div class="print-warehouse">
			<h3>${escapeHtml(t('section_warehouse_state'))}</h3>
			<table class="print-warehouse-table">
				<thead>
					<tr>
						<th class="wh-name">${escapeHtml(t('label_product'))}</th>
						<th>${escapeHtml(t('label_raw_stock'))}</th>
						<th>${escapeHtml(t('label_pkg_stock'))}</th>
						<th>${escapeHtml(t('label_assigned_packs'))}</th>
						<th>${escapeHtml(t('label_pack_count'))}</th>
						<th>${escapeHtml(t('label_deficit'))}</th>
					</tr>
				</thead>
				<tbody>${body}</tbody>
			</table>
		</div>
	`;
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

	// WebView2 fires the `load` event twice: once for the initial about:blank
	// when the iframe is appended, and again after `document.write` flushes.
	// Without a guard, `win.print()` runs twice → two system dialogs, requiring
	// the user to hit Cancel twice. Guard with both `{ once: true }` and an
	// explicit boolean so defense-in-depth catches any WebView2 quirk.
	let printed = false;
	const doPrint = () => {
		if (printed) return;
		printed = true;
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

	frame.addEventListener(
		'load',
		() => {
			// Give the WebView one paint tick to finalize layout inside the iframe.
			setTimeout(doPrint, 50);
		},
		{ once: true }
	);

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
	t: TranslateFn,
	opts: PrintSingleOptions = {}
): void {
	const header = renderOrderHeader(order, t);
	const printRows = buildPrintRows(
		order,
		items,
		flowerSorts,
		inventoryItems,
		constants,
		opts.packagingLog ?? [],
		opts.packAssignments ?? []
	);
	const { html, subtotal } = renderItemsTable(printRows, currencyCode, t);
	// Prefer the computed subtotal when order.total_amount is stale (common
	// for legacy orders where total_amount wasn't re-aggregated) — but if the
	// row set itself is empty/legacy, keep whatever the order carries.
	const totalAmount = subtotal > 0 ? subtotal : order.total_amount;
	const warehouse = renderWarehouseBlock(
		printRows,
		flowerSorts,
		opts.packAssignments,
		t
	);

	// Aggregate totals across all print rows so the customer-facing receipt
	// shows pack count + stem count alongside the money total.
	let totalPacks = 0;
	let totalStems = 0;
	for (const r of printRows) {
		totalPacks += r.packCount;
		totalStems += r.packCount * r.stemsPerPack;
	}

	const body = `
		${header}
		${html}
		<div class="print-totals-grid">
			<div class="tot-cell">
				<span class="tot-label">${escapeHtml(t('label_pack_count'))}</span>
				<span class="tot-val">${totalPacks || '—'}</span>
			</div>
			<div class="tot-cell">
				<span class="tot-label">${escapeHtml(t('flowers_total_raw'))}</span>
				<span class="tot-val">${totalStems || '—'}</span>
			</div>
			<div class="tot-cell tot-cell-grand">
				<span class="tot-label">${escapeHtml(t('print_summary'))}</span>
				<span class="tot-val">${formatMoney(totalAmount, currencyCode)}</span>
			</div>
		</div>
		${warehouse}
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
	t: TranslateFn,
	opts: PrintMultiOptions = {}
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
		const printRows = buildPrintRows(
			order,
			items,
			flowerSorts,
			inventoryItems,
			constants,
			opts.packagingLogByOrder?.[order.id] ?? [],
			opts.packAssignmentsByOrder?.[order.id] ?? []
		);
		const { html, subtotal } = renderItemsTable(printRows, currencyCode, t);
		const orderTotal = subtotal > 0 ? subtotal : order.total_amount;
		grandTotal += orderTotal;

		for (const r of printRows) {
			const key = r.sortId ?? r.sortName;
			const label = r.variety ? `${r.sortName} — ${r.variety}` : r.sortName;
			const existing = sortBreakdown.get(key) ?? {
				packs: 0,
				stems: 0,
				total: 0,
				name: label,
			};
			existing.packs += r.packCount;
			existing.stems += r.packCount * r.stemsPerPack;
			existing.total += r.lineTotal;
			sortBreakdown.set(key, existing);
		}

		const warehouse = renderWarehouseBlock(
			printRows,
			flowerSorts,
			opts.packAssignmentsByOrder?.[order.id],
			t
		);

		sections.push(`
			<section class="print-order">
				${header}
				${html}
				<div class="print-order-subtotal">
					<span>${t('print_summary')}:</span>
					<strong>${formatMoney(orderTotal, currencyCode)}</strong>
				</div>
				${warehouse}
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

export async function printOrdersRegistry(
	ordersList: Order[],
	getItems: GetItemsFn,
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	currencyCode: string,
	t: TranslateFn,
	range?: { from: string; to: string },
	opts: PrintMultiOptions = {}
): Promise<void> {
	// Pre-fetch all items in parallel, preserving input order.
	const pairs = await Promise.all(
		ordersList.map(async (o) => ({ order: o, items: await getItems(o.id) }))
	);

	const rows: string[] = [];
	let grandTotal = 0;
	let grandPacks = 0;
	let grandStems = 0;
	let rowIdx = 0;

	for (const { order, items } of pairs) {
		const dateShort = escapeHtml(formatDateShort(order.created_at));
		const customer = escapeHtml(order.customer_name || '—');

		const orderAssignments = opts.packAssignmentsByOrder?.[order.id] ?? [];
		const orderPackagingLog = opts.packagingLogByOrder?.[order.id] ?? [];

		const printRows = buildPrintRows(
			order,
			items,
			flowerSorts,
			inventoryItems,
			constants,
			orderPackagingLog,
			orderAssignments
		);

		if (printRows.length === 0) {
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
					<td class="reg-total">—</td>
				</tr>
			`);
			continue;
		}

		let orderSubtotal = 0;
		const rowspan = printRows.length;
		printRows.forEach((r, i) => {
			rowIdx++;
			if (r.lineTotal === 0 && r.packCount === 0) {
				console.warn('[printOrdersRegistry] zero-value line', {
					order: order.id, sort: r.sortId, source: r.source,
				});
			}
			orderSubtotal += r.lineTotal;
			grandPacks += r.packCount;
			grandStems += r.packCount * r.stemsPerPack;
			const sortLabel = r.variety
				? `${escapeHtml(r.sortName)} — ${escapeHtml(r.variety)}`
				: escapeHtml(r.sortName);
			const sourceHint =
				r.source === 'packaging_log'
					? ` <span style="color:#888;font-size:9pt;">(${escapeHtml(
							t('print_row_from_packaging') || 'из упаковки'
						)})</span>`
					: r.source === 'legacy'
						? ` <span style="color:#b91c1c;font-size:9pt;">(${escapeHtml(
								t('print_row_legacy') || 'архив'
							)})</span>`
						: '';
			const isFirst = i === 0;
			rows.push(`
				<tr>
					<td class="reg-num">${rowIdx}</td>
					${isFirst ? `<td class="reg-date" rowspan="${rowspan}">${dateShort}</td>` : ''}
					${isFirst ? `<td class="reg-cust" rowspan="${rowspan}">${customer}</td>` : ''}
					<td class="reg-sort">${sortLabel}${sourceHint}</td>
					<td class="reg-qty">${r.packCount || '—'}</td>
					<td class="reg-qty">${r.stemsPerPack || '—'}</td>
					<td class="reg-price">${r.pricePerPack > 0 ? formatMoney(r.pricePerPack, currencyCode) : '—'}</td>
					<td class="reg-total">${formatMoney(r.lineTotal, currencyCode)}</td>
				</tr>
			`);
		});

		const finalTotal = orderSubtotal > 0 ? orderSubtotal : order.total_amount;
		grandTotal += finalTotal;
		rows.push(`
			<tr class="order-subtotal-row">
				<td colspan="7">${escapeHtml(t('print_summary'))}:</td>
				<td class="reg-total">${formatMoney(finalTotal, currencyCode)}</td>
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
				</tr>
			</thead>
			<tbody>
				${
					rows.length === 0
						? `<tr><td colspan="8" class="reg-empty" style="padding:24px;">${escapeHtml(t('print_dialog_no_orders'))}</td></tr>`
						: rows.join('')
				}
			</tbody>
		</table>
	`;

	const totalsFooter = `
		<div class="print-registry-footer">
			<span>${escapeHtml(t('print_total_orders'))}: <strong>${ordersList.length}</strong></span>
			<span>${escapeHtml(t('print_registry_grand_packs'))}: <strong>${grandPacks}</strong></span>
			<span>${escapeHtml(t('print_registry_grand_stems'))}: <strong>${grandStems}</strong></span>
			<span class="grand">${escapeHtml(t('print_registry_grand_total'))}: ${formatMoney(grandTotal, currencyCode)}</span>
		</div>
	`;

	const body = header + tableHtml + totalsFooter;
	printViaIframe(t('print_registry_title'), body);
}

// ────────────────────────────────────────────────────────────────
// Contacts × Sorts matrix print.
//
// Excel-style pivot: rows are clients, columns are flower sorts,
// cell = sum of packs ordered by that client for that sort in the
// window. Rightmost column and bottom row are totals — the bottom-
// right cell is the grand total, and "row totals sum == col totals
// sum == grand total" is the debit/credit invariant the user asked
// for. Orders without a contact are grouped under a single row.
// ────────────────────────────────────────────────────────────────
export async function printContactsMatrix(
	ordersList: Order[],
	getItems: GetItemsFn,
	flowerSorts: FlowerSort[],
	inventoryItems: Item[],
	constants: FlowerConstants,
	contactsList: Contact[],
	currencyCode: string,
	t: TranslateFn,
	range?: { from: string; to: string },
	opts: PrintMultiOptions = {}
): Promise<void> {
	// Build the cell map in a single pass: for each order, resolve its print
	// rows (order_items → packaging_log → legacy fallback, same pipeline the
	// registry uses) and accumulate pack_count by (client_key, sort_id).
	type CellKey = string; // "<clientKey>|<sortId>"
	const NO_CONTACT = '__none__';
	const cellPacks = new Map<CellKey, number>();
	const cellMoney = new Map<CellKey, number>();
	const rowPacks = new Map<string, number>();
	const rowMoney = new Map<string, number>();
	const colPacks = new Map<string, number>();
	const colMoney = new Map<string, number>();
	const rowLabels = new Map<string, string>();       // clientKey → display name
	const colLabels = new Map<string, string>();       // sortId → display name
	const colColors = new Map<string, string | null>();// sortId → hex (optional)
	let grandPacks = 0;
	let grandMoney = 0;

	// Seed "No contact" row lazily — only insert it if any order lacks a
	// contact_id, to keep the printed grid tight when every order is tagged.
	function clientKey(order: Order): { key: string; label: string } {
		if (order.contact_id) {
			const c = contactsList.find((x) => x.id === order.contact_id);
			const display = c
				? c.name + (c.surname ? ` ${c.surname}` : '')
				: order.customer_name || '—';
			return { key: order.contact_id, label: display };
		}
		return {
			key: NO_CONTACT,
			label: order.customer_name?.trim() || t('print_matrix_no_contact') || '—',
		};
	}

	const pairs = await Promise.all(
		ordersList.map(async (o) => ({ order: o, items: await getItems(o.id) }))
	);

	for (const { order, items } of pairs) {
		const rows = buildPrintRows(
			order,
			items,
			flowerSorts,
			inventoryItems,
			constants,
			opts.packagingLogByOrder?.[order.id] ?? [],
			opts.packAssignmentsByOrder?.[order.id] ?? []
		);
		const { key: rKey, label: rLabel } = clientKey(order);
		if (!rowLabels.has(rKey)) rowLabels.set(rKey, rLabel);

		for (const r of rows) {
			if (r.packCount <= 0) continue;
			// Collapse legacy rows (sortId = null) under a synthetic "—" column so
			// we never silently drop them. The user needs the invariant to hold.
			const cKey = r.sortId ?? '__legacy__';
			if (!colLabels.has(cKey)) {
				const label = r.variety ? `${r.sortName} — ${r.variety}` : r.sortName;
				colLabels.set(cKey, label);
				// Prefer the flower sort's color_hex when present — so the header
				// cell can show a swatch without guessing from the HEX picker.
				const sort = r.sortId
					? flowerSorts.find((s) => s.id === r.sortId)
					: undefined;
				colColors.set(cKey, sort?.color_hex ?? null);
			}
			const cellK = `${rKey}|${cKey}`;
			cellPacks.set(cellK, (cellPacks.get(cellK) ?? 0) + r.packCount);
			cellMoney.set(cellK, (cellMoney.get(cellK) ?? 0) + r.lineTotal);
			rowPacks.set(rKey, (rowPacks.get(rKey) ?? 0) + r.packCount);
			rowMoney.set(rKey, (rowMoney.get(rKey) ?? 0) + r.lineTotal);
			colPacks.set(cKey, (colPacks.get(cKey) ?? 0) + r.packCount);
			colMoney.set(cKey, (colMoney.get(cKey) ?? 0) + r.lineTotal);
			grandPacks += r.packCount;
			grandMoney += r.lineTotal;
		}
	}

	const rowKeys = Array.from(rowLabels.keys()).sort((a, b) => {
		// Sort by total packs desc, but keep the "no contact" row at the bottom.
		if (a === NO_CONTACT) return 1;
		if (b === NO_CONTACT) return -1;
		return (rowPacks.get(b) ?? 0) - (rowPacks.get(a) ?? 0);
	});
	const colKeys = Array.from(colLabels.keys()).sort(
		(a, b) => (colPacks.get(b) ?? 0) - (colPacks.get(a) ?? 0)
	);

	const rangeLine =
		range && (range.from || range.to)
			? `<div class="print-registry-range">${escapeHtml(t('print_registry_period'))}: ${escapeHtml(range.from)} — ${escapeHtml(range.to)}</div>`
			: '';

	const header = `
		<div class="print-registry-header">
			<h1>${escapeHtml(t('print_matrix_title'))}</h1>
			${rangeLine}
			<div class="print-header-date">${escapeHtml(t('print_date'))}: ${escapeHtml(formatDateTime(new Date().toISOString()))}</div>
		</div>
	`;

	if (rowKeys.length === 0 || colKeys.length === 0) {
		const body = header + `<p style="padding:24px;text-align:center;color:#666;font-size:12pt;">${escapeHtml(t('print_matrix_empty'))}</p>`;
		printViaIframe(t('print_matrix_title'), body);
		return;
	}

	// Header row: "#", "Клиент", <sort columns...>, "Итого (клиент)"
	const thCols = colKeys
		.map((ck, idx) => {
			const label = colLabels.get(ck) ?? '—';
			const color = colColors.get(ck);
			const swatch = color
				? `<span class="matrix-swatch" style="background:${escapeHtml(color)}"></span>`
				: '';
			return `<th class="matrix-sort" title="${escapeHtml(label)}"><span class="matrix-sort-num">${idx + 1}</span>${swatch}<span class="matrix-sort-name">${escapeHtml(label)}</span></th>`;
		})
		.join('');

	const bodyRows = rowKeys
		.map((rk, rIdx) => {
			const label = rowLabels.get(rk) ?? '—';
			const cells = colKeys
				.map((ck) => {
					const packs = cellPacks.get(`${rk}|${ck}`) ?? 0;
					return `<td class="matrix-cell ${packs === 0 ? 'matrix-empty' : ''}">${packs > 0 ? packs : ''}</td>`;
				})
				.join('');
			const rowTotal = rowPacks.get(rk) ?? 0;
			const rowMoneyVal = rowMoney.get(rk) ?? 0;
			return `
				<tr>
					<td class="matrix-num">${rIdx + 1}</td>
					<td class="matrix-client">${escapeHtml(label)}</td>
					${cells}
					<td class="matrix-total">
						<div>${rowTotal}</div>
						<div class="matrix-total-money">${formatMoney(rowMoneyVal, currencyCode)}</div>
					</td>
				</tr>
			`;
		})
		.join('');

	const footerCells = colKeys
		.map((ck) => {
			const packs = colPacks.get(ck) ?? 0;
			const money = colMoney.get(ck) ?? 0;
			return `<td class="matrix-col-total">
				<div>${packs}</div>
				<div class="matrix-total-money">${formatMoney(money, currencyCode)}</div>
			</td>`;
		})
		.join('');

	const table = `
		<table class="print-matrix">
			<thead>
				<tr>
					<th class="matrix-num">#</th>
					<th class="matrix-client">${escapeHtml(t('label_customer_name'))}</th>
					${thCols}
					<th class="matrix-total">${escapeHtml(t('print_matrix_row_total'))}</th>
				</tr>
			</thead>
			<tbody>${bodyRows}</tbody>
			<tfoot>
				<tr>
					<td class="matrix-num"></td>
					<td class="matrix-client matrix-col-total">${escapeHtml(t('print_matrix_col_total'))}</td>
					${footerCells}
					<td class="matrix-grand">
						<div>${grandPacks}</div>
						<div class="matrix-total-money">${formatMoney(grandMoney, currencyCode)}</div>
					</td>
				</tr>
			</tfoot>
		</table>
	`;

	const extraCss = `
		<style>
			table.print-matrix {
				width: 100%;
				border-collapse: collapse;
				font-size: 12pt;
				margin-top: 10px;
			}
			table.print-matrix thead th,
			table.print-matrix tbody td,
			table.print-matrix tfoot td {
				border: 1px solid #999;
				padding: 7px 9px;
				text-align: center;
				vertical-align: middle;
			}
			table.print-matrix thead th {
				background: #e6e6e6;
				font-weight: 700;
				font-size: 11pt;
			}
			table.print-matrix th.matrix-sort {
				min-width: 76px;
				max-width: 112px;
				word-break: break-word;
				white-space: normal;
				line-height: 1.25;
			}
			.matrix-sort-num {
				display: inline-block;
				min-width: 20px;
				padding: 1px 5px;
				border-radius: 50%;
				background: #111;
				color: #fff;
				font-size: 9.5pt;
				font-weight: 700;
				margin-right: 4px;
			}
			.matrix-swatch {
				display: inline-block;
				width: 11px;
				height: 11px;
				border-radius: 50%;
				margin-right: 4px;
				border: 1px solid rgba(0,0,0,0.2);
				vertical-align: middle;
			}
			.matrix-sort-name { font-size: 10pt; }
			table.print-matrix th.matrix-client,
			table.print-matrix td.matrix-client {
				text-align: left;
				font-weight: 600;
				white-space: nowrap;
				padding-left: 12px;
			}
			table.print-matrix td.matrix-num {
				width: 34px;
				color: #555;
				font-weight: 600;
			}
			table.print-matrix td.matrix-cell {
				font-weight: 700;
				font-size: 13pt;
			}
			/* Empty grid cells stay visually quiet, matching the paper-journal
			   aesthetic the user referenced. */
			table.print-matrix td.matrix-empty { color: transparent; }
			table.print-matrix td.matrix-total,
			table.print-matrix td.matrix-col-total,
			table.print-matrix td.matrix-grand {
				background: #f3f3f3;
				font-weight: 700;
			}
			table.print-matrix td.matrix-grand {
				background: #111;
				color: #fff;
			}
			.matrix-total-money {
				font-size: 10pt;
				font-weight: 500;
				color: #555;
				margin-top: 2px;
			}
			table.print-matrix td.matrix-grand .matrix-total-money {
				color: #eee;
			}
			table.print-matrix tbody tr:nth-child(even) td { background: #f9f9f9; }
			table.print-matrix tbody tr:nth-child(even) td.matrix-total { background: #e8e8e8; }
			@page { size: landscape; margin: 10mm; }
		</style>
	`;

	const body = extraCss + header + table;
	printViaIframe(t('print_matrix_title'), body);
}
