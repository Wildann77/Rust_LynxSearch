import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { formatDateTime, formatRelativeTime, formatDuration } from '../lib/date';

describe('Date helpers', () => {
  describe('formatDateTime', () => {
    it('handles null, undefined, and empty string gracefully', () => {
      expect(formatDateTime(null)).toBe('-');
      expect(formatDateTime(undefined)).toBe('-');
      expect(formatDateTime('')).toBe('-');
    });

    it('returns "-" for invalid date strings', () => {
      expect(formatDateTime('invalid-date')).toBe('-');
      expect(formatDateTime('2026-99-99T99:99:99Z')).toBe('-');
    });

    it('formats valid ISO date strings using Intl', () => {
      const result = formatDateTime('2026-10-07T12:30:00Z');
      expect(result).not.toBe('-');
      expect(result).toMatch(/2026/);
    });
  });

  describe('formatRelativeTime', () => {
    const fixedNow = new Date('2026-10-07T12:00:00Z').getTime();

    beforeEach(() => {
      vi.useFakeTimers();
      vi.setSystemTime(fixedNow);
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('handles null, undefined, and empty string gracefully', () => {
      expect(formatRelativeTime(null)).toBe('-');
      expect(formatRelativeTime(undefined)).toBe('-');
      expect(formatRelativeTime('')).toBe('-');
    });

    it('returns "-" for invalid date strings', () => {
      expect(formatRelativeTime('not-a-valid-date')).toBe('-');
    });

    it('formats past dates (seconds, minutes, hours, days, weeks, months)', () => {
      // 30 seconds ago
      const thirtySecAgo = new Date(fixedNow - 30 * 1000).toISOString();
      expect(formatRelativeTime(thirtySecAgo)).toMatch(/30 (seconds ago|detik yang lalu)/i);

      // 5 minutes ago
      const fiveMinAgo = new Date(fixedNow - 5 * 60 * 1000).toISOString();
      expect(formatRelativeTime(fiveMinAgo)).toMatch(/5 (minutes ago|menit yang lalu)/i);

      // 3 hours ago
      const threeHoursAgo = new Date(fixedNow - 3 * 3600 * 1000).toISOString();
      expect(formatRelativeTime(threeHoursAgo)).toMatch(/3 (hours ago|jam yang lalu)/i);

      // 2 days ago
      const twoDaysAgo = new Date(fixedNow - 2 * 86400 * 1000).toISOString();
      expect(formatRelativeTime(twoDaysAgo)).toMatch(/2 (days ago|hari yang lalu)/i);

      // 2 weeks ago
      const twoWeeksAgo = new Date(fixedNow - 14 * 86400 * 1000).toISOString();
      expect(formatRelativeTime(twoWeeksAgo)).toMatch(/2 (weeks ago|minggu yang lalu)/i);

      // 2 months ago
      const twoMonthsAgo = new Date(fixedNow - 60 * 86400 * 1000).toISOString();
      expect(formatRelativeTime(twoMonthsAgo)).toMatch(/2 (months ago|bulan yang lalu)/i);
    });

    it('formats future dates (minutes, hours, days)', () => {
      // In 10 minutes
      const inTenMins = new Date(fixedNow + 10 * 60 * 1000).toISOString();
      expect(formatRelativeTime(inTenMins)).toMatch(/(in 10 minutes|dalam 10 menit)/i);

      // In 4 hours
      const inFourHours = new Date(fixedNow + 4 * 3600 * 1000).toISOString();
      expect(formatRelativeTime(inFourHours)).toMatch(/(in 4 hours|dalam 4 jam)/i);

      // In 3 days
      const inThreeDays = new Date(fixedNow + 3 * 86400 * 1000).toISOString();
      expect(formatRelativeTime(inThreeDays)).toMatch(/(in 3 days|dalam 3 hari)/i);
    });

    it('formats current time as now or 0 seconds ago', () => {
      const nowStr = new Date(fixedNow).toISOString();
      const formatted = formatRelativeTime(nowStr);
      expect(formatted).toMatch(/(now|0 seconds ago|sekarang|0 detik yang lalu)/i);
    });
  });

  describe('formatDuration', () => {
    it('handles null, undefined, or invalid timestamps', () => {
      expect(formatDuration(null)).toBe('-');
      expect(formatDuration(undefined)).toBe('-');
      expect(formatDuration('invalid-date')).toBe('-');
      expect(formatDuration('2026-10-07T10:00:00Z', 'invalid-end')).toBe('-');
    });

    it('formats sub-minute duration in seconds', () => {
      const start = '2026-10-07T10:00:00Z';
      const end = '2026-10-07T10:00:25Z';
      expect(formatDuration(start, end)).toBe('25s');
    });

    it('formats multi-minute duration in minutes and seconds', () => {
      const start = '2026-10-07T10:00:00Z';
      const end = '2026-10-07T10:02:15Z';
      expect(formatDuration(start, end)).toBe('2m 15s');
    });
  });
});
