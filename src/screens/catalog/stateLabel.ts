import type { PluginEntry } from '../../api/types';
import { t } from '../../i18n';

export function stateLabel(entry: PluginEntry): string {
  // A package whose files are on the player through another one, a bundle for instance.
  if (entry.state === 'manual' && entry.files_match && entry.installed_by) {
    return t('Installed with {name}', { name: entry.installed_by });
  }
  switch (entry.state) {
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
  // Content that is already there byte for byte has nothing to install.
  return (
    !entry.incompatible &&
    (entry.state === 'available' || (entry.state === 'manual' && !entry.files_match))
  );
}

export function canUpdate(entry: PluginEntry): boolean {
  return entry.state === 'update';
}

export function canUninstall(entry: PluginEntry): boolean {
  return entry.installed_version !== null;
}
