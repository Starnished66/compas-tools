import { Notice } from '../../components/Notice';
import { t } from '../../i18n';

/** A player is connected as a drive, but the install needs it over ADB. */
export function StorageModeNotice() {
  return (
    <Notice tone="info">
      <p>
        {t(
          'Your player is in Storage mode. Turn on ADB in the player’s boot menu (Tools ▸ ADB) to install or update.',
        )}
      </p>
    </Notice>
  );
}
