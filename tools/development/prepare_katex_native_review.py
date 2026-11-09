# SPDX-License-Identifier: AGPL-3.0-only
"""Prepare native captures and HML re-export for the pinned KaTeX fixtures."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'artifacts/katex-math-review'
def write(name, value):
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')

source = json.loads((ROOT/'tests/katex/cases.json').read_text(encoding='utf-8'))
converted = json.loads((OUT/'conversions.json').read_text(encoding='utf-8'))
assert len(converted['cases']) == len(source['cases'])
cases = []
for fixture, result in zip(source['cases'], converted['cases']):
    assert result['input'] == fixture['tex'] and result['source_record'] == fixture
    if result['status'] == 'converted':
        cases.append({'case_id': fixture['id'], 'name': fixture['name'],
                      'script': result['output'], 'hancom_script': result['output'],
                      'original_tex': fixture['tex']})
write('native-input.json', {'cases': cases})
exe = ROOT/'target/debug/md2hwp-math.exe'
subprocess.run([str(exe), 'batch', 'hml-to-tex', str(OUT/'native-input.json'), str(OUT/'reexport.json')], check=True)
write('converter-provenance.json', {'path': str(exe.relative_to(ROOT)),
      'sha256': hashlib.sha256(exe.read_bytes()).hexdigest(),
      'source_sha256': {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in sorted((ROOT/'crates/md2hwp-math/src').glob('*.rs'))}})
print('Native cases:', len(cases))
