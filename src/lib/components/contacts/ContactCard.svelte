<script lang="ts">
	import { globalCurrency, formatAmount } from '$lib/stores/currency';
	import { t } from '$lib/stores/i18n';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { appDataDir, join } from '@tauri-apps/api/path';
	import type { Contact } from '$lib/tauri/types';

	interface Props {
		contact: Contact;
		selected?: boolean;
		onclick?: () => void;
		/** Compact print icon appears on hover when this is provided. */
		onprint?: () => void;
	}

	let { contact, selected = false, onclick, onprint }: Props = $props();

	// Derive 1-2 initials for the avatar fallback — works for "Давид", "David
	// Schreiry", "ООО Ромашка".
	const initials = $derived.by(() => {
		const parts = (contact.name || '?').trim().split(/\s+/).filter(Boolean);
		if (parts.length === 0) return '?';
		if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
		return (parts[0][0] + parts[1][0]).toUpperCase();
	});

	const accentColor = $derived(contact.card_color ?? '');

	// Photos live under `app_data_dir/contact_photos/…`. The path is stored as
	// a relative string; to render in the webview we must resolve the full FS
	// path, then run it through convertFileSrc to get an asset:// URL. A naive
	// `/path` src 404s because the webview has no such route.
	let photoSrc = $state<string>('');
	$effect(() => {
		const p = contact.photo_path;
		if (!p) { photoSrc = ''; return; }
		(async () => {
			try {
				const base = await appDataDir();
				const full = await join(base, p);
				photoSrc = convertFileSrc(full);
			} catch (e) {
				console.warn('Failed to resolve contact photo:', e);
				photoSrc = '';
			}
		})();
	});

	const hasPhoto = $derived(!!photoSrc);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	class="contact-card"
	class:selected
	class:has-color={!!accentColor}
	style:--card-accent={accentColor || 'transparent'}
	style:--card-tint={accentColor
		? `color-mix(in srgb, ${accentColor} 10%, transparent)`
		: 'transparent'}
	role="button"
	tabindex="0"
	onclick={() => onclick?.()}
	onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onclick?.()}
>
	<div class="avatar" class:with-photo={hasPhoto}>
		{#if hasPhoto}
			<img src={photoSrc} alt="" />
		{:else}
			<span class="initials">{initials}</span>
		{/if}
	</div>

	{#if onprint && contact.order_count > 0}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<button
			type="button"
			class="card-print-btn"
			onclick={(e) => { e.stopPropagation(); onprint?.(); }}
			title={$t('contact_print_all_orders')}
			aria-label={$t('contact_print_all_orders')}
		>
			<svg viewBox="0 0 24 24" width="12" height="12" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<polyline points="6 9 6 2 18 2 18 9"/>
				<path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>
				<rect x="6" y="14" width="12" height="8"/>
			</svg>
		</button>
	{/if}

	<div class="info">
		<div class="name" title={contact.name}>{contact.name}</div>
		{#if contact.surname}
			<div class="surname" title={contact.surname}>{contact.surname}</div>
		{:else if contact.company}
			<div class="surname" title={contact.company}>{contact.company}</div>
		{/if}

		<div class="metrics">
			<span class="metric">
				<strong>{contact.order_count}</strong>
				<span class="metric-label">{$t('contact_orders_count', { n: contact.order_count })}</span>
			</span>
			{#if contact.total_spent > 0}
				<span class="metric total">
					{formatAmount(contact.total_spent, $globalCurrency)}
				</span>
			{/if}
		</div>
	</div>
</div>

<style>
	.contact-card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		aspect-ratio: 1 / 1;
		padding: 12px;
		background: var(--color-surface-container);
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-left: 3px solid transparent;
		border-radius: var(--glass-radius, 14px);
		cursor: pointer;
		transition: transform 0.12s var(--ease-spring, cubic-bezier(.2,.9,.3,1.4)),
		            box-shadow 0.15s,
		            border-color 0.15s;
		backdrop-filter: blur(14px);
		overflow: hidden;
		text-align: left;
		position: relative;
	}
	.contact-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 6px 20px rgba(0,0,0,0.2);
	}
	.contact-card.selected {
		border-color: var(--color-primary);
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-primary) 35%, transparent);
	}
	.contact-card.has-color {
		border-left-color: var(--card-accent);
		background:
			linear-gradient(var(--card-tint), var(--card-tint)),
			var(--color-surface-container);
	}

	.avatar {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: color-mix(in srgb, var(--color-primary) 14%, transparent);
		color: var(--color-primary);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 1rem;
		flex-shrink: 0;
		overflow: hidden;
	}
	.avatar.with-photo { background: transparent; }
	.avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
	.name {
		font-size: 0.88rem;
		font-weight: 600;
		color: var(--color-on-surface);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.surname {
		font-size: 0.72rem;
		color: var(--color-outline);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.metrics {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-top: auto;
		flex-wrap: wrap;
	}
	.metric {
		display: inline-flex;
		align-items: baseline;
		gap: 4px;
		font-size: 0.72rem;
		color: var(--color-outline);
	}
	.metric strong {
		font-size: 0.95rem;
		color: var(--color-primary);
		font-weight: 700;
	}
	.metric-label { font-size: 0.68rem; opacity: 0.8; }
	.metric.total {
		font-weight: 600;
		color: var(--color-on-surface);
		font-size: 0.78rem;
	}

	.card-print-btn {
		position: absolute;
		top: 6px;
		right: 6px;
		width: 24px;
		height: 24px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: var(--glass-bg, rgba(0,0,0,0.3));
		border: 1px solid var(--glass-border, var(--color-outline-variant));
		border-radius: 6px;
		color: var(--color-on-surface);
		opacity: 0;
		cursor: pointer;
		transition: opacity 0.15s, background 0.15s;
		padding: 0;
		z-index: 2;
	}
	.contact-card:hover .card-print-btn,
	.contact-card:focus-within .card-print-btn { opacity: 0.9; }
	.card-print-btn:hover {
		opacity: 1;
		background: color-mix(in srgb, var(--color-primary) 20%, transparent);
		color: var(--color-primary);
	}
</style>
