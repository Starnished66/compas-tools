import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import {
  adbStatus,
  backupDevice,
  collectLogs,
  downloadAdb,
  installOnDevice,
  listDevices,
  probeDevice,
  removeUpdateFile,
  saveLogs,
} from '../../api/adb';
import { latestRelease } from '../../api/releases';
import { bundle, info, latest, renderScreen, tools, view } from '../../test/installScreen';

vi.mock('../../api/adb');
vi.mock('../../api/releases');
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(), revealItemInDir: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(() => Promise.resolve('/home/me/logs.zip')),
}));

describe('DeviceScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(adbStatus).mockResolvedValue(tools);
    vi.mocked(listDevices).mockResolvedValue([view()]);
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available'));
  });

  it('asks before downloading adb when none is installed, then scans', async () => {
    vi.mocked(adbStatus).mockResolvedValue({ ...tools, found: false, source: null, version: null });
    vi.mocked(downloadAdb).mockResolvedValue({ ...tools, source: 'downloaded' });
    renderScreen();

    await userEvent.click(await screen.findByRole('button', { name: 'Download adb…' }));
    const asking = await screen.findByRole('dialog', { name: 'Download adb?' });
    expect(asking).toBeVisible();
    expect(asking).toHaveTextContent(/platform-tools 37\.0\.1/);
    expect(asking).toHaveTextContent(/pinned SHA-256/);
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
    expect((await screen.findAllByText(/HiBy R1 · Stock HiBy firmware/)).length).toBeGreaterThan(0);
    expect(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    ).toBeInTheDocument();
  });

  it('explains an unauthorized device instead of offering it', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      {
        serial: 'abc',
        state: 'unauthorized',
        product: null,
        model: null,
        likely_player: true,
        info: null,
      },
    ]);
    renderScreen();
    expect(await screen.findByText(/Allow this computer on the player/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Use abc' })).toBeDisabled();
    expect(screen.queryByText('Install and update')).not.toBeInTheDocument();
  });

  it('makes the first install wait for a backup and then installs the full image', async () => {
    vi.mocked(backupDevice).mockResolvedValue({
      path: '/data/backups/ingenic_1-1',
      quality: 'complete',
      files: 12,
      skipped: [{ path: 'mnt', reason: 'private' }],
      redacted: 1,
    });
    vi.mocked(installOnDevice).mockResolvedValue({
      remote_path: '/data/mnt/sd_0/r1.upt',
      device: info({ firmware: 'compas', player_version: '1.2.0' }),
      candidates: [],
      freshness: 'up_to_date',
      rebooted: true,
      confirmed: true,
    });
    renderScreen();

    const install = await screen.findByRole('button', { name: 'Install the latest firmware' });
    expect(install).toBeDisabled();
    expect(screen.getByText(/Copy the player’s settings first/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Copy now' }));
    expect(
      await screen.findByText(/Settings copy saved to \/data\/backups\/ingenic_1-1/),
    ).toBeInTheDocument();
    expect(screen.getByText(/12 files copied, 1 left out on purpose/)).toBeInTheDocument();

    expect(install).toBeEnabled();
    await userEvent.click(install);
    expect(installOnDevice).not.toHaveBeenCalled();
    const dialog = await screen.findByRole('dialog', { name: 'Install on ingenic_1?' });
    expect(dialog).toHaveTextContent('r1.upt');
    expect(dialog).toHaveTextContent('HiBy R1');
    expect(dialog).toHaveTextContent('ingenic_1');
    expect(dialog).toHaveTextContent(/reboot into recovery and install/);
    await userEvent.click(screen.getByRole('button', { name: 'Install and reboot' }));
    expect(await screen.findByText(/Installed. The player now runs 1.2.0/)).toBeInTheDocument();
    expect(installOnDevice).toHaveBeenCalledWith('ingenic_1', 'full_image', expect.any(Function));
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
    await userEvent.click(await screen.findByRole('button', { name: 'Install and reboot' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('bootmode.sh not found');
    expect(screen.getByText(/hold Power \+ Play\/Pause/)).toBeInTheDocument();
  });

  it('refuses to install on a player whose model is not recognised', async () => {
    vi.mocked(listDevices).mockResolvedValue([view({ board: null, firmware: 'compas' })]);
    renderScreen();
    expect(
      await screen.findByText('This player model is not recognised, so nothing can be installed.'),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText('Which model is your player?')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeDisabled();
  });

  it('does not call an unconfirmed install installed, and keeps the update file', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.3.0' }),
    ]);
    vi.mocked(installOnDevice).mockResolvedValue({
      remote_path: '/data/mnt/sd_0/r1.upt',
      device: info({ firmware: 'compas', player_version: 'v1.3.0' }),
      candidates: [],
      freshness: 'up_to_date',
      rebooted: false,
      confirmed: false,
    });
    renderScreen();
    await userEvent.click(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    );
    await userEvent.click(await screen.findByRole('button', { name: 'Install and reboot' }));
    expect(await screen.findByText(/The install is not confirmed/)).toBeInTheDocument();
    expect(screen.queryByText(/Installed\./)).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Delete the update file from the player' }),
    ).not.toBeInTheDocument();
  });

  it('blocks the first install after an unusable settings copy until the user insists', async () => {
    vi.mocked(backupDevice).mockResolvedValue({
      quality: 'unusable',
      path: '/data/backups/x',
      files: 0,
      skipped: [{ path: 'theme_overrides/icon.bin', reason: 'binary' }],
      redacted: 0,
    });
    renderScreen();
    const install = await screen.findByRole('button', { name: 'Install the latest firmware' });
    await userEvent.click(screen.getByRole('button', { name: 'Copy now' }));
    expect(await screen.findByText(/this copy is not usable/)).toBeInTheDocument();
    expect(install).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'Install without a settings copy' }));
    await waitFor(() => {
      expect(install).toBeEnabled();
    });
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

  it('asks again with the right file name before a player-only update, and not before backing nothing up', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ board: 'r3proii', firmware: 'compas', player_version: '1.1.0' }),
    ]);
    renderScreen();
    await userEvent.click(await screen.findByRole('button', { name: 'Update only the player' }));
    const dialog = await screen.findByRole('dialog', { name: 'Install on ingenic_1?' });
    expect(dialog).toHaveTextContent('compas_player_r3proii');
    expect(dialog).toHaveTextContent(/replace the player program/);
    expect(dialog).not.toHaveTextContent(/recovery/);
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(installOnDevice).not.toHaveBeenCalled();
  });

  it('does not probe a phone until the user asks', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      {
        serial: 'R58M12345AB',
        state: 'device',
        product: 'beyond1lte',
        model: 'SM_G973F',
        likely_player: false,
        info: null,
      },
    ]);
    vi.mocked(probeDevice).mockResolvedValue(info({ serial: 'R58M12345AB', firmware: 'unknown' }));
    renderScreen();
    expect(await screen.findByText(/Not recognised as a player \(SM_G973F\)/)).toBeInTheDocument();
    expect(probeDevice).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole('button', { name: 'Check R58M12345AB anyway' }));
    await waitFor(() => {
      expect(probeDevice).toHaveBeenCalledWith('R58M12345AB');
    });
  });

  it('has a Scan again button while adb is missing', async () => {
    vi.mocked(adbStatus).mockResolvedValue({ ...tools, found: false, source: null, version: null });
    renderScreen();
    await userEvent.click(await screen.findByRole('button', { name: 'Scan again' }));
    await waitFor(() => {
      expect(adbStatus).toHaveBeenCalledTimes(2);
    });
  });

  it('refreshes the player list after an install and offers to delete the update file', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: '1.1.0' }),
    ]);
    vi.mocked(installOnDevice).mockResolvedValue({
      remote_path: '/data/mnt/sd_0/r1.upt',
      device: info({ firmware: 'compas', player_version: '1.2.0' }),
      candidates: [],
      freshness: 'up_to_date',
      rebooted: true,
      confirmed: true,
    });
    vi.mocked(removeUpdateFile).mockResolvedValue(null);
    renderScreen();
    await userEvent.click(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    );
    await userEvent.click(await screen.findByRole('button', { name: 'Install and reboot' }));
    await userEvent.click(
      await screen.findByRole('button', { name: 'Delete the update file from the player' }),
    );
    expect(removeUpdateFile).not.toHaveBeenCalled();
    const confirm = await screen.findByRole('dialog', { name: 'Delete the update file?' });
    expect(confirm).toHaveTextContent('/data/mnt/sd_0/r1.upt');
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await waitFor(() => {
      expect(removeUpdateFile).toHaveBeenCalled();
    });
    expect(removeUpdateFile).toHaveBeenCalledWith('ingenic_1', '/data/mnt/sd_0/r1.upt');
    expect(await screen.findByText(/The update file was deleted/)).toBeInTheDocument();
    expect(vi.mocked(listDevices).mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it('asks which player is theirs when several answered after the restart', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: '1.1.0' }),
    ]);
    vi.mocked(installOnDevice).mockResolvedValue({
      remote_path: '/data/mnt/sd_0/r1.upt',
      device: null,
      candidates: [info({ serial: 'ingenic_a' }), info({ serial: 'ingenic_b' })],
      freshness: 'unknown',
      rebooted: false,
      confirmed: false,
    });
    renderScreen();
    await userEvent.click(
      await screen.findByRole('button', { name: 'Install the latest firmware' }),
    );
    await userEvent.click(await screen.findByRole('button', { name: 'Install and reboot' }));
    expect(await screen.findByText(/Which one is yours/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'ingenic_a' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'ingenic_b' })).toBeInTheDocument();
  });
});
