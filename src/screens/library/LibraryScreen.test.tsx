import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { buildArt, libraryStatus, scanLibrary } from '../../api/library';
import { card } from '../../test/fixtures';
import { LibraryScreen } from './LibraryScreen';

vi.mock('../../api/library');
vi.mock('../../state/CardContext', () => ({ useCard: () => ({ card }) }));

const status = {
  sdtool_version: '1.0.0',
  database_format: 5,
  generation: 12,
  songs: 4032,
  format_mismatch: false,
};

describe('LibraryScreen', () => {
  beforeEach(() => {
    vi.mocked(libraryStatus).mockResolvedValue(status);
  });

  it('shows the library status of the card', async () => {
    render(<LibraryScreen onChooseCard={vi.fn()} />);
    expect(await screen.findByText('4032 songs, library generation 12.')).toBeInTheDocument();
  });

  it('warns about a database layout mismatch and an empty library', async () => {
    vi.mocked(libraryStatus).mockResolvedValue({ ...status, generation: 0, format_mismatch: true });
    render(<LibraryScreen onChooseCard={vi.fn()} />);
    expect(await screen.findByText('No library on this card yet.')).toBeInTheDocument();
    expect(screen.getByText(/different library layout/)).toBeInTheDocument();
  });

  it('reports the counts of a scan, failures and the time shift', async () => {
    vi.mocked(scanLibrary).mockResolvedValue({
      files: 10,
      parsed: 8,
      unchanged: 0,
      failed: 2,
      skipped: 0,
      songs: 8,
      generation: 13,
      written: true,
      mtime_offset: 3600,
    });
    render(<LibraryScreen onChooseCard={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Update library' }));

    expect(await screen.findByText(/Library updated: 8 songs, generation 13/)).toBeInTheDocument();
    expect(screen.getByText('Failed').nextSibling).toHaveTextContent('2');
    expect(screen.getByText(/shifted by 3600 seconds/)).toBeInTheDocument();
    expect(scanLibrary).toHaveBeenCalledWith(false, expect.any(Function));
  });

  it('shows the friendly error when the tool fails, and builds album art', async () => {
    vi.mocked(scanLibrary).mockRejectedValue({
      code: 'sdtool',
      message: 'No music was found on the card.',
    });
    vi.mocked(buildArt).mockResolvedValue({
      board: 'r1',
      albums: 20,
      written: 18,
      unchanged: 0,
      no_art: 2,
      mtime_offset: 0,
    });
    render(<LibraryScreen onChooseCard={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Re-read all tags' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('No music was found on the card.');

    await userEvent.click(screen.getByRole('button', { name: 'Build album art' }));
    expect(await screen.findByText(/Album art checked for 20 albums/)).toBeInTheDocument();
    expect(screen.queryByText(/shifted by/)).not.toBeInTheDocument();
  });

  it('shows partial album art counts together with the warning', async () => {
    vi.mocked(buildArt).mockResolvedValue({
      board: 'r1',
      albums: 10,
      written: 8,
      unchanged: 1,
      no_art: 1,
      mtime_offset: 0,
      warning: 'Some album covers could not be written; the others were.',
    });
    render(<LibraryScreen onChooseCard={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Build album art' }));
    expect(await screen.findByText(/Album art checked for 10 albums/)).toBeInTheDocument();
    expect(screen.getByText(/Some album covers could not be written/)).toBeInTheDocument();
    expect(screen.getByText('Written').nextSibling).toHaveTextContent('8');
  });
});
