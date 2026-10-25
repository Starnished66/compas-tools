import { render, screen } from '@testing-library/react';

import { MiddleEllipsis } from './MiddleEllipsis';

describe('MiddleEllipsis', () => {
  it('keeps the last folder and the file name whole, cut at a slash', () => {
    const path = '/home/someone/Music/Collections/Lossless/Artist/Album/01 Opening.flac';
    render(<MiddleEllipsis text={path} />);
    expect(screen.getByTitle(path)).toBeInTheDocument();
    expect(screen.getByText('/Album/01 Opening.flac')).toBeInTheDocument();
    expect(screen.getByText('/home/someone/Music/Collections/Lossless/Artist')).toBeInTheDocument();
  });

  it('falls back to a fixed ending when the last folder is too long', () => {
    const path = `/music/${'x'.repeat(80)}.flac`;
    render(<MiddleEllipsis text={path} />);
    expect(screen.getByText(path.slice(-24))).toBeInTheDocument();
  });
});
