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
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: 'USD',
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
		<span class="card-badge">{item.category}</span>
	</div>

	<div class="card-metrics">
		<div class="metric">
			<span class="mlabel">{$t('table_header_price')}</span>
			<span class="mvalue accent">{formatCurrency(item.current_price)}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_stock')}</span>
			<span class="mvalue">{item.current_stock}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_sold')}</span>
			<span class="mvalue">{item.sold_count}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_revenue')}</span>
			<span class="mvalue">{formatCurrency(item.revenue)}</span>
		</div>
		{#if margin !== null}
			<div class="metric">
				<span class="mlabel">Margin</span>
				<span class="mvalue" class:positive={margin >= 0} class:negative={margin < 0}>
					{margin.toFixed(1)}%
				</span>
			</div>
		{/if}
		{#if item.production_cost > 0}
			<div class="metric">
				<span class="mlabel">Cost</span>
				<span class="mvalue dim">{formatCurrency(item.production_cost)}</span>
			</div>
		{/if}
	</div>

	<!-- Travelling shimmer on hover -->
	<div class="card-shimmer" aria-hidden="true"></div>
</div>

<style>
	.glass-card {
		position: relative;
		background: rgba(255, 255, 255, 0.04);
		backdrop-filter: blur(24px) saturate(180%);
		-webkit-backdrop-filter: blur(24px) saturate(180%);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-top: 1px solid rgba(255, 255, 255, 0.17);
		border-left: 1px solid rgba(255, 255, 255, 0.10);
		border-radius: 16px;
		padding: 16px;
		overflow: hidden;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.18),
			0 8px 32px rgba(0, 0, 0, 0.45),
			0 2px 8px rgba(0, 0, 0, 0.25);
		transition:
			transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1),
			box-shadow 0.4s cubic-bezier(0.2, 0.8, 0.2, 1),
			border-color 0.4s cubic-bezier(0.2, 0.8, 0.2, 1),
			background 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
	}

	.glass-card:hover {
		transform: translateY(-4px) scale(1.012);
		background: rgba(255, 255, 255, 0.06);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.24),
			0 24px 60px rgba(0, 0, 0, 0.55),
			0 8px 20px rgba(0, 0, 0, 0.3);
		border-top-color: rgba(255, 255, 255, 0.26);
		border-left-color: rgba(255, 255, 255, 0.15);
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
		transition: left 0.7s cubic-bezier(0.2, 0.8, 0.2, 1);
	}

	.glass-card:hover .card-shimmer {
		left: 170%;
	}

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
		transition: transform 0.55s cubic-bezier(0.2, 0.8, 0.2, 1);
	}

	.glass-card:hover .card-image img {
		transform: scale(1.06);
	}

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
		color: rgba(255, 255, 255, 0.92);
		text-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
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
		background: rgba(255, 255, 255, 0.07);
		border: 1px solid rgba(255, 255, 255, 0.10);
		padding: 3px 8px;
		border-radius: 6px;
		color: rgba(255, 255, 255, 0.5);
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
		color: rgba(255, 255, 255, 0.33);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 500;
	}

	.mvalue {
		font-size: 0.875rem;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.82);
	}

	.mvalue.accent {
		color: var(--color-primary, #34d399);
		text-shadow: 0 0 12px rgba(52, 211, 153, 0.25);
	}

	.mvalue.positive { color: #34d399; }
	.mvalue.negative { color: #f87171; }
	.mvalue.dim { color: rgba(255, 255, 255, 0.45); }
</style>
