#!/usr/bin/env node
// Writes the licence notices of the third-party code inside the app: the Rust crates it is
// built from and the npm packages in its UI bundle. Identical licence texts are written once,
// followed by the packages they apply to. Run it after `npm ci` and `cargo fetch`; the result
// (src-tauri/licenses-generated/THIRD_PARTY_NOTICES.txt) is bundled with the installers.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(root, 'src-tauri', 'licenses-generated', 'THIRD_PARTY_NOTICES.txt');
const LICENCE_FILE = /^(licen[sc]e|copying|notice|unlicense)([-_.].*)?$/i;

/** Licence and notice files at the top of a package folder. */
function licenceTexts(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile() && LICENCE_FILE.test(e.name))
    .map((e) => readFileSync(join(dir, e.name), 'utf8').replace(/\r\n/g, '\n').trim())
    .filter(Boolean);
}

const packages = [];

// Rust: everything cargo resolves except this repository's own crates.
// Only the crates for the platform being built end up in its installer.
const host = /^host: (.+)$/m.exec(execFileSync('rustc', ['-vV'], { encoding: 'utf8' }))?.[1];
const metadata = JSON.parse(
  execFileSync(
    'cargo',
    ['metadata', '--format-version', '1', '--locked', '--filter-platform', host],
    {
      cwd: root,
      maxBuffer: 1 << 28,
    },
  ),
);
const members = new Set(metadata.workspace_members);
for (const p of metadata.packages) {
  if (members.has(p.id)) continue;
  packages.push({
    kind: 'rust',
    name: p.name,
    version: p.version,
    licence: p.license ?? p.license_file ?? 'unknown',
    source: p.repository ?? '',
    texts: licenceTexts(dirname(p.manifest_path)),
  });
}

// npm: the production dependencies and everything they need.
const seen = new Set();
function visit(name, from) {
  let dir = from;
  for (;;) {
    const candidate = join(dir, 'node_modules', name);
    if (existsSync(join(candidate, 'package.json'))) return walk(candidate);
    const up = dirname(dir);
    if (up === dir) return;
    dir = up;
  }
}
function walk(dir) {
  const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  const key = `${pkg.name}@${pkg.version}`;
  if (seen.has(key)) return;
  seen.add(key);
  const licence =
    typeof pkg.license === 'string'
      ? pkg.license
      : (pkg.license?.type ?? pkg.licenses?.[0]?.type ?? 'unknown');
  packages.push({
    kind: 'npm',
    name: pkg.name,
    version: pkg.version,
    licence,
    source: typeof pkg.repository === 'string' ? pkg.repository : (pkg.repository?.url ?? ''),
    texts: licenceTexts(dir),
  });
  for (const dep of Object.keys({ ...pkg.dependencies, ...pkg.optionalDependencies }))
    visit(dep, dir);
}
const own = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
for (const dep of Object.keys(own.dependencies ?? {})) visit(dep, root);

// Group by text.
const groups = new Map();
const without = [];
for (const p of packages) {
  if (p.texts.length === 0) {
    without.push(p);
    continue;
  }
  const text = p.texts.join('\n\n----\n\n');
  const hash = createHash('sha256').update(text).digest('hex');
  if (!groups.has(hash)) groups.set(hash, { text, packages: [] });
  groups.get(hash).packages.push(p);
}
const label = (p) =>
  `${p.name} ${p.version} (${p.kind}, ${p.licence})${p.source ? ` ${p.source}` : ''}`;

let body = `Third-party code in Compás Tools\n\n${packages.length} packages: Rust crates the app is built from and npm packages in its interface.\nThe text of each licence follows, with the packages it applies to.\n`;
for (const { text, packages: list } of [...groups.values()].sort(
  (a, b) => b.packages.length - a.packages.length,
)) {
  body += `\n${'='.repeat(78)}\n`;
  body += list
    .sort((a, b) => a.name.localeCompare(b.name))
    .map(label)
    .join('\n');
  body += `\n${'-'.repeat(78)}\n${text}\n`;
}
if (without.length > 0) {
  body += `\n${'='.repeat(78)}\nPackages that ship no licence file in their sources; the licence is the one named here.\n`;
  body += `${'-'.repeat(78)}\n${without
    .sort((a, b) => a.name.localeCompare(b.name))
    .map(label)
    .join('\n')}\n`;
}
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, body);
console.log(`${packages.length} packages, ${groups.size} distinct licence texts -> ${out}`);
