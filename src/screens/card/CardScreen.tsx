import { open } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useState } from 'react';

import { listCards } from '../../api/card';
import { errorMessage } from '../../api/invoke';
import type { CardInfo } from '../../api/types';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { CardDetails } from './CardDetails';
import { CardList } from './CardList';

export function CardScreen() {
  const { card, model, chooseModel, select, refresh } = useCard();
  const [cards, setCards] = useState<CardInfo[]>([]);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const guard = useCallback(async (task: () => Promise<void>) => {
    setError(null);
    try {
      await task();
    } catch (failure) {
      setError(errorMessage(failure));
    }
  }, []);

  const scan = useCallback(
    () =>
      guard(async () => {
        setScanning(true);
        try {
          setCards(await listCards());
        } finally {
          setScanning(false);
        }
      }),
    [guard],
  );

  useEffect(() => {
    void scan();
  }, [scan]);

  // Free space changes while other screens write to the card.
  useEffect(() => {
    void guard(refresh);
  }, [guard, refresh]);

  useEffect(() => {
    const [only] = cards;
    if (!card && cards.length === 1 && only) void guard(() => select(only.root));
  }, [card, cards, guard, select]);

  const chooseFolder = () =>
    guard(async () => {
      const folder = await open({
        directory: true,
        title: t("Choose the player's drive or folder"),
      });
      if (folder) await select(folder);
    });

  return (
    <section className="screen">
      <div className="screen__header">
        <h1>{t('Player')}</h1>
        <div className="row">
          <Button icon="refresh" onClick={() => void scan()} disabled={scanning}>
            {t('Scan again')}
          </Button>
          <Button icon="chooseFolder" variant="primary" onClick={() => void chooseFolder()}>
            {t('Choose folder…')}
          </Button>
        </div>
      </div>
      {error && (
        <ErrorBanner
          message={error}
          onDismiss={() => {
            setError(null);
          }}
        />
      )}
      <CardList
        cards={cards}
        selectedRoot={card?.root ?? null}
        scanning={scanning}
        onSelect={(chosen) => void guard(() => select(chosen.root))}
      />
      {card && <CardDetails card={card} model={model} onChooseModel={chooseModel} />}
    </section>
  );
}
