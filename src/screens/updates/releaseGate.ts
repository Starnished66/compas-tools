import type { LatestRelease, Model } from '../../api/types';
import { t } from '../../i18n';

/** What the latest release means for installing it on one player. */
export interface ReleaseGate {
  /** Installing is the expected next step: an update is available or the version is unknown. */
  recommended: boolean;
  /** The player is newer than the release, so installing goes back to an older version. */
  downgrade: boolean;
  /** The release ships a player-only file for this model. Unknown until the release is loaded. */
  playerOnlyAvailable: boolean;
}

export function releaseGate(
  latest: LatestRelease | null | undefined,
  model: Model | null,
): ReleaseGate {
  const freshness = latest?.freshness;
  return {
    recommended: freshness !== 'newer' && freshness !== 'up_to_date',
    downgrade: freshness === 'newer',
    playerOnlyAvailable: !latest || !model || latest.release.player_only_boards.includes(model),
  };
}

export const firmwareOnlyText = () =>
  t('This release is a full firmware update only. Use the full firmware option instead.');

export function downgradeText(installed: string | null, latest: LatestRelease): string {
  return t(
    'Your player has {version}, newer than the latest release {release}. Installing the release goes back to the older version.',
    { version: installed ?? '', release: latest.release.tag },
  );
}
