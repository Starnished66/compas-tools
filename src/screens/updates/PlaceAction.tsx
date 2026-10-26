import type { Model, PlacedFile, Progress, ReleaseKind } from '../../api/types';
import { Button } from '../../components/Button';
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
}: Props) {
  return (
    <section className="panel stack" aria-label={title}>
      <h2>{title}</h2>
      <p className="muted">{description}</p>
      <div>
        <Button icon="copy" variant="primary" disabled={disabled || busy} onClick={onPlace}>
          {t('Install and update')}
        </Button>
      </div>
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
