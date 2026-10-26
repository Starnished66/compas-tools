import type { Model, PlacedFile, Progress, ReleaseKind } from '../../api/types';
import { useState } from 'react';

import { Button } from '../../components/Button';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { t } from '../../i18n';
import { finishingSteps } from './instructions';

interface Props {
  kind: ReleaseKind;
  model: Model | null;
  title: string;
  description: string;
  disabled: boolean;
  busy: boolean;
  progress: Progress | null;
  placed: PlacedFile | null;
  onPlace: () => void;
  /** Installing is the expected next step; otherwise the button is not highlighted. */
  recommended?: boolean;
  /** Set when installing goes back to an older version; asks first. */
  downgradeMessage?: string | null;
  /** Why this action is not offered with this release; the button stays disabled. */
  unavailable?: string | null;
}

/** One way of putting a release on the card, with what to do next. */
export function PlaceAction({
  kind,
  model,
  title,
  description,
  disabled,
  busy,
  progress,
  placed,
  onPlace,
  recommended = true,
  downgradeMessage = null,
  unavailable = null,
}: Props) {
  const [asking, setAsking] = useState(false);
  return (
    <section className="panel stack" aria-label={title}>
      <h2>{title}</h2>
      <p className="muted">{description}</p>
      <div>
        <Button
          icon="copy"
          variant={recommended && !unavailable ? 'primary' : 'secondary'}
          disabled={disabled || busy || !!unavailable}
          onClick={() => {
            if (downgradeMessage) setAsking(true);
            else onPlace();
          }}
        >
          {t('Install and update')}
        </Button>
      </div>
      {unavailable && <p className="muted">{unavailable}</p>}
      {asking && downgradeMessage && (
        <ConfirmDialog
          open
          title={t('Go back to the older version?')}
          message={downgradeMessage}
          confirmLabel={t('Install anyway')}
          onConfirm={() => {
            setAsking(false);
            onPlace();
          }}
          onCancel={() => {
            setAsking(false);
          }}
        />
      )}
      {busy && <ProgressBar progress={progress} label={title} />}
      {placed && !busy && (
        <Notice tone="success">
          <p>{t('Saved to {path}', { path: placed.path })}</p>
          <ol className="steps">
            {finishingSteps(kind, model).map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        </Notice>
      )}
    </section>
  );
}
