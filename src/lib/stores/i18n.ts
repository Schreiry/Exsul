import { derived, writable } from 'svelte/store';
import en from '$lib/i18n/en.json';
import ru from '$lib/i18n/ru.json';

export type Locale = 'en' | 'ru';

type Translations = Record<string, string>;
const translations: Record<Locale, Translations> = { en, ru };

function getInitialLocale(): Locale {
	if (typeof localStorage === 'undefined') return 'en';
	const stored = localStorage.getItem('locale');
	if (stored === 'en' || stored === 'ru') return stored;
	return 'en';
}

export const locale = writable<Locale>(getInitialLocale());

locale.subscribe((l) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('locale', l);
	}
});

/**
 * Reactive translation function. Usage in templates: `$t('key')`.
 * Supports simple `{n}` interpolation for numeric values.
 */
export const t = derived(locale, ($locale) => {
	const dict = translations[$locale] ?? en;
	return (key: string, vars?: Record<string, string | number>): string => {
		let str = dict[key] ?? key;
		if (vars) {
			for (const [k, v] of Object.entries(vars)) {
				str = str.replace(`{${k}}`, String(v));
			}
		}
		return str;
	};
});
