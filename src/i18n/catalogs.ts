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
