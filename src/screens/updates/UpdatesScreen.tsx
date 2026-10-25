import { useCallback, useEffect, useState } from 'react';

import { errorMessage } from '../../api/invoke';
import { latestRelease, otherUpts, placeRelease } from '../../api/releases';
import type { LatestRelease, PlacedFile, ReleaseKind } from '../../api/types';
import { Button } from '../../components/Button';
import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { ModelPicker } from '../../components/ModelPicker';
import { Notice } from '../../components/Notice';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { PlaceAction } from './PlaceAction';
import { ReleaseSummary } from './ReleaseSummary';

interface Props {
  onChooseCard: () => void;
}

export function UpdatesScreen({ onChooseCard }: Props) {
  const { card, model, chooseModel } = useCard();
  const [latest, setLatest] = useState<LatestRelease | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [placed, setPlaced] = useState<{ kind: ReleaseKind; file: PlacedFile } | null>(null);
  const [activeKind, setActiveKind] = useState<ReleaseKind | null>(null);
  const [strays, setStrays] = useState<string[]>([]);
  const operation = useOperation();
  const cardRoot = card?.root ?? null;

  const check = useCallback(async (refresh: boolean) => {
    setLoadError(null);
    try {
      setLatest(await latestRelease(refresh));
    } catch (failure) {
      setLoadError(errorMessage(failure));
    }
  }, []);

  useEffect(() => {
    if (cardRoot) void check(false);
  }, [cardRoot, check]);

  // Stray .upt files matter before and after copying, so look whenever the plan changes.
  const plannedKind = placed?.kind ?? 'full_image';
  useEffect(() => {
    if (!cardRoot || !model) return;
    otherUpts(plannedKind, model).then(setStrays, () => {
      setStrays([]);
    });
  }, [cardRoot, model, plannedKind, placed]);

  if (!card) {
    return (
      <section className="screen">
        <h1>{t('Updates')}</h1>
        <EmptyState title={t('Choose your card first')}>
          <p className="muted">{t('Updates are copied onto the SD card of your player.')}</p>
          <Button variant="primary" onClick={onChooseCard}>
            {t('Go to Card')}
          </Button>
        </EmptyState>
      </section>
    );
  }

  const place = async (kind: ReleaseKind) => {
    if (!model) return;
    setActiveKind(kind);
    setPlaced(null);
    const file = await operation.run((onProgress) => placeRelease(kind, model, onProgress));
    setActiveKind(null);
    if (file) setPlaced({ kind, file });
  };
  const strayCount = strays.length;

  return (
    <section className="screen">
      <div className="screen__header">
        <h1>{t('Updates')}</h1>
        <Button onClick={() => void check(true)} disabled={operation.busy}>
          {t('Check again')}
        </Button>
      </div>
      {loadError && <ErrorBanner message={loadError} />}
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}
      {latest && (
        <ReleaseSummary latest={latest} installedVersion={card.device?.player_version ?? null} />
      )}
      {!card.device && (
        <Notice tone="warning">
          <p>
            {model
              ? t('The model comes from your choice, because the card has no device information.')
              : t('Choose the player model to pick the right update file.')}
          </p>
          <ModelPicker value={model} onChange={chooseModel} />
        </Notice>
      )}
      {strayCount > 0 && (
        <Notice tone="warning">
          <p>{t('{count} other .upt file on the card', { count: strayCount })}</p>
          <p>{strays.join(', ')}</p>
          <p>{t('Only the image you mean to install should be on the card. Delete the others.')}</p>
        </Notice>
      )}
      <div className="action-grid">
        <PlaceAction
          kind="player_only"
          model={model}
          title={t('Player only')}
          description={t(
            'Replaces just the player program. Quick, and keeps the rest of the system as it is.',
          )}
          disabled={!latest || !model || operation.busy}
          busy={operation.busy && activeKind === 'player_only'}
          progress={operation.progress}
          placed={placed?.kind === 'player_only' ? placed.file : null}
          onPlace={() => void place('player_only')}
        />
        <PlaceAction
          kind="full_image"
          model={model}
          title={t('Full firmware image')}
          description={t(
            'The complete system image. Use it when a release says the system itself changed.',
          )}
          disabled={!latest || !model || operation.busy}
          busy={operation.busy && activeKind === 'full_image'}
          progress={operation.progress}
          placed={placed?.kind === 'full_image' ? placed.file : null}
          onPlace={() => void place('full_image')}
        />
      </div>
    </section>
  );
}
