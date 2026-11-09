# SPDX-License-Identifier: AGPL-3.0-only
"""Build a standalone TeX/native-SCRIPT comparison from explicit capture results."""
import base64
from collections import Counter
from html import escape
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT/'artifacts/katex-math-review'
SUPPORT_EXCLUSIONS = {
    'katex-016': '이 전체 예제는 보험수리 표기의 위·오른쪽 테두리인 angl 때문에 지원 대상에서 제외합니다. 일반 boxed는 계속 지원하며, angl을 ANGLE이나 분리된 선으로 대체하지 않습니다.',
    'katex-018': 'CD 도식 환경은 지원 대상에서 제외합니다. 원문과 변환 오류를 유지합니다.',
    'katex-038': '사용자 정의 매크로와 외부 매크로 설정은 지원 대상에서 제외합니다. 매크로를 실행하거나 원문 명령을 삭제하지 않습니다.',
    'katex-047': 'rule로 실제 직사각형을 그리는 예제이므로 지원 대상에서 제외합니다. 분자에 있는 파란 사각형을 생략하지 않습니다.',
    'katex-046': 'Lap 계열은 내부 내용을 일반 배치로 변환합니다. 이 전체 예제에는 아직 미지원인 phantom이 남아 있어 지원 대상에서 제외하며 원문과 오류를 유지합니다.',
    'katex-093': '사용자 방침에 따라 raisebox 등 수직 이동 조판 예제는 지원 대상에서 제외합니다. 내용을 임의 배치로 바꾸지 않습니다.',
    'katex-095': '사용자 방침에 따라 reflectbox/mathreflectbox 반사 조판 예제는 지원 대상에서 제외합니다.',
    'katex-052': 'rule은 실제 사각형을 그리므로 지원 대상에서 제외합니다. 크기 지시자로 취급하거나 조용히 지우지 않습니다. 보이지 않는 닫힘 구분자 right. 자체는 RIGHT .으로 지원하지만 이 전체 예제는 rule 때문에 미지원입니다.',
    'katex-098': '예제 전체가 rule로 크기가 다른 실제 직사각형을 그리는 시험이므로 지원 대상에서 제외합니다. 크기 지시자나 빈칸으로 바꾸거나 조용히 지우지 않습니다.',
    'katex-121': '사용자 방침에 따라 verb는 미지원으로 결정했으므로 이 전체 예제를 지원 대상에서 제외합니다. 앞의 texttt와 텍스트 공백 행은 지원하지만, 마지막 verb 행을 text로 바꾸거나 지우지 않습니다.',
    'katex-127': '사용자 방침에 따라 verb와 verb*는 미지원으로 결정했으므로 지원 대상에서 제외합니다. 리터럴 공백이나 가시적 공백 기호를 임의로 재현하지 않습니다.',
    'katex-123': '사용자 방침에 따라 text 안의 $...$ 수식과 텍스트 서체 범위를 오가는 처리는 미지원으로 결정했으므로 지원 대상에서 제외합니다. 내부 수식을 일반 텍스트로 바꾸거나 지우지 않습니다.',
    'katex-088': '내용을 숨기는 phantom·hphantom·vphantom은 미지원으로 결정했으므로 지원 대상에서 제외합니다. 숨김 내용을 빈칸으로 바꾸거나 지우지 않습니다. Lap·smash의 일반 배치 정규화와는 구별합니다.',
    'katex-126': '의도적으로 알 수 없는 명령을 넣은 오류 시험이므로 지원 대상에서 제외합니다. 변환 오류가 정상 결과이며 명령을 다른 기호로 바꾸지 않습니다.',
    'katex-048': 'LaTeX·TeX 로고 조판 예제는 지원 대상에서 제외합니다. 로고 명령을 빈 문자열로 지우지는 않습니다.',
    'katex-042': '수식 내부 그림 삽입은 지원 대상에서 제외합니다. 원본의 includegraphics와 사용자 정의 매크로는 변환하지 않습니다.',
    'katex-040': '수평 대괄호 모양에 직접 대응하는 한글 native 수식 명령이 없어 지원 대상에서 제외합니다. OVERBRACE로 대체하지 않습니다.',
    'katex-041': 'HTML/CSS 확장은 수식 변환 지원 대상에서 제외합니다. 이 예제는 의도적 문법 오류가 아니며, KaTeX의 trust=false 설정에 따른 HTML 확장 거부와 HML 변환 미지원은 별도로 기록합니다.',
    'katex-044': 'KaTeX 제품 로고 명령은 수식 변환 지원 대상에서 제외합니다. 원문이나 알 수 없는 명령을 삭제하지 않습니다.',
}
def read(path):
    return json.loads(path.read_text(encoding='utf-8'))
def image(path, alt):
    raw = path.read_bytes()
    if raw[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('Not a PNG: '+str(path))
    return '<img alt="'+escape(alt, quote=True)+'" src="data:image/png;base64,'+base64.b64encode(raw).decode('ascii')+'">'
def pre(value):
    return '<pre>'+escape(str(value))+'</pre>'

fixture = read(ROOT/'tests/katex/cases.json')
converted = read(OUT/'conversions.json')
tex_report = read(OUT/'tex/render-report.json')
tex = {c['id']:c for c in tex_report['cases']}
native_report = read(OUT/'native/gallery.json')
native = {c['case_id']:{**c,'state':'captured'} for c in native_report['cases']}
for key,state in [('failures','error'),('empty_renders','empty')]:
    for c in native_report.get(key,[]):
        assert c['case_id'] not in native
        native[c['case_id']] = {**c,'state':state}
cleanup = read(OUT/'native/render-cleanup.json')
assert not cleanup['owned_process_alive']
reexport = {c['source_record']['case_id']:c for c in read(OUT/'reexport.json')['cases']}
assert len(fixture['cases'])==len(converted['cases'])==len(tex)==128
source = fixture['source']
assert hashlib.sha256((OUT/'upstream/ss_data.yaml').read_bytes()).hexdigest()==source['sha256']
cards=[];records=[]
for f,c in zip(fixture['cases'], converted['cases']):
    ident=f['id'];name=f['name']; t=tex[ident]
    assert c['input']==f['tex'] and c['source_record']==f
    assert t['tex'] == f['tex'] and t['options'] == f['options']
    generated=c['status']=='converted'
    n=native.get(ident)
    if generated:
        assert n is not None and n['script']==c['output']
    else:
        assert n is None
    native_ok=bool(n and n['state']=='captured')
    state='native' if native_ok else ('native-error' if generated else 'unsupported')
    policy={'excluded':ident in SUPPORT_EXCLUSIONS,'reason':SUPPORT_EXCLUSIONS.get(ident)}
    if policy['excluded']:
        assert state == 'unsupported'
    record={**f,'conversion':{'ok':generated,'output':c['output'],'error':c['error']},'tex_render':t,
            'native_render':n,'reexport':reexport.get(ident),'state':state,'support_policy':policy}
    records.append(record)
    tex_image=t.get('image')
    left=image(OUT/'tex'/tex_image, name+' — KaTeX') if tex_image else ''
    if not t['ok']:
        left+='<p class="error">KaTeX: '+escape(t['status'])+'</p>'+pre(t.get('error') or json.dumps(t.get('warnings',[]),ensure_ascii=False,indent=2))
    right=''
    if native_ok:
        assert n['script_equal'] and n['hancom_script']==c['output']
        path=OUT/'native'/n['image']
        assert hashlib.sha256(path.read_bytes()).hexdigest()==n['image_sha256']
        right=image(path,name+' — 한글')
    elif generated:
        right='<p class="error">한글 렌더링 '+('빈 이미지' if n['state']=='empty' else '실패')+'</p>'+pre(n.get('error','표시할 내용 없음'))
    else:
        right='<p class="error">HML 변환 미지원</p>'+pre(c['error'])
    if policy['excluded']:
        right='<p><strong>지원 대상 제외</strong> · '+escape(policy['reason'])+'</p>'+right
    warning=''
    if ident == 'katex-067':
        warning += '<p>그룹 또는 명시적 limits가 없는 mathop은 내부 기호·내용을 일반 기호로 표시합니다. 연산자 클래스의 자동 간격은 재현하지 않습니다.</p>'
    if ident == 'katex-072':
        warning += '<p>matrix*/pmatrix*의 l/c/r 열 정렬 옵션은 무시하고 기존 matrix/pmatrix로 정규화합니다.</p>'
    if ident in ('katex-069', 'katex-070', 'katex-083'):
        warning += '<p>사용자 방침에 따라 sans-serif는 별도 서체로 표현하지 않습니다. roman/italic과 굵기·고정폭 등 지원되는 속성만 유지합니다.</p>'
    if ident == 'katex-081':
        warning += '<p>not abc는 첫 글자 a에 사선을 그리고 bc를 남기는 native 동작입니다. 단순 영문 그룹을 허용하지만 임의 복합식 전체 취소선으로 확장하지 않습니다.</p>'
    if ident in ('katex-108', 'katex-109'):
        warning += '<p>근사 변환: cancel·bcancel·xcancel·sout은 모두 한글 not의 한 글자 사선으로 바꿉니다. 여러 글자는 첫 글자에만 사선이 그어지고, 역사선·X자·가로줄 구별은 사라집니다. 한글은 int·oint 등 적분 명령어 위에 사선을 그리지 않으므로 사용자 결정에 따라 ∫·∬·∭·∮·∯·∰ 문자로 바꿉니다. 이 적분은 일반 적분 기호보다 작게 표시되며, 첨자가 있으면 사선이 뒤의 항까지 길게 지나갑니다.</p>'
    if ident == 'katex-122':
        warning += '<p>textbf는 native bold, textit은 native it으로 옮기고 중첩 범위가 끝나면 이전 글꼴로 되돌립니다. 한글 it은 숫자를 기울이지 않으므로 이탤릭·굵은 이탤릭 범위의 123은 KaTeX와 달리 똑바로 표시됩니다. 사용자 방침에 따라 sans-serif는 로만으로 표시하며, textit 안의 textsf는 KaTeX처럼 똑바른 글꼴로 돌아갑니다. 역변환은 mathrm·mathbf·mathit·boldsymbol의 수학 서체 명령을 사용합니다.</p>'
    if ident == 'katex-076':
        warning += '<p>mathbin의 명시적 중간 간격(4mu)을 display/text 스타일에 적용하고 기존 native 공백 단위로 근사합니다. script 스타일에서는 생략합니다.</p>'
    if ident == 'katex-077':
        warning += '<p>기존 음수 kern 처리 방침에 따라 음수 얇은 공백은 0으로 정규화합니다.</p>'
    if ident == 'katex-080':
        warning += '<p>일반 수학 그룹 안의 단순 줄바꿈은 TeX의 가로 상자 조판에 맞춰 한 줄로 처리합니다. 직접 지원하는 직전 glue는 unskip으로 제거하고 kern은 남깁니다. 바깥 줄바꿈과 명시적 행렬·정렬의 행 구분은 유지합니다.</p>'
    if ident in ('katex-084', 'katex-085'):
        warning += '<p>기본 displaylimits 연산자는 text 스타일에서 첨자를 오른쪽에, display 스타일에서는 위·아래에 둡니다. 명시적 limits/nolimits가 우선합니다. 일반 int와 intop의 기본 배치는 구별합니다.</p>'
    if ident == 'katex-089':
        warning += '<p>사용자 방침에 따라 phase는 각도 기호와 본문만 표시하고 긴 밑줄은 생략합니다. <a href="phase-options.html">검토했던 대안 비교</a>도 남겼습니다.</p>'
    if ident == 'katex-094':
        warning += '<p>사용자 결정에 따라 xrightleftarrows·xrightequilibrium·xleftequilibrium은 모두 native rel EXARROW로 변환합니다. 비대칭 작살표 두 문자(U+1F8D2/U+1F8D3)는 Unicode 17.0에 있지만 현재 한글 글꼴에서 표시되지 않습니다. 주석은 유지하지만 원래 화살표 모양과 긴/짧은 선 구별은 사라집니다.</p>'
    if ident in ('katex-097', 'katex-101'):
        warning += '<p>겹침·폭·높이 조정 명령은 생략하고 내부 수식은 일반 배치로 보존합니다. phantom처럼 내용을 숨기는 명령까지 없애는 정책은 아닙니다.</p>'
    if ident == 'katex-090':
        warning += '<p>pmb의 겹쳐 찍기는 일반 수학 bold로 정규화합니다. 원래의 중첩 인쇄 효과는 재현하지 않습니다.</p>'
    if ident == 'katex-087':
        warning += '<p>longrightarrow는 기존 rarrow로 정규화하며 원래 고정 길이의 차이는 유지하지 않습니다.</p>'
    if ident in ('katex-061', 'katex-063', 'katex-064', 'katex-066', 'katex-071'):
        warning += '<p>수식 구조를 유지하며 지원되는 글자는 native 또는 Unicode 수학 글꼴로 옮깁니다. 대응하지 않는 글자·숫자는 기본 글꼴로 남으며, Unicode 글꼴의 세부 모양은 KaTeX와 다를 수 있습니다.</p>'
    if ident == 'katex-059':
        warning += '<p>TeX 스타일에 맞는 분기만 남깁니다. 선택 결과는 D T S SS X의 첨자 S, 그 첨자 SS입니다. 역변환은 선택된 일반 수식이며 mathchoice를 복원하지 않습니다.</p>'
    if ident in ('katex-060', 'katex-062', 'katex-065'):
        warning += '<p>breve는 가능한 경우 미리 조합된 Unicode 문자를 사용하며, 없으면 독립 기호 ˘를 본체 뒤에 놓습니다. 결합형과 underover는 출력하지 않습니다.</p>'
    if ident in ('katex-106', 'katex-107'):
        warning += '<p>근사 변환: 왼쪽·이중 화살표와 작살표·수평선은 underover 위쪽의 고정 폭 기호입니다. 선분 끝획은 생략하고, 위쪽 곡선·넓은 모자·물결·체크는 한글 기본 장식으로 정규화합니다.</p>'
    if f.get('original_tex'):
        warning += '<details><summary>원본에서 변경한 부분: '+escape(f['adaptation_reason'])+'</summary>'+pre(f['original_tex'])+'</details>'
    if ident == 'katex-054':
        warning += '<p>표시 한계: 원본 전체는 한 줄 수식으로 변환되며, 한글 페이지 이미지에서 오른쪽이 잘립니다. 자동 줄 나눔 제약은 보존하지 않습니다.</p>'
    if ident == 'katex-055':
        warning += '<p>근사 변환: 아래 화살표·곡선·물결·수평선은 underover의 아래 첨자 위치에 놓는 고정 폭 기호입니다. 본체 폭에 맞춰 늘어나지 않으며, underlinesegment의 양 끝 세로획은 생략합니다. ⎵는 원본과 다른 모양이므로 사용하지 않습니다.</p>'
    if f.get('upstream_options'):
        warning='<details><summary>원본 테스트 옵션</summary>'+pre(json.dumps(f['upstream_options'],ensure_ascii=False,indent=2))+'</details>'
    export=''
    if ident in reexport:
        r=reexport[ident]
        export='<details><summary>HML → TeX 재출력'+(' 오류' if r['error'] else '')+'</summary>'+pre(r['error'] or r['output'])+'</details>'
    label={'native':'한글 렌더링 있음','native-error':'한글 렌더링 실패','unsupported':'HML 미지원'}[state]
    search=(ident+' '+name+' '+f['tex']+' '+(c['output'] or '')+' '+(policy['reason'] or '')).lower()
    cards.append('<article id="'+ident+'" data-state="'+state+'" data-tex-error="'+str(not t['ok']).lower()+'" data-reexport-error="'+str(bool(reexport.get(ident,{}).get('error'))).lower()+'" data-search="'+escape(search,quote=True)+'"><header><h2>'+ident+' · '+escape(name)+'</h2><span class="badge '+state+'">'+label+'</span></header><div class="compare"><section><h3>TeX · KaTeX</h3><div class="render">'+left+'</div>'+pre(f['tex'])+'</section><section><h3>HML · 한글 수식 SCRIPT</h3><div class="render">'+right+'</div>'+(pre(c['output']) if generated else '')+export+'</section></div>'+warning+'</article>')

summary={'total':len(records),'hml_generated':sum(r['conversion']['ok'] for r in records),
         'hml_unsupported':sum(not r['conversion']['ok'] for r in records),
         'native_rendered':sum(r['state']=='native' for r in records),
         'native_render_failed':sum(r['state']=='native-error' for r in records),
         'tex_rendered':sum(r['tex_render']['ok'] for r in records),
         'tex_special_errors':sum(not r['tex_render']['ok'] for r in records),
         'reexport_errors':sum(bool(r['reexport'] and r['reexport']['error']) for r in records)}
result={'source':source,'summary':summary,'converter':read(OUT/'converter-provenance.json'),
        'tex_environment':{k:v for k,v in tex_report.items() if k!='cases'},
        'native_environment':native_report['rendering'],'cases':records}
(OUT/'report.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
license_text=(ROOT/'tests/katex/LICENSE').read_text(encoding='utf-8')
body='''<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>KaTeX 테스트 수식 — TeX와 HML 비교</title>
<style>*{box-sizing:border-box}body{background:#f1f4f7;color:#192b3b;font:15px/1.6 system-ui,"Malgun Gothic",sans-serif;margin:0}main{max-width:1480px;margin:auto;padding:24px}h1{font-size:27px}h2{font-size:19px;margin:0}h3{font-size:16px}nav{position:sticky;top:0;z-index:5;display:flex;gap:12px;flex-wrap:wrap;padding:14px;background:#fff;border:1px solid #c9d7e2;border-radius:8px}input,select{font:inherit;padding:7px;max-width:100%}input{min-width:260px}article{padding:20px;background:white;border:1px solid #d4dee7;border-radius:10px;margin-top:18px;scroll-margin-top:100px}header{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap}.compare{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:20px}section{min-width:0}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f6f8fa;padding:12px;border:1px solid #e0e6ed;border-radius:5px;font:13px/1.6 Consolas,monospace}.render{overflow:auto;min-height:85px;padding:12px;border:1px solid #e0e6ed;background:white}.render img{max-width:none;display:block;max-height:460px;width:auto;height:auto}.error{color:#9c3131}.badge{padding:3px 10px;border-radius:16px;background:#eee;white-space:nowrap}.native{background:#e2f2e7;color:#205d39}.unsupported,.native-error{background:#fbe8dc;color:#88441c}summary{cursor:pointer;padding:9px 0}[hidden]{display:none!important}@media(max-width:800px){main{padding:12px}.compare{grid-template-columns:1fr}.render img{max-width:100%;height:auto;max-height:none}}
</style><main><h1>KaTeX 테스트 수식 — TeX와 HML 비교</h1>
<p>KaTeX 시각 테스트 128개를 수록했습니다. 060번 이후 승인된 로고 항 제거는 각 예제에 원문과 함께 표시합니다. 왼쪽은 KaTeX 렌더링, 오른쪽은 현재 변환기로 만든 HML 코드와 한글 2020 렌더링입니다. 그림의 크기·서체는 서로 다릅니다. 렌더링 성공이 시각적 일치를 뜻하지는 않습니다.</p>
<p>@@SUMMARY@@</p><p>스타일 크기 선언은 HML의 scale 70·50·100으로 보존합니다. 현재 한글에서는 SCALE에 따른 실제 크기 변화가 관찰되지 않아 TeX와 표시 크기가 다를 수 있습니다.</p><p>매크로는 KaTeX 쪽 테스트 설정에만 적용되며 HML 변환기에는 원래 TeX를 그대로 전달합니다. pre/post HTML과 원본 CSS는 실행하지 않습니다. 오류 표시를 시험하는 noThrow 항목도 엄격한 렌더링 오류로 기록합니다. 세부 차이는 원본 옵션과 렌더링 환경에 남겼습니다.</p>
<nav><input id="query" type="search" placeholder="이름 · TeX · HML 검색" aria-label="수식 검색"><select id="filter" aria-label="결과 필터"><option value="all">전체</option><option value="native">한글 렌더링 있음</option><option value="unsupported">HML 미지원</option><option value="native-error">한글 렌더링 실패</option><option value="tex-error">KaTeX 렌더링 오류</option><option value="reexport-error">HML 재출력 오류</option></select><span id="count" aria-live="polite"></span></nav>
@@CARDS@@
<details><summary>출처 · 라이선스 · 실행 환경</summary><p>KaTeX 테스트는 MIT 라이선스이며 아래 원문 고지를 유지합니다.</p><p><a href="@@SOURCE_URL@@">원본 고정 커밋</a></p>@@METADATA@@<pre>@@LICENSE@@</pre></details></main>
<script>const cards=[...document.querySelectorAll('article')];function apply(){const q=document.querySelector('#query').value.toLowerCase(),f=document.querySelector('#filter').value;let n=0;for(const c of cards){const state=f==='all'||(f==='tex-error'?c.dataset.texError==='true':f==='reexport-error'?c.dataset.reexportError==='true':c.dataset.state===f);c.hidden=!(state&&c.dataset.search.includes(q));if(!c.hidden)n++;}document.querySelector('#count').textContent=n+' / '+cards.length+'개 표시';}document.querySelector('#query').addEventListener('input',apply);document.querySelector('#filter').addEventListener('change',apply);apply();</script></html>'''
body=body.replace('@@SUMMARY@@',f"전체 <b>{summary['total']}</b> · HML 생성 <b>{summary['hml_generated']}</b> · HML 미지원 <b>{summary['hml_unsupported']}</b> · 한글 렌더링 <b>{summary['native_rendered']}</b> · HML 재출력 오류 <b>{summary['reexport_errors']}</b>")
body=body.replace('@@CARDS@@','\n'.join(cards)).replace('@@SOURCE_URL@@',escape(source['url'],quote=True))
body=body.replace('@@METADATA@@',pre(json.dumps({k:v for k,v in result.items() if k!='cases'},ensure_ascii=False,indent=2))).replace('@@LICENSE@@',escape(license_text))
destination=ROOT/'docs/math/katex.html'
destination.write_text(body,encoding='utf-8')
print(json.dumps({'path':str(destination),'summary':summary},ensure_ascii=False))
