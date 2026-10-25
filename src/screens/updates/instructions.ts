import type { ReleaseKind } from '../../api/types';
import { t } from '../../i18n';

export function finishingSteps(kind: ReleaseKind): string[] {
  if (kind === 'player_only') {
    return [
      t('Safely eject the card and put it back in the player.'),
      t('On the player, open Settings > Updates > Player Update > Install from SD card.'),
      t('Or simply restart the player; it finds the new file on its own.'),
    ];
  }
  return [
    t('Safely eject the card and put it back in the player.'),
    t('On the player, open Settings > Updates > Firmware Update > Install from SD card.'),
    t('If the player does not start, use recovery: hold Power + Volume Up on the R1.'),
  ];
}
