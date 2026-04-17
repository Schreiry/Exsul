<script lang="ts">
	import { inventory, totalRevenue, totalStock, totalItems } from '$lib/stores/inventory';
	import { orders } from '$lib/stores/orders';
	import { flowerSorts, totalRawStems, totalPacks, flowerFinancials, flowerConstants } from '$lib/stores/flowers';
	import { preset } from '$lib/stores/preset';
	import { commands } from '$lib/tauri/commands';
	import { t } from '$lib/stores/i18n';
	import { showDetailedPricing } from '$lib/stores/appSettings';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { hlcToDate } from '$lib/utils/time';
	import HarvestTimeline from '$lib/components/charts/HarvestTimeline.svelte';
	import PackagingEfficiency from '$lib/components/charts/PackagingEfficiency.svelte';
	import type { PackagingLogEntry, HarvestLogEntry } from '$lib/tauri/types';

	const TZ = 'Asia/Tbilisi';

	// ── Date range filter ─────────────────────────────────────
	let dateFrom = $state('');
	let dateTo = $state('');

	// ── Search & sort ─────────────────────────────────────────
	let searchQuery = $state('');
	let catSortKey = $state<'category' | 'count' | 'stock' | 'sold' | 'revenue'>('revenue');
	let catSortDir = $state<'asc' | 'desc'>('desc');
	let topSortKey = $state<'name' | 'category' | 'sold_count' | 'revenue'>('sold_count');
	let topSortDir = $state<'asc' | 'desc'>('desc');

	function toggleCatSort(key: typeof catSortKey) {
		if (catSortKey === key) catSortDir = catSortDir === 'asc' ? 'desc' : 'asc';
		else { catSortKey = key; catSortDir = 'desc'; }
	}

	function toggleTopSort(key: typeof topSortKey) {
		if (topSortKey === key) topSortDir = topSortDir === 'asc' ? 'desc' : 'asc';
		else { topSortKey = key; topSortDir = 'desc'; }
	}

	// ── Tabs ─────────────────────────────────────────────────
	type AnalyticsTab = 'orders' | 'greenhouse' | 'warehouse' | 'summary';
	let activeTab = $state<AnalyticsTab>($preset === 'flowers' ? 'summary' : 'orders');

	// ── Assembly dynamics (flowers) ───────────────────────────
	let packagingLog = $state<PackagingLogEntry[]>([]);
	let harvestLog = $state<HarvestLogEntry[]>([]);
	let harvestLoading = $state(false);

	$effect(() => {
		orders.load();
		if ($preset === 'flowers') {
			flowerSorts.load();
			commands.getPackagingLog(200).then((log) => (packagingLog = log)).catch(() => {});
			harvestLoading = true;
			commands.getHarvestLog(undefined, 500)
				.then((log) => { harvestLog = log; })
				.catch(() => {})
				.finally(() => { harvestLoading = false; });
		}
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

	// ── Inventory / Warehouse analytics ───────────────────────
	const warehouseStats = $derived.by(() => {
		if ($preset === 'flowers' && $flowerSorts.length > 0) {
			const fin = $flowerFinancials;
			const margin = fin.totalValue > 0
				? Math.round(((fin.totalValue - fin.totalPurchaseValue) / fin.totalValue) * 100)
				: 0;
			return {
				totalCost: fin.totalPurchaseValue,
				totalValue: fin.totalValue,
				avgMargin: margin,
				totalSold: packagingLog.reduce((s, e) => s + e.pack_count, 0),
			};
		}
		const stock = $inventory.reduce((s, i) => s + i.current_stock, 0);
		const weighted = stock === 0 ? 0 : $inventory.reduce((s, i) => {
			const m = i.current_price > 0
				? (i.current_price - i.production_cost) / i.current_price : 0;
			return s + m * i.current_stock;
		}, 0);
		return {
			totalCost: $inventory.reduce((s, i) => s + i.production_cost * i.current_stock, 0),
			totalValue: $inventory.reduce((s, i) => s + i.current_price * i.current_stock, 0),
			avgMargin: stock === 0 ? 0 : Math.round((weighted / stock) * 100),
			totalSold: $inventory.reduce((s, i) => s + i.sold_count, 0),
		};
	});

	const sq = $derived(searchQuery.toLowerCase().trim());

	// Packaging history aggregated by sort_id (actual packed count, not current stock)
	const packBySort = $derived(
		packagingLog.reduce((acc, e) => {
			acc[e.sort_id] = (acc[e.sort_id] ?? 0) + e.pack_count;
			return acc;
		}, {} as Record<string, number>)
	);

	let categoryStats = $derived.by(() => {
		let entries: [string, { count: number; stock: number; revenue: number; sold: number }][];

		if ($preset === 'flowers' && $flowerSorts.length > 0) {
			const c = $flowerConstants;
			const map: Record<string, { count: number; stock: number; revenue: number; sold: number }> = {};
			for (const s of $flowerSorts) {
				if (!map[s.name]) map[s.name] = { count: 0, stock: 0, revenue: 0, sold: 0 };
				const fpp = s.flowers_per_pack_override ?? c.flowers_per_pack;
				map[s.name].count++;
				map[s.name].stock += s.raw_stock + s.pkg_stock;
				map[s.name].revenue += s.pkg_stock * fpp * s.sell_price_stem + s.raw_stock * s.sell_price_stem;
				map[s.name].sold += packBySort[s.id] ?? 0;
			}
			entries = Object.entries(map);
		} else {
			entries = Object.entries(
				$inventory.reduce(
					(acc, item) => {
						if (!acc[item.category]) acc[item.category] = { count: 0, stock: 0, revenue: 0, sold: 0 };
						acc[item.category].count++;
						acc[item.category].stock += item.current_stock;
						acc[item.category].revenue += item.revenue;
						acc[item.category].sold += item.sold_count;
						return acc;
					},
					{} as Record<string, { count: number; stock: number; revenue: number; sold: number }>
				)
			);
		}

		const filtered = sq ? entries.filter(([cat]) => cat.toLowerCase().includes(sq)) : entries;
		return filtered.sort((a, b) => {
			const va = catSortKey === 'category' ? a[0] : a[1][catSortKey];
			const vb = catSortKey === 'category' ? b[0] : b[1][catSortKey];
			const cmp = typeof va === 'string' ? va.localeCompare(vb as string) : (va as number) - (vb as number);
			return catSortDir === 'asc' ? cmp : -cmp;
		});
	});

	let topSellers = $derived.by(() => {
		if ($preset === 'flowers' && $flowerSorts.length > 0) {
			const c = $flowerConstants;
			let items = [...$flowerSorts]
				.map(s => {
					const fpp = s.flowers_per_pack_override ?? c.flowers_per_pack;
					const sold = packBySort[s.id] ?? 0;
					return {
						name: `${s.name}${s.variety ? ' — ' + s.variety : ''}`,
						category: s.name,
						sold_count: sold,
						revenue: sold * fpp * s.sell_price_stem,
					};
				})
				.sort((a, b) => b.sold_count - a.sold_count)
				.slice(0, 20);
			if (sq) items = items.filter(i => i.name.toLowerCase().includes(sq) || i.category.toLowerCase().includes(sq));
			return items.sort((a, b) => {
				const va = a[topSortKey as keyof typeof a];
				const vb = b[topSortKey as keyof typeof b];
				const cmp = typeof va === 'string' ? va.localeCompare(vb as string) : (va as number) - (vb as number);
				return topSortDir === 'asc' ? cmp : -cmp;
			});
		}
		let items = [...$inventory].sort((a, b) => b.sold_count - a.sold_count).slice(0, 20);
		if (sq) items = items.filter(i => i.name.toLowerCase().includes(sq) || i.category.toLowerCase().includes(sq));
		return items.sort((a, b) => {
			const va = a[topSortKey];
			const vb = b[topSortKey];
			const cmp = typeof va === 'string' ? va.localeCompare(vb as string) : (va as number) - (vb as number);
			return topSortDir === 'asc' ? cmp : -cmp;
		});
	});

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

	<!-- ── Tab navigation (flowers preset) ─────────────────── -->
	{#if $preset === 'flowers'}
		<div class="tab-nav">
			<button type="button" class="tab-btn" class:active={activeTab === 'summary'}    onclick={() => (activeTab = 'summary')}>Сводка</button>
			<button type="button" class="tab-btn" class:active={activeTab === 'orders'}     onclick={() => (activeTab = 'orders')}>Заказы</button>
			<button type="button" class="tab-btn" class:active={activeTab === 'greenhouse'} onclick={() => (activeTab = 'greenhouse')}>Оранжерея</button>
			<button type="button" class="tab-btn" class:active={activeTab === 'warehouse'}  onclick={() => (activeTab = 'warehouse')}>Склад</button>
		</div>
	{/if}

	<!-- ══ ORDERS TAB (or all content for non-flowers) ══════════ -->
	{#if $preset !== 'flowers' || activeTab === 'orders'}
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
				<span class="metric-value">{fmt(orderStats.totalAmount)}</span>
			</div>
			<div class="metric-card">
				<span class="metric-label">Активных заказов</span>
				<span class="metric-value">{orderStats.pending + orderStats.inProgress}</span>
			</div>
			<div class="metric-card">
				<span class="metric-label">Сумма в работе</span>
				<span class="metric-value">{fmt(orderStats.pendingAmount)}</span>
			</div>
			<div class="metric-card">
				<span class="metric-label">Выполнено</span>
				<span class="metric-value">{fmt(orderStats.completedAmount)}</span>
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

		<!-- Charts row -->
		<div class="charts-row">
			<div class="chart-card">
				<h2>{$t('analytics_sales_velocity')}</h2>
				<canvas bind:this={salesCanvas} class="chart-canvas" style="width:100%;height:200px;"></canvas>
			</div>
			<div class="chart-card">
				<h2>{$t('analytics_revenue_by_category')}</h2>
				<canvas bind:this={revenueCanvas} class="chart-canvas" style="width:100%;height:200px;"></canvas>
			</div>
		</div>
	{/if}

	<!-- ══ WAREHOUSE TAB ════════════════════════════════════════ -->
	{#if $preset !== 'flowers' || activeTab === 'warehouse'}
		<!-- ── Flowers warehouse summary ─────────────────────── -->
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
				<div class="metric-card accent">
					<span class="metric-label">Стоимость склада</span>
					<span class="metric-value">{fmt(warehouseStats.totalValue)}</span>
				</div>
				{#if $showDetailedPricing}
				<div class="metric-card">
					<span class="metric-label">Себестоимость</span>
					<span class="metric-value">{fmt(warehouseStats.totalCost)}</span>
				</div>
				<div class="metric-card">
					<span class="metric-label">Ср. маржа</span>
					<span class="metric-value">{warehouseStats.avgMargin}%</span>
				</div>
				{/if}
			</div>
		</section>
		{/if}

		<!-- Search -->
		<div class="search-row">
			<input class="search-input" type="text" bind:value={searchQuery} placeholder={$t('analytics_search_placeholder')} />
		</div>

		{#if categoryStats.length > 0}
			<section class="section">
				<h2>{$t('table_header_category')}</h2>
				<div class="table-wrapper">
					<table>
						<thead>
							<tr>
								<th class="sortable" onclick={() => toggleCatSort('category')}>{$t('table_header_category')} {catSortKey === 'category' ? (catSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleCatSort('count')}>{$t('table_header_item')} {catSortKey === 'count' ? (catSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleCatSort('stock')}>{$t('table_header_stock')} {catSortKey === 'stock' ? (catSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleCatSort('sold')}>{$t('table_header_sold')} {catSortKey === 'sold' ? (catSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleCatSort('revenue')}>{$t('table_header_revenue')} {catSortKey === 'revenue' ? (catSortDir === 'asc' ? '↑' : '↓') : ''}</th>
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
				<h2>{$t('analytics_top_sellers')}</h2>
				<div class="table-wrapper">
					<table>
						<thead>
							<tr>
								<th class="sortable" onclick={() => toggleTopSort('name')}>{$t('table_header_item')} {topSortKey === 'name' ? (topSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleTopSort('category')}>{$t('table_header_category')} {topSortKey === 'category' ? (topSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleTopSort('sold_count')}>{$t('table_header_sold')} {topSortKey === 'sold_count' ? (topSortDir === 'asc' ? '↑' : '↓') : ''}</th>
								<th class="sortable" onclick={() => toggleTopSort('revenue')}>{$t('table_header_revenue')} {topSortKey === 'revenue' ? (topSortDir === 'asc' ? '↑' : '↓') : ''}</th>
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
				<h2>{$t('analytics_run_rate')}</h2>
				<div class="table-wrapper">
					<table>
						<thead>
							<tr>
								<th>{$t('table_header_item')}</th>
								<th>{$t('table_header_stock')}</th>
								<th>{$t('analytics_weeks_remaining')}</th>
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

		<!-- Packaging KPIs (flowers) -->
		{#if $preset === 'flowers'}
			<div class="tab-section">
				<h2 class="tab-section-title">Эффективность упаковки</h2>
				<div class="gh-kpi-row">
					<div class="gh-kpi">
						<span class="gh-kpi-val">{packagingLog.reduce((s,e) => s + e.pack_count, 0)}</span>
						<span class="gh-kpi-label">Всего упаковок</span>
					</div>
					<div class="gh-kpi">
						<span class="gh-kpi-val">{packagingLog.reduce((s,e) => s + e.stems_used, 0)}</span>
						<span class="gh-kpi-label">Стеблей использовано</span>
					</div>
					<div class="gh-kpi">
						<span class="gh-kpi-val">{packagingLog.length > 0
							? Math.round(packagingLog.reduce((s,e) => s + e.stems_used, 0) / packagingLog.reduce((s,e) => s + e.pack_count, 1))
							: 0}</span>
						<span class="gh-kpi-label">Ср. стеблей/упак.</span>
					</div>
				</div>
				<div class="chart-card">
					<div class="chart-header">
						<span class="chart-title">Упаковка по неделям (последние 8 нед.)</span>
					</div>
					<PackagingEfficiency log={packagingLog} weeks={8} />
				</div>
				{#if packagingLog.length > 0}
					<div class="sort-breakdown">
						<h3 class="section-sub">Журнал упаковки</h3>
						<div class="sort-table-wrap">
							<table class="sort-table">
								<thead>
									<tr><th>Сорт</th><th>Упаковок</th><th>Стеблей</th><th>Дата</th></tr>
								</thead>
								<tbody>
									{#each packagingLog.slice(0, 50) as e (e.id)}
										<tr>
											<td class="sort-name-cell">{e.sort_name}</td>
											<td class="num">{e.pack_count}</td>
											<td class="num">{e.stems_used}</td>
											<td class="muted">{new Date(e.created_at).toLocaleDateString('ru')}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</div>
				{/if}
			</div>
		{/if}
	{/if}

	<!-- ══ GREENHOUSE TAB ════════════════════════════════════════ -->
	{#if $preset === 'flowers' && activeTab === 'greenhouse'}
		<div class="tab-section">
			<h2 class="tab-section-title">Оранжерея — сбор урожая</h2>

			<!-- Summary KPIs -->
			<div class="gh-kpi-row">
				<div class="gh-kpi">
					<span class="gh-kpi-val">{$flowerSorts.reduce((s,x) => s + x.raw_stock, 0)}</span>
					<span class="gh-kpi-label">Стеблей сейчас</span>
				</div>
				<div class="gh-kpi">
					<span class="gh-kpi-val">{$flowerSorts.reduce((s,x) => s + (x.total_harvested ?? 0), 0)}</span>
					<span class="gh-kpi-label">Всего собрано</span>
				</div>
				<div class="gh-kpi">
					<span class="gh-kpi-val">{$flowerSorts.length}</span>
					<span class="gh-kpi-label">Видов сырья</span>
				</div>
			</div>

			<!-- Harvest timeline chart -->
			<div class="chart-card">
				<div class="chart-header">
					<span class="chart-title">Сбор за 30 дней</span>
					<span class="chart-sub">{harvestLog.length} записей</span>
				</div>
				{#if harvestLoading}
					<p class="loading-msg">Загрузка…</p>
				{:else}
					<HarvestTimeline entries={harvestLog} days={30} />
				{/if}
			</div>

			<!-- Per-sort breakdown table -->
			<div class="sort-breakdown">
				<h3 class="section-sub">По сортам</h3>
				{#if $flowerSorts.length === 0}
					<p class="empty-hint">Нет сырья в оранжерее</p>
				{:else}
					<div class="sort-table-wrap">
						<table class="sort-table">
							<thead>
								<tr>
									<th>Название</th>
									<th>Сорт</th>
									<th>Сейчас (шт.)</th>
									<th>Упаковано (уп.)</th>
									<th>Всего собрано</th>
									<th>Цена/шт.</th>
								</tr>
							</thead>
							<tbody>
								{#each [...$flowerSorts].sort((a,b) => b.total_harvested - a.total_harvested) as sort (sort.id)}
									<tr>
										<td class="sort-name-cell">{sort.name}</td>
										<td class="muted">{sort.variety ?? '—'}</td>
										<td class="num">{sort.raw_stock}</td>
										<td class="num">{sort.pkg_stock}</td>
										<td class="num primary">{sort.total_harvested}</td>
										<td class="num">{sort.sell_price_stem > 0 ? sort.sell_price_stem : '—'}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}

				<!-- Assembly Dynamics -->
				{#if packagingLog.length > 0}
					<div class="sort-breakdown">
						<h3 class="section-sub">Динамика сборки</h3>
						<div class="sort-table-wrap">
							<table class="sort-table">
								<thead>
									<tr>
										<th>Сорт</th>
										<th>Упаковок</th>
										<th>Стеблей</th>
										<th>Дата</th>
									</tr>
								</thead>
								<tbody>
									{#each packagingLog.slice(0, 30) as entry (entry.id)}
										<tr>
											<td class="sort-name-cell">{entry.sort_name}</td>
											<td class="num">{entry.pack_count}</td>
											<td class="num">{entry.stems_used}</td>
											<td class="muted">{new Date(entry.created_at).toLocaleDateString('ru')}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- ══ SUMMARY TAB ═══════════════════════════════════════════ -->
	{#if $preset === 'flowers' && activeTab === 'summary'}
		<div class="tab-section">
			<h2 class="tab-section-title">Сводка по всем модулям</h2>

			<div class="summary-grid">
				<!-- Greenhouse block -->
				<div class="summary-block">
					<div class="summary-block-header">
						<span class="summary-icon">
							<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round">
								<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12"/>
								<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(72 12 12)"/>
								<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(144 12 12)"/>
								<circle cx="12" cy="12" r="2"/>
							</svg>
						</span>
						<h3 class="summary-block-title">Оранжерея</h3>
					</div>
					<div class="summary-metrics">
						<div class="summary-metric">
							<span class="summary-metric-val">{$flowerSorts.reduce((s,x)=>s+x.raw_stock,0)}</span>
							<span class="summary-metric-label">стеблей сейчас</span>
						</div>
						<div class="summary-metric">
							<span class="summary-metric-val">{$flowerSorts.reduce((s,x)=>s+(x.total_harvested??0),0)}</span>
							<span class="summary-metric-label">всего собрано</span>
						</div>
					</div>
					<div class="summary-mini-chart">
						<HarvestTimeline entries={harvestLog} days={14} />
					</div>
				</div>

				<!-- Warehouse block -->
				<div class="summary-block">
					<div class="summary-block-header">
						<span class="summary-icon">
							<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/></svg>
						</span>
						<h3 class="summary-block-title">Склад</h3>
					</div>
					<div class="summary-metrics">
						<div class="summary-metric">
							<span class="summary-metric-val">{$flowerSorts.reduce((s,x)=>s+x.pkg_stock,0)}</span>
							<span class="summary-metric-label">упак. готово</span>
						</div>
						<div class="summary-metric">
							<span class="summary-metric-val">{packagingLog.reduce((s,e)=>s+e.pack_count,0)}</span>
							<span class="summary-metric-label">всего упаковано</span>
						</div>
					</div>
					<div class="summary-mini-chart">
						<PackagingEfficiency log={packagingLog} weeks={6} />
					</div>
				</div>

				<!-- Orders block -->
				<div class="summary-block">
					<div class="summary-block-header">
						<span class="summary-icon">
							<svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M9 11l3 3L22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/></svg>
						</span>
						<h3 class="summary-block-title">Заказы</h3>
					</div>
					<div class="summary-metrics">
						<div class="summary-metric">
							<span class="summary-metric-val">{orderStats.total}</span>
							<span class="summary-metric-label">всего заказов</span>
						</div>
						<div class="summary-metric primary">
							<span class="summary-metric-val">{fmt(orderStats.totalAmount)}</span>
							<span class="summary-metric-label">общая сумма</span>
						</div>
					</div>
					<div class="summary-status-row">
						<span class="status-pill pending">{orderStats.pending} ожидает</span>
						<span class="status-pill progress">{orderStats.inProgress} в работе</span>
						<span class="status-pill done">{orderStats.completed} готово</span>
					</div>
				</div>
			</div>
		</div>
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

	/* ── Tab navigation ── */
	.tab-nav {
		display: flex;
		gap: 4px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 14px;
		padding: 4px;
		margin-bottom: 24px;
		width: fit-content;
	}

	.tab-btn {
		background: none; border: none;
		border-radius: 10px;
		padding: 7px 16px;
		font-size: 0.85rem;
		color: var(--color-outline);
		cursor: pointer;
		transition: background 0.12s, color 0.12s;
		white-space: nowrap;
	}
	.tab-btn:hover { color: var(--color-on-surface); }
	.tab-btn.active {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		font-weight: 600;
	}

	/* ── Tab sections ── */
	.tab-section { display: flex; flex-direction: column; gap: 20px; }
	.tab-section-title { font-size: 1.1rem; font-weight: 700; margin: 0; letter-spacing: -0.02em; color: var(--color-on-surface); }
	.section-sub { font-size: 0.82rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-outline); margin: 0; }
	.loading-msg { font-size: 0.85rem; color: var(--color-outline); margin: 0; padding: 24px 0; text-align: center; }
	.empty-hint { font-size: 0.85rem; color: var(--color-outline); margin: 0; }

	/* ── Greenhouse KPIs ── */
	.gh-kpi-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
	.gh-kpi {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 14px; padding: 14px 16px;
		display: flex; flex-direction: column; gap: 3px;
	}
	.gh-kpi-val { font-size: 1.7rem; font-weight: 700; color: var(--color-primary); line-height: 1; }
	.gh-kpi-label { font-size: 0.72rem; color: var(--color-outline); }

	/* ── Chart cards ── */
	.chart-card {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 14px; padding: 16px; overflow: hidden;
	}
	.chart-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
	.chart-title { font-size: 0.88rem; font-weight: 600; color: var(--color-on-surface); }
	.chart-sub { font-size: 0.75rem; color: var(--color-outline); }

	/* ── Sort table ── */
	.sort-breakdown { display: flex; flex-direction: column; gap: 10px; }
	.sort-table-wrap { overflow-x: auto; }
	.sort-table {
		width: 100%; border-collapse: collapse;
		font-size: 0.85rem;
	}
	.sort-table th {
		text-align: left; padding: 8px 12px;
		font-size: 0.72rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;
		color: var(--color-outline); border-bottom: 1px solid var(--glass-border);
	}
	.sort-table td {
		padding: 8px 12px;
		border-bottom: 1px solid var(--glass-border);
		color: var(--color-on-surface);
	}
	.sort-table tr:last-child td { border-bottom: none; }
	.sort-table tr:hover td { background: var(--glass-bg-hover); }
	.sort-name-cell { font-weight: 600; }
	.num { text-align: right; font-variant-numeric: tabular-nums; }
	.num.primary { color: var(--color-primary); font-weight: 600; }
	.muted { color: var(--color-outline); }

	/* ── Summary grid ── */
	.summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 16px; }
	.summary-block {
		background: var(--glass-bg); border: 1px solid var(--glass-border);
		border-radius: 16px; padding: 16px;
		display: flex; flex-direction: column; gap: 12px;
	}
	.summary-block-header { display: flex; align-items: center; gap: 8px; }
	.summary-icon { color: var(--color-primary); display: flex; }
	.summary-block-title { font-size: 0.9rem; font-weight: 700; margin: 0; color: var(--color-on-surface); }
	.summary-metrics { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
	.summary-metric { display: flex; flex-direction: column; gap: 2px; }
	.summary-metric.primary .summary-metric-val { color: var(--color-primary); }
	.summary-metric-val { font-size: 1.3rem; font-weight: 700; color: var(--color-on-surface); line-height: 1; }
	.summary-metric-label { font-size: 0.68rem; color: var(--color-outline); }
	.summary-mini-chart { height: 100px; overflow: hidden; border-radius: 8px; }
	.summary-status-row { display: flex; gap: 6px; flex-wrap: wrap; }
	.status-pill {
		font-size: 0.7rem; padding: 3px 8px;
		border-radius: 20px; font-weight: 500;
	}
	.status-pill.pending { background: rgba(245,158,11,0.12); color: #f59e0b; border: 1px solid rgba(245,158,11,0.3); }
	.status-pill.progress { background: rgba(59,130,246,0.12); color: #3b82f6; border: 1px solid rgba(59,130,246,0.3); }
	.status-pill.done { background: rgba(16,185,129,0.12); color: #10b981; border: 1px solid rgba(16,185,129,0.3); }

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

	.chart-canvas {
		display: block;
		max-width: 100%;
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

	/* ── Date filter ── */
	.date-filter {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 20px;
		flex-wrap: wrap;
	}

	.date-filter-label {
		font-size: 0.85rem;
		color: var(--color-outline);
	}

	.date-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 6px 10px;
		color: var(--color-on-surface);
		font-size: 0.85rem;
		font-family: inherit;
		outline: none;
	}

	.date-input:focus { border-color: var(--color-primary); }

	.btn-ghost-sm {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 8px;
		padding: 5px 10px;
		color: var(--color-on-surface);
		font-size: 0.8rem;
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-ghost-sm:hover { background: var(--glass-bg-hover); }

	/* ── Search ── */
	.search-row {
		margin-bottom: 20px;
	}

	.search-input {
		width: 100%;
		max-width: 400px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 9px;
		padding: 9px 14px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		transition: border-color 0.15s;
	}

	.search-input:focus { border-color: var(--color-primary); }

	/* ── Sortable headers ── */
	.sortable {
		cursor: pointer;
		user-select: none;
		transition: color 0.15s;
	}

	.sortable:hover { color: var(--color-primary); opacity: 1; }

	@media (max-width: 640px) {
		.charts-row { grid-template-columns: 1fr; }
		.overview-grid { grid-template-columns: repeat(2, 1fr); }
	}
</style>
