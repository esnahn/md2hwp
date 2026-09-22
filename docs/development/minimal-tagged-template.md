# minimal 태그 템플릿 사용법

`tests/fixtures/templates/minimal-tagged-v1.hwp`는 실제로 사용할 수 있는
`minimal-1` 템플릿이다. 원본 `minimal.hwp`의 표지와 본문 앞 고정 내용을
유지하고, 기존 콘텐츠 부분을 명시적인 샘플 영역으로 바꿨다.
`render-tagged`는 **IR JSON + 이 HWP 템플릿**을 받아 새 HWP를 만든다.
외부 프로필 JSON은 읽지 않는다. IR에서 참조하는 PNG 파일은 별도로 필요하다.

## 실행

저장소 루트에서 먼저 빌드한다.

```powershell
pwsh -NoProfile -File .\tools\development\dotnet.ps1 build `
  .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj `
  --configuration Debug
```

한글 창을 모두 닫은 뒤, 검증된 사용자 대화형 세션에서 실행한다.
보안 모듈은 [환경 설정](environment.md)에 따라 이미 등록되어 있어야 한다.

```powershell
& 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode render-tagged `
  -Ir .\examples\ir-v0.1.json `
  -Template .\tests\fixtures\templates\minimal-tagged-v1.hwp `
  -Output .\artifacts\my-report.hwp -Configuration Debug
```

기본값은 한글 창 숨김이다. 서비스나 로그아웃 상태에서의 실행을 뜻하지 않는다.
출력 파일이 이미 있으면 실패한다. 원본 대신 임시 복사본을 편집하고,
저장·재열기 검증을 통과한 경우에만 지정한 출력 경로로 이동한다.
동시에 여러 문서를 처리하지 않는다.

## `minimal-1` 선언 규약

선언은 대소문자와 공백을 포함해 정확히 일치해야 한다. 각 선언은 한 문단의
단일 논리 줄을 차지한다. 캡션과 박스 내부 슬롯만 아래 구조에 따른다.
문서는 한 구역이어야 하며, 구조는 다음과 같다.

```text
보존할 고정 내용 (한 문단 이상)
{{md2hwp:begin:samples}}
  계약 버전·스타일 샘플·설정·박스 및 그림 프로토타입
{{md2hwp:end:samples}}
{{md2hwp:content}}
빈 마지막 문단
```

`end:samples`, `content`, 마지막 빈 문단은 서로 바로 이어진다.
샘플 영역 전체와 경계 문단은 출력에서 제거된다. IR은 템플릿 선언을 검사한
뒤 삽입하므로 원고에 쓰인 `{{md2hwp:body}}` 같은 문자열은 그대로 출력한다.

샘플 영역에 각각 하나씩 필요한 선언:

| 선언 | 의미 |
| --- | --- |
| `{{md2hwp:contract:minimal-1}}` | 닫힌 규약 버전 |
| `{{md2hwp:body}}` | 이 문단의 실제 한글 문단 스타일을 본문에 연결 |
| `{{md2hwp:heading.1}}` … `{{md2hwp:heading.6}}` | 각 단계 제목 스타일 연결 |
| `{{md2hwp:reset}}` | 본문과 다른 임시 전환 스타일 |
| `{{md2hwp:figure.max-width-mm:142}}` | 그림 최대 너비, 0 초과 142 이하 |
| `{{md2hwp:lists.max-depth:6}}` | 목록 깊이 상한, 1~6; 최상위 깊이는 0 |
| `{{md2hwp:lists.indent-hwp:2000}}` | 목록 단계당 들여쓰기, HWP 단위 정수 1~10000 |
| `{{md2hwp:source-label:출처:}}` | 그림과 박스에 사용할 출처 접두어 |

스타일은 샘플 문단의 **명명된 한글 스타일 정의**를 따른다. 샘플에만 직접
적용한 글자 모양을 새 스타일로 추론하지 않는다. 모양을 바꾸려면 한글에서
해당 스타일 정의를 수정한다. 렌더링은 전체 파일 해시 허용 목록에 묶이지 않는다.

박스는 `{{md2hwp:begin:block.box}}`와 `{{md2hwp:end:block.box}}` 사이의
정확히 한 루트 문단이다. 본문 스타일의 글자처럼 취급되는 기존 표를 복제한다.
셀 안 `{{md2hwp:slot:box.content}}` 문단이 내용과 `block.box` 스타일을 지정한다.
기존 `출처: ` 문단은 장식으로 유지하며 그림 출처와 같은 스타일을 사용한다.
임의 표 구조를 지원하지 않으므로 제공된 박스의 셀·캡션 구조를 유지한다.

그림은 `{{md2hwp:begin:figure}}`와 `{{md2hwp:end:figure}}` 사이에 아래 세
문단이 순서대로 있어야 한다. 박스와 그림 범위는 겹칠 수 없다.

1. `{{md2hwp:slot:figure.image}}`: 본문 스타일의 그림 앵커.
2. `[그림 <한글 그림 자동번호>] {{md2hwp:slot:figure.caption}}`: 기존 자동번호
   컨트롤을 유지한 캡션. 숫자를 직접 입력해 대체하면 안 된다.
3. `{{md2hwp:slot:figure.source}}`: 바로 이어지는 출처 문단.

모든 필수 역할과 두 프로토타입은 원고의 사용 여부와 관계없이 검사한다.
알 수 없는 선언, 중복, 잘못된 범위·슬롯·스타일·설정은 출력 게시 전에 거부한다.
템플릿의 머리말·꼬리말 등에 별도 선언을 숨겨 두는 방식도 허용하지 않는다.

## 생성과 검증 근거

`author-tagged`는 조사된 원본 `minimal.hwp`에만 적용하는 재현 도구다.
해시와 원래 구조를 검사한 뒤 별도 출력에 태그를 작성한다. 일반 템플릿 편집기는 아니다.

```powershell
& 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode author-tagged -Template .\tests\fixtures\templates\minimal.hwp `
  -Output .\artifacts\my-tagged-template.hwp -Configuration Debug
```

추적된 파일의 SHA-256:

- 원본: `5CAEABF6C3BF1EE10B68FF810678374B0C775CB6A27B423F5326F3ED97080D55`
- 태그 템플릿: `263DA74FC009DBE491E7433607900DEFA66C3E94E38F98A93BC372E93AEC45F9`

2026-09-22, 기록된 Windows PowerShell 5.1 x64 STA 사용자 세션과 Hancom 2020에서
다음 원고를 실제로 생성하고 저장·재열기 검증했다.

- `examples/ir-v0.1.json`: 본문, 제목, 강조, 목록, 박스, 그림.
- `examples/commonmark-v0.1.expected.ir.json`: 기존 CommonMark 정규화 결과 재생.
- `tests/fixtures/ir/two-figures-v0.1.json`: 그림 두 개와 자동번호 1, 2.
- `tests/fixtures/ir/tagged-template-conformance-v0.1.json`: 문단 내부 줄바꿈,
  번호 목록 내부 줄바꿈, 시작 번호 3, 반복 박스와 빈 줄, 제목 2~6,
  태그 모양 원고 문자열, 마지막 제목 뒤 빈 글머리표 제거.

페이지 PNG를 직접 확인했다. 표지 PNG 해시는 모든 비교 문서에서
`8B12E3BC4909E9201617D853643F789BD7128F3ED67EA0B298BF7C6D5908D56B`로 같았다.
원본 고정 내용과 스타일 정의, 상대 도형 순서, 실제 테두리 속성을 비교한다.
한글이 저장 중 재배정하는 테두리 ID는 참조 대상을 비교하며, 색·선의 변경은
테스트에서 거부한다. 그림은 글자 취급, 저장된 가로·세로 크기 및 비율을 검사한다.
태그 없는 원본과 거부된 IR의 실행은 출력 파일을 남기지 않았다.

스타일 덮어쓰기 대화상자는 같은 스타일 재적용 전에 다른 스타일로 전환해
방지한다. 렌더링 위치의 실제 HWPML 문단 스타일을 읽어 판단한다.
박스 삽입 후 `GetDefault("Style")` 값만으로 판별하는 방법은 신뢰할 수 없었다.
대화상자를 자동 승인하거나 스타일 정의를 덮어쓰지 않는다.

COM 없는 테스트는 선언·소유 범위·설정과 구조 보존을 검사한다.

```powershell
pwsh -NoProfile -File .\tools\development\dotnet.ps1 run `
  --project .\tools\investigation\hancom-automation\template-declarations-tests\TemplateDeclarations.Tests.csproj
pwsh -NoProfile -File .\tools\smoke\test-contracts.ps1
pwsh -NoProfile -File .\tools\smoke\test-hancom-ir-preview-plan.ps1
```

## 지원 범위

이는 조사용 C# 실행기에 구현된 `minimal-1` HWP 렌더 경로다. 기존 외부 프로필
`render` 명령은 별도로 유지한다. 일반 AURI 전체 템플릿, HWPX, 정식 Rust 앱과의
worker 프로토콜 연결, 한글 필드·책갈피 방식과의 비교는 아직 완료하지 않았다.
기존 `experimental-1` 텍스트 범위 실험과 `minimal-1`은 다른 규약이다.

글꼴은 템플릿을 보존한다. 예제의 도시 이모지는 현재 템플릿 글꼴에서 네모로
표시되지만 Unicode 텍스트는 보존된다. 링크는 서식 있는 레이블만 출력하며,
PNG 리소스와 figure/source 메타데이터는 기존 직접 IR 계약을 따른다.
