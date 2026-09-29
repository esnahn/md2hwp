# md2hwp v0.1.0

지원하는 CommonMark 원고를 IR JSON으로 정규화한 뒤, 태그가 있는 기존 HWP
템플릿을 한글 COM으로 편집합니다. 현재 지원 대상은 Windows x64와 HWP입니다.
HWPX, 생성 표, 각주, RST 직접 입력은 이번 배포 범위에 포함하지 않습니다.

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
실패한 HWP 생성은 이전 HWP를 보존합니다. 원고와 템플릿은 덮어쓰지 않습니다.
간편 실행의 이미지 경로는 원고/IR 폴더 안에서 해석합니다.
간편 실행은 출력 경로를 위치 인자 또는 `--output`으로 받습니다. 두 방식을 중복 지정할 수는 없습니다.
Rust 간편 실행은 `--template`, `--worker`, `--dotnet`도 받으며, 백엔드는 `--template`을 받습니다.
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
현재 IR `0.2`, 정규화 규칙과 스키마는 실행 파일에 포함됩니다.
입력 IR과 템플릿의 IR 버전은 프로그램의 현재 IR 버전과 정확히 같아야 합니다.
변환 규칙과 템플릿에는 별도의 계약 버전을 두지 않습니다.

## 기본 서식 템플릿 만들기

```powershell
.\md2hwp.exe init-template 새템플릿.hwp
.\md2hwp-backend.exe init-template 새템플릿.hwp
```

두 명령은 같은 기능입니다. 경로를 생략하면 **현재 작업 폴더**에 `template.hwp`를
만들며, `--output <경로.hwp>`도 지원합니다. 기존 파일은 덮어쓰지 않습니다.
Rust 명령은 `--worker`, `--dotnet`도 지원합니다.

기존 템플릿 파일 없이 한글의 새 빈 문서 기본 서식에서 시작합니다. 현재 IR의
필수 선언, 제목 1~6단계·본문 스타일, 단일 셀 박스와 출처, 그림 자리·자동번호 캡션·출처를
구성하고 저장·재열기 및 프로토타입 검증이 성공한 파일만 내보냅니다.
제목을 포함한 역할별 `md2hwp.*` 스타일은 처음에는 같은 기본 서식이며,
생성한 템플릿에서 각 스타일을 편집해 문서 디자인을 정할 수 있습니다.
템플릿 정의 영역은 변환 결과에서 제거됩니다.

```text
{{md2hwp:begin:template}}
{{md2hwp:ir-version:0.2}}
스타일 선언과 박스·그림·출처 원형
{{md2hwp:end:template}}
{{md2hwp:content}}
```

이번 원고에서 사용하지 않더라도 해당 IR이 지원하는 모든 요소의 스타일·슬롯·원형이
필요합니다. IR 버전이 바뀌면 원고에서 IR을 다시 만들고 `init-template`으로 새 템플릿을
생성해 꾸밉니다. 구버전 IR 및 `samples`/`minimal-1`/`source-label` 템플릿은 지원하지 않으며,
프로그램이 기존 사용자 템플릿을 자동으로 수정하지 않습니다.

그림 캡션은 `{{md2hwp:slot:figure.caption}}`이 들어 있는 **한 문단 전체**를
원형으로 복제합니다. 같은 문단에 실제 한글 그림 자동번호 개체가 하나 있어야 합니다.
`[그림 자동번호] 설명슬롯`, `Figure 자동번호. 설명슬롯 (참고)`처럼 앞뒤 문구와
자동번호 모양을 템플릿에서 바꿀 수 있으며, 설명 슬롯이 번호 앞에 있어도 됩니다.
코드는 설명 슬롯만 원고의 캡션으로 치환합니다. 별도의 캡션 begin/end 태그는 없습니다.
설명 슬롯은 한 번만 사용하고, 캡션 안의 추가 문단·강제 줄바꿈·다른 개체는 지원하지 않습니다.

박스와 그림 출처도 각각 한 문단을 원형으로 사용합니다. 새 템플릿에는 다음 슬롯이
들어 있으며, 슬롯 앞뒤의 문구와 각 문단의 스타일을 별도로 편집할 수 있습니다.

```text
자료: {{md2hwp:slot:box.source}} 제공
Source — {{md2hwp:slot:figure.source}}
```

박스 출처는 박스 표의 네이티브 캡션 안에, 그림 출처는 그림 캡션 다음 문단에 둡니다.
원고의 출처가 없으면 박스 캡션 영역 또는 그림 출처 문단 자체를 생략합니다.
마크다운에서는 박스 코드 블록이나 독립된 그림 문단 다음에 `출처: 내용` 문단을 둡니다.
그림과 출처 사이에는 빈 줄을 넣어 별도 문단으로 구분합니다.
머리말은 각 출처 슬롯 앞뒤의 문구로 지정합니다.

생성에는 한글·보안 모듈·.NET 런타임이 필요하며 Pandoc은 필요하지 않습니다.
두 EXE만 배포한 뒤 이 명령으로 템플릿을 만들 수도 있습니다.
생성 파일을 EXE 옆의 `template.hwp`로 두면 변환 시 자동으로 찾습니다.
다른 위치의 템플릿은 `ir2hwp`의 `--template`으로 지정합니다.

## 사용자 환경

- 한글 설치 및 로그인된 대화형 Windows 세션이 필요합니다. 기준 환경은
  한글 2020 HWP 11.0.0.9136입니다. 실행 중인 한글 문서는 먼저 닫습니다.
- .NET 10 x64 런타임이 필요합니다. Rust는 실행 전에 확인하고 없으면
  [공식 설치 페이지](https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0)를 안내합니다.
  백엔드 직접 실행 시 별도 설치 런타임은 `DOTNET_ROOT_X64`로 지정합니다.
- 한글 보안 모듈은 사용자가 [공식 안내](https://developer.hancom.com/hwpautomation)에
  따라 설치·등록합니다. 앱은 설치·등록을 변경하지 않으며 RegisterModule 성공을 요구합니다.
- Pandoc은 `md2hwp setup-pandoc`으로 공식 배포본을 다운로드할 수 있습니다.
  `%LOCALAPPDATA%\md2hwp\pandoc`에 원본 ZIP·문서·저작권 고지를 보관합니다.
  탐색 순서는 `--pandoc`, 관리 다운로드, PATH입니다. 변환 중 자동 설치는 하지 않습니다.
  다운로드 명령은 빌드 시 lock의 버전을 우선하고, 없으면 공식 최신 안정판을 확인합니다.
  버전 번호 대신 Pandoc JSON 계약의 호환성을 검사합니다.

## v0.1.0 저장소 정리

빌드에 불필요한 탐색 스크립트, 외부 프로필, 과거 설계·검증 문서와 독립 스모크
테스트는 Git 이력으로 남겼습니다. 현재 코드·빌드 설정·템플릿과 Rust 테스트에서
직접 포함하는 JSON fixture만 유지합니다. C#의 일부 클래스 이름은 이전 탐색 시기의
이름을 유지하지만 현재 렌더러가 사용하는 코드입니다. 과거 탐색 명령은 제공하지 않습니다.
`dependencies/lock.json`은 개발 재현성 메타데이터이며 배포 파일이 아닙니다.
한글 보안 모듈 개발 pin은 유지하되 사용자 런타임에서는 강제하지 않습니다.
