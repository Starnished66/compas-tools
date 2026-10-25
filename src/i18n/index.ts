import { useSyncExternalStore } from 'react';

import type { Language } from './catalogs';
import { resolveLanguage, translate, type TranslationVariables } from './translate';

export type LanguagePreference = Language | 'system';

const STORAGE_KEY = 'compas_tools.language';
const LANGUAGES: readonly Language[] = ['en', 'es', 'de', 'fr', 'it', 'ru'];

function readPreference(): LanguagePreference {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return LANGUAGES.includes(stored as Language) ? (stored as Language) : 'system';
  } catch {
    return 'system';
  }
}

let preference = readPreference();
const listeners = new Set<() => void>();

function current(): Language {
  return resolveLanguage(preference, navigator.language);
}

/** Translates a static English source string. Every UI string goes through here. */
export function t(source: string, variables?: TranslationVariables): string {
  return translate(source, current(), variables);
}

export function setLanguagePreference(next: LanguagePreference): void {
  preference = next;
  try {
    if (next === 'system') localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // The choice then lasts until the app closes.
  }
  document.documentElement.lang = current();
  listeners.forEach((listener) => {
    listener();
  });
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The language in use and the user's choice; re-renders its component when the choice changes. */
export function useLanguage(): { language: Language; preference: LanguagePreference } {
  useSyncExternalStore(subscribe, () => preference);
  return { language: current(), preference };
}
