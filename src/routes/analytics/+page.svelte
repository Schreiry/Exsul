<script lang="ts">
	import { inventory, totalRevenue, totalStock, totalItems } from '$lib/stores/inventory';
	import { commands } from '$lib/tauri/commands';
	import { t } from '$lib/stores/i18n';

	function formatCurrency(value: number): string {
		return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(value);
	}

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

	type DailyBucket = Record<string, number>;

	async function loadSalesBuckets(): Promise<DailyBucket> {
		const events = await commands.getEvents(undefined, 500);
		const buckets: DailyBucket = {};
		for (const ev of events) {
			if (ev.event_type !== 'SaleRecorded') continue;
			// HLC timestamp format: "{ms}:{counter}:{nodeId}" — extract ms prefix
			const ms = parseInt(ev.hlc_timestamp.split(':')[0], 10);
			if (isNaN(ms)) continue;
			const day = new Date(ms).toISOString().slice(0, 10);
			const qty = (ev.data as { quantity?: number }).quantity ?? 0;
			buckets[day] = (buckets[day] ?? 0) + qty;
		}
		return buckets;
	}

	let salesBuckets = $state<DailyBucket>({});

	$effect(() => {
		loadSalesBuckets().then((b) => (salesBuckets = b));
	});

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

		// Build last 30 days
		const days: string[] = [];
		for (let i = 29; i >= 0; i--) {
			const d = new Date(Date.now() - i * 86400000);
			days.push(d.toISOString().slice(0, 10));
		}

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
</script>

<div class="analytics-page">
	<h1>{$t('page_analytics_title')}</h1>

	<div class="overview-grid">
		<div class="metric-card accent">
			<span class="metric-label">{$t('stat_total_revenue')}</span>
			<span class="metric-value">{formatCurrency($totalRevenue)}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">{$t('stat_total_items')}</span>
			<span class="metric-value">{$totalItems}</span>
		</div>
		<div class="metric-card">
			<span class="metric-label">{$t('stat_total_stock')}</span>
			<span class="metric-value">{$totalStock}</span>
		</div>
	</div>

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
								<td>{formatCurrency(stats.revenue)}</td>
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
								<td>{formatCurrency(item.revenue)}</td>
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

	@media (max-width: 640px) {
		.charts-row { grid-template-columns: 1fr; }
	}
</style>
