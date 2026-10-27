import { useCallback, useEffect, useState } from 'react';

import { adbStatus, listDevices, probeDevice } from '../../api/adb';
import { errorMessage } from '../../api/invoke';
import type { DeviceView, ToolsStatus } from '../../api/types';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import { useCard } from '../../state/CardContext';
import { useMode } from '../../state/ModeContext';
import { AppUpdatePanel } from './AppUpdatePanel';
import { DeviceList } from './DeviceList';
import { InstallPanel } from './InstallPanel';
import { LogsPanel } from './LogsPanel';
import { ReleaseSummary } from './ReleaseSummary';
import { SetupGuide } from './SetupGuide';
import { StorageModeNotice } from './StorageModeNotice';
import { ToolsSetup } from './ToolsSetup';
import { useLatestRelease } from './useLatestRelease';

export function InstallScreen() {
  const [tools, setTools] = useState<ToolsStatus | null>(null);
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { card } = useCard();
  const { cards, reportDevices } = useMode();

  const scan = useCallback(async () => {
    setScanning(true);
    setError(null);
    try {
      const found = await listDevices();
      setDevices(found);
      reportDevices(found);
      // Keep the choice while that player is still connected; else take the only one.
      setSelected((current) => {
        const ready = found.filter((d) => d.state === 'device');
        if (current && ready.some((d) => d.serial === current)) return current;
        return ready.length === 1 ? (ready[0]?.serial ?? null) : null;
      });
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setScanning(false);
    }
  }, [reportDevices]);

  const probe = async (serial: string) => {
    try {
      const info = await probeDevice(serial);
      setDevices((current) => current.map((d) => (d.serial === serial ? { ...d, info } : d)));
      setSelected(serial);
    } catch (failure) {
      setError(errorMessage(failure));
    }
  };

  const checkTools = useCallback(() => {
    adbStatus().then(
      (status) => {
        setTools(status);
        if (status.found) void scan();
      },
      (failure: unknown) => {
        setError(errorMessage(failure));
      },
    );
  }, [scan]);

  useEffect(() => {
    checkTools();
  }, [checkTools]);

  const installed = (status: ToolsStatus) => {
    setTools(status);
    void scan();
  };
  const device = devices.find((d) => d.serial === selected)?.info ?? null;
  const release = useLatestRelease(device?.player_version ?? null);
  const seenOverAdb = devices.some((d) => d.state === 'device' && d.info !== null);
  const inStorageMode = card !== null || cards.length > 0;

  return (
    <section className="screen">
      <h1>{t('Installation and Updates')}</h1>
      <p className="muted">
        {t('Install or update the player’s software and collect logs over USB, using adb.')}
      </p>
      <AppUpdatePanel />
      {error && (
        <ErrorBanner
          message={error}
          onDismiss={() => {
            setError(null);
          }}
        />
      )}
      {tools && <ToolsSetup status={tools} onInstalled={installed} onCheckAgain={checkTools} />}
      {tools?.found && (
        <DeviceList
          devices={devices}
          selected={selected}
          scanning={scanning}
          onSelect={setSelected}
          onScan={() => void scan()}
          onProbe={(serial) => void probe(serial)}
        />
      )}
      {tools?.found && !scanning && !seenOverAdb && inStorageMode && <StorageModeNotice />}
      {tools && <SetupGuide platform={tools.platform} />}
      {!device && release.error && <ErrorBanner message={release.error} />}
      {release.latest && (
        <ReleaseSummary
          latest={release.latest}
          hasPlayer={device !== null}
          installedVersion={device?.player_version ?? null}
          onCheckAgain={release.reload}
        />
      )}
      {device && (
        <InstallPanel
          key={device.serial}
          device={device}
          onFinished={() => void scan()}
          onChoose={setSelected}
          latest={release.latest}
          releaseError={release.error}
        />
      )}
      {device && <LogsPanel serial={device.serial} />}
    </section>
  );
}
