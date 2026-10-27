import { open } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';

import { errorMessage } from '../api/invoke';
import { t } from '../i18n';
import { CardList } from '../screens/library/CardList';
import { useCard } from '../state/CardContext';
import { useMode } from '../state/ModeContext';
import { Button } from './Button';
import { ErrorBanner } from './ErrorBanner';

/** Finds a player that shows up as a drive: the list, Scan again and the folder picker. */
export function FindPlayer() {
  const { card, select } = useCard();
  const { cards, scanning, rescan } = useMode();
  const [error, setError] = useState<string | null>(null);

  const guard = async (task: () => Promise<void>) => {
    setError(null);
    try {
      await task();
    } catch (failure) {
      setError(errorMessage(failure));
    }
  };
  const chooseFolder = () =>
    guard(async () => {
      const folder = await open({
        directory: true,
        title: t("Choose the player's drive or folder"),
      });
      if (folder) await select(folder);
    });

  return (
    <div className="stack">
      <div className="row">
        <Button icon="refresh" onClick={() => void rescan()} disabled={scanning}>
          {t('Scan again')}
        </Button>
        <Button icon="chooseFolder" variant="primary" onClick={() => void chooseFolder()}>
          {t('Choose folder…')}
        </Button>
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
    </div>
  );
}
