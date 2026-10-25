import type { Firmware, Model, ReleaseKind } from '../../api/types';
import { t } from '../../i18n';

export const BOARD_NAMES: Record<Model, string> = {
  r1: 'Hiby R1',
  r3proii: 'Hiby R3 Pro II',
  r3ii_2025: 'Hiby R3 II (2025)',
};

export function firmwareLabel(firmware: Firmware): string {
  switch (firmware) {
    case 'stock':
      return t('Stock HiBy firmware');
    case 'compas':
      return t('Compás');
    case 'unknown':
      return t('Not recognised');
  }
}

/** The release file an install puts on the card (mirrors the release asset names). */
export function releaseFileName(kind: ReleaseKind, model: Model): string {
  if (kind === 'full_image') return `${model}.upt`;
  return model === 'r1' ? 'compas_player' : `compas_player_${model}`;
}
