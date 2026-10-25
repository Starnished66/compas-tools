// Shared verbatim with compas-app (src/i18n/translate.ts); change both together.
import { catalogs, Language, TranslationValue } from './catalogs';

export type TranslationVariables = Record<string, string | number> & { count?: number };

export function normalizeLanguageTag(tag?: string | null): Language {
  const language = tag?.trim().replace('_', '-').split('-')[0]?.toLowerCase();
  return language === 'es' ||
    language === 'de' ||
    language === 'fr' ||
    language === 'it' ||
    language === 'ru'
    ? language
    : 'en';
}

export function resolveLanguage(
  preference: Language | 'system',
  systemLocale?: string | null,
): Language {
  return preference === 'system' ? normalizeLanguageTag(systemLocale) : preference;
}

/**
 * Return the small subset of CLDR cardinal rules needed by our six supported
 * locales. Hermes does not consistently ship Intl.PluralRules, so keep this
 * selector independent of runtime Intl polyfills.
 */
function pluralCategory(language: Language, count: number): 'one' | 'few' | 'many' | 'other' {
  if (!Number.isFinite(count)) return 'other';
  const absolute = Math.abs(count);

  if (
    (language === 'es' || language === 'fr' || language === 'it') &&
    absolute !== 0 &&
    Number.isInteger(absolute) &&
    absolute % 1_000_000 === 0
  ) {
    return 'many';
  }
  if (language === 'fr') return Math.floor(absolute) <= 1 ? 'one' : 'other';
  if (language === 'ru') {
    // Russian uses the integer part only when there are no visible fraction digits.
    if (!Number.isInteger(absolute)) return 'other';
    const mod10 = absolute % 10;
    const mod100 = absolute % 100;
    if (mod10 === 1 && mod100 !== 11) return 'one';
    if (mod10 >= 2 && mod10 <= 4 && !(mod100 >= 12 && mod100 <= 14)) return 'few';
    if (mod10 === 0 || (mod10 >= 5 && mod10 <= 9) || (mod100 >= 11 && mod100 <= 14)) return 'many';
    return 'other';
  }

  return absolute === 1 ? 'one' : 'other';
}

function selectPlural(
  value: TranslationValue,
  language: Language,
  count?: number,
): string | undefined {
  if (typeof value === 'string') return value;
  if (count == null) return value.other;
  const rule = pluralCategory(language, count);
  return (
    value[rule] ??
    value.other ??
    value.one ??
    Object.values(value).find((text): text is string => Boolean(text))
  );
}

/** Translate an explicit static UI source string. Unknown source strings stay readable in English. */
export function translate(
  source: string,
  language: Language = 'en',
  variables: TranslationVariables = {},
): string {
  const template =
    selectPlural(catalogs[language][source] ?? source, language, variables.count) ?? source;
  return template.replace(/\{([a-zA-Z][a-zA-Z0-9_]*)\}/g, (match, name: string) => {
    const replacement = variables[name];
    return replacement == null ? match : String(replacement);
  });
}
