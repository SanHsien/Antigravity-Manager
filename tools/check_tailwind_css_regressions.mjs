import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { twMerge } from 'tailwind-merge';

const assetsDirectory = join(process.cwd(), 'dist', 'assets');
const cssFiles = (await readdir(assetsDirectory)).filter((name) => name.endsWith('.css'));
assert.ok(cssFiles.length > 0, 'Expected npm run build to emit a CSS asset before this check.');
const css = (await Promise.all(cssFiles.map((name) => readFile(join(assetsDirectory, name), 'utf8')))).join('\n');

const expectedCss = [
  [/\.shadow-xs\{--tw-shadow:0 1px 2px 0 /, 'v3 shadow-sm dimensions'],
  [/--radius-xs:\.125rem/, 'v3 rounded-sm radius token'],
  [/\.rounded-xs\{border-radius:var\(--radius-xs\)/, 'v3 rounded-sm compatibility utility'],
  [/\.outline-hidden\{--tw-outline-style:none;outline-style:none\}@media\(forced-colors:active\)\{\.outline-hidden\{outline-offset:2px;outline:2px solid #0000\}\}/, 'v3 outline-none forced-colors behavior'],
  [/\.dark\\:bg-base-100\b/, 'class-based dark variant'],
  [/\[data-theme=light\]/, 'light daisyUI theme'],
  [/\[data-theme=dark\]/, 'dark daisyUI theme'],
  [/--color-base-100:oklch\(100% 0 0\)/, 'light base-100 color'],
  [/--color-base-100:oklch\(25\.33% \.016 252\.42\)/, 'baseline dark base-100 color'],
  [/--color-base-200:oklch\(23\.26% \.014 253\.1\)/, 'baseline dark base-200 color'],
  [/--color-base-300:oklch\(21\.15% \.012 254\.09\)/, 'baseline dark base-300 color'],
  [/--color-primary:oklch\(58% \.233 277\.117\)/, 'baseline dark primary color'],
  [/--color-secondary:oklch\(65% \.241 354\.308\)/, 'baseline dark secondary color'],
  [/\.\\@container\\\/logo\b/, 'native container query utility'],
  [/\.ring-2\{--tw-ring-shadow:[^}]*calc\(2px \+ var\(--tw-ring-offset-width\)\)/, 'existing explicit ring dimensions'],
  [/\.\\!mt-0\{margin-top:0!important\}/, 'important utility output'],
];

for (const [pattern, label] of expectedCss) {
  assert.match(css, pattern, `Missing ${label} in compiled CSS.`);
}

assert.equal(twMerge('shadow-xs shadow-sm'), 'shadow-sm');
assert.equal(twMerge('rounded-xs rounded-sm'), 'rounded-sm');
assert.equal(twMerge('ring-3 ring-2'), 'ring-2');
assert.equal(twMerge('outline-hidden outline-none'), 'outline-none');

console.log('TAILWIND CSS REGRESSIONS GREEN');
