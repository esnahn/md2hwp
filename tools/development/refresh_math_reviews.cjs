// SPDX-License-Identifier: AGPL-3.0-only
// Refresh review HTML with local PNGs only; runtime, SVG and font payloads stay local.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawnSync} = require('node:child_process');

const args = process.argv.slice(2), options = {};
for (let i = 0; i < args.length; i += 2) {
  if (!args[i]?.startsWith('--') || !args[i + 1]) throw Error('Expected option/value pairs');
  const key = args[i].slice(2);
  if (!['runtime', 'converter', 'playwright', 'roundtrip-dir'].includes(key)) throw Error('Unknown option: --' + key);
  if (options[key]) throw Error('Duplicate option: --' + key);
  options[key] = args[i + 1];
}
if (!options.runtime || !options['roundtrip-dir']) {
  throw Error('Required: --runtime <local MathJax package root> --roundtrip-dir <native capture folder> [--converter <exe>] [--playwright <module>]');
}
const {chromium} = require(options.playwright || 'playwright');
const root = path.resolve(__dirname, '../..');
const runtime = path.resolve(options.runtime);
const roundtripRoot = path.resolve(options['roundtrip-dir']);
const fontRoot = [path.join(runtime, 'fonts/mathjax-newcm-font'), path.join(runtime, '@mathjax/mathjax-newcm-font')]
  .find(directory => fs.existsSync(path.join(directory, 'package.json')));
if (!fontRoot) throw Error('Missing @mathjax/mathjax-newcm-font package');
const converter = path.resolve(options.converter || path.join(root, 'target/debug/md2hwp-math.exe'));
const version = JSON.parse(fs.readFileSync(path.join(runtime, 'mathjax/package.json'), 'utf8')).version;
const fontVersion = JSON.parse(fs.readFileSync(path.join(fontRoot, 'package.json'), 'utf8')).version;
if (version !== '4.1.3' || fontVersion !== version) throw Error('Inspect renderer/font changes before changing the 4.1.3 pin');
const pages = [
  {name: 'gallery', fixture: 'supported-equations.json', count: 46},
  {name: 'toolbar', fixture: 'toolbar-equations.json', count: 76}
];
const mime = {'.js': 'application/javascript', '.json': 'application/json', '.woff2': 'font/woff2', '.woff': 'font/woff'};

function withoutFinalNewline(value) {
  return value.replace(/\r?\n$/, '');
}

function runConverter(command, input, id) {
  const result = spawnSync(converter, [command, input], {encoding: 'utf8', windowsHide: true});
  if (result.error) throw Error('Could not run converter for ' + id + ': ' + result.error.message);
  if (result.status !== 0) throw Error('Converter ' + command + ' failed for ' + id + ': ' + result.stderr);
  return withoutFinalNewline(result.stdout);
}

function captureId(item) {
  return item.case_id ?? item.id;
}

function loadRoundtrips(spec, fixtureCases) {
  const folder = path.join(roundtripRoot, spec.name);
  const jsonPath = path.join(folder, 'gallery.json');
  const document = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
  if (!Array.isArray(document.cases) || document.cases.length !== spec.count) {
    throw Error(spec.name + ' capture case count must be ' + spec.count);
  }
  if (fixtureCases.length !== spec.count) throw Error(spec.name + ' fixture case count must be ' + spec.count);
  const fixtures = new Map(fixtureCases.map(item => [String(item.id ?? item.case_id), item]));
  const captured = new Map();
  for (const item of document.cases) {
    const id = String(captureId(item) ?? '');
    if (!id || captured.has(id) || !fixtures.has(id)) throw Error(spec.name + ' capture has an unknown or duplicate case ID: ' + id);
    if (typeof item.script !== 'string' || typeof item.hancom_script !== 'string' || item.script !== item.hancom_script) {
      throw Error(spec.name + ' capture SCRIPT/readback mismatch: ' + id);
    }
    const source = fixtures.get(id).hancom_script;
    if (typeof source !== 'string') throw Error(spec.name + ' fixture is missing original SCRIPT: ' + id);
    if (item.original_script !== undefined && item.original_script !== source) {
      throw Error(spec.name + ' capture original_script mismatch: ' + id);
    }
    const image = item.image;
    if (typeof image !== 'string' || !image.trim() || path.isAbsolute(image)) {
      throw Error(spec.name + ' capture image must be a relative path: ' + id);
    }
    const imagePath = path.resolve(folder, image);
    if (!imagePath.startsWith(folder + path.sep) || !fs.existsSync(imagePath) || !fs.statSync(imagePath).isFile()) {
      throw Error(spec.name + ' capture image is missing or outside its capture folder: ' + id);
    }
    const bytes = fs.readFileSync(imagePath);
    if (bytes.length < 8 || !bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) {
      throw Error(spec.name + ' capture image is not a PNG: ' + id);
    }
    const tex = runConverter('hml-to-tex', source, spec.name + ' ' + id);
    if (item.tex !== undefined && item.tex !== tex) throw Error(spec.name + ' capture tex metadata mismatch: ' + id);
    const roundtripScript = runConverter('tex-to-hml', tex, spec.name + ' ' + id);
    if (item.script !== roundtripScript || item.hancom_script !== roundtripScript) {
      throw Error(spec.name + ' current tex-to-hml output differs from native capture: ' + id);
    }
    captured.set(id, {
      id,
      source,
      tex,
      roundtripScript,
      image: 'data:image/png;base64,' + bytes.toString('base64')
    });
  }
  for (const id of fixtures.keys()) if (!captured.has(id)) throw Error(spec.name + ' capture is missing case ID: ' + id);
  return captured;
}

const server = http.createServer((request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (pathname === '/') {
      response.writeHead(200, {'Content-Type': 'text/html'});
      return response.end('<!doctype html><meta charset="utf-8"><style>body{margin:0;background:white;color:black;font:58px serif}#math{width:max-content}svg{display:block}</style><div id="math"></div>');
    }
    const fontPrefix = '/fonts/mathjax-newcm-font';
    const file = pathname.startsWith(fontPrefix + '/')
      ? path.resolve(fontRoot, '.' + pathname.slice(fontPrefix.length))
      : path.resolve(runtime, '.' + pathname);
    if (!file.startsWith(runtime + path.sep) || !fs.statSync(file).isFile()) throw Error('Invalid file');
    response.writeHead(200, {'Content-Type': mime[path.extname(file)] || 'application/octet-stream'});
    fs.createReadStream(file).pipe(response);
  } catch { response.writeHead(404); response.end(); }
});

async function main() {
  const loaded = pages.map(spec => {
    const fixture = JSON.parse(fs.readFileSync(path.join(root, 'crates/md2hwp-math/tests/fixtures', spec.fixture), 'utf8')).cases;
    return {...spec, fixture, captures: loadRoundtrips(spec, fixture), file: path.join(root, 'docs/math', spec.name + '.html'), source: fs.readFileSync(path.join(root, 'docs/math', spec.name + '.html'), 'utf8')};
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = 'http://127.0.0.1:' + server.address().port;
  const browser = await chromium.launch({channel: 'msedge', headless: true});
  const output = [], evidence = {
    version,
    font: 'mathjax-newcm',
    fontVersion,
    pipeline: ['hml_to_tex', 'tex_to_hml'],
    pages: [],
    blocked: []
  };
  try {
    const render = await browser.newPage({viewport: {width: 2200, height: 1400}, deviceScaleFactor: 1});
    await render.route(/^https?:/, route => {
      if (new URL(route.request().url()).origin === origin) return route.continue();
      evidence.blocked.push(route.request().url());
      return route.abort();
    });
    await render.goto(origin);
    await render.evaluate(origin => {
      window.MathJax = {
        loader: {paths: {mathjax: origin + '/mathjax', fonts: origin + '/fonts'}, load: ['[tex]/color', '[tex]/enclose', '[tex]/boldsymbol', '[tex]/textmacros', '[tex]/textcomp']},
        tex: {packages: {'[+]': ['color', 'enclose', 'boldsymbol', 'textmacros', 'textcomp'], '[-]': ['noundefined']}},
        output: {font: 'mathjax-newcm'}, svg: {fontCache: 'local'},
        options: {enableMenu: false}, startup: {typeset: false}
      };
    }, origin);
    await render.addScriptTag({url: origin + '/mathjax/tex-svg.js'});
    await render.evaluate(() => MathJax.startup.promise);
    if (await render.evaluate(() => MathJax.version) !== version) throw Error('Renderer version mismatch');
    async function renderPng(tex, label) {
      const status = await render.evaluate(async tex => {
        MathJax.texReset();
        const node = await MathJax.tex2svgPromise(tex, {display: true});
        const error = node.querySelector('[data-mjx-error],[data-mml-node="merror"]');
        if (error) return {error: error.getAttribute('data-mjx-error') || error.textContent};
        const svg = node.querySelector('svg');
        if (!svg) return {error: 'Missing SVG'};
        document.querySelector('#math').replaceChildren(svg);
        return {error: null};
      }, tex);
      if (status.error) throw Error(label + ': ' + status.error);
      return render.locator('#math').screenshot();
    }
    const editor = await browser.newPage();
    for (const spec of loaded) {
      const rows = await editor.evaluate(({source, name}) => {
        const document = new DOMParser().parseFromString(source, 'text/html');
        return [...document.querySelectorAll('article')].map(article => {
          const heading = article.querySelector('h2')?.textContent ?? '';
          const id = name === 'gallery'
            ? article.id.replace(/^case-/, '')
            : (heading.match(/^\s*(\d+)/)?.[1] ?? '');
          const sourcePanel = article.querySelector('[data-role="native-source"]')
            ?? article.querySelector('.grid>section,.pair>section');
          const directPanels = [...article.querySelectorAll('.grid>section,.pair>section')];
          return {id, sourceScript: sourcePanel?.querySelector('pre')?.textContent ?? '', roles: [...article.querySelectorAll('[data-role]')].map(node => node.dataset.role), panelCount: directPanels.length};
        });
      }, {source: spec.source, name: spec.name});
      if (rows.length !== spec.count || spec.fixture.length !== spec.count || spec.captures.size !== spec.count) {
        throw Error(spec.name + ' review/capture case count mismatch');
      }
      const changes = [];
      for (let i = 0; i < rows.length; i++) {
        const row = rows[i];
        const selected = spec.fixture.find(item => String(item.id ?? item.case_id) === row.id);
        const captured = spec.captures.get(row.id);
        if (!selected || !captured) throw Error(spec.name + ' review/capture ID mismatch: ' + row.id);
        if (row.sourceScript !== selected.hancom_script || row.sourceScript !== captured.source) {
          throw Error(spec.name + ' original source panel does not match fixture: ' + row.id);
        }
        const hasRoles = row.roles.includes('native-source') || row.roles.includes('native-roundtrip') || row.roles.includes('hml-to-tex');
        if (hasRoles && (!row.roles.includes('native-source') || !row.roles.includes('native-roundtrip') || !row.roles.includes('hml-to-tex') || row.panelCount !== 3)) {
          throw Error(spec.name + ' review has an incomplete or duplicate panel role set: ' + row.id);
        }
        if (!hasRoles && row.panelCount !== (spec.name === 'gallery' ? 4 : 2)) {
          throw Error(spec.name + ' unsupported legacy panel count: ' + row.id);
        }
        const png = await renderPng(captured.tex, spec.name + ' ' + row.id);
        changes.push({...captured, png: 'data:image/png;base64,' + png.toString('base64')});
        if ((i + 1) % 20 === 0) console.log(spec.name + ' ' + (i + 1) + '/' + rows.length);
      }
      const html = await editor.evaluate(({source, changes, version, name}) => {
        const document = new DOMParser().parseFromString(source, 'text/html');
        const articleId = article => name === 'gallery'
          ? article.id.replace(/^case-/, '')
          : (article.querySelector('h2')?.textContent.match(/^\s*(\d+)/)?.[1] ?? '');
        const setPanel = (panel, role, title, code, image, alt) => {
          panel.dataset.role = role;
          const heading = panel.querySelector('h3') ?? document.createElement('h3');
          heading.textContent = title;
          if (!heading.parentNode) panel.prepend(heading);
          let pre = panel.querySelector('pre');
          if (!pre) { pre = document.createElement('pre'); panel.append(pre); }
          pre.textContent = code;
          let picture = panel.querySelector('img');
          if (!picture) {
            const holder = document.createElement('div');
            holder.className = role === 'hml-to-tex' ? (name === 'gallery' ? 'image' : 'math') : (name === 'gallery' ? 'image' : 'native-image');
            picture = document.createElement('img');
            holder.append(picture);
            panel.insertBefore(holder, pre);
          }
          picture.src = image;
          picture.alt = alt;
          if (role === 'hml-to-tex') {
            panel.dataset.input = 'native-source';
            panel.dataset.output = 'hml_to_tex';
            panel.dataset.renderer = 'MathJax';
            panel.dataset.rendererVersion = version;
            panel.dataset.rendererFont = 'mathjax-newcm';
          } else {
            delete panel.dataset.renderer;
            delete panel.dataset.rendererVersion;
            delete panel.dataset.rendererFont;
            delete panel.dataset.input;
            delete panel.dataset.output;
          }
        };
        for (const change of changes) {
          const article = [...document.querySelectorAll('article')].find(item => articleId(item) === change.id);
          if (!article) throw Error('Missing article during migration: ' + change.id);
          const existingSource = article.querySelector('[data-role="native-source"]');
          const grid = existingSource?.parentElement ?? article.querySelector('.grid,.pair');
          if (!grid) throw Error('Missing panel grid: ' + change.id);
          const oldPanels = [...grid.querySelectorAll(':scope>section')];
          const sourcePanel = existingSource ?? oldPanels[0];
          const existingRoundtrip = article.querySelector('[data-role="native-roundtrip"]');
          const existingTex = article.querySelector('[data-role="hml-to-tex"]');
          let roundtripPanel = existingRoundtrip;
          let texPanel = existingTex;
          if (!roundtripPanel) {
            roundtripPanel = name === 'gallery' && oldPanels.length === 4
              ? oldPanels[1]
              : sourcePanel.cloneNode(true);
          }
          if (!texPanel) texPanel = name === 'gallery' ? oldPanels[3] : oldPanels[1];
          if (!sourcePanel || !roundtripPanel || !texPanel) throw Error('Could not map panels: ' + change.id);
          if (grid.classList.contains('pair')) grid.className = 'grid';
          grid.replaceChildren(sourcePanel, roundtripPanel, texPanel);
          setPanel(sourcePanel, 'native-source', '원본 한글 · HML/SCRIPT', change.source, sourcePanel.querySelector('img')?.getAttribute('src') ?? '', '원본 한글 수식');
          setPanel(roundtripPanel, 'native-roundtrip', 'HML→TeX→HML 실제 한글 출력', change.roundtripScript, change.image, 'HML→TeX→HML 역변환 후 한글 실제 출력');
          setPanel(texPanel, 'hml-to-tex', 'HML→TeX 현재 출력 · MathJax 4.1.3', change.tex, change.png, '현재 HML→TeX 출력의 MathJax 이미지');
          texPanel.classList.add('full-width');
          texPanel.dataset.conversion = 'hml_to_tex';
          article.querySelectorAll('details').forEach(node => node.remove());
        }
        const paragraphs = [...document.querySelectorAll('main>p')];
        const lead = paragraphs.find(paragraph => paragraph.textContent.startsWith('각 항목은'));
        if (lead) {
          lead.textContent = '각 항목은 위쪽에 원본 한글 수식의 HML/SCRIPT와 이를 한글→TeX→한글로 왕복한 실제 한글 출력을 나란히 표시합니다. 아래에는 원본 HML/SCRIPT의 현재 한글→TeX 변환 코드와 MathJax 이미지를 표시합니다.';
        }
        const snapshot = paragraphs.find(paragraph => /^\d{4}-\d{2}-\d{2} 스냅샷\./.test(paragraph.textContent));
        if (snapshot) {
          snapshot.textContent = '2026-10-30 스냅샷. 왕복 한글 이미지는 한글 2020에서 캡처했으며, TeX 이미지는 원본 HML/SCRIPT의 현재 hml_to_tex 출력으로 MathJax ' + version + ' · New Computer Modern을 사용해 렌더링했습니다. 모든 이미지는 PNG이며 실행 라이브러리·폰트 파일은 포함하지 않습니다. 글꼴·간격·기호 크기 차이가 남으며 픽셀 동등성을 뜻하지 않습니다.';
        }
        for (const paragraph of document.querySelectorAll('.current-note')) {
          if (paragraph.textContent.trim() === '대응하는 일반 TeX 구문을 제안합니다. 글꼴·세부 간격의 동일성을 보장하지 않습니다.') {
            paragraph.textContent = '원본 HML/SCRIPT의 현재 한글→TeX 변환 결과입니다. 글꼴·세부 간격의 동일성을 보장하지 않습니다.';
          }
        }
        const style = document.querySelector('style');
        if (style && !style.textContent.includes('.grid>.full-width')) {
          style.textContent += '.grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:28px}.grid>.full-width{grid-column:1/-1}@media(max-width:900px){.grid{grid-template-columns:1fr}.grid>.full-width{grid-column:1}}';
        }
        const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
        while (walk.nextNode()) walk.currentNode.nodeValue = walk.currentNode.nodeValue.replace(/MathJax 3\.2\.2/g, 'MathJax ' + version + ' · New Computer Modern');
        return '<!doctype html>\n' + document.documentElement.outerHTML + '\n';
      }, {source: spec.source, changes, version, name: spec.name});
      if (html.includes('<svg') || html.includes('<path ') || html.includes('<script src=')) throw Error('Runtime/font payload in review HTML');
      output.push({file: spec.file, html});
      evidence.pages.push({name: spec.name, cases: rows.length, rendered: changes.length, panels: 3});
    }
    if (evidence.blocked.length) throw Error('Unexpected external network requests: ' + JSON.stringify(evidence.blocked));
    for (const item of output) fs.writeFileSync(item.file, item.html);
    const folder = path.join(root, 'artifacts/mathjax-update');
    fs.mkdirSync(folder, {recursive: true});
    fs.writeFileSync(path.join(folder, 'rendering.json'), JSON.stringify(evidence, null, 2) + '\n');
    console.log(JSON.stringify(evidence));
  } finally { await browser.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => server.close());
