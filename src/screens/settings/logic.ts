import type {
  Refusal,
  SettingAction,
  SettingDef,
  SettingGroup,
  SettingsSchema,
  SettingValue,
} from '../../api/settings';
import { t } from '../../i18n';

export type Values = Record<string, SettingValue>;

export interface GroupView {
  group: SettingGroup;
  settings: SettingDef[];
  actions: SettingAction[];
}

/** The groups the player lists, in its order, each with its settings and actions. */
export function groupsOf(schema: SettingsSchema): GroupView[] {
  const views = new Map<string, GroupView>();
  const view = (id: string): GroupView => {
    let found = views.get(id);
    if (!found) {
      // A group the player used but did not list: show it under its id.
      found = { group: { id, label: id, labelId: id }, settings: [], actions: [] };
      views.set(id, found);
    }
    return found;
  };
  for (const group of schema.groups) views.set(group.id, { group, settings: [], actions: [] });
  for (const setting of schema.settings) view(setting.group).settings.push(setting);
  for (const action of schema.actions) view(action.group).actions.push(action);
  return [...views.values()].filter((v) => v.settings.length > 0 || v.actions.length > 0);
}

/** Whether the player lets this one be changed from here at all. */
export const isLocked = (def: SettingDef): boolean => def.readOnly || !def.available;

/** A setting that depends on another is off while that one is off. */
export function dependencyUnmet(def: SettingDef, effective: Values): boolean {
  return def.dependsOn !== null && effective[def.dependsOn] === false;
}

/** The values as they would be once the staged changes were applied. */
export const merged = (values: Values, staged: Values): Values => ({ ...values, ...staged });

/**
 * Stages a change, or unstages it when it puts the value back. Switching off a setting another
 * one depends on also takes that one's staged "on" away, as the player turns it off itself.
 */
export function stage(
  schema: SettingsSchema,
  values: Values,
  staged: Values,
  key: string,
  value: SettingValue,
): Values {
  const withKey = { ...staged, [key]: value };
  const effective = merged(values, withKey);
  const dropped = new Set(
    schema.settings
      .filter((def) => def.dependsOn === key && dependencyUnmet(def, effective))
      .map((def) => def.key),
  );
  if (values[key] === value) dropped.add(key);
  return Object.fromEntries(Object.entries(withKey).filter(([k]) => !dropped.has(k)));
}

/** Whether raising a setting to `value` passes the level the player asks to confirm. */
export function isLoud(def: SettingDef, value: SettingValue, current: SettingValue | undefined) {
  return (
    def.confirmAbove !== null &&
    typeof value === 'number' &&
    value > def.confirmAbove &&
    value > Number(current ?? 0)
  );
}

/** The staged settings that need the user's confirmation before they are sent. */
export function loudChanges(schema: SettingsSchema, values: Values, staged: Values) {
  return schema.settings.filter((def) => {
    const value = staged[def.key];
    return value !== undefined && isLoud(def, value, values[def.key]);
  });
}

/** A value with its unit, as a person would say it. */
export function describe(def: SettingDef, value: SettingValue | undefined): string {
  if (value === undefined) return '—';
  if (typeof value === 'boolean') return value ? t('On') : t('Off');
  if (def.type === 'enum') {
    return def.options.find((o) => o.value === value)?.label ?? String(value);
  }
  const unit = def.unit ?? '';
  return unit === '%' ? `${value} %` : unit ? `${value} ${unit}` : String(value);
}

/** The line under a row for when a change takes effect; nothing for a live one. */
export function applyHint(apply: string): string | null {
  switch (apply) {
    case 'next_track':
      return t('Applies from the next track.');
    case 'next_use':
      return t('Applies the next time it is used.');
    case 'next_boot':
      return t('Applies after the player restarts.');
    default:
      return null;
  }
}

/** Why a setting is locked, in words. */
export function lockedReason(def: SettingDef): string {
  if (!def.available) return t('Not available on this player model.');
  if (def.reason === 'would_lock_out_remote') {
    return t('Changing this could cut the connection, so it can only be changed on the player.');
  }
  return t('This can only be changed on the player.');
}

/** What the player's refusal means for the user, without showing its codes. */
export function refusalMessage(refusal: Refusal, schema: SettingsSchema): string {
  const label = (key: string) => schema.settings.find((s) => s.key === key)?.label ?? key;
  switch (refusal.code) {
    case 'revision_mismatch':
      return t(
        'Settings changed on the player in the meantime. They were reloaded; check them and apply again.',
      );
    case 'player_busy':
      return t(
        'The player is busy. Wait a moment and try again; the list was reloaded in case the change went through.',
      );
    case 'settings_busy':
      return t('The player is applying another change. Try again in a moment.');
    case 'rescan_running':
      return t('The player is already updating its music database.');
    case 'remote_changes_disabled':
      return t('The player does not allow changes from this connection.');
    case 'validation_failed': {
      const first = refusal.errors?.[0];
      if (!first) return t('The player did not accept the change.');
      if (first.code === 'out_of_range' && first.min !== undefined && first.max !== undefined) {
        return t('{name}: the value must be between {min} and {max}.', {
          name: label(first.key),
          min: first.min,
          max: first.max,
        });
      }
      if (first.code === 'dependency_unmet') {
        return t('{name} needs another setting to be on first.', { name: label(first.key) });
      }
      if (first.code === 'read_only' || first.code === 'unavailable') {
        return t('{name} cannot be changed on this player.', { name: label(first.key) });
      }
      return t('The player did not accept {name}.', { name: label(first.key) });
    }
    default:
      return t('The player refused the request ({status}).', { status: refusal.status });
  }
}
