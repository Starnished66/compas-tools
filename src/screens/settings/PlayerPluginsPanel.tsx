import { useState } from 'react';

import * as api from '../../api/playerPlugins';
import { Badge } from '../../components/Badge';
import { Button } from '../../components/Button';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import { usePlayerPlugins } from './usePlayerPlugins';
import { PlayerPluginPreview } from './PlayerPluginPreview';

export type PluginSection = 'plugin_store' | 'plugin_management' | 'player_layouts';
type Confirmation = { id: string; name: string; action: 'install' | 'update' | 'uninstall' };

export function PlayerPluginsPanel({
  session,
  features,
  section,
}: {
  session: number;
  features: string[];
  section: PluginSection;
}) {
  const s = usePlayerPlugins(session, features);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('');
  const [previewId, setPreviewId] = useState<string | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const disabled = s.busy || s.changing || s.statusUnknown;
  const run = (action: () => ReturnType<typeof api.refreshPlayerPlugins>) => {
    void s.execute(action);
  };
  const categories = [
    ...new Set(s.store?.plugins.map((item) => item.category).filter(Boolean)),
  ].sort();
  const shown = s.store?.plugins.filter(
    (item) =>
      (!category || item.category === category) &&
      `${item.name} ${item.author} ${item.description}`
        .toLocaleLowerCase()
        .includes(query.toLocaleLowerCase()),
  );

  return (
    <section className="settings-panel" aria-busy={s.busy}>
      <div className="toolbar">
        <Button icon="refresh" disabled={s.busy} onClick={() => void s.reload()}>
          {t('Reload from player')}
        </Button>
        {section === 'plugin_store' && (
          <>
            <Button
              disabled={disabled}
              onClick={() => {
                run(() => api.refreshPluginStore(session));
              }}
            >
              {t('Refresh store catalog')}
            </Button>
            <Button
              disabled={disabled || !s.store?.plugins.some((p) => p.state === 'update')}
              onClick={() => {
                run(() => api.updateAllPlugins(session));
              }}
            >
              {t('Update all')}
            </Button>
          </>
        )}
        {features.includes('plugin_management') && (
          <Button
            disabled={disabled}
            onClick={() => {
              run(() => api.refreshPlayerPlugins(session));
            }}
          >
            {t('Refresh plugins')}
          </Button>
        )}
      </div>
      {s.error && <ErrorBanner message={s.error} />}
      {s.installed?.refresh_pending && (
        <p role="status">{t('Waiting for the player to refresh plugins…')}</p>
      )}
      {s.store?.refresh_required && (
        <p role="status">{t('Refresh plugins to load installed changes.')}</p>
      )}
      {s.store?.error && <ErrorBanner message={s.store.error} />}
      {s.changing && s.store && (
        <div role="status">
          <progress max={100} value={s.store.percent} aria-label={t('Working on the player')} />
          <p>
            {t('Working on the player')}: {s.store.current_plugin} · {s.store.percent}%
          </p>
        </div>
      )}
      {section === 'plugin_store' && (
        <>
          <div className="toolbar">
            <input
              className="field toolbar__search"
              aria-label={t('Search plugins')}
              placeholder={t('Search plugins')}
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
              }}
            />
            <select
              className="field"
              aria-label={t('Category')}
              value={category}
              onChange={(e) => {
                setCategory(e.target.value);
              }}
            >
              <option value="">{t('All categories')}</option>
              {categories.map((name) => (
                <option key={name}>{name}</option>
              ))}
            </select>
          </div>
          <ul className="plugin-grid">
            {shown?.map((item) => (
              <li key={item.id} className="plugin-card">
                <h3>{item.name}</h3>
                <Button
                  disabled={s.busy}
                  onClick={() => {
                    setPreviewId(previewId === item.id ? null : item.id);
                  }}
                >
                  {t('Preview')}
                </Button>
                {previewId === item.id && (
                  <PlayerPluginPreview
                    session={session}
                    id={item.id}
                    kind="store"
                    name={item.name}
                  />
                )}
                <p>
                  {item.version} · {item.author}
                </p>
                <p>{item.description}</p>
                <Badge>{pluginState(item.state)}</Badge>
                {item.incompatible ? (
                  <Badge tone="warning">{t('Not compatible with this player')}</Badge>
                ) : (
                  <div className="toolbar">
                    {item.needs_confirmation ? (
                      <Button
                        disabled={disabled}
                        onClick={() => {
                          setConfirmation({
                            id: item.id,
                            name: item.name,
                            action: item.state === 'update' ? 'update' : 'install',
                          });
                        }}
                      >
                        {t('Review file replacement')}
                      </Button>
                    ) : item.state === 'update' ? (
                      <Button
                        disabled={disabled}
                        onClick={() => {
                          run(() => api.updatePlugin(session, item.id, false));
                        }}
                      >
                        {t('Update')}
                      </Button>
                    ) : item.state === 'available' || item.state === 'manual' ? (
                      <Button
                        disabled={disabled}
                        onClick={() => {
                          run(() => api.installPlugin(session, item.id, false));
                        }}
                      >
                        {t('Install')}
                      </Button>
                    ) : null}
                    {['installed', 'update', 'removed'].includes(item.state) && (
                      <Button
                        disabled={disabled}
                        variant="danger"
                        onClick={() => {
                          setConfirmation({ id: item.id, name: item.name, action: 'uninstall' });
                        }}
                      >
                        {t('Uninstall')}
                      </Button>
                    )}
                  </div>
                )}
              </li>
            ))}
          </ul>
          {!s.busy && !shown?.length && (
            <p className="muted">
              {t('No plugins found. Refresh the store catalog to browse available plugins.')}
            </p>
          )}
        </>
      )}
      {section === 'plugin_management' && (
        <>
          {s.installed?.truncated && (
            <p>{t('The player reached its plugin scan limit. Some files are not listed.')}</p>
          )}
          <ul className="plugin-grid">
            {s.installed?.plugins.map((item) => (
              <li key={item.id} className="plugin-card">
                <h3>{item.name || item.id}</h3>
                <p className="muted">{item.id}</p>
                <Badge>
                  {item.loaded ? t('Loaded') : item.enabled ? t('Not loaded') : t('Disabled')}
                </Badge>
                {item.error && (
                  <p>
                    {item.error === 'load_limit_reached'
                      ? t('Plugin load limit reached.')
                      : t('This plugin could not be loaded.')}
                  </p>
                )}
                {item.can_open_settings && (
                  <Button
                    disabled={disabled}
                    onClick={() => {
                      run(() => api.openPluginSettings(session, item.id));
                    }}
                  >
                    {t('Open on player')}
                  </Button>
                )}
                <label className="row">
                  <input
                    type="checkbox"
                    checked={item.enabled}
                    disabled={disabled}
                    onChange={(e) => {
                      run(() => api.setPluginEnabled(session, item.id, e.target.checked));
                    }}
                  />
                  {t('Enabled')}
                </label>
              </li>
            ))}
          </ul>
          {!s.busy && !s.installed?.plugins.length && (
            <p className="muted">{t('No installed plugins.')}</p>
          )}
        </>
      )}
      {section === 'player_layouts' && (
        <ul className="plugin-grid">
          {s.layouts?.layouts.map((item) => (
            <li key={item.id} className="plugin-card">
              <PlayerPluginPreview
                session={session}
                id={item.id}
                kind="layout"
                name={item.name}
                available={
                  item.has_preview &&
                  ((s.layouts?.layouts.indexOf(item) ?? 100) < 12 || previewId === item.id)
                }
              />
              {item.has_preview && (s.layouts?.layouts.indexOf(item) ?? 0) >= 12 && (
                <Button
                  onClick={() => {
                    setPreviewId(item.id);
                  }}
                >
                  {t('Preview')}
                </Button>
              )}
              <h3>{item.name}</h3>
              <p>{item.source === 'builtin_c' ? t('Built in') : t('Installed layout')}</p>
              {s.layouts?.current === item.id ? (
                <Badge tone="success">{t('Current layout')}</Badge>
              ) : (
                <Button
                  disabled={disabled}
                  onClick={() => {
                    run(() => api.selectPlayerLayout(session, item.id));
                  }}
                >
                  {t('Use layout')}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={confirmation !== null}
        destructive={confirmation?.action === 'uninstall'}
        title={
          confirmation?.action === 'uninstall'
            ? t('Uninstall plugin?')
            : t('Replace local plugin files?')
        }
        message={
          confirmation?.action === 'uninstall'
            ? t('Remove {name} from the player?', { name: confirmation.name })
            : t('Local changes may be replaced. Continue only if you want to replace these files.')
        }
        confirmLabel={confirmation?.action === 'uninstall' ? t('Uninstall') : t('Replace files')}
        details={confirmation ? [confirmation.name] : []}
        onCancel={() => {
          setConfirmation(null);
        }}
        onConfirm={() => {
          const pending = confirmation;
          setConfirmation(null);
          if (!pending || disabled) return;
          if (pending.action === 'uninstall') run(() => api.uninstallPlugin(session, pending.id));
          else if (pending.action === 'update')
            run(() => api.updatePlugin(session, pending.id, true));
          else run(() => api.installPlugin(session, pending.id, true));
        }}
      />
    </section>
  );
}

function pluginState(state: string): string {
  switch (state) {
    case 'available':
      return t('Available');
    case 'installed':
      return t('Installed');
    case 'update':
      return t('Update available');
    case 'manual':
      return t('Local files');
    case 'incompatible':
      return t('Incompatible');
    case 'removed':
      return t('No longer in catalog');
    default:
      return t('Unknown');
  }
}
