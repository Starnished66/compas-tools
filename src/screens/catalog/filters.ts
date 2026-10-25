import type { PluginEntry } from '../../api/types';

export type CatalogView = 'plugins' | 'layouts' | 'themes';

/** Layouts and themes are filtered views of the same catalog. */
export function inView(entry: PluginEntry, view: CatalogView): boolean {
  switch (view) {
    case 'layouts':
      return entry.player_layout;
    case 'themes':
      return entry.theme_pack;
    case 'plugins':
      return !entry.player_layout && !entry.theme_pack;
  }
}

export function categoriesOf(entries: PluginEntry[]): string[] {
  return [...new Set(entries.map((e) => e.category).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b),
  );
}

export function matches(entry: PluginEntry, query: string, category: string): boolean {
  if (category && entry.category !== category) return false;
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return [entry.name, entry.id, entry.description, entry.author, entry.category].some((field) =>
    field.toLowerCase().includes(needle),
  );
}

export function countUpdates(entries: PluginEntry[]): number {
  return entries.filter((e) => e.state === 'update').length;
}
