import type { LibrarySupport } from '../../api/types';
import { Notice } from '../../components/Notice';
import { t } from '../../i18n';

interface Props {
  support: Exclude<LibrarySupport, 'ready'>;
}

/** Why Compás Tools leaves the library to the player for this card. */
export function LibraryNotReady({ support }: Props) {
  return (
    <Notice tone="warning">
      <p>
        {support === 'player_newer'
          ? t(
              'Your player is newer than this version of Compás Tools. Update Compás Tools so it can build the library on this computer; until then the player builds it on its next start.',
            )
          : t(
              'The library will be built by the player on its next start. Updating the player (Updates screen) lets Compás Tools build it on this computer.',
            )}
      </p>
    </Notice>
  );
}
