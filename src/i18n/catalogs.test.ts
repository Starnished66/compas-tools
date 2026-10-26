import { catalogs, type Language, type TranslationValue } from './catalogs';

const sources = import.meta.glob<string>('../**/*.{ts,tsx}', {
  query: '?raw',
  import: 'default',
  eager: true,
});

const TRANSLATED: Exclude<Language, 'en'>[] = ['es', 'de', 'fr', 'it', 'ru'];
const FORMS: Record<Exclude<Language, 'en'>, string[]> = {
  es: ['one', 'other'],
  de: ['one', 'other'],
  fr: ['one', 'other'],
  it: ['one', 'other'],
  ru: ['one', 'few', 'many', 'other'],
};

/** Every `t('…')` call in the UI code: the strings that must be translated. */
function usedStrings(): { keys: Set<string>; nonLiteral: string[] } {
  const keys = new Set<string>();
  const nonLiteral: string[] = [];
  const call =
    /(?<![\w.])t\(\s*(?:('(?:[^'\\]|\\.)*')|("(?:[^"\\]|\\.)*")|(`(?:[^`\\]|\\.)*`)|([^'"`\s)][^,)]*))/g;
  for (const [path, text] of Object.entries(sources)) {
    // `./` is this folder; tests and test helpers are not UI.
    if (path.startsWith('./') || /\.test\.tsx?$/.test(path) || path.includes('/test/')) continue;
    for (const match of text.matchAll(call)) {
      if (match[4]) nonLiteral.push(`${path}: ${match[4]}`);
      else {
        const raw = (match[1] ?? match[2] ?? match[3] ?? '').slice(1, -1);
        keys.add(raw.replace(/\\'/g, "'").replace(/\\"/g, '"').replace(/\\n/g, '\n'));
      }
    }
  }
  return { keys, nonLiteral };
}

const placeholders = (text: string) =>
  [...text.matchAll(/\{([a-zA-Z][a-zA-Z0-9_]*)\}/g)].map((m) => m[1] ?? '').sort();

const forms = (value: TranslationValue): string[] =>
  typeof value === 'string' ? [value] : Object.values(value).filter((v): v is string => !!v);

describe('translation catalogs', () => {
  const { keys, nonLiteral } = usedStrings();

  it('finds the UI strings, and none is built at run time (those cannot be checked)', () => {
    expect(keys.size).toBeGreaterThan(200);
    expect(nonLiteral).toEqual([]);
  });

  it.each(TRANSLATED)('%s has every UI string', (language) => {
    const missing = [...keys].filter((key) => !(key in catalogs[language]));
    expect(missing, `missing from ${language}`).toEqual([]);
  });

  it.each(TRANSLATED)('%s has nothing the UI no longer uses', (language) => {
    const unused = Object.keys(catalogs[language]).filter((key) => !keys.has(key));
    expect(unused, `unused in ${language}`).toEqual([]);
  });

  it('English only lists strings that exist', () => {
    expect(Object.keys(catalogs.en).filter((key) => !keys.has(key))).toEqual([]);
  });

  it.each(TRANSLATED)('%s keeps every {placeholder} and its plural forms', (language) => {
    const problems: string[] = [];
    for (const key of keys) {
      const translation = catalogs[language][key];
      const english = catalogs.en[key];
      if (translation === undefined) continue;
      if (typeof english === 'object') {
        const present = typeof translation === 'object' ? Object.keys(translation) : [];
        const needed = FORMS[language];
        if (!needed.every((form) => present.includes(form))) problems.push(`${key}: forms`);
      } else if (typeof translation !== 'string') {
        problems.push(`${key}: plural form without an English plural`);
      }
      for (const form of forms(translation)) {
        if (placeholders(form).join() !== placeholders(key).join()) {
          problems.push(`${key}: placeholders of "${form}"`);
        }
      }
    }
    expect(problems).toEqual([]);
  });
});
