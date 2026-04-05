<script lang="ts">
	import Dock from '$lib/components/dock/Dock.svelte';
	import SyncModal from '$lib/components/sync/SyncModal.svelte';
	import type { DockItemConfig } from '$lib/components/dock/types';
	import { applyTheme } from '$lib/theme/apply';
	import { seedColor } from '$lib/stores/theme';
	import { inventory } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { preset } from '$lib/stores/preset';
	import { t } from '$lib/stores/i18n';
	import { loadSyncState } from '$lib/stores/sync';
	import IconDashboard from '$lib/components/icons/IconDashboard.svelte';
	import IconInventory from '$lib/components/icons/IconInventory.svelte';
	import IconAnalytics from '$lib/components/icons/IconAnalytics.svelte';
	import IconOrders from '$lib/components/icons/IconOrders.svelte';
	import IconAudit from '$lib/components/icons/IconAudit.svelte';
	import IconSettings from '$lib/components/icons/IconSettings.svelte';
	import IconSync from '$lib/components/icons/IconSync.svelte';
	import '../app.css';

	let { children } = $props();

	let syncOpen = $state(false);

	// Sync dock item uses a special onClick handler instead of href
	const dockItems = $derived<DockItemConfig[]>([
		{ id: 'dashboard',  icon: IconDashboard,  label: $t('nav_dashboard'),  href: '/' },
		{ id: 'inventory',  icon: IconInventory,  label: $t('nav_inventory'),  href: '/inventory' },
		...(($preset === 'flowers')
			? [{ id: 'flowers', icon: IconInventory, label: $t('nav_flowers'), href: '/flowers' }]
			: []),
		{ id: 'analytics',  icon: IconAnalytics,  label: $t('nav_analytics'),  href: '/analytics' },
		{ id: 'orders',     icon: IconOrders,     label: $t('nav_orders'),     href: '/orders' },
		{ id: 'audit',      icon: IconAudit,      label: $t('nav_audit'),      href: '/audit' },
		{ id: 'settings',   icon: IconSettings,   label: $t('nav_settings'),   href: '/settings' },
		{ id: 'sync',       icon: IconSync,       label: $t('nav_sync'),       onclick: () => (syncOpen = true) },
	]);

	$effect(() => {
		return seedColor.subscribe((color) => {
			applyTheme(color);
		});
	});

	$effect(() => {
		inventory.load();
		categories.load();
		preset.load();
		loadSyncState();
	});
</script>

<svelte:head>
	<title>Exsul</title>
</svelte:head>

<main class="app-main">
	{@render children()}
</main>

<Dock items={dockItems} />

<!-- Glassmorphic Sync Modal -->
<SyncModal open={syncOpen} onclose={() => (syncOpen = false)} />

<!-- Exsul logo watermark — bottom corner, ultra-transparent -->
<div class="logo-watermark" aria-hidden="true">Exsul</div>

<style>
	.app-main {
		min-height: 100vh;
		padding: 24px;
		padding-bottom: 96px;
	}

	.logo-watermark {
		position: fixed;
		bottom: 18px;
		left: 20px;
		font-size: 1.6rem;
		font-weight: 900;
		letter-spacing: -0.04em;
		color: rgba(255, 255, 255, 0.06);
		pointer-events: none;
		user-select: none;
		z-index: 999;
		backdrop-filter: none;
	}
</style>
