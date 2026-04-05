<script lang="ts">
	import type { Item } from '$lib/tauri/types';
	import { t } from '$lib/stores/i18n';
	import { convertFileSrc } from '@tauri-apps/api/core';

	interface Props {
		item: Item;
		appDataDir?: string;
		onclick?: () => void;
	}

	let { item, appDataDir = '', onclick }: Props = $props();

	function formatCurrency(value: number): string {
		return new Intl.NumberFormat('ru-RU', {
			style: 'currency',
			currency: 'RUB',
			maximumFractionDigits: 0
		}).format(value);
	}

	function getImageSrc(path: string | null | undefined, baseDir: string): string | null {
		if (!path || !baseDir) return null;
		const base = baseDir.endsWith('\\') || baseDir.endsWith('/') ? baseDir : baseDir + '/';
		return convertFileSrc(base + path.replace(/\\/g, '/'));
	}

	let imageSrc = $derived(getImageSrc(item.image_path, appDataDir));

	let margin = $derived(
		item.production_cost > 0
			? ((item.current_price - item.production_cost) / item.current_price) * 100
			: null
	);
</script>

<div
	class="glass-card"
	class:clickable={!!onclick}
	role={onclick ? 'button' : undefined}
	tabindex={onclick ? 0 : undefined}
	{onclick}
	onkeydown={(e) => e.key === 'Enter' && onclick?.()}
>
	{#if imageSrc}
		<div class="card-image">
			<img src={imageSrc} alt={item.name} loading="lazy" />
		</div>
	{/if}

	<div class="card-header">
		<h3 class="card-title">{item.name}</h3>
		{#if item.category && item.category !== 'uncategorized'}
			<span class="card-badge">{item.category}</span>
		{/if}
	</div>

	<div class="card-metrics">
		<div class="metric">
			<span class="mlabel">{$t('table_header_price')}</span>
			<span class="mvalue color-price">{formatCurrency(item.current_price)}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_stock')}</span>
			<span class="mvalue color-stock">{item.current_stock}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_sold')}</span>
			<span class="mvalue color-sold">{item.sold_count}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_revenue')}</span>
			<span class="mvalue color-revenue">{formatCurrency(item.revenue)}</span>
		</div>
		{#if margin !== null}
			<div class="metric">
				<span class="mlabel">{$t('label_margin')}</span>
				<span class="mvalue color-margin">
					{margin.toFixed(1)}%
				</span>
			</div>
		{/if}
		{#if item.production_cost > 0}
			<div class="metric">
				<span class="mlabel">{$t('label_cost')}</span>
				<span class="mvalue color-cost">{formatCurrency(item.production_cost)}</span>
			</div>
		{/if}
	</div>

	<!-- Travelling shimmer on hover -->
	<div class="card-shimmer" aria-hidden="true"></div>
</div>

<style>
	.glass-card {
		position: relative;
		background: var(--glass-bg);
		backdrop-filter: var(--glass-blur);
		-webkit-backdrop-filter: var(--glass-blur);
		border: 1px solid var(--glass-border);
		border-top: 1px solid var(--glass-border-top);
		border-radius: 16px;
		padding: 16px;
		overflow: hidden;
		box-shadow: var(--glass-shadow);
		transition:
			transform 0.4s var(--ease-spring),
			box-shadow 0.4s var(--ease-spring),
			border-color 0.4s var(--ease-spring),
			background 0.4s var(--ease-spring);
	}

	.glass-card.clickable { cursor: pointer; }

	.glass-card:hover {
		transform: translateY(-4px) scale(1.012);
		background: var(--glass-bg-hover);
		box-shadow: var(--glass-shadow-hover);
	}

	.glass-card:active {
		transform: translateY(-1px) scale(1.004);
		transition-duration: 0.1s;
	}

	/* Travelling shimmer on hover */
	.card-shimmer {
		position: absolute;
		top: 0;
		left: -120%;
		width: 50%;
		height: 100%;
		background: linear-gradient(
			105deg,
			transparent 30%,
			rgba(255, 255, 255, 0.05) 50%,
			transparent 70%
		);
		pointer-events: none;
		transition: left 0.7s var(--ease-spring);
	}

	.glass-card:hover .card-shimmer { left: 170%; }

	/* Image */
	.card-image {
		margin: -16px -16px 14px;
		height: 148px;
		overflow: hidden;
		border-radius: 16px 16px 0 0;
	}

	.card-image img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		transition: transform 0.55s var(--ease-spring);
	}

	.glass-card:hover .card-image img { transform: scale(1.06); }

	/* Header */
	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 8px;
		margin-bottom: 14px;
	}

	.card-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0;
		letter-spacing: -0.01em;
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.card-badge {
		flex-shrink: 0;
		font-size: 0.67rem;
		background: var(--glass-bg-hover);
		border: 1px solid var(--glass-border);
		padding: 3px 8px;
		border-radius: 6px;
		color: var(--color-outline);
		letter-spacing: 0.02em;
		white-space: nowrap;
	}

	/* Metrics grid */
	.card-metrics {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.metric {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.mlabel {
		font-size: 0.62rem;
		color: var(--color-outline);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 500;
	}

	.mvalue {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-on-surface);
	}

	/* Color-coded metric values */
	.color-price   { color: var(--color-primary); text-shadow: 0 0 12px color-mix(in srgb, var(--color-primary) 30%, transparent); }
	.color-stock   { color: #60a5fa; }
	.color-sold    { color: #34d399; }
	.color-revenue { color: #fbbf24; }
	.color-margin  { color: #a78bfa; }
	.color-cost    { color: var(--color-outline); }
</style>
