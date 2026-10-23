# md2hwp v0.4.4

Markdown 원고를 한글 템플릿의 서식에 맞춰 변환합니다. Windows x64에서 실행하며, 현재 IR 버전은 0.4입니다. 원고는 Markdown 또는 Pandoc JSON으로 입력하며, HWPX·RST 원고 직접 입력은 지원하지 않습니다.

v0.4.4부터 HWPX 템플릿과 HWPX 출력을 지원합니다. 확장자로 형식을 선택하며 기본값은 HWP입니다. [HWPX 사용 범위](docs/hwpx.md)를 참고하십시오.

배포 파일은 [v0.4.4 릴리스](https://github.com/esnahn/md2hwp/releases/tag/v0.4.4)에서 확인할 수 있습니다.
변경 사항은 [릴리스 노트](docs/release-v0.4.4.md)를 참고하십시오.

현재 개발 빌드는 HWP 전용 `md2hwp-rhwp.exe`와 한글 자동화 백엔드 `md2hwp-hancom.exe`를 함께 제공합니다. 일반 문단·제목 블록·메타데이터·중첩 목록·코드 박스·PNG/JPEG/EMF 그림·표·각주·번호 상호참조를 지원하며, 현재 기본 템플릿의 장 표지와 절 목록을 사용할 수 있습니다. 각주는 본문·제목·목록·표 셀에서 지원하며, native 캡션·머리말·꼬리말·바탕쪽에는 넣을 수 없습니다. 여러 문단 목록 항목은 native 번호·글머리표의 실제 폭을 측정해 후속 문단을 정렬합니다. 표의 자동 열 폭과 목록 표식 실측에는 Windows에 설치된 TTF/OTF 글꼴이 필요하며, 지원하지 않는 번호 형식·표식 글리프는 명시적으로 거부합니다. `--worker`를 생략하면 EXE 옆의 rhwp를 우선하고, 파일이 없을 때 한글 백엔드를 선택합니다. 템플릿 또는 출력이 `.hwpx`이면 한글 백엔드만 자동 선택합니다. 명시한 `--worker`는 그대로 사용하며, 실행 실패 후 다른 백엔드로 재시도하지 않습니다. [개발 범위와 사용 방법](docs/rhwp-backend-strategy.md), [각주 시험](docs/rhwp-footnotes-2026-10-22.md), [상호참조 구현과 검증 범위](docs/rhwp-references-2026-10-22.md), [목록 표식 실측 범위](docs/rhwp-list-widths-2026-10-22.md), [통합 비교·성능](docs/rhwp-integration-2026-10-22.md)를 참고하십시오.

`md2hwp validate-template --worker md2hwp-rhwp.exe`와 `md2hwp init-template 새템플릿.hwp --worker md2hwp-rhwp.exe`도 한글·.NET 없이 실행할 수 있습니다. 생성되는 템플릿은 기존 보고서 서식을 복사하지 않은 기본 서식이며, 기존 파일은 덮어쓰지 않습니다. rhwp 템플릿 검사는 선언·원형·서식 참조를 검사하며 실제 한글 페이지 배치까지 판정하지 않습니다.

양쪽의 엄격 검사 판정과 조건부 범위의 native 필드 보존을 맞췄습니다. 한글의 JPEG 재인코딩에 따른 관찰된 표시 차이는 허용하며, 긴 고정 크기 박스·표의 잘림과 지원하지 않는 템플릿 구조 등의 제한은 남아 있습니다. [기능 동등성 검토](docs/rhwp-parity-2026-10-23.md)에 실제 출력 비교와 남은 템플릿 제한을 기록했습니다. 임의 템플릿까지 포함한 완전한 기능 동등성을 보장하지는 않습니다.

## 라이선스

md2hwp는 **AGPL-3.0-only**로 제공합니다.
[전문](LICENSE)을 확인하십시오. 라이선스 조건에 따라
수정·재배포할 수 있으며 보증은 제공하지 않습니다. 사용자 원고의 권리는 작성자에게 있습니다.

외부 라이브러리의 저작권·라이선스는 [제3자 고지](THIRD-PARTY-NOTICES.txt)와
`licenses/`에 보존합니다. 제3자 자료는 각 원저작자의 조건을 따르며,
프로젝트의 AGPL 선언이 해당 자료의 권리를 변경하지 않습니다.

실행 파일을 배포할 때는 라이선스·고지를 함께 제공하고, 실행 파일과 정확히 대응하는
소스의 무료 다운로드 링크를 같은 배포 페이지에 표시하십시오. GitHub Release의
동일 태그 자동 소스 다운로드를 사용하며, 이전 태그나 이동하는 브랜치 링크로 대신하지 않습니다.
이 릴리스의 대응 소스: [v0.4.4 소스 다운로드](https://github.com/esnahn/md2hwp/archive/refs/tags/v0.4.4.zip).
수정한 프로그램을 네트워크 서비스로 제공하는 경우 AGPL 제13조도 적용될 수 있습니다.

Pandoc은 `setup-pandoc`이 따로 다운로드하는 GPL-2.0-or-later 프로그램이며 실행용 ZIP에 포함하지 않습니다.
[Pandoc 저작권·라이선스](https://github.com/jgm/pandoc/blob/main/COPYRIGHT)를 참고하십시오.
한글·보안 모듈·서체·.NET 런타임도 별도 제품으로, 각 제공자의 조건을 따릅니다.

## 시작하기

`md2hwp`는 간략 사용법만 표시합니다. `md2hwp --help` 또는 `md2hwp -h`는 기본 변환 옵션과 명령별 한 줄 사용법을 표시합니다. `md2hwp md2ir --help`처럼 명령 뒤에 `--help`를 붙이면 해당 명령의 사용법·옵션·기본값만 확인할 수 있습니다. 도움말은 Pandoc·한글·.NET을 실행하지 않습니다.

`md2hwp.exe`, `md2hwp-rhwp.exe`, `md2hwp-hancom.exe`, `template.hwp`는 같은 폴더에 둡니다. HWP를 rhwp로 생성할 때는 한글·.NET이 필요하지 않습니다. HWPX와 한글 백엔드, `hwp2pdf.exe`에는 한글, .NET 10 x64 런타임, 한글 보안 모듈이 필요합니다. 이 경우 실행 중인 한글을 닫습니다. [설치 안내](docs/usage.md)를 참고하십시오.

```powershell
.\md2hwp.exe setup-pandoc
.\md2hwp.exe 원고.md
.\md2hwp.exe 원고.md --output 결과.hwp --template 내템플릿.hwp
.\md2hwp.exe 원고.md --output 결과.hwpx --template 내템플릿.hwpx
```

기본 출력은 원고 옆의 `원고.ir.json`과 `원고.output.hwp`입니다. HWP/HWPX는 생성·재열기 검증 성공 후 교체하며, 실패하면 기존 결과를 보존합니다. 원고와 원본 템플릿은 변경하지 않습니다.

## PowerShell 자동완성

배포 폴더에 동봉된 `md2hwp-completion.ps1`을 현재 세션에서 불러옵니다.
Windows PowerShell 5.1과 PowerShell 7에서 사용할 수 있습니다.

```powershell
. .\md2hwp-completion.ps1
```

`md2hwp v` 뒤에서 Tab을 누르면 `validate`·`validate-template`을 완성합니다.
명령별 옵션, `--from` 값, 파일 경로도 완성합니다.
Windows PowerShell 5.1에서는 `--`만 입력하면 셸이 자동완성을 호출하지 않으므로
`--t`처럼 옵션의 첫 글자까지 입력하십시오. PowerShell 7은 `--`부터 완성합니다.
`md2hwp`, `md2hwp.exe`, `./md2hwp.exe` 또는 전체 실행 파일 경로를 사용할 수 있습니다.
자동완성은 md2hwp·Pandoc·한글·.NET을 실행하지 않습니다.

매번 불러오려면 `$PROFILE` 파일에 아래 한 줄을 추가하십시오. 경로는 실제 배포 위치로
고쳐야 합니다. 프로그램과 빌드 스크립트는 사용자 프로필을 수정하지 않습니다.

```powershell
. 'C:\Tools\md2hwp\md2hwp-completion.ps1'
```

개발 저장소에서는 `tools/completions/md2hwp-completion.ps1`이 원본입니다.
빌드할 때 Debug·Release 폴더에 복사하며, 배포 ZIP에도 포함합니다.
명령이나 옵션을 추가할 때 이 스크립트도 갱신해야 합니다. Debug 전용 `--legacy-com`은
배포용 자동완성 목록에 포함하지 않습니다.

## HWP를 PDF로 변환

`hwp2pdf.exe`는 기존 HWP를 PDF로 변환하는 별도 실행 파일입니다.
HWPX 입력은 지원하지 않습니다.

```powershell
.\hwp2pdf.exe 원고.hwp
.\hwp2pdf.exe 원고.hwp --output 결과.pdf
```

기본 출력은 원본 옆의 `원고.pdf`입니다. 권장 설정인 `PrintToPDFEx`와
`GraphicQuality=100`을 사용하며, 완료 확인 후 기존 PDF를 교체합니다.
한글·.NET 10 x64·한글 보안 모듈이 필요하며 템플릿·Pandoc은 필요하지 않습니다.
[실행 방법과 검증 범위](docs/hwp2pdf.md)를 참고하십시오.

## 원고 문법

간략한 작성 방법과 예시는 [원고 작성 안내](docs/manuscript/README-MANUSCRIPT.md)를 참고하십시오.

변환 전에 `md2hwp.exe validate 원고.md --require-meta title`로 엄격한 작성 규칙을 검사할 수 있습니다.
Pandoc은 필요하지만 한글·백엔드를 실행하거나 IR/HWP를 쓰지 않습니다.
개체에 연결되지 않은 `주:`·`출처:`·표 캡션, 잘못된 접두어, 중복/미정의 각주,
닫히지 않은 코드 울타리, 표 칸 수와 기존 AST/IR 계약을 검사합니다.
필수 정보는 `--require-meta <키>`를 반복 지정하며 오류는 종료 코드 1입니다.
일반 변환의 문단 처리 방식은 유지합니다.
검사 범위와 템플릿·시각 검증의 차이는 [엄격한 원고 검사](docs/manuscript/AGENTS.md#엄격한-원고-검사)를 참고하십시오.

### 본문

CommonMark를 기본으로 YAML·각주·속성·독립 그림·파이프 표 확장을 지원합니다.

- **문자:** 유니코드로 작성합니다. 한글 전용 비유니코드 특수문자(예: 옛 문자표의 사각형 숫자 10)는 입력할 수 없습니다.
- **문단 계속:** 빈 줄 없이 줄을 바꾸면 같은 문단으로 이어지며, 줄바꿈은 공백이 됩니다.
- **문단 나누기:** 문단 사이에 빈 줄을 넣습니다.
- **강제 줄바꿈:** 줄 끝에 공백 두 칸 또는 `\`를 넣고 줄을 바꿉니다.
- **강조:** `**굵게**`, `*기울임*`으로 작성합니다.

#### 링크와 미지원 문법

일반 링크 `[표시문](주소)`는 연결 기능 없이 표시문과 글자 서식만 출력합니다. 내부 번호 참조는 아래 상호참조 규칙을 따릅니다. 인라인 코드·인용 블록·HTML 등 미지원 요소는 무시하지 않고 오류로 처리합니다.

### 메타데이터

원고 맨 앞에 YAML로 문서 제목을 적습니다. 동봉 템플릿에는 `title`이 필요하며, 문서 제목은 첫 장 제목과 별개입니다.

```yaml
---
title: 문서 제목
---
```

저자·날짜 등은 [메타데이터 안내](docs/yaml-variables.md)를 참고하십시오.

### 장·절 제목(개요번호, 헤딩)

`# 장 제목`, `## 절 제목`처럼 `#` 1~6개로 작성하며 템플릿의 `heading1`~`heading6`에 대응합니다. 실제 한글 개요 번호는 템플릿에 설정되어 있어야 표시됩니다.

- **템플릿:** 단일 `{{md2hwp:heading1}}` 선언 또는 `begin:heading1`~`end:heading1` 범위 안의 `slot:heading1`로 제목을 배치합니다.
- **하위 제목 목록:** `begin:each.child:heading2`~`end:each.child:heading2` 안에 `slot:heading2`를 두면 해당 장의 절 제목을 순서대로 반복합니다. 제목과 반복 범위에서 `num:heading1`~`num:heading6`으로 자신의 번호와 상위 번호를 함께 표시할 수 있습니다.
- **장 시작 번호:** YAML의 `md2hwp-heading1-start: 3`으로 지정합니다. 1일 경우 지정하지 않은 것과 같은 결과를 얻습니다. 장 번호는 장마다 증가합니다.

태그는 모두 `{{md2hwp:…}}` 형태입니다. [템플릿 상세](docs/templates.md)를 참고하십시오.

헤딩 템플릿의 `once` 범위는 첫 사용에만, `except.once` 범위는 두 번째 이후에만
적용합니다. 기본 템플릿은 Heading1 앞의 문단 경계와 쪽 나눔을 `except.once`로
감싸 첫 장 앞 빈 쪽을 없애고, 두 번째 장부터 강제 쪽 나눔을 유지합니다.

### 그림

`![그림 설명](image.png)`를 독립 문단에 작성합니다. PNG·JPG·JPEG·EMF를 지원하며 설명은 그림 캡션으로 출력합니다.
EMF는 원본 파일을 내장하고 물리적 프레임의 비율로 크기를 정합니다. [지원 범위와 표시 검증](docs/emf-figures.md)을 참고하십시오.

하단 설명은 빈 줄로 구분한 `출처: 내용`, `주.1: 내용` 등의 문단을 개체 뒤에 연속해서 적습니다. 접두어와 순서를 보존하고 템플릿의 출처 문단 서식을 사용합니다. [지원 접두어와 문법](docs/object-sources.md)을 참고하십시오.

### 글상자

백틱 세 개로 감싼 코드 블록을 사용합니다. 공백·탭·본문 빈 줄을 보존하며 블록 안의 Markdown은 해석하지 않습니다. 아래의 `제목:`과 `출처:`는 CommonMark나 Pandoc 확장 문법이 아닌 md2hwp의 원고 규칙입니다.

````markdown
```
제목: 글상자 제목

글상자 본문 첫 줄입니다.
글상자 본문 둘째 줄입니다.
```

출처: 작성자
````

첫 줄에 `제목: 제목 내용`을 적으면 템플릿에 `slot:code.title`이 있을 때 제목으로 분리하고 바로 뒤 빈 줄을 제거합니다. 제목 슬롯이 없으면 원문을 유지합니다.

하단 설명은 블록 뒤에 빈 줄로 구분한 `출처: 내용`, `주: 내용` 등의 문단을 적습니다. 여러 문단을 지원합니다.

### 표

파이프(`|`)로 열을 나누고 머리행 아래에 구분선을 둡니다. `:---`는 왼쪽, `:---:`는 가운데, `---:`는 오른쪽 정렬이며 `---`는 템플릿 정렬을 따릅니다.

```markdown
표: 항목별 수량

| 항목 | 수량 |
| :--- | ---: |
| 예시 | 10 |

출처: 작성자
```

제목은 표 앞이나 뒤의 별도 문단에 `표: 내용`으로 적습니다. `Table:`, `table:`, `:`도 지원합니다.

출처는 표 뒤에 적되, 아래쪽 제목이 있으면 그 다음에 적습니다.

[표 상세](docs/tables.md)를 참고하십시오.

### 상호참조

헤딩·그림에 `{#ID}`를 붙이거나 표 바로 앞의 독립된 줄에 `{#ID}`를 적고 `[참조](#ID)`로 연결하면 템플릿의 그림·표·헤딩 번호 참조로 출력합니다. 헤딩 참조는 원고의 순서로 장·절 번호를 계산한 고정 텍스트이며 헤딩별 템플릿 양식을 사용합니다. 그림·표 참조는 한글 상호참조이고, 표 참조 대상에는 캡션이 필요합니다.

각주는 본문에 `[^이름]`, 별도 문단에 `[^이름]: 각주 내용`으로 작성합니다. [번호 상호참조](docs/cross-references.md)와 [원고 작성 안내](docs/manuscript/README-MANUSCRIPT.md)를 참고하십시오.

## 템플릿과 상세 안내

박스·표·그림의 템플릿 범위에서 샘플 개체 앞뒤에 빈 문단을 넣으면, 생성된 개체의 앞뒤에 서식과 쪽·단 나눔을 보존하여 복제합니다. [범위 작성 규칙](docs/templates.md)을 참고하십시오.

템플릿만 검사하려면 `md2hwp validate-template [template.hwp]`를 실행합니다. 경로를 생략하면 EXE 옆의 `template.hwp`를 검사하며, `--template`, `--worker`, `--dotnet`도 지정할 수 있습니다. 명시한 `.hwpx` 템플릿도 검사합니다. 백엔드에서도 `md2hwp-hancom validate-template [template.hwp]`로 실행할 수 있습니다.

rhwp 검사는 한글·.NET 없이 실행합니다. 한글 백엔드 검사는 한글·.NET과 등록된 보안 모듈이 필요하며, 실행 전 한글 문서를 닫아야 합니다. 원고·Pandoc 없이 IR 버전, 선언, 슬롯과 원형 구조를 검사합니다. 원본은 저장하지 않으며 첫 오류에서 실패합니다. 성공 결과의 `RequiredMetadata`는 원고 YAML에 필요한 키입니다. `RequiresRecognizedDate: true`이면 `date-meta` 생성을 위해 인식 가능한 `date`도 필요합니다. 실제 원고 값, 표 폭, 제목 넘침 등 문서 생성·시각 검증이 필요한 조건은 이 검사만으로 보장하지 않습니다.

개발할 때 기본 서식은 `templates/template.hwp`에서 편집합니다. 빌드는 `target/debug`·`target/release`의 템플릿을 매번 이 파일로 교체합니다. 실행용 배포본에서는 EXE 옆의 `template.hwp`를 사용합니다. 새 파일은 `md2hwp init-template 새템플릿.hwp`로 만들며, `.hwpx` 확장자로 HWPX 템플릿도 만들 수 있습니다. 이전 IR은 다시 생성하고, 기존 사용자 템플릿의 새 선언은 직접 반영하거나 별도 파일로 재생성해야 합니다. HWPX 템플릿·출력의 범위와 실행 예는 [HWPX 안내](docs/hwpx.md)에 있습니다.

Debug와 Release는 XML 구성 방식으로 생성합니다. 개발용 성능 비교는
[빌드·검증 안내](docs/development.md#생성-방식-비교)를 참고하십시오.

- [설치·실행 옵션](docs/usage.md) · [템플릿 편집·태그](docs/templates.md) · [HWPX 템플릿·출력](docs/hwpx.md)
- [표](docs/tables.md) · [번호 상호참조](docs/cross-references.md) · [메타데이터](docs/yaml-variables.md) · [날짜](docs/date-metadata.md)
- [빌드·검증](docs/development.md) · [통합 예제](examples/all-features/) · [v0.4.4 릴리스 노트](docs/release-v0.4.4.md)

저장·재열기 검증만으로 잘림·겹침이나 각주 표시 번호를 보장하지 않으므로 생성된 문서의 배치도 확인하십시오.

그림·표 캡션과 코드·그림·표의 출처 및 하단 설명에서도 헤딩·그림·표 번호 상호참조를
사용할 수 있습니다. 각주는 해당 영역에서 지원하지 않습니다.
[캡션 내부 기능과 한글 2020 검증 결과](docs/caption-inlines.md)를 참고하십시오.
