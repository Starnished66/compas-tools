import type { Model, ReleaseKind } from '../../api/types';
import { t } from '../../i18n';

/** Recovery key combination of each board (`firmware_update.c`). */
function recoveryKeys(model: Model): string {
  return model === 'r3ii_2025' ? t('Power + Play/Pause') : t('Power + Volume Up');
}

export function finishingSteps(kind: ReleaseKind, model: Model | null): string[] {
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
    ...(model
      ? [
          t('If the player does not start, use recovery: hold {keys}.', {
            keys: recoveryKeys(model),
          }),
        ]
      : []),
  ];
}
