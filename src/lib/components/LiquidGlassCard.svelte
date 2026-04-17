<script lang="ts">
	import type { Item } from '$lib/tauri/types';
	import { t } from '$lib/stores/i18n';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { globalCurrency, itemCurrencies, formatAmount } from '$lib/stores/currency';
	import { preset } from '$lib/stores/preset';
	import { commands } from '$lib/tauri/commands';
	import { inventory } from '$lib/stores/inventory';
	import { showDetailedPricing } from '$lib/stores/appSettings';

	interface Props {
		item: Item;
		appDataDir?: string;
		onclick?: () => void;
		ondelete?: (item: Item) => void;
		onduplicate?: (item: Item) => void;
		onpack?: (item: Item) => void;
		onsell?: (item: Item) => void;
	}

	let { item, appDataDir = '', onclick, ondelete, onduplicate, onpack, onsell }: Props = $props();

	let hovered = $state(false);
	let longPressed = $state(false);
	let stockHovered = $state(false);
	let longPressTimer: ReturnType<typeof setTimeout> | null = null;

	let currency = $derived($itemCurrencies[item.id] ?? $globalCurrency);

	function fmt(value: number): string {
		return formatAmount(value, currency);
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

	let showActions = $derived(hovered || longPressed);

	let cardBg = $derived(
		item.card_color
			? `background: linear-gradient(135deg, ${item.card_color}22 0%, var(--glass-bg) 60%);`
			: undefined
	);

	// ── Touch: long-press detection ────────────────────────────
	function onTouchStart() {
		longPressTimer = setTimeout(() => {
			longPressed = true;
			navigator.vibrate?.(50);
		}, 500);
	}

	function onTouchEnd() {
		if (longPressTimer) { clearTimeout(longPressTimer); longPressTimer = null; }
	}

	function onTouchMove() {
		if (longPressTimer) { clearTimeout(longPressTimer); longPressTimer = null; }
		longPressed = false;
	}

	// ── Card action handlers ───────────────────────────────────
	async function handleDelete(e: MouseEvent) {
		e.stopPropagation();
		if (!confirm(`Удалить "${item.name}"?`)) return;
		if (ondelete) { ondelete(item); return; }
		await commands.deleteItem(item.id);
		await inventory.load();
		longPressed = false;
	}

	async function handleDuplicate(e: MouseEvent) {
		e.stopPropagation();
		if (onduplicate) { onduplicate(item); return; }
		await commands.duplicateItem(item.id);
		await inventory.load();
		longPressed = false;
	}

	function handlePack(e: MouseEvent) {
		e.stopPropagation();
		onpack?.(item);
		longPressed = false;
	}

	function handleSell(e: MouseEvent) {
		e.stopPropagation();
		onsell?.(item);
		longPressed = false;
	}

	async function adjustStock(delta: number, e: MouseEvent) {
		e.stopPropagation();
		await commands.adjustStock({ item_id: item.id, delta });
		await inventory.load();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	class="glass-card"
	class:clickable={!!onclick}
	role={onclick ? 'button' : undefined}
	tabindex={onclick ? 0 : undefined}
	style={cardBg}
	onclick={() => { if (!longPressed) onclick?.(); }}
	onkeydown={(e) => e.key === 'Enter' && onclick?.()}
	onmouseenter={() => (hovered = true)}
	onmouseleave={() => { hovered = false; }}
	ontouchstart={onTouchStart}
	ontouchend={onTouchEnd}
	ontouchmove={onTouchMove}
>
	{#if imageSrc}
		<div class="card-image">
			<img src={imageSrc} alt={item.name} loading="lazy" />
		</div>
	{/if}

	<!-- Action buttons overlay -->
	<div class="card-actions" class:actions-visible={showActions}>
		{#if $preset === 'flowers'}
			<button class="card-action-btn" onclick={handlePack} title="Упаковать" aria-label="Упаковать">📦</button>
		{/if}
		<button class="card-action-btn sell" onclick={handleSell} title="Продать" aria-label="Продать">🛒</button>
		<button class="card-action-btn" onclick={handleDuplicate} title="Копировать" aria-label="Копировать">⧉</button>
		<button class="card-action-btn danger" onclick={handleDelete} title="Удалить" aria-label="Удалить">✕</button>
	</div>

	<div class="card-header">
		<h3 class="card-title">{item.name}</h3>
		{#if item.category && item.category !== 'uncategorized'}
			<span class="card-badge">{item.category}</span>
		{/if}
	</div>

	<div class="card-metrics">
		<div class="metric">
			<span class="mlabel">{$t('table_header_price')}</span>
			<span class="mvalue color-price">{fmt(item.current_price)}</span>
		</div>

		<!-- Stock with quick ±1 adjust -->
		<div
			class="metric stock-metric"
			role="group"
			aria-label="Остаток"
			onmouseenter={() => (stockHovered = true)}
			onmouseleave={() => (stockHovered = false)}
		>
			<span class="mlabel">Остаток</span>
			<div class="stock-adjust-row">
				{#if stockHovered}
					<button class="micro-btn" onclick={(e) => adjustStock(-1, e)} aria-label="−1">−</button>
				{/if}
				<span class="mvalue color-stock">{item.current_stock}</span>
				{#if stockHovered}
					<button class="micro-btn" onclick={(e) => adjustStock(+1, e)} aria-label="+1">+</button>
				{/if}
			</div>
		</div>

		<div class="metric">
			<span class="mlabel">{$t('table_header_sold')}</span>
			<span class="mvalue color-sold">{item.sold_count}</span>
		</div>
		<div class="metric">
			<span class="mlabel">{$t('table_header_revenue')}</span>
			<span class="mvalue color-revenue">{fmt(item.revenue)}</span>
		</div>
		{#if margin !== null && $showDetailedPricing}
			<div class="metric">
				<span class="mlabel">{$t('label_margin')}</span>
				<span class="mvalue color-margin">{margin.toFixed(1)}%</span>
			</div>
		{/if}
		{#if item.production_cost > 0 && $showDetailedPricing}
			<div class="metric">
				<span class="mlabel">{$t('label_cost')}</span>
				<span class="mvalue color-cost">{fmt(item.production_cost)}</span>
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
		box-shadow: var(--glass-shadow-hover);
	}

	.glass-card:active {
		transform: translateY(-1px) scale(1.004);
		transition-duration: 0.1s;
	}

	/* ── Action buttons overlay ── */
	.card-actions {
		position: absolute;
		top: 10px;
		right: 10px;
		display: flex;
		gap: 5px;
		opacity: 0;
		transform: scale(0.85);
		transition: opacity 200ms ease, transform 200ms var(--ease-overshoot);
		z-index: 10;
	}

	.card-actions.actions-visible {
		opacity: 1;
		transform: scale(1);
	}

	.card-action-btn {
		background: rgba(255, 255, 255, 0.12);
		backdrop-filter: blur(16px) saturate(180%);
		-webkit-backdrop-filter: blur(16px) saturate(180%);
		border: 1px solid rgba(255, 255, 255, 0.18);
		border-radius: 10px;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.2);
		cursor: pointer;
		font-size: 0.8rem;
		padding: 5px 8px;
		color: var(--color-on-surface);
		transition: background 0.15s, transform 0.15s;
		line-height: 1;
	}

	.card-action-btn:hover {
		background: rgba(255, 255, 255, 0.22);
		transform: translateY(-1px);
	}

	.card-action-btn.sell:hover {
		background: rgba(52, 211, 153, 0.3);
		border-color: rgba(52, 211, 153, 0.5);
	}

	.card-action-btn.danger:hover {
		background: rgba(248, 113, 113, 0.3);
		border-color: rgba(248, 113, 113, 0.5);
		color: #f87171;
	}

	/* ── Stock quick-adjust ── */
	.stock-metric { position: relative; }

	.stock-adjust-row {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.micro-btn {
		width: 16px;
		height: 16px;
		border-radius: 4px;
		border: 1px solid var(--glass-border);
		background: var(--glass-bg-hover);
		color: var(--color-on-surface);
		font-size: 0.7rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		line-height: 1;
		transition: background 0.1s;
	}

	.micro-btn:hover { background: var(--color-primary); color: #0a0a0a; }

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
