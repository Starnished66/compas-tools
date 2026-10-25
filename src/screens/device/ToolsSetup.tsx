import { useState } from 'react';

import { downloadAdb } from '../../api/adb';
import type { ToolsStatus } from '../../api/types';
import { Button } from '../../components/Button';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';

interface Props {
  status: ToolsStatus;
  onInstalled: (status: ToolsStatus) => void;
}

/** adb is needed to talk to the player; a system one is used, else Google's is downloaded once. */
export function ToolsSetup({ status, onInstalled }: Props) {
  const operation = useOperation();
  const [asking, setAsking] = useState(false);

  if (status.found) return null;

  const download = async () => {
    setAsking(false);
    const result = await operation.run((onProgress) => downloadAdb(onProgress));
    if (result) onInstalled(result);
  };

  return (
    <section className="panel stack" aria-label={t('adb')}>
      <h2>{t('adb is needed')}</h2>
      <p>
        {t(
          'Compás Tools talks to the player through adb, Google’s Android Debug Bridge. No adb was found on this computer.',
        )}
      </p>
      <div>
        <Button
          variant="primary"
          icon="install"
          disabled={operation.busy}
          onClick={() => {
            setAsking(true);
          }}
        >
          {t('Download adb…')}
        </Button>
      </div>
      {operation.busy && <ProgressBar progress={operation.progress} label={t('Downloading adb')} />}
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}
      <Notice tone="info">
        <p>{t('Or install it yourself (for example the android-tools package) and scan again.')}</p>
      </Notice>
      <ConfirmDialog
        open={asking}
        title={t('Download adb?')}
        message={t(
          'This downloads Google’s platform-tools (about 15 MB) over HTTPS and keeps them in this app’s data folder. Nothing else is installed.',
        )}
        confirmLabel={t('Download')}
        onConfirm={() => void download()}
        onCancel={() => {
          setAsking(false);
        }}
      />
    </section>
  );
}
