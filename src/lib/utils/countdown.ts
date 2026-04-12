/**
 * Formats time remaining until a deadline as a human-readable string.
 * Updates live when the returned value is re-evaluated.
 */
export function formatCountdown(deadline: string): string {
	const diff = new Date(deadline).getTime() - Date.now();
	if (diff <= 0) return 'просрочен';

	const days    = Math.floor(diff / 86_400_000);
	const hours   = Math.floor((diff % 86_400_000) / 3_600_000);
	const minutes = Math.floor((diff % 3_600_000) / 60_000);

	const parts: string[] = [];
	if (days > 0)    parts.push(`${days} дн.`);
	if (hours > 0)   parts.push(`${hours} ч.`);
	parts.push(`${minutes} мин.`);

	return 'через ' + parts.join(' ');
}

export function formatDeadlineShort(deadline: string): string {
	const diff = new Date(deadline).getTime() - Date.now();
	const diffDays = Math.round(diff / 86_400_000);
	if (diffDays === 0) return 'сегодня';
	if (diffDays > 0) return `через ${diffDays} дн.`;
	return `просрочен ${Math.abs(diffDays)} дн.`;
}
