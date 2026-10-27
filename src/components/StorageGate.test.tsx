import { render, screen } from '@testing-library/react';

import type { CardInfo, DeviceInfo } from '../api/types';
import { card } from '../test/fixtures';
import { StorageGate } from './StorageGate';

let current: CardInfo | null = null;
let adbPlayers: DeviceInfo[] = [];

vi.mock('../state/CardContext', () => ({ useCard: () => ({ card: current, select: vi.fn() }) }));
vi.mock('../state/ModeContext', () => ({
  useMode: () => ({ cards: [], adbPlayers, scanning: false, rescan: vi.fn() }),
}));

describe('StorageGate', () => {
  it('words the ADB-mode notice for plugins', () => {
    current = null;
    adbPlayers = [{ serial: 'x' } as DeviceInfo];
    render(<StorageGate need="plugins">content</StorageGate>);
    expect(
      screen.getByText(
        'Your player is connected in ADB mode. Put it in Storage mode to manage plugins.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText('content')).not.toBeInTheDocument();
  });

  it('lets the content through once a player is a drive', () => {
    current = card;
    render(<StorageGate need="plugins">content</StorageGate>);
    expect(screen.getByText('content')).toBeInTheDocument();
  });
});
