import { useMemo, useState } from 'react';

import { ConfirmDialog } from '../../components/ConfirmDialog';
import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { useCatalog } from '../../state/CatalogContext';
import { categoriesOf, countUpdates, inView, matches, type CatalogView as View } from './filters';
import { PluginCard } from './PluginCard';
import { PluginDetails } from './PluginDetails';
import { Toolbar } from './Toolbar';
import { UpdateAllReport } from './UpdateAllReport';
import { usePluginActions } from './usePluginActions';

interface Props {
  view: View;
  onChooseCard: () => void;
}

export function CatalogView({ view, onChooseCard }: Props) {
  const { card } = useCard();
  const { entries, loading, error, load } = useCatalog();
  const actions = usePluginActions(() => load(false));
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const inThisView = useMemo(() => entries.filter((e) => inView(e, view)), [entries, view]);
  const shown = useMemo(
    () => inThisView.filter((e) => matches(e, query, category)),
    [inThisView, query, category],
  );
  const selected = entries.find((e) => e.id === selectedId) ?? null;
  const dialogEntry = actions.dialog && entries.find((e) => e.id === actions.dialog?.id);
  const title = { plugins: t('Plugins'), layouts: t('Layouts'), themes: t('Themes') }[view];

  if (!card) {
    return (
      <section className="screen">
        <h1>{title}</h1>
        <EmptyState title={t('Choose your card first')}>
          <p className="muted">{t('Plugins are installed onto the SD card of your player.')}</p>
          <button type="button" className="button button--primary" onClick={onChooseCard}>
            {t('Go to Card')}
          </button>
        </EmptyState>
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
      {error && <ErrorBanner message={error} />}
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
      <ConfirmDialog
        open={actions.dialog !== null}
        title={
          actions.dialog?.type === 'remove' ? t('Remove this plugin?') : t('Replace a local file?')
        }
        message={
          actions.dialog?.type === 'remove'
            ? t('{name} and its files will be removed from the card.', {
                name: dialogEntry?.name ?? '',
              })
            : t('{name} would replace a file on the card that the store did not install.', {
                name: dialogEntry?.name ?? '',
              })
        }
        confirmLabel={actions.dialog?.type === 'remove' ? t('Remove') : t('Replace')}
        destructive
        onConfirm={() => void actions.confirm()}
        onCancel={actions.cancelDialog}
      />
    </section>
  );
}
