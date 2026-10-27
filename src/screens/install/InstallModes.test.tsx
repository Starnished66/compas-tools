import { screen, waitFor } from '@testing-library/react';

import { adbStatus, listDevices } from '../../api/adb';
import { latestRelease } from '../../api/releases';
import type { CardInfo } from '../../api/types';
import { card } from '../../test/fixtures';
import { latest, renderScreen, tools, view } from '../../test/installScreen';

let current: CardInfo | null = null;
let cards: CardInfo[] = [];

vi.mock('../../api/adb');
vi.mock('../../api/releases');
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(), revealItemInDir: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }));
vi.mock('./AppUpdatePanel', () => ({ AppUpdatePanel: () => null }));
vi.mock('../../state/CardContext', () => ({
  CardProvider: ({ children }: { children: React.ReactNode }) => children,
  useCard: () => ({ card: current, select: vi.fn() }),
}));
vi.mock('../../state/ModeContext', () => ({
  useMode: () => ({
    cards,
    adbPlayers: [],
    scanning: false,
    rescan: vi.fn(),
    reportDevices: vi.fn(),
  }),
}));

describe('InstallScreen and the player modes', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    current = null;
    cards = [];
    vi.mocked(adbStatus).mockResolvedValue(tools);
    vi.mocked(latestRelease).mockResolvedValue(latest('update_available'));
  });

  it('tells a player in Storage mode how to get to ADB', async () => {
    cards = [card];
    vi.mocked(listDevices).mockResolvedValue([]);
    renderScreen();
    expect(
      await screen.findByText(
        'Your player is in Storage mode. Turn on ADB in the player’s boot menu (Tools ▸ ADB) to install or update.',
      ),
    ).toBeInTheDocument();
  });

  it('does not mention Storage mode when nothing is connected as a drive', async () => {
    vi.mocked(listDevices).mockResolvedValue([]);
    renderScreen();
    await screen.findByText('Installation and Updates');
    expect(screen.queryByText(/is in Storage mode/)).not.toBeInTheDocument();
  });

  it('shows the latest release with how the connected player compares', async () => {
    vi.mocked(listDevices).mockResolvedValue([
      view({ firmware: 'compas', player_version: 'v1.2.0' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(
        screen.getByText('Your player has v1.2.0; an update is available.'),
      ).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Compás', level: 2 })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'View on GitHub' })).toBeInTheDocument();
    });
  });
});
