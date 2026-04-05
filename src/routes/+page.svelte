<script lang="ts">
	import { inventory, totalStock, totalRevenue, totalItems } from '$lib/stores/inventory';

	function formatCurrency(value: number): string {
		return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(value);
	}
</script>

<div class="dashboard">
	<h1 class="title">Exsul</h1>
	<p class="subtitle">Inventory & Analytics</p>

	<div class="stats-grid">
		<div class="stat-card">
			<span class="stat-label">Total Items</span>
			<span class="stat-value">{$totalItems}</span>
		</div>
		<div class="stat-card">
			<span class="stat-label">Total Stock</span>
			<span class="stat-value">{$totalStock}</span>
		</div>
		<div class="stat-card">
			<span class="stat-label">Revenue</span>
			<span class="stat-value">{formatCurrency($totalRevenue)}</span>
		</div>
	</div>

	{#if $inventory.length > 0}
		<section class="recent-items">
			<h2>Recent Items</h2>
			<div class="items-list">
				{#each $inventory.slice(0, 5) as item (item.id)}
					<div class="item-row">
						<div class="item-info">
							<span class="item-name">{item.name}</span>
							<span class="item-category">{item.category}</span>
						</div>
						<div class="item-meta">
							<span class="item-price">{formatCurrency(item.current_price)}</span>
							<span class="item-stock">Stock: {item.current_stock}</span>
						</div>
					</div>
				{/each}
			</div>
		</section>
	{:else}
		<div class="empty-state">
			<p>No items yet. Go to <a href="/inventory">Inventory</a> to add your first item.</p>
		</div>
	{/if}
</div>

<style>
	.dashboard {
		max-width: 800px;
		margin: 0 auto;
	}

	.title {
		font-size: 2rem;
		font-weight: 700;
		color: var(--color-primary);
	}

	.subtitle {
		color: var(--color-outline);
		margin-bottom: 32px;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 16px;
		margin-bottom: 32px;
	}

	.stat-card {
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.stat-label {
		font-size: 0.85rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--color-on-surface);
	}

	.recent-items h2 {
		font-size: 1.1rem;
		margin-bottom: 12px;
		color: var(--color-on-surface);
	}

	.items-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.item-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
	}

	.item-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.item-name {
		font-weight: 500;
	}

	.item-category {
		font-size: 0.8rem;
		color: var(--color-outline);
	}

	.item-meta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.item-price {
		font-weight: 600;
		color: var(--color-primary);
	}

	.item-stock {
		font-size: 0.8rem;
		color: var(--color-outline);
	}

	.empty-state {
		text-align: center;
		padding: 48px;
		color: var(--color-outline);
	}
</style>
