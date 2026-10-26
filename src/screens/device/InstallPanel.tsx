import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { useEffect, useState } from 'react';

import { backupDevice, installOnDevice, removeUpdateFile } from '../../api/adb';
import { errorMessage } from '../../api/invoke';
import { latestRelease } from '../../api/releases';
import type {
  BackupReport,
  DeviceInfo,
  InstallReport,
  LatestRelease,
  Model,
  ReleaseKind,
} from '../../api/types';
import { Button } from '../../components/Button';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { ErrorBanner } from '../../components/ErrorBanner';
import { ModelPicker } from '../../components/ModelPicker';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { useOperation } from '../../hooks/useOperation';
import { t } from '../../i18n';
import { recoveryKeys } from '../updates/instructions';
import { downgradeText, firmwareOnlyText, releaseGate } from '../updates/releaseGate';
import { BOARD_NAMES, firmwareLabel, releaseFileName } from './labels';

interface Props {
  device: DeviceInfo;
  /** Called when an install ended, so the player list can be read again. */
  onFinished: () => void;
  /** Several players answered after the install; the user picks theirs. */
  onChoose: (serial: string) => void;
}

const SKIP_TEXT = {
  private: () => t('credentials or private data'),
  too_large: () => t('too large'),
  binary: () => t('not text, so it cannot be masked'),
  unreadable: () => t('could not be read'),
  unsupported_name: () => t('its name cannot be copied safely'),
};

export function InstallPanel({ device, onFinished, onChoose }: Props) {
  const operation = useOperation();
  const [chosen, setChosen] = useState<Model | null>(null);
  const [backup, setBackup] = useState<BackupReport | null>(null);
  const [report, setReport] = useState<InstallReport | null>(null);
  const [failed, setFailed] = useState(false);
  const [asking, setAsking] = useState<ReleaseKind | null>(null);
  const [removed, setRemoved] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  const [latest, setLatest] = useState<LatestRelease | null>(null);
  const [releaseError, setReleaseError] = useState<string | null>(null);

  const model = device.board ?? chosen;
  const installedVersion = device.player_version;
  const olderThan = device.older_than;
  // Compared with the version read over adb, not with any selected player.
  useEffect(() => {
    const live = { current: true };
    setLatest(null);
    setReleaseError(null);
    void (async () => {
      try {
        const found = await latestRelease(false, installedVersion ?? '', olderThan !== null);
        if (live.current) setLatest(found);
      } catch (failure) {
        if (live.current) setReleaseError(errorMessage(failure));
      }
    })();
    return () => {
      live.current = false;
    };
  }, [installedVersion, olderThan]);
  const gate = releaseGate(latest, model);
  const downgrade = latest && gate.downgrade ? downgradeText(installedVersion, latest) : null;
  // The first install replaces the vendor's system; take the user's settings along first.
  const needsBackup = device.firmware !== 'compas' && backup === null;

  const runBackup = async () => {
    setReport(null);
    const result = await operation.run((p) => backupDevice(device.serial, p));
    if (result) setBackup(result);
  };

  const install = async (kind: ReleaseKind) => {
    setAsking(null);
    if (!model) return;
    setReport(null);
    setRemoved(false);
    const result = await operation.run((p) => installOnDevice(device.serial, kind, model, p));
    setFailed(result === undefined);
    if (result) setReport(result);
    onFinished();
  };

  const removeImage = async (path: string) => {
    setDeleting(null);
    const done = await operation.run(() => removeUpdateFile(device.serial, path));
    if (done !== undefined) setRemoved(true);
  };

  return (
    <section className="panel stack" aria-label={t('Install and update')}>
      <h2>{t('Install and update')}</h2>
      <p className="muted">
        {device.board ? BOARD_NAMES[device.board] : t('Unknown model')} ·{' '}
        {firmwareLabel(device.firmware)}
        {device.player_version ? ` · ${device.player_version}` : ''}
        {!device.player_version && olderThan
          ? ` · ${t('older than {version}', { version: olderThan })}`
          : ''}
      </p>
      {!device.board && <ModelPicker value={chosen} onChange={setChosen} />}

      <div className="stack">
        <h3>{t('1. Back up the player')}</h3>
        <p>
          {t(
            'Copies the player’s own settings and logs to this computer. Your music, account tokens, server passwords, Wi-Fi details and the remote-control PIN are never copied, and anything that looks secret is masked before it is saved.',
          )}
        </p>
        <div>
          <Button disabled={operation.busy} onClick={() => void runBackup()}>
            {t('Back up now')}
          </Button>
        </div>
        {backup && <BackupResult backup={backup} />}
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
            variant={gate.recommended ? 'primary' : 'secondary'}
            disabled={operation.busy || needsBackup || !model || !latest}
            onClick={() => {
              setAsking('full_image');
            }}
          >
            {t('Install the latest firmware')}
          </Button>
          {device.firmware === 'compas' && (
            <Button
              disabled={operation.busy || !model || !latest || !gate.playerOnlyAvailable}
              onClick={() => {
                setAsking('player_only');
              }}
            >
              {t('Update only the player')}
            </Button>
          )}
        </div>
        {!latest && !releaseError && <p className="muted">{t('Checking the latest release…')}</p>}
        {releaseError && <ErrorBanner message={releaseError} />}
        {device.firmware === 'compas' && !gate.playerOnlyAvailable && (
          <p className="muted">{firmwareOnlyText()}</p>
        )}
        {downgrade && (
          <Notice tone="warning">
            <p>{downgrade}</p>
          </Notice>
        )}
        {latest?.freshness === 'up_to_date' && (
          <p className="muted">
            {t(
              'Your player already has {version}, the latest release. Reinstalling is possible if you need it.',
              {
                version: installedVersion ?? '',
              },
            )}
          </p>
        )}
        <p className="muted">
          {t(
            'Firmware: the image is copied to the player, which restarts into recovery and installs it, and this app checks the version afterwards. Player only: just the player program is replaced and the player restarts. Keep the cable connected and do not switch the player off.',
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
      {report && (
        <InstallResult
          report={report}
          removed={removed}
          busy={operation.busy}
          onChoose={onChoose}
          onRemove={setDeleting}
        />
      )}

      {deleting && (
        <ConfirmDialog
          open
          title={t('Delete the update file?')}
          message={t(
            '{path} will be deleted from the player ({serial}). The installed player is not affected.',
            { path: deleting, serial: device.serial },
          )}
          confirmLabel={t('Delete')}
          destructive
          onConfirm={() => void removeImage(deleting)}
          onCancel={() => {
            setDeleting(null);
          }}
        />
      )}
      {asking && model && (
        <ConfirmDialog
          open
          title={
            downgrade
              ? t('Go back to the older version?')
              : t('Install on {serial}?', { serial: device.serial })
          }
          message={`${downgrade ? `${downgrade} ` : ''}${
            asking === 'full_image'
              ? t(
                  '{file} will be copied to {board} ({serial}). The player will then reboot into recovery and install it. Do not disconnect it or switch it off.',
                  {
                    file: releaseFileName(asking, model),
                    board: BOARD_NAMES[model],
                    serial: device.serial,
                  },
                )
              : t(
                  '{file} will replace the player program on {board} ({serial}). The player will then restart.',
                  {
                    file: releaseFileName(asking, model),
                    board: BOARD_NAMES[model],
                    serial: device.serial,
                  },
                )
          }`}
          confirmLabel={
            downgrade
              ? t('Install anyway')
              : asking === 'full_image'
                ? t('Install and reboot')
                : t('Update and restart')
          }
          onConfirm={() => void install(asking)}
          onCancel={() => {
            setAsking(null);
          }}
        />
      )}
    </section>
  );
}

function BackupResult({ backup }: { backup: BackupReport }) {
  return (
    <Notice tone="success">
      <p>{t('Backup saved to {path}', { path: backup.path })}</p>
      <p className="muted">
        {t('{count} files copied, {skipped} left out on purpose.', {
          count: backup.files,
          skipped: backup.skipped.length,
        })}
      </p>
      {backup.skipped.length > 0 && (
        <details>
          <summary>{t('What was left out')}</summary>
          <ul>
            {backup.skipped.map((s) => (
              <li key={s.path}>
                {s.path}: {SKIP_TEXT[s.reason]()}
              </li>
            ))}
          </ul>
        </details>
      )}
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
  );
}

interface ResultProps {
  report: InstallReport;
  removed: boolean;
  busy: boolean;
  onChoose: (serial: string) => void;
  onRemove: (path: string) => void;
}

function InstallResult({ report, removed, busy, onChoose, onRemove }: ResultProps) {
  if (report.candidates.length > 0) {
    return (
      <Notice tone="warning">
        <p>{t('Several players answered after the restart. Which one is yours?')}</p>
        <div className="row">
          {report.candidates.map((c) => (
            <Button
              key={c.serial}
              onClick={() => {
                onChoose(c.serial);
              }}
            >
              {c.serial}
            </Button>
          ))}
        </div>
      </Notice>
    );
  }
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
  const confirmed = report.freshness === 'up_to_date';
  return (
    <Notice tone={confirmed ? 'success' : 'warning'}>
      <p>
        {confirmed
          ? t('Installed. The player now runs {version}.', { version })
          : t(
              'The player is back and reports version {version}, which is not the new release yet.',
              {
                version,
              },
            )}
      </p>
      <p className="muted">{t('Copied to {path} on the player.', { path: report.remote_path })}</p>
      {confirmed && report.remote_path.toLowerCase().endsWith('.upt') && !removed && (
        <div className="stack">
          <p>{t('The update file is still on the player. It is not needed any more.')}</p>
          <div>
            <Button
              disabled={busy}
              onClick={() => {
                onRemove(report.remote_path);
              }}
            >
              {t('Delete the update file from the player')}
            </Button>
          </div>
        </div>
      )}
      {removed && <p>{t('The update file was deleted from the player.')}</p>}
    </Notice>
  );
}
