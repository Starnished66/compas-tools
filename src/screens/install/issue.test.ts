import { issueUrl } from './issue';

describe('issueUrl', () => {
  it('pre-fills a new issue on the player repository and uploads nothing', () => {
    const url = new URL(issueUrl('Problem with my player', 'Board: r1\nFirmware: Compas'));
    expect(url.origin + url.pathname).toBe(
      'https://github.com/Starnished66/compas-player/issues/new',
    );
    expect(url.searchParams.get('title')).toBe('Problem with my player');
    const body = url.searchParams.get('body') ?? '';
    expect(body).toContain('Board: r1');
    expect(body).toContain('attached to this issue');
  });
});
