import { generateFullPalette } from './engine';

/**
 * Apply the generated palette as CSS custom properties on :root.
 */
export function applyTheme(seedHex: string) {
	const palette = generateFullPalette(seedHex);
	const root = document.documentElement;

	root.style.setProperty('--color-primary', palette.primary);
	root.style.setProperty('--color-surface', palette.surface);
	root.style.setProperty('--color-secondary', palette.secondary);
	root.style.setProperty('--color-tertiary', palette.tertiary);
	root.style.setProperty('--color-on-primary', palette.onPrimary);
	root.style.setProperty('--color-on-surface', palette.onSurface);
	root.style.setProperty('--color-surface-container', palette.surfaceContainer);
	root.style.setProperty('--color-surface-container-high', palette.surfaceContainerHigh);
	root.style.setProperty('--color-outline', palette.outline);
	root.style.setProperty('--color-outline-variant', palette.outlineVariant);
	root.style.setProperty('--accent', palette.primary);

	// Dock-specific
	root.style.setProperty('--dock-bg', palette.surfaceContainer + 'd9'); // ~85% opacity via hex alpha
	root.style.setProperty('--dock-border', palette.outlineVariant + '33');
	root.style.setProperty('--dock-fg', palette.onSurface);
}
