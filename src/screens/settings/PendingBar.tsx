import type { SettingsSchema } from '../../api/settings';
import { Button } from '../../components/Button';
import { Icon } from '../../components/Icon';
import { t } from '../../i18n';
import { describe, type Values } from './logic';

interface Props {
  schema: SettingsSchema;
  staged: Values;
  busy: boolean;
  onDiscard: () => void;
  onApply: () => void;
}

/** The changes not yet sent, with Discard and Apply to player. */
export function PendingBar({ schema, staged, busy, onDiscard, onApply }: Props) {
  const keys = Object.keys(staged);
  if (keys.length === 0) return null;
  const summary = keys
    .map((key) => {
      const def = schema.settings.find((s) => s.key === key);
      return def ? `${def.label} ${describe(def, staged[key])}` : key;
    })
    .join(', ');
  return (
    <div className="settings-pending" role="status">
      <Icon name="warning" />
      <p className="settings-pending__text">
        <strong>{t('{count} changes not sent yet', { count: keys.length })}</strong>: {summary}
      </p>
      <Button onClick={onDiscard} disabled={busy}>
        {t('Discard')}
      </Button>
      <Button variant="primary" icon="ok" onClick={onApply} disabled={busy}>
        {busy ? t('Applying…') : t('Apply to player')}
      </Button>
    </div>
  );
}
