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
.\md2hwp-backend.exe 원고.ir.json
```

첫 명령은 원고 옆에 `원고.ir.json`, `원고.result.hwp`를 생성합니다.
백엔드는 기존 IR에서 HWP만 생성합니다. 두 번째 인자는 선택적인 HWP 출력 경로입니다.
검증된 IR은 교체하며, HWP는 생성·재열기 검증 성공 후 기존 결과를 교체합니다.
실패한 HWP 생성은 이전 HWP를 보존합니다. 원고와 템플릿은 덮어쓰지 않습니다.
간편 실행의 이미지 경로는 원고/IR 폴더 안에서 해석합니다.

기존 옵션 방식도 사용할 수 있습니다.

```powershell
.\md2hwp.exe md2ir --from commonmark --input 원고.md --output 원고.ir.json --force
.\md2hwp.exe render-hwp --ir 원고.ir.json --output 원고.result.hwp
.\md2hwp-backend.exe render-tagged --ir 원고.ir.json --output 원고.result.hwp
```

`--template`과 Rust의 `--worker`를 생략하면 각 EXE 옆의 `template.hwp`,
`md2hwp-backend.exe`를 찾습니다. 옵션 방식의 이미지 기준 폴더는 작업 폴더입니다.
IR 0.1/0.2, 정규화 규칙과 스키마는 실행 파일에 포함됩니다.

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
