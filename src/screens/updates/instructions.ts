import type { Model } from '../../api/types';
import { t } from '../../i18n';

/** Recovery key combination of each board (`firmware_update.c`). */
export function recoveryKeys(model: Model): string {
  return model === 'r3ii_2025' ? t('Power + Play/Pause') : t('Power + Volume Up');
}
