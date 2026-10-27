import type { ReactNode } from 'react';

import { t } from '../i18n';
import { useCard } from '../state/CardContext';
import { useMode } from '../state/ModeContext';
import { FindPlayer } from './FindPlayer';
import { Notice } from './Notice';

/** What a screen needs the player's files for, to word the ADB-mode notice. */
export type StorageNeed = 'music' | 'plugins';

function HowToSwitch() {
  return (
    <details>
      <summary>{t('How to switch')}</summary>
      <ol className="steps">
        <li>{t('On the player, turn ADB off: Settings ▸ Developer Options ▸ ADB.')}</li>
        <li>{t('Then choose Settings ▸ System ▸ USB Mode ▸ Storage.')}</li>
        <li>{t('Unplug and plug in the USB cable, then press Scan again.')}</li>
      </ol>
    </details>
  );
}

/**
 * Shows its children when a player is available as a drive. A player seen only over ADB gets an
 * explanation of how to switch; "no player found" is shown only when nothing is connected.
 */
export function StorageGate({ need, children }: { need: StorageNeed; children: ReactNode }) {
  const { card } = useCard();
  const { adbPlayers, cards } = useMode();
  if (card) return <>{children}</>;
  if (adbPlayers.length > 0 && cards.length === 0) {
    return (
      <Notice tone="info">
        <p>
          {need === 'music'
            ? t(
                'Your player is connected in ADB mode. Put it in Storage mode to manage music and the library.',
              )
            : t('Your player is connected in ADB mode. Put it in Storage mode to manage plugins.')}
        </p>
        <HowToSwitch />
      </Notice>
    );
  }
  return <FindPlayer />;
}
