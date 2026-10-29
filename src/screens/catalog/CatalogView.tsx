import { useEffect, useMemo, useState } from 'react';

import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Button } from '../../components/Button';
import { Notice } from '../../components/Notice';
import { StorageGate } from '../../components/StorageGate';
import { ProgressBar } from '../../components/ProgressBar';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { useCatalog } from '../../state/CatalogContext';
import { useMode } from '../../state/ModeContext';
import { categoriesOf, countUpdates, inView, matches, type CatalogView as View } from './filters';
import { PluginCard } from './PluginCard';
import { PluginDetails } from './PluginDetails';
import { Toolbar } from './Toolbar';
import { UpdateAllReport } from './UpdateAllReport';

interface Props {
  view: View;
}

export function CatalogView({ view }: Props) {
  const { card } = useCard();
  const { cardMissing } = useMode();
  const { entries, loading, error, recordDamaged, load, actions } = useCatalog();
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // The card may have changed behind the app's back since this view was last open.
  useEffect(() => {
    void load(false);
  }, [load]);

  const inThisView = useMemo(() => entries.filter((e) => inView(e, view)), [entries, view]);
  const shown = useMemo(
    () => inThisView.filter((e) => matches(e, query, category)),
    [inThisView, query, category],
  );
  const selected = entries.find((e) => e.id === selectedId) ?? null;
  const title = { plugins: t('Plugins'), layouts: t('Layouts'), themes: t('Themes') }[view];

  if (!card || cardMissing) {
    return (
      <section className="screen">
        <h1>{title}</h1>
        <StorageGate need="plugins">{null}</StorageGate>
      </section>
    );
  }

  return (
    <section className="screen" aria-busy={loading}>
      <div className="screen__header">
        <h1>{title}</h1>
        <span className="muted">{t('{count} plugin', { count: inThisView.length })}</span>
      </div>
      <Toolbar
        query={query}
        category={category}
        categories={categoriesOf(inThisView)}
        updateCount={countUpdates(entries)}
        busy={actions.busy || loading}
        onQuery={setQuery}
        onCategory={setCategory}
        onUpdateAll={() => void actions.updateAll()}
        onRefresh={() => void load(true)}
      />
      {!card.device && (
        <Notice tone="info">
          <p>
            {t(
              'Compatibility cannot be checked yet: no released player writes device.json. Newer player versions record it automatically.',
            )}
          </p>
        </Notice>
      )}
      {error && <ErrorBanner message={error} />}
      {recordDamaged && (
        <div>
          <Button disabled={actions.busy} onClick={() => void actions.moveRecordAside()}>
            {t('Move the plugin record file aside')}
          </Button>
        </div>
      )}
      {actions.busy && !shown.some((e) => e.id === actions.activeId) && (
        <ProgressBar progress={actions.progress} label={t('Working on the player')} />
      )}
      {actions.error && <ErrorBanner message={actions.error} onDismiss={actions.clearError} />}
      {actions.report && (
        <UpdateAllReport
          report={actions.report}
          entries={entries}
          onDismiss={actions.dismissReport}
        />
      )}
      {loading && entries.length === 0 && <p className="muted">{t('Loading the plugin list…')}</p>}
      {!loading && !error && shown.length === 0 && (
        <EmptyState title={t('Nothing to show')}>
          <p className="muted">{t('No plugins match your search.')}</p>
        </EmptyState>
      )}
      <div className={`catalog ${selected ? 'catalog--with-details' : ''}`}>
        <ul className="plugin-grid">
          {shown.map((entry) => (
            <PluginCard
              key={entry.id}
              entry={entry}
              selected={entry.id === selectedId}
              active={actions.activeId === entry.id}
              progress={actions.progress}
              disabled={actions.busy}
              onSelect={() => {
                setSelectedId(entry.id === selectedId ? null : entry.id);
              }}
              onInstall={() => void actions.install(entry.id)}
              onUpdate={() => void actions.update(entry.id)}
            />
          ))}
        </ul>
        {selected && (
          <PluginDetails
            entry={selected}
            disabled={actions.busy}
            pluginApi={card.device?.plugin_api ?? null}
            onClose={() => {
              setSelectedId(null);
            }}
            onInstall={() => void actions.install(selected.id)}
            onUpdate={() => void actions.update(selected.id)}
            onUninstall={() => {
              actions.askUninstall(selected.id);
            }}
            onToggle={(enabled) => void actions.toggle(selected, enabled)}
          />
        )}
      </div>
    </section>
  );
}
