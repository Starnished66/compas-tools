import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { useState } from 'react';

import { backupDevice, installOnDevice } from '../../api/adb';
import type { BackupReport, DeviceInfo, InstallReport, Model, ReleaseKind } from '../../api/types';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { ModelPicker } from '../../components/ModelPicker';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';
import { recoveryKeys } from '../updates/instructions';
import { BOARD_NAMES, firmwareLabel } from './labels';

interface Props {
  device: DeviceInfo;
}

export function InstallPanel({ device }: Props) {
  const operation = useOperation();
  const [chosen, setChosen] = useState<Model | null>(null);
  const [backup, setBackup] = useState<BackupReport | null>(null);
  const [report, setReport] = useState<InstallReport | null>(null);
  const [failed, setFailed] = useState(false);

  const model = device.board ?? chosen;
  // The first install replaces the vendor's system; take the user's data along first.
  const needsBackup = device.firmware !== 'compas' && backup === null;

  const runBackup = async () => {
    setReport(null);
    const result = await operation.run((p) => backupDevice(device.serial, p));
    if (result) setBackup(result);
  };

  const install = async (kind: ReleaseKind) => {
    if (!model) return;
    setReport(null);
    const result = await operation.run((p) => installOnDevice(device.serial, kind, model, p));
    setFailed(result === undefined);
    if (result) setReport(result);
  };

  return (
    <section className="panel stack" aria-label={t('Install and update')}>
      <h2>{t('Install and update')}</h2>
      <p className="muted">
        {board(device)} · {firmwareLabel(device.firmware)}
        {device.player_version ? ` · ${device.player_version}` : ''}
      </p>
      {!device.board && <ModelPicker value={chosen} onChange={setChosen} />}

      <div className="stack">
        <h3>{t('1. Back up the player')}</h3>
        <p>
          {t(
            'Copies the player’s user data to this computer. Account tokens, Wi-Fi details and the remote-control PIN are never copied.',
          )}
        </p>
        <div>
          <Button disabled={operation.busy} onClick={() => void runBackup()}>
            {t('Back up now')}
          </Button>
        </div>
        {backup && (
          <Notice tone="success">
            <p>{t('Backup saved to {path}', { path: backup.path })}</p>
            <p className="muted">
              {t('{count} files copied, {skipped} left out on purpose.', {
                count: backup.files,
                skipped: backup.skipped.length,
              })}
            </p>
            <div>
              <Button
                onClick={() => {
                  void revealItemInDir(backup.path);
                }}
              >
                {t('Show in folder')}
              </Button>
            </div>
          </Notice>
        )}
      </div>

      <div className="stack">
        <h3>{t('2. Install')}</h3>
        {needsBackup && (
          <p className="muted">
            {t('Back up the player first; installing is available after that.')}
          </p>
        )}
        <div className="row">
          <Button
            variant="primary"
            disabled={operation.busy || needsBackup || !model}
            onClick={() => void install('full_image')}
          >
            {t('Install the latest firmware')}
          </Button>
          {device.firmware === 'compas' && (
            <Button disabled={operation.busy || !model} onClick={() => void install('player_only')}>
              {t('Update only the player')}
            </Button>
          )}
        </div>
        <p className="muted">
          {t(
            'The file goes to the SD card, the player restarts into recovery and installs it, and this app checks the version afterwards. Keep the cable connected and do not turn the player off.',
          )}
        </p>
      </div>

      {operation.busy && (
        <ProgressBar progress={operation.progress} label={t('Working on the player')} />
      )}
      {operation.error && (
        <ErrorBanner message={operation.error} onDismiss={operation.clearError} />
      )}
      {failed && model && (
        <Notice tone="warning">
          <p>
            {t(
              'If the player does not start the update by itself, put it into recovery by hand: switch it off, then hold {keys} while connecting.',
              { keys: recoveryKeys(model) },
            )}
          </p>
        </Notice>
      )}
      {report && <InstallResult report={report} />}
    </section>
  );
}

function board(device: DeviceInfo): string {
  return device.board ? BOARD_NAMES[device.board] : t('Unknown model');
}

function InstallResult({ report }: { report: InstallReport }) {
  if (!report.device) {
    return (
      <Notice tone="warning">
        <p>
          {t(
            'The player did not come back in time. Installing can take several minutes; wait for it to start, then scan for it again.',
          )}
        </p>
      </Notice>
    );
  }
  const version = report.device.player_version ?? t('unknown');
  return (
    <Notice tone={report.freshness === 'up_to_date' ? 'success' : 'warning'}>
      <p>
        {report.freshness === 'up_to_date'
          ? t('Installed. The player now runs {version}.', { version })
          : t(
              'The player is back and reports version {version}, which is not the new release yet.',
              {
                version,
              },
            )}
      </p>
      <p className="muted">{t('Copied to {path} on the card.', { path: report.remote_path })}</p>
    </Notice>
  );
}
