<script lang="ts">
	import { auditLog } from '$lib/stores/audit';
	import { t } from '$lib/stores/i18n';
	import { formatDateTime, formatTime, formatDayLabel, groupByDay } from '$lib/utils/time';
	import type { AuditLogFilter } from '$lib/tauri/types';

	let filterSince = $state('');
	let filterUntil = $state('');
	let filterType = $state('');
	let expandedId = $state<number | null>(null);

	$effect(() => {
		auditLog.load({ limit: 500 });
	});

	function applyFilters() {
		const filter: AuditLogFilter = {
			action: filterType.trim() || undefined,
			since: filterSince || undefined,
			until: filterUntil || undefined,
			limit: 500,
		};
		auditLog.load(filter);
	}

	function resetFilters() {
		filterSince = '';
		filterUntil = '';
		filterType = '';
		auditLog.load({ limit: 500 });
	}

	// ── Action metadata ────────────────────────────────────────
	type ActionMeta = { icon: string; label: string; color: string };

	const ACTION_META: Record<string, ActionMeta> = {
		ItemCreated:        { icon: '➕', label: 'Добавлен товар',       color: '#34d399' },
		ItemUpdated:        { icon: '✏️',  label: 'Изменён товар',        color: '#60a5fa' },
		StockAdjusted:      { icon: '📦', label: 'Изменён остаток',       color: '#a78bfa' },
		PriceChanged:       { icon: '💰', label: 'Изменена цена',         color: '#fbbf24' },
		SaleRecorded:       { icon: '🛒', label: 'Продажа',               color: '#34d399' },
		ItemImageSaved:     { icon: '🖼️', label: 'Фото товара',           color: '#5bb8d0' },
		OrderStatusChanged: { icon: '📋', label: 'Статус заказа',         color: '#f472b6' },
	};

	function getMeta(action: string): ActionMeta {
		return ACTION_META[action] ?? { icon: '🔹', label: action, color: 'var(--color-outline)' };
	}

	// ── Human-readable payload ─────────────────────────────────
	function describePayload(action: string, payload: Record<string, unknown>): string {
		switch (action) {
			case 'SaleRecorded': {
				const q = payload.quantity ?? 1;
				const p = payload.sale_price;
				return p ? `Продано ${q} шт. по ${p}` : `Продано ${q} шт.`;
			}
			case 'StockAdjusted': {
				const d = payload.delta as number;
				return d > 0 ? `Добавлено +${d} шт.` : `Списано ${d} шт.`;
			}
			case 'PriceChanged':
				return `Новая цена: ${payload.new_price}`;
			case 'ItemCreated':
				return `${payload.name ?? ''}${payload.category ? ` · ${payload.category}` : ''}`;
			case 'ItemUpdated': {
				const parts: string[] = [];
				if (payload.name) parts.push(`Имя: «${payload.name}»`);
				if (payload.production_cost !== undefined && payload.production_cost !== null)
					parts.push(`Себест.: ${payload.production_cost}`);
				if (payload.category) parts.push(`Категория: ${payload.category}`);
				return parts.join(' · ') || 'Обновлены данные';
			}
			case 'ItemImageSaved':
				return 'Фото загружено';
			case 'OrderStatusChanged':
				return `Статус → ${payload.status ?? ''}`;
			default:
				return '';
		}
	}

	// ── Group logs by day ──────────────────────────────────────
	const groupedLogs = $derived(
		groupByDay($auditLog, (log) => log.timestamp)
	);

	const sortedDays = $derived(
		[...groupedLogs.keys()].sort((a, b) => b.localeCompare(a))
	);

	// Action type options for filter
	const actionTypes = Object.entries(ACTION_META).map(([k, v]) => ({ id: k, label: v.label }));
</script>

<div class="page">
	<div class="page-header">
		<h1>{$t('page_audit_title')}</h1>
		<span class="entry-count">{$auditLog.length} записей</span>
	</div>

	<!-- Filters -->
	<div class="filter-bar">
		<select class="filter-input" bind:value={filterType}>
			<option value="">Все действия</option>
			{#each actionTypes as at}
				<option value={at.id}>{at.label}</option>
			{/each}
		</select>
		<input class="filter-input date-input" type="date" bind:value={filterSince} title="С даты" />
		<input class="filter-input date-input" type="date" bind:value={filterUntil} title="По дату" />
		<button class="btn-primary" onclick={applyFilters}>Применить</button>
		{#if filterSince || filterUntil || filterType}
			<button class="btn-ghost" onclick={resetFilters}>✕ Сброс</button>
		{/if}
	</div>

	{#if $auditLog.length === 0}
		<p class="empty">Журнал пуст</p>
	{:else}
		<div class="log-feed">
			{#each sortedDays as day}
				{@const entries = groupedLogs.get(day) ?? []}
				<div class="day-group">
					<div class="day-label">
						<span class="day-chip">{formatDayLabel(day)}</span>
						<span class="day-count">{entries.length} событий</span>
					</div>

					<div class="day-entries">
						{#each entries as log (log.id)}
							{@const meta = getMeta(log.action)}
							{@const desc = describePayload(log.action, log.payload)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="entry"
								class:entry-expanded={expandedId === log.id}
								onclick={() => (expandedId = expandedId === log.id ? null : log.id)}
								onkeydown={(e) => e.key === 'Enter' && (expandedId = expandedId === log.id ? null : log.id)}
								role="button"
								tabindex="0"
							>
								<div class="entry-icon" style:color={meta.color}>{meta.icon}</div>
								<div class="entry-body">
									<div class="entry-main">
										<span class="entry-label" style:color={meta.color}>{meta.label}</span>
										{#if desc}
											<span class="entry-desc">{desc}</span>
										{/if}
									</div>
									<time class="entry-time">{formatTime(log.timestamp)}</time>
								</div>
								{#if expandedId === log.id}
									<div class="entry-detail">
										<div class="detail-row">
											<span class="detail-key">Время:</span>
											<span class="detail-val">{formatDateTime(log.timestamp)}</span>
										</div>
										<div class="detail-row">
											<span class="detail-key">Тип:</span>
											<code class="detail-code">{log.action}</code>
										</div>
										{#if Object.keys(log.payload).length > 0}
											<div class="detail-row detail-row--col">
												<span class="detail-key">Данные:</span>
												<pre class="detail-pre">{JSON.stringify(log.payload, null, 2)}</pre>
											</div>
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.page { max-width: 800px; margin: 0 auto; }

	.page-header {
		display: flex;
		align-items: baseline;
		gap: 12px;
		margin-bottom: 20px;
	}

	h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-on-surface);
		margin: 0;
	}

	.entry-count {
		font-size: 0.8rem;
		color: var(--color-outline);
	}

	/* ── Filters ── */
	.filter-bar {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		align-items: center;
		margin-bottom: 24px;
	}

	.filter-input {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-radius: 9px;
		padding: 7px 12px;
		color: var(--color-on-surface);
		font-size: 0.85rem;
		font-family: inherit;
		outline: none;
		flex: 1;
		min-width: 140px;
		transition: border-color 0.15s;
	}

	.filter-input:focus { border-color: var(--color-primary); }
	.date-input { max-width: 160px; }

	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary);
		border: none;
		border-radius: 9px;
		padding: 8px 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		white-space: nowrap;
		font-family: inherit;
		transition: opacity 0.15s;
	}

	.btn-primary:hover { opacity: 0.85; }

	.btn-ghost {
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		color: var(--color-on-surface);
		border-radius: 9px;
		padding: 8px 14px;
		font-size: 0.875rem;
		cursor: pointer;
		font-family: inherit;
		transition: background 0.15s;
	}

	.btn-ghost:hover { background: var(--glass-bg-hover); }

	/* ── Day groups ── */
	.log-feed { display: flex; flex-direction: column; gap: 20px; }

	.day-group { display: flex; flex-direction: column; gap: 2px; }

	.day-label {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-bottom: 8px;
	}

	.day-chip {
		font-size: 0.75rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-outline);
		padding: 3px 10px;
		background: var(--color-surface-container);
		border-radius: 20px;
	}

	.day-count {
		font-size: 0.72rem;
		color: var(--color-outline);
		opacity: 0.6;
	}

	/* ── Entries ── */
	.day-entries {
		display: flex;
		flex-direction: column;
		gap: 1px;
		background: var(--glass-bg);
		border: 1px solid var(--glass-border);
		border-top-color: var(--glass-border-top);
		border-radius: 14px;
		overflow: hidden;
	}

	.entry {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 12px 16px;
		cursor: pointer;
		transition: background 0.1s;
		border-bottom: 1px solid var(--color-outline-variant);
	}

	.entry:last-child { border-bottom: none; }
	.entry:hover { background: var(--glass-bg-hover); }
	.entry:focus-visible { outline: 2px solid var(--color-primary); outline-offset: -2px; }
	.entry-expanded { background: var(--glass-bg-hover); }

	.entry-icon {
		font-size: 1.1rem;
		flex-shrink: 0;
		width: 28px;
		text-align: center;
		margin-top: 1px;
	}

	.entry-body {
		flex: 1;
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		min-width: 0;
	}

	.entry-main {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.entry-label {
		font-size: 0.875rem;
		font-weight: 600;
	}

	.entry-desc {
		font-size: 0.78rem;
		color: var(--color-on-surface);
		opacity: 0.65;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.entry-time {
		font-size: 0.72rem;
		color: var(--color-outline);
		white-space: nowrap;
		flex-shrink: 0;
		font-variant-numeric: tabular-nums;
	}

	/* ── Expanded detail ── */
	.entry-detail {
		grid-column: 1 / -1;
		width: 100%;
		margin-top: 10px;
		padding: 12px;
		background: var(--color-surface-container);
		border-radius: 9px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	/* Restyle entry to be grid when expanded */
	.entry-expanded {
		flex-wrap: wrap;
	}

	.detail-row {
		display: flex;
		align-items: baseline;
		gap: 8px;
		font-size: 0.8rem;
	}

	.detail-row--col { flex-direction: column; align-items: flex-start; }

	.detail-key {
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-outline);
		white-space: nowrap;
		min-width: 50px;
	}

	.detail-val { color: var(--color-on-surface); }

	.detail-code {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		background: var(--color-surface-container-high);
		padding: 1px 6px;
		border-radius: 4px;
		color: var(--color-primary);
	}

	.detail-pre {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--color-on-surface);
		opacity: 0.8;
		white-space: pre-wrap;
		word-break: break-all;
		background: var(--color-surface-container-high);
		border-radius: 6px;
		padding: 8px;
		margin: 2px 0 0;
		max-height: 200px;
		overflow-y: auto;
		width: 100%;
	}

	.empty {
		color: var(--color-outline);
		text-align: center;
		padding: 60px 0;
	}
</style>
