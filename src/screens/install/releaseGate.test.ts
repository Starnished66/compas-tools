import type { LatestRelease } from '../../api/types';
import { downgradeText, releaseGate } from './releaseGate';

function latest(freshness: LatestRelease['freshness'], boards: string[]): LatestRelease {
  return {
    freshness,
    release: {
      tag: 'v1.3.0',
      name: 'Compás',
      notes: '',
      published_at: '',
      url: '',
      player_only_boards: boards,
      full_image_boards: ['r1', 'r3proii', 'r3ii_2025', 'tempotec_v1'],
    },
  };
}

describe('releaseGate', () => {
  it('recommends installing when an update is available or the version is unknown', () => {
    expect(releaseGate(latest('update_available', ['r1']), 'r1').recommended).toBe(true);
    expect(releaseGate(latest('unknown', ['r1']), 'r1').recommended).toBe(true);
  });

  it('does not recommend reinstalling the same version', () => {
    const gate = releaseGate(latest('up_to_date', ['r1']), 'r1');
    expect(gate.recommended).toBe(false);
    expect(gate.downgrade).toBe(false);
  });

  it('flags a downgrade when the player is newer than the release', () => {
    const gate = releaseGate(latest('newer', ['r1']), 'r1');
    expect(gate.recommended).toBe(false);
    expect(gate.downgrade).toBe(true);
    expect(downgradeText('v1.3.1', latest('newer', []))).toBe(
      'Your player has v1.3.1, newer than the latest release v1.3.0. Installing the release goes back to the older version.',
    );
  });

  it('knows when a release has no player-only file for the model', () => {
    expect(releaseGate(latest('update_available', []), 'r1').playerOnlyAvailable).toBe(false);
    expect(releaseGate(latest('update_available', ['r3proii']), 'r1').playerOnlyAvailable).toBe(
      false,
    );
    expect(releaseGate(latest('update_available', ['r1']), 'r1').playerOnlyAvailable).toBe(true);
    expect(releaseGate(null, 'r1').playerOnlyAvailable).toBe(true);
    // A firmware-only release leaves the V1 with its full image only.
    expect(releaseGate(latest('update_available', []), 'tempotec_v1').playerOnlyAvailable).toBe(
      false,
    );
    expect(
      releaseGate(latest('update_available', ['tempotec_v1']), 'tempotec_v1').playerOnlyAvailable,
    ).toBe(true);
  });

  it('knows when a release has no firmware image for the model', () => {
    const release = latest('update_available', []);
    release.release.full_image_boards = ['r1'];
    expect(releaseGate(release, 'r1').fullImageAvailable).toBe(true);
    expect(releaseGate(release, 'tempotec_v1').fullImageAvailable).toBe(false);
    expect(releaseGate(null, 'tempotec_v1').fullImageAvailable).toBe(true);
  });
});
