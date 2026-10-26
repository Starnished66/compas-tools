import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { open } from '@tauri-apps/plugin-dialog';

import { copyTransfer, planTransfer, updateLibrary } from '../../api/music';
import type { CopyView, LibraryUpdate, PlanView } from '../../api/types';
import { card } from '../../test/fixtures';
import { MusicProvider } from '../../state/MusicContext';
import { MusicScreen } from './MusicScreen';

function renderScreen() {
  return render(
    <MusicProvider>
      <MusicScreen onChooseCard={vi.fn()} />
    </MusicProvider>,
  );
}

vi.mock('../../api/music');
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('../../hooks/useFileDrop', () => ({ useFileDrop: () => false }));
vi.mock('../../state/CardContext', () => ({ useCard: () => ({ card }) }));

const plan: PlanView = {
  files: 3,
  total_bytes: 3 * 1024 * 1024,
  existing: 1,
  deletions: [],
  issues: [{ source: '/music/huge.flac', kind: 'too_large_for_fat' }],
  renames: [{ from: 'AC:DC.flac', to: 'AC_DC.flac' }],
  skipped_clutter: 2,
  deletions_blocked: false,
  free_bytes: 10 * 1024 * 1024,
  max_file_bytes: 4294967295,
};

const update: LibraryUpdate = {
  scan: {
    files: 3,
    parsed: 2,
    unchanged: 0,
    failed: 1,
    skipped: 0,
    songs: 2,
    generation: 4,
    written: true,
    mtime_offset: 3600,
  },
  art: { board: 'r1', albums: 2, written: 2, unchanged: 0, no_art: 0, mtime_offset: 3600 },
  art_error: null,
};

const result: CopyView = {
  copied: 2,
  replaced: 0,
  skipped_identical: 1,
  deleted: 0,
  failed: [{ path: '/music/bad.flac', message: 'disk error' }],
  tidy: { removed: 4, marker_written: true },
  library: { status: 'updated', ...update },
};

describe('MusicScreen', () => {
  beforeEach(() => {
    vi.mocked(planTransfer).mockResolvedValue(plan);
    vi.mocked(copyTransfer).mockReset();
    vi.mocked(updateLibrary).mockReset();
    vi.mocked(copyTransfer).mockResolvedValue(result);
    vi.mocked(open).mockResolvedValue(['/music/Album']);
  });

  it('keeps the sources and options when the screen is left and opened again', async () => {
    const { rerender } = renderScreen();
    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    await userEvent.type(screen.getByPlaceholderText('For example: Music'), 'Rock');
    await userEvent.click(screen.getByRole('checkbox'));

    rerender(<MusicProvider>{null}</MusicProvider>);
    rerender(
      <MusicProvider>
        <MusicScreen onChooseCard={vi.fn()} />
      </MusicProvider>,
    );
    expect(screen.getByTitle('/music/Album')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('For example: Music')).toHaveValue('Rock');
    expect(screen.getByRole('checkbox')).toBeChecked();
  });

  it('previews the plan for the chosen folder and shows what is left out', async () => {
    renderScreen();
    expect(screen.getByRole('button', { name: 'Copy to card' })).toBeDisabled();

    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    expect(await screen.findByTitle('/music/Album')).toBeInTheDocument();
    expect(await screen.findByText(/3 files, 3\.0 MB/)).toBeInTheDocument();
    expect(screen.getByText(/Too big for a FAT32 card/)).toBeInTheDocument();
    expect(screen.getByText(/AC:DC.flac → AC_DC.flac/)).toBeInTheDocument();
    expect(screen.getByText(/2 hidden files, macOS leftovers/)).toBeInTheDocument();
  });

  it('explains why a mirror removes nothing when a source could not be read', async () => {
    vi.mocked(planTransfer).mockResolvedValue({ ...plan, deletions_blocked: true });
    renderScreen();
    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    expect(await screen.findByText(/Nothing will be removed from the card/)).toBeInTheDocument();
  });

  it('copies and reports the result with eject advice', async () => {
    renderScreen();
    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    const copy = screen.getByRole('button', { name: 'Copy to card' });
    await waitFor(() => {
      expect(copy).toBeEnabled();
    });
    await userEvent.click(copy);

    expect(await screen.findByText(/Copied 2 files/)).toBeInTheDocument();
    expect(copyTransfer).toHaveBeenCalledWith(
      { sources: ['/music/Album'], destDir: '', deleteExtra: false },
      expect.any(Function),
    );
    expect(screen.getByText(/bad.flac: disk error/)).toBeInTheDocument();
    expect(screen.getByText(/Cleaned up 4 macOS leftovers/)).toBeInTheDocument();
    expect(screen.getByText(/eject it in your system/)).toBeInTheDocument();
  });

  async function copyOnce() {
    renderScreen();
    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    const copy = screen.getByRole('button', { name: 'Copy to card' });
    await waitFor(() => {
      expect(copy).toBeEnabled();
    });
    await userEvent.click(copy);
  }

  it('shows the library update with its counts after the copy, and only then the eject advice', async () => {
    await copyOnce();
    expect(await screen.findByText(/Library updated: 2 songs, generation 4/)).toBeInTheDocument();
    expect(screen.getByText('Failed').nextSibling).toHaveTextContent('1');
    expect(screen.getAllByText(/shifted by 3600 seconds/)).toHaveLength(2);
    expect(screen.getByText(/Album art checked for 2 albums/)).toBeInTheDocument();
    expect(screen.getByText(/eject it in your system/)).toBeInTheDocument();
    expect(updateLibrary).not.toHaveBeenCalled();
  });

  it('says nothing changed when the library was not updated', async () => {
    vi.mocked(copyTransfer).mockResolvedValue({
      ...result,
      copied: 0,
      skipped_identical: 3,
      failed: [],
      library: { status: 'skipped' },
    });
    await copyOnce();
    expect(await screen.findByText(/Nothing changed on the card/)).toBeInTheDocument();
    expect(screen.queryByText(/Library updated/)).not.toBeInTheDocument();
  });

  it("explains that the player builds the library when its format is not the tool's", async () => {
    vi.mocked(copyTransfer).mockResolvedValue({
      ...result,
      failed: [],
      library: { status: 'unsupported' },
    });
    await copyOnce();
    expect(await screen.findByText(/built by the player on its next start/)).toBeInTheDocument();
    expect(screen.getByText(/Copied 2 files/)).toBeInTheDocument();
    expect(screen.queryByText(/Library updated/)).not.toBeInTheDocument();
  });

  it('keeps the copy when the library update fails and retries only the update', async () => {
    vi.mocked(copyTransfer).mockResolvedValue({
      ...result,
      library: { status: 'failed', message: 'The library tool could not be started.' },
    });
    vi.mocked(updateLibrary).mockResolvedValue(update);
    await copyOnce();

    expect(await screen.findByRole('alert')).toHaveTextContent('could not be started');
    expect(screen.getByText(/Copied 2 files/)).toBeInTheDocument();
    expect(screen.queryByText(/eject it in your system/)).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByText(/Library updated: 2 songs/)).toBeInTheDocument();
    expect(updateLibrary).toHaveBeenCalledTimes(1);
    expect(copyTransfer).toHaveBeenCalledTimes(1);
    expect(screen.getByText(/eject it in your system/)).toBeInTheDocument();
  });

  it('asks before a mirror removes files from the card', async () => {
    vi.mocked(planTransfer).mockResolvedValue({ ...plan, deletions: ['Album/old.flac'] });
    renderScreen();
    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    await userEvent.click(screen.getByRole('checkbox'));
    const copy = screen.getByRole('button', { name: 'Copy to card' });
    await waitFor(() => {
      expect(copy).toBeEnabled();
    });
    await userEvent.click(copy);

    expect(
      await screen.findByRole('dialog', { name: 'Remove files from the card?' }),
    ).toBeVisible();
    expect(copyTransfer).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Copy and remove' }));
    await waitFor(() => {
      expect(copyTransfer).toHaveBeenCalledWith(
        expect.objectContaining({ deleteExtra: true }),
        expect.any(Function),
      );
    });
  });
});
