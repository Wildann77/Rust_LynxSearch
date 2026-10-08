/**
 * Localized Date & Relative Time formatting using native Intl API
 * Referensi: DESIGN.md:L432-L455
 */

export function formatDateTime(dateStr: string | null | undefined): string {
  if (!dateStr) return '-';
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return '-';
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(d);
  } catch {
    return '-';
  }
}

export function formatRelativeTime(dateStr: string | null | undefined): string {
  if (!dateStr) return '-';
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return '-';
    const deltaSeconds = Math.round((d.getTime() - Date.now()) / 1000);
    const cutoffs = [60, 3600, 86400, 86400 * 7, 86400 * 30, Infinity];
    const units: Intl.RelativeTimeFormatUnit[] = ['second', 'minute', 'hour', 'day', 'week', 'month'];
    const unitIndex = cutoffs.findIndex((cutoff) => cutoff > Math.abs(deltaSeconds));
    const divisor = unitIndex > 0 ? cutoffs[unitIndex - 1] : 1;
    const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });
    return rtf.format(Math.round(deltaSeconds / divisor), units[unitIndex]);
  } catch {
    return '-';
  }
}

export function formatDuration(
  startStr: string | null | undefined,
  endStr?: string | null | undefined,
): string {
  if (!startStr) return '-';
  try {
    const start = new Date(startStr).getTime();
    if (isNaN(start)) return '-';
    const end = endStr ? new Date(endStr).getTime() : Date.now();
    if (isNaN(end)) return '-';
    const diffSec = Math.max(0, Math.round((end - start) / 1000));
    if (diffSec < 60) return `${diffSec}s`;
    const mins = Math.floor(diffSec / 60);
    const remSec = diffSec % 60;
    return `${mins}m ${remSec}s`;
  } catch {
    return '-';
  }
}
