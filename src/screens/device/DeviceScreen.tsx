import { useCallback, useEffect, useState } from 'react';

import { adbStatus, listDevices } from '../../api/adb';
import { errorMessage } from '../../api/invoke';
import type { DeviceView, ToolsStatus } from '../../api/types';
import { ErrorBanner } from '../../components/ErrorBanner';
import { t } from '../../i18n';
import { DeviceList } from './DeviceList';
import { InstallPanel } from './InstallPanel';
import { LogsPanel } from './LogsPanel';
import { SetupGuide } from './SetupGuide';
import { ToolsSetup } from './ToolsSetup';

export function DeviceScreen() {
  const [tools, setTools] = useState<ToolsStatus | null>(null);
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const scan = useCallback(async () => {
    setScanning(true);
    setError(null);
    try {
      const found = await listDevices();
      setDevices(found);
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
  }, []);

  useEffect(() => {
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

  const installed = (status: ToolsStatus) => {
    setTools(status);
    void scan();
  };
  const device = devices.find((d) => d.serial === selected)?.info ?? null;

  return (
    <section className="screen">
      <h1>{t('Device')}</h1>
      <p className="muted">
        {t('Install or update the player’s software and collect logs over USB, using adb.')}
      </p>
      {error && (
        <ErrorBanner
          message={error}
          onDismiss={() => {
            setError(null);
          }}
        />
      )}
      {tools && <ToolsSetup status={tools} onInstalled={installed} />}
      {tools?.found && (
        <DeviceList
          devices={devices}
          selected={selected}
          scanning={scanning}
          onSelect={setSelected}
          onScan={() => void scan()}
        />
      )}
      {tools && <SetupGuide platform={tools.platform} />}
      {device && <InstallPanel key={device.serial} device={device} />}
      {device && <LogsPanel serial={device.serial} />}
    </section>
  );
}
