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

export type Phase =
  | 'downloading'
  | 'installing'
  | 'removing'
  | 'updating'
  | 'scanning'
  | 'reading_tags'
  | 'saving'
  | 'building_art'
  | 'copying'
  | 'cleaning';

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

export type IssueKind =
  'too_large_for_fat' | 'name_too_long' | 'path_too_long' | 'name_conflict' | 'unreadable';

export interface PlanView {
  files: number;
  total_bytes: number;
  /** Files whose name already exists on the card. */
  existing: number;
  deletions: string[];
  issues: { source: string; kind: IssueKind }[];
  renames: { from: string; to: string }[];
  skipped_clutter: number;
  free_bytes: number;
  max_file_bytes: number | null;
}

export interface CopyView {
  copied: number;
  replaced: number;
  skipped_identical: number;
  deleted: number;
  failed: { path: string; message: string }[];
  tidy: { removed: number; marker_written: boolean };
  library: LibraryOutcome;
}

export interface LibraryUpdate {
  scan: ScanDone;
  art: ArtDone;
}

/** What happened to the library after a copy. `failed` leaves the copied files in place. */
export type LibraryOutcome =
  | { status: 'skipped' }
  | ({ status: 'updated' } & LibraryUpdate)
  | { status: 'failed'; message: string };

export interface LibraryStatus {
  sdtool_version: string;
  database_format: number;
  /** 0 when the card has no library yet. */
  generation: number;
  songs: number;
  format_mismatch: boolean;
}

export interface ScanDone {
  files: number;
  parsed: number;
  unchanged: number;
  failed: number;
  skipped: number;
  songs: number;
  generation: number;
  written: boolean;
  mtime_offset: number;
}

export interface ArtDone {
  board: string;
  albums: number;
  written: number;
  unchanged: number;
  no_art: number;
  mtime_offset: number;
}
