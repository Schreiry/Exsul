import { writable } from 'svelte/store';

// Default seed color — a teal/emerald tone
export const seedColor = writable<string>('#34d399');

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
