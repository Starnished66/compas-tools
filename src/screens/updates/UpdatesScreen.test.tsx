import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { latestRelease } from '../../api/releases';
import type { LatestRelease } from '../../api/types';
import { UpdatesScreen } from './UpdatesScreen';

vi.mock('../../api/releases');
vi.mock('./AppUpdatePanel', () => ({ AppUpdatePanel: () => null }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));

function latest(boards: string[]): LatestRelease {
  return {
    freshness: 'unknown',
    release: {
      tag: 'v1.3.0',
      name: 'Compás 1.3',
      notes: 'What changed',
      published_at: '2026-10-17T17:45:54Z',
      url: 'https://example.test/release',
      player_only_boards: boards,
    },
  };
}

describe('UpdatesScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows the release and sends the user to the Device screen to install it', async () => {
    vi.mocked(latestRelease).mockResolvedValue(latest([]));
    const goToDevice = vi.fn();
    render(<UpdatesScreen onGoToDevice={goToDevice} />);
    expect(await screen.findByText('Compás 1.3')).toBeInTheDocument();
    expect(screen.getByText('This release is a full firmware update only.')).toBeInTheDocument();
    expect(screen.getByText(/installed over USB with ADB/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Install and update' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Go to Device' }));
    expect(goToDevice).toHaveBeenCalled();
  });

  it('says when a release can also update just the player', async () => {
    vi.mocked(latestRelease).mockResolvedValue(latest(['r1']));
    render(<UpdatesScreen onGoToDevice={vi.fn()} />);
    expect(
      await screen.findByText('This release can update the whole firmware, or just the player.'),
    ).toBeInTheDocument();
  });

  it('shows why the release could not be loaded', async () => {
    vi.mocked(latestRelease).mockRejectedValue({ code: 'network', message: 'offline' });
    render(<UpdatesScreen onGoToDevice={vi.fn()} />);
    expect(await screen.findByText(/offline/)).toBeInTheDocument();
  });
});
