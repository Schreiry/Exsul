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

// ── Background image (data URL, persisted in localStorage) ─────────────────
// Stored as a data URL so it survives reloads and works without filesystem
// access. Settings UI clamps the file size before storing to keep the LS
// quota safe.
function getInitialBackground(): string | null {
	if (typeof localStorage === 'undefined') return null;
	return localStorage.getItem('background-image') || null;
}

export const backgroundImage = writable<string | null>(getInitialBackground());

backgroundImage.subscribe((value) => {
	if (typeof localStorage === 'undefined') return;
	if (value) {
		try {
			localStorage.setItem('background-image', value);
		} catch (e) {
			// QuotaExceededError — image too large for localStorage. Drop it
			// silently rather than corrupting other settings; the UI surfaces
			// a separate size warning at upload time.
			console.warn('background-image: localStorage quota exceeded', e);
		}
	} else {
		localStorage.removeItem('background-image');
	}
});

// ── Background overlay strength (0..1, default 0.55) ──────────────────────
// Determines how strongly the theme darkens (dark mode) or lightens (light
// mode) the image so the UI keeps its monochrome feel. Surfacing this lets
// the user dial it in instead of fighting our default.
function getInitialOverlay(): number {
	if (typeof localStorage === 'undefined') return 0.55;
	const v = parseFloat(localStorage.getItem('background-overlay') ?? '0.55');
	return isNaN(v) ? 0.55 : Math.min(0.95, Math.max(0, v));
}

export const backgroundOverlay = writable<number>(getInitialOverlay());

backgroundOverlay.subscribe((v) => {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('background-overlay', String(v));
	}
});
