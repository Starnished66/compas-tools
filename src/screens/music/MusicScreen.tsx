import { useState } from 'react';

import { Button } from '../../components/Button';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { ProgressBar } from '../../components/ProgressBar';
import { useFileDrop } from '../../hooks/useFileDrop';
import { useMusicTransfer } from '../../hooks/useMusicTransfer';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { CopyResult } from './CopyResult';
import { PlanPreview } from './PlanPreview';
import { SourcePicker } from './SourcePicker';

interface Props {
  onChooseCard: () => void;
}

export function MusicScreen({ onChooseCard }: Props) {
  const { card } = useCard();
  const transfer = useMusicTransfer(card !== null);
  const [pickError, setPickError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const hovering = useFileDrop(transfer.add);

  if (!card) {
    return (
      <section className="screen">
        <h1>{t('Music')}</h1>
        <EmptyState title={t('Choose your card first')}>
          <p className="muted">{t('Music is copied onto the SD card of your player.')}</p>
          <Button variant="primary" onClick={onChooseCard}>
            {t('Go to Card')}
          </Button>
        </EmptyState>
      </section>
    );
  }

  const { plan } = transfer;
  const removing = plan?.deletions.length ?? 0;
  const start = () => {
    setConfirming(false);
    void transfer.copy();
  };

  return (
    <section className="screen">
      <h1>{t('Music')}</h1>
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
          <span>{t('Folder on the card (leave empty for the top level)')}</span>
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
            {t('Mirror a folder: also remove files from the card that are no longer in it')}
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
      {transfer.busy && <ProgressBar progress={transfer.progress} label={t('Copying music')} />}
      <div>
        <Button
          variant="primary"
          disabled={transfer.busy || !plan || plan.files + removing === 0}
          onClick={() => {
            if (removing > 0) setConfirming(true);
            else start();
          }}
        >
          {t('Copy to card')}
        </Button>
      </div>
      {transfer.result && <CopyResult result={transfer.result} />}
      <ConfirmDialog
        open={confirming}
        title={t('Remove files from the card?')}
        message={t('{count} files on the card are not in the folder and will be deleted.', {
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
