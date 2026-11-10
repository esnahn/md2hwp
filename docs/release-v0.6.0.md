# md2hwp v0.6.0

Markdown 원고를 HWP 문서로 변환합니다.
AURI 보고서 양식의 서식과 문서 구조를 구현·출력하는 것을 기준으로 합니다.
프로그램 버전은 **0.6.0**, IR은 **0.6**입니다. IR 버전은 편의상 프로그램 버전에 맞춰 0.4에서 0.6으로 올렸고, 따라서 IR 0.5는 없습니다. 프로그램과 IR은 계속 독립적으로 버전을 관리합니다.

## 이번 버전의 변경사항

- Markdown 원고에 포함된 LaTeX 수식을 한글 수식으로 변환하여 입력합니다.
- 취소선과 위·아래 첨자를 지원합니다.
- 기존 한글 실행 중인 상태에서 한컴 백엔드를 사용하여 HWP 문서 변환이 가능합니다. `hwp2pdf.exe`와 새 `hwpeqrecalc.exe`도 같습니다. (rhwp 백엔드는 원래도 가능하였음)

### 버그 수정

- rhwp 백엔드가 그림(PNG·JPG·EMF)의 원본 크기 값을 한글과 같은 방식으로 저장합니다. 표시 크기는 바뀌지 않으며, 한글이 문서를 다시 조판할 때 이 값을 바꾸던 문제가 없어졌습니다.
- (내부) 표가 있는 HWP 변환 결과에서 쪽 이미지가 셀 하나만 나오던 문제를 수정.

### 수식

- 문장 안 수식은 `$...$`, 별도 문단 수식은 문단 전체를 `$$...$$`로 씁니다. 지원하는 TeX 범위와 변환 규칙은 [수식 변환 안내](https://github.com/esnahn/md2hwp/blob/v0.6.0/docs/math.md)에 있습니다.
- 문장 안 수식은 본문·목록 문단과 그 각주 본문에 쓸 수 있습니다. 제목·표 셀·캡션·대체 텍스트·출처·링크 글자의 수식과 목록 항목 안의 별도 문단 수식은 명시적으로 거부합니다.
- 여는 `$`의 오른쪽에는 빈칸이 아닌 글자가 있어야 합니다. 닫는 `$`는 왼쪽에 빈칸이 아닌 글자가 있어야 하고, 오른쪽에는 숫자가 올 수 없습니다. 따라서 금액에 붙는 `$`는 일관된 양식으로 작성된다면 문제가 없지만, 필요하다면 `\$`로 이스케이프하여 적습니다.
- 문장 안 수식은 속한 문단 스타일의 글자 크기를 쓰고 개체 번호가 없습니다.
- 별도 문단 수식은 템플릿 표본 문단을 따르고, 수식 글자 크기는 `display_math.fontsize-pt`, 번호는 수식 번호(AUTONUM)를 씁니다.
- 두 백엔드 모두 수식을 생성합니다. 한글 백엔드는 한글이 조판한 크기를 그대로 씁니다. rhwp 백엔드는 한글 2020 실측에 맞춘 계산기로 수식의 폭·높이·기준선을 추정합니다. 수식 글꼴은 두 백엔드 모두 `HancomEQN`입니다.

```markdown
피타고라스 정리는 $a^2+b^2=c^2$이다.

$$
\int_0^1 x^2\,dx = \frac{1}{3}
$$
```

> 피타고라스 정리는 $a^2+b^2=c^2$이다.
>
> ```math
> \int_0^1 x^2\,dx = \frac{1}{3}
> ```
[^1]

[^1]: GitHub는 이스케이프되지 않은 `\,`을 `,`으로 해석하기 때문에, 미리보기를 위해 ` ```math … ``` ` 문법 사용. 실제 pandoc에서는 정상 표현됨.


설계와 한글 실측 근거는 [수식의 IR 표현](https://github.com/esnahn/md2hwp/blob/v0.6.0/docs/math/ir-design.md)에 있습니다.

### 취소선과 위·아래 첨자

- `~~취소선~~`은 한글 취소선이 됩니다(#9). `1~2`처럼 물결표 하나는 그대로 글자로 남습니다.
- 위·아래 첨자는 HTML 태그 `m<sup>2</sup>`, `H<sub>2</sub>O`로 쓰며 한글의 위 첨자·아래 첨자가 됩니다(#10). 태그는 대소문자를 가리지 않지만 속성은 쓸 수 없고, 같은 강조·링크 범위 안에서 열고 닫아야 합니다. 그 밖의 HTML은 계속 오류입니다.
- 취소선과 위·아래 첨자는 굵게·기울임과 같은 곳(본문, 제목, 목록, 각주, 표 셀, 캡션, 출처)에 쓸 수 있습니다. 위·아래 첨자를 서로 겹치거나 그 안에 수식을 넣는 것은 거부합니다.
- 취소선은 기본적으로 실선·검은색이며, 템플릿 글자 모양에 이미 취소선이 있으면 그 모양과 색을 유지합니다.

## 실행

`md2hwp-v0.6.0-windows-x64.zip`을 풀고 같은 폴더에서 다음 명령을 실행합니다.

```powershell
.\md2hwp.exe setup-pandoc
.\md2hwp.exe path\to\원고.md
```

원고 옆에 `원고.ir.json`과 `원고.output.hwp`가 생성됩니다.

기존 한컴 백엔드를 지정하려면 다음 명령을 실행합니다.

```powershell
.\md2hwp.exe 원고.md --worker .\md2hwp-hancom.exe
```

## 필요한 환경

Windows x64 환경에서 실행됩니다. 기본 rhwp 백엔드를 통한 HWP 문서 생성에는 `setup-pandoc`으로 받는 Pandoc 외에 추가 환경이 필요하지 않습니다.
단, 표 열 폭과 목록 표식 실측에는 Windows에 설치된 TTF/OTF 글꼴이 필요합니다.
HWPX와 한글 백엔드, `hwp2pdf.exe`, `hwpeqrecalc.exe`에는 한컴 한글, .NET 10 x64 런타임과 등록된 한글 보안 모듈이 추가로 필요합니다.
보안 모듈은 [한컴 공식 안내](https://developer.hancom.com/hwpautomation)에 따라 설치·등록하십시오.

> [!TIP]
> 저장소 소스코드를 다운받으면 [설치 스크립트](https://github.com/esnahn/md2hwp/tree/main/dependencies/install-hancom-security-module.ps1)를 통해 자동 설치가 가능합니다.
> [lock.json](https://github.com/esnahn/md2hwp/tree/main/dependencies/lock.json)을 같은 폴더에 저장한 뒤,
> 한글을 모두 닫고 그 폴더에서 실행할 수 있습니다.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-hancom-security-module.ps1
> ```
>
> 스크립트는 다운로드 파일의 해시를 확인하고 DLL을 설치·등록합니다. 설치된 DLL은 이후에도 그 위치에 유지하십시오.
> 실행용 배포 ZIP에는 스크립트와 `lock.json`이 포함되지 않으며 프로그램 실행에 `lock.json`은 필요하지 않습니다.

## 이전 버전 사용자

- 기본 rhwp 백엔드가 속도가 훨씬 빠르므로 기본 백엔드 사용을 우선 고려하십시오. rhwp 백엔드 사용을 위해 템플릿을 HWP 파일로 작성하는 것을 추천합니다.
- 입력 IR과 템플릿의 `ir-version`은 프로그램 IR과 정확히 일치해야 합니다. 이전 IR 파일은 원고에서 다시 생성하십시오.
- 기존 사용자 템플릿은 `{{md2hwp:ir-version:0.6}}`과 별도 문단 수식 선언(`display_math.fontsize-pt`, `begin:display_math`·`end:display_math`와 `slot:display_math` 표본)을 직접 반영하거나 `init-template`으로 새로 만들어야 합니다. 선언 형식은 [템플릿 안내](https://github.com/esnahn/md2hwp/blob/v0.6.0/docs/templates.md)에 있습니다. 동봉한 기본 템플릿 `template.hwp`에는 수식 스타일과 선언이 들어 있습니다.

## 새 도구: hwpeqrecalc.exe

한글은 파일을 열 때 저장된 수식 크기를 그대로 씁니다. `hwpeqrecalc.exe`는 HWP·HWPX 문서를 한글로 열어 모든 수식을 다시 조판하고 별도 파일로 저장합니다. rhwp 결과는 거의 정확하지만, 확실하게 한글이 만든 크기로 바꾸고 싶을 때 씁니다. 저장·재열기 비교에서 예상 외의 변경이 있으면 `hwpeqrecalc: warning:`으로 처음 다른 위치를 알리고 출력은 그대로 저장합니다.

```powershell
.\hwpeqrecalc.exe 결과.hwp
.\hwpeqrecalc.exe 결과.hwp --output 결과.hwpx
```

[동작과 검증 범위](https://github.com/esnahn/md2hwp/blob/v0.6.0/docs/hwpeqrecalc.md)를 참고하십시오.

## 배포 파일 구성

Release 빌드 ZIP은 `md2hwp.exe`, `md2hwp-rhwp.exe`, `md2hwp-hancom.exe`, `hwp2pdf.exe`, `hwpeqrecalc.exe` 다섯 실행 파일을 포함합니다. ZIP에는 기본 HWP 템플릿, README, 원고 작성 안내 두 문서, PowerShell 자동완성과 `LICENSE`, `THIRD-PARTY-NOTICES.txt`, `licenses/`도 포함합니다.
프로젝트 코드는 AGPL-3.0-only이며 제3자 자료는 각 원저작자의 조건을 따릅니다.

`md2hwp-v0.6.0-windows-x64.zip` SHA-256:
```
D133683EA945873207DF9C1338C82CF3A34D7D07971D3F4E5FCC61CF2D29A036
```

## 지원 범위와 알려진 제한

- rhwp의 수식 크기는 추정값입니다. 실측 자료에서 높이 오차 90%값은 맞춤 자료 4, 툴바 수식 50, 크기별 자료 15~28 HWPUNIT(75 = 1픽셀)입니다. 110쪽·수식 149개 시험 원고에서 쪽 나눔은 한글 백엔드와 같았지만, 별도 문단 수식 폭이 최대 4.5픽셀 달라 가운데 정렬 위치가 조금 다릅니다. 폭을 작게 추정하면 수식이 뒤 글자에 붙을 수 있습니다. 한글과 정확히 같은 크기가 필요하면 `hwpeqrecalc.exe`를 사용하십시오.
- 목록 항목 안의 별도 문단 수식, 그리고 제목·표 셀·캡션·출처의 수식은 지원하지 않습니다.
- 변환기가 지원하지 않는 TeX 명령(예: `\mid`)은 원고 위치와 함께 오류로 보고합니다.
- `hwpeqrecalc.exe`로 HWP→HWPX 변환을 하면 한글의 HWPX 저장이 수식 밖의 내용을 바꿀 수 있습니다(예: 한글 기본 문서의 개요 수준 8 문단 모양). 이때는 경고가 나오며 출력은 저장됩니다. 이런 손실을 피하려면 원본과 같은 형식으로 출력하십시오.
- v0.5.0의 알려진 제한은 그대로입니다. 긴 고정 크기 코드 박스와 추적 템플릿의 TreatAsChar 표는 잘릴 수 있고, rhwp는 HWPX를 지원하지 않으며, 한글 백엔드 `init-template`이 만든 새 HWPX 템플릿은 원고 변환의 HWPX→HWP 정규화 검사에서 실패할 수 있습니다. 이번 릴리스에서 이 문제들을 다시 시험하거나 고치지 않았습니다. [v0.5.0 릴리스 노트](https://github.com/esnahn/md2hwp/blob/v0.6.0/docs/release-v0.5.0.md)를 참고하십시오.
- 저장·재열기 구조 검증만으로 수식의 넘침·겹침이나 표시 번호를 판정할 수 없으므로 생성된 문서의 배치를 확인하십시오.
