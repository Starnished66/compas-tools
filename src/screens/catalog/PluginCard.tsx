import type { PluginEntry, Progress } from '../../api/types';
import { Badge } from '../../components/Badge';
import { Button } from '../../components/Button';
import { ProgressBar } from '../../components/ProgressBar';
import { t } from '../../i18n';
import { Preview } from './Preview';
import { canInstall, canUpdate, stateLabel, stateTone } from './stateLabel';

interface Props {
  entry: PluginEntry;
  selected: boolean;
  /** Set while this plugin is being changed. */
  active: boolean;
  progress: Progress | null;
  disabled: boolean;
  onSelect: () => void;
  onInstall: () => void;
  onUpdate: () => void;
}

export function PluginCard({
  entry,
  selected,
  active,
  progress,
  disabled,
  onSelect,
  onInstall,
  onUpdate,
}: Props) {
  return (
    <li className={`plugin-card ${selected ? 'plugin-card--selected' : ''}`}>
      <button
        type="button"
        className="plugin-card__open"
        aria-pressed={selected}
        aria-label={t('Details for {name}', { name: entry.name })}
        onClick={onSelect}
      >
        <Preview pluginId={entry.id} available={entry.has_preview} />
        <span className="plugin-card__title">{entry.name}</span>
        <span className="plugin-card__summary">{entry.description}</span>
      </button>
      <div className="plugin-card__footer">
        <div className="row">
          <Badge tone={stateTone(entry)}>{stateLabel(entry)}</Badge>
          <span className="muted">v{entry.installed_version ?? entry.version}</span>
        </div>
        {active && <ProgressBar progress={progress} label={entry.name} />}
        {canInstall(entry) && (
          <Button
            icon="install"
            variant="primary"
            disabled={disabled}
            aria-label={t('Install {name}', { name: entry.name })}
            onClick={onInstall}
          >
            {t('Install')}
          </Button>
        )}
        {canUpdate(entry) && (
          <Button
            icon="update"
            variant="primary"
            disabled={disabled}
            aria-label={t('Update {name} to {version}', {
              name: entry.name,
              version: entry.version,
            })}
            onClick={onUpdate}
          >
            {t('Update')}
          </Button>
        )}
      </div>
    </li>
  );
}
