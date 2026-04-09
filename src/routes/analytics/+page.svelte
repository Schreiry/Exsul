<script lang="ts">
	import { inventory, totalRevenue, totalStock, totalItems } from '$lib/stores/inventory';
	import { orders } from '$lib/stores/orders';
	import { flowerSorts } from '$lib/stores/flowers';
	import { preset } from '$lib/stores/preset';
	import { commands } from '$lib/tauri/commands';
	import { t } from '$lib/stores/i18n';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { hlcToDate } from '$lib/utils/time';

	const TZ = 'Asia/Tbilisi';

	// ── Date range filter ─────────────────────────────────────
	let dateFrom = $state('');
	let dateTo = $state('');

	$effect(() => {
		orders.load();
		if ($preset === 'flowers') flowerSorts.load();
	});

	function fmt(value: number): string {
		return formatAmount(value, $globalCurrency);
	}

	// ── Orders analytics ──────────────────────────────────────
	const orderStats = $derived({
		total: $orders.length,
		pending: $orders.filter((o) => o.status === 'pending').length,
		inProgress: $orders.filter((o) => o.status === 'in_progress').length,
		completed: $orders.filter((o) => o.status === 'completed').length,
		cancelled: $orders.filter((o) => o.status === 'cancelled').length,
		totalAmount: $orders.reduce((s, o) => s + o.total_amount, 0),
		pendingAmount: $orders
			.filter((o) => o.status === 'pending' || o.status === 'in_progress')
			.reduce((s, o) => s + o.total_amount, 0),
		completedAmount: $orders
			.filter((o) => o.status === 'completed')
			.reduce((s, o) => s + o.total_amount, 0),
	});

	// ── Inventory analytics ───────────────────────────────────
	const warehouseStats = $derived({
		totalCost: $inventory.reduce((s, i) => s + i.production_cost * i.current_stock, 0),
		totalValue: $inventory.reduce((s, i) => s + i.current_price * i.current_stock, 0),
		avgMargin: (() => {
			const totalStock = $inventory.reduce((s, i) => s + i.current_stock, 0);
			if (totalStock === 0) return 0;
			const weighted = $inventory.reduce((s, i) => {
				const m = i.current_price > 0
					? (i.current_price - i.production_cost) / i.current_price
					: 0;
				return s + m * i.current_stock;
			}, 0);
			return Math.round((weighted / totalStock) * 100);
		})(),
		totalSold: $inventory.reduce((s, i) => s + i.sold_count, 0),
	});

	let categoryStats = $derived(
		Object.entries(
			$inventory.reduce(
				(acc, item) => {
					if (!acc[item.category]) {
						acc[item.category] = { count: 0, stock: 0, revenue: 0, sold: 0 };
					}
					acc[item.category].count++;
					acc[item.category].stock += item.current_stock;
					acc[item.category].revenue += item.revenue;
					acc[item.category].sold += item.sold_count;
					return acc;
				},
				{} as Record<string, { count: number; stock: number; revenue: number; sold: number }>
			)
		).sort((a, b) => b[1].revenue - a[1].revenue)
	);

	let topSellers = $derived(
		[...$inventory].sort((a, b) => b.sold_count - a.sold_count).slice(0, 10)
	);

	// Supply run-rate: weeks of stock remaining based on last 8 weeks of sales pace
	let runRate = $derived(
		$inventory
			.filter((item) => item.sold_count > 0)
			.map((item) => {
				const weeksLeft = item.current_stock / (item.sold_count / 8);
				return { name: item.name, weeksLeft, stock: item.current_stock };
			})
			.sort((a, b) => a.weeksLeft - b.weeksLeft)
	);

	// ── Canvas: Revenue by Category Bar Chart ──
	let revenueCanvas = $state<HTMLCanvasElement | null>(null);

	$effect(() => {
		const canvas = revenueCanvas;
		if (!canvas) return;
		const data = categoryStats.slice(0, 8);
		if (data.length === 0) return;

		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const style = getComputedStyle(document.documentElement);
		const primaryColor = style.getPropertyValue('--color-primary').trim() || '#34d399';
		const secondaryColor = style.getPropertyValue('--color-secondary').trim() || '#5bb8d0';
		const tertiaryColor = style.getPropertyValue('--color-tertiary').trim() || '#a78bfa';
		const outlineVariant = style.getPropertyValue('--color-outline-variant').trim() || 'rgba(255,255,255,0.1)';
		const onSurface = style.getPropertyValue('--color-on-surface').trim() || '#e0e0e0';

		const barColors = [primaryColor, secondaryColor, tertiaryColor];

		const dpr = window.devicePixelRatio || 1;
		const rect = canvas.getBoundingClientRect();
		canvas.width = rect.width * dpr;
		canvas.height = rect.height * dpr;
		ctx.scale(dpr, dpr);

		const W = rect.width;
		const H = rect.height;
		const padTop = 16;
		const padBottom = 48;
		const padLeft = 56;
		const padRight = 16;
		const chartW = W - padLeft - padRight;
		const chartH = H - padTop - padBottom;

		ctx.clearRect(0, 0, W, H);

		const maxRevenue = Math.max(...data.map(([, s]) => s.revenue), 1);
		const barCount = data.length;
		const totalGap = chartW * 0.3;
		const barWidth = (chartW - totalGap) / barCount;
		const gapWidth = totalGap / (barCount + 1);

		// Axis line
		ctx.beginPath();
		ctx.strokeStyle = outlineVariant;
		ctx.lineWidth = 1;
		ctx.moveTo(padLeft, padTop);
		ctx.lineTo(padLeft, padTop + chartH);
		ctx.lineTo(padLeft + chartW, padTop + chartH);
		ctx.stroke();

		// Y-axis labels (3 ticks)
		ctx.fillStyle = onSurface;
		ctx.globalAlpha = 0.4;
		ctx.font = '10px system-ui, sans-serif';
		ctx.textAlign = 'right';
		for (let t = 0; t <= 2; t++) {
			const val = (maxRevenue * t) / 2;
			const y = padTop + chartH - (chartH * t) / 2;
			ctx.fillText(
				new Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 1 }).format(val),
				padLeft - 6,
				y + 4
			);
			ctx.beginPath();
			ctx.strokeStyle = outlineVariant;
			ctx.globalAlpha = 0.3;
			ctx.lineWidth = 1;
			ctx.moveTo(padLeft, y);
			ctx.lineTo(padLeft + chartW, y);
			ctx.stroke();
			ctx.globalAlpha = 0.4;
		}

		// Bars
		data.forEach(([category, stats], i) => {
			const barH = (stats.revenue / maxRevenue) * chartH;
			const x = padLeft + gapWidth + i * (barWidth + gapWidth);
			const y = padTop + chartH - barH;

			ctx.globalAlpha = 1;
			ctx.fillStyle = barColors[i % barColors.length];
			const radius = Math.min(4, barWidth / 2);
			ctx.beginPath();
			ctx.roundRect(x, y, barWidth, barH, [radius, radius, 0, 0]);
			ctx.fill();

			// Category label (rotated if many)
			ctx.save();
			ctx.globalAlpha = 0.7;
			ctx.fillStyle = onSurface;
			ctx.font = '10px system-ui, sans-serif';
			ctx.textAlign = barCount > 5 ? 'right' : 'center';
			const labelX = x + barWidth / 2;
			const labelY = padTop + chartH + 12;
			if (barCount > 5) {
				ctx.translate(labelX, labelY);
				ctx.rotate(-Math.PI / 4);
				ctx.fillText(category.slice(0, 12), 0, 0);
			} else {
				ctx.fillText(category.slice(0, 14), labelX, labelY);
			}
			ctx.restore();
		});
	});

	// ── Canvas: Sales Velocity Line Chart ──
	let salesCanvas = $state<HTMLCanvasElement | null>(null);
	let stockCanvas = $state<HTMLCanvasElement | null>(null);

	type DailyBucket = Record<string, number>;

	/** Convert HLC ms to YYYY-MM-DD in Tbilisi TZ */
	function hlcToDay(hlcStr: string): string | null {
		const d = hlcToDate(hlcStr);
		if (!d) return null;
		return d.toLocaleDateString('sv', { timeZone: TZ });
	}

	/** Apply optional date-range filter to a YYYY-MM-DD key */
	function inRange(day: string): boolean {
		if (dateFrom && day < dateFrom) return false;
		if (dateTo && day > dateTo) return false;
		return true;
	}

	async function loadSalesBuckets(): Promise<DailyBucket> {
		const events = await commands.getEvents(undefined, 2000);
		const buckets: DailyBucket = {};
		for (const ev of events) {
			if (ev.event_type !== 'SaleRecorded') continue;
			const day = hlcToDay(ev.hlc_timestamp);
			if (!day || !inRange(day)) continue;
			const qty = (ev.data as { quantity?: number }).quantity ?? 0;
			buckets[day] = (buckets[day] ?? 0) + qty;
		}
		return buckets;
	}

	async function loadStockBuckets(): Promise<DailyBucket> {
		const events = await commands.getEvents(undefined, 2000);
		const buckets: DailyBucket = {};
		for (const ev of events) {
			if (ev.event_type !== 'StockAdjusted' && ev.event_type !== 'ItemCreated') continue;
			const day = hlcToDay(ev.hlc_timestamp);
			if (!day || !inRange(day)) continue;
			let qty = 0;
			if (ev.event_type === 'StockAdjusted') {
				const delta = (ev.data as { delta?: number }).delta ?? 0;
				if (delta > 0) qty = delta; // count only additions
			} else if (ev.event_type === 'ItemCreated') {
				qty = (ev.data as { initial_stock?: number }).initial_stock ?? 0;
			}
			if (qty > 0) buckets[day] = (buckets[day] ?? 0) + qty;
		}
		return buckets;
	}

	let salesBuckets = $state<DailyBucket>({});
	let stockBuckets = $state<DailyBucket>({});

	$effect(() => {
		// eslint-disable-next-line @typescript-eslint/no-unused-expressions
		dateFrom; dateTo; // reactive: reload when range changes
		loadSalesBuckets().then((b) => (salesBuckets = b));
		loadStockBuckets().then((b) => (stockBuckets = b));
	});

	/** Build last-N-days array in Tbilisi TZ */
	function buildDays(n: number): string[] {
		const days: string[] = [];
		const now = new Date();
		for (let i = n - 1; i >= 0; i--) {
			const d = new Date(now.getTime() - i * 86400000);
			days.push(d.toLocaleDateString('sv', { timeZone: TZ }));
		}
		return days;
	}

	$effect(() => {
		const canvas = salesCanvas;
		if (!canvas) return;

		const style = getComputedStyle(document.documentElement);
		const primaryColor = style.getPropertyValue('--color-primary').trim() || '#34d399';
		const outlineVariant = style.getPropertyValue('--color-outline-variant').trim() || 'rgba(255,255,255,0.1)';
		const onSurface = style.getPropertyValue('--color-on-surface').trim() || '#e0e0e0';

		const dpr = window.devicePixelRatio || 1;
		const rect = canvas.getBoundingClientRect();
		canvas.width = rect.width * dpr;
		canvas.height = rect.height * dpr;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		ctx.scale(dpr, dpr);

		const W = rect.width;
		const H = rect.height;
		const padTop = 16;
		const padBottom = 32;
		const padLeft = 36;
		const padRight = 16;
		const chartW = W - padLeft - padRight;
		const chartH = H - padTop - padBottom;

		ctx.clearRect(0, 0, W, H);

		// Build last 30 days using Tbilisi timezone
		const days = buildDays(30);
		const values = days.map((d) => salesBuckets[d] ?? 0);
		const maxVal = Math.max(...values, 1);

		// Axis
		ctx.beginPath();
		ctx.strokeStyle = outlineVariant;
		ctx.lineWidth = 1;
		ctx.moveTo(padLeft, padTop);
		ctx.lineTo(padLeft, padTop + chartH);
		ctx.lineTo(padLeft + chartW, padTop + chartH);
		ctx.stroke();

		// Y ticks
		ctx.fillStyle = onSurface;
		ctx.globalAlpha = 0.4;
		ctx.font = '10px system-ui, sans-serif';
		ctx.textAlign = 'right';
		for (let tick = 0; tick <= 3; tick++) {
			const val = Math.round((maxVal * tick) / 3);
			const y = padTop + chartH - (chartH * tick) / 3;
			ctx.fillText(String(val), padLeft - 4, y + 4);
			ctx.beginPath();
			ctx.strokeStyle = outlineVariant;
			ctx.globalAlpha = 0.2;
			ctx.lineWidth = 1;
			ctx.moveTo(padLeft, y);
			ctx.lineTo(padLeft + chartW, y);
			ctx.stroke();
			ctx.globalAlpha = 0.4;
		}

		// Line
		ctx.globalAlpha = 1;
		ctx.beginPath();
		ctx.strokeStyle = primaryColor;
		ctx.lineWidth = 2;
		ctx.lineJoin = 'round';
		values.forEach((val, i) => {
			const x = padLeft + (i / (days.length - 1)) * chartW;
			const y = padTop + chartH - (val / maxVal) * chartH;
			if (i === 0) ctx.moveTo(x, y);
			else ctx.lineTo(x, y);
		});
		ctx.stroke();

		// Fill under line
		ctx.globalAlpha = 0.15;
		ctx.fillStyle = primaryColor;
		ctx.beginPath();
		values.forEach((val, i) => {
			const x = padLeft + (i / (days.length - 1)) * chartW;
			const y = padTop + chartH - (val / maxVal) * chartH;
			if (i === 0) ctx.moveTo(x, y);
			else ctx.lineTo(x, y);
		});
		ctx.lineTo(padLeft + chartW, padTop + chartH);
		ctx.lineTo(padLeft, padTop + chartH);
		ctx.closePath();
		ctx.fill();

		// X-axis labels (every 7 days)
		ctx.globalAlpha = 0.4;
		ctx.fillStyle = onSurface;
		ctx.textAlign = 'center';
		[0, 7, 14, 21, 29].forEach((i) => {
			const x = padLeft + (i / (days.length - 1)) * chartW;
			ctx.fillText(days[i]?.slice(5) ?? '', x, padTop + chartH + 18);
		});
	});

	// ── Canvas: Stock Additions Chart ──
	$effect(() => {
		const canvas = stockCanvas;
		if (!canvas) return;

		const style = getComputedStyle(document.documentElement);
		const color = style.getPropertyValue('--color-tertiary').trim() || '#a78bfa';
		const outline = style.getPropertyValue('--color-outline-variant').trim() || 'rgba(255,255,255,0.1)';
		const onSurface = style.getPropertyValue('--color-on-surface').trim() || '#e0e0e0';

		const dpr = window.devicePixelRatio || 1;
		const rect = canvas.getBoundingClientRect();
		canvas.width = rect.width * dpr;
		canvas.height = rect.height * dpr;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		ctx.scale(dpr, dpr);

		const W = rect.width, H = rect.height;
		const padTop = 16, padBottom = 32, padLeft = 36, padRight = 16;
		const chartW = W - padLeft - padRight;
		const chartH = H - padTop - padBottom;

		ctx.clearRect(0, 0, W, H);

		const days = buildDays(30);
		const values = days.map((d) => stockBuckets[d] ?? 0);
		const maxVal = Math.max(...values, 1);
		const barW = Math.max(2, chartW / days.length - 2);

		// Axis
		ctx.beginPath(); ctx.strokeStyle = outline; ctx.lineWidth = 1;
		ctx.moveTo(padLeft, padTop); ctx.lineTo(padLeft, padTop + chartH);
		ctx.lineTo(padLeft + chartW, padTop + chartH); ctx.stroke();

		// Y ticks
		ctx.fillStyle = onSurface; ctx.globalAlpha = 0.4;
		ctx.font = '10px system-ui'; ctx.textAlign = 'right';
		for (let tick = 0; tick <= 3; tick++) {
			const val = Math.round((maxVal * tick) / 3);
			const y = padTop + chartH - (chartH * tick) / 3;
			ctx.fillText(String(val), padLeft - 4, y + 4);
		}

		// Bars
		ctx.globalAlpha = 1;
		values.forEach((val, i) => {
			if (val === 0) return;
			const bh = (val / maxVal) * chartH;
			const x = padLeft + (i / days.length) * chartW;
			const y = padTop + chartH - bh;
			ctx.fillStyle = color;
			ctx.globalAlpha = 0.75;
			ctx.beginPath();
			ctx.roundRect(x, y, barW, bh, [3, 3, 0, 0]);
			ctx.fill();
		});

		// X labels
		ctx.globalAlpha = 0.4; ctx.fillStyle = onSurface; ctx.textAlign = 'center';
		[0, 7, 14, 21, 29].forEach((i) => {
			const x = padLeft + (i / (days.length - 1)) * chartW;
			ctx.fillText(days[i]?.slice(5) ?? '', x, padTop + chartH + 18);
		});
	});
</script>

<div class="analytics-page">
	<h1>{$t('page_analytics_title')}</h1>

	<!-- ── Date range filter ─────────────────────────────────── -->
	<div class="date-filter">
		<span class="date-filter-label">Период:</span>
		<input class="date-input" type="date" bind:value={dateFrom} title="С даты" />
		<span style="color:var(--color-outline)">—</span>
		<input class="date-input" type="date" bind:value={dateTo} title="По дату" />
		{#if dateFrom || dateTo}
			<button class="btn-ghost-sm" onclick={() => { dateFrom = ''; dateTo = ''; }}>✕ Сброс</button>
		{/if}
	</div>

	<!-- ── KPI Overview ─────────────────────────────────────── -->
	<div class="overview-grid">
		<div class="metric-card accent">
			<span class="metric-label">{$t('stat_total_revenue')}</span>
			<span class="metric-value">{fmt($totalRevenue)}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">{$t('stat_total_items')}</span>
			<span class="metric-value">{$totalItems}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">{$t('stat_total_stock')}</span>
			<span class="metric-value">{$totalStock}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">Себест. склада</span>
			<span class="metric-value">{fmt(warehouseStats.totalCost)}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">Стоимость склада</span>
			<span class="metric-value">{fmt(warehouseStats.totalValue)}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">Ср. маржа</span>
			<span class="metric-value">{warehouseStats.avgMargin}%</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">Продано всего</span>
			<span class="metric-value">{warehouseStats.totalSold}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">Активных заказов</span>
			<span class="metric-value">{orderStats.pending + orderStats.inProgress}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">Сумма в работе</span>
			<span class="metric-value">{fmt(orderStats.pendingAmount)}</span>
		</div>
	</div>

	<!-- ── Orders breakdown ──────────────────────────────────── -->
	{#if orderStats.total > 0}
	<section class="section">
		<h2>Заказы</h2>
		<div class="orders-status-grid">
			<div class="order-status-card status-pending">
				<span class="ost-num">{orderStats.pending}</span>
				<span class="ost-lbl">Ожидают</span>
			</div>
			<div class="order-status-card status-inprogress">
				<span class="ost-num">{orderStats.inProgress}</span>
				<span class="ost-lbl">В работе</span>
			</div>
			<div class="order-status-card status-completed">
				<span class="ost-num">{orderStats.completed}</span>
				<span class="ost-lbl">Выполнено</span>
			</div>
			<div class="order-status-card status-cancelled">
				<span class="ost-num">{orderStats.cancelled}</span>
				<span class="ost-lbl">Отменено</span>
			</div>
		</div>
		<div class="orders-amounts">
			<div class="amount-row">
				<span class="amount-lbl">Выручка (выполн.):</span>
				<span class="amount-val color-primary">{fmt(orderStats.completedAmount)}</span>
			</div>
			<div class="amount-row">
				<span class="amount-lbl">В ожидании:</span>
				<span class="amount-val">{fmt(orderStats.pendingAmount)}</span>
			</div>
			<div class="amount-row">
				<span class="amount-lbl">Всего по заказам:</span>
				<span class="amount-val">{fmt(orderStats.totalAmount)}</span>
			</div>
		</div>
	</section>
	{/if}

	<!-- ── Flowers summary (preset only) ─────────────────────── -->
	{#if $preset === 'flowers' && $flowerSorts.length > 0}
	<section class="section">
		<h2>🌸 Цветочный склад</h2>
		<div class="overview-grid">
			<div class="metric-card">
				<span class="metric-label">Сортов</span>
				<span class="metric-value">{$flowerSorts.length}</span>
			</div>
			<div class="metric-card">
				<span class="metric-label">Стеблей</span>
				<span class="metric-value">{$flowerSorts.reduce((s, f) => s + f.raw_stock, 0)}</span>
			</div>
			<div class="metric-card">
				<span class="metric-label">Упаковок</span>
				<span class="metric-value">{$flowerSorts.reduce((s, f) => s + f.pkg_stock, 0)}</span>
			</div>
		</div>
	</section>
	{/if}

	<!-- Charts row -->
	<div class="charts-row">
		<div class="chart-card">
			<h2>Sales Velocity — Last 30 Days</h2>
			<canvas bind:this={salesCanvas} class="chart-canvas" style="width:100%;height:200px;"></canvas>
		</div>
		<div class="chart-card">
			<h2>Revenue by Category</h2>
			<canvas bind:this={revenueCanvas} class="chart-canvas" style="width:100%;height:200px;"></canvas>
		</div>
	</div>

	{#if categoryStats.length > 0}
		<section class="section">
			<h2>{$t('table_header_category')}</h2>
			<div class="table-wrapper">
				<table>
					<thead>
						<tr>
							<th>{$t('table_header_category')}</th>
							<th>{$t('table_header_item')}</th>
							<th>{$t('table_header_stock')}</th>
							<th>{$t('table_header_sold')}</th>
							<th>{$t('table_header_revenue')}</th>
						</tr>
					</thead>
					<tbody>
						{#each categoryStats as [category, stats]}
							<tr>
								<td>{category}</td>
								<td>{stats.count}</td>
								<td>{stats.stock}</td>
								<td>{stats.sold}</td>
								<td>{fmt(stats.revenue)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{/if}

	{#if topSellers.length > 0}
		<section class="section">
			<h2>Top Sellers</h2>
			<div class="table-wrapper">
				<table>
					<thead>
						<tr>
							<th>{$t('table_header_item')}</th>
							<th>{$t('table_header_category')}</th>
							<th>{$t('table_header_sold')}</th>
							<th>{$t('table_header_revenue')}</th>
						</tr>
					</thead>
					<tbody>
						{#each topSellers as item}
							<tr>
								<td>{item.name}</td>
								<td>{item.category}</td>
								<td>{item.sold_count}</td>
								<td>{fmt(item.revenue)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{/if}

	<!-- Supply Run-Rate Table -->
	{#if runRate.length > 0}
		<section class="section">
			<h2>Supply Run-Rate</h2>
			<div class="table-wrapper">
				<table>
					<thead>
						<tr>
							<th>{$t('table_header_item')}</th>
							<th>{$t('table_header_stock')}</th>
							<th>Weeks Remaining</th>
						</tr>
					</thead>
					<tbody>
						{#each runRate as item}
							<tr>
								<td>{item.name}</td>
								<td>{item.stock}</td>
								<td>
									<span
										class="weeks-badge"
										class:green={item.weeksLeft > 4}
										class:yellow={item.weeksLeft > 1 && item.weeksLeft <= 4}
										class:red={item.weeksLeft <= 1}
									>
										{item.weeksLeft === Infinity ? '∞' : item.weeksLeft.toFixed(1)}w
									</span>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{/if}
</div>

<style>
	.analytics-page { max-width: 1000px; margin: 0 auto; }

	h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 24px;
	}

	h2 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 12px;
	}

	.overview-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 16px;
		margin-bottom: 32px;
	}

	.metric-card {
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.metric-card.accent { border-color: var(--color-primary); }

	.metric-label {
		font-size: 0.75rem;
		color: var(--color-on-surface);
		opacity: 0.5;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.metric-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-on-surface);
	}

	.charts-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
		margin-bottom: 32px;
	}

	.chart-card {
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
		padding: 16px;
	}

	.chart-canvas {
		display: block;
	}

	.section { margin-bottom: 32px; }

	.table-wrapper { overflow-x: auto; }

	table { width: 100%; border-collapse: collapse; }

	th, td {
		padding: 10px 14px;
		text-align: left;
		border-bottom: 1px solid var(--color-outline-variant);
	}

	th {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-on-surface);
		opacity: 0.5;
	}

	td {
		font-size: 0.875rem;
		color: var(--color-on-surface);
	}

	.weeks-badge {
		display: inline-block;
		padding: 2px 10px;
		border-radius: 999px;
		font-size: 0.8rem;
		font-weight: 600;
	}

	.weeks-badge.green { background: rgba(16, 185, 129, 0.15); color: #10b981; }
	.weeks-badge.yellow { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
	.weeks-badge.red { background: rgba(239, 68, 68, 0.15); color: #ef4444; }

	/* ── Orders breakdown ── */
	.orders-status-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 10px;
		margin-bottom: 16px;
	}

	@media (max-width: 640px) {
		.orders-status-grid { grid-template-columns: repeat(2, 1fr); }
	}

	.order-status-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		padding: 14px;
		border-radius: 12px;
		border: 1px solid var(--color-outline-variant);
	}

	.status-pending    { border-color: rgba(251,191,36,0.3);  background: rgba(251,191,36,0.06);  }
	.status-inprogress { border-color: rgba(96,165,250,0.3);  background: rgba(96,165,250,0.06);  }
	.status-completed  { border-color: rgba(52,211,153,0.3);  background: rgba(52,211,153,0.06);  }
	.status-cancelled  { border-color: rgba(248,113,113,0.3); background: rgba(248,113,113,0.06); }

	.status-pending    .ost-num { color: #fbbf24; }
	.status-inprogress .ost-num { color: #60a5fa; }
	.status-completed  .ost-num { color: #34d399; }
	.status-cancelled  .ost-num { color: #f87171; }

	.ost-num { font-size: 1.6rem; font-weight: 700; letter-spacing: -0.03em; }
	.ost-lbl { font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.07em; color: var(--color-outline); }

	.orders-amounts {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 12px 16px;
		background: var(--color-surface-container);
		border-radius: 10px;
	}

	.amount-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: 0.875rem;
	}

	.amount-lbl { color: var(--color-on-surface); opacity: 0.6; }
	.amount-val { font-weight: 600; color: var(--color-on-surface); }
	.color-primary { color: var(--color-primary); }

	@media (max-width: 640px) {
		.charts-row { grid-template-columns: 1fr; }
		.overview-grid { grid-template-columns: repeat(2, 1fr); }
	}
</style>
