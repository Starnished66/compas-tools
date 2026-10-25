import type { ArtDone, ScanDone } from '../../api/types';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';

function OffsetNote({ seconds }: { seconds: number }) {
  if (seconds === 0) return null;
  return (
    <p>
      {t("File times were shifted by {seconds} seconds to match the player's clock.", { seconds })}
    </p>
  );
}

export function ScanResult({ done }: { done: ScanDone }) {
  return (
    <Notice tone={done.failed > 0 ? 'warning' : 'success'}>
      <p>
        {done.written
          ? t('Library updated: {songs} songs, generation {generation}.', {
              songs: done.songs,
              generation: done.generation,
            })
          : t('The library was already up to date; the card was not changed.')}
      </p>
      <dl className="details-list">
        <dt>{t('Files found')}</dt>
        <dd>{done.files}</dd>
        <dt>{t('Read')}</dt>
        <dd>{done.parsed}</dd>
        <dt>{t('Unchanged')}</dt>
        <dd>{done.unchanged}</dd>
        <dt>{t('Skipped')}</dt>
        <dd>{done.skipped}</dd>
        <dt>{t('Failed')}</dt>
        <dd>{done.failed}</dd>
      </dl>
      {done.failed > 0 && (
        <p>{t('Files that failed have no entry in the library. Check that they play.')}</p>
      )}
      <OffsetNote seconds={done.mtime_offset} />
    </Notice>
  );
}

export function ArtResult({ done }: { done: ArtDone }) {
  return (
    <Notice tone={done.warning ? 'warning' : 'success'}>
      <p>{t('Album art checked for {albums} albums.', { albums: done.albums })}</p>
      <dl className="details-list">
        <dt>{t('Written')}</dt>
        <dd>{done.written}</dd>
        <dt>{t('Unchanged')}</dt>
        <dd>{done.unchanged}</dd>
        <dt>{t('No art found')}</dt>
        <dd>{done.no_art}</dd>
      </dl>
      {done.warning && <p>{done.warning}</p>}
      <OffsetNote seconds={done.mtime_offset} />
    </Notice>
  );
}
