export type Language = 'en' | 'es' | 'de' | 'fr' | 'it' | 'ru';

export type TranslationValue = string | Partial<Record<Intl.LDMLPluralRule, string>>;

/** English source strings are the keys; only plurals need an entry. Other languages come in phase 4. */
export const catalogs: Record<Language, Record<string, TranslationValue>> = {
  en: {
    '{count} plugin': { one: '{count} plugin', other: '{count} plugins' },
    '{count} file': { one: '{count} file', other: '{count} files' },
    '{count} update installed': {
      one: '{count} update installed',
      other: '{count} updates installed',
    },
    '{count} files, {size}; {free} free on the card.': {
      one: '{count} file, {size}; {free} free on the card.',
      other: '{count} files, {size}; {free} free on the card.',
    },
    '{count} already on the card. Identical files are skipped; different ones are replaced.': {
      one: '{count} is already on the card. An identical file is skipped; a different one is replaced.',
      other:
        '{count} already on the card. Identical files are skipped; different ones are replaced.',
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
    '{count} files on the card are not in the folder and will be deleted.': {
      one: '{count} file on the card is not in the folder and will be deleted.',
      other: '{count} files on the card are not in the folder and will be deleted.',
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
    '{count} files on the card are not in the folder and will be removed:': {
      one: '{count} file on the card is not in the folder and will be removed:',
      other: '{count} files on the card are not in the folder and will be removed:',
    },
    '{count} files failed:': { one: '{count} file failed:', other: '{count} files failed:' },
    '{count} other .upt file on the card': {
      one: '{count} other .upt file on the card',
      other: '{count} other .upt files on the card',
    },
  },
  es: {},
  de: {},
  fr: {},
  it: {},
  ru: {},
};
