import type { PluginEntry } from '../../api/types';
import { Badge } from '../../components/Badge';
import { Button } from '../../components/Button';
import { Notice } from '../../components/Notice';
import { Switch } from '../../components/Switch';
import { formatBytes } from '../../format';
import { t } from '../../i18n';
import { canInstall, canUninstall, canUpdate, stateLabel, stateTone } from './stateLabel';

interface Props {
  entry: PluginEntry;
  disabled: boolean;
  pluginApi: number | null;
  onClose: () => void;
  onInstall: () => void;
  onUpdate: () => void;
  onUninstall: () => void;
  onToggle: (enabled: boolean) => void;
}

export function PluginDetails({
  entry,
  disabled,
  pluginApi,
  onClose,
  onInstall,
  onUpdate,
  onUninstall,
  onToggle,
}: Props) {
  const showSwitch = entry.script !== null && entry.installed_version !== null;
  return (
    <aside
      className="panel details stack"
      aria-label={t('Details for {name}', { name: entry.name })}
    >
      <div className="row">
        <h2>{entry.name}</h2>
        <Badge tone={stateTone(entry)}>{stateLabel(entry.state)}</Badge>
      </div>
      {entry.incompatible && entry.state !== 'installed' && (
        <Notice tone="warning">
          <p>
            {t('This plugin needs plugin API {required}; this player has {actual}.', {
              required: entry.api_min ?? 0,
              actual: pluginApi ?? 0,
            })}
          </p>
        </Notice>
      )}
      {entry.description && <p>{entry.description}</p>}
      <dl className="details-list">
        <dt>{t('Version')}</dt>
        <dd>{entry.version}</dd>
        {entry.installed_version && (
          <>
            <dt>{t('On the player')}</dt>
            <dd>{entry.installed_version}</dd>
          </>
        )}
        {entry.author && (
          <>
            <dt>{t('Author')}</dt>
            <dd>{entry.author}</dd>
          </>
        )}
        {entry.category && (
          <>
            <dt>{t('Category')}</dt>
            <dd>{entry.category}</dd>
          </>
        )}
        {entry.size !== null && (
          <>
            <dt>{t('Size')}</dt>
            <dd>{formatBytes(entry.size)}</dd>
          </>
        )}
        {entry.api_min !== null && (
          <>
            <dt>{t('Plugin API')}</dt>
            <dd>{t('{version} or newer', { version: entry.api_min })}</dd>
          </>
        )}
      </dl>
      {showSwitch && (
        <Switch
          checked={entry.enabled}
          disabled={disabled}
          label={t('Enabled on the player')}
          onChange={onToggle}
        />
      )}
      <div className="row">
        {canInstall(entry) && (
          <Button icon="install" variant="primary" disabled={disabled} onClick={onInstall}>
            {t('Install')}
          </Button>
        )}
        {canUpdate(entry) && (
          <Button icon="update" variant="primary" disabled={disabled} onClick={onUpdate}>
            {t('Update')}
          </Button>
        )}
        {canUninstall(entry) && (
          <Button icon="uninstall" variant="danger" disabled={disabled} onClick={onUninstall}>
            {t('Uninstall')}
          </Button>
        )}
        <Button icon="close" onClick={onClose}>
          {t('Close')}
        </Button>
      </div>
    </aside>
  );
}
