import { call } from './invoke';

/** The player's remote-settings JSON, as compas-core passes it on (player field names). */

export interface EnumOption {
  value: number;
  label: string;
  labelId: string;
}

export interface SettingDef {
  key: string;
  group: string;
  label: string;
  labelId: string;
  /** `bool`, `enum`, `int`, `percent`, `preset` or `string`. */
  type: string;
  /** `live`, `next_track`, `next_use` or `next_boot`. */
  apply: string;
  available: boolean;
  readOnly: boolean;
  /** `would_lock_out_remote`, `device_only` or `not_on_this_model`. */
  reason: string | null;
  min: number | null;
  max: number | null;
  unit: string | null;
  confirmAbove: number | null;
  options: EnumOption[];
  values: number[];
  dependsOn: string | null;
}

export interface SettingGroup {
  id: string;
  label: string;
  labelId: string;
}

export interface SettingAction {
  id: string;
  group: string;
  label: string;
  labelId: string;
}

export interface SettingsSchema {
  schemaVersion: number;
  device: { model: string | null; board: string | null; playerVersion: string | null };
  groups: SettingGroup[];
  settings: SettingDef[];
  actions: SettingAction[];
}

export type SettingValue = boolean | number | string;

export interface SettingsSnapshot {
  epoch: number;
  revision: number;
  full: boolean;
  writable: boolean;
  values: Record<string, SettingValue>;
}

export interface SettingsStatus {
  settingsRevision: number | null;
  settingsEpoch: number | null;
}

export interface ApplyRequest {
  epoch?: number;
  baseRevision?: number;
  changes: Record<string, SettingValue>;
  acknowledge?: string[];
}

export interface Applied {
  epoch: number;
  revision: number;
  applied: string[];
  pending: { key: string; reason: string }[];
  alsoChanged: unknown[];
}

/** Why the player said no. A refusal is an answer, not a failure of the connection. */
export interface Refusal {
  status: number;
  code: string;
  /** Per-key errors of `validation_failed`: `{ key, code, min?, max?, confirmAbove? }`. */
  errors: { key: string; code: string; min?: number; max?: number; confirmAbove?: number }[] | null;
  current: Record<string, SettingValue> | null;
}

export type Reply<T> = { result: 'ok'; data: T } | { result: 'refused'; refusal: Refusal };

export type Connection =
  /** Nothing answers behind the forward: Remote Control is off on the player. */
  | { state: 'unreachable' }
  /** The player answers but does not offer remote settings. */
  | { state: 'unsupported' }
  | { state: 'pin_required'; wrong: boolean }
  /** The player's client workers are busy; try again shortly. */
  | { state: 'busy' }
  /** Too many wrong PINs. */
  | { state: 'rate_limited' }
  | {
      state: 'ready';
      schema: SettingsSchema;
      snapshot: SettingsSnapshot;
      status: SettingsStatus;
    };

/** What opening answers: the session to quote in every later call, and what was found. */
export type Opened = Connection & { session: number };

/**
 * Opens the player's settings over `adb forward`. `pin` is what the user typed; leave it out to
 * reuse the one the app already holds for this player. It is kept in memory only.
 */
export const connectSettings = (serial: string, pin?: string) =>
  call<Opened>('settings_connect', { serial, pin: pin ?? null });

export const settingsStatus = (session: number) =>
  call<Reply<SettingsStatus>>('settings_status', { session });

/** Every value, or with `since` only what changed after that revision (same epoch). */
export const settingsValues = (session: number, since?: { revision: number; epoch: number }) =>
  call<Reply<SettingsSnapshot>>('settings_values', {
    session,
    since: since ? [since.revision, since.epoch] : null,
  });

export const applySettings = (session: number, request: ApplyRequest) =>
  call<Reply<Applied>>('settings_apply', { session, request });

export const rescanPlayerLibrary = (session: number) =>
  call<Reply<{ accepted: boolean }>>('settings_rescan', { session });

/** Removes the forward and forgets the PIN, unless a newer session has taken over. */
export const disconnectSettings = (session: number) =>
  call<null>('settings_disconnect', { session });
