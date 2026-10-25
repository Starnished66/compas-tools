import type { CopyView } from '../../api/types';
import { EjectAdvice } from '../../components/EjectAdvice';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';

export function CopyResult({ result }: { result: CopyView }) {
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
      <p className="muted">
        {t('Next: build the library on the Library screen so the player starts fast.')}
      </p>
      <EjectAdvice />
    </section>
  );
}
