<script lang="ts">
	import { onMount } from 'svelte';
	import { flowerSorts, totalRawStems, totalPacks, flowerConstants } from '$lib/stores/flowers';
	import { commands } from '$lib/tauri/commands';
	import type { FlowerSort, PackAssignment } from '$lib/tauri/types';
	import FlowerCard from '$lib/components/greenhouse/FlowerCard.svelte';
	import FlowerDetailModal from '$lib/components/greenhouse/FlowerDetailModal.svelte';
	import AddSortModal from '$lib/components/greenhouse/AddSortModal.svelte';
	import SortManagerPanel from '$lib/components/greenhouse/SortManagerPanel.svelte';

	// ── State ─────────────────────────────────────────────────────
	let detailSort = $state<FlowerSort | null>(null);
	let addOpen = $state(false);
	let manageSorts = $state(false);
	let searchQuery = $state('');
	let filterMode = $state<'all' | 'raw' | 'pkg'>('all');
	let packAssignments = $state<PackAssignment[]>([]);

	// Reload reservations after any navigation back to this page. Pure read,
	// cheap enough (single SELECT). Order mutations elsewhere will be picked up
	// next time the user visits greenhouse; badges aren't live-synced because
	// reservations only change when the operator explicitly packs/assigns.
	async function loadAssignments() {
		try {
			packAssignments = await commands.getPackAssignments();
		} catch (e) {
			console.warn('Failed to load pack assignments:', e);
			packAssignments = [];
		}
	}

	// ── Load ──────────────────────────────────────────────────────
	onMount(async () => {
		await flowerSorts.load();
		await flowerConstants.load();
		await loadAssignments();
	});

	// ── Derived: reservations per sort ────────────────────────────
	// Built once per pack_assignments change so the O(N) scan doesn't run per
	// card. `delivered` rows are excluded — those packs have physically left
	// and shouldn't count as active reservations anymore.
	const reservationsBySort = $derived.by(() => {
		const reserved: Record<string, number> = {};
		const orderSets: Record<string, Set<string>> = {};
		for (const a of packAssignments) {
			if (!a.order_id) continue;
			if (a.status === 'delivered') continue;
			reserved[a.sort_id] = (reserved[a.sort_id] ?? 0) + a.pack_count;
			if (!orderSets[a.sort_id]) orderSets[a.sort_id] = new Set();
			orderSets[a.sort_id].add(a.order_id);
		}
		const waiting: Record<string, number> = {};
		for (const [sid, set] of Object.entries(orderSets)) waiting[sid] = set.size;
		return { reserved, waiting };
	});

	// ── Derived ───────────────────────────────────────────────────
	const totalHarvested = $derived(
		$flowerSorts.reduce((s, x) => s + (x.total_harvested ?? 0), 0)
	);

	const visibleSorts = $derived(() => {
		let list = $flowerSorts;
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			list = list.filter(
				(s) => s.name.toLowerCase().includes(q) || (s.variety ?? '').toLowerCase().includes(q)
			);
		}
		if (filterMode === 'raw') list = list.filter((s) => s.raw_stock > 0);
		if (filterMode === 'pkg') list = list.filter((s) => s.pkg_stock > 0);
		return list;
	});
</script>

<div class="greenhouse-page">

	<!-- ── Header ───────────────────────────────────────────── -->
	<div class="page-header">
		<div class="header-left">
			<h1 class="page-title">Теплица</h1>
			<p class="page-sub">Управление сырьём и урожаем</p>
		</div>
		<div class="header-actions">
			<button
				type="button"
				class="btn-toggle"
				class:active={manageSorts}
				onclick={() => (manageSorts = !manageSorts)}
			>
				<svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
				Сорта
			</button>
			<button
				type="button"
				class="btn-add"
				onclick={() => (addOpen = true)}
			>
				+ Добавить сырьё
			</button>
		</div>
	</div>

	<!-- ── KPI Strip ─────────────────────────────────────────── -->
	<div class="kpi-strip">
		<div class="kpi-card">
			<span class="kpi-val">{$totalRawStems}</span>
			<span class="kpi-label">Стеблей в наличии</span>
		</div>
		<div class="kpi-card">
			<span class="kpi-val">{$totalPacks}</span>
			<span class="kpi-label">Упаковок готово</span>
		</div>
		<div class="kpi-card">
			<span class="kpi-val">{$flowerSorts.length}</span>
			<span class="kpi-label">Видов сырья</span>
		</div>
		<div class="kpi-card">
			<span class="kpi-val">{totalHarvested}</span>
			<span class="kpi-label">Всего собрано</span>
		</div>
	</div>

	<!-- ── Controls ──────────────────────────────────────────── -->
	<div class="controls-row">
		<input
			type="search"
			class="search-input"
			placeholder="Поиск…"
			bind:value={searchQuery}
		/>
		<div class="filter-tabs">
			{#each [['all','Все'],['raw','Есть стебли'],['pkg','Упакованы']] as [val, lbl]}
				<button
					type="button"
					class="filter-tab"
					class:active={filterMode === val}
					onclick={() => (filterMode = val as 'all' | 'raw' | 'pkg')}
				>{lbl}</button>
			{/each}
		</div>
	</div>

	<!-- ── Sort Manager Panel ────────────────────────────────── -->
	{#if manageSorts}
		<SortManagerPanel sorts={$flowerSorts} />
	{/if}

	<!-- ── Card Grid ─────────────────────────────────────────── -->
	{#if $flowerSorts.length === 0}
		<div class="empty-state">
			<div class="empty-icon" aria-hidden="true">
				<svg viewBox="0 0 24 24" width="48" height="48" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" opacity="0.3">
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(72 12 12)"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(144 12 12)"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(216 12 12)"/>
					<path d="M12 12 C10.5 10 10.5 7.5 12 7 C13.5 7.5 13.5 10 12 12" transform="rotate(288 12 12)"/>
					<circle cx="12" cy="12" r="2"/>
				</svg>
			</div>
			<p class="empty-title">Пока нет сырья</p>
			<p class="empty-sub">Нажмите «+ Добавить сырьё», чтобы начать</p>
			<button type="button" class="btn-add" onclick={() => (addOpen = true)}>+ Добавить первое сырьё</button>
		</div>
	{:else if visibleSorts().length === 0}
		<p class="no-results">Ничего не найдено</p>
	{:else}
		<div class="card-grid">
			{#each visibleSorts() as sort (sort.id)}
				<FlowerCard
					{sort}
					reservedPacks={reservationsBySort.reserved[sort.id] ?? 0}
					waitingOrders={reservationsBySort.waiting[sort.id] ?? 0}
					onclick={() => (detailSort = sort)}
				/>
			{/each}
		</div>
	{/if}

</div>

<!-- ── Modals ──────────────────────────────────────────────── -->
{#if detailSort}
	<FlowerDetailModal
		bind:sort={detailSort}
		onclose={() => { detailSort = null; loadAssignments(); }}
	/>
{/if}

{#if addOpen}
	<AddSortModal onclose={() => (addOpen = false)} />
{/if}

<style>
	.greenhouse-page {
		display: flex;
		flex-direction: column;
		gap: 20px;
		min-height: 100%;
	}

	.page-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
	}

	.page-title {
		font-size: 1.8rem;
		font-weight: 800;
		letter-spacing: -0.03em;
		margin: 0;
		color: var(--color-on-surface);
	}

	.page-sub {
		font-size: 0.82rem;
		color: var(--color-outline);
		margin: 2px 0 0;
	}

	.header-actions {
		display: flex;
		gap: 10px;
		align-items: center;
		flex-shrink: 0;
	}

	.btn-add {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		border: none;
		border-radius: 12px;
		padding: 10px 18px;
		font-size: 0.9rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
		white-space: nowrap;
	}
	.btn-add:hover { opacity: 0.88; }

	.btn-toggle {
		display: flex;
		align-items: center;
		gap: 6px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 12px;
		padding: 9px 14px;
		font-size: 0.85rem;
		color: var(--color-outline);
		cursor: pointer;
		transition: color 0.15s, background 0.15s, border-color 0.15s;
	}
	.btn-toggle:hover, .btn-toggle.active {
		color: var(--color-primary);
		border-color: var(--color-primary);
		background: color-mix(in srgb, var(--color-primary) 8%, transparent);
	}

	.kpi-strip {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
	}

	@media (max-width: 700px) {
		.kpi-strip { grid-template-columns: repeat(2, 1fr); }
	}

	.kpi-card {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 14px;
		padding: 14px 16px;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.kpi-val {
		font-size: 1.7rem;
		font-weight: 700;
		color: var(--color-primary);
		line-height: 1;
	}

	.kpi-label {
		font-size: 0.72rem;
		color: var(--color-outline);
	}

	.controls-row {
		display: flex;
		gap: 12px;
		align-items: center;
		flex-wrap: wrap;
	}

	.search-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 8px 14px;
		font-size: 0.9rem;
		color: var(--color-on-surface);
		outline: none;
		flex: 1;
		min-width: 180px;
		max-width: 280px;
		transition: border-color 0.15s;
	}
	.search-input:focus { border-color: var(--color-primary); }

	.filter-tabs {
		display: flex;
		gap: 4px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 10px;
		padding: 3px;
	}

	.filter-tab {
		background: none;
		border: none;
		border-radius: 8px;
		padding: 5px 12px;
		font-size: 0.82rem;
		color: var(--color-outline);
		cursor: pointer;
		transition: background 0.12s, color 0.12s;
		white-space: nowrap;
	}
	.filter-tab.active {
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
	}

	.card-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 16px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 60px 24px;
		text-align: center;
		color: var(--color-on-surface);
	}

	.empty-icon { color: var(--color-primary); }
	.empty-title { font-size: 1.1rem; font-weight: 600; margin: 0; opacity: 0.7; }
	.empty-sub { font-size: 0.85rem; color: var(--color-outline); margin: 0; }
	.no-results { font-size: 0.9rem; color: var(--color-outline); text-align: center; padding: 40px 0; margin: 0; }
</style>
