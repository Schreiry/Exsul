<script lang="ts">
	import type { HarvestLogEntry } from '$lib/tauri/types';

	interface Props {
		entries: HarvestLogEntry[];
		days?: number;
	}

	let { entries, days = 30 }: Props = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);

	// Group entries by day — last N days
	const chartData = $derived(() => {
		const dayMap = new Map<string, number>();
		const now = Date.now();
		for (let i = days - 1; i >= 0; i--) {
			const d = new Date(now - i * 86_400_000);
			dayMap.set(d.toISOString().slice(0, 10), 0);
		}
		for (const e of entries) {
			const day = e.created_at.slice(0, 10);
			if (dayMap.has(day) && e.delta > 0) {
				dayMap.set(day, (dayMap.get(day) ?? 0) + e.delta);
			}
		}
		return Array.from(dayMap.entries()).map(([date, val]) => ({ date, val }));
	});

	const maxVal = $derived(Math.max(1, ...chartData().map(d => d.val)));

	// Render canvas chart
	$effect(() => {
		const c = canvas;
		const data = chartData();
		if (!c || data.length === 0) return;

		const ctx = c.getContext('2d');
		if (!ctx) return;

		const dpr = window.devicePixelRatio || 1;
		const w = c.offsetWidth;
		const h = c.offsetHeight;
		c.width = w * dpr;
		c.height = h * dpr;
		ctx.scale(dpr, dpr);

		const primary = getComputedStyle(c).getPropertyValue('--color-primary').trim() || '#34d399';
		const outline = getComputedStyle(c).getPropertyValue('--color-outline').trim() || '#555';

		ctx.clearRect(0, 0, w, h);

		const barW = Math.max(2, (w - 20) / data.length - 2);
		const maxH = h - 24;

		data.forEach((d, i) => {
			const barH = d.val > 0 ? Math.max(3, (d.val / maxVal) * maxH) : 0;
			const x = 10 + i * ((w - 20) / data.length);
			const y = h - 20 - barH;

			// Bar
			ctx.fillStyle = d.val > 0 ? primary : outline;
			ctx.globalAlpha = d.val > 0 ? 0.75 : 0.12;
			ctx.beginPath();
			ctx.roundRect(x, y, barW, barH, 2);
			ctx.fill();
			ctx.globalAlpha = 1;
		});

		// X-axis labels: first and last day
		ctx.fillStyle = outline;
		ctx.font = `10px system-ui, sans-serif`;
		ctx.textAlign = 'left';
		if (data.length > 0) {
			const firstDate = new Date(data[0].date);
			ctx.fillText(firstDate.toLocaleDateString('ru', { day: '2-digit', month: 'short' }), 10, h - 4);
		}
		if (data.length > 1) {
			const lastDate = new Date(data[data.length - 1].date);
			const txt = lastDate.toLocaleDateString('ru', { day: '2-digit', month: 'short' });
			ctx.textAlign = 'right';
			ctx.fillText(txt, w - 10, h - 4);
		}
	});
</script>

<div class="timeline-wrap">
	{#if entries.length === 0}
		<p class="empty-msg">Нет данных о сборе урожая</p>
	{:else}
		<canvas bind:this={canvas} class="timeline-canvas" aria-label="График сбора урожая"></canvas>
	{/if}
</div>

<style>
	.timeline-wrap {
		width: 100%;
		height: 120px;
		position: relative;
	}

	.timeline-canvas {
		width: 100%;
		height: 100%;
		display: block;
	}

	.empty-msg {
		font-size: 0.82rem;
		color: var(--color-outline);
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		margin: 0;
	}
</style>
