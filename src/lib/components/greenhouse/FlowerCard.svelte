<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';
	import type { FlowerSort } from '$lib/tauri/types';
	import { globalCurrency, formatAmount } from '$lib/stores/currency';

	interface Props {
		sort: FlowerSort;
		compact?: boolean;
		selected?: boolean;
		/** Packs currently reserved for active orders of this sort (not yet delivered).
		 *  Parent is expected to pre-compute this from pack_assignments so we avoid
		 *  N queries on a grid. Zero or undefined → badge hidden. */
		reservedPacks?: number;
		/** Count of active orders that have reservations for this sort.
		 *  Drives the tooltip on the reservation badge. */
		waitingOrders?: number;
		onclick?: () => void;
	}

	let {
		sort,
		compact = false,
		selected = false,
		reservedPacks = 0,
		waitingOrders = 0,
		onclick,
	}: Props = $props();

	const reservedTitle = $derived(
		waitingOrders > 0
			? `${reservedPacks} уп. зарезервировано — ${waitingOrders} заказ(ов)`
			: `${reservedPacks} уп. зарезервировано`
	);

	let appDataDir = $state('');

	$effect(() => {
		import('@tauri-apps/api/path').then(({ appDataDir: getDir }) =>
			getDir().then((dir) => { appDataDir = dir; })
		).catch(() => {});
	});

	function resolvePhotoSrc(photoPath: string | null | undefined, baseDir: string): string | null {
		if (!photoPath) return null;
		// If already absolute, use directly
		if (photoPath.includes(':') || photoPath.startsWith('/')) {
			return convertFileSrc(photoPath);
		}
		// Relative path — prepend appDataDir
		if (!baseDir) return null;
		const base = baseDir.endsWith('\\') || baseDir.endsWith('/') ? baseDir : baseDir + '/';
		return convertFileSrc(base + photoPath.replace(/\\/g, '/'));
	}

	const photoSrc = $derived(resolvePhotoSrc(sort.photo_path, appDataDir));

	// Only publish the color as a CSS variable — the gradient, border and
	// ambient glow are handled in CSS via `.has-color` rules so we can tune
	// static vs. hover consistently without duplicating values in JS.
	const cardStyle = $derived(
		sort.color_hex ? `--card-color: ${sort.color_hex};` : ''
	);
</script>

<button
	class="flower-card"
	class:compact
	class:selected
	class:has-color={!!sort.color_hex}
	type="button"
	style={cardStyle}
	{onclick}
>
	<!-- Photo area -->
	<div class="card-photo">
		{#if photoSrc}
			<img src={photoSrc} alt={sort.name} class="card-img" />
		{:else}
			<div class="card-placeholder" aria-hidden="true">
				<svg viewBox="0 0 24 24" width="36" height="36" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" opacity="0.35">
					<path d="M12 22 L12 10"/>
					<path d="M12 17 C9 15.5 7.5 13 9 11"/>
					<path d="M12 14 C15 12.5 16.5 10 15 8"/>
					<path d="M12 10 C10 8 9.5 5 12 3"/>
					<path d="M12 10 C14 8 14.5 5 12 3"/>
					<path d="M12 10 C9.5 9 8 6.5 9 4.5"/>
					<path d="M12 10 C14.5 9 16 6.5 15 4.5"/>
				</svg>
			</div>
		{/if}

		{#if sort.pkg_stock > 0}
			<span class="pkg-badge">{sort.pkg_stock} уп.</span>
		{/if}
		{#if reservedPacks > 0}
			<span class="reserved-badge" title={reservedTitle}>
				<svg viewBox="0 0 24 24" width="10" height="10" stroke="currentColor" fill="none" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M12 2 L3 7 v6 c0 5 4 9 9 9 s9 -4 9 -9 V7 z"/>
				</svg>
				{reservedPacks}
			</span>
		{/if}
	</div>

	<!-- Info area -->
	<div class="card-info">
		<p class="card-name">{sort.name}</p>
		{#if sort.variety}
			<p class="card-variety">{sort.variety}</p>
		{/if}
		<p class="card-stock">
			<span class="stock-num">{sort.raw_stock}</span>
			<span class="stock-unit"> шт.</span>
		</p>
		{#if sort.sell_price_stem > 0}
			<p class="card-price">{formatAmount(sort.sell_price_stem, $globalCurrency)}/шт.</p>
		{/if}
	</div>
</button>

<style>
	.flower-card {
		display: flex;
		flex-direction: column;
		background: var(--glass-bg-base, rgba(255,255,255,0.14));
		border: 1px solid var(--glass-border, rgba(255,255,255,0.14));
		border-top-color: var(--glass-border-top, rgba(255,255,255,0.28));
		border-radius: 16px;
		cursor: pointer;
		text-align: left;
		transition: transform 0.15s var(--ease-spring), box-shadow 0.15s, border-color 0.15s;
		overflow: hidden;
		padding: 0;
		/* Glass blur with strong saturation so the wallpaper bleeds through
		   tinted but text under the photo stays legible. */
		backdrop-filter: var(--glass-blur, blur(20px) saturate(180%));
		-webkit-backdrop-filter: var(--glass-blur, blur(20px) saturate(180%));
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.18),
			0 4px 14px rgba(0, 0, 0, 0.22);
	}

	.flower-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 8px 24px rgba(0,0,0,0.2);
		border-color: var(--color-outline, rgba(255,255,255,0.18));
	}

	/* Static tint: saturated enough to read clearly in any theme,
	   light enough that text on top stays legible. Gradient diagonals
	   from a strong color wash in the top-left to the neutral surface
	   so the card still feels like part of the glass layer. */
	.flower-card.has-color {
		background:
			linear-gradient(135deg,
				color-mix(in srgb, var(--card-color) 32%, var(--glass-bg)) 0%,
				color-mix(in srgb, var(--card-color) 10%, var(--glass-bg)) 100%);
		border-color: color-mix(in srgb, var(--card-color) 42%, var(--glass-border));
		box-shadow:
			inset 0 1px 0 color-mix(in srgb, var(--card-color) 22%, transparent),
			0 2px 10px color-mix(in srgb, var(--card-color) 12%, transparent);
	}

	.flower-card.has-color:hover {
		box-shadow: 0 8px 24px rgba(0,0,0,0.2), 0 0 24px color-mix(in srgb, var(--card-color) 25%, transparent);
		border-color: color-mix(in srgb, var(--card-color) 55%, var(--glass-border));
	}

	.flower-card.selected {
		border-color: var(--color-primary);
		box-shadow: 0 0 0 2px var(--color-primary), 0 8px 24px rgba(0,0,0,0.2);
	}

	/* Photo */
	.card-photo {
		position: relative;
		width: 100%;
		aspect-ratio: 4/3;
		background: color-mix(in srgb, var(--color-primary) 8%, transparent);
		overflow: hidden;
		flex-shrink: 0;
	}

	.compact .card-photo {
		aspect-ratio: 3/2;
	}

	.card-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.card-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-primary);
	}

	.pkg-badge {
		position: absolute;
		top: 8px;
		right: 8px;
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		font-size: 0.7rem;
		font-weight: 600;
		padding: 2px 7px;
		border-radius: 20px;
		line-height: 1.4;
	}

	/* Stacks below .pkg-badge when both are present. Uses amber by default
	   (reservation exists), switches to alert-red when the reservation hasn't
	   yet been fully covered by packed stock (i.e. operator still owes packs
	   to the order). */
	.reserved-badge {
		position: absolute;
		top: 34px;
		right: 8px;
		display: inline-flex;
		align-items: center;
		gap: 3px;
		background: color-mix(in srgb, #f59e0b 85%, rgba(0,0,0,0.5));
		color: #fff;
		font-size: 0.68rem;
		font-weight: 600;
		padding: 2px 7px 2px 5px;
		border-radius: 20px;
		line-height: 1.4;
		border: 1px solid color-mix(in srgb, #f59e0b 60%, transparent);
		box-shadow: 0 2px 6px rgba(0,0,0,0.2);
	}

	/* Info */
	.card-info {
		padding: 10px 12px 12px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.compact .card-info {
		padding: 8px 10px 10px;
	}

	.card-name {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.card-variety {
		font-size: 0.75rem;
		color: var(--color-outline, #888);
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.card-stock {
		margin: 4px 0 0;
	}

	.stock-num {
		font-size: 1.4rem;
		font-weight: 700;
		color: var(--color-primary);
		line-height: 1;
	}

	.compact .stock-num {
		font-size: 1.1rem;
	}

	.stock-unit {
		font-size: 0.75rem;
		color: var(--color-outline);
	}

	.card-price {
		margin: 2px 0 0;
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--color-primary);
		opacity: 0.85;
	}
</style>
