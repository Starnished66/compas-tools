import { render } from '@testing-library/react';

import type { DeviceInfo, DeviceView, LatestRelease, LogBundle, ToolsStatus } from '../api/types';
import { OperationProvider } from '../hooks/useOperation';
import { CardProvider } from '../state/CardContext';
import { InstallScreen } from '../screens/install/InstallScreen';

export const tools: ToolsStatus = {
  found: true,
  source: 'system',
  version: 'Android Debug Bridge 1.0.41',
  platform: 'linux',
  download_version: '37.0.1',
};

export function latest(
  freshness: LatestRelease['freshness'],
  boards: string[] = ['r1', 'r3proii', 'r3ii_2025'],
): LatestRelease {
  return {
    freshness,
    release: {
      tag: 'v1.3.0',
      name: 'Compás',
      notes: '',
      published_at: '',
      url: '',
      player_only_boards: boards,
      full_image_boards: ['r1', 'r3proii', 'r3ii_2025', 'tempotec_v1'],
    },
  };
}

export function info(overrides: Partial<DeviceInfo> = {}): DeviceInfo {
  return {
    serial: 'ingenic_1',
    board: 'r1',
    firmware: 'stock',
    player_version: null,
    plugin_api: null,
    sd_mounted: true,
    ...overrides,
  };
}

export function view(overrides: Partial<DeviceInfo> = {}): DeviceView {
  return {
    serial: 'ingenic_1',
    state: 'device',
    product: 'mako',
    model: 'Nexus_4',
    likely_player: true,
    info: info(overrides),
  };
}

export const bundle: LogBundle = {
  summary: 'Board: r1',
  files: [
    {
      name: 'usr_data/player_crash.log',
      source: '/usr/data/player_crash.log',
      content: 'pin=<redacted>',
      bytes: 14,
      redactions: 1,
      missing: false,
    },
    {
      name: 'sd/usb_dac_bridge.log',
      source: 'x',
      content: '',
      bytes: 0,
      redactions: 0,
      missing: true,
    },
  ],
};

export function renderScreen() {
  return render(
    <CardProvider>
      <OperationProvider>
        <InstallScreen />
      </OperationProvider>
    </CardProvider>,
  );
}
