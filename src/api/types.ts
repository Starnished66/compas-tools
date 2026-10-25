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
  | 'cleaning'
  | 'backing_up'
  | 'pushing'
  | 'verifying'
  | 'rebooting'
  | 'waiting'
  | 'collecting';

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
  | 'too_large_for_fat'
  | 'name_too_long'
  | 'path_too_long'
  | 'name_conflict'
  | 'unreadable'
  | 'reserved_name';

export interface PlanView {
  files: number;
  total_bytes: number;
  /** Files whose name already exists on the card. */
  existing: number;
  deletions: string[];
  issues: { source: string; kind: IssueKind }[];
  renames: { from: string; to: string }[];
  skipped_clutter: number;
  /** A mirror removes nothing because some source could not be read. */
  deletions_blocked: boolean;
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
  /** `null` when the covers failed completely; the library itself is fine. */
  art: ArtDone | null;
  art_error: string | null;
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
  /** Albums whose cover could not be written. */
  failed?: number;
  mtime_offset: number;
  /** The tool finished but some covers could not be written. */
  warning?: string | null;
}

export type Firmware = 'stock' | 'compas' | 'unknown';

export interface DeviceInfo {
  serial: string;
  /** From the screen size; the product name over ADB does not identify the board. */
  board: Model | null;
  firmware: Firmware;
  player_version: string | null;
  plugin_api: number | null;
}

export interface DeviceView {
  serial: string;
  /** `device`, `unauthorized`, `offline`, ... */
  state: string;
  product: string | null;
  model: string | null;
  /** An Ingenic serial or the player's USB product. Others are probed only on request. */
  likely_player: boolean;
  info: DeviceInfo | null;
}

export interface ToolsStatus {
  found: boolean;
  source: 'override' | 'system' | 'downloaded' | null;
  version: string | null;
  /** `linux`, `macos` or `windows`. */
  platform: string;
  /** The platform-tools release the download button installs. */
  download_version: string;
}

export interface BackupReport {
  path: string;
  files: number;
  skipped: { path: string; reason: 'private' | 'too_large' | 'binary' | 'unreadable' }[];
  redacted: number;
}

export interface InstallReport {
  remote_path: string;
  device: DeviceInfo | null;
  /** Several players of the same board answered; the user says which is theirs. */
  candidates: DeviceInfo[];
  freshness: Freshness;
}

export interface LogFile {
  name: string;
  source: string;
  content: string;
  bytes: number;
  redactions: number;
  missing: boolean;
}

export interface LogBundle {
  files: LogFile[];
  summary: string;
}
