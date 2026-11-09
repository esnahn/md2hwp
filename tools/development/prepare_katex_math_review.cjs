// SPDX-License-Identifier: AGPL-3.0-only
// Extract pinned fixtures with user-authorized logo removal from case 060 onward.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const yaml = require(path.join(root, '.local/dependencies/katex/js-yaml-4.1.1/package'));
const folder = path.join(root, 'artifacts/katex-math-review');
const upstream = path.join(folder, 'upstream');
const raw = fs.readFileSync(path.join(upstream, 'ss_data.yaml'));
const data = yaml.load(raw.toString('utf8'));
const commit = fs.readFileSync(path.join(upstream, 'commit.txt'), 'utf8').trim().replace(/^\uFEFF/, '');
if (!/^[0-9a-f]{40}$/.test(commit)) throw Error('Invalid upstream commit');
const source = {project: 'KaTeX/KaTeX', commit,
  path: 'test/screenshotter/ss_data.yaml',
  url: `https://github.com/KaTeX/KaTeX/blob/${commit}/test/screenshotter/ss_data.yaml`,
  sha256: crypto.createHash('sha256').update(raw).digest('hex'),
  license: 'MIT', copyright: 'Copyright (c) 2013-2020 Khan Academy and other contributors'};
const cases = Object.entries(data).map(([name, value], index) => {
  const record = typeof value === 'string' ? {tex: value} : value;
  if (!record || typeof record.tex !== 'string') throw Error('Missing TeX: ' + name);
  const options = {displayMode: Boolean(record.display)};
  if (record.macros) options.macros = record.macros;
  const item = {id: `katex-${String(index + 1).padStart(3, '0')}`, name,
    tex: record.tex, options, upstream_options: Object.fromEntries(Object.entries(record).filter(([key]) => key !== 'tex'))};
  if (index + 1 >= 60 && /\\KaTeX(?![A-Za-z])/.test(record.tex)) {
    item.original_tex = record.tex;
    item.tex = record.tex.replace(/\+?\\KaTeX(?![A-Za-z])/g, '');
    item.adaptation_reason = 'User-authorized removal of \\KaTeX logo terms from case 060 onward, including their preceding + where present.';
  }
  if (index + 1 === 96) {
    const firstRule = record.tex.indexOf('\\rule');
    const rowBreak = record.tex.lastIndexOf('\\\\', firstRule);
    const end = record.tex.lastIndexOf('\\end{array}');
    if (firstRule < 0 || rowBreak < 0 || end < firstRule ||
        record.tex.slice(rowBreak + 2, firstRule).trim().length !== 0) {
      throw Error('Unexpected rule-only final row: ' + item.id);
    }
    item.original_tex = record.tex;
    item.tex = record.tex.slice(0, rowBreak).trimEnd() + '\n' + record.tex.slice(end);
    item.adaptation_reason = 'User-authorized removal of the rule-only final row from case 096; the relative-unit expressions are unchanged.';
  }
  return item;
});
const result = JSON.stringify({source, cases}, null, 2) + '\n';
const fixture = path.join(root, 'tests/katex');
fs.mkdirSync(fixture, {recursive: true});
fs.writeFileSync(path.join(fixture, 'cases.json'), result);
fs.writeFileSync(path.join(folder, 'cases.json'), result);
fs.copyFileSync(path.join(upstream, 'LICENSE'), path.join(fixture, 'LICENSE'));
console.log(JSON.stringify({count: cases.length, source}));
