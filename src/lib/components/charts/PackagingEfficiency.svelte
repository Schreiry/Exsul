<script lang="ts">
	import type { PackagingLogEntry } from '$lib/tauri/types';

	interface Props {
		log: PackagingLogEntry[];
		weeks?: number;
	}

	let { log, weeks = 8 }: Props = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);

	// Group packaging log by ISO week (last N weeks)
	const weeklyData = $derived(() => {
		function isoWeek(date: Date): string {
			const d = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
			d.setUTCDate(d.getUTCDate() + 4 - (d.getUTCDay() || 7));
			const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
			const weekNum = Math.ceil((((d.getTime() - yearStart.getTime()) / 86400000) + 1) / 7);
			return `${d.getUTCFullYear()}-W${String(weekNum).padStart(2, '0')}`;
		}

		// Pre-fill last N weeks
		const weekMap = new Map<string, { packs: number; stems: number }>();
		const now = new Date();
		for (let i = weeks - 1; i >= 0; i--) {
			const d = new Date(now.getTime() - i * 7 * 86_400_000);
			weekMap.set(isoWeek(d), { packs: 0, stems: 0 });
		}

		for (const entry of log) {
			const week = isoWeek(new Date(entry.created_at));
			if (weekMap.has(week)) {
				const cur = weekMap.get(week)!;
				weekMap.set(week, {
					packs: cur.packs + entry.pack_count,
					stems: cur.stems + entry.stems_used,
				});
			}
		}

		return Array.from(weekMap.entries()).map(([week, val]) => ({ week, ...val }));
	});

	const maxPacks = $derived(Math.max(1, ...weeklyData().map(d => d.packs)));

	$effect(() => {
		const c = canvas;
		const data = weeklyData();
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

		const barW = Math.max(4, (w - 20) / data.length - 4);
		const maxH = h - 28;

		data.forEach((d, i) => {
			const barH = d.packs > 0 ? Math.max(4, (d.packs / maxPacks) * maxH) : 0;
			const x = 10 + i * ((w - 20) / data.length);
			const y = h - 24 - barH;

			ctx.fillStyle = primary;
			ctx.globalAlpha = d.packs > 0 ? 0.8 : 0.1;
			ctx.beginPath();
			ctx.roundRect(x, y, barW, Math.max(2, barH), 3);
			ctx.fill();

			if (d.packs > 0) {
				ctx.globalAlpha = 1;
				ctx.fillStyle = primary;
				ctx.font = `bold 9px system-ui`;
				ctx.textAlign = 'center';
				ctx.fillText(String(d.packs), x + barW / 2, y - 3);
			}
			ctx.globalAlpha = 1;

			// Week label
			ctx.fillStyle = outline;
			ctx.font = `9px system-ui`;
			ctx.textAlign = 'center';
			const weekLabel = d.week.split('-W')[1] ? `Н${d.week.split('-W')[1]}` : '';
			ctx.fillText(weekLabel, x + barW / 2, h - 8);
		});
	});
</script>

<div class="chart-wrap">
	{#if log.length === 0}
		<p class="empty-msg">Нет данных об упаковке</p>
	{:else}
		<canvas bind:this={canvas} class="chart-canvas" aria-label="Упаковка по неделям"></canvas>
	{/if}
</div>

<style>
	.chart-wrap {
		width: 100%;
		height: 140px;
		position: relative;
	}

	.chart-canvas {
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
