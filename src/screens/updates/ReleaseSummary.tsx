import { openUrl } from '@tauri-apps/plugin-opener';

import type { LatestRelease } from '../../api/types';
import { Button } from '../../components/Button';
import { formatDate } from '../../format';
import { t } from '../../i18n';

interface Props {
  latest: LatestRelease;
}

/** Which kinds of update the release offers: a firmware-only release ships no player file. */
export function releaseKindText(latest: LatestRelease): string {
  return latest.release.player_only_boards.length > 0
    ? t('This release can update the whole firmware, or just the player.')
    : t('This release is a full firmware update only.');
}

export function ReleaseSummary({ latest }: Props) {
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
      <p>{releaseKindText(latest)}</p>
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
