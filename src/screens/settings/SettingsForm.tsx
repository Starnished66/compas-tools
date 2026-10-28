import { useState } from 'react';

import type { SettingsSchema } from '../../api/settings';
import { Button } from '../../components/Button';
import { Icon } from '../../components/Icon';
import type { IconName } from '../../components/icons';
import { t } from '../../i18n';
import { SettingRow } from './SettingRow';
import { dependencyUnmet, groupsOf, type Values } from './logic';
import type { Outcome } from './useSettings';

/** The icons of the groups the player lists today; any other group gets the generic one. */
const GROUP_ICONS: Record<string, IconName> = {
  sound: 'groupSound',
  display: 'groupDisplay',
  power: 'groupPower',
  playback: 'groupPlayback',
  connections: 'groupConnections',
  library: 'library',
  buttons: 'groupButtons',
};

interface Props {
  schema: SettingsSchema;
  values: Values;
  effective: Values;
  staged: Values;
  outcomes: Record<string, Outcome>;
  rescanStarted: boolean;
  /** Edits are held while a change is being sent. */
  disabled: boolean;
  onChange: (key: string, value: boolean | number | string) => void;
  onRescan: () => void;
}

/** The groups on the left and the selected group's settings on the right. */
export function SettingsForm({
  schema,
  values,
  effective,
  staged,
  outcomes,
  rescanStarted,
  disabled,
  onChange,
  onRescan,
}: Props) {
  const groups = groupsOf(schema);
  const [selected, setSelected] = useState(groups[0]?.group.id ?? '');
  const current = groups.find((g) => g.group.id === selected) ?? groups[0];
  if (!current) return <p className="muted">{t('The player lists no settings.')}</p>;
  return (
    <div className="settings-layout">
      <nav className="settings-groups" aria-label={t('Setting groups')}>
        {groups.map(({ group }) => (
          <button
            key={group.id}
            type="button"
            className="nav-item"
            aria-current={group.id === current.group.id ? 'page' : undefined}
            onClick={() => {
              setSelected(group.id);
            }}
          >
            <Icon name={GROUP_ICONS[group.id] ?? 'settings'} />
            {group.label}
          </button>
        ))}
      </nav>
      <section className="settings-panel" aria-label={current.group.label}>
        <h2 className="settings-panel__title">{current.group.label}</h2>
        <div className="settings-panel__rows">
          {current.settings.map((def) => (
            <SettingRow
              key={def.key}
              def={def}
              current={values[def.key]}
              value={effective[def.key]}
              disabled={disabled}
              staged={def.key in staged}
              outcome={outcomes[def.key]}
              dependencyOff={dependencyUnmet(def, effective)}
              onChange={(value) => {
                onChange(def.key, value);
              }}
            />
          ))}
          {current.actions.map((action) =>
            action.id === 'library.rescan' ? (
              <div className="settings-row" key={action.id}>
                <span className="settings-row__label">{action.label}</span>
                <div className="settings-row__body">
                  <div className="settings-row__control">
                    <Button icon="refresh" onClick={onRescan} disabled={disabled}>
                      {t('Scan now')}
                    </Button>
                  </div>
                  <p className="settings-row__hint">
                    {rescanStarted
                      ? t('The player started updating its music database.')
                      : t('Scans the player’s music folders for changes.')}
                  </p>
                </div>
              </div>
            ) : null,
          )}
        </div>
      </section>
    </div>
  );
}
