import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { installPlugin, loadEntries, moveRecordAside } from '../../api/catalog';
import { CatalogProvider } from '../../state/CatalogContext';
import { ActionDialog } from './ActionDialog';
import { card, entry } from '../../test/fixtures';
import { CatalogView } from './CatalogView';

const entries = [
  entry({ id: 'a', name: 'Alpha' }),
  entry({ id: 'b', name: 'Beta', state: 'update', installed_version: '0.9', version: '1.0' }),
  entry({ id: 'c', name: 'Gamma', state: 'incompatible', incompatible: true, api_min: 20 }),
  entry({ id: 'd', name: 'Delta', player_layout: true }),
];

vi.mock('../../api/catalog');
const mockedCard = { ...card };
vi.mock('../../state/CardContext', () => ({ useCard: () => ({ card: mockedCard }) }));

function renderView(withView = true) {
  return render(
    <CatalogProvider>
      {withView && <CatalogView view="plugins" onChooseCard={vi.fn()} />}
      <ActionDialog />
    </CatalogProvider>,
  );
}

describe('CatalogView', () => {
  beforeEach(() => {
    mockedCard.device = card.device;
    mockedCard.root = card.root;
    vi.mocked(installPlugin).mockReset();
    vi.mocked(loadEntries).mockResolvedValue(entries);
  });

  it('shows only the entries of its view and filters by search', async () => {
    renderView();
    expect(await screen.findByText('Alpha')).toBeInTheDocument();
    expect(screen.queryByText('Delta')).not.toBeInTheDocument();

    await userEvent.type(screen.getByRole('searchbox', { name: 'Search' }), 'beta');
    expect(screen.queryByText('Alpha')).not.toBeInTheDocument();
    expect(screen.getByText('Beta')).toBeInTheDocument();
  });

  it('counts updates and offers no install for incompatible plugins', async () => {
    renderView();
    await screen.findByText('Alpha');
    expect(screen.getByRole('button', { name: 'Update all (1)' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'Install Gamma' })).not.toBeInTheDocument();
    expect(screen.getByText('Needs a newer player')).toBeInTheDocument();
  });

  it('asks before replacing a local file, then retries with force', async () => {
    vi.mocked(installPlugin)
      .mockResolvedValueOnce('needs_confirmation')
      .mockResolvedValueOnce('done');
    renderView();

    await userEvent.click(await screen.findByRole('button', { name: 'Install Alpha' }));
    expect(await screen.findByRole('dialog', { name: 'Replace a local file?' })).toBeVisible();
    expect(installPlugin).toHaveBeenLastCalledWith('a', false, expect.any(Function));

    await userEvent.click(screen.getByRole('button', { name: 'Replace' }));
    await waitFor(() => {
      expect(installPlugin).toHaveBeenLastCalledWith('a', true, expect.any(Function));
    });
  });

  it('shows the failure message when an install fails', async () => {
    vi.mocked(installPlugin).mockRejectedValueOnce({ code: 'network', message: 'No connection' });
    renderView();
    await userEvent.click(await screen.findByRole('button', { name: 'Install Alpha' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('No connection');
  });

  it('keeps the confirmation open when the view is switched away', async () => {
    vi.mocked(installPlugin).mockResolvedValueOnce('needs_confirmation');
    const { rerender } = renderView();
    await userEvent.click(await screen.findByRole('button', { name: 'Install Alpha' }));
    await screen.findByRole('dialog', { name: 'Replace a local file?' });

    rerender(
      <CatalogProvider>
        <ActionDialog />
      </CatalogProvider>,
    );
    expect(screen.getByRole('dialog', { name: 'Replace a local file?' })).toBeVisible();
  });

  it('says compatibility is unknown when the card has no device.json', async () => {
    mockedCard.device = null;
    renderView();
    expect(await screen.findByText(/Compatibility cannot be checked/)).toBeInTheDocument();
  });

  it('offers to move a damaged record aside and reloads', async () => {
    vi.mocked(loadEntries).mockRejectedValueOnce({ code: 'record_damaged', message: 'Damaged' });
    vi.mocked(moveRecordAside).mockResolvedValue(null);
    renderView();
    expect(await screen.findByRole('alert')).toHaveTextContent('Damaged');
    expect(screen.queryByText('Alpha')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Move the record aside' }));
    expect(moveRecordAside).toHaveBeenCalled();
    expect(await screen.findByText('Alpha')).toBeInTheDocument();
  });

  it('drops a pending confirmation when the card changes', async () => {
    vi.mocked(installPlugin).mockResolvedValueOnce('needs_confirmation');
    const { rerender } = renderView();
    await userEvent.click(await screen.findByRole('button', { name: 'Install Alpha' }));
    await screen.findByRole('dialog', { name: 'Replace a local file?' });

    mockedCard.root = '/media/other';
    rerender(
      <CatalogProvider>
        <CatalogView view="plugins" onChooseCard={vi.fn()} />
        <ActionDialog />
      </CatalogProvider>,
    );
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });
});
