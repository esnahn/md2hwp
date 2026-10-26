# 로컬 파일과 저장소 복원

tracked 소스·템플릿·예제·시험 입력과 유지 중인 설치 스크립트가 제품 재구성의 기준이다.

## 유지 범위

| 경로 | 역할 | 새 환경에서 준비하는 방법 |
| --- | --- | --- |
| `.local/dependencies/dotnet/` | 설치된 개발 SDK | `dependencies/install-dotnet-sdk.ps1` |
| `.local/dependencies/pandoc/` | 설치된 Pandoc | `md2hwp setup-pandoc` |
| `.local/dependencies/rhwp-source/` | 사용자가 보존하는 별도 rhwp checkout. 현행 Cargo 의존성과는 별개 | 현재 checkout의 Git 원격·커밋·작업 내용을 기준으로 별도 관리 |
| `.local/state/dotnet/` | SDK 작업 상태·NuGet 캐시 | 빌드가 생성하고 공식 NuGet 소스에서 복원 |
| `target/debug/`, `target/release/` | 현재 실행파일·템플릿·배포 문서 | `tools/development/build.ps1` |
| `target/dist/` | 보관 중인 릴리스 배포본 | 해당 태그에서 빌드·패키징. ZIP 바이트 동일성은 보장하지 않음 |
| `reference/` | 사용자가 보존하는 보고서·서체·API 원본 | `reference/README.md`의 출처·해시로 별도 보존/준비. 빌드 입력은 아님 |
| `dependencies/FilePathCheckerModuleExample.dll` | 현재 한글에 등록된 보안 모듈 | 사용자가 한컴 안내에 따라 설치·등록. 파일 이동·삭제 시 등록 경로도 영향을 받음 |

Rust는 rustup과 `rust-toolchain.toml`로, MSVC·Windows SDK는
`dependencies/install-windows-build-tools.ps1` 및 `dependencies/lock.json`으로 준비한다.
설치된 SDK·캐시·빌드 결과는 계속 ignored로 둔다. 이를 Git에 넣어야 복원되는 구조는 아니다.
참고 원본과 보안 모듈의 배포 권한은 프로그램 소스의 라이선스와 별개다.

## 복원 검증 범위

tracked checkout과 설치된 dependency만 둔 별도 worktree에서 빌드를 시험했다.
누락 NuGet 패키지는 공식 소스를 명시한 설정으로 자동 다운로드되어 Release 빌드에 성공했다.
C# EXE 두 개는 기존 배포 파일과 바이트가 같았다. Rust EXE에는 timestamp/GUID 및
정적 데이터 링크 배치 차이가 있었고, ZIP 항목 시각과 일부 라이선스 줄바꿈도 달랐다.
따라서 빌드 복구 가능성과 배포 파일 전체의 바이트 재현성은 구분한다.
이 시험은 같은 PC의 Rust·MSVC를 사용했으며, 깨끗한 Windows에서의 실제 설치 시험은 하지 않았다.
