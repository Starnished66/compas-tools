import { t } from './i18n';
import type { Progress } from './api/types';

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${unit === 0 || value >= 100 ? Math.round(value) : value.toFixed(1)} ${UNITS[unit] ?? 'B'}`;
}

export function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleDateString();
}

export function progressLabel({ phase, done, total }: Progress): string {
  const percent = total > 0 ? Math.round((done / total) * 100) : 0;
  switch (phase) {
    case 'downloading':
      return t('Downloading… {percent}%', { percent });
    case 'installing':
      return t('Installing…');
    case 'removing':
      return t('Removing…');
    case 'updating':
      return t('Updating {done} of {total}…', { done: Math.min(done + 1, total), total });
  }
}
