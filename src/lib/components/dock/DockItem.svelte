<script lang="ts">
	import type { Component } from 'svelte';
	import type { DockItemConfig } from './types';
	import { createSpring, DOCK_SPRING } from './spring';
	import { page } from '$app/state';

	interface Props {
		config: DockItemConfig;
		vertical?: boolean;
		index?: number;
		draggable?: boolean;
		ondragmove?: (from: number, to: number) => void;
	}

	let { config, vertical = false, index = 0, draggable = false, ondragmove }: Props = $props();

	const scale = createSpring(1, DOCK_SPRING);
	const glow = createSpring(0, DOCK_SPRING);

	let scaleValue = $state(1);
	let glowValue = $state(0);
	let pressed = $state(false);
	let isDragOver = $state(false);

	$effect(() => {
		return scale.subscribe((v) => (scaleValue = v));
	});

	$effect(() => {
		return glow.subscribe((v) => (glowValue = v));
	});

	let isActive = $derived(config.href ? page.url.pathname === config.href : false);

	const IconComponent: Component = config.icon;

	function onPointerEnter() {
		scale.set(1.15);
		glow.set(1);
	}

	function onPointerLeave() {
		scale.set(1);
		glow.set(0);
		pressed = false;
	}

	function onPointerDown() {
		scale.set(0.9);
		pressed = true;
	}

	function onPointerUp() {
		if (pressed) {
			scale.set(1.15);
		}
		pressed = false;
	}

	function onDragStart(e: DragEvent) {
		if (e.dataTransfer) {
			e.dataTransfer.setData('text/plain', String(index));
			e.dataTransfer.effectAllowed = 'move';
		}
	}

	function onDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
		isDragOver = true;
	}

	function onDragLeave() {
		isDragOver = false;
	}

	function onDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		const fromIndex = parseInt(e.dataTransfer?.getData('text/plain') ?? '-1', 10);
		if (fromIndex >= 0 && fromIndex !== index && ondragmove) {
			ondragmove(fromIndex, index);
		}
	}

	function onDragEnd() {
		isDragOver = false;
	}
</script>

{#if config.onclick}
	<button
		type="button"
		class="dock-item"
		class:active={isActive}
		class:vertical
		class:drag-over={isDragOver}
		onpointerenter={onPointerEnter}
		onpointerleave={onPointerLeave}
		onpointerdown={onPointerDown}
		onpointerup={onPointerUp}
		onclick={config.onclick}
		style:transform="scale({scaleValue})"
		style:--glow-opacity={glowValue}
		aria-label={config.label}
		title={config.label}
	>
		<IconComponent />
		{#if config.badge}
			<span class="badge">{config.badge}</span>
		{/if}
	</button>
{:else}
	<a
		href={config.href}
		class="dock-item"
		class:active={isActive}
		class:vertical
		class:drag-over={isDragOver}
		draggable={draggable ? 'true' : undefined}
		onpointerenter={onPointerEnter}
		onpointerleave={onPointerLeave}
		onpointerdown={onPointerDown}
		onpointerup={onPointerUp}
		ondragstart={draggable ? onDragStart : undefined}
		ondragover={draggable ? onDragOver : undefined}
		ondragleave={draggable ? onDragLeave : undefined}
		ondrop={draggable ? onDrop : undefined}
		ondragend={draggable ? onDragEnd : undefined}
		style:transform="scale({scaleValue})"
		style:--glow-opacity={glowValue}
		aria-label={config.label}
		title={config.label}
	>
		<IconComponent />
		{#if config.badge}
			<span class="badge">{config.badge}</span>
		{/if}
	</a>
{/if}

<style>
	.dock-item {
		position: relative;
		width: calc(50px * var(--ui-scale, 1));
		height: calc(50px * var(--ui-scale, 1));
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: calc(12px * var(--ui-scale, 1));
		background: transparent;
		cursor: pointer;
		color: var(--dock-fg, #e0e0e0);
		will-change: transform;
		-webkit-tap-highlight-color: transparent;
		text-decoration: none;
	}

	/* Inner SVG icons grow with the same scale so an icon never looks
	   stranded inside an over-sized dock cell at high zoom. */
	.dock-item :global(svg) {
		width: calc(22px * var(--ui-scale, 1));
		height: calc(22px * var(--ui-scale, 1));
	}

	/* Responsive dock sizing — bumps for very wide screens. Uses the same
	   --ui-scale multiplier so the user's scale slider still has effect. */
	@media (min-width: 1600px) and (max-width: 2399px) {
		.dock-item {
			width: calc(54px * var(--ui-scale, 1));
			height: calc(54px * var(--ui-scale, 1));
			border-radius: calc(13px * var(--ui-scale, 1));
		}
		.dock-item :global(svg) {
			width: calc(24px * var(--ui-scale, 1));
			height: calc(24px * var(--ui-scale, 1));
		}
	}

	@media (min-width: 2400px) {
		.dock-item {
			width: calc(60px * var(--ui-scale, 1));
			height: calc(60px * var(--ui-scale, 1));
			border-radius: calc(14px * var(--ui-scale, 1));
		}
		.dock-item :global(svg) {
			width: calc(26px * var(--ui-scale, 1));
			height: calc(26px * var(--ui-scale, 1));
		}
	}

	.dock-item::before {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		background: var(--dock-item-hover, rgba(255, 255, 255, 0.1));
		opacity: var(--glow-opacity, 0);
		transition: opacity 0.1s;
	}

	.dock-item.drag-over {
		outline: 2px solid var(--color-primary, #34d399);
		outline-offset: 2px;
	}

	.dock-item.active {
		color: var(--accent, #6ee7b7);
	}

	.dock-item.active::after {
		content: '';
		position: absolute;
		bottom: 2px;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--accent, #6ee7b7);
	}

	.vertical.active::after {
		bottom: auto;
		right: 2px;
		left: auto;
	}

	.badge {
		position: absolute;
		top: 2px;
		right: 2px;
		min-width: 16px;
		height: 16px;
		border-radius: 8px;
		background: #ef4444;
		color: white;
		font-size: 10px;
		font-weight: 700;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0 4px;
	}
</style>
