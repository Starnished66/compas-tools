import { useState } from 'react';

import { buildArt, scanLibrary } from '../../api/library';
import type { ArtDone, ScanDone } from '../../api/types';
import { Button } from '../../components/Button';
import { EjectAdvice } from '../../components/EjectAdvice';
import { ErrorBanner } from '../../components/ErrorBanner';
import { FindPlayer } from '../../components/FindPlayer';
import { ModelPicker } from '../../components/ModelPicker';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { formatBytes } from '../../format';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { BOARD_NAMES } from '../install/labels';
import { ArtResult, ScanResult } from './LibraryResult';
import { LibraryStatus } from './LibraryStatus';
import { useLibraryStatus } from './useLibraryStatus';

type Result = { type: 'scan'; done: ScanDone } | { type: 'art'; done: ArtDone } | null;

/** The selected player: what it is, its space and library, and the library actions. */
export function PlayerPanel() {
  const { card, model, chooseModel } = useCard();
  const operation = useOperation();
  const [result, setResult] = useState<Result>(null);
  const [version, setVersion] = useState(0);
  const { status, problem } = useLibraryStatus(version);
  if (!card) return null;

  const { device } = card;
  const ready = status?.support === 'ready';
  const run = async (task: Parameters<typeof operation.run<Result>>[0]) => {
    setResult(null);
    const outcome = await operation.run(task);
    if (outcome) setResult(outcome);
    setVersion((v) => v + 1);
  };

  return (
    <section className="panel stack" aria-label={t('Player details')}>
      <h2>{card.label}</h2>
      <dl className="details-list">
        <dt>{t('Model')}</dt>
        <dd>{model ? BOARD_NAMES[model] : t('Unknown model')}</dd>
        {device && (
          <>
            <dt>{t('Player version')}</dt>
            <dd>{device.player_version}</dd>
            <dt>{t('Plugin API')}</dt>
            <dd>{device.plugin_api}</dd>
          </>
        )}
        <dt>{t('Location')}</dt>
        <dd>{card.root}</dd>
        <dt>{t('File system')}</dt>
        <dd>{card.file_system || t('Unknown')}</dd>
        <dt>{t('Space')}</dt>
        <dd>
          {t('{free} free of {total}', {
            free: formatBytes(card.free_bytes),
            total: formatBytes(card.total_bytes),
          })}
        </dd>
      </dl>
      {!device && (
        <Notice tone="info">
          <p>
            {t(
              'Your player has no device information yet. Update the player once, or tell Compás Tools which player it is.',
            )}
          </p>
          <ModelPicker value={model} onChange={chooseModel} />
        </Notice>
      )}
      <LibraryStatus status={status} problem={problem} heading={false} />
      <div className="row">
        <Button
          icon="refresh"
          variant="primary"
          disabled={operation.busy || !ready}
          onClick={() =>
            void run(async (p) => ({ type: 'scan', done: await scanLibrary(false, p) }))
          }
        >
          {t('Update library')}
        </Button>
        <Button
          icon="tags"
          disabled={operation.busy || !ready}
          onClick={() =>
            void run(async (p) => ({ type: 'scan', done: await scanLibrary(true, p) }))
          }
        >
          {t('Re-read all tags')}
        </Button>
        <Button
          icon="albumArt"
          disabled={operation.busy || !ready}
          onClick={() => void run(async (p) => ({ type: 'art', done: await buildArt(p) }))}
        >
          {t('Build album art')}
        </Button>
      </div>
      {operation.busy && <ProgressBar progress={operation.progress} label={t('Library')} />}
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}
      {result?.type === 'scan' && <ScanResult done={result.done} />}
      {result?.type === 'art' && <ArtResult done={result.done} />}
      <EjectAdvice />
      <details>
        <summary>{t('Choose another player')}</summary>
        <FindPlayer />
      </details>
    </section>
  );
}
