import { resolveLanguage, translate, type TranslationVariables } from './translate';

const language = resolveLanguage('system', navigator.language);

/** Translates a static English source string. Every UI string goes through here. */
export function t(source: string, variables?: TranslationVariables): string {
  return translate(source, language, variables);
}
