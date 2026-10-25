import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import {
  adbStatus,
  backupDevice,
  collectLogs,
  downloadAdb,
  installOnDevice,
  listDevices,
  saveLogs,
} from '../../api/adb';
import type { DeviceInfo, DeviceView, LogBundle, ToolsStatus } from '../../api/types';
import { OperationProvider } from '../../hooks/useOperation';
import { DeviceScreen } from './DeviceScreen';

vi.mock('../../api/adb');
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(), revealItemInDir: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(() => Promise.resolve('/home/me/logs.zip')),
}));

const tools: ToolsStatus = {
  found: true,
  source: 'system',
  version: 'Android Debug Bridge 1.0.41',
  platform: 'linux',
};

function info(overrides: Partial<DeviceInfo> = {}): DeviceInfo {
  return {
    serial: 'ingenic_1',
    board: 'r1',
    firmware: 'stock',
    player_version: null,
    plugin_api: null,
    ...overrides,
  };
}

function view(overrides: Partial<DeviceInfo> = {}): DeviceView {
  return { serial: 'ingenic_1', state: 'device', info: info(overrides) };
}

const bundle: LogBundle = {
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

function renderScreen() {
  return render(
    <OperationProvider>
      <DeviceScreen />
    </OperationProvider>,
  );
}

describe('DeviceScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(adbStatus).mockResolvedValue(tools);
    vi.mocked(listDevices).mockResolvedValue([view()]);
  });

  it('asks before downloading adb when none is installed, then scans', async () => {
    vi.mocked(adbStatus).mockResolvedValue({ ...tools, found: false, source: null, version: null });
    vi.mocked(downloadAdb).mockResolvedValue({ ...tools, source: 'downloaded' });
    renderScreen();

    await userEvent.click(await screen.findByRole('button', { name: 'Download adb…' }));
    expect(await screen.findByRole('dialog', { name: 'Download adb?' })).toBeVisible();
    expect(downloadAdb).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole('button', { name: 'Download' }));
    await waitFor(() => {
      expect(downloadAdb).toHaveBeenCalled();
    });
    expect(await screen.findByText('Connected players')).toBeInTheDocument();
    expect(listDevices).toHaveBeenCalled();
  });

  it('shows the board and firmware it recognised and picks the only player', async () => {
    renderScreen();
    expect((await screen.findAllByText(/Hiby R1 · Stock HiBy firmware/)).length).toBeGreaterThan(0);
    expect(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    ).toBeInTheDocument();
  });

  it('explains an unauthorized device instead of offering it', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      { serial: 'abc', state: 'unauthorized', info: null },
    ]);
    renderScreen();
    expect(await screen.findByText(/Allow this computer on the player/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Use abc' })).toBeDisabled();
    expect(screen.queryByText('Install and update')).not.toBeInTheDocument();
  });

  it('makes the first install wait for a backup and then installs the full image', async () => {
    vi.mocked(backupDevice).mockResolvedValue({
      path: '/data/backups/ingenic_1-1',
      files: 12,
      skipped: ['.compas/plugins'],
      redacted: 1,
    });
    vi.mocked(installOnDevice).mockResolvedValue({
      remote_path: '/data/mnt/sd_0/r1.upt',
      device: info({ firmware: 'compas', player_version: '1.2.0' }),
      freshness: 'up_to_date',
    });
    renderScreen();

    const install = await screen.findByRole('button', { name: 'Install the latest firmware' });
    expect(install).toBeDisabled();
    expect(screen.getByText(/Back up the player first/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Back up now' }));
    expect(
      await screen.findByText(/Backup saved to \/data\/backups\/ingenic_1-1/),
    ).toBeInTheDocument();
    expect(screen.getByText(/12 files copied, 1 left out on purpose/)).toBeInTheDocument();

    expect(install).toBeEnabled();
    await userEvent.click(install);
    expect(await screen.findByText(/Installed. The player now runs 1.2.0/)).toBeInTheDocument();
    expect(installOnDevice).toHaveBeenCalledWith(
      'ingenic_1',
      'full_image',
      'r1',
      expect.any(Function),
    );
  });

  it('lets a Compás device update without a backup and offers player-only', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: '1.1.0' }),
    ]);
    renderScreen();
    expect(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    ).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Update only the player' })).toBeEnabled();
  });

  it('names the right recovery keys when the install cannot start', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ board: 'r3ii_2025', firmware: 'compas', player_version: '1.1.0' }),
    ]);
    vi.mocked(installOnDevice).mockRejectedValue({
      code: 'adb',
      message: 'adb: bootmode.sh not found',
    });
    renderScreen();
    await userEvent.click(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    );
    expect(await screen.findByRole('alert')).toHaveTextContent('bootmode.sh not found');
    expect(screen.getByText(/hold Power \+ Play\/Pause/)).toBeInTheDocument();
  });

  it('asks for the model when the screen size did not identify it', async () => {
    vi.mocked(listDevices).mockResolvedValue([view({ board: null, firmware: 'compas' })]);
    renderScreen();
    expect(await screen.findByLabelText('Which player is this card for?')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeDisabled();
  });

  it('shows exactly what the log report contains before saving it', async () => {
    vi.mocked(collectLogs).mockResolvedValue(bundle);
    vi.mocked(saveLogs).mockResolvedValue(null);
    renderScreen();

    await userEvent.click(await screen.findByRole('button', { name: 'Collect logs' }));
    expect(await screen.findByText('usr_data/player_crash.log')).toBeInTheDocument();
    expect(screen.getByText(/1 masked/)).toBeInTheDocument();
    expect(screen.queryByText('sd/usb_dac_bridge.log')).not.toBeInTheDocument();
    expect(screen.getByText(/1 file was not on this player/)).toBeInTheDocument();
    expect(saveLogs).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole('button', { name: 'Save as zip…' }));
    await waitFor(() => {
      expect(saveLogs).toHaveBeenCalledWith('/home/me/logs.zip');
    });
    expect(await screen.findByText(/Saved to \/home\/me\/logs.zip/)).toBeInTheDocument();
  });

  it('shows the Windows driver guidance only on Windows', async () => {
    renderScreen();
    await screen.findByText('How to connect the player');
    expect(screen.queryByText('Windows driver')).not.toBeInTheDocument();
  });

  it('shows the Windows driver link on Windows', async () => {
    vi.mocked(adbStatus).mockResolvedValue({ ...tools, platform: 'windows' });
    renderScreen();
    expect(await screen.findByText('Windows driver')).toBeInTheDocument();
    expect(screen.getByText(/vendor ID 18d1, product ID d002/)).toBeInTheDocument();
  });
});
