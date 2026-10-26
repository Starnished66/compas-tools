import { Notice } from '../../components/Notice';
import { t } from '../../i18n';

/** Why Compás Tools leaves the library to the player for this card. */
export function LibraryNotReady() {
  return (
    <Notice tone="warning">
      <p>
        {t(
          'The library will be built by the player on its next start. Updating the player (Updates screen) lets Compás Tools build it on this computer.',
        )}
      </p>
    </Notice>
  );
}
