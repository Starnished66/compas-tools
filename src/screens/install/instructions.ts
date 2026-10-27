import type { Model } from '../../api/types';
import { t } from '../../i18n';

/** Recovery key combination of each board (`firmware_update.c`); null where not confirmed. */
export function recoveryKeys(model: Model): string | null {
  if (model === 'tempotec_v1') return null;
  return model === 'r3ii_2025' ? t('Power + Play/Pause') : t('Power + Volume Up');
}
