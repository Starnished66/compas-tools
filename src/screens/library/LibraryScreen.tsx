import { StorageGate } from '../../components/StorageGate';
import { t } from '../../i18n';
import { AddMusic } from './AddMusic';
import { PlayerPanel } from './PlayerPanel';

/** The player's music and library, managed through its files (Storage mode). */
export function LibraryScreen() {
  return (
    <section className="screen">
      <h1>{t('Music Library')}</h1>
      <p className="muted">
        {t(
          'Adds music and builds the music database and album art on this computer, so the player does not have to scan on its next start.',
        )}
      </p>
      <StorageGate need="music">
        <PlayerPanel />
        <AddMusic />
      </StorageGate>
    </section>
  );
}
