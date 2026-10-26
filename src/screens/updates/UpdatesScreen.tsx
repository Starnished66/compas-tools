import { useCallback, useEffect, useState } from 'react';

import { errorMessage } from '../../api/invoke';
import { latestRelease } from '../../api/releases';
import type { LatestRelease } from '../../api/types';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';
import { AppUpdatePanel } from './AppUpdatePanel';
import { ReleaseSummary } from './ReleaseSummary';

interface Props {
  onGoToDevice: () => void;
}

/** The latest release. It is installed from the Device screen, over USB with ADB. */
export function UpdatesScreen({ onGoToDevice }: Props) {
  const [latest, setLatest] = useState<LatestRelease | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const check = useCallback(async (refresh: boolean) => {
    setLoadError(null);
    try {
      setLatest(await latestRelease(refresh));
    } catch (failure) {
      setLoadError(errorMessage(failure));
    }
  }, []);

  useEffect(() => {
    void check(false);
  }, [check]);

  return (
    <section className="screen">
      <div className="screen__header">
        <h1>{t('Updates')}</h1>
        <Button icon="refresh" onClick={() => void check(true)}>
          {t('Check again')}
        </Button>
      </div>
      <AppUpdatePanel />
      {loadError && <ErrorBanner message={loadError} />}
      {latest && <ReleaseSummary latest={latest} />}
      <Notice tone="info">
        <p>
          {t(
            'Firmware and player updates are installed over USB with ADB, not through the SD card. Connect your player over USB with ADB turned on.',
          )}
        </p>
        <div>
          <Button variant="primary" onClick={onGoToDevice}>
            {t('Go to Device')}
          </Button>
        </div>
      </Notice>
    </section>
  );
}
