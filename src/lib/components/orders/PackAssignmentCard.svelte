<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { t } from '$lib/stores/i18n';
	import type { PackAssignment, FlowerSort } from '$lib/tauri/types';

	interface Props {
		assignment: PackAssignment;
		sort?: FlowerSort;
		ondelete?: () => void;
	}

	let { assignment, sort, ondelete }: Props = $props();

	let appDataDir = $state('');

	$effect(() => {
		import('@tauri-apps/api/path').then(({ appDataDir: getDir }) =>
			getDir().then((dir) => { appDataDir = dir; })
		).catch(() => {});
	});

	function resolvePhotoSrc(photoPath: string | null | undefined, baseDir: string): string | null {
		if (!photoPath) return null;
		if (photoPath.includes(':') || photoPath.startsWith('/')) {
			return convertFileSrc(photoPath);
		}
		if (!baseDir) return null;
		const base = baseDir.endsWith('\\') || baseDir.endsWith('/') ? baseDir : baseDir + '/';
		return convertFileSrc(base + photoPath.replace(/\\/g, '/'));
	}

	const photoSrc = $derived(resolvePhotoSrc(sort?.photo_path, appDataDir));

	const cardStyle = $derived(
		sort?.color_hex ? `--card-color: ${sort.color_hex};` : ''
	);

	const initials = $derived(
		(sort?.name ?? assignment.sort_id ?? '?').slice(0, 2).toUpperCase()
	);

	const totalStems = $derived(assignment.pack_count * assignment.stems_per_pack);

	const createdLabel = $derived(
		new Date(assignment.created_at).toLocaleString('ru', {
			day: '2-digit',
			month: 'short',
			hour: '2-digit',
			minute: '2-digit',
		})
	);
</script>

<div
	class="pack-card"
	class:has-color={!!sort?.color_hex}
	style={cardStyle}
>
	<div class="pack-photo">
		{#if photoSrc}
			<img src={photoSrc} alt={sort?.name ?? assignment.sort_id} class="pack-img" />
		{:else}
			<div class="pack-placeholder" aria-hidden="true">{initials}</div>
		{/if}
		<span class="status-badge status-{assignment.status}">
			{$t('pack_status_' + assignment.status)}
		</span>
	</div>

	<div class="pack-info">
		<p class="pack-name">{sort?.name ?? assignment.sort_id}</p>
		{#if sort?.variety}
			<p class="pack-variety">{sort.variety}</p>
		{/if}

		<div class="pack-counts">
			<span class="count-main">{assignment.pack_count}</span>
			<span class="count-unit"> уп.</span>
			<span class="count-sep">×</span>
			<span class="count-stems">{assignment.stems_per_pack} шт.</span>
		</div>

		<div class="pack-footer">
			<span class="pack-date">{createdLabel}</span>
			{#if ondelete}
				<button
					type="button"
					class="btn-delete"
					onclick={ondelete}
					title={$t('action_delete_assignment')}
					aria-label={$t('action_delete_assignment')}
				>×</button>
			{/if}
		</div>
		{#if assignment.pack_count > 0 && totalStems > 0}
			<p class="pack-total-stems">{totalStems} стеблей всего</p>
		{/if}
	</div>
</div>

<style>
	.pack-card {
		display: flex;
		flex-direction: column;
		background: var(--glass-bg, rgba(255,255,255,0.04));
		border: 1px solid var(--glass-border, rgba(255,255,255,0.09));
		border-radius: 14px;
		overflow: hidden;
		transition: transform 0.15s var(--ease-spring), box-shadow 0.15s, border-color 0.15s;
	}

	.pack-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 8px 24px rgba(0,0,0,0.2);
		border-color: var(--color-outline, rgba(255,255,255,0.18));
	}

	.pack-card.has-color {
		background:
			linear-gradient(135deg,
				color-mix(in srgb, var(--card-color) 28%, var(--glass-bg)) 0%,
				color-mix(in srgb, var(--card-color) 8%, var(--glass-bg)) 100%);
		border-color: color-mix(in srgb, var(--card-color) 38%, var(--glass-border));
		box-shadow:
			inset 0 1px 0 color-mix(in srgb, var(--card-color) 18%, transparent),
			0 2px 10px color-mix(in srgb, var(--card-color) 10%, transparent);
	}

	.pack-card.has-color:hover {
		box-shadow:
			0 8px 24px rgba(0,0,0,0.2),
			0 0 22px color-mix(in srgb, var(--card-color) 22%, transparent);
		border-color: color-mix(in srgb, var(--card-color) 50%, var(--glass-border));
	}

	.pack-photo {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		background: color-mix(in srgb, var(--color-primary) 8%, transparent);
		overflow: hidden;
		flex-shrink: 0;
	}

	.pack-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.pack-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.4rem;
		font-weight: 700;
		color: var(--color-primary);
		opacity: 0.7;
		letter-spacing: 0.04em;
	}

	.status-badge {
		position: absolute;
		top: 6px;
		right: 6px;
		padding: 2px 8px;
		border-radius: 999px;
		font-size: 0.65rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		border: 1px solid var(--glass-border);
		background: rgba(0,0,0,0.55);
		backdrop-filter: blur(4px);
		color: var(--color-on-surface);
	}
	.status-badge.status-prepared {
		background: color-mix(in srgb, #f59e0b 80%, rgba(0,0,0,0.4));
		border-color: color-mix(in srgb, #f59e0b 55%, transparent);
		color: #fff;
	}
	.status-badge.status-loaded {
		background: color-mix(in srgb, #3b82f6 80%, rgba(0,0,0,0.4));
		border-color: color-mix(in srgb, #3b82f6 55%, transparent);
		color: #fff;
	}
	.status-badge.status-delivered {
		background: color-mix(in srgb, #10b981 80%, rgba(0,0,0,0.4));
		border-color: color-mix(in srgb, #10b981 55%, transparent);
		color: #fff;
	}

	.pack-info {
		padding: 10px 12px 10px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.pack-name {
		font-size: 0.92rem;
		font-weight: 600;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin: 0;
	}

	.pack-variety {
		font-size: 0.72rem;
		color: var(--color-outline);
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pack-counts {
		margin-top: 4px;
		display: flex;
		align-items: baseline;
		gap: 4px;
		flex-wrap: wrap;
		font-variant-numeric: tabular-nums;
	}
	.count-main {
		font-size: 1.4rem;
		font-weight: 700;
		color: var(--color-primary);
		line-height: 1;
	}
	.count-unit {
		font-size: 0.78rem;
		color: var(--color-outline);
	}
	.count-sep {
		font-size: 0.85rem;
		color: var(--color-outline);
		opacity: 0.6;
	}
	.count-stems {
		font-size: 0.82rem;
		color: var(--color-on-surface);
		opacity: 0.85;
	}

	.pack-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		margin-top: 6px;
	}

	.pack-date {
		font-size: 0.7rem;
		color: var(--color-outline);
		white-space: nowrap;
	}

	.pack-total-stems {
		font-size: 0.7rem;
		color: var(--color-outline);
		opacity: 0.8;
		margin: 2px 0 0;
	}

	.btn-delete {
		background: transparent;
		border: 1px solid transparent;
		color: var(--color-outline);
		font-size: 1rem;
		line-height: 1;
		cursor: pointer;
		padding: 2px 8px;
		border-radius: 6px;
		transition: background 0.1s, color 0.1s, border-color 0.1s;
	}
	.btn-delete:hover {
		background: color-mix(in srgb, var(--color-alert-red, #ef4444) 14%, transparent);
		border-color: color-mix(in srgb, var(--color-alert-red, #ef4444) 40%, transparent);
		color: var(--color-alert-red, #ef4444);
	}
</style>
