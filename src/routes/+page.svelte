<script lang="ts">
	import { inventory, totalStock, totalRevenue, totalItems } from '$lib/stores/inventory';
	import { preset } from '$lib/stores/preset';
	import { flowerSorts, totalRawStems, totalPacks, flowerFinancials, flowerConstants } from '$lib/stores/flowers';
	import { orders } from '$lib/stores/orders';
	import { auditLog } from '$lib/stores/audit';
	import { nodeId, wsServerRunning, wsPeers, loadWsStatus } from '$lib/stores/sync';
	import { t } from '$lib/stores/i18n';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';

	type WidgetId = 'sync' | 'orders' | 'inventory' | 'chart' | 'activity';
	const ALL_WIDGETS: WidgetId[] = ['sync', 'orders', 'inventory', 'chart', 'activity'];

	function loadVisibleWidgets(): Set<WidgetId> {
		try {
			const raw = localStorage.getItem('dashboard-widgets');
			if (raw) return new Set(JSON.parse(raw) as WidgetId[]);
		} catch { /* ignore */ }
		return new Set(ALL_WIDGETS);
	}

	let visibleWidgets = $state(loadVisibleWidgets());
	let showWidgetMenu = $state(false);

	function toggleWidget(id: WidgetId) {
		const next = new Set(visibleWidgets);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		visibleWidgets = next;
		localStorage.setItem('dashboard-widgets', JSON.stringify([...next]));
	}

	const widgetLabels: Record<WidgetId, string> = {
		sync: $t('widget_sync'),
		orders: $t('widget_orders'),
		inventory: $t('widget_inventory'),
		chart: $t('widget_chart'),
		activity: $t('widget_activity'),
	};

	let chartCanvas = $state<HTMLCanvasElement | null>(null);

	$effect(() => {
		if ($preset === 'flowers') {
			flowerSorts.load();
		}
		orders.load();
		auditLog.load({ limit: 8 });
		loadWsStatus();
	});

	// Draw a sparkline bar chart using the top items' stock
	$effect(() => {
		if (!chartCanvas) return;
		const canvas = chartCanvas;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const dpr = window.devicePixelRatio || 1;
		const w = canvas.offsetWidth;
		const h = canvas.offsetHeight;
		canvas.width = w * dpr;
		canvas.height = h * dpr;
		ctx.scale(dpr, dpr);

		const isFlowers = $preset === 'flowers' && $flowerSorts.length > 0;
		const chartItems = isFlowers
			? $flowerSorts.slice(0, 8).map(s => ({ label: s.name, value: s.pkg_stock * $flowerConstants.price_per_pack }))
			: $inventory.slice(0, 8).map(i => ({ label: i.name, value: i.revenue }));
		if (chartItems.length === 0) {
			ctx.clearRect(0, 0, w, h);
			return;
		}

		const maxRev = Math.max(...chartItems.map((i) => i.value), 1);
		const style = getComputedStyle(canvas);
		const primary = style.getPropertyValue('--color-primary').trim() || '#34d399';
		const secondary = style.getPropertyValue('--color-secondary').trim() || '#5bb8d0';
		const outline = style.getPropertyValue('--color-outline').trim() || '#525252';

		ctx.clearRect(0, 0, w, h);

		const barW = (w - 24) / chartItems.length;
		const padH = 8;

		chartItems.forEach((item, i) => {
			const barH = Math.max(3, ((item.value / maxRev) * (h - padH * 2)));
			const x = 12 + i * barW + barW * 0.15;
			const bw = barW * 0.7;
			const y = h - padH - barH;

			const grad = ctx.createLinearGradient(x, y, x, h - padH);
			grad.addColorStop(0, primary);
			grad.addColorStop(1, secondary + '44');
			ctx.fillStyle = grad;

			const r = 4;
			ctx.beginPath();
			ctx.moveTo(x + r, y);
			ctx.lineTo(x + bw - r, y);
			ctx.quadraticCurveTo(x + bw, y, x + bw, y + r);
			ctx.lineTo(x + bw, h - padH);
			ctx.lineTo(x, h - padH);
			ctx.lineTo(x, y + r);
			ctx.quadraticCurveTo(x, y, x + r, y);
			ctx.closePath();
			ctx.fill();
		});

		// Axis line
		ctx.strokeStyle = outline + '33';
		ctx.lineWidth = 1;
		ctx.beginPath();
		ctx.moveTo(12, h - padH);
		ctx.lineTo(w - 12, h - padH);
		ctx.stroke();
	});

	function fmt(value: number): string {
		return formatAmount(value, $globalCurrency);
	}

	function truncateId(id: string): string {
		if (!id) return '—';
		return id.length > 16 ? id.slice(0, 8) + '…' + id.slice(-4) : id;
	}

	function formatTime(ts: string): string {
		try {
			return new Date(ts).toLocaleTimeString('ru', {
				timeZone: 'Asia/Tbilisi',
				hour: '2-digit',
				minute: '2-digit',
			});
		} catch {
			return ts;
		}
	}

	const statusColors: Record<string, string> = {
		pending: '#fbbf24',
		in_progress: '#60a5fa',
		completed: '#34d399',
		cancelled: '#f87171',
	};

	const pendingOrders = $derived($orders.filter((o) => o.status === 'pending' || o.status === 'in_progress'));
</script>

<div class="cc-page">
	<header class="cc-header">
		<div class="cc-title-row">
			<h1 class="cc-title">{$t('page_control_center_title')}</h1>
			<div class="cc-header-actions">
				<div class="widget-menu-wrap">
					<button class="widget-gear-btn" onclick={() => (showWidgetMenu = !showWidgetMenu)} aria-label={$t('dashboard_widget_toggle')}>
						⚙
					</button>
					{#if showWidgetMenu}
						<div class="widget-menu">
							{#each ALL_WIDGETS as wid}
								<label class="widget-toggle">
									<input type="checkbox" checked={visibleWidgets.has(wid)} onchange={() => toggleWidget(wid)} />
									<span>{widgetLabels[wid]}</span>
								</label>
							{/each}
						</div>
					{/if}
				</div>
				<div class="cc-node-badge" class:online={$wsServerRunning}>
					<span class="cc-node-dot"></span>
					<span class="cc-node-id">{truncateId($nodeId)}</span>
				</div>
			</div>
		</div>
	</header>

	<div class="bento-grid">

		<!-- ── SYNC STATUS ────────────────────────────────────── -->
		{#if visibleWidgets.has('sync')}
		<div class="bento-card bento-sync">
			<div class="bento-card-header accent-sync">
				<span class="bento-icon">⚡</span>
				<span class="bento-label">{$t('bento_sync_title')}</span>
			</div>
			<div class="bento-card-body">
				<div class="sync-status-row">
					<div class="sync-dot-wrap">
						<span class="sync-dot" class:sync-dot-on={$wsServerRunning}></span>
						<span class="sync-status-text">
							{$wsServerRunning ? $t('bento_node_online') : $t('bento_node_offline')}
						</span>
					</div>
					<span class="sync-peers-badge">
						{$t('bento_peers_connected', { n: String($wsPeers.length) })}
					</span>
				</div>
				<div class="sync-node-row">
					<span class="sync-node-label">Node ID</span>
					<code class="sync-node-value">{truncateId($nodeId)}</code>
				</div>
				{#if $wsPeers.length > 0}
					<div class="sync-peer-list">
						{#each $wsPeers.slice(0, 3) as peer}
							<div class="sync-peer-row">
								<span class="sync-peer-dot" style:background={peer.state === 'connected' ? '#34d399' : '#fbbf24'}></span>
								<span class="sync-peer-ip">{peer.ip}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		{/if}

		<!-- ── ORDERS ─────────────────────────────────────────── -->
		{#if visibleWidgets.has('orders')}
		<div class="bento-card bento-orders">
			<div class="bento-card-header accent-orders">
				<span class="bento-icon">📋</span>
				<span class="bento-label">{$t('bento_orders_title')}</span>
				<a href="/orders" class="bento-view-all">{$t('bento_view_all')}</a>
			</div>
			<div class="bento-card-body">
				{#if pendingOrders.length === 0}
					<p class="bento-empty">{$t('bento_no_orders')}</p>
				{:else}
					<div class="orders-kpi-row">
						<div class="orders-kpi">
							<span class="orders-kpi-num">{pendingOrders.length}</span>
							<span class="orders-kpi-lbl">активных</span>
						</div>
						<div class="orders-kpi">
							<span class="orders-kpi-num">{fmt(pendingOrders.reduce((s, o) => s + o.total_amount, 0))}</span>
							<span class="orders-kpi-lbl">сумма</span>
						</div>
					</div>
					<div class="order-mini-list">
						{#each pendingOrders.slice(0, 3) as order}
							<div class="order-mini-row">
								<span class="order-mini-name">{order.customer_name}</span>
								<span class="order-mini-status" style:color={statusColors[order.status] ?? '#e5e5e5'}>
									●
								</span>
								<span class="order-mini-amount">{fmt(order.total_amount)}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		{/if}

		<!-- ── INVENTORY SUMMARY ──────────────────────────────── -->
		{#if visibleWidgets.has('inventory')}
		<div class="bento-card bento-inventory">
			<div class="bento-card-header accent-inventory">
				<span class="bento-icon">📦</span>
				<span class="bento-label">{$t('bento_inventory_title')}</span>
				<a href="/inventory" class="bento-view-all">{$t('bento_view_all')}</a>
			</div>
			<div class="bento-card-body">
				{#if $preset === 'flowers' && $flowerSorts.length > 0}
					<div class="inv-kpi-grid">
						<div class="inv-kpi">
							<span class="inv-kpi-val">{$flowerSorts.length}</span>
							<span class="inv-kpi-lbl">сортов</span>
						</div>
						<div class="inv-kpi">
							<span class="inv-kpi-val">{$totalRawStems}</span>
							<span class="inv-kpi-lbl">стеблей</span>
						</div>
						<div class="inv-kpi">
							<span class="inv-kpi-val">{$totalPacks}</span>
							<span class="inv-kpi-lbl">упаковок</span>
						</div>
					</div>
					<div class="inv-kpi-grid" style="margin-top:6px">
						<div class="inv-kpi" style="grid-column: span 2">
							<span class="inv-kpi-val color-revenue">{fmt($flowerFinancials.totalValue)}</span>
							<span class="inv-kpi-lbl">стоимость склада</span>
						</div>
						<div class="inv-kpi">
							<span class="inv-kpi-val">{fmt($flowerFinancials.totalPurchaseValue)}</span>
							<span class="inv-kpi-lbl">себестоимость</span>
						</div>
					</div>
					<div class="inv-recent-list">
						{#each $flowerSorts.slice(0, 3) as sort}
							<div class="inv-recent-row">
								<div class="inv-recent-info">
									<span class="inv-recent-name">{sort.name}</span>
									<span class="inv-recent-cat">{sort.variety ?? ''}</span>
								</div>
								<span class="inv-recent-stock">📦 {sort.pkg_stock} · 🌸 {sort.raw_stock}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="inv-kpi-grid">
						<div class="inv-kpi">
							<span class="inv-kpi-val">{$totalItems}</span>
							<span class="inv-kpi-lbl">{$t('stat_total_items')}</span>
						</div>
						<div class="inv-kpi">
							<span class="inv-kpi-val">{$totalStock}</span>
							<span class="inv-kpi-lbl">{$t('stat_total_stock')}</span>
						</div>
						<div class="inv-kpi">
							<span class="inv-kpi-val color-revenue">{fmt($totalRevenue)}</span>
							<span class="inv-kpi-lbl">{$t('stat_total_revenue')}</span>
						</div>
					</div>
					{#if $inventory.length > 0}
						<div class="inv-recent-list">
							{#each $inventory.slice(0, 3) as item}
								<div class="inv-recent-row">
									<div class="inv-recent-info">
										<span class="inv-recent-name">{item.name}</span>
										<span class="inv-recent-cat">{item.category}</span>
									</div>
									<span class="inv-recent-stock">{item.current_stock} шт.</span>
								</div>
							{/each}
						</div>
					{/if}
				{/if}
			</div>
		</div>

		{/if}

		<!-- ── ANALYTICS MINI-CHART ───────────────────────────── -->
		{#if visibleWidgets.has('chart')}
		<div class="bento-card bento-chart bento-chart-wide">
			<div class="bento-card-header accent-chart">
				<span class="bento-icon">📈</span>
				<span class="bento-label">{$t('bento_chart_title')}</span>
				<a href="/analytics" class="bento-view-all">{$t('bento_view_all')}</a>
			</div>
			<div class="bento-card-body chart-body">
				{#if $inventory.length > 0}
					<canvas bind:this={chartCanvas} class="mini-chart"></canvas>
					<p class="chart-hint">Выручка по товарам</p>
				{:else}
					<p class="bento-empty">{$t('empty_no_items')}</p>
				{/if}
			</div>
		</div>

		{/if}

		<!-- ── RECENT ACTIVITY ────────────────────────────────── -->
		{#if visibleWidgets.has('activity')}
		<div class="bento-card bento-activity">
			<div class="bento-card-header accent-activity">
				<span class="bento-icon">🕐</span>
				<span class="bento-label">{$t('bento_activity_title')}</span>
				<a href="/audit" class="bento-view-all">{$t('bento_view_all')}</a>
			</div>
			<div class="bento-card-body">
				{#if $auditLog.length === 0}
					<p class="bento-empty">{$t('bento_no_activity')}</p>
				{:else}
					<div class="activity-list">
						{#each $auditLog.slice(0, 5) as entry}
							<div class="activity-row">
								<span class="activity-time">{formatTime(entry.timestamp)}</span>
								<span class="activity-action">{entry.action}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		{/if}

	</div>
</div>

<style>
	.cc-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	/* ── Header ── */
	.cc-header {
		margin-bottom: 28px;
	}

	.cc-title-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
	}

	.cc-title {
		font-size: clamp(1.4rem, 2.5vw, 2rem);
		font-weight: 800;
		letter-spacing: -0.03em;
		color: var(--color-primary);
		margin: 0;
	}

	.cc-node-badge {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 14px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 24px;
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
	}

	.cc-node-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-outline);
		transition: background 0.3s;
	}

	.cc-node-badge.online .cc-node-dot {
		background: #34d399;
		box-shadow: 0 0 6px #34d39988;
	}

	.cc-node-id {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-on-surface);
		opacity: 0.7;
	}

	/* ── Bento Grid ── */
	.bento-grid {
		display: grid;
		grid-template-columns: repeat(12, 1fr);
		grid-template-rows: auto;
		gap: 16px;
	}

	/* Grid placement — row 1: sync(4) + orders(3) + inventory(5) = 12 */
	.bento-sync      { grid-column: span 4; }
	.bento-orders    { grid-column: span 3; }
	.bento-inventory { grid-column: span 5; }
	/* row 2: chart(8) + activity(4) = 12 */
	.bento-chart-wide { grid-column: span 8; }
	.bento-activity  { grid-column: span 4; }

	@media (max-width: 1024px) {
		.bento-sync, .bento-orders, .bento-inventory,
		.bento-chart-wide, .bento-activity {
			grid-column: span 6;
		}
	}

	@media (max-width: 640px) {
		.bento-sync, .bento-orders, .bento-inventory,
		.bento-chart-wide, .bento-activity {
			grid-column: span 12;
		}
	}

	/* ── Bento Card ── */
	.bento-card {
		background: var(--glass-bg);
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 20px;
		box-shadow: var(--glass-shadow);
		overflow: hidden;
		display: flex;
		flex-direction: column;
		transition: transform 0.25s var(--ease-spring), box-shadow 0.25s var(--ease-spring);
	}

	.bento-card:hover {
		transform: translateY(-3px);
		box-shadow: var(--glass-shadow-hover);
	}

	/* ── Card Header ── */
	.bento-card-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 14px 16px 10px;
		border-bottom: 1px solid var(--glass-border);
	}

	/* Accent left-border colors per block */
	.accent-sync     { border-left: 3px solid #60a5fa; }
	.accent-orders   { border-left: 3px solid #fbbf24; }
	.accent-inventory { border-left: 3px solid var(--color-primary); }
	.accent-chart    { border-left: 3px solid var(--color-secondary); }
	.accent-activity { border-left: 3px solid var(--color-tertiary); }

	.bento-icon { font-size: 1rem; }

	.bento-label {
		font-size: 0.78rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-on-surface);
		opacity: 0.7;
		flex: 1;
	}

	.bento-view-all {
		font-size: 0.72rem;
		color: var(--color-primary);
		text-decoration: none;
		opacity: 0.8;
		transition: opacity 0.15s;
		white-space: nowrap;
	}

	.bento-view-all:hover { opacity: 1; text-decoration: none; }

	.bento-card-body {
		padding: 14px 16px;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.bento-empty {
		font-size: 0.8rem;
		color: var(--color-outline);
		text-align: center;
		padding: 12px 0;
	}

	/* ── Sync block ── */
	.sync-status-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.sync-dot-wrap {
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.sync-dot {
		width: 9px;
		height: 9px;
		border-radius: 50%;
		background: var(--color-outline);
		transition: background 0.3s;
	}

	.sync-dot-on {
		background: #34d399;
		box-shadow: 0 0 6px #34d39966;
	}

	.sync-status-text {
		font-size: 0.82rem;
		color: var(--color-on-surface);
	}

	.sync-peers-badge {
		font-size: 0.72rem;
		padding: 3px 9px;
		border-radius: 12px;
		background: var(--color-surface-container-high);
		color: var(--color-on-surface);
		opacity: 0.7;
	}

	.sync-node-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.sync-node-label {
		font-size: 0.72rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.sync-node-value {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--color-on-surface);
		opacity: 0.6;
	}

	.sync-peer-list {
		display: flex;
		flex-direction: column;
		gap: 5px;
		margin-top: 4px;
	}

	.sync-peer-row {
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.sync-peer-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.sync-peer-ip {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--color-on-surface);
		opacity: 0.6;
	}

	/* ── Orders block ── */
	.orders-kpi-row {
		display: flex;
		gap: 16px;
	}

	.orders-kpi {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.orders-kpi-num {
		font-size: 1.3rem;
		font-weight: 700;
		color: var(--color-on-surface);
		letter-spacing: -0.02em;
	}

	.orders-kpi-lbl {
		font-size: 0.65rem;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-outline);
	}

	.order-mini-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.order-mini-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		background: var(--color-surface-container);
		border-radius: 8px;
		font-size: 0.8rem;
	}

	.order-mini-name {
		flex: 1;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.order-mini-status { font-size: 0.7rem; }

	.order-mini-amount {
		font-weight: 600;
		color: var(--color-primary);
		white-space: nowrap;
	}

	/* ── Inventory block ── */
	.inv-kpi-grid {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: 8px;
	}

	.inv-kpi {
		display: flex;
		flex-direction: column;
		gap: 3px;
		padding: 10px;
		background: var(--color-surface-container);
		border-radius: 10px;
	}

	.inv-kpi-val {
		font-size: 1.1rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--color-on-surface);
	}

	.inv-kpi-lbl {
		font-size: 0.62rem;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-outline);
	}

	.inv-recent-list {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.inv-recent-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 10px;
		background: var(--color-surface-container);
		border-radius: 8px;
	}

	.inv-recent-info {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.inv-recent-name {
		font-size: 0.82rem;
		font-weight: 500;
		color: var(--color-on-surface);
	}

	.inv-recent-cat {
		font-size: 0.68rem;
		color: var(--color-outline);
	}

	.inv-recent-stock {
		font-size: 0.78rem;
		color: var(--color-secondary);
		font-weight: 500;
	}

	/* ── Chart block ── */
	.chart-body {
		gap: 6px;
	}

	.mini-chart {
		width: 100%;
		height: 90px;
		display: block;
	}

	.chart-hint {
		font-size: 0.68rem;
		color: var(--color-outline);
		text-align: center;
		margin: 0;
	}

	/* ── Activity block ── */
	.activity-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.activity-row {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 10px;
		background: var(--color-surface-container);
		border-radius: 8px;
	}

	.activity-time {
		font-family: var(--font-mono);
		font-size: 0.7rem;
		color: var(--color-outline);
		white-space: nowrap;
		flex-shrink: 0;
	}

	.activity-action {
		font-size: 0.78rem;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* ── Color tokens ── */
	.color-revenue  { color: var(--color-primary); }

	/* ── Widget menu ── */
	.cc-header-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.widget-menu-wrap {
		position: relative;
	}

	.widget-gear-btn {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		font-size: 1.1rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 0.15s, transform 0.15s;
	}

	.widget-gear-btn:hover {
		background: var(--glass-bg-hover);
		transform: rotate(30deg);
	}

	.widget-menu {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		z-index: 1200;
		min-width: 180px;
		padding: 8px;
		background: rgba(18, 18, 22, 0.92);
		backdrop-filter: blur(24px) saturate(180%);
		-webkit-backdrop-filter: blur(24px) saturate(180%);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	:global([data-theme="light"]) .widget-menu {
		background: rgba(248, 248, 252, 0.95);
	}

	.widget-toggle {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 6px;
		font-size: 0.82rem;
		color: var(--color-on-surface);
		cursor: pointer;
		transition: background 0.1s;
	}

	.widget-toggle:hover { background: rgba(255, 255, 255, 0.06); }

	.widget-toggle input[type="checkbox"] {
		accent-color: var(--color-primary);
		cursor: pointer;
	}
</style>
