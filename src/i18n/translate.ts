// Shared between Compás App and Compás Tools; keep translation rules in sync.
import { catalogs, Language, TranslationValue } from './catalogs';

export type TranslationVariables = Record<string, string | number> & { count?: number };

export function normalizeLanguageTag(tag?: string | null): Language {
  const normalized = tag?.trim().replaceAll('_', '-').toLowerCase();
  if (!normalized) return 'en';
  const parts = normalized.split('-');
  const language = parts[0];
  if (language === 'pt') return parts.slice(1).includes('pt') ? 'pt_PT' : 'pt_BR';
  if (language === 'zh') return 'zh_CN';
  return (['es', 'de', 'fr', 'it', 'ru', 'fi', 'ja', 'nl', 'uk', 'vi'].includes(language ?? '')
    ? language
    : 'en') as Language;
}

export function resolveLanguage(
  preference: Language | 'system',
  systemLocale?: string | null,
): Language {
  return preference === 'system' ? normalizeLanguageTag(systemLocale) : preference;
}

/**
 * Return the subset of CLDR cardinal rules needed by our supported
 * locales, without depending on Intl.PluralRules.
 */
export function pluralCategory(language: Language, count: number): 'one' | 'few' | 'many' | 'other' {
  if (!Number.isFinite(count)) return 'other';
  const absolute = Math.abs(count);

  if (['es', 'fr', 'it', 'pt_BR', 'pt_PT'].includes(language) &&
      absolute !== 0 && Number.isInteger(absolute) && absolute % 1_000_000 === 0) return 'many';
  if (language === 'fr' || language === 'pt_BR') return Math.floor(absolute) <= 1 ? 'one' : 'other';
  if (language === 'ru' || language === 'uk') {
    if (!Number.isInteger(absolute)) return 'other';
    const mod10 = absolute % 10;
    const mod100 = absolute % 100;
    if (mod10 === 1 && mod100 !== 11) return 'one';
    if (mod10 >= 2 && mod10 <= 4 && !(mod100 >= 12 && mod100 <= 14)) return 'few';
    if (mod10 === 0 || (mod10 >= 5 && mod10 <= 9) || (mod100 >= 11 && mod100 <= 14)) return 'many';
    return 'other';
  }
  if (language === 'ja' || language === 'zh_CN' || language === 'vi') return 'other';

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
