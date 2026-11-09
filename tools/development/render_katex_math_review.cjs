// SPDX-License-Identifier: AGPL-3.0-only
// Render the pinned KaTeX screenshotter inputs without running fixture HTML.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const crypto = require('node:crypto');
const repo = path.resolve(__dirname, '../..');
const args = process.argv.slice(2);
const options = {};
for (let i = 0; i < args.length; i += 2) {
  if (!['--input', '--output', '--runtime', '--playwright'].includes(args[i]) || !args[i + 1] || options[args[i]]) throw Error('Expected unique --input/--output/--runtime/--playwright values');
  options[args[i]] = args[i + 1];
}
const {chromium} = require(options['--playwright'] || 'playwright');
const input = path.resolve(options['--input'] || path.join(repo, 'tests/katex/cases.json'));
const output = path.resolve(options['--output'] || path.join(repo, 'artifacts/katex-math-review/tex'));
const runtime = fs.realpathSync(path.resolve(options['--runtime'] || path.join(repo, '.local/dependencies/katex/katex-0.19.1/package')));
const version = JSON.parse(fs.readFileSync(path.join(runtime, 'package.json'), 'utf8')).version;
if (version !== '0.19.1') throw Error('Inspect renderer changes before changing the 0.19.1 pin');
const source = JSON.parse(fs.readFileSync(input, 'utf8'));
if (!Array.isArray(source.cases) || !source.cases.length) throw Error('Expected nonempty cases');
const ids = new Set();
for (const item of source.cases) {
  if (!/^katex-\d+$/.test(item.id) || ids.has(item.id) || typeof item.tex !== 'string') throw Error('Invalid case');
  ids.add(item.id);
}
fs.mkdirSync(output, {recursive: true});
const report = {renderer:'KaTeX', version, input_sha256:crypto.createHash('sha256').update(fs.readFileSync(input)).digest('hex'),
  settings:{throwOnError:true,strict:false,trust:false,base_font_px:48,css:'dist/katex.min.css'},
  deviations_from_upstream_harness:['Fixture pre/post HTML and styles are preserved in data but not executed or applied.',
    'trust is false rather than upstream true; untrusted HTML/URL commands are recorded.',
    'throwOnError is always true, including the deliberately invalid noThrow case.',
    'Upstream Mincho/Batang fixture fonts are not copied; installed fallback fonts may differ.'],
  cases:[],blocked_requests:[],browser_errors:[]};
const server = http.createServer((request,response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url,'http://localhost').pathname);
    if (pathname === '/') {
      response.writeHead(200,{'Content-Type':'text/html;charset=utf-8'});
      return response.end('<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/dist/katex.min.css"><style>body{margin:0;background:white;color:black;font:48px serif}#math{display:inline-block;width:max-content;padding:15px}.katex-display{margin:0}</style><div id="math"></div>');
    }
    const file = fs.realpathSync(path.resolve(runtime,'.'+pathname));
    if (!file.startsWith(runtime+path.sep) || !fs.statSync(file).isFile()) throw Error('Invalid resource');
    const type={'.js':'application/javascript','.css':'text/css','.woff2':'font/woff2','.woff':'font/woff','.ttf':'font/ttf'}[path.extname(file)] || 'application/octet-stream';
    response.writeHead(200,{'Content-Type':type});fs.createReadStream(file).pipe(response);
  } catch {response.writeHead(404);response.end();}
});
async function main() {
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const origin='http://127.0.0.1:'+server.address().port;
  const browser=await chromium.launch({channel:'msedge',headless:true});
  try {
    const page=await browser.newPage({viewport:{width:2200,height:1400}});
    page.on('pageerror',error=>report.browser_errors.push(String(error)));
    await page.route(/^https?:/,route=>{
      if(new URL(route.request().url()).origin===origin)return route.continue();
      report.blocked_requests.push(route.request().url());return route.abort();
    });
    await page.goto(origin);
    await page.addScriptTag({url:origin+'/dist/katex.min.js'});
    if(await page.evaluate(()=>katex.version)!==version)throw Error('Unexpected loaded version');
    for(const item of source.cases) {
      const result={id:item.id,name:item.name,tex:item.tex,options:item.options || {}};
      try {
        const render=await page.evaluate(async item=>{
          const target=document.querySelector('#math');target.replaceChildren();
          const warnings=[];
          const options={displayMode:Boolean(item.options?.displayMode),throwOnError:true,
            strict:(code,message)=>{warnings.push({code,message});return 'ignore';},
            trust:context=>{warnings.push({code:'untrustedCommand',command:context.command});return false;},
            maxExpand:1000,maxSize:100};
          if(item.options?.macros)options.macros={...item.options.macros};
          try{katex.render(item.tex,target,options);}catch(error){return {error:error.message,warnings};}
          await document.fonts.ready;
          if(target.querySelector('.katex-error'))return {error:'KaTeX error node',warnings};
          const bounds=target.getBoundingClientRect();
          if(bounds.width>20000||bounds.height>10000)throw Error('Rendered dimensions exceed review limits');
          return {width:Math.ceil(bounds.width),height:Math.ceil(bounds.height),warnings,html:target.innerHTML};
        },item);
        result.warnings=render.warnings;
        if(render.error)throw Error(render.error);
        // KaTeX may show rejected trusted commands as error-colored text while
        // returning success. Such output is retained but not called valid math.
        if(render.warnings.some(w=>w.code==='untrustedCommand')) {
          result.untrusted_commands=true;
          result.error='Untrusted HTML/URL command(s): '+[...new Set(render.warnings.filter(w=>w.code==='untrustedCommand').map(w=>w.command))].join(', ');
        }
        const bytes=await page.locator('#math').screenshot();
        fs.writeFileSync(path.join(output,item.id+'.png'),bytes);
        fs.writeFileSync(path.join(output,item.id+'.html'),render.html);
        Object.assign(result,{ok:!result.untrusted_commands,status:result.untrusted_commands?'untrusted_command':'rendered',image:item.id+'.png',image_sha256:crypto.createHash('sha256').update(bytes).digest('hex'),width:render.width,height:render.height});
      } catch(error) {Object.assign(result,{ok:false,status:'render_error',error:String(error.message||error)});}
      report.cases.push(result);
      fs.writeFileSync(path.join(output,'render-report.json'),JSON.stringify(report,null,2)+'\n');
      console.log(item.id,result.status,result.error||'');
    }
    report.summary={cases:report.cases.length,rendered:report.cases.filter(c=>c.ok).length,
      render_errors:report.cases.filter(c=>c.status==='render_error').length,untrusted:report.cases.filter(c=>c.untrusted_commands).length};
    fs.writeFileSync(path.join(output,'render-report.json'),JSON.stringify(report,null,2)+'\n');
    console.log(JSON.stringify(report.summary));
    if(report.blocked_requests.length||report.browser_errors.length)throw Error('Unexpected browser/network errors; inspect report');
  }finally{await browser.close();}
}
main().catch(error=>{console.error(error);process.exitCode=1;}).finally(()=>server.close());
