import { writable, get } from 'svelte/store';

// ── Supported currencies (RUB excluded) ──────────────────────────────────────
export interface CurrencyDef {
	code: string;
	symbol: string;
	name: string;
	locale: string;
}

export const CURRENCIES: CurrencyDef[] = [
	{ code: 'USD', symbol: '$',    name: 'US Dollar',          locale: 'en-US' },
	{ code: 'EUR', symbol: '€',    name: 'Euro',               locale: 'de-DE' },
	{ code: 'GEL', symbol: '₾',    name: 'Georgian Lari',      locale: 'ka-GE' },
	{ code: 'GBP', symbol: '£',    name: 'British Pound',      locale: 'en-GB' },
	{ code: 'AED', symbol: 'د.إ',  name: 'UAE Dirham',         locale: 'ar-AE' },
	{ code: 'TRY', symbol: '₺',    name: 'Turkish Lira',       locale: 'tr-TR' },
	{ code: 'UAH', symbol: '₴',    name: 'Ukrainian Hryvnia',  locale: 'uk-UA' },
	{ code: 'KZT', symbol: '₸',    name: 'Kazakhstani Tenge',  locale: 'kk-KZ' },
	{ code: 'UZS', symbol: 'сўм',  name: 'Uzbekistani Sum',    locale: 'uz-UZ' },
	{ code: 'CNY', symbol: '¥',    name: 'Chinese Yuan',       locale: 'zh-CN' },
	{ code: 'JPY', symbol: '¥',    name: 'Japanese Yen',       locale: 'ja-JP' },
	{ code: 'CHF', symbol: 'Fr',   name: 'Swiss Franc',        locale: 'de-CH' },
	{ code: 'CAD', symbol: 'C$',   name: 'Canadian Dollar',    locale: 'en-CA' },
	{ code: 'AUD', symbol: 'A$',   name: 'Australian Dollar',  locale: 'en-AU' },
];

export function findCurrency(code: string): CurrencyDef {
	return CURRENCIES.find((c) => c.code === code) ?? CURRENCIES[0];
}

/**
 * Pure format function — no store dependency.
 * Pass the currency code explicitly; components get it from $globalCurrency.
 */
export function formatAmount(value: number, currencyCode: string): string {
	const def = findCurrency(currencyCode);
	try {
		return new Intl.NumberFormat(def.locale, {
			style: 'currency',
			currency: def.code,
			maximumFractionDigits: 2,
		}).format(value);
	} catch {
		return `${def.symbol}${value.toFixed(2)}`;
	}
}

// ── Global currency preference ────────────────────────────────────────────────
function getStoredGlobal(): string {
	if (typeof localStorage === 'undefined') return 'GEL';
	const v = localStorage.getItem('global-currency');
	return CURRENCIES.some((c) => c.code === v) ? (v as string) : 'GEL';
}

export const globalCurrency = writable<string>(getStoredGlobal());

globalCurrency.subscribe((code) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('global-currency', code);
	}
});

// ── Per-item currency overrides ───────────────────────────────────────────────
function loadItemCurrencies(): Record<string, string> {
	if (typeof localStorage === 'undefined') return {};
	try {
		return JSON.parse(localStorage.getItem('item-currencies') ?? '{}');
	} catch {
		return {};
	}
}

function saveItemCurrencies(map: Record<string, string>) {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('item-currencies', JSON.stringify(map));
	}
}

export const itemCurrencies = writable<Record<string, string>>(loadItemCurrencies());
itemCurrencies.subscribe(saveItemCurrencies);

/** Set or clear a per-item currency override. */
export function setItemCurrency(itemId: string, code: string | null) {
	itemCurrencies.update((map) => {
		const next = { ...map };
		if (code) next[itemId] = code;
		else delete next[itemId];
		return next;
	});
}

/** Get resolved currency code for an item (per-item override or global). */
export function getCurrencyForItem(itemId: string): string {
	const overrides = get(itemCurrencies);
	return overrides[itemId] ?? get(globalCurrency);
}
