import type { PluginEntry, PluginState } from '../../api/types';
import { t } from '../../i18n';

export function stateLabel(state: PluginState): string {
  switch (state) {
    case 'available':
      return t('Not installed');
    case 'installed':
      return t('Installed');
    case 'update':
      return t('Update available');
    case 'manual':
      return t('Installed manually');
    case 'incompatible':
      return t('Needs a newer player');
    case 'removed':
      return t('No longer in the catalog');
  }
}

export function stateTone(entry: PluginEntry): 'neutral' | 'accent' | 'success' | 'warning' {
  if (entry.incompatible && entry.state !== 'installed') return 'warning';
  switch (entry.state) {
    case 'update':
      return 'accent';
    case 'installed':
      return 'success';
    case 'manual':
    case 'removed':
    case 'incompatible':
      return 'warning';
    case 'available':
      return 'neutral';
  }
}

/** The store entry can be installed or updated from the catalog. */
export function canInstall(entry: PluginEntry): boolean {
  return !entry.incompatible && (entry.state === 'available' || entry.state === 'manual');
}

export function canUpdate(entry: PluginEntry): boolean {
  return entry.state === 'update';
}

export function canUninstall(entry: PluginEntry): boolean {
  return entry.installed_version !== null;
}
