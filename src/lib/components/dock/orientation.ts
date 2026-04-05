import { readable } from 'svelte/store';

export type Orientation = 'portrait' | 'landscape';
export type DeviceClass =
	| 'mobile-portrait'
	| 'mobile-landscape'
	| 'tablet-portrait'
	| 'tablet-landscape'
	| 'desktop';

export interface ViewportInfo {
	width: number;
	height: number;
	orientation: Orientation;
	device: DeviceClass;
}

function getViewport(): ViewportInfo {
	if (typeof window === 'undefined') {
		return { width: 1200, height: 800, orientation: 'landscape', device: 'desktop' };
	}

	const width = window.innerWidth;
	const height = window.innerHeight;
	const orientation: Orientation = width >= height ? 'landscape' : 'portrait';
	const minDim = Math.min(width, height);
	const maxDim = Math.max(width, height);

	let device: DeviceClass;
	if (maxDim < 640) {
		device = orientation === 'portrait' ? 'mobile-portrait' : 'mobile-landscape';
	} else if (maxDim < 1024) {
		device = orientation === 'portrait' ? 'tablet-portrait' : 'tablet-landscape';
	} else {
		device = 'desktop';
	}

	return { width, height, orientation, device };
}

export const viewport = readable<ViewportInfo>(getViewport(), (set) => {
	if (typeof window === 'undefined') return;

	const onResize = () => set(getViewport());
	window.addEventListener('resize', onResize);
	return () => window.removeEventListener('resize', onResize);
});
