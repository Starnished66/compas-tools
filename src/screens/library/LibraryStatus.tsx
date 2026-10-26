import type { LibraryStatus as Status } from '../../api/types';
import { t } from '../../i18n';
import { LibraryNotReady } from './LibraryNotReady';

interface Props {
  status: Status | null;
  problem: string | null;
  /** Show the "Library" heading; the Library screen has its own. */
  heading?: boolean;
}

/** What `compas-sdtool info` says about the library on the selected card. */
export function LibraryStatus({ status, problem, heading = true }: Props) {
  return (
    <section className="stack" aria-label={t('Library')}>
      {heading && <h2>{t('Library')}</h2>}
      {status && (
        <p>
          {status.generation === 0
            ? t('No library on this card yet.')
            : t('{count} songs, library generation {generation}.', {
                count: status.songs,
                generation: status.generation,
              })}
        </p>
      )}
      {status && status.support !== 'ready' && <LibraryNotReady support={status.support} />}
      {problem && (
        <p className="muted">{t('Library information is not available: {problem}', { problem })}</p>
      )}
      {!status && !problem && <p className="muted">{t('Reading the library…')}</p>}
    </section>
  );
}
