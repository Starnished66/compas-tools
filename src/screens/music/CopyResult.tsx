import type { CopyView } from '../../api/types';
import { Button } from '../../components/Button';
import { EjectAdvice } from '../../components/EjectAdvice';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';
import { ArtResult, ScanResult } from '../library/LibraryResult';

interface Props {
  result: CopyView;
  busy: boolean;
  /** Reruns only the library update. */
  onRetryLibrary: () => void;
}

export function CopyResult({ result, busy, onRetryLibrary }: Props) {
  const { library } = result;
  return (
    <section className="stack" aria-label={t('Result')}>
      <Notice tone={result.failed.length > 0 ? 'warning' : 'success'}>
        <p>
          {t('Copied {copied} files ({replaced} replaced), skipped {skipped} identical.', {
            copied: result.copied,
            replaced: result.replaced,
            skipped: result.skipped_identical,
          })}
        </p>
        {result.deleted > 0 && (
          <p>
            {t('Removed {count} files that were not in the folder.', { count: result.deleted })}
          </p>
        )}
        {result.tidy.removed > 0 && (
          <p>{t('Cleaned up {count} macOS leftovers.', { count: result.tidy.removed })}</p>
        )}
        {result.failed.length > 0 && (
          <>
            <p>{t('{count} files failed:', { count: result.failed.length })}</p>
            <ul>
              {result.failed.map((f) => (
                <li key={f.path}>
                  {f.path}: {f.message}
                </li>
              ))}
            </ul>
          </>
        )}
      </Notice>
      {library.status === 'skipped' && (
        <p className="muted">{t('Nothing changed on the card, so the library was not updated.')}</p>
      )}
      {library.status === 'updated' && (
        <>
          <ScanResult done={library.scan} />
          <ArtResult done={library.art} />
        </>
      )}
      {library.status === 'failed' && (
        <>
          <ErrorBanner message={library.message} />
          <p>{t('The copied files are on the card. Only the library still needs updating.')}</p>
          <div>
            <Button variant="primary" disabled={busy} onClick={onRetryLibrary}>
              {t('Try again')}
            </Button>
          </div>
        </>
      )}
      {library.status !== 'failed' && <EjectAdvice />}
    </section>
  );
}
