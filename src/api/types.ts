/** Mirrors the serde types in compas-core; field names are the wire names. */

export type Model = 'r1' | 'r3proii' | 'r3ii_2025';

export interface Device {
  schema: number;
  model: Model;
  player_version: string;
  plugin_api: number;
  database_format: number | null;
  art_cache: string | null;
}

export interface CardInfo {
  label: string;
  root: string;
  file_system: string;
  total_bytes: number;
  free_bytes: number;
  device: Device | null;
}

export type PluginState =
  'available' | 'installed' | 'update' | 'manual' | 'incompatible' | 'removed';

export interface PluginEntry {
  id: string;
  name: string;
  version: string;
  installed_version: string | null;
  state: PluginState;
  incompatible: boolean;
  api_min: number | null;
  description: string;
  category: string;
  author: string;
  size: number | null;
  player_layout: boolean;
  theme_pack: boolean;
  has_preview: boolean;
  script: string | null;
  enabled: boolean;
}

export type Outcome = 'done' | 'needs_confirmation';

export interface UpdateAllReport {
  updated: string[];
  needs_confirmation: string[];
  failed: { id: string; message: string }[];
}

export type Phase = 'downloading' | 'installing' | 'removing' | 'updating';

export interface Progress {
  phase: Phase;
  done: number;
  total: number;
}

export type ReleaseKind = 'player_only' | 'full_image';
export type Freshness = 'unknown' | 'up_to_date' | 'update_available' | 'newer';

export interface Release {
  tag: string;
  name: string;
  notes: string;
  published_at: string;
  url: string;
}

export interface LatestRelease {
  release: Release;
  freshness: Freshness;
}

export interface PlacedFile {
  path: string;
  other_upts: string[];
}
