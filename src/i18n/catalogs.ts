import { de } from './catalogs/de';
import { es } from './catalogs/es';
import { fr } from './catalogs/fr';
import { it } from './catalogs/it';
import { ru } from './catalogs/ru';

export type Language = 'en' | 'es' | 'de' | 'fr' | 'it' | 'ru';

export type TranslationValue = string | Partial<Record<Intl.LDMLPluralRule, string>>;

/** English source strings are the keys; English only needs entries for plurals. */
export const catalogs: Record<Language, Record<string, TranslationValue>> = {
  en: {
    '{count} plugin': { one: '{count} plugin', other: '{count} plugins' },
    '{count} update installed': {
      one: '{count} update installed',
      other: '{count} updates installed',
    },
    '{count} files, {size}; {free} free on the player.': {
      one: '{count} file, {size}; {free} free on the player.',
      other: '{count} files, {size}; {free} free on the player.',
    },
    '{count} already on the player. Identical files are skipped; different ones are replaced.': {
      one: '{count} is already on the player. An identical file is skipped; a different one is replaced.',
      other:
        '{count} already on the player. Identical files are skipped; different ones are replaced.',
    },
    '{count} hidden files, macOS leftovers and firmware images are left out.': {
      one: '{count} hidden file, macOS leftover or firmware image is left out.',
      other: '{count} hidden files, macOS leftovers and firmware images are left out.',
    },
    'Copied {count} files ({replaced} replaced), skipped {skipped} identical.': {
      one: 'Copied {count} file ({replaced} replaced), skipped {skipped} identical.',
      other: 'Copied {count} files ({replaced} replaced), skipped {skipped} identical.',
    },
    'Removed {count} files that were not in the folder.': {
      one: 'Removed {count} file that was not in the folder.',
      other: 'Removed {count} files that were not in the folder.',
    },
    'Cleaned up {count} macOS leftovers.': {
      one: 'Cleaned up {count} macOS leftover.',
      other: 'Cleaned up {count} macOS leftovers.',
    },
    '{count} files on the player are not in the folder and will be deleted.': {
      one: '{count} file on the player is not in the folder and will be deleted.',
      other: '{count} files on the player are not in the folder and will be deleted.',
    },
    '{count} songs, library generation {generation}.': {
      one: '{count} song, library generation {generation}.',
      other: '{count} songs, library generation {generation}.',
    },
    'Library updated: {count} songs, generation {generation}.': {
      one: 'Library updated: {count} song, generation {generation}.',
      other: 'Library updated: {count} songs, generation {generation}.',
    },
    'Album art checked for {count} albums.': {
      one: 'Album art checked for {count} album.',
      other: 'Album art checked for {count} albums.',
    },
    '{count} names change because FAT does not allow them:': {
      one: '{count} name changes because FAT does not allow it:',
      other: '{count} names change because FAT does not allow them:',
    },
    '{count} files will not be copied:': {
      one: '{count} file will not be copied:',
      other: '{count} files will not be copied:',
    },
    '{count} files on the player are not in the folder and will be removed:': {
      one: '{count} file on the player is not in the folder and will be removed:',
      other: '{count} files on the player are not in the folder and will be removed:',
    },
    '{count} files failed:': { one: '{count} file failed:', other: '{count} files failed:' },
    '{count} files were not on this player and are left out.': {
      one: '{count} file was not on this player and is left out.',
      other: '{count} files were not on this player and are left out.',
    },
    '{count} files copied, {skipped} left out on purpose.': {
      one: '{count} file copied, {skipped} left out on purpose.',
      other: '{count} files copied, {skipped} left out on purpose.',
    },
  },
  es,
  de,
  fr,
  it,
  ru,
};
