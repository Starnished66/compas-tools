import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { open } from '@tauri-apps/plugin-dialog';

import { copyTransfer, planTransfer } from '../../api/music';
import type { CopyView, PlanView } from '../../api/types';
import { card } from '../../test/fixtures';
import { MusicScreen } from './MusicScreen';

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
  free_bytes: 10 * 1024 * 1024,
  max_file_bytes: 4294967295,
};

const result: CopyView = {
  copied: 2,
  replaced: 0,
  skipped_identical: 1,
  deleted: 0,
  failed: [{ path: '/music/bad.flac', message: 'disk error' }],
  tidy: { removed: 4, marker_written: true },
};

describe('MusicScreen', () => {
  beforeEach(() => {
    vi.mocked(planTransfer).mockResolvedValue(plan);
    vi.mocked(copyTransfer).mockResolvedValue(result);
    vi.mocked(open).mockResolvedValue(['/music/Album']);
  });

  it('previews the plan for the chosen folder and shows what is left out', async () => {
    render(<MusicScreen onChooseCard={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Copy to card' })).toBeDisabled();

    await userEvent.click(screen.getByRole('button', { name: 'Add folders…' }));
    expect(await screen.findByText('/music/Album')).toBeInTheDocument();
    expect(await screen.findByText(/3 files, 3\.0 MB/)).toBeInTheDocument();
    expect(screen.getByText(/Too big for a FAT32 card/)).toBeInTheDocument();
    expect(screen.getByText(/AC:DC.flac → AC_DC.flac/)).toBeInTheDocument();
    expect(screen.getByText(/2 macOS leftover files/)).toBeInTheDocument();
  });

  it('copies and reports the result with eject advice', async () => {
    render(<MusicScreen onChooseCard={vi.fn()} />);
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

  it('asks before a mirror removes files from the card', async () => {
    vi.mocked(planTransfer).mockResolvedValue({ ...plan, deletions: ['Album/old.flac'] });
    render(<MusicScreen onChooseCard={vi.fn()} />);
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
