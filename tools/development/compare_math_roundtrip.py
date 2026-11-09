# SPDX-License-Identifier: AGPL-3.0-only
"""Compare newly captured native equations with the approved HTML originals."""
import argparse
import base64
from html.parser import HTMLParser
import io
import json
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw, ImageFont

ROOT=Path(__file__).resolve().parents[2]

class Originals(HTMLParser):
    def __init__(self):
        super().__init__();self.active=False;self.image=None;self.images={};self.title=False;self.heading='';self.identifier=None
    def handle_starttag(self,tag,attrs):
        if tag=='article':
            self.active=True;self.image=None;self.heading='';self.identifier=dict(attrs).get('id')
        if tag=='h2' and self.active:self.title=True
        if tag=='img' and self.active and self.image is None:
            self.image=dict(attrs).get('src')
    def handle_endtag(self,tag):
        if tag=='h2':self.title=False
        if tag=='article':
            if not self.image or not self.image.startswith('data:image/png;base64,'):
                raise ValueError('Missing embedded native original')
            ident=self.identifier.removeprefix('case-') if self.identifier else self.heading.split(' · ')[0].strip()
            if not ident or ident in self.images:raise ValueError('Missing or duplicate original case ID')
            self.images[ident]=base64.b64decode(self.image.split(',',1)[1]);self.active=False
    def handle_data(self,text):
        if self.title:self.heading+=text

def compare(folder):
    summary={}
    font=ImageFont.truetype('C:/Windows/Fonts/malgun.ttf',17)
    for name,filename,count in [('gallery','supported-equations.json',46),('toolbar','toolbar-equations.json',76)]:
        fixture=json.loads((ROOT/'crates/md2hwp-math/tests/fixtures'/filename).read_text(encoding='utf-8'))['cases']
        capture=json.loads((folder/name/'gallery.json').read_text(encoding='utf-8'))
        generated={c['case_id']:c for c in capture['cases']}
        originals=Originals();originals.feed((ROOT/'docs/math'/f'{name}.html').read_text(encoding='utf-8'))
        if len(originals.images)!=count or len(generated)!=count or len(fixture)!=count:
            raise ValueError(f'Incomplete {name} evidence')
        metrics=[];rows=[]
        for f in fixture:
            ident=f.get('case_id',f.get('id'));c=generated[ident];encoded=originals.images[ident]
            if c['original_script']!=f['hancom_script'] or not c['script_equal']:
                raise ValueError(f'Mismatched {name}/{ident} source or saved SCRIPT')
            a=Image.open(io.BytesIO(encoded)).convert('RGB')
            b=Image.open(folder/name/c['image']).convert('RGB')
            equal=a.size==b.size and ImageChops.difference(a,b).getbbox() is None
            metrics.append({'case_id':ident,'name':f['name'],'pixel_equal':equal,'original_size':a.size,'roundtrip_size':b.size})
            a.thumbnail((620,240));b.thumbnail((620,240));row=Image.new('RGB',(1320,max(a.height,b.height)+45),'white')
            draw=ImageDraw.Draw(row);draw.text((12,6),ident+' '+f['name']+(' [동일]' if equal else ' [차이]'),font=font,fill='#174a29' if equal else '#ad4c00')
            row.paste(a,(12,40));row.paste(b,(675,40));rows.append(row)
        out=folder/'comparison';out.mkdir(parents=True,exist_ok=True)
        for start in range(0,len(rows),10):
            selected=rows[start:start+10];sheet=Image.new('RGB',(1320,sum(r.height for r in selected)+36),'#eef2f5')
            ImageDraw.Draw(sheet).text((12,5),'왼쪽: 원래 한글 / 오른쪽: HML → TeX → HML 실제 한글',font=font,fill='black');y=36
            for row in selected:sheet.paste(row,(0,y));y+=row.height
            sheet.save(out/f'{name}-{start//10+1:02}.png')
        summary[name]={'count':count,'pixel_equal':sum(c['pixel_equal'] for c in metrics),'cases':metrics}
        print(name,summary[name]['pixel_equal'],'/',count,'native pixel equality',flush=True)
    (folder/'comparison.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    return summary

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--roundtrip-dir',type=Path,default=ROOT/'artifacts/math-roundtrip')
    compare(parser.parse_args().roundtrip_dir.resolve())
