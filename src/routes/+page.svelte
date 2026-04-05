<script lang="ts">
	import { inventory, totalStock, totalRevenue, totalItems } from '$lib/stores/inventory';
	import { preset } from '$lib/stores/preset';
	import { flowerSorts, flowerFinancials, flowerConstants, sortsBySpecies } from '$lib/stores/flowers';
	import { t } from '$lib/stores/i18n';
	// Load flower data when preset is flowers
	$effect(() => {
		if ($preset === 'flowers') {
			flowerSorts.load();
			flowerConstants.load();
		}
	});

	function formatCurrency(value: number): string {
		return new Intl.NumberFormat('ru-RU', { style: 'currency', currency: 'RUB', maximumFractionDigits: 0 }).format(value);
	}
</script>

<div class="dashboard">
	{#if $preset === 'flowers'}
		<!-- ── FLOWERS DASHBOARD ──────────────────────────── -->
		<div class="flowers-header">
			<h1 class="flowers-title">{$t('page_flowers_title')}</h1>
			<p class="flowers-subtitle">{$t('page_flowers_subtitle')}</p>
		</div>

		<!-- KPI strip -->
		<div class="kpi-strip">
			<div class="kpi-card">
				<span class="kpi-label">{$t('flowers_raw_stock')}</span>
				<span class="kpi-value color-raw">{$flowerFinancials.totalRaw}</span>
			</div>
			<div class="kpi-card">
				<span class="kpi-label">{$t('flowers_total_packs')}</span>
				<span class="kpi-value color-pkg">{$flowerFinancials.totalPkg}</span>
			</div>
			<div class="kpi-card">
				<span class="kpi-label">{$t('flowers_potential_packs')}</span>
				<span class="kpi-value color-potential">{$flowerFinancials.potentialPacks}</span>
			</div>
			<div class="kpi-card">
				<span class="kpi-label">{$t('flowers_total_weight')}</span>
				<span class="kpi-value color-weight">{$flowerFinancials.totalWeight.toFixed(1)} кг</span>
			</div>
			<div class="kpi-card">
				<span class="kpi-label">{$t('label_total_packs_value')}</span>
				<span class="kpi-value color-value">{formatCurrency($flowerFinancials.packValue)}</span>
			</div>
		</div>

		<!-- Species grid -->
		{#if $flowerSorts.length > 0}
			<div class="species-grid">
				{#each [...$sortsBySpecies.entries()] as [species, sorts]}
					{@const totalRaw = sorts.reduce((s, f) => s + f.raw_stock, 0)}
					{@const totalPkg = sorts.reduce((s, f) => s + f.pkg_stock, 0)}
					{@const speciesColor = sorts[0]?.color_hex ?? null}
					<a href="/flowers" class="species-card glass-card">
						{#if speciesColor}
							<div class="species-color-bar" style:background={speciesColor}></div>
						{/if}

						<div class="species-body">
							<div class="species-name">{species}</div>
							{#if sorts.length > 1}
								<div class="species-variety-count">{sorts.length} сортов</div>
							{:else if sorts[0]?.variety}
								<div class="species-variety-count">{sorts[0].variety}</div>
							{/if}

							<div class="species-metrics">
								<div class="s-metric">
									<span class="s-label">{$t('flowers_raw_stock')}</span>
									<span class="s-value color-raw">{totalRaw}</span>
								</div>
								<div class="s-metric">
									<span class="s-label">{$t('label_packaged')}</span>
									<span class="s-value color-pkg">{totalPkg}</span>
								</div>
								<div class="s-metric">
									<span class="s-label">{$t('label_total_packs_value')}</span>
									<span class="s-value color-value">{formatCurrency(totalPkg * $flowerConstants.price_per_pack)}</span>
								</div>
							</div>
						</div>
					</a>
				{/each}
			</div>
		{:else}
			<div class="empty-state">
				<p>{$t('empty_no_items')} <a href="/flowers">→ {$t('nav_flowers')}</a></p>
			</div>
		{/if}

	{:else}
		<!-- ── DEFAULT DASHBOARD ──────────────────────────── -->
		<p class="page-subtitle">{$t('page_dashboard_subtitle')}</p>

		<div class="stats-grid">
			<div class="stat-card">
				<span class="stat-label">{$t('stat_total_items')}</span>
				<span class="stat-value">{$totalItems}</span>
			</div>
			<div class="stat-card">
				<span class="stat-label">{$t('stat_total_stock')}</span>
				<span class="stat-value">{$totalStock}</span>
			</div>
			<div class="stat-card">
				<span class="stat-label">{$t('stat_total_revenue')}</span>
				<span class="stat-value">{formatCurrency($totalRevenue)}</span>
			</div>
		</div>

		{#if $inventory.length > 0}
			<section class="recent-items">
				<h2>{$t('section_recent_items')}</h2>
				<div class="items-list">
					{#each $inventory.slice(0, 5) as item (item.id)}
						<div class="item-row">
							<div class="item-info">
								<span class="item-name">{item.name}</span>
								<span class="item-category">{item.category}</span>
							</div>
							<div class="item-meta">
								<span class="item-price">{formatCurrency(item.current_price)}</span>
								<span class="item-stock">{$t('table_header_stock')}: {item.current_stock}</span>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{:else}
			<div class="empty-state">
				<p>{$t('empty_go_to_inventory')} <a href="/inventory">→ {$t('nav_inventory')}</a></p>
			</div>
		{/if}
	{/if}
</div>

<style>
	.dashboard { max-width: 900px; margin: 0 auto; }

	/* ── Flowers header ── */
	.flowers-header { margin-bottom: 28px; }

	.flowers-title {
		font-size: 2rem;
		font-weight: 800;
		letter-spacing: -0.03em;
		color: var(--color-primary);
		margin: 0;
	}

	.flowers-subtitle {
		color: var(--color-outline);
		margin-top: 4px;
		font-size: 0.9rem;
	}

	/* ── KPI strip ── */
	.kpi-strip {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 12px;
		margin-bottom: 28px;
	}

	.kpi-card {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 14px;
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
	}

	.kpi-label {
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-outline);
		font-weight: 500;
	}

	.kpi-value {
		font-size: 1.5rem;
		font-weight: 700;
		letter-spacing: -0.02em;
	}

	/* ── Species grid ── */
	.species-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 14px;
	}

	.species-card {
		display: flex;
		flex-direction: column;
		text-decoration: none;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 16px;
		overflow: hidden;
		box-shadow: var(--glass-shadow);
		transition: transform 0.3s var(--ease-spring), box-shadow 0.3s var(--ease-spring);
	}

	.species-card:hover {
		transform: translateY(-3px) scale(1.01);
		box-shadow: var(--glass-shadow-hover);
		text-decoration: none;
	}

	.species-color-bar { height: 6px; }

	.species-body { padding: 14px; display: flex; flex-direction: column; gap: 10px; }

	.species-name {
		font-size: 1rem;
		font-weight: 700;
		color: var(--color-on-surface);
		letter-spacing: -0.01em;
	}

	.species-variety-count {
		font-size: 0.75rem;
		color: var(--color-outline);
		margin-top: -6px;
	}

	.species-metrics {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.s-metric { display: flex; flex-direction: column; gap: 2px; }

	.s-label {
		font-size: 0.6rem;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--color-outline);
		font-weight: 500;
	}

	.s-value { font-size: 0.9rem; font-weight: 600; }

	/* ── Default dashboard ── */
	.page-subtitle {
		color: var(--color-outline);
		margin-bottom: 28px;
		font-size: 0.9rem;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 16px;
		margin-bottom: 32px;
	}

	.stat-card {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 12px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
	}

	.stat-label {
		font-size: 0.75rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-weight: 500;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-on-surface);
	}

	.recent-items h2 {
		font-size: 1rem;
		font-weight: 600;
		margin-bottom: 12px;
		color: var(--color-on-surface);
		letter-spacing: -0.01em;
	}

	.items-list { display: flex; flex-direction: column; gap: 8px; }

	.item-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
	}

	.item-info { display: flex; flex-direction: column; gap: 2px; }

	.item-name { font-weight: 500; font-size: 0.9rem; color: var(--color-on-surface); }

	.item-category { font-size: 0.78rem; color: var(--color-outline); }

	.item-meta { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }

	.item-price { font-weight: 600; color: var(--color-primary); font-size: 0.9rem; }

	.item-stock { font-size: 0.78rem; color: var(--color-outline); }

	.empty-state {
		text-align: center;
		padding: 48px;
		color: var(--color-outline);
	}

	/* ── Color tokens ── */
	.color-raw      { color: #60a5fa; }
	.color-pkg      { color: #34d399; }
	.color-potential { color: #fbbf24; }
	.color-weight   { color: #94a3b8; }
	.color-value    { color: var(--color-primary); }
</style>
