// Time utilities — Tbilisi timezone (Asia/Tbilisi = UTC+4)
const TZ = 'Asia/Tbilisi';

export function formatDateTime(utcStr: string): string {
	try {
		return new Date(utcStr).toLocaleString('ru', {
			timeZone: TZ,
			day: '2-digit',
			month: '2-digit',
			year: '2-digit',
			hour: '2-digit',
			minute: '2-digit',
		});
	} catch {
		return utcStr;
	}
}

export function formatTime(utcStr: string): string {
	try {
		return new Date(utcStr).toLocaleTimeString('ru', {
			timeZone: TZ,
			hour: '2-digit',
			minute: '2-digit',
		});
	} catch {
		return utcStr;
	}
}

export function formatDate(utcStr: string): string {
	try {
		return new Date(utcStr).toLocaleDateString('ru', {
			timeZone: TZ,
			day: '2-digit',
			month: '2-digit',
			year: 'numeric',
		});
	} catch {
		return utcStr;
	}
}

/** Parse HLC timestamp "ms:counter:node_id" → Date */
export function hlcToDate(hlcStr: string): Date | null {
	try {
		const ms = parseInt(hlcStr.split(':')[0], 10);
		return isNaN(ms) ? null : new Date(ms);
	} catch {
		return null;
	}
}

export function hlcToDateTime(hlcStr: string): string {
	const d = hlcToDate(hlcStr);
	if (!d) return hlcStr;
	return d.toLocaleString('ru', {
		timeZone: TZ,
		day: '2-digit',
		month: '2-digit',
		year: '2-digit',
		hour: '2-digit',
		minute: '2-digit',
	});
}

/** Group items by local date string (YYYY-MM-DD in Tbilisi TZ) */
export function groupByDay<T>(items: T[], getDate: (item: T) => string): Map<string, T[]> {
	const map = new Map<string, T[]>();
	for (const item of items) {
		const raw = getDate(item);
		let dayKey: string;
		try {
			dayKey = new Date(raw).toLocaleDateString('sv', { timeZone: TZ }); // sv gives YYYY-MM-DD
		} catch {
			dayKey = raw.slice(0, 10);
		}
		const arr = map.get(dayKey) ?? [];
		arr.push(item);
		map.set(dayKey, arr);
	}
	return map;
}

/** Format day key "YYYY-MM-DD" → human readable */
export function formatDayLabel(dayKey: string): string {
	try {
		const d = new Date(dayKey + 'T12:00:00'); // noon to avoid DST issues
		const today = new Date().toLocaleDateString('sv', { timeZone: TZ });
		const yesterday = new Date(Date.now() - 86400000).toLocaleDateString('sv', { timeZone: TZ });
		if (dayKey === today) return 'Сегодня';
		if (dayKey === yesterday) return 'Вчера';
		return d.toLocaleDateString('ru', { day: 'numeric', month: 'long', year: 'numeric' });
	} catch {
		return dayKey;
	}
}
