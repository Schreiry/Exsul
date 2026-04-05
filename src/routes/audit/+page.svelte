<script lang="ts">
	import { auditLog } from '$lib/stores/audit';
	import { t } from '$lib/stores/i18n';
	import type { AuditLogFilter } from '$lib/tauri/types';

	let filterUser = $state('');
	let filterAction = $state('');
	let filterSince = $state('');
	let filterUntil = $state('');

	let expandedRow = $state<number | null>(null);

	$effect(() => {
		auditLog.load();
	});

	function applyFilters() {
		const filter: AuditLogFilter = {
			user_id: filterUser.trim() || undefined,
			action: filterAction.trim() || undefined,
			since: filterSince || undefined,
			until: filterUntil || undefined,
			limit: 500,
		};
		auditLog.load(filter);
	}

	function toggleRow(id: number) {
		expandedRow = expandedRow === id ? null : id;
	}

	function formatTimestamp(ts: string): string {
		try {
			return new Date(ts).toLocaleString();
		} catch {
			return ts;
		}
	}

	function formatPayload(payload: Record<string, unknown>): string {
		return JSON.stringify(payload, null, 2);
	}

	function payloadPreview(payload: Record<string, unknown>): string {
		const str = JSON.stringify(payload);
		return str.length > 60 ? str.slice(0, 60) + '…' : str;
	}
</script>

<div class="page">
	<h1>{$t('page_audit_title')}</h1>

	<div class="filter-bar">
		<input
			type="text"
			placeholder="{$t('label_user')}…"
			bind:value={filterUser}
		/>
		<input
			type="text"
			placeholder="{$t('label_action')}…"
			bind:value={filterAction}
		/>
		<input type="date" bind:value={filterSince} title="Since" />
		<input type="date" bind:value={filterUntil} title="Until" />
		<button class="btn-primary" onclick={applyFilters}>{$t('action_apply')}</button>
	</div>

	{#if $auditLog.length === 0}
		<p class="empty">{$t('empty_no_audit')}</p>
	{:else}
		<div class="table-wrap">
			<table>
				<thead>
					<tr>
						<th>{$t('label_timestamp')}</th>
						<th>{$t('label_user')}</th>
						<th>{$t('label_action')}</th>
						<th>{$t('label_payload')}</th>
					</tr>
				</thead>
				<tbody>
					{#each $auditLog as log (log.id)}
						<tr
							class="log-row"
							class:expanded={expandedRow === log.id}
							onclick={() => toggleRow(log.id)}
							role="button"
							tabindex="0"
							onkeydown={(e) => e.key === 'Enter' && toggleRow(log.id)}
						>
							<td class="ts">{formatTimestamp(log.timestamp)}</td>
							<td class="user">{log.user_id}</td>
							<td class="action"><span class="action-badge">{log.action}</span></td>
							<td class="payload-cell">
								{#if expandedRow === log.id}
									<pre class="payload-full">{formatPayload(log.payload)}</pre>
								{:else}
									<span class="payload-preview">{payloadPreview(log.payload)}</span>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.page { max-width: 1100px; margin: 0 auto; }

	h1 {
		font-size: 1.75rem;
		font-weight: 600;
		color: var(--color-on-surface);
		margin: 0 0 24px;
	}

	.filter-bar {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
		align-items: center;
		margin-bottom: 20px;
		padding: 16px;
		background: var(--color-surface-container);
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
	}

	.filter-bar input {
		background: var(--color-surface-container-high);
		border: 1px solid var(--color-outline-variant);
		border-radius: 8px;
		padding: 8px 12px;
		color: var(--color-on-surface);
		font-size: 0.875rem;
		font-family: inherit;
		outline: none;
		min-width: 140px;
		flex: 1;
	}

	.filter-bar input:focus {
		border-color: var(--color-primary);
	}

	.btn-primary {
		background: var(--color-primary);
		color: var(--color-on-primary);
		border: none;
		border-radius: 8px;
		padding: 8px 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		white-space: nowrap;
		transition: opacity 0.15s;
	}

	.btn-primary:hover { opacity: 0.85; }

	.empty {
		color: var(--color-on-surface);
		opacity: 0.5;
		text-align: center;
		padding: 60px 0;
	}

	.table-wrap {
		overflow-x: auto;
		border: 1px solid var(--color-outline-variant);
		border-radius: 12px;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.875rem;
	}

	thead tr {
		background: var(--color-surface-container);
		border-bottom: 1px solid var(--color-outline-variant);
	}

	th {
		padding: 12px 16px;
		text-align: left;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--color-on-surface);
		opacity: 0.6;
		white-space: nowrap;
	}

	.log-row {
		cursor: pointer;
		border-bottom: 1px solid var(--color-outline-variant);
		transition: background 0.1s;
	}

	.log-row:last-child { border-bottom: none; }

	.log-row:hover {
		background: var(--color-surface-container-high);
	}

	.log-row.expanded {
		background: var(--color-surface-container);
	}

	td {
		padding: 10px 16px;
		color: var(--color-on-surface);
		vertical-align: top;
	}

	.ts {
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
		opacity: 0.8;
		font-size: 0.8rem;
	}

	.user {
		white-space: nowrap;
		opacity: 0.7;
	}

	.action-badge {
		display: inline-block;
		background: color-mix(in srgb, var(--color-primary) 15%, transparent);
		color: var(--color-primary);
		border-radius: 6px;
		padding: 2px 8px;
		font-size: 0.75rem;
		font-weight: 600;
		white-space: nowrap;
	}

	.payload-cell { max-width: 480px; }

	.payload-preview {
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		opacity: 0.6;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		display: block;
	}

	.payload-full {
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: var(--color-on-surface);
		opacity: 0.85;
		white-space: pre-wrap;
		word-break: break-all;
		background: var(--color-surface-container-high);
		border-radius: 6px;
		padding: 10px;
		margin: 4px 0 0;
		max-height: 240px;
		overflow-y: auto;
	}
</style>
