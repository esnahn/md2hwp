# SPDX-License-Identifier: AGPL-3.0-only
"""Capture equations from an explicit JSON fixture in a separate owned document.

Accepts cases with script/case_id or hancom_script/id. Requires an existing
registered security module; never installs it or edits the registry.
"""
from __future__ import annotations
import argparse
import base64
from collections import Counter
from html import escape
import hashlib
import json
from pathlib import Path
import time
import winreg

from hancom_math_capture import json_bytes, write_atomic, process_table, process_creation, terminate_recorded_hwp, equation_script, native_process_state

ROOT = Path(__file__).resolve().parents[2]


def load_cases(input_path):
    """Validate explicit input and output-path components before creating COM."""
    raw = input_path.read_bytes()
    source = json.loads(raw.decode('utf-8-sig'))
    if not isinstance(source, dict):
        raise ValueError('Input must be an object containing cases.')
    cases = source.get('cases')
    if not isinstance(cases, list) or not cases:
        raise ValueError('Input must contain a nonempty cases array.')
    normalized = []
    seen = set()
    for original in cases:
        if not isinstance(original, dict):
            raise ValueError('Every case must be an object.')
        case_id = original.get('case_id', original.get('id'))
        script = original.get('script', original.get('hancom_script'))
        if not isinstance(case_id, str) or not case_id or case_id in {'.', '..'} or \
                case_id.endswith(('.', ' ')) or any(ord(c) < 32 or c in '\\/:*?"<>|' for c in case_id):
            raise ValueError('Each case requires a safe nonempty case_id or id.')
        if case_id.split('.', 1)[0].upper() in {'CON', 'PRN', 'AUX', 'NUL'} | \
                {f'{prefix}{n}' for prefix in ('COM', 'LPT') for n in range(1, 10)}:
            raise ValueError('Case ID must not be a Windows device name.')
        if case_id.casefold() in seen:
            raise ValueError(f'Duplicate case ID: {case_id}')
        if not isinstance(script, str) or not script.strip():
            raise ValueError(f'Case {case_id} requires nonempty script or hancom_script.')
        seen.add(case_id.casefold())
        name = original.get('name', case_id)
        category = original.get('category', '수식')
        if not isinstance(name, str) or not isinstance(category, str):
            raise ValueError('Case name and category must be strings.')
        normalized.append({**original, 'case_id': case_id, 'script': script,
                           'name': name, 'category': category})
    provenance = source.get('source')
    if provenance is not None:
        if not isinstance(provenance, dict) or not isinstance(provenance.get('path'), str) or \
                not isinstance(provenance.get('sha256'), str):
            raise ValueError('Optional source requires path and sha256 strings.')
        if hashlib.sha256(Path(provenance['path']).read_bytes()).hexdigest() != provenance['sha256']:
            raise ValueError('Input source resource has changed.')
    else:
        provenance = {'path': str(input_path.resolve()), 'sha256': hashlib.sha256(raw).hexdigest(),
                      'use': 'Explicit user-selected equation fixture.'}
    return {**source, 'source': provenance, 'cases': normalized}


def make_html(output, data):
    cards = []
    for item in data['cases']:
        image = output / item['image']
        encoded = base64.b64encode(image.read_bytes()).decode('ascii')
        text = (item['name'] + ' ' + item['script'] + ' ' + item['category']).lower()
        cards.append(f'''<article data-category="{escape(item['category'],quote=True)}" data-search="{escape(text,quote=True)}"><header>#{item['case_id']} · {escape(item['category'])} · <b>{escape(item['name'])}</b></header><div class="pair"><div class="image"><img alt="{escape(item['name'],quote=True)}" src="data:image/png;base64,{encoded}"></div><div><h3>한글 수식 코드</h3><pre>{escape(item['script'])}</pre><details><summary>생성된 개체의 실제 코드 · 일치 {item['script_equal']}</summary><pre>{escape(item['hancom_script'])}</pre></details></div></div></article>''')
    page = '''<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>한글 수식 실제 렌더링</title><style>*{box-sizing:border-box}body{margin:0;background:#f3f6f8;color:#172c3c;font:15px/1.6 system-ui,'Malgun Gothic',sans-serif}main{max-width:1400px;margin:auto;padding:24px}h1{font-size:26px}nav{position:sticky;top:0;background:#fffef5;padding:13px;border:1px solid #d9e2e8;z-index:2;display:flex;gap:12px;align-items:center;flex-wrap:wrap}input,select{padding:7px;font:inherit;border:1px solid #b5c5ce;border-radius:5px}article{background:white;border:1px solid #d9e2e8;border-radius:8px;margin:20px 0;content-visibility:auto;contain-intrinsic-size:auto 260px}header{padding:12px 18px;border-bottom:1px solid #d9e2e8}.pair{display:grid;grid-template-columns:1fr 1fr;gap:20px;padding:22px}.image{display:flex;align-items:center;justify-content:center;min-height:120px;overflow:auto}img{max-width:100%;height:auto}pre{font:14px/1.6 Consolas,monospace;white-space:pre-wrap;overflow-wrap:anywhere;padding:14px;background:#f5f7f9}h3{font-size:13px;color:#627784}summary,.note{font-size:12px;color:#627784}summary{cursor:pointer}[hidden]{display:none!important}@media(max-width:750px){.pair{grid-template-columns:1fr}nav{position:static}}</style><main><h1>한글 수식 실제 렌더링</h1><p>@@COUNT@@개 · 일반 11 / 분수·지수 10 / 도형 11 / 기타 공식 13</p><p class="note">명시적으로 선택한 JSON 입력의 수식 코드를 별도 한글 문서에 직접 삽입하여 그린 실제 결과입니다. HancomEQN 14pt · 300dpi · 수식 주위 여백만 잘라냄. 외부 JavaScript나 인터넷 연결이 필요 없습니다.</p><nav><input id="q" placeholder="이름 · 수식 코드 검색"><select id="category"><option value="">전체</option><option>일반</option><option>분수/지수</option><option>도형</option><option>기타 공식</option></select><span id="count">@@COUNT@@개 표시</span></nav>@@CARDS@@<details><summary>원본 리소스·렌더링 환경</summary><pre>@@META@@</pre></details></main><script>function filter(){const q=document.querySelector('#q').value.toLowerCase(),c=document.querySelector('#category').value;let n=0;document.querySelectorAll('article').forEach(a=>{a.hidden=!(a.dataset.search.includes(q)&&(!c||a.dataset.category===c));if(!a.hidden)n++;});document.querySelector('#count').textContent=n+'개 표시';}document.querySelector('#q').addEventListener('input',filter);document.querySelector('#category').addEventListener('change',filter);</script></html>'''
    page = page.replace('@@COUNT@@',str(len(cards))).replace('@@CARDS@@','\n'.join(cards)).replace('@@META@@',escape(json.dumps({key:value for key,value in data.items() if key!='cases'},ensure_ascii=False,indent=2)))
    page = page.replace('<option>일반</option><option>분수/지수</option><option>도형</option><option>기타 공식</option>', ''.join(f'<option>{escape(name)}</option>' for name in dict.fromkeys(item['category'] for item in data['cases'])))
    page = page.replace('일반 11 / 분수·지수 10 / 도형 11 / 기타 공식 13', ' / '.join(f'{escape(name)} {count}' for name,count in Counter(item['category'] for item in data['cases']).items()))
    write_atomic(output / 'gallery.html',page.encode('utf-8'))


def run(input_path, output, limit=None, continue_empty=False, continue_errors=False):
    source = load_cases(input_path)
    if limit is not None and limit <= 0:
        raise ValueError('Limit must be positive.')
    import win32com.client
    import win32process
    from PIL import Image, ImageChops
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER,r'Software\HNC\HwpAutomation\Modules') as key:
        module = winreg.QueryValueEx(key,'FilePathCheckerModuleExample')[0]
    if not Path(module).is_file():
        raise RuntimeError('Registered security module missing.')
    output.mkdir(parents=True,exist_ok=True)
    before = process_table()
    hwp = win32com.client.DispatchEx('HWPFrame.HwpObject')
    pid = win32process.GetWindowThreadProcessId(hwp.XHwpWindows.Active_XHwpWindow.WindowHandle)[1]
    if pid in before:
        raise RuntimeError('Refusing to edit or close a preexisting HWP instance.')
    stamp = process_creation(pid)
    data = {'count':0,'source':source['source'],'rendering':{'font':'HancomEQN','point_size':14,'dpi':300,
            'owned_pid':pid,'owned_creation':stamp,'preexisting_hwp_pids':[p for p,n in before.items() if n in {'hwp.exe','hwp64.exe'}]},'cases':[]}
    write_atomic(output/'render-owner.json',json_bytes(data['rendering']))
    try:
        if not hwp.RegisterModule('FilePathCheckDLL','FilePathCheckerModuleExample'):
            raise RuntimeError('RegisterModule failed.')
        data['rendering']['hancom_version'] = str(hwp.Version)
        hwp.XHwpWindows.Active_XHwpWindow.Visible = False
        bmp=output/'render-page.bmp'
        for original in source['cases'][:limit]:
            if not native_process_state(pid,stamp)['alive']:
                raise RuntimeError('Owned Hancom process exited.')
            try:
                folder = output/'cases'/original['case_id']
                folder.mkdir(parents=True,exist_ok=True)
                hwp.Clear(1)
                params=hwp.HParameterSet.HEqEdit
                hwp.HAction.GetDefault('EquationCreate',params.HSet)
                params.string=original['script']
                params.BaseUnit=hwp.PointToHwpUnit(14)
                params.EqFontName='HancomEQN'
                if not hwp.HAction.Execute('EquationCreate',params.HSet):
                    raise RuntimeError(f'EquationCreate failed: {original["case_id"]}')
                xml=hwp.GetTextFile('HWPML2X','')
                write_atomic(folder/'equation.xml',xml.encode('utf-8'))
                actual=equation_script(xml)
                bmp=output/'render-page.bmp'
                if not hwp.CreatePageImage(str(bmp),0,300,24,'bmp'):
                    raise RuntimeError(f'Page rendering failed: {original["case_id"]}')
                if hwp.PageCount != 1:
                    raise RuntimeError('Equation does not fit one page.')
                with Image.open(bmp) as page:
                    rgb=page.convert('RGB')
                    bounds=ImageChops.difference(rgb,Image.new('RGB',rgb.size,'white')).getbbox()
                    if not bounds:
                        if continue_empty:
                            data.setdefault('empty_renders',[]).append({**original,'hancom_script':actual})
                            write_atomic(output/'gallery.json',json_bytes(data))
                            print(original['case_id'],'EMPTY (recorded)',flush=True)
                            continue
                        raise RuntimeError('Empty equation render.')
                    rect=(max(0,bounds[0]-15),max(0,bounds[1]-15),min(rgb.width,bounds[2]+15),min(rgb.height,bounds[3]+15))
                    rgb.crop(rect).save(folder/'equation.png')
                item={**original,'hancom_script':actual,'script_equal':actual==original['script'],
                      'image':(folder/'equation.png').relative_to(output).as_posix(),'crop_rect':rect,
                      'image_sha256':hashlib.sha256((folder/'equation.png').read_bytes()).hexdigest()}
                write_atomic(folder/'case.json',json_bytes(item))
                data['cases'].append(item)
                data['count']=len(data['cases'])
                write_atomic(output/'gallery.json',json_bytes(data))
                print(original['case_id'],original['name'],flush=True)
            except Exception as error:
                if not continue_errors or not native_process_state(pid,stamp)['alive']:
                    raise
                data.setdefault('failures',[]).append({**original,'error':str(error)})
                write_atomic(output/'gallery.json',json_bytes(data))
                print(original['case_id'],'FAILED (recorded)',str(error),flush=True)
        bmp.unlink(missing_ok=True)
        make_html(output,data)
    finally:
        try:
            hwp.Clear(1)
            hwp.Quit()
        except Exception:
            terminate_recorded_hwp(pid,stamp)
        deadline=time.monotonic()+5
        while native_process_state(pid,stamp)['alive'] and time.monotonic()<deadline:
            time.sleep(.1)
        if native_process_state(pid,stamp)['alive']:
            terminate_recorded_hwp(pid,stamp)
        write_atomic(output/'render-cleanup.json',json_bytes({'owned_pid':pid,'owned_process_alive':native_process_state(pid,stamp)['alive']}))
    return {'count':data['count'],'script_equal':sum(c['script_equal'] for c in data['cases']),
            'html':str((output/'gallery.html').resolve())}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input',type=Path,required=True,help='Explicit JSON cases fixture (script/case_id or hancom_script/id).')
    parser.add_argument('--output',type=Path,default=ROOT/'artifacts/math-native-capture')
    parser.add_argument('--limit',type=int)
    parser.add_argument('--continue-empty',action='store_true',help='Record invisible native equations and continue; default is strict failure.')
    parser.add_argument('--continue-errors',action='store_true',help='Record per-equation failures while the owned Hancom process remains alive; default is strict failure.')
    args=parser.parse_args()
    print(json.dumps(run(args.input.resolve(),args.output.resolve(),args.limit,args.continue_empty,args.continue_errors),ensure_ascii=False))


if __name__=='__main__':
    main()
