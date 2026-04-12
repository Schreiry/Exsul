import type { Component } from 'svelte';

export interface DockItemConfig {
	id: string;
	icon: Component;
	label: string;
	href?: string;
	onclick?: () => void;
	badge?: number | string;
	separator_before?: boolean;
}

export interface DockConfig {
	items: DockItemConfig[];
	gap: number;
	padding: number;
	cornerRadius: number;
}
