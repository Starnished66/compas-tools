import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { installPlugin } from '../../api/catalog';
import { card, entry } from '../../test/fixtures';
import { CatalogView } from './CatalogView';

const load = vi.fn(() => Promise.resolve());
const entries = [
  entry({ id: 'a', name: 'Alpha' }),
  entry({ id: 'b', name: 'Beta', state: 'update', installed_version: '0.9', version: '1.0' }),
  entry({ id: 'c', name: 'Gamma', state: 'incompatible', incompatible: true, api_min: 20 }),
  entry({ id: 'd', name: 'Delta', player_layout: true }),
];

vi.mock('../../api/catalog');
vi.mock('../../state/CardContext', () => ({ useCard: () => ({ card }) }));
vi.mock('../../state/CatalogContext', () => ({
  useCatalog: () => ({ entries, loading: false, error: null, load }),
}));

describe('CatalogView', () => {
  beforeEach(() => {
    vi.mocked(installPlugin).mockReset();
    load.mockClear();
  });

  it('shows only the entries of its view and filters by search', async () => {
    render(<CatalogView view="plugins" onChooseCard={vi.fn()} />);
    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.queryByText('Delta')).not.toBeInTheDocument();

    await userEvent.type(screen.getByRole('searchbox', { name: 'Search' }), 'beta');
    expect(screen.queryByText('Alpha')).not.toBeInTheDocument();
    expect(screen.getByText('Beta')).toBeInTheDocument();
  });

  it('counts updates and offers no install for incompatible plugins', () => {
    render(<CatalogView view="plugins" onChooseCard={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Update all (1)' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'Install Gamma' })).not.toBeInTheDocument();
    expect(screen.getByText('Needs a newer player')).toBeInTheDocument();
  });

  it('asks before replacing a local file, then retries with force', async () => {
    vi.mocked(installPlugin)
      .mockResolvedValueOnce('needs_confirmation')
      .mockResolvedValueOnce('done');
    render(<CatalogView view="plugins" onChooseCard={vi.fn()} />);

    await userEvent.click(screen.getByRole('button', { name: 'Install Alpha' }));
    expect(await screen.findByRole('dialog', { name: 'Replace a local file?' })).toBeVisible();
    expect(installPlugin).toHaveBeenLastCalledWith('a', false, expect.any(Function));

    await userEvent.click(screen.getByRole('button', { name: 'Replace' }));
    await waitFor(() => {
      expect(installPlugin).toHaveBeenLastCalledWith('a', true, expect.any(Function));
    });
  });

  it('shows the failure message when an install fails', async () => {
    vi.mocked(installPlugin).mockRejectedValueOnce({ code: 'network', message: 'No connection' });
    render(<CatalogView view="plugins" onChooseCard={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Install Alpha' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('No connection');
  });
});
