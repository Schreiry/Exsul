<script lang="ts">
	import Dock from '$lib/components/dock/Dock.svelte';
	import type { DockItemConfig } from '$lib/components/dock/types';
	import { applyTheme } from '$lib/theme/apply';
	import { seedColor } from '$lib/stores/theme';
	import { inventory } from '$lib/stores/inventory';
	import { categories } from '$lib/stores/categories';
	import { t } from '$lib/stores/i18n';
	import IconDashboard from '$lib/components/icons/IconDashboard.svelte';
	import IconInventory from '$lib/components/icons/IconInventory.svelte';
	import IconAnalytics from '$lib/components/icons/IconAnalytics.svelte';
	import IconOrders from '$lib/components/icons/IconOrders.svelte';
	import IconAudit from '$lib/components/icons/IconAudit.svelte';
	import IconSettings from '$lib/components/icons/IconSettings.svelte';
	import '../app.css';

	let { children } = $props();

	const dockItems: DockItemConfig[] = [
		{ id: 'dashboard', icon: IconDashboard, label: $t('nav_dashboard'), href: '/' },
		{ id: 'inventory', icon: IconInventory, label: $t('nav_inventory'), href: '/inventory' },
		{ id: 'analytics', icon: IconAnalytics, label: $t('nav_analytics'), href: '/analytics' },
		{ id: 'orders', icon: IconOrders, label: $t('nav_orders'), href: '/orders' },
		{ id: 'audit', icon: IconAudit, label: $t('nav_audit'), href: '/audit' },
		{ id: 'settings', icon: IconSettings, label: $t('nav_settings'), href: '/settings' },
	];

	$effect(() => {
		return seedColor.subscribe((color) => {
			applyTheme(color);
		});
	});

	$effect(() => {
		inventory.load();
		categories.load();
	});
</script>

<svelte:head>
	<title>Exsul</title>
</svelte:head>

<main class="app-main">
	{@render children()}
</main>

<Dock items={dockItems} />

<style>
	.app-main {
		min-height: 100vh;
		padding: 24px;
		padding-bottom: 96px;
	}
</style>
