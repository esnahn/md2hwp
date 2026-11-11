# md2hwp

Markdown 원고를 한글 템플릿의 서식에 맞춰 HWP·HWPX 문서로 변환합니다. Windows x64에서 실행합니다.
원고에는 내용만 적고, 글꼴·번호·배치는 템플릿으로 자동 생성합니다.

현재 버전은 v0.6.0입니다. 배포 파일은 [GitHub Releases](https://github.com/esnahn/md2hwp/releases)에서 받고,
변경 사항은 [v0.6.0 릴리스 노트](docs/release-v0.6.0.md)를 참고하십시오.

## 실행

배포 ZIP을 한 폴더에 풀고 그 폴더에서 실행합니다.

```powershell
.\md2hwp.exe setup-pandoc
.\md2hwp.exe 원고.md
```

첫 명령은 Pandoc을 내려받으며 한 번만 실행합니다. 둘째 명령은 원고 옆에 `원고.output.hwp`와
중간 파일 `원고.ir.json`을 만듭니다. 원고와 템플릿은 바꾸지 않으며, 변환에 실패하면 기존 결과 파일을 그대로 둡니다.

```powershell
.\md2hwp.exe 원고.md --output 결과.hwp --template 내템플릿.hwp
.\md2hwp.exe 원고.md --output 결과.hwpx --template 내템플릿.hwpx
```

출력과 템플릿 경로를 지정할 수 있습니다.
생략하면 출력은 원고 경로 옆의 `원고.output.hwp`, 템플릿은 EXE 옆의 `template.hwp`를 사용합니다.

| 작업 | 필요한 것 |
| --- | --- |
| Markdown 원고 읽기 | Pandoc |
| HWP 만들기 | 템플릿이 쓰는 글꼴(Windows에 설치된 TTF/OTF) |
| HWPX 템플릿·출력, PDF 변환, 수식 크기 다시 계산 | 한글, .NET 10 x64 런타임, 한글 보안 모듈 |

설치 방법과 모든 옵션은 [설치와 실행](docs/usage.md)에 있습니다.

## 명령

| 명령 | 하는 일 |
| --- | --- |
| `md2hwp 원고.md [결과.hwp]` | 원고를 한글 문서로 변환합니다. `--output`, `--template`, `--worker`를 지정할 수 있습니다. |
| `md2hwp validate 원고.md --require-meta title` | 변환하지 않고 원고 작성 규칙만 검사합니다. |
| `md2hwp md2ir 원고.md` | 중간 파일 `원고.ir.json`만 만듭니다. |
| `md2hwp ir2hwp --ir 원고.ir.json --output 결과.hwp` | 중간 파일에서 한글 문서를 만듭니다. |
| `md2hwp init-template 새템플릿.hwp` | 새 기본 템플릿을 만듭니다. 기존 파일은 덮어쓰지 않습니다. |
| `md2hwp validate-template [template.hwp]` | 템플릿의 선언과 샘플 구조를 검사합니다. |
| `md2hwp setup-pandoc` | Pandoc을 내려받습니다. |
| `md2hwp --help`, `md2hwp <명령> --help`, `md2hwp --version` | 사용법과 버전을 표시합니다. |
| `hwp2pdf 원고.hwp` | HWP를 PDF로 변환합니다. [상세](docs/hwp2pdf.md) |
| `hwpeqrecalc 결과.hwp` | 한글이 모든 수식 크기를 다시 계산하게 하고 `결과.recalc.hwp`로 저장합니다. [상세](docs/hwpeqrecalc.md) |
| `. .\md2hwp-completion.ps1` | 현재 PowerShell 세션에 Tab 자동완성을 등록합니다. [상세](docs/usage.md#powershell-자동완성) |

## 원고 문법

- **문서 정보:** 원고 맨 앞 YAML에 `title`, `subtitle`, `author`, `date`, `publisher`, `md2hwp-<이름>`을 적습니다.
- **문단:** 빈 줄로 나눕니다. 문단 안 줄바꿈은 줄 끝에 공백 두 칸이나 `\`를 붙입니다.
- **제목:** `#`~`######`로 적으며 번호는 템플릿이 붙입니다.
- **글자 서식:** `**굵게**`, `*기울임*`, `~~취소선~~`, `<sup>위 첨자</sup>`, `<sub>아래 첨자</sub>`.
- **목록:** `-` 글머리표와 `1.` 번호 목록을 쓰고, 들여써서 하위 목록을 만듭니다.
- **수식:** 문장 안은 `$...$`, 별행은 문단 전체를 `$$...$$`로 적습니다.
- **그림:** 독립 문단의 `![캡션](파일)`. PNG·JPG·JPEG·EMF를 지원합니다.
- **글상자:** 백틱 세 개로 감싼 블록. 첫 줄 `제목: `으로 제목을 붙입니다.
- **표:** 파이프 표. 제목은 표 앞이나 뒤 문단에 `표: 제목`으로 적습니다.
- **출처와 설명:** 그림·글상자·표 뒤 문단에 `출처: 내용`, `주: 내용`처럼 적습니다.
- **각주:** 본문에 `[^이름]`, 별도 문단에 `[^이름]: 내용`.
- **번호 참조:** 제목·그림·표에 `{#ID}`를 붙이고 `[문구](#ID)`로 번호를 가리킵니다.
- **링크:** `[표시문](주소)`는 표시문만 출력합니다.

`제목:`, `출처:`, `표:` 규칙은 CommonMark나 Pandoc 확장이 아닌 md2hwp의 원고 규칙입니다.
지원하지 않는 문법은 무시하지 않고 오류로 알립니다.

예시와 세부 규칙은 [원고 작성 안내](docs/manuscript/README-MANUSCRIPT.md)에 있습니다.

## 템플릿

템플릿은 한글에서 편집하는 HWP·HWPX 파일입니다. `{{md2hwp:…}}` 태그로 원고 내용이 들어갈 자리와
제목·본문·목록·글상자·그림·표·각주·번호 참조의 샘플 서식을 정합니다.

- `--template`을 생략하면 EXE 옆의 `template.hwp`를 사용합니다.
- 새 템플릿은 `md2hwp init-template 새템플릿.hwp`로 만들어 한글에서 서식을 고칩니다.
- 고친 뒤 `md2hwp validate-template 새템플릿.hwp`로 검사합니다.
- 템플릿의 `ir-version`은 프로그램의 IR 버전(v0.6.0은 0.6)과 같아야 합니다. 이전 버전 템플릿은 새로 만들거나 선언을 직접 고칩니다.
- 변환은 원본 템플릿을 수정하지 않습니다.

태그와 작성 규칙은 [템플릿 만들기와 편집](docs/templates.md), HWPX는 [HWPX 템플릿과 출력](docs/hwpx.md)을 참고하십시오.

## 백엔드

문서를 실제로 만드는 프로그램은 두 가지이며, `md2hwp.exe`가 자동으로 고릅니다.

| 백엔드 | 형식 | 한글·.NET·보안 모듈 |
| --- | --- | --- |
| `md2hwp-rhwp.exe` | HWP | 필요 없음 |
| `md2hwp-hancom.exe` | HWP, HWPX | 필요 |

- 템플릿과 출력이 모두 HWP이면 rhwp를(파일이 없으면 한글 백엔드를), 하나라도 HWPX이면 한글 백엔드를 사용합니다.
- `--worker <경로>`로 직접 지정할 수 있습니다. 실행에 실패해도 다른 백엔드로 다시 시도하지 않습니다.
- 한글을 쓰는 프로그램은 자기 한글을 따로 띄우고 닫습니다. 열어 둔 한글 창과 문서는 건드리지 않으며, 저장하지 않은 변경은 읽지 않습니다.
- 두 백엔드의 결과가 모든 템플릿에서 같다고 보장하지는 않습니다.

[rhwp 백엔드 범위](docs/rhwp-backend-strategy.md)와 [두 백엔드 비교](docs/rhwp-parity-2026-10-23.md)를 참고하십시오.

## 제한

- 변환은 저장한 문서를 다시 열어 구조를 검증하지만 화면 배치는 확인하지 않습니다. 잘림·겹침, 쪽 나눔, 각주 번호는 결과 문서에서 직접 확인하십시오.
- 크기가 고정된 글상자·제목 영역과 긴 표는 내용이 넘치면 잘릴 수 있습니다.
- rhwp가 저장하는 수식 크기는 추정값입니다. 한글이 계산한 크기가 필요하면 `hwpeqrecalc.exe`를 실행하십시오.
- 장·절 번호 참조는 고정된 글자입니다. 한글에서 제목을 옮겨도 바뀌지 않으므로 원고를 고쳐 다시 변환하십시오.
- 한글은 JPEG 그림을 다시 인코딩할 수 있습니다.
- 원고 입력은 Markdown과 Pandoc JSON만 지원합니다. `hwp2pdf.exe`는 HWP만 읽습니다.
- 한글 백엔드의 `init-template 새템플릿.hwpx`로 만든 템플릿은 검사에 성공해도 변환에 실패할 수 있습니다. [재현 기록](docs/backend-selection-2026-10-23.md#별도-발견-새-hancom-hwpx-기본-템플릿)

## 소스에서 빌드

Windows x64, PowerShell 7.4 이상, Rust 도구 체인이 필요합니다. 저장소 루트에서 실행하며,
MSVC·Windows SDK를 설치해야 하면 첫 명령은 관리자 PowerShell에서 실행하십시오.

```powershell
pwsh -NoProfile -File dependencies/install-windows-build-tools.ps1
pwsh -NoProfile -File dependencies/install-dotnet-sdk.ps1
pwsh -NoProfile -File tools/development/build.ps1
```

결과는 `target/release`에 모입니다. [빌드와 검증](docs/development.md)을 참고하십시오.

## 문서

- 사용: [설치와 실행](docs/usage.md) · [원고 작성 안내](docs/manuscript/README-MANUSCRIPT.md) · [예시](examples/README.md)
- 원고 기능: [표](docs/tables.md) · [번호 참조](docs/cross-references.md) · [출처와 설명](docs/object-sources.md) · [캡션 안의 참조](docs/caption-inlines.md) · [메타데이터](docs/yaml-variables.md) · [날짜](docs/date-metadata.md) · [EMF 그림](docs/emf-figures.md) · [수식](docs/math.md)
- 템플릿과 도구: [템플릿](docs/templates.md) · [HWPX](docs/hwpx.md) · [hwp2pdf](docs/hwp2pdf.md) · [hwpeqrecalc](docs/hwpeqrecalc.md)
- 개발: [빌드와 검증](docs/development.md) · [로컬 파일](docs/local-files.md) · [rhwp 백엔드](docs/rhwp-backend-strategy.md)

## 라이선스

md2hwp는 **AGPL-3.0-only**로 제공하며 보증은 없습니다. [전문](LICENSE)을 확인하십시오.
외부 라이브러리는 [제3자 고지](THIRD-PARTY-NOTICES.txt)와 `licenses/`의 각 라이선스를 따릅니다.
실행 파일을 다시 배포할 때는 라이선스·고지와 대응 소스([v0.6.0 소스](https://github.com/esnahn/md2hwp/archive/refs/tags/v0.6.0.zip))의 링크를 함께 제공하십시오.
Pandoc·한글·보안 모듈·서체·.NET 런타임은 포함하지 않는 별도 제품이며 각 제공자의 조건을 따릅니다.
