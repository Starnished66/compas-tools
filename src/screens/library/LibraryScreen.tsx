import { useState } from 'react';

import { buildArt, scanLibrary } from '../../api/library';
import type { ArtDone, ScanDone } from '../../api/types';
import { Button } from '../../components/Button';
import { EjectAdvice } from '../../components/EjectAdvice';
import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { ProgressBar } from '../../components/ProgressBar';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { ArtResult, ScanResult } from './LibraryResult';
import { LibraryStatus } from './LibraryStatus';

interface Props {
  onChooseCard: () => void;
}

type Result = { type: 'scan'; done: ScanDone } | { type: 'art'; done: ArtDone } | null;

export function LibraryScreen({ onChooseCard }: Props) {
  const { card } = useCard();
  const operation = useOperation();
  const [result, setResult] = useState<Result>(null);
  const [version, setVersion] = useState(0);

  if (!card) {
    return (
      <section className="screen">
        <h1>{t('Library')}</h1>
        <EmptyState title={t('Choose your card first')}>
          <p className="muted">{t('The library is built on the SD card of your player.')}</p>
          <Button variant="primary" onClick={onChooseCard}>
            {t('Go to Card')}
          </Button>
        </EmptyState>
      </section>
    );
  }

  const run = async (task: Parameters<typeof operation.run<Result>>[0]) => {
    setResult(null);
    const outcome = await operation.run(task);
    if (outcome) setResult(outcome);
    setVersion((v) => v + 1);
  };

  return (
    <section className="screen">
      <h1>{t('Library')}</h1>
      <p className="muted">
        {t(
          'Builds the music database and album art on this computer, so the player does not have to scan on its next start. This is the same code the player uses.',
        )}
      </p>
      <div className="panel stack">
        <LibraryStatus version={version} heading={false} />
        <div className="row">
          <Button
            icon="refresh"
            variant="primary"
            disabled={operation.busy}
            onClick={() =>
              void run(async (p) => ({ type: 'scan', done: await scanLibrary(false, p) }))
            }
          >
            {t('Update library')}
          </Button>
          <Button
            icon="tags"
            disabled={operation.busy}
            onClick={() =>
              void run(async (p) => ({ type: 'scan', done: await scanLibrary(true, p) }))
            }
          >
            {t('Re-read all tags')}
          </Button>
          <Button
            icon="albumArt"
            disabled={operation.busy}
            onClick={() => void run(async (p) => ({ type: 'art', done: await buildArt(p) }))}
          >
            {t('Build album art')}
          </Button>
        </div>
        {operation.busy && <ProgressBar progress={operation.progress} label={t('Library')} />}
      </div>
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}
      {result?.type === 'scan' && <ScanResult done={result.done} />}
      {result?.type === 'art' && <ArtResult done={result.done} />}
      {result && <EjectAdvice />}
    </section>
  );
}
