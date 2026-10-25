import { t } from '../i18n';
import { Notice } from './Notice';

/** Reminder shown wherever the card has just been written to. */
export function EjectAdvice() {
  return (
    <Notice tone="info">
      <p>
        {t(
          'Before removing the card, eject it in your system (or use Safely Remove) so every change is written to it.',
        )}
      </p>
    </Notice>
  );
}
