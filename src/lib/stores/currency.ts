import { writable, get } from 'svelte/store';

// ── Supported currencies (RUB excluded) ──────────────────────────────────────
export interface CurrencyDef {
	code: string;
	symbol: string;
	name: string;
	locale: string;
	/** Approximate exchange rate: 1 unit of this currency = rateToUsd USD */
	rateToUsd: number;
}

export const CURRENCIES: CurrencyDef[] = [
	{ code: 'USD', symbol: '$',    name: 'US Dollar',          locale: 'en-US', rateToUsd: 1.0 },
	{ code: 'EUR', symbol: '€',    name: 'Euro',               locale: 'de-DE', rateToUsd: 1.09 },
	{ code: 'GEL', symbol: '₾',    name: 'Georgian Lari',      locale: 'ka-GE', rateToUsd: 0.37 },
	{ code: 'GBP', symbol: '£',    name: 'British Pound',      locale: 'en-GB', rateToUsd: 1.26 },
	{ code: 'AED', symbol: 'د.إ',  name: 'UAE Dirham',         locale: 'ar-AE', rateToUsd: 0.27 },
	{ code: 'TRY', symbol: '₺',    name: 'Turkish Lira',       locale: 'tr-TR', rateToUsd: 0.031 },
	{ code: 'UAH', symbol: '₴',    name: 'Ukrainian Hryvnia',  locale: 'uk-UA', rateToUsd: 0.024 },
	{ code: 'KZT', symbol: '₸',    name: 'Kazakhstani Tenge',  locale: 'kk-KZ', rateToUsd: 0.002 },
	{ code: 'UZS', symbol: 'сўм',  name: 'Uzbekistani Sum',    locale: 'uz-UZ', rateToUsd: 0.000078 },
	{ code: 'CNY', symbol: '¥',    name: 'Chinese Yuan',       locale: 'zh-CN', rateToUsd: 0.14 },
	{ code: 'JPY', symbol: '¥',    name: 'Japanese Yen',       locale: 'ja-JP', rateToUsd: 0.0067 },
	{ code: 'CHF', symbol: 'Fr',   name: 'Swiss Franc',        locale: 'de-CH', rateToUsd: 1.12 },
	{ code: 'CAD', symbol: 'C$',   name: 'Canadian Dollar',    locale: 'en-CA', rateToUsd: 0.73 },
	{ code: 'AUD', symbol: 'A$',   name: 'Australian Dollar',  locale: 'en-AU', rateToUsd: 0.65 },
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

/**
 * Convert an amount from one currency to another using approximate USD rates.
 * Rates are static approximations — not live market data.
 */
export function convertAmount(amount: number, fromCode: string, toCode: string): number {
	if (fromCode === toCode) return amount;
	const from = findCurrency(fromCode);
	const to = findCurrency(toCode);
	const usd = amount * from.rateToUsd;
	return to.rateToUsd > 0 ? usd / to.rateToUsd : usd;
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
