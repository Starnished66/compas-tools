import { useEffect, useState, type ReactNode } from 'react';

import { pluginsCapabilities } from '../../api/playerPlugins';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import { PlayerPluginsPanel, type PluginSection } from './PlayerPluginsPanel';

/** Reuse the authenticated settings connection; API capabilities are not user toggles. */
export function PlayerManagement({
  session,
  children,
}: {
  session: number | null;
  children: ReactNode;
}) {
  const [features, setFeatures] = useState<string[]>([]);
  const [section, setSection] = useState<'settings' | PluginSection>('settings');
  const [error, setError] = useState(false);
  useEffect(() => {
    let active = true;
    setFeatures([]);
    setSection('settings');
    setError(false);
    if (session !== null)
      void pluginsCapabilities(session)
        .then((caps) => {
          if (active) setFeatures(caps.features);
        })
        .catch(() => {
          if (active) setError(true);
        });
    return () => {
      active = false;
    };
  }, [session]);
  const options: { id: PluginSection; label: string }[] = [
    { id: 'plugin_store', label: t('Plugin Store') },
    { id: 'plugin_management', label: t('Installed plugins') },
    { id: 'player_layouts', label: t('Player layouts') },
  ];
  return (
    <>
      {error && (
        <ErrorBanner message={t('Could not read player capabilities. Reconnect to try again.')} />
      )}
      {features.some((feature) => options.some((option) => option.id === feature)) && (
        <nav className="toolbar" aria-label={t('Player controls')}>
          <Button
            variant={section === 'settings' ? 'primary' : 'secondary'}
            onClick={() => {
              setSection('settings');
            }}
          >
            {t('Settings')}
          </Button>
          {options
            .filter((option) => features.includes(option.id))
            .map((option) => (
              <Button
                key={option.id}
                variant={section === option.id ? 'primary' : 'secondary'}
                onClick={() => {
                  setSection(option.id);
                }}
              >
                {option.label}
              </Button>
            ))}
        </nav>
      )}
      {section === 'settings'
        ? children
        : session !== null && (
            <PlayerPluginsPanel
              key={session}
              session={session}
              features={features}
              section={section}
            />
          )}
    </>
  );
}
