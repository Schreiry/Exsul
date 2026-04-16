<script lang="ts">
	import { untrack } from 'svelte';
	import { viewport, type DeviceClass } from './orientation';
	import DockItem from './DockItem.svelte';
	import type { DockItemConfig } from './types';

	interface Props {
		items: DockItemConfig[];
		presetKey?: string;
	}

	let { items: propItems, presetKey = 'default' }: Props = $props();

	// Internal mutable order — initialized from props, overridden by localStorage.
	// untrack() signals intentional one-time capture; the $effect below keeps items
	// in sync whenever propItems changes (e.g. after async preset.load()).
	let items = $state<DockItemConfig[]>(untrack(() => [...propItems]));

	let device = $state<DeviceClass>('desktop');
	let windowWidth = $state(typeof window !== 'undefined' ? window.innerWidth : 1200);

	$effect(() => {
		return viewport.subscribe((v) => (device = v.device));
	});

	$effect(() => {
		if (typeof window === 'undefined') return;
		const onResize = () => { windowWidth = window.innerWidth; };
		window.addEventListener('resize', onResize);
		return () => window.removeEventListener('resize', onResize);
	});

	// On narrow screens (<900px) always use bottom floating layout, never split rails
	const effectiveDevice = $derived(
		windowWidth < 900 ? ('tablet-portrait' as DeviceClass) : device
	);

	// Restore saved order from localStorage on mount — keyed per preset so
	// switching presets never corrupts the order of the other preset's items.
	// NOTE: we read `propItems` unconditionally before any early-return so that
	// Svelte 5 tracks it as a reactive dependency and re-runs this effect when
	// the parent changes the items array (e.g. after async preset.load()).
	$effect(() => {
		const currentItems = propItems; // track as dependency
		if (typeof localStorage === 'undefined') {
			items = [...currentItems];
			return;
		}
		const storageKey = `dock-order-${presetKey}`;
		const saved = localStorage.getItem(storageKey);
		if (!saved) {
			items = [...currentItems];
			return;
		}
		try {
			const order: string[] = JSON.parse(saved);
			const sorted = order
				.map((id) => currentItems.find((it) => it.id === id))
				.filter(Boolean) as DockItemConfig[];
			// Add any items not in saved order (new items added after last save)
			const missing = currentItems.filter((it) => !order.includes(it.id));
			items = [...sorted, ...missing];
		} catch {
			// malformed localStorage entry — fall back to prop order
			items = [...currentItems];
		}
	});

	function handleDragMove(fromIndex: number, toIndex: number) {
		const next = [...items];
		const [moved] = next.splice(fromIndex, 1);
		next.splice(toIndex, 0, moved);
		items = next;
		if (typeof localStorage !== 'undefined') {
			const storageKey = `dock-order-${presetKey}`;
			localStorage.setItem(storageKey, JSON.stringify(items.map((i) => i.id)));
		}
	}

	// Communicate dock dimensions to root so app-main can compensate
	$effect(() => {
		const root = document.documentElement;
		if (effectiveDevice === 'tablet-landscape') {
			root.style.setProperty('--dock-bottom-clearance', '0px');
			root.style.setProperty('--dock-side-clearance', '76px');
		} else if (effectiveDevice === 'mobile-portrait') {
			root.style.setProperty('--dock-bottom-clearance', '72px');
			root.style.setProperty('--dock-side-clearance', '0px');
		} else {
			root.style.setProperty('--dock-bottom-clearance', '96px');
			root.style.setProperty('--dock-side-clearance', '0px');
		}
	});

	// Drag-reorder is only enabled on non-split layouts
	let canDrag = $derived(effectiveDevice !== 'tablet-landscape');

	let midpoint = $derived(Math.ceil(items.length / 2));
	let leftItems = $derived(items.slice(0, midpoint));
	let rightItems = $derived(items.slice(midpoint));

	const layoutClass: Record<DeviceClass, string> = {
		'mobile-portrait': 'dock-bottom dock-full',
		'mobile-landscape': 'dock-bottom dock-compact',
		'tablet-portrait': 'dock-bottom dock-floating',
		'tablet-landscape': 'dock-split',
		desktop: 'dock-bottom dock-floating',
	};

	// Cursor-reactive spotlight — tracks mouse X within the dock bar
	let dockEl = $state<HTMLElement | null>(null);
	let mouseXRel = $state(-200);

	function onDockPointerMove(e: PointerEvent) {
		if (!dockEl) return;
		const rect = dockEl.getBoundingClientRect();
		mouseXRel = e.clientX - rect.left;
		dockEl.style.setProperty('--dock-mouse-x', `${mouseXRel}px`);
	}

	function onDockPointerLeave() {
		mouseXRel = -200;
		dockEl?.style.setProperty('--dock-mouse-x', '-200px');
	}
</script>

{#if effectiveDevice === 'tablet-landscape'}
	<nav class="dock-rail dock-rail-left" aria-label="Primary">
		{#each leftItems as item, i (item.id)}
			{#if item.separator_before}
				<div class="dock-sep dock-sep-vertical" role="separator" aria-hidden="true"></div>
			{/if}
			<DockItem config={item} vertical index={i} draggable={false} />
		{/each}
	</nav>
	<nav class="dock-rail dock-rail-right" aria-label="Secondary">
		{#each rightItems as item, i (item.id)}
			{#if item.separator_before}
				<div class="dock-sep dock-sep-vertical" role="separator" aria-hidden="true"></div>
			{/if}
			<DockItem config={item} vertical index={midpoint + i} draggable={false} />
		{/each}
	</nav>
{:else}
	<nav
		bind:this={dockEl}
		class="dock-bar {layoutClass[effectiveDevice]}"
		aria-label="Primary"
		onpointermove={onDockPointerMove}
		onpointerleave={onDockPointerLeave}
	>
		{#each items as item, i (item.id)}
			{#if item.separator_before}
				<div class="dock-sep" role="separator" aria-hidden="true"></div>
			{/if}
			<DockItem
				config={item}
				index={i}
				draggable={canDrag}
				ondragmove={handleDragMove}
			/>
		{/each}
	</nav>
{/if}

<style>
	.dock-bar {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		padding: 8px 12px;
		position: relative;
		overflow: hidden;

		/* Layered glass background */
		background:
			linear-gradient(135deg, rgba(255, 255, 255, 0.07) 0%, rgba(255, 255, 255, 0.02) 100%),
			var(--dock-bg, rgba(18, 18, 18, 0.82));

		/* Enhanced blur + saturation for depth */
		backdrop-filter: blur(32px) saturate(1.8) brightness(1.04);
		-webkit-backdrop-filter: blur(32px) saturate(1.8) brightness(1.04);

		border: 1px solid var(--dock-border, rgba(255, 255, 255, 0.10));
		border-top-color: rgba(255, 255, 255, 0.18);

		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.12),
			0 8px 32px rgba(0, 0, 0, 0.35),
			0 2px 8px rgba(0, 0, 0, 0.20);

		z-index: 1000;
		transition: box-shadow 0.3s var(--ease-spring, ease);
	}

	/* Cursor-reactive spotlight overlay */
	.dock-bar::before {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		background: radial-gradient(
			circle 80px at var(--dock-mouse-x, -200px) 50%,
			rgba(255, 255, 255, 0.07),
			transparent 70%
		);
		pointer-events: none;
		transition: background 0.05s linear;
	}

	.dock-bottom {
		position: fixed;
		bottom: 16px;
		left: 50%;
		transform: translateX(-50%);
	}

	.dock-floating {
		border-radius: 20px;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.14),
			0 12px 40px rgba(0, 0, 0, 0.40),
			0 4px 12px rgba(0, 0, 0, 0.25);
	}

	.dock-full {
		left: 0;
		right: 0;
		bottom: 0;
		transform: none;
		border-radius: 0;
	}

	.dock-compact {
		border-radius: 20px;
		max-width: 80%;
	}

	.dock-rail {
		position: fixed;
		top: 50%;
		transform: translateY(-50%);
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 12px 8px;
		background:
			linear-gradient(180deg, rgba(255, 255, 255, 0.07) 0%, rgba(255, 255, 255, 0.02) 100%),
			var(--dock-bg, rgba(18, 18, 18, 0.82));
		backdrop-filter: blur(32px) saturate(1.8) brightness(1.04);
		-webkit-backdrop-filter: blur(32px) saturate(1.8) brightness(1.04);
		border: 1px solid var(--dock-border, rgba(255, 255, 255, 0.10));
		border-top-color: rgba(255, 255, 255, 0.18);
		border-radius: 20px;
		z-index: 1000;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.12),
			0 8px 32px rgba(0, 0, 0, 0.35);
	}

	.dock-rail-left  { left: 12px; }
	.dock-rail-right { right: 12px; }

	/* Separator — horizontal line with gap on each side */
	.dock-sep {
		width: 1px;
		height: 24px;
		margin: 0 6px;
		background: var(--dock-border, rgba(255, 255, 255, 0.12));
		border-radius: 1px;
		flex-shrink: 0;
	}

	.dock-sep-vertical {
		width: 24px;
		height: 1px;
		margin: 6px 0;
	}

	/* Responsive dock sizing */
	@media (min-width: 1600px) and (max-width: 2399px) {
		.dock-bar { gap: 5px; padding: 9px 14px; }
		.dock-sep { height: 26px; margin: 0 7px; }
	}

	@media (min-width: 2400px) {
		.dock-bar { gap: 6px; padding: 10px 16px; }
		.dock-sep { height: 30px; margin: 0 8px; }
	}

	/* Light mode overrides */
	:global([data-theme="light"]) .dock-bar {
		background:
			linear-gradient(135deg, rgba(255, 255, 255, 0.55) 0%, rgba(255, 255, 255, 0.35) 100%),
			var(--dock-bg, rgba(240, 240, 240, 0.75));
		border-top-color: rgba(255, 255, 255, 0.85);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.80),
			0 8px 32px rgba(0, 0, 0, 0.12),
			0 2px 8px rgba(0, 0, 0, 0.06);
	}

	:global([data-theme="light"]) .dock-bar::before {
		background: radial-gradient(
			circle 80px at var(--dock-mouse-x, -200px) 50%,
			rgba(0, 0, 0, 0.04),
			transparent 70%
		);
	}

	:global([data-theme="light"]) .dock-rail {
		background:
			linear-gradient(180deg, rgba(255, 255, 255, 0.55) 0%, rgba(255, 255, 255, 0.35) 100%),
			var(--dock-bg, rgba(240, 240, 240, 0.75));
	}
</style>
