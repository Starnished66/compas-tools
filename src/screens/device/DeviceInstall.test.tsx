import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { adbStatus, installOnDevice, listDevices } from '../../api/adb';
import { latestRelease } from '../../api/releases';
import { latest, renderScreen, tools, view } from '../../test/deviceScreen';

vi.mock('../../api/adb');
vi.mock('../../api/releases');
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(), revealItemInDir: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }));

describe('DeviceScreen install checks', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(adbStatus).mockResolvedValue(tools);
    vi.mocked(listDevices).mockResolvedValue([view()]);
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available'));
  });

  it('warns before an install would go back to an older version', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.3.1' }),
    ]);
    vi.mocked(latestRelease).mockResolvedValue(latest('newer'));
    renderScreen();
    expect(
      await screen.findByText(/newer than the latest release v1\.3\.0\. Installing/),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Install the latest firmware' }));
    expect(await screen.findByText('Go back to the older version?')).toBeInTheDocument();
    expect(installOnDevice).not.toHaveBeenCalled();
    expect(latestRelease).toHaveBeenCalledWith(false, 'v1.3.1');
  });

  it('keeps reinstalling possible when the player is up to date', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.3.0' }),
    ]);
    vi.mocked(latestRelease).mockResolvedValue(latest('up_to_date'));
    renderScreen();
    expect(await screen.findByText(/already has v1\.3\.0, the latest release/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeEnabled();
  });

  it('disables player-only for a firmware-only release and says why', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.2.0' }),
    ]);
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available', []));
    renderScreen();
    expect(await screen.findByText(/full firmware update only/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Update only the player' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeEnabled();
  });
  it('offers the v1.3.0 firmware as the normal update for a v1.2.0 player', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.2.0' }),
    ]);
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available', []));
    renderScreen();
    const install = await screen.findByRole('button', { name: 'Install the latest firmware' });
    await waitFor(() => {
      expect(install).toBeEnabled();
    });
    expect(screen.getAllByText(/v1\.2\.0/).length).toBeGreaterThan(0);
    expect(latestRelease).toHaveBeenCalledWith(false, 'v1.2.0');
    expect(screen.queryByText(/goes back to the older version/)).not.toBeInTheDocument();
    expect(install.className).toContain('button--primary');
    expect(screen.getByRole('button', { name: 'Update only the player' })).toBeDisabled();
    await userEvent.click(install);
    expect(await screen.findByRole('button', { name: 'Install and reboot' })).toBeInTheDocument();
  });

  it('does not guess a version: unknown is not a downgrade and not "older"', async () => {
    vi.mocked(listDevices).mockResolvedValue([view({ firmware: 'compas', player_version: null })]);
    vi.mocked(latestRelease).mockResolvedValue(latest('unknown'));
    renderScreen();
    const install = await screen.findByRole('button', { name: 'Install the latest firmware' });
    await waitFor(() => {
      expect(install).toBeEnabled();
    });
    expect(latestRelease).toHaveBeenCalledWith(false, '');
    expect(screen.queryByText(/goes back to the older version/)).not.toBeInTheDocument();
  });

  it('asks for an SD card and blocks both installs until one is mounted', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.2.0', sd_mounted: false }),
    ]);
    renderScreen();
    expect(await screen.findByText('Please insert an SD card to upgrade.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Update only the player' })).toBeDisabled();
  });

  it('keeps installing disabled until the latest release is known', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.2.0' }),
    ]);
    vi.mocked(latestRelease).mockRejectedValue({ code: 'network', message: 'offline' });
    renderScreen();
    expect(await screen.findByText(/offline/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Update only the player' })).toBeDisabled();
  });
  it('names a TempoTec V1, offers its firmware and explains the stock case', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ board: 'tempotec_v1', firmware: 'compas', player_version: 'v1.2.0' }),
    ]);
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available', []));
    renderScreen();
    expect((await screen.findAllByText(/TempoTec Variations V1/)).length).toBeGreaterThan(0);
    const install = await screen.findByRole('button', { name: 'Install the latest firmware' });
    await waitFor(() => {
      expect(install).toBeEnabled();
    });
    await userEvent.click(install);
    expect(await screen.findByText(/v1\.upt/)).toBeInTheDocument();
  });

  it('does not offer to install Compás on a stock TempoTec V1', async () => {
    vi.mocked(listDevices).mockResolvedValue([view({ board: 'tempotec_v1', firmware: 'stock' })]);
    renderScreen();
    expect(
      await screen.findByText(/stock TempoTec Variations V1 is not supported yet/),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Install the latest firmware' })).toBeDisabled();
  });
});
