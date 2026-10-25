import type { PluginEntry, UpdateAllReport as Report } from '../../api/types';
import { Button } from '../../components/Button';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';

interface Props {
  report: Report;
  entries: PluginEntry[];
  onDismiss: () => void;
}

export function UpdateAllReport({ report, entries, onDismiss }: Props) {
  const name = (id: string) => entries.find((e) => e.id === id)?.name ?? id;
  const tone =
    report.failed.length > 0 || report.needs_confirmation.length > 0 ? 'warning' : 'success';
  return (
    <Notice tone={tone}>
      <p>{t('{count} update installed', { count: report.updated.length })}</p>
      {report.needs_confirmation.length > 0 && (
        <p>
          {t('These would replace local files; update them one by one: {names}', {
            names: report.needs_confirmation.map(name).join(', '),
          })}
        </p>
      )}
      {report.failed.map((failure) => (
        <p key={failure.id}>
          {name(failure.id)}: {failure.message}
        </p>
      ))}
      <div>
        <Button onClick={onDismiss}>{t('Dismiss')}</Button>
      </div>
    </Notice>
  );
}
