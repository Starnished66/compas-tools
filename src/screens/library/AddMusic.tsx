import { useState } from 'react';

import { Button } from '../../components/Button';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { ErrorBanner } from '../../components/ErrorBanner';
import { ProgressBar } from '../../components/ProgressBar';
import { useFileDrop } from '../../hooks/useFileDrop';
import { t } from '../../i18n';
import { useMusic } from '../../state/MusicContext';
import { CopyResult } from './CopyResult';
import { PlanPreview } from './PlanPreview';
import { SourcePicker } from './SourcePicker';

/** The Add music panel: sources, destination, the plan and the copy. */
export function AddMusic() {
  const transfer = useMusic();
  const [pickError, setPickError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const hovering = useFileDrop(transfer.add);

  const { plan } = transfer;
  const removing = plan?.deletions.length ?? 0;
  const start = () => {
    setConfirming(false);
    void transfer.copy();
  };

  return (
    <section className="stack" aria-label={t('Add music')}>
      <h2>{t('Add music')}</h2>
      <SourcePicker
        sources={transfer.sources}
        hovering={hovering}
        disabled={transfer.busy}
        onAdd={transfer.add}
        onRemove={transfer.remove}
        onError={setPickError}
      />
      <div className="panel stack">
        <label className="stack">
          <span>{t('Folder on the player (leave empty for the top level)')}</span>
          <input
            className="field"
            value={transfer.destDir}
            placeholder={t('For example: Music')}
            disabled={transfer.busy}
            onChange={(event) => {
              transfer.setDestDir(event.target.value);
            }}
          />
        </label>
        <label className="row">
          <input
            type="checkbox"
            checked={transfer.mirror}
            disabled={transfer.busy}
            onChange={(event) => {
              transfer.setMirror(event.target.checked);
            }}
          />
          <span>
            {t('Mirror a folder: also remove files from the player that are no longer in it')}
          </span>
        </label>
      </div>
      {pickError && (
        <ErrorBanner
          message={pickError}
          onDismiss={() => {
            setPickError(null);
          }}
        />
      )}
      {transfer.planError && <ErrorBanner message={transfer.planError} />}
      {transfer.error && <ErrorBanner message={transfer.error} onDismiss={transfer.clearError} />}
      {plan && <PlanPreview plan={plan} />}
      {transfer.busy && (
        <ProgressBar
          progress={transfer.progress}
          label={t('Copying music and updating the library')}
        />
      )}
      <div>
        <Button
          icon="copy"
          variant="primary"
          disabled={transfer.busy || !plan || plan.files + removing === 0}
          onClick={() => {
            if (removing > 0) setConfirming(true);
            else start();
          }}
        >
          {t('Copy to player')}
        </Button>
      </div>
      {transfer.result && (
        <CopyResult
          result={transfer.result}
          busy={transfer.busy}
          onRetryLibrary={() => void transfer.retryLibrary()}
        />
      )}
      <ConfirmDialog
        open={confirming}
        title={t('Remove files from the player?')}
        message={t('{count} files on the player are not in the folder and will be deleted.', {
          count: removing,
        })}
        confirmLabel={t('Copy and remove')}
        destructive
        onConfirm={start}
        onCancel={() => {
          setConfirming(false);
        }}
      />
    </section>
  );
}
