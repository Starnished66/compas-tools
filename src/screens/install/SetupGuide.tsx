import { openUrl } from '@tauri-apps/plugin-opener';

import { Button } from '../../components/Button';
import { t } from '../../i18n';

const DRIVER_URL = 'https://developer.android.com/studio/run/win-usb';

/** How to switch ADB on, and why the card disappears from the computer while it is on. */
export function SetupGuide({ platform }: { platform: string }) {
  return (
    <details className="panel guide">
      <summary>{t('How to connect the player')}</summary>
      <div className="stack">
        <h3>{t('Stock HiBy firmware')}</h3>
        <ol className="steps">
          <li>{t('Open Settings > About.')}</li>
          <li>{t('Tap “About” 10 times until developer options are unlocked.')}</li>
          <li>{t('Turn on USB debugging (ADB), then connect the USB cable.')}</li>
        </ol>
        <h3>{t('Compás')}</h3>
        <ol className="steps">
          <li>{t('Open Settings > System > About > Developer Options.')}</li>
          <li>{t('Turn on ADB, then connect the USB cable.')}</li>
        </ol>
        <p>
          {t(
            'ADB and USB storage share the same USB port. While ADB is on, the player does not show up as a drive on your computer; to copy music as a drive, turn ADB off again (and the other way round).',
          )}
        </p>
        <p className="muted">
          {t('The player may ask to allow this computer: accept on the player’s screen.')}
        </p>
        {platform === 'windows' && (
          <div className="stack">
            <h3>{t('Windows driver')}</h3>
            <p>
              {t(
                'Windows needs the Google USB driver for the player (vendor ID 18d1, product ID d002). Install it, then connect the player again.',
              )}
            </p>
            <div>
              <Button
                onClick={() => {
                  void openUrl(DRIVER_URL);
                }}
              >
                {t('Get the Google USB driver')}
              </Button>
            </div>
          </div>
        )}
      </div>
    </details>
  );
}
