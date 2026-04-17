import { writable, get } from 'svelte/store';
import { commands } from '$lib/tauri/commands';

const KEY_DETAILED_PRICING = 'flower.show_detailed_pricing';

export const showDetailedPricing = writable<boolean>(true);

export async function loadPricingToggle(): Promise<void> {
	try {
		const v = await commands.getSetting(KEY_DETAILED_PRICING);
		if (v === null || v === undefined) {
			showDetailedPricing.set(true);
			return;
		}
		showDetailedPricing.set(v === 'true' || v === '1');
	} catch (e) {
		console.error('Failed to load pricing toggle', e);
		showDetailedPricing.set(true);
	}
}

export async function setPricingToggle(on: boolean): Promise<void> {
	showDetailedPricing.set(on);
	try {
		await commands.setSetting(KEY_DETAILED_PRICING, on ? 'true' : 'false');
	} catch (e) {
		console.error('Failed to save pricing toggle', e);
	}
}

export function getPricingToggleValue(): boolean {
	return get(showDetailedPricing);
}
