#!/usr/bin/env node
// Validate the tools catalogs and literal translation calls without a test runner.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const ts = require('typescript');

const root = path.resolve(__dirname, '..');
const src = path.join(root, 'src');
const expected = [
  'en',
  'es',
  'de',
  'fr',
  'it',
  'ru',
  'fi',
  'ja',
  'nl',
  'pt_BR',
  'pt_PT',
  'uk',
  'vi',
  'zh_CN',
];
const errors = [];
const moduleCache = new Map();
function loadTs(file, cache = new Map()) {
  file = path.resolve(file);
  if (cache.has(file)) return cache.get(file).exports;
  let source;
  try {
    source = fs.readFileSync(file, 'utf8');
  } catch {
    errors.push(`Missing catalog module: ${path.relative(root, file)}`);
    const stem = path.basename(file, '.ts');
    const exportName = stem;
    return { [exportName]: {} };
  }
  const js = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  const mod = { exports: {} };
  cache.set(file, mod);
  const localRequire = (id) => {
    if (id.startsWith('.')) {
      let p = path.resolve(path.dirname(file), id);
      if (!path.extname(p)) p += '.ts';
      return loadTs(p, cache);
    }
    return require(id);
  };
  vm.runInNewContext(`(function(require,module,exports){${js}\n})`, {
    Intl: undefined,
    Object,
    Array,
    Number,
    String,
    Math,
    RegExp,
    Set,
    Map,
    console,
  })(localRequire, mod, mod.exports);
  return mod.exports;
}
function walk(dir) {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .flatMap((d) =>
      d.isDirectory()
        ? walk(path.join(dir, d.name))
        : /\.(tsx?|jsx?)$/.test(d.name)
          ? [path.join(dir, d.name)]
          : [],
    );
}
const { catalogs } = loadTs(path.join(src, 'i18n/catalogs.ts'), moduleCache);
const { translate, normalizeLanguageTag } = loadTs(
  path.join(src, 'i18n/translate.ts'),
  moduleCache,
);
if (Object.keys(catalogs).sort().join() !== expected.slice().sort().join())
  errors.push(`Locale set mismatch: ${Object.keys(catalogs).join(', ')}`);
const placeholders = (s) =>
  [...String(s).matchAll(/\{([a-zA-Z][a-zA-Z0-9_]*)\}/g)]
    .map((m) => m[1])
    .sort()
    .join('|');
for (const lang of expected) {
  const cat = catalogs[lang];
  if (!cat) {
    errors.push(`${lang}: missing catalog`);
    continue;
  }
  for (const [key, value] of Object.entries(cat))
    for (const [form, text] of Object.entries(typeof value === 'string' ? { value } : value)) {
      if (typeof text !== 'string' || !text.trim())
        errors.push(`${lang}.${key} (${form}): empty/non-string value`);
      else if (placeholders(key) !== placeholders(text))
        errors.push(
          `${lang}.${key} (${form}): placeholders differ (${placeholders(key)} vs ${placeholders(text)})`,
        );
    }
}
const literals = new Set();
for (const file of walk(src)) {
  if (/\/i18n\//.test(file) || /\.test\.tsx?$/.test(file) || /\/(?:__tests__|test)\//.test(file))
    continue;
  const ast = ts.createSourceFile(
    file,
    fs.readFileSync(file, 'utf8'),
    ts.ScriptTarget.Latest,
    true,
    file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  function visit(node) {
    if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      ['t', 'translate'].includes(node.expression.text)
    ) {
      const arg = node.arguments[0];
      if (arg && (ts.isStringLiteral(arg) || ts.isNoSubstitutionTemplateLiteral(arg)))
        literals.add(arg.text);
    }
    ts.forEachChild(node, visit);
  }
  visit(ast);
}
const knownSources = new Set([...literals, ...Object.keys(catalogs.en || {})]);
for (const lang of expected)
  for (const source of knownSources)
    if (lang !== 'en' && !Object.hasOwn(catalogs[lang] || {}, source))
      errors.push(`${lang}: no catalog entry for UI literal ${JSON.stringify(source)}`);
for (const [tag, want] of [
  ['fi-FI', 'fi'],
  ['ja-JP', 'ja'],
  ['nl-NL', 'nl'],
  ['pt-BR', 'pt_BR'],
  ['pt-PT', 'pt_PT'],
  ['uk-UA', 'uk'],
  ['vi-VN', 'vi'],
  ['zh-CN', 'zh_CN'],
])
  if (normalizeLanguageTag(tag) !== want)
    errors.push(`Locale probe ${tag}: expected ${want}, got ${normalizeLanguageTag(tag)}`);
const firmwarePo = path.resolve(root, '../compas-firmware/i18n/po');
if (fs.existsSync(firmwarePo)) {
  const firmwareLocales = fs
    .readdirSync(firmwarePo)
    .filter((name) => name.endsWith('.po'))
    .map((name) => name.slice(0, -3))
    .sort();
  if (
    firmwareLocales.join() !==
    expected
      .filter((lang) => lang !== 'en')
      .sort()
      .join()
  )
    errors.push(`Firmware locale set differs: ${firmwareLocales.join(', ')}`);
}
for (const lang of expected)
  if (!translate('unknown source', lang).includes('unknown source'))
    errors.push(`${lang}: unknown source fallback failed`);
for (const lang of expected)
  if (catalogs[lang])
    catalogs[lang].__plural_probe = { one: 'one', few: 'few', many: 'many', other: 'other' };
for (const [lang, count, want] of [
  ['en', 1, 'one'],
  ['en', 2, 'other'],
  ['fr', 0, 'one'],
  ['fr', 2, 'other'],
  ['ru', 2, 'few'],
  ['ru', 5, 'many'],
  ['ru', 1, 'one'],
  ['fi', 1, 'one'],
  ['ja', 2, 'other'],
  ['uk', 22, 'few'],
  ['pt_BR', 1, 'one'],
  ['vi', 2, 'other'],
  ['zh_CN', 2, 'other'],
])
  if (translate('__plural_probe', lang, { count }) !== want)
    errors.push(`${lang}: plural probe count ${count} expected ${want}`);
// Compare our Hermes-safe selector with the host's CLDR implementation.
for (const lang of expected) {
  const rules = new Intl.PluralRules(lang.replace('_', '-'));
  for (const count of [
    0, 0.5, 1, 1.5, 2, 3, 5, 11, 12, 14, 21, 22, 25, 101, 111, 1000000, -1, -2,
  ]) {
    if (translate('__plural_probe', lang, { count }) !== rules.select(count))
      errors.push(`${lang}: CLDR mismatch at ${count}`);
  }
  for (const [key, value] of Object.entries(catalogs[lang])) {
    if (
      typeof value === 'object' &&
      ['fi', 'ja', 'nl', 'pt_BR', 'pt_PT', 'uk', 'vi', 'zh_CN'].includes(lang)
    ) {
      for (const form of rules.resolvedOptions().pluralCategories) {
        if (!value[form]) errors.push(`${lang}.${key}: missing plural form ${form}`);
      }
    }
    for (const text of typeof value === 'string' ? [value] : Object.values(value)) {
      if (/ZXQ|QXZ|⟪BR⟫|9870000|[\u200B-\u200D\uFEFF]/.test(text))
        errors.push(`${lang}.${key}: translation marker leaked`);
      if (text !== text.normalize('NFC')) errors.push(`${lang}.${key}: not NFC`);
    }
  }
}
for (const lang of expected) delete catalogs[lang]?.__plural_probe;
if (errors.length) {
  const sample = errors.filter((e) => !e.includes('no catalog entry')).slice(0, 12);
  const coverage = expected
    .map(
      (lang) => `${lang}:${errors.filter((e) => e.startsWith(`${lang}: no catalog entry`)).length}`,
    )
    .filter((x) => !x.endsWith(':0'));
  console.error(
    `Translation validation failed (${errors.length}); coverage gaps by locale: ${coverage.join(', ') || 'none'}${sample.length ? `\nOther findings:\n- ${sample.join('\n- ')}` : ''}`,
  );
  process.exitCode = 1;
} else
  console.log(
    `Translation validation passed: ${expected.length} locales, ${Object.keys(catalogs.en).length} English catalog keys, ${literals.size} literal UI strings.`,
  );
