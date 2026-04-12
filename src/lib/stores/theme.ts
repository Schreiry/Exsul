import { writable } from 'svelte/store';

// Default seed color — neutral grey (no color bias on first launch)
const DEFAULT_SEED = '#6b7280';

function getInitialSeed(): string {
	if (typeof localStorage === 'undefined') return DEFAULT_SEED;
	const stored = localStorage.getItem('seed-color');
	return stored ?? DEFAULT_SEED;
}

export const seedColor = writable<string>(getInitialSeed());

seedColor.subscribe((color) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('seed-color', color);
	}
});

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

// ── Palette mode (default / monochrome) ──────────────────────────────────────
export type PaletteMode = 'default' | 'monochrome';

function getInitialPalette(): PaletteMode {
	if (typeof localStorage === 'undefined') return 'default';
	const stored = localStorage.getItem('palette-mode');
	if (stored === 'monochrome') return 'monochrome';
	return 'default';
}

export const paletteMode = writable<PaletteMode>(getInitialPalette());

paletteMode.subscribe((mode) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('palette-mode', mode);
	}
});
