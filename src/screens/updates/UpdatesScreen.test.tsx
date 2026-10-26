import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { latestRelease, otherUpts, placeRelease } from '../../api/releases';
import type { LatestRelease } from '../../api/types';
import { OperationProvider } from '../../hooks/useOperation';
import { card } from '../../test/fixtures';
import { UpdatesScreen } from './UpdatesScreen';

vi.mock('../../api/releases');
vi.mock('./AppUpdatePanel', () => ({ AppUpdatePanel: () => null }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
let current: typeof card = card;
vi.mock('../../state/CardContext', () => ({
  useCard: () => ({ card: current, model: 'r1', chooseModel: vi.fn() }),
}));

function latest(freshness: LatestRelease['freshness'], boards: string[] = ['r1']): LatestRelease {
  return {
    freshness,
    release: {
      tag: 'v1.3.0',
      name: 'Compás',
      notes: '',
      published_at: '',
      url: '',
      player_only_boards: boards,
    },
  };
}

function renderScreen() {
  return render(
    <OperationProvider>
      <UpdatesScreen onChooseCard={vi.fn()} />
    </OperationProvider>,
  );
}

describe('UpdatesScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    current = card;
    vi.mocked(otherUpts).mockResolvedValue([]);
    vi.mocked(placeRelease).mockResolvedValue({ path: '/media/sd/r1.upt', other_upts: [] });
  });

  it('asks before installing a release older than the player', async () => {
    vi.mocked(latestRelease).mockResolvedValue(latest('newer'));
    renderScreen();
    const buttons = await screen.findAllByRole('button', { name: 'Install and update' });
    await userEvent.click(buttons[1] as HTMLElement);
    expect(await screen.findByText('Go back to the older version?')).toBeInTheDocument();
    expect(placeRelease).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Install anyway' }));
    expect(placeRelease).toHaveBeenCalledWith('full_image', 'r1', expect.any(Function));
  });

  it('does not offer player-only for a firmware-only release', async () => {
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available', []));
    renderScreen();
    expect(await screen.findByText(/full firmware update only/)).toBeInTheDocument();
    const buttons = await screen.findAllByRole('button', { name: 'Install and update' });
    expect(buttons[0]).toBeDisabled();
    expect(buttons[1]).toBeEnabled();
  });

  it('keeps player-only working when the release ships it', async () => {
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available'));
    renderScreen();
    const buttons = await screen.findAllByRole('button', { name: 'Install and update' });
    expect(buttons[0]).toBeEnabled();
    await userEvent.click(buttons[0] as HTMLElement);
    expect(placeRelease).toHaveBeenCalledWith('player_only', 'r1', expect.any(Function));
  });
  it('offers the v1.3.0 firmware as the normal update for a v1.2.0 player without device.json', async () => {
    current = { ...card, device: null };
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available', []));
    renderScreen();
    expect(
      await screen.findByText('This player is older than v1.3.0; an update is available.'),
    ).toBeInTheDocument();
    const buttons = await screen.findAllByRole('button', { name: 'Install and update' });
    expect(buttons[1]?.className).toContain('button--primary');
    expect(buttons[0]).toBeDisabled();
    await userEvent.click(buttons[1] as HTMLElement);
    expect(screen.queryByText('Go back to the older version?')).not.toBeInTheDocument();
    expect(placeRelease).toHaveBeenCalledWith('full_image', 'r1', expect.any(Function));
  });

  it('never warns about a downgrade when the version is simply unknown', async () => {
    current = { ...card, device: null };
    vi.mocked(latestRelease).mockResolvedValue(latest('unknown', ['r1']));
    renderScreen();
    const buttons = await screen.findAllByRole('button', { name: 'Install and update' });
    await userEvent.click(buttons[1] as HTMLElement);
    expect(screen.queryByText('Go back to the older version?')).not.toBeInTheDocument();
    expect(placeRelease).toHaveBeenCalled();
  });
});
