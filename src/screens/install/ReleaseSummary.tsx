import { openUrl } from '@tauri-apps/plugin-opener';

import type { LatestRelease } from '../../api/types';
import { Button } from '../../components/Button';
import { Notice } from '../../components/Notice';
import { formatDate } from '../../format';
import { t } from '../../i18n';

interface Props {
  latest: LatestRelease;
  /** A player is connected over ADB, so its version (or its absence) can be compared. */
  hasPlayer: boolean;
  /** The version read from the player over ADB. */
  installedVersion: string | null;
  onCheckAgain: () => void;
}

function freshnessText(latest: LatestRelease, installed: string | null): string {
  const version = installed ?? '';
  switch (latest.freshness) {
    case 'up_to_date':
      return t('Your player is up to date ({version}).', { version });
    case 'update_available':
      return t('Your player has {version}; an update is available.', { version });
    case 'newer':
      return t('Your player has {version}, newer than the latest release.', { version });
    case 'unknown':
      return t('The player version is not known yet.');
  }
}

/** Which kinds of update the release offers: a firmware-only release ships no player file. */
export function releaseKindText(latest: LatestRelease): string {
  return latest.release.player_only_boards.length > 0
    ? t('This release can update the whole firmware, or just the player.')
    : t('This release is a full firmware update only.');
}

export function ReleaseSummary({ latest, hasPlayer, installedVersion, onCheckAgain }: Props) {
  const { release } = latest;
  const date = formatDate(release.published_at);
  return (
    <section className="panel stack" aria-label={t('Latest release')}>
      <div className="row">
        <h2>{release.name}</h2>
        <span className="muted">
          {release.tag}
          {date && ` · ${date}`}
        </span>
      </div>
      {hasPlayer && (
        <Notice tone={latest.freshness === 'update_available' ? 'warning' : 'info'}>
          <p>{freshnessText(latest, installedVersion)}</p>
        </Notice>
      )}
      <p>{releaseKindText(latest)}</p>
      {release.notes && (
        <details>
          <summary>{t('Release notes')}</summary>
          <pre className="release-notes">{release.notes}</pre>
        </details>
      )}
      <div className="row">
        <Button icon="refresh" onClick={onCheckAgain}>
          {t('Check again')}
        </Button>
        <Button
          onClick={() => {
            void openUrl(release.url);
          }}
        >
          {t('View on GitHub')}
        </Button>
      </div>
    </section>
  );
}
