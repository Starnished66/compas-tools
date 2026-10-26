import { releaseFileName } from './labels';

describe('releaseFileName', () => {
  it('names the file each install puts on the card, like the release assets', () => {
    expect(releaseFileName('full_image', 'r1')).toBe('r1.upt');
    expect(releaseFileName('full_image', 'r3ii_2025')).toBe('r3ii_2025.upt');
    expect(releaseFileName('full_image', 'tempotec_v1')).toBe('v1.upt');
    expect(releaseFileName('player_only', 'tempotec_v1')).toBe('compas_player_tempotec_v1');
    expect(releaseFileName('player_only', 'r1')).toBe('compas_player');
    expect(releaseFileName('player_only', 'r3proii')).toBe('compas_player_r3proii');
  });
});
