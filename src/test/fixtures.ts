import type { CardInfo, PluginEntry } from '../api/types';

export function entry(overrides: Partial<PluginEntry> = {}): PluginEntry {
  return {
    id: 'example.plugin',
    name: 'Example',
    version: '1.0',
    installed_version: null,
    state: 'available',
    incompatible: false,
    api_min: 10,
    description: 'Does a thing.',
    category: 'Audio',
    author: 'Compás',
    size: 2048,
    player_layout: false,
    theme_pack: false,
    has_preview: false,
    script: 'Example.lua',
    enabled: true,
    ...overrides,
  };
}

export const card: CardInfo = {
  label: 'COMPAS',
  root: '/media/compas',
  file_system: 'exfat',
  total_bytes: 64e9,
  free_bytes: 32e9,
  device: {
    schema: 1,
    model: 'r1',
    player_version: '1.2.0',
    plugin_api: 17,
    database_format: 3,
    art_cache: 'v4',
  },
};
