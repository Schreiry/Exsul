<script lang="ts">
	import { viewport, type DeviceClass } from './orientation';
	import DockItem from './DockItem.svelte';
	import type { DockItemConfig } from './types';

	interface Props {
		items: DockItemConfig[];
	}

	let { items: propItems }: Props = $props();

	// Internal mutable order — initialized from props, overridden by localStorage
	let items = $state<DockItemConfig[]>([...propItems]);

	let device = $state<DeviceClass>('desktop');

	$effect(() => {
		return viewport.subscribe((v) => (device = v.device));
	});

	// Restore saved order from localStorage on mount
	$effect(() => {
		if (typeof localStorage === 'undefined') return;
		const saved = localStorage.getItem('dock-order');
		if (!saved) return;
		try {
			const order: string[] = JSON.parse(saved);
			const sorted = order
				.map((id) => propItems.find((it) => it.id === id))
				.filter(Boolean) as DockItemConfig[];
			// Add any items not in saved order (new items added after last save)
			const missing = propItems.filter((it) => !order.includes(it.id));
			items = [...sorted, ...missing];
		} catch {
			// malformed localStorage entry — use default order
		}
	});

	function handleDragMove(fromIndex: number, toIndex: number) {
		const next = [...items];
		const [moved] = next.splice(fromIndex, 1);
		next.splice(toIndex, 0, moved);
		items = next;
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem('dock-order', JSON.stringify(items.map((i) => i.id)));
		}
	}

	// Drag-reorder is only enabled on non-split layouts (two separate rails
	// can't share a drag context, so reordering is disabled there)
	let canDrag = $derived(device !== 'tablet-landscape');

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
</script>

{#if device === 'tablet-landscape'}
	<nav class="dock-rail dock-rail-left" role="navigation" aria-label="Primary">
		{#each leftItems as item, i (item.id)}
			<DockItem config={item} vertical index={i} draggable={false} />
		{/each}
	</nav>
	<nav class="dock-rail dock-rail-right" role="navigation" aria-label="Secondary">
		{#each rightItems as item, i (item.id)}
			<DockItem config={item} vertical index={midpoint + i} draggable={false} />
		{/each}
	</nav>
{:else}
	<nav class="dock-bar {layoutClass[device]}" role="navigation" aria-label="Primary">
		{#each items as item, i (item.id)}
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
		padding: 8px 16px;
		background: var(--dock-bg, rgba(30, 30, 30, 0.85));
		backdrop-filter: blur(24px) saturate(1.4);
		-webkit-backdrop-filter: blur(24px) saturate(1.4);
		border: 1px solid var(--dock-border, rgba(255, 255, 255, 0.08));
		z-index: 1000;
	}

	.dock-bottom {
		position: fixed;
		bottom: 16px;
		left: 50%;
		transform: translateX(-50%);
	}

	.dock-floating {
		border-radius: 16px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
	}

	.dock-full {
		left: 0;
		right: 0;
		bottom: 0;
		transform: none;
		border-radius: 0;
	}

	.dock-compact {
		border-radius: 16px;
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
		background: var(--dock-bg, rgba(30, 30, 30, 0.85));
		backdrop-filter: blur(24px) saturate(1.4);
		-webkit-backdrop-filter: blur(24px) saturate(1.4);
		border: 1px solid var(--dock-border, rgba(255, 255, 255, 0.08));
		border-radius: 16px;
		z-index: 1000;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
	}

	.dock-rail-left {
		left: 12px;
	}

	.dock-rail-right {
		right: 12px;
	}
</style>
