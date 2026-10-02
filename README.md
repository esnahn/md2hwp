# md2hwp v0.3.0 (개발 중)

지원하는 CommonMark 원고를 IR JSON으로 정규화한 뒤, 태그가 있는 기존 HWP
템플릿을 한글 COM으로 편집합니다. 현재 지원 대상은 Windows x64와 HWP입니다.
HWPX, 생성 표, 각주, RST 직접 입력은 이번 배포 범위에 포함하지 않습니다.

기본 템플릿과 프로그램은 IR 0.3을 사용합니다. 이전 0.2 원고는 IR을 다시 생성하고,
사용자 템플릿은 `init-template`으로 0.3 파일을 새로 생성하여 사용하십시오.

## 빌드

Rust 버전은 `rust-toolchain.toml`, .NET SDK는 `global.json`에 고정되어 있습니다.
.NET SDK가 없으면 `dependencies/install-dotnet-sdk.ps1`로 개발용 SDK를 준비합니다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1
cargo test --workspace
```

Release 빌드 결과는 `target/release/`의 다음 세 파일입니다.
`-Configuration Debug`를 주면 `target/debug/`에 모입니다.

- `md2hwp.exe`
- `md2hwp-backend.exe`
- `template.hwp`

C# 단독 프로젝트는 `backends/hancom-automation/Md2Hwp.Backend.csproj`입니다.
`templates/template.hwp`를 게시 폴더와 Rust 빌드 폴더에 복사합니다.
빌드 결과는 Git에서 제외합니다. 배포할 때 위 세 파일만 같은 폴더에 복사합니다.

## 실행

```powershell
.\md2hwp.exe 원고.md
.\md2hwp.exe 원고.md 결과.hwp
.\md2hwp.exe 원고.md --output 결과.hwp --template 내템플릿.hwp
.\md2hwp-backend.exe 원고.ir.json
.\md2hwp-backend.exe 원고.ir.json --output 결과.hwp --template 내템플릿.hwp
```

첫 명령은 원고 옆에 `원고.ir.json`, `원고.output.hwp`를 생성합니다.
백엔드는 기존 IR에서 HWP만 생성합니다. 두 번째 인자는 선택적인 HWP 출력 경로입니다.
검증된 IR은 교체하며, HWP는 생성·재열기 검증 성공 후 기존 결과를 교체합니다.
서식 보존 검사는 저장 중 바뀔 수 있는 문단·글자 모양의 내부 번호 대신 실제 설정을 비교합니다.
실패한 HWP 생성은 이전 HWP를 보존합니다. 원고와 템플릿은 덮어쓰지 않습니다.
간편 실행의 이미지 경로는 원고/IR 폴더 안에서 해석합니다.
간편 실행은 출력 경로를 위치 인자 또는 `--output`으로 받습니다. 두 방식을 중복 지정할 수는 없습니다.
Rust 간편 실행은 `--template`, `--worker`, `--dotnet`도 받으며, 백엔드는 `--template`을 받습니다.
변환 작업별 목록은 기본적으로 숨깁니다. 간편 실행이나 `ir2hwp`에 `--verbose`를 붙이면 표시합니다. 오류와 최종 결과는 항상 출력합니다.
옵션 입력 순서는 자유롭습니다. 명시한 경로는 실행한 작업 폴더 기준이며,
이미지의 기준 폴더만 원고/IR 폴더로 유지됩니다.

기존 옵션 방식도 사용할 수 있습니다.

```powershell
.\md2hwp.exe md2ir 원고.md --force
.\md2hwp.exe ir2hwp --ir 원고.ir.json --output 원고.output.hwp
.\md2hwp-backend.exe ir2hwp --ir 원고.ir.json --output 원고.output.hwp
```

`md2ir`도 `md2ir 원고.md [결과.ir.json]`처럼 위치 인자를 받으며, 기존 `--input`·`--output` 방식도 지원합니다. 같은 항목의 중복 지정은 거부합니다.

`md2ir`에서 `--output`을 생략하면 입력 옆의 `원고.ir.json`, `--from`을 생략하면 `commonmark`를 사용합니다. Pandoc JSON 입력은 `--from pandoc-json`으로 지정합니다.

`--template`과 Rust의 `--worker`를 생략하면 각 EXE 옆의 `template.hwp`,
`md2hwp-backend.exe`를 찾습니다. 옵션 방식의 이미지 기준 폴더는 작업 폴더입니다.
현재 IR `0.3`, 정규화 규칙과 스키마는 실행 파일에 포함됩니다.
입력 IR과 템플릿의 IR 버전은 프로그램의 현재 IR 버전과 정확히 같아야 합니다.
변환 규칙과 템플릿에는 별도의 계약 버전을 두지 않습니다.

개발 참고 자료의 파일명·SHA-256·출처는 [reference/README.md](reference/README.md)에 기록합니다.

## 기능 검증 원고

[`examples/all-features-twice/`](examples/all-features-twice/)에는 검증 원고
`all-features.md`, 예상 IR `all-features.ir.json`, 그림 `image.png`를 함께 보관합니다.
제목 1~6단계, 본문 강조와 줄바꿈, 링크, 중첩 목록, 출처가 있는/없는 박스와 그림을
두 장에서 반복합니다. 각 장에 2단계 제목을 세 개씩 두어 장 표지의 절 목록도 확인합니다.

저장소 루트에서 빌드한 프로그램으로 실행합니다.

```powershell
.\target\release\md2hwp.exe .\examples\all-features-twice\all-features.md
```

원고 옆의 IR을 갱신하고 `all-features.output.hwp`를 생성합니다.
IR은 Git에 포함하며, 생성된 HWP는 Git에서 제외합니다.
IR 변환만 확인하려면 다음 명령을 사용합니다.

```powershell
.\target\release\md2hwp.exe md2ir .\examples\all-features-twice\all-features.md --force
```

## 기본 서식 템플릿 만들기

```powershell
.\md2hwp.exe init-template 새템플릿.hwp
.\md2hwp-backend.exe init-template 새템플릿.hwp
```

두 명령은 같은 기능입니다. 경로를 생략하면 **현재 작업 폴더**에 `template.hwp`를
만들며, `--output <경로.hwp>`도 지원합니다. 기존 파일은 덮어쓰지 않습니다.
Rust 명령은 `--worker`, `--dotnet`도 지원합니다.

기존 템플릿 파일 없이 한글의 새 빈 문서 기본 서식에서 시작합니다. 현재 IR의
필수 선언, 제목 1~6단계·본문 스타일, 글머리표·번호 목록 원형, 단일 셀 박스와 출처, 샘플 그림과 실제 자동번호 캡션·출처를
구성하고 저장·재열기 및 프로토타입 검증이 성공한 파일만 내보냅니다.
제목을 포함한 역할별 `md2hwp.*` 스타일은 처음에는 같은 기본 서식이며,
생성한 템플릿에서 각 스타일을 편집해 문서 디자인을 정할 수 있습니다.
템플릿 정의 영역은 변환 결과에서 제거됩니다.

```text
{{md2hwp:begin:template}}
{{md2hwp:ir-version:0.3}}
스타일 선언과 목록·박스·그림·출처 원형
{{md2hwp:end:template}}
{{md2hwp:content}}
[빈 문단]
```

`begin:template` 앞에는 최소 한 문단이 있어야 합니다. 그 앞의 표지·안내 등 고정 내용은
출력에 보존됩니다. `end:template` 바로 다음 문단에 `content`를 두고,
그 뒤에는 **문서의 마지막 빈 문단 하나**를 둡니다. 위 예시의 `[빈 문단]`은
설명이며 실제 글자로 입력하지 않습니다. `content`는 생성 내용을 넣을 위치입니다.

### 태그 참조

아래 표의 이름은 모두 `{{md2hwp:이름}}`으로 감쌉니다. `N`·`값`은 실제 값으로 바꿉니다.
필수 선언은 원고에서 해당 기능을 사용하지 않더라도 필요합니다.
설정·스타일·박스·그림 선언은 `begin:template`과 `end:template` 사이에 둡니다.

| 태그 이름 | 필수 여부 | 의미·배치 |
| --- | --- | --- |
| `begin:template` / `end:template` | 필수 | 템플릿 정의 영역의 시작·끝. 각각 독립 문단 |
| `ir-version:0.3` | 필수 | 프로그램과 일치해야 하는 IR 버전 |
| `content` | 필수 | 정의 영역 바로 뒤의 삽입 위치. 독립 문단이며 뒤에 마지막 빈 문단 필요 |
| `body` | 필수 | 본문에 사용할 한글 문단 스타일을 지정하는 샘플 문단 |
| `heading.N` | 수준별 필수, 블록으로 대체 가능 | N=1~6. 단일 제목 스타일 샘플 |
| `reset` | 필수 | 스타일 재적용 때 잠시 거치는 별도 문단 스타일 |
| `list.max-depth:값` | 필수 설정 | 허용할 중첩 깊이. 정수 1~6 |
| `list.indent-hwp:값` | 필수 설정 | 깊이별 추가 왼쪽 들여쓰기. 정수 1~10000 HWPUNIT |
| `list.bullet` / `list.ordered` | 둘 다 필수 | 실제 한글 글머리표·문단 번호를 적용한 독립 샘플 문단 |
| `begin:block.box` / `end:block.box` | 필수 | 박스 표가 붙은 문단 하나를 감싸는 독립 경계 문단 |
| `slot:box.content` | 필수 | 박스의 한 셀 안에 두는 내용 슬롯 문단 |
| `slot:box.source` | 필수 | 박스 표의 실제 캡션에 두는 출처 슬롯 |
| `figure.max-width-mm:값` | 필수 설정 | 그림 최대 폭. 0 초과~142mm, 소수점은 `.` 사용 |
| `begin:figure.caption` / `end:figure.caption` | 필수 | 실제 캡션이 붙은 샘플 그림 문단 하나를 감싸는 독립 경계 문단 |
| `slot:figure.caption` / `slot:figure.source` | 둘 다 필수 | 그림의 실제 캡션 안에 설명·출처 순서로 각각 한 문단 |
| `begin:heading.N` / `end:heading.N` | 선택 | 해당 수준의 단일 제목 선언 대신 사용하는 블록 경계 |
| `slot:heading.N` | 헤딩 블록·반복 범위에서 필수 | 해당 범위의 제목을 넣는 독립 문단. 여러 개 사용 가능 |
| `begin:each.child:heading.N` / `end:each.child:heading.N` | 선택 | 헤딩 블록 안에서 바로 아래 수준의 제목마다 반복. 중첩 가능 |
| `begin:once` / `end:once` | 선택 | 헤딩 블록 안의 같은 문단에서 첫 사용에만 남길 인라인 범위 |

### 필수 선언과 샘플

이번 원고에서 사용하지 않더라도 해당 IR이 지원하는 모든 요소의 스타일·슬롯·원형이
필요합니다. IR 버전이 바뀌면 원고에서 IR을 다시 만들고 `init-template`으로 새 템플릿을
생성해 꾸밉니다. 구버전 IR 및 `samples`/`minimal-1`/`source-label` 템플릿은 지원하지 않으며,
프로그램이 기존 사용자 템플릿을 자동으로 수정하지 않습니다.

본문은 `{{md2hwp:body}}` 문단에 적용한 스타일을 사용합니다. 박스가 놓인 문단과
그림 자리 문단은 각각의 샘플 스타일을 유지하므로 본문과 같은 스타일일 필요는 없습니다.

`{{md2hwp:reset}}`은 같은 스타일을 다시 적용할 때 잠시 거치는 스타일을 지정합니다.
한글의 스타일 덮어쓰기 확인창을 피하기 위한 선언이며, 출력 문단의 최종 스타일은
본문·제목 등 해당 역할의 스타일로 적용됩니다. `reset` 문단에는 `body`와 **다른 이름의
한글 문단 스타일**을 지정해야 합니다. 두 스타일의 시각적 서식은 같아도 됩니다.
`init-template`이 만든 별도 reset 스타일을 그대로 사용해도 됩니다.

목록 설정 `{{md2hwp:list.max-depth:6}}`, `{{md2hwp:list.indent-hwp:1000}}`은
기본 템플릿에서 두 목록 샘플 바로 앞에 둡니다. `list.max-depth`는 1~6,
`list.indent-hwp`는 1~10000의 정수입니다. 들여쓰기 단위는 HWPUNIT이며 7200이 1인치입니다.
기존 `lists.*` 태그는 `list.*`로 바꿉니다.

목록은 정의 영역 안의 `{{md2hwp:list.bullet}}`, `{{md2hwp:list.ordered}}` 문단에서
실제 한글 글머리표·문단 번호 정의를 가져옵니다. 각 태그를 별도의 한 문단에 넣고,
문자 `●`나 `1.`을 직접 입력하는 대신 한글의 글머리표·문단 번호 기능을 적용합니다.
번호 원형에는 지원하는 모든 중첩 단계의 번호 모양이 있어야 합니다.
기호, 단계별 번호 모양·앞뒤 장식, 번호와 본문 간격 등은 템플릿에서 편집합니다.
시작 번호·항목 순서는 원고에서 가져오며 목록 본문은 `body` 스타일을 사용합니다.
단계별 왼쪽 들여쓰기는 기존 `list.indent-hwp` 계산을 유지합니다.
목록이 없는 원고에도 두 선언은 필수입니다. 이전 템플릿에는 두 샘플 문단을 추가하거나
`init-template`으로 새 파일을 생성해야 합니다. 현재 IR 버전은 0.3입니다.

박스 원형은 다음처럼 구성합니다. 대괄호 안은 배치 설명이며 입력할 문구가 아닙니다.

```text
{{md2hwp:begin:block.box}}
[한 문단에 붙은 실제 단일 셀 표 하나]
  [표 셀 안의 독립 문단] {{md2hwp:slot:box.content}}
  [표에 붙은 실제 캡션 문단] 출처: {{md2hwp:slot:box.source}}
{{md2hwp:end:block.box}}
```

두 경계 사이의 최상위 문단은 표가 붙은 문단 하나여야 합니다. 내용 슬롯은 셀 안에
정확히 하나 두고, 출처 슬롯은 표 바깥의 일반 본문이 아니라 **표 자체의 한글 캡션**에
한 번 넣습니다. 경계 문단 사이에 별도의 설명 문단을 추가하지 않습니다.
박스의 테두리·셀 서식과 내용·출처 문단의 스타일은 샘플에서 가져옵니다.

그림 원형은 다음처럼 선언합니다. 최대 폭 선언도 `begin:template` 영역 안에 둡니다.
`figure.max-width-mm`는 0보다 크고 142 이하인 mm 값이며, 소수점은 `.`으로 씁니다.

```text
{{md2hwp:figure.max-width-mm:142}}
{{md2hwp:begin:figure.caption}}
[실제 샘플 그림 하나 — 아래 두 문단은 그림에 붙은 한글 캡션 안에 작성]
  [그림 <자동번호>] {{md2hwp:slot:figure.caption}}
  출처: {{md2hwp:slot:figure.source}}
{{md2hwp:end:figure.caption}}
```

샘플 이미지 내용과 크기는 출력에 사용하지 않습니다. 원고 이미지를 비율에 맞춰 삽입하고,
샘플의 캡션 위치·간격·폭 설정과 캡션 문단 서식을 가져옵니다. 결과의 설명과 출처는
실제 그림 개체에 붙은 캡션입니다. 기존 `begin:figure`/`slot:figure.image` 템플릿은
`init-template`으로 다시 생성하거나 위 구조로 변경해야 합니다. 현재 IR 버전은 0.3입니다.

그림 캡션은 `{{md2hwp:slot:figure.caption}}`이 들어 있는 **한 문단 전체**를
원형으로 복제합니다. 같은 문단에 실제 한글 그림 자동번호 개체가 하나 있어야 합니다.
`[그림 자동번호] 설명슬롯`, `Figure 자동번호. 설명슬롯 (참고)`처럼 앞뒤 문구와
자동번호 모양을 템플릿에서 바꿀 수 있으며, 설명 슬롯이 번호 앞에 있어도 됩니다.
코드는 설명 슬롯만 원고의 캡션으로 치환합니다.
설명 슬롯은 한 번만 사용하고, 캡션은 설명과 출처의 두 문단으로 구성하며, 추가 문단·강제 줄바꿈·다른 개체는 지원하지 않습니다.

박스와 그림 출처도 각각 한 문단을 원형으로 사용합니다. 새 템플릿에는 다음 슬롯이
들어 있으며, 슬롯 앞뒤의 문구와 각 문단의 스타일을 별도로 편집할 수 있습니다.

```text
자료: {{md2hwp:slot:box.source}} 제공
Source — {{md2hwp:slot:figure.source}}
```

박스 출처는 박스 표의 네이티브 캡션 안에, 그림 출처는 그림의 실제 캡션 안에서 설명 다음 문단에 둡니다.
원고의 출처가 없으면 박스 캡션 영역 또는 그림 출처 문단 자체를 생략합니다.
마크다운에서는 박스 코드 블록이나 독립된 그림 문단 다음에 `출처: 내용` 문단을 둡니다.
그림과 출처 사이에는 빈 줄을 넣어 별도 문단으로 구분합니다.
머리말은 각 출처 슬롯 앞뒤의 문구로 지정합니다.
`init-template`은 박스·그림 출처 샘플의 슬롯 태그 바로 앞에서 각각 한글의 Shift+Tab 대응 동작을 실행하고,
각 문단의 서식으로 계산된 내어쓰기를 해당 출처 스타일에 저장합니다. 긴 출처의 다음 줄이
접두사 뒤에서 시작하도록 하는 초기 설정이며, 이후 글꼴·접두사·양쪽 정렬에 따른
간격을 바꾸면 필요에 따라 내어쓰기도 다시 조정합니다.

생성에는 한글·보안 모듈·.NET 런타임이 필요하며 Pandoc은 필요하지 않습니다.
두 EXE만 배포한 뒤 이 명령으로 템플릿을 만들 수도 있습니다.
생성 파일을 EXE 옆의 `template.hwp`로 두면 변환 시 자동으로 찾습니다.
다른 위치의 템플릿은 `ir2hwp`의 `--template`으로 지정합니다.

### 선택적인 헤딩 블록

기본 `heading.1`~`heading.6`은 단일 문단 선언입니다. 특정 수준을 여러 문단으로
구성하려면 그 선언 대신 다음 범위를 사용합니다. `init-template`에도 안내가 포함됩니다.

```text
{{md2hwp:begin:heading.1}}
장 제목 앞에 반복할 문구
{{md2hwp:slot:heading.1}}
장 제목 뒤에 반복할 문구
{{md2hwp:end:heading.1}}
```

범위 안의 문단과 표·글상자·묶음 도형을 복제하고 독립 문단인 제목 슬롯만 원고의 제목으로
교체합니다. 경계 태그도 각각 독립 문단이어야 합니다. 같은 수준의 단일 선언과
블록 선언은 동시에 사용할 수 없습니다. 1~6 수준별로 선택하며 블록은 1~64개
최상위 문단을 지원합니다. 제목 슬롯은 하나 이상 둘 수 있으며 모두 같은 제목으로 채웁니다.
표 셀·글상자·머리말·꼬리말 안에도 독립 문단으로 둘 수 있습니다.
각 슬롯의 문단·글자 서식을 각각 유지하고, 원고의 굵게·기울임 강조를 그 위에 적용합니다.
시작 경계 문단의 쪽 나눔은 블록 첫 문단에, 끝 경계 문단의 쪽 나눔은 블록 다음 문단에 적용합니다.
쪽 나눔, 머리말·꼬리말, 감추기, 새 번호 제어와 개체 배치를 보존합니다.
제어문자는 제목 슬롯과 별도의 문단에 둡니다. 지원하지 않는 중첩 선언, 블록 내부 구역·단 정의,
내장 그림·OLE·동영상은 지원하지 않으며 명시적으로 오류를 냅니다.
제목을 담는 표·셀의 높이 변화는 구조 손실 오류와 분리해 배치 안내를 출력합니다.
글자 수 제한이나 잘림·겹침 자동 판정은 하지 않으므로 긴 제목의 줄바꿈과 쪽 배치는 출력에서 확인해야 합니다.
표 너비·위치, 문단·글자 서식, 내용과 제어문자의 검증은 유지합니다.
장 표지의 절 목록은 헤딩 블록 안에 아래 반복 범위를 둡니다. 경계와 제목 슬롯은 각각
독립 문단이며, 경계 쌍은 같은 본문·표 셀·글상자 문단 목록 안에 있어야 합니다.

```text
{{md2hwp:begin:each.child:heading.2}}
{{md2hwp:slot:heading.2}}
{{md2hwp:begin:each.child:heading.3}}
{{md2hwp:slot:heading.3}}
{{md2hwp:end:each.child:heading.3}}
{{md2hwp:end:each.child:heading.2}}
```

`heading.1` 블록에서는 그 장의 헤딩2를, 안쪽 반복에서는 현재 헤딩2의 헤딩3을 원고
순서대로 가져옵니다. 다음 같은 수준 또는 상위 헤딩에서 소속 범위가 끝납니다.
반복 대상은 정확히 한 단계 아래여야 하며, 건너뛴 수준은 끌어올리지 않습니다.
자식이 없으면 반복 내용은 생략합니다. 빈 글상자·표 셀 자체는 유지됩니다.
각 범위에는 해당 수준의 제목 슬롯이 하나 이상 필요하고, 슬롯마다 샘플 서식을 유지합니다.
샘플에 한글 문단 번호가 있으면 각 반복 범위에서 1부터 번호를 매깁니다.
직접 입력한 숫자는 자동 번호로 해석하지 않습니다. 중첩 목록의 들여쓰기·번호 모양도
각 샘플 문단에서 지정합니다. 원고·IR 형식은 바뀌지 않습니다.

헤딩 블록 안에서 `{{md2hwp:begin:once}}`와 `{{md2hwp:end:once}}` 사이의
텍스트·제어문자는 해당 수준의 헤딩이 처음 나올 때만 포함합니다. 이후에는 범위 안의
내용만 생략하며, 양쪽 태그는 항상 제거합니다. 두 태그는 같은 문단에 두어야 하고,
중첩하거나 제목 슬롯·다른 템플릿 선언을 감쌀 수 없습니다. 여러 범위는 사용할 수 있습니다.
예: `{{md2hwp:begin:once}}[한글의 실제 새 쪽 번호 제어]{{md2hwp:end:once}}`.
대괄호 설명을 입력하는 것이 아니라 조판 부호 표시로 확인한 실제 제어문자 앞뒤에 태그를 넣습니다.
범위 밖의 그림·표 번호 제어와 문단 자체의 서식·쪽 나눔은 매번 유지됩니다.

장 번호 자동 생성이나 IR 변경은 포함하지 않습니다.

## 사용자 환경

- 한글 설치 및 로그인된 대화형 Windows 세션이 필요합니다. 기준 환경은
  한글 2020 HWP 11.0.0.9136입니다. 실행 중인 한글 문서는 먼저 닫습니다.
- .NET 10 x64 런타임이 필요합니다. Rust는 실행 전에 확인하고 없으면
  [공식 설치 페이지](https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0)를 안내합니다.
  백엔드 직접 실행 시 별도 설치 런타임은 `DOTNET_ROOT_X64`로 지정합니다.
- 한글 보안 모듈의 설치·등록이 필요합니다. 아래 [설치 안내](#한글-보안-모듈-설치등록)를
  따라 현재 사용자 계정에 등록하십시오. 앱은 RegisterModule 성공을 요구합니다.
- Pandoc은 `md2hwp setup-pandoc`으로 공식 배포본을 다운로드할 수 있습니다.
  `%LOCALAPPDATA%\md2hwp\pandoc`에 원본 ZIP·문서·저작권 고지를 보관합니다.
  탐색 순서는 `--pandoc`, 관리 다운로드, PATH입니다. 변환 중 자동 설치는 하지 않습니다.
  다운로드 명령은 빌드 시 lock의 버전을 우선하고, 없으면 공식 최신 안정판을 확인합니다.
  버전 번호 대신 Pandoc JSON 계약의 호환성을 검사합니다.

### 한글 보안 모듈 설치·등록

[한컴 공식 한글 오토메이션 페이지](https://developer.hancom.com/hwpautomation)에서
**보안모듈(Automation).zip**을 내려받아 압축을 풀고, 동봉된 안내에 따라 현재 사용자
계정에 등록하십시오. DLL을 둘 폴더는 자유롭게 선택할 수 있습니다.

- 레지스트리 키: `HKEY_CURRENT_USER\Software\HNC\HwpAutomation\Modules`
- 값 이름: `FilePathCheckerModuleExample`
- 값 형식: `REG_SZ`
- 값 내용: DLL의 절대 경로

등록 후 DLL은 해당 위치에 유지하십시오. DLL을 옮기면 등록 경로도 수정해야 합니다.
프로그램은 보안 모듈을 자동으로 설치하거나 등록하지 않습니다. 등록이 없거나 등록된
DLL을 찾을 수 없으면 안내와 함께 HWP 생성을 중단하고 기존 결과 파일을 보존합니다.

> [!TIP]
> 다운로드·설치·등록을 돕는 [설치 스크립트](dependencies/install-hancom-security-module.ps1)를
> 사용할 수 있습니다. 저장소 루트에서 한글을 모두 닫고 다음 명령을 실행하십시오.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\dependencies\install-hancom-security-module.ps1
> ```
>
> 소스 ZIP 전체를 받을 필요는 없습니다. 스크립트와 [lock.json](dependencies/lock.json)을
> 같은 폴더에 저장했다면, 그 폴더에서 다음 명령을 실행하십시오.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-hancom-security-module.ps1
> ```
>
> 스크립트는 옆의 `lock.json`에서 다운로드 URL과 SHA-256 해시를 읽어 파일을 검증하고,
> 같은 폴더에 DLL을 설치하여 현재 사용자 레지스트리에 등록합니다. 마지막으로 한글의
> `RegisterModule` 성공 여부를 확인합니다. 등록된 DLL은 이후에도 그 위치에 유지하십시오.
>
> 실행용 배포 ZIP에는 스크립트와 `lock.json`이 포함되지 않습니다. 이들은 설치 보조 도구이며,
> 프로그램 실행에 외부 `lock.json`은 필요하지 않습니다. 프로그램이 스크립트를 자동으로
> 실행하지도 않습니다.

IR 0.3의 YAML 메타데이터와 템플릿 태그는 [메타데이터 문서](docs/yaml-variables.md)에 정리했습니다.

## YAML 메타데이터와 날짜

원고 맨 앞의 YAML에서 `title`, `subtitle`, `author`, `date`, `publisher`와
최상위 `md2hwp-<이름>` 문자열 키를 사용할 수 있습니다. author는 문자열 목록도 받습니다.
템플릿 태그는 `{{md2hwp:meta:<키>}}`이며 표지·문단·표 셀·글상자·머리말·꼬리말과 반복 원형에 넣을 수 있습니다.
참조하는 값이 없거나 비어 있으면 해당 태그를 포함한 오류를 냅니다.

```yaml
---
title: 도시 공간 연구
author: [홍길동, 김연구]
date: "2026년 1월 2일"
publisher: 건축공간연구원
md2hwp-report-number: "기본 2026-01"
---
```

`date`는 원문을 보존합니다. Rust core에서 인식한 날짜는 `date-meta`에 `YYYY-MM-DD`로 저장합니다.
연도만 입력하면 1월 1일, 연월만 입력하면 1일로 보완하며 현재 날짜는 사용하지 않습니다.
인식하지 못하는 문구는 date만 보존합니다. YAML에서 date-meta를 직접 지정할 수는 없습니다.

```text
{{md2hwp:meta:title}}
{{md2hwp:meta:date}}
{{md2hwp:meta:date-meta}}
{{md2hwp:meta:date-meta:fmt:%Y년 %-m월 %-d일}}
{{md2hwp:meta:md2hwp-report-number}}
```

입력 형식과 fmt 지시자는 [날짜 계약](docs/date-metadata.md)에 정리했습니다.
검증 원고와 IR은 [`examples/metadata/`](examples/metadata/)에 있습니다.
0.2 IR은 원고에서 다시 생성해야 하며, 템플릿도 0.3으로 다시 생성하여 사용하십시오.

백엔드의 COM 없는 메타데이터 계약 검증:

```powershell
pwsh -NoProfile -File tools/development/dotnet.ps1 run --project tests/backend-contract/Md2Hwp.Backend.Tests.csproj
```

## 저장소 구성과 정리 이력

첫 릴리스 준비 과정에서 빌드에 불필요한 탐색 스크립트, 외부 프로필, 과거 설계·검증
문서와 독립 스모크 테스트를 정리했으며, 실제 릴리스 버전은 v0.2.0으로 확정했습니다.
삭제한 자료는 Git 이력에 남아 있습니다. 현재는 코드·빌드 설정·템플릿뿐 아니라 사용
문서, 검증 원고·이미지·IR, 테스트 fixture와 유지하는 의존성 스크립트도 추적합니다.
Rust 테스트에서 직접 포함하는 JSON fixture는 테스트 빌드에 필요합니다. C#의 일부
클래스 이름은 이전 탐색 시기의 이름을 유지하지만 현재 렌더러가 사용하는 코드입니다.
과거 탐색 명령은 제공하지 않습니다.
`dependencies/lock.json`은 개발 재현성 메타데이터이며 배포 파일이 아닙니다.
한글 보안 모듈 개발 pin은 유지하되 사용자 런타임에서는 강제하지 않습니다.
