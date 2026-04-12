<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';
	import type { FlowerSort } from '$lib/tauri/types';

	interface Props {
		sort: FlowerSort;
		compact?: boolean;
		selected?: boolean;
		onclick?: () => void;
	}

	let { sort, compact = false, selected = false, onclick }: Props = $props();

	const photoSrc = $derived(
		sort.photo_path ? convertFileSrc(sort.photo_path) : null
	);
</script>

<button
	class="flower-card"
	class:compact
	class:selected
	type="button"
	{onclick}
>
	<!-- Photo area -->
	<div class="card-photo">
		{#if photoSrc}
			<img src={photoSrc} alt={sort.name} class="card-img" />
		{:else}
			<div class="card-placeholder" aria-hidden="true">
				<!-- Minimal flower SVG placeholder -->
				<svg viewBox="0 0 24 24" width="36" height="36" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" opacity="0.35">
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(72 12 12)"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(144 12 12)"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(216 12 12)"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(288 12 12)"/>
					<circle cx="12" cy="12" r="2"/>
				</svg>
			</div>
		{/if}

		{#if sort.pkg_stock > 0}
			<span class="pkg-badge">{sort.pkg_stock} уп.</span>
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
	</div>
</button>

<style>
	.flower-card {
		display: flex;
		flex-direction: column;
		background: var(--glass-bg, rgba(255,255,255,0.04));
		border: 1px solid var(--glass-border, rgba(255,255,255,0.09));
		border-radius: 16px;
		cursor: pointer;
		text-align: left;
		transition: transform 0.15s var(--ease-spring), box-shadow 0.15s, border-color 0.15s;
		overflow: hidden;
		padding: 0;
	}

	.flower-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 8px 24px rgba(0,0,0,0.2);
		border-color: var(--color-outline, rgba(255,255,255,0.18));
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
</style>
