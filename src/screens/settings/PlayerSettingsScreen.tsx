import { useState } from 'react';

import { Button } from '../../components/Button';
import { Badge } from '../../components/Badge';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { EmptyState } from '../../components/EmptyState';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { useMode } from '../../state/ModeContext';
import { BOARD_NAMES } from '../install/labels';
import { ConnectPanel } from './ConnectPanel';
import { PendingBar } from './PendingBar';
import { SettingsForm } from './SettingsForm';
import { PlayerManagement } from './PlayerManagement';
import { describe, loudChanges } from './logic';
import { usePlayerSettings } from './useSettings';

/** The player seen over ADB: open its settings, edit them in a batch and apply. */
function AdbSettings({ serial }: { serial: string }) {
  const s = usePlayerSettings(serial);
  const [confirming, setConfirming] = useState(false);
  const { schema } = s;
  const loud = schema ? loudChanges(schema, s.values, s.staged) : [];

  const send = () => {
    if (loud.length > 0) setConfirming(true);
    else void s.apply([]);
  };

  if (s.phase.kind === 'loading') return <p className="muted">{t('Connecting to the player…')}</p>;
  if (s.phase.kind === 'connect' || !schema) {
    const problem = s.phase.kind === 'connect' ? s.phase.problem : null;
    const error = s.phase.kind === 'connect' ? s.phase.error : null;
    return (
      <ConnectPanel problem={problem} error={error} onConnect={(pin) => void s.connect(pin)} />
    );
  }

  const { device } = schema;
  return (
    <>
      <div className="settings-header">
        <p className="settings-strip">
          <span className="settings-strip__dot" aria-hidden="true" />
          <span>
            <strong>{device.model ?? t('Unknown model')}</strong>
            {' · '}
            {device.playerVersion
              ? t('player {version}', { version: device.playerVersion })
              : t('player version unknown')}
            {' · '}
            {t('ADB mode')}
            {s.revision !== null &&
              ` · ${t('settings revision {revision}', { revision: s.revision })}`}
          </span>
          <Badge tone="success">{t('Remote Control on')}</Badge>
          <Badge>{t('Connected through USB (adb forward)')}</Badge>
        </p>
        <Button icon="refresh" onClick={() => void s.reload()} disabled={s.busy}>
          {t('Reload from player')}
        </Button>
      </div>
      {s.error && <ErrorBanner message={s.error} onDismiss={s.dismissError} />}
      {s.notice && (
        <Notice tone="info">
          <p>{s.notice}</p>
          <div>
            <Button onClick={s.dismissNotice}>{t('Dismiss')}</Button>
          </div>
        </Notice>
      )}
      {!s.writable && (
        <Notice tone="warning">
          <p>
            {t(
              'The player is not accepting changes from this connection, so the settings are read-only.',
            )}
          </p>
        </Notice>
      )}
      <PlayerManagement session={s.sessionId}>
        <SettingsForm
          schema={schema}
          values={s.values}
          effective={s.effective}
          staged={s.staged}
          outcomes={s.outcomes}
          rescanStarted={s.rescanState === 'started'}
          disabled={s.busy}
          onChange={s.change}
          onRescan={() => void s.rescan()}
        />
        <PendingBar
          schema={schema}
          staged={s.staged}
          busy={s.busy}
          onDiscard={s.discard}
          onApply={send}
        />
      </PlayerManagement>
      <ConfirmDialog
        open={confirming}
        title={t('Raise the volume limit?')}
        message={t(
          'This goes above the level the player warns about. Loud sound can damage your hearing and the headphones.',
        )}
        details={loud.map((def) => `${def.label} ${describe(def, s.staged[def.key])}`)}
        confirmLabel={t('Raise the limit')}
        onCancel={() => {
          setConfirming(false);
        }}
        onConfirm={() => {
          setConfirming(false);
          void s.apply(loud.map((def) => def.key));
        }}
      />
    </>
  );
}

/** Changing player settings from the computer: needs the player over ADB. */
export function PlayerSettingsScreen() {
  const { adbPlayers, cards, scanning, rescan } = useMode();
  const { card } = useCard();
  const [chosen, setChosen] = useState<string | null>(null);
  const adb = adbPlayers.find((p) => p.serial === chosen) ?? adbPlayers[0];

  return (
    <section className="screen">
      <h1>{t('Player settings')}</h1>
      {adbPlayers.length > 1 && (
        <label className="row">
          <span>{t('Player')}</span>
          <select
            className="field"
            value={adb?.serial ?? ''}
            onChange={(event) => {
              setChosen(event.target.value);
            }}
          >
            {adbPlayers.map((p) => (
              <option key={p.serial} value={p.serial}>
                {`${p.board ? BOARD_NAMES[p.board] : t('Unknown model')} (${p.serial})`}
              </option>
            ))}
          </select>
        </label>
      )}
      {adb ? (
        <AdbSettings key={adb.serial} serial={adb.serial} />
      ) : card || cards.length > 0 ? (
        <Notice tone="info">
          <p>{t('Changing player settings needs ADB mode or Wi-Fi.')}</p>
          <details>
            <summary>{t('How to switch')}</summary>
            <ol className="steps">
              <li>{t('On the player, turn on ADB in the boot menu (Tools ▸ ADB).')}</li>
              <li>{t('Unplug and plug in the USB cable, then press Scan again.')}</li>
            </ol>
          </details>
        </Notice>
      ) : (
        <EmptyState title={t('No player selected')}>
          <p className="muted">{t('Connect the player by USB with ADB on, then scan.')}</p>
          <Button icon="refresh" onClick={() => void rescan()} disabled={scanning}>
            {t('Scan again')}
          </Button>
        </EmptyState>
      )}
    </section>
  );
}
