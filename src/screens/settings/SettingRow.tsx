import { useEffect, useState } from 'react';

import type { SettingDef, SettingValue } from '../../api/settings';
import { Icon } from '../../components/Icon';
import { Switch } from '../../components/Switch';
import { t } from '../../i18n';
import type { Outcome } from './useSettings';
import { applyHint, describe, isLocked, isLoud, lockedReason } from './logic';

interface Props {
  def: SettingDef;
  /** What the player has now. */
  current: SettingValue | undefined;
  /** What it would have once the staged changes were sent. */
  value: SettingValue | undefined;
  staged: boolean;
  /** A change is being sent; nothing may be edited meanwhile. */
  disabled?: boolean;
  outcome: Outcome | undefined;
  /** A setting this one depends on is off. */
  dependencyOff: boolean;
  onChange: (value: SettingValue) => void;
}

/** A whole-number box that only stages a value the player would accept. */
function NumberBox({
  def,
  value,
  id,
  onChange,
}: Pick<Props, 'def' | 'value' | 'onChange'> & { id: string }) {
  const [draft, setDraft] = useState(String(value ?? ''));
  useEffect(() => {
    setDraft(String(value ?? ''));
  }, [value]);
  const parsed = draft.trim() === '' ? NaN : Number(draft);
  const valid =
    Number.isInteger(parsed) &&
    (def.min === null || parsed >= def.min) &&
    (def.max === null || parsed <= def.max);
  return (
    <>
      <input
        id={id}
        className="field settings-number"
        type="number"
        inputMode="numeric"
        min={def.min ?? undefined}
        max={def.max ?? undefined}
        value={draft}
        aria-invalid={!valid}
        onChange={(event) => {
          setDraft(event.target.value);
          const next = Number(event.target.value);
          if (
            event.target.value.trim() !== '' &&
            Number.isInteger(next) &&
            (def.min === null || next >= def.min) &&
            (def.max === null || next <= def.max)
          ) {
            onChange(next);
          }
        }}
      />
      {def.unit && <span className="muted">{def.unit}</span>}
      {!valid && def.min !== null && def.max !== null && (
        <span className="settings-row__warn">
          {t('Enter a whole number from {min} to {max}.', { min: def.min, max: def.max })}
        </span>
      )}
    </>
  );
}

function Control({ def, value, id, locked, onChange }: Props & { id: string; locked: boolean }) {
  if (def.type === 'bool') {
    return (
      <Switch
        checked={value === true}
        label={describe(def, value === true)}
        disabled={locked}
        onChange={onChange}
      />
    );
  }
  if (def.type === 'enum') {
    return (
      <select
        id={id}
        className="field settings-select"
        value={String(value ?? '')}
        disabled={locked}
        onChange={(event) => {
          onChange(Number(event.target.value));
        }}
      >
        {value === undefined && <option value="">—</option>}
        {def.options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    );
  }
  if (def.type === 'preset') {
    return (
      <select
        id={id}
        className="field settings-select"
        value={String(value ?? '')}
        disabled={locked}
        onChange={(event) => {
          onChange(Number(event.target.value));
        }}
      >
        {value === undefined && <option value="">—</option>}
        {def.values.map((preset) => (
          <option key={preset} value={preset}>
            {describe(def, preset)}
          </option>
        ))}
      </select>
    );
  }
  if (def.type === 'percent' && def.min !== null && def.max !== null) {
    return (
      <>
        <input
          id={id}
          className="settings-slider"
          type="range"
          min={def.min}
          max={def.max}
          step={1}
          value={Number(value ?? def.min)}
          disabled={locked}
          onChange={(event) => {
            onChange(Number(event.target.value));
          }}
        />
        <output htmlFor={id} className="settings-slider__value">
          {describe(def, value)}
        </output>
      </>
    );
  }
  if (def.type === 'int' || def.type === 'percent') {
    return locked ? (
      <span>{describe(def, value)}</span>
    ) : (
      <NumberBox def={def} value={value} id={id} onChange={onChange} />
    );
  }
  return <span>{describe(def, value)}</span>;
}

/** One setting: its label on the left, its control and small notes on the right. */
export function SettingRow(props: Props) {
  const { def, current, value, staged, outcome, dependencyOff } = props;
  const locked = isLocked(def) || dependencyOff || props.disabled === true;
  const id = `setting-${def.key}`;
  const labelled =
    def.type === 'enum' || def.type === 'preset' || def.type === 'percent' || def.type === 'int';
  const loud = value !== undefined && isLoud(def, value, current);
  const hint = applyHint(def.apply);
  return (
    <div
      className={`settings-row${staged ? ' settings-row--staged' : ''}${isLocked(def) ? ' settings-row--locked' : ''}`}
      role="group"
      aria-labelledby={`${id}-label`}
    >
      {labelled && !isLocked(def) ? (
        <label id={`${id}-label`} htmlFor={id} className="settings-row__label">
          {def.label}
        </label>
      ) : (
        <span id={`${id}-label`} className="settings-row__label">
          {def.label}
        </span>
      )}
      <div className="settings-row__body">
        <div className="settings-row__control">
          <Control {...props} id={id} locked={locked} />
        </div>
        {isLocked(def) && <p className="settings-row__hint">{lockedReason(def)}</p>}
        {!isLocked(def) && dependencyOff && def.dependsOn && (
          <p className="settings-row__hint">{t('Needs the setting it depends on to be on.')}</p>
        )}
        {loud && def.confirmAbove !== null && (
          <p className="settings-row__warn">
            {t('Above the player’s loud-volume level ({level} %).', { level: def.confirmAbove })}
          </p>
        )}
        {!isLocked(def) && hint && !outcome && <p className="settings-row__hint">{hint}</p>}
        {outcome === 'applied' && (
          <p className="settings-row__done">
            <Icon name="ok" />
            {t('Applied')}
          </p>
        )}
        {outcome && outcome !== 'applied' && (
          <p className="settings-row__done">
            <Icon name="ok" />
            {t('Saved.')} {applyHint(outcome)}
          </p>
        )}
      </div>
    </div>
  );
}
