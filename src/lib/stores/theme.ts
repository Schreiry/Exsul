import { writable } from 'svelte/store';

// Default seed color — a teal/emerald tone
export const seedColor = writable<string>('#34d399');

// ── UI Scale (0.8 – 1.4, default 1.0) ────────────────────────────────────────
function getInitialScale(): number {
	if (typeof localStorage === 'undefined') return 1.0;
	const stored = parseFloat(localStorage.getItem('ui-scale') ?? '1');
	return isNaN(stored) ? 1.0 : Math.min(1.4, Math.max(0.8, stored));
}

export const uiScale = writable<number>(getInitialScale());

uiScale.subscribe((scale) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('ui-scale', String(scale));
	}
	if (typeof document !== 'undefined') {
		document.documentElement.style.setProperty('--ui-scale', String(scale));
	}
});

// ── Color mode (dark / light) ─────────────────────────────────────────────────
function getInitialMode(): 'dark' | 'light' {
	if (typeof localStorage === 'undefined') return 'dark';
	const stored = localStorage.getItem('color-mode');
	if (stored === 'light' || stored === 'dark') return stored;
	return 'dark';
}

export const colorMode = writable<'dark' | 'light'>(getInitialMode());

// Persist to localStorage on every change
colorMode.subscribe((mode) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('color-mode', mode);
	}
});
