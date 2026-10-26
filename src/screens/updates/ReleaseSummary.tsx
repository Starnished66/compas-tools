import { openUrl } from '@tauri-apps/plugin-opener';

import type { Freshness, LatestRelease } from '../../api/types';
import { Button } from '../../components/Button';
import { Notice } from '../../components/Notice';
import { formatDate } from '../../format';
import { t } from '../../i18n';

interface Props {
  latest: LatestRelease;
  installedVersion: string | null;
}

function freshnessText(freshness: Freshness, installed: string | null, tag: string): string {
  switch (freshness) {
    case 'up_to_date':
      return t('Your player is up to date ({version}).', { version: installed ?? '' });
    case 'update_available':
      // No version is known: the player predates device.json, so it is older than this release.
      if (!installed)
        return t('This player is older than {version}; an update is available.', { version: tag });
      return t('Your player has {version}; an update is available.', { version: installed });
    case 'newer':
      return t('Your player has {version}, newer than the latest release.', {
        version: installed ?? '',
      });
    case 'unknown':
      return t('The player version is not known yet.');
  }
}

export function ReleaseSummary({ latest, installedVersion }: Props) {
  const { release, freshness } = latest;
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
      <Notice tone={freshness === 'update_available' ? 'warning' : 'info'}>
        <p>{freshnessText(freshness, installedVersion, release.tag)}</p>
      </Notice>
      {release.notes && (
        <details>
          <summary>{t('Release notes')}</summary>
          <pre className="release-notes">{release.notes}</pre>
        </details>
      )}
      <div>
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
