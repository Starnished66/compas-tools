import { useEffect, useState } from 'react';

import { checkAppUpdate, installAppUpdate, type AppUpdate } from '../../api/appUpdate';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';

/** Updates of this app, shown only in builds that have the updater (see docs/RELEASING.md). */
export function AppUpdatePanel() {
  const operation = useOperation();
  const [update, setUpdate] = useState<AppUpdate | null>(null);
  const [installed, setInstalled] = useState(false);

  useEffect(() => {
    checkAppUpdate().then(setUpdate, () => undefined);
  }, []);

  if (!update?.enabled) return null;

  const install = async () => {
    const done = await operation.run(() => installAppUpdate());
    if (done) setInstalled(true);
  };

  return (
    <section className="panel stack" aria-label={t('Compás Tools updates')}>
      <h2>{t('Compás Tools updates')}</h2>
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}
      {installed ? (
        <Notice tone="success">
          <p>{t('Restart Compás Tools to finish the update.')}</p>
        </Notice>
      ) : update.version ? (
        <>
          <Notice tone="info">
            <p>{t('Compás Tools {version} is available.', { version: update.version })}</p>
          </Notice>
          <div>
            <Button variant="primary" disabled={operation.busy} onClick={() => void install()}>
              {t('Download and install')}
            </Button>
          </div>
        </>
      ) : (
        <p className="muted">{t('Compás Tools is up to date.')}</p>
      )}
    </section>
  );
}
