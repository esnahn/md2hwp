// SPDX-License-Identifier: AGPL-3.0-only
// Freeze the specification review as PNGs; no runtime or font payload is published.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const crypto = require('node:crypto');

const options = {};
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i]?.replace(/^--/, '');
  if (!process.argv[i]?.startsWith('--') || !['runtime', 'report', 'native-dir', 'playwright'].includes(key) || !process.argv[i + 1] || options[key]) throw Error('Expected unique option/value pairs: --runtime --report --native-dir [--playwright]');
  options[key] = process.argv[i + 1];
}
for (const key of ['runtime', 'report', 'native-dir']) if (!options[key]) throw Error('Missing --' + key);
const root = path.resolve(__dirname, '../..');
const runtime = fs.realpathSync(path.resolve(options.runtime));
const nativeRoot = fs.realpathSync(path.resolve(options['native-dir']));
const reportPath = path.resolve(options.report);
const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
const native = JSON.parse(fs.readFileSync(path.join(nativeRoot, 'gallery.json'), 'utf8'));
const {chromium} = require(options.playwright || 'playwright');
const fontRoot = [path.join(runtime, 'fonts/mathjax-newcm-font'), path.join(runtime, '@mathjax/mathjax-newcm-font')].find(directory => fs.existsSync(path.join(directory, 'package.json')));
if (!fontRoot) throw Error('Missing @mathjax/mathjax-newcm-font package');
const version = JSON.parse(fs.readFileSync(path.join(runtime, 'mathjax/package.json'), 'utf8')).version;
const fontVersion = JSON.parse(fs.readFileSync(path.join(fontRoot, 'package.json'), 'utf8')).version;
if (version !== '4.1.3' || fontVersion !== version) throw Error('Inspect renderer/font changes before changing the 4.1.3 pin');
const inside = (base, file) => file.startsWith(base + path.sep);
const esc = value => String(value ?? '').replace(/[&<>"']/g, c => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}[c]));
if (!report.source || !Array.isArray(report.cases) || !report.cases.length) throw Error('Missing source/cases in conversion report');
const sourcePath = path.resolve(root, report.source.path || '');
if (inside(root, sourcePath) && fs.existsSync(sourcePath) && report.source.sha256) {
  const digest = crypto.createHash('sha256').update(fs.readFileSync(sourcePath)).digest('hex');
  if (digest.toLowerCase() !== report.source.sha256.toLowerCase()) throw Error('Source PDF SHA-256 mismatch');
}
const ids = new Set();
for (const item of report.cases) {
  if (typeof item.id !== 'string' || !item.id || ids.has(item.id) || typeof item.hancom_script !== 'string') throw Error('Invalid or duplicate report case: ' + item.id);
  ids.add(item.id);
  for (const key of ['hml_to_tex', 'tex_to_hml']) {
    const result = item[key];
    if (!result || typeof result.ok !== 'boolean' || (result.ok ? typeof result.output !== 'string' : typeof result.error !== 'string')) throw Error('Invalid ' + key + ' result: ' + item.id);
  }
}
const captures = new Map();
for (const [key, state] of [['cases', 'captured'], ['empty_renders', 'empty'], ['failures', 'failure']]) {
  if (native[key] !== undefined && !Array.isArray(native[key])) throw Error('Invalid native ' + key);
  for (const item of native[key] || []) {
    if (typeof item.case_id !== 'string' || !item.case_id || captures.has(item.case_id)) throw Error('Duplicate/invalid native capture ID: ' + item.case_id);
    const entry = {...item, state};
    if (state === 'captured') {
      if (typeof item.script !== 'string' || typeof item.hancom_script !== 'string' || item.script !== item.hancom_script || item.script_equal !== true) throw Error('Native SCRIPT readback mismatch: ' + item.case_id);
      if (typeof item.image !== 'string' || !item.image || path.isAbsolute(item.image) || /^[A-Za-z]:/.test(item.image) || item.image.includes('\\')) throw Error('Native PNG must have a relative portable path: ' + item.case_id);
      const file = fs.realpathSync(path.resolve(nativeRoot, item.image));
      if (!inside(nativeRoot, file) || !fs.statSync(file).isFile()) throw Error('Native PNG escaped capture folder: ' + item.case_id);
      const bytes = fs.readFileSync(file);
      if (!bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) throw Error('Not a PNG: ' + item.case_id);
      entry.image = 'data:image/png;base64,' + bytes.toString('base64');
    }
    captures.set(item.case_id, entry);
  }
}
function capture(id, expected) {
  if (!id) return {state: 'unavailable', error: '변환 결과가 없어 네이티브 캡처를 요청하지 않았습니다.'};
  const item = captures.get(id);
  if (!item) return {state: 'missing', error: '네이티브 캡처 결과 없음: ' + id};
  if (typeof item.script === 'string' && item.script !== expected) throw Error('Native input differs from reported conversion: ' + id);
  return item;
}
const evidence = {renderer: 'MathJax', version, font: 'mathjax-newcm', fontVersion, source: report.source, cases: [], blocked: []};
const mime = {'.js': 'application/javascript', '.json': 'application/json', '.woff2': 'font/woff2', '.woff': 'font/woff'};
const server = http.createServer((request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (pathname === '/') {
      response.writeHead(200, {'Content-Type': 'text/html; charset=utf-8'});
      return response.end('<!doctype html><meta charset="utf-8"><style>body{margin:0;background:white;color:black;font:58px serif}#math{width:max-content}svg{display:block}</style><div id="math"></div>');
    }
    const prefix = '/fonts/mathjax-newcm-font/';
    const base = pathname.startsWith(prefix) ? fs.realpathSync(fontRoot) : runtime;
    const relative = pathname.startsWith(prefix) ? pathname.slice(prefix.length) : '.' + pathname;
    const file = fs.realpathSync(path.resolve(base, relative));
    if (!inside(base, file) || !fs.statSync(file).isFile()) throw Error('Invalid file');
    response.writeHead(200, {'Content-Type': mime[path.extname(file)] || 'application/octet-stream'});
    fs.createReadStream(file).pipe(response);
  } catch { response.writeHead(404); response.end(); }
});

async function main() {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = 'http://127.0.0.1:' + server.address().port;
  const browser = await chromium.launch({channel: 'msedge', headless: true});
  try {
    const page = await browser.newPage({viewport: {width: 2200, height: 1400}, deviceScaleFactor: 1});
    await page.route(/^https?:/, route => {
      if (new URL(route.request().url()).origin === origin) return route.continue();
      evidence.blocked.push(route.request().url());
      return route.abort();
    });
    await page.goto(origin);
    await page.evaluate(origin => {
      window.MathJax = {
        loader: {paths: {mathjax: origin + '/mathjax', fonts: origin + '/fonts'}, load: ['[tex]/color', '[tex]/enclose', '[tex]/boldsymbol', '[tex]/textmacros', '[tex]/textcomp']},
        tex: {packages: {'[+]': ['color', 'enclose', 'boldsymbol', 'textmacros', 'textcomp'], '[-]': ['noundefined']}}, output: {font: 'mathjax-newcm'},
        svg: {fontCache: 'local'}, options: {enableMenu: false}, startup: {typeset: false}
      };
    }, origin);
    await page.addScriptTag({url: origin + '/mathjax/tex-svg.js'});
    await page.evaluate(() => MathJax.startup.promise);
    if (await page.evaluate(() => MathJax.version) !== version) throw Error('Renderer version mismatch');
    async function renderPng(tex) {
      const result = await page.evaluate(async tex => {
        MathJax.texReset();
        const node = await MathJax.tex2svgPromise(tex, {display: true});
        const error = node.querySelector('[data-mjx-error],[data-mml-node="merror"]');
        if (error) return {error: error.getAttribute('data-mjx-error') || error.textContent};
        const svg = node.querySelector('svg');
        if (!svg) return {error: 'Missing SVG'};
        document.querySelector('#math').replaceChildren(svg);
        return {};
      }, tex);
      if (result.error) throw Error(result.error);
      return 'data:image/png;base64,' + (await page.locator('#math').screenshot()).toString('base64');
    }
    async function comparePixels(left, right) {
      return page.evaluate(async ([left, right]) => {
        async function decode(src) {
          const image = new Image(); image.src = src; await image.decode();
          const canvas = document.createElement('canvas'); canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
          const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
          return {width: canvas.width, height: canvas.height, data: context.getImageData(0, 0, canvas.width, canvas.height).data};
        }
        const a = await decode(left), b = await decode(right);
        return {pixel_equal: a.width === b.width && a.height === b.height && a.data.every((value, index) => value === b.data[index]), original_size: [a.width, a.height], roundtrip_size: [b.width, b.height]};
      }, [left, right]);
    }
    function nativePanel(role, title, script, item) {
      const status = item.state === 'captured' ? '<div class="image"><img alt="' + esc(title) + '" src="' + item.image + '"></div>' : '<p class="diagnostic">' + esc(item.state === 'empty' ? '한글 캡처: 표시 결과가 비어 있습니다.' : item.error || '한글 캡처 실패') + '</p>';
      return '<section data-role="' + role + '" data-native-state="' + esc(item.state) + '"><h3>' + title + '</h3>' + status + '<pre>' + esc(script) + '</pre></section>';
    }
    const cards = [];
    for (const item of report.cases) {
      const inputScript = item.input_script ?? item.hancom_script;
      const original = capture(item.native_original_id, inputScript);
      const roundtrip = item.tex_to_hml.ok ? capture(item.native_roundtrip_id, item.tex_to_hml.output) : {state: 'unavailable', error: 'TeX→HML 변환 오류: ' + item.tex_to_hml.error};
      const row = {id: item.id, hml_to_tex: item.hml_to_tex.ok, tex_to_hml: item.tex_to_hml.ok, native_original: original.state, native_roundtrip: roundtrip.state, pixel_equal: null, reused_capture: Boolean(item.native_original_id && item.native_original_id === item.native_roundtrip_id)};
      if (original.state === 'captured' && roundtrip.state === 'captured') Object.assign(row, await comparePixels(original.image, roundtrip.image));
      let texImage = '', mathjaxError = null;
      if (item.hml_to_tex.ok) {
        try { texImage = await renderPng(item.hml_to_tex.output); } catch (error) { mathjaxError = String(error.message || error); }
      }
      row.mathjax = item.hml_to_tex.ok ? (mathjaxError ? {ok: false, error: mathjaxError} : {ok: true}) : {ok: false, error: 'HML→TeX 변환 결과 없음'};
      const diagnostic = item.test_script
        ? '<p class="note">명세 원문은 아래와 같고, 실제 시험에는 피연산자·문맥을 넣었습니다. ' + esc(item.test_notes || '') + '</p><pre>' + esc(item.hancom_script) + '</pre>'
        : item.source_kind === 'syntax' ? '<p class="note">명세의 구문 토큰을 그대로 입력했습니다. 피연산자·문맥이 필요한 토큰의 오류는 완성 수식의 지원 여부 판정이 아닙니다.</p>' : '';
      const duplicateSources = (item.duplicate_sources || []).length
        ? '<details class="note"><summary>함께 정리한 명세 출처 ' + (item.duplicate_sources.length + 1) + '곳</summary>' + item.duplicate_sources.map(source => '<p>PDF ' + esc(source.pdf_page) + '쪽 · ' + esc(source.name) + '</p><pre>' + esc(source.source_text || source.hancom_script) + '</pre>').join('') + '</details>'
        : '';
      const texStatus = !item.hml_to_tex.ok ? '<p class="diagnostic">HML→TeX 변환 오류: ' + esc(item.hml_to_tex.error) + '</p>' : mathjaxError ? '<p class="diagnostic">MathJax 렌더링 오류: ' + esc(mathjaxError) + '</p>' : '<div class="image"><img alt="현재 HML→TeX 결과의 MathJax 이미지" src="' + texImage + '"></div>';
      const pixelLabel = row.pixel_equal === null ? '픽셀 비교 불가' : row.pixel_equal ? '디코딩 PNG 픽셀 같음' : '디코딩 PNG 픽셀 다름';
      cards.push('<article id="case-' + esc(item.id) + '"><h2>' + esc(item.id + ' · ' + (item.display_name || item.name)) + '</h2><p class="meta">' + esc(item.source_section || item.category) + (item.continued_from_pdf_page ? ' · PDF ' + esc(item.continued_from_pdf_page) + '쪽 표의 계속' : '') + ' · PDF ' + esc(item.pdf_page) + '쪽 / 인쇄 ' + esc(item.printed_page) + '쪽 · ' + esc(item.source_kind) + ' · ' + pixelLabel + (row.reused_capture ? ' · 동일 SCRIPT의 같은 캡처를 재사용' : '') + '</p>' + (item.notes ? '<p class="note">' + esc(Array.isArray(item.notes) ? item.notes.join(' ') : item.notes) + '</p>' : '') + diagnostic + duplicateSources + '<div class="grid">' + nativePanel('native-source', '원본 한글 · HML/SCRIPT', inputScript, original) + nativePanel('native-roundtrip', 'HML→TeX→HML 실제 한글 출력', item.tex_to_hml.ok ? item.tex_to_hml.output : '', roundtrip) + '<section class="full-width" data-role="hml-to-tex" data-input="native-source" data-output="hml_to_tex" data-conversion="hml_to_tex" data-renderer="MathJax" data-renderer-version="' + version + '" data-renderer-font="mathjax-newcm"><h3>HML→TeX 현재 출력 · MathJax ' + version + '</h3>' + texStatus + '<pre>' + esc(item.hml_to_tex.ok ? item.hml_to_tex.output : '') + '</pre></section></div></article>');
      evidence.cases.push(row);
      if (cards.length % 40 === 0) console.log('specification ' + cards.length + '/' + report.cases.length);
    }
    if (evidence.blocked.length) throw Error('Unexpected external network requests: ' + JSON.stringify(evidence.blocked));
    const count = predicate => evidence.cases.filter(predicate).length;
    evidence.summary = {cases: evidence.cases.length, hml_to_tex_ok: count(x => x.hml_to_tex), tex_to_hml_ok: count(x => x.tex_to_hml), mathjax_ok: count(x => x.mathjax.ok), native_captured: [...captures.values()].filter(x => x.state === 'captured').length, native_empty: [...captures.values()].filter(x => x.state === 'empty').length, native_failure: [...captures.values()].filter(x => x.state === 'failure').length, native_original_captured: count(x => x.native_original === 'captured'), native_roundtrip_captured: count(x => x.native_roundtrip === 'captured'), pixel_equal: count(x => x.pixel_equal === true), pixel_different: count(x => x.pixel_equal === false), pixel_unavailable: count(x => x.pixel_equal === null), reused_capture: count(x => x.reused_capture)};
    const summary = evidence.summary;
    const html = '<!doctype html>\n<html lang="ko"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src \'none\'; img-src data:; style-src \'unsafe-inline\'"><title>명세 예시 수식</title><style>body{margin:0;background:#f3f5f8;color:#182030;font:16px/1.6 system-ui,sans-serif}main{max-width:1440px;margin:auto;padding:32px}a{color:#1459a1}article{background:white;border:1px solid #ccd3de;border-radius:10px;margin:24px 0;padding:24px}h1,h2,h3{line-height:1.35}h2{font-size:21px}h3{font-size:17px}.grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px}.full-width{grid-column:1/-1}section{min-width:0;border-top:1px solid #dde2ea}.image{overflow:auto;background:white;padding:16px}.image img{max-width:100%;height:auto;object-fit:contain}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f0f3f7;padding:14px;border-radius:5px;font:14px/1.6 Consolas,monospace}.meta,.note{color:#526071}.diagnostic{color:#8c2532;background:#fff0f1;padding:12px;overflow-wrap:anywhere}@media(max-width:800px){main{padding:12px}article{padding:16px}.grid{grid-template-columns:1fr}.full-width{grid-column:1}}</style></head><body><main><h1>명세 예시 수식</h1><p><a href="../math.md">수식 변환 문서</a> · <a href="gallery.html">메뉴 예시 수식</a> · <a href="toolbar.html">툴바 수식</a></p><p>한글문서파일형식 수식 revision 1.2의 입력 예시·개별 기호·구문을 원문 그대로 보존하고, 보충 시험은 별도로 표시한 검토 자료입니다. 위에는 원본 한글과 실제 HML→TeX→HML 왕복 한글 출력, 아래에는 현재 변환기의 TeX 코드와 로컬 MathJax 4.1.3 · New Computer Modern 렌더링을 표시합니다. 변환·캡처·렌더링 오류와 빈 출력도 모두 남깁니다.</p><p>LaTeX에서 사용하려면 <code>\\usepackage{amsmath,amssymb,textcomp}</code>를 준비합니다. RTANGLE·well·base·benzene은 유니코드로 출력하며 일반 pdfLaTeX에서는 별도 설정이 필요합니다. 호환 제한은 각 카드에 표시했습니다.</p><p>전체 ' + summary.cases + '개 · HML→TeX 성공 ' + summary.hml_to_tex_ok + '개 · TeX→HML 성공 ' + summary.tex_to_hml_ok + '개 · MathJax 성공 ' + summary.mathjax_ok + '개 · 고유 네이티브 캡처 ' + summary.native_captured + '개 / 빈 출력 ' + summary.native_empty + '개 / 실패 ' + summary.native_failure + '개 · 왕복 픽셀 같음 ' + summary.pixel_equal + '개 / 다름 ' + summary.pixel_different + '개 / 비교 불가 ' + summary.pixel_unavailable + '개.</p><p>PNG를 디코딩한 크기와 RGBA 픽셀을 비교했습니다. 같은 캡처를 재사용한 항목은 별도 표시합니다. 원본·왕복 한글의 픽셀 같음은 가운데 TeX와의 서체 일치나 독립적인 원문 명세의 시각 정확성 검증을 뜻하지 않습니다. 이 HTML은 PNG와 코드만 포함하며 인터넷 없이 열립니다. 렌더링 라이브러리·폰트 파일·PDF 페이지 이미지는 포함하지 않습니다.</p><p>출처: <a href="../../reference/hancom-api/' + encodeURIComponent('한글문서파일형식_수식_revision1.2.pdf') + '">한글문서파일형식_수식_revision1.2.pdf</a> · SHA-256 <code>' + esc(report.source.sha256) + '</code>. 본 제품은 한글과컴퓨터의 글 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다. 프로젝트 라이선스가 한컴의 내장 예시·서체 등에 대한 권리를 대신하지 않습니다.</p>' + cards.join('\n') + '</main></body></html>\n';
    if (html.includes('<svg') || html.includes('<script') || html.includes('<path ')) throw Error('Runtime/font payload in review HTML');
    fs.mkdirSync(path.join(root, 'docs/math'), {recursive: true});
    fs.writeFileSync(path.join(root, 'docs/math/specification.html'), html);
    fs.writeFileSync(path.join(path.dirname(reportPath), 'html-rendering.json'), JSON.stringify(evidence, null, 2) + '\n');
    console.log(JSON.stringify(evidence.summary));
  } finally { await browser.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => server.close());
