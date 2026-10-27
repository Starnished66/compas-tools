import { render, screen } from '@testing-library/react';

import type { CardInfo, DeviceInfo } from '../../api/types';
import { card } from '../../test/fixtures';
import { LibraryScreen } from './LibraryScreen';

let current: CardInfo | null = null;
let adbPlayers: DeviceInfo[] = [];
let cards: CardInfo[] = [];

vi.mock('../../state/CardContext', () => ({
  useCard: () => ({ card: current, select: vi.fn() }),
}));
vi.mock('../../state/ModeContext', () => ({
  useMode: () => ({ cards, adbPlayers, scanning: false, rescan: vi.fn() }),
}));
vi.mock('./PlayerPanel', () => ({ PlayerPanel: () => <p>player panel</p> }));
vi.mock('./AddMusic', () => ({ AddMusic: () => <p>add music</p> }));

const adbPlayer = { serial: 'ingenic_1', board: 'r1' } as DeviceInfo;

describe('LibraryScreen', () => {
  beforeEach(() => {
    current = null;
    adbPlayers = [];
    cards = [];
  });

  it('shows the player and the music panel when the player is a drive', () => {
    current = card;
    render(<LibraryScreen />);
    expect(screen.getByRole('heading', { name: 'Music Library' })).toBeInTheDocument();
    expect(screen.getByText('player panel')).toBeInTheDocument();
    expect(screen.getByText('add music')).toBeInTheDocument();
  });

  it('explains ADB mode instead of saying no player was found', () => {
    adbPlayers = [adbPlayer];
    render(<LibraryScreen />);
    expect(
      screen.getByText(
        'Your player is connected in ADB mode. Put it in Storage mode to manage music and the library.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText('How to switch')).toBeInTheDocument();
    expect(screen.queryByText(/No player found/)).not.toBeInTheDocument();
    expect(screen.queryByText('add music')).not.toBeInTheDocument();
  });

  it('offers to find the player when nothing is connected', () => {
    render(<LibraryScreen />);
    expect(screen.getByRole('button', { name: 'Scan again' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Choose folder…' })).toBeInTheDocument();
    expect(
      screen.getByText('No player found. Connect your player, or choose its folder.'),
    ).toBeInTheDocument();
  });

  it('still finds a drive when a player is also seen over ADB', () => {
    adbPlayers = [adbPlayer];
    cards = [card];
    render(<LibraryScreen />);
    expect(screen.getByRole('button', { name: 'Use COMPAS' })).toBeInTheDocument();
    expect(screen.queryByText(/connected in ADB mode/)).not.toBeInTheDocument();
  });
});
