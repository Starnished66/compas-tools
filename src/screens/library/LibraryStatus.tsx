import { useEffect, useState } from 'react';

import { libraryStatus } from '../../api/library';
import { errorMessage } from '../../api/invoke';
import type { LibraryStatus as Status } from '../../api/types';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';

interface Props {
  /** Change it to read the status again, e.g. after a scan. */
  version?: number;
}

/** What `compas-sdtool info` says about the library on the selected card. */
export function LibraryStatus({ version = 0 }: Props) {
  const { card } = useCard();
  const root = card?.root ?? null;
  const [status, setStatus] = useState<Status | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    if (!root) return;
    let current = true;
    setStatus(null);
    setProblem(null);
    libraryStatus().then(
      (value) => {
        if (current) setStatus(value);
      },
      (failure: unknown) => {
        if (current) setProblem(errorMessage(failure));
      },
    );
    return () => {
      current = false;
    };
  }, [root, version]);

  return (
    <section className="stack" aria-label={t('Library')}>
      <h2>{t('Library')}</h2>
      {status && (
        <p>
          {status.generation === 0
            ? t('No library on this card yet.')
            : t('{songs} songs, library generation {generation}.', {
                songs: status.songs,
                generation: status.generation,
              })}
        </p>
      )}
      {status?.format_mismatch && (
        <Notice tone="warning">
          <p>
            {t(
              'The player on this card writes a different library layout than this app. Update Compás Tools or the player before building the library.',
            )}
          </p>
        </Notice>
      )}
      {problem && (
        <p className="muted">{t('Library information is not available: {problem}', { problem })}</p>
      )}
      {!status && !problem && <p className="muted">{t('Reading the library…')}</p>}
    </section>
  );
}
