# 빌드와 검증

## 빌드

Rust 버전은 `rust-toolchain.toml`, .NET SDK는 `global.json`에 고정되어 있습니다.
Windows x64에서 PowerShell 7.4 이상과 Rust 도구 체인을 준비합니다.
저장소 루트에서 Windows 빌드 도구와 개발용 .NET SDK를 준비하고 빌드합니다.
MSVC·Windows SDK 설치가 필요한 경우 첫 명령은 관리자 PowerShell에서 실행합니다.

```powershell
pwsh -NoProfile -File dependencies/install-windows-build-tools.ps1
pwsh -NoProfile -File dependencies/install-dotnet-sdk.ps1
pwsh -NoProfile -File tools/development/build.ps1
cargo test --workspace
```

Windows 도구 설치 스크립트는 `dependencies/lock.json`의 `windows-build-tools`를 읽습니다.
Visual Studio Build Tools 2022의 검증 기준 버전과 공식 설치 출처, MSVC·Windows SDK의
기준 버전 및 설치 구성요소 ID를 이 파일에서 관리합니다. 현재 기준은 Build Tools
17.14.37614.0, MSVC 14.44.35207, SDK 10.0.26100.0입니다. MSVC·SDK는 기준 이상을
허용하며, Build Tools 제품 버전은 기록용입니다. 기존 적합한 전체 Visual Studio도
재사용하며, 설치된 버전을 낮추거나 자동 업데이트를 막지 않습니다.

상태 확인과 설치 계획은 시스템 변경 없이 실행할 수 있습니다.

```powershell
pwsh -NoProfile -File dependencies/install-windows-build-tools.ps1 -Check
pwsh -NoProfile -File dependencies/install-windows-build-tools.ps1 -WhatIf
```

적합한 설치가 있으면 다운로드·관리자 권한 없이 종료합니다. 부족하면 기존 Build Tools
2022에 두 구성요소를 추가하거나 새 Build Tools를 설치하며, 사용자의 전체 IDE는 수정하지
않습니다. C++ 작업 구성 전체나 추천 구성요소는 요청하지 않습니다. 새 설치에서는 공식
bootstrapper를 `.local/dependencies/windows-build-tools`에 다운로드하고 유효한 Microsoft
서명을 확인합니다. 이 URL은 최신 2022 설치 파일을 제공하므로 고정 파일 해시는 사용하지
않습니다. 실제 도구는 시스템에 설치되며 캐시 폴더만 복사해서 사용할 수 없습니다.
`-Check`는 필요한 도구가 없으면 실패합니다. 설치가 재부팅을 요구하면 종료 코드 3010을
반환하며, 재부팅 후 `-Check`에 성공한 것을 확인한 뒤 빌드하십시오.

설치 스크립트의 격리된 검사·명령 구성 테스트는 다음 명령으로 실행합니다.

```powershell
pwsh -NoProfile -File tests/dependencies/windows-build-tools.Tests.ps1
```

이 테스트는 임시 파일과 가상 설치 정보를 사용하며 실제 다운로드·설치는 하지 않습니다.
`-Check`도 버전과 필수 파일의 존재를 확인하는 검사이며 컴파일·링크 성공을 보장하지는
않습니다. 실제 설치 검증은 별도 Windows x64 VM에서 도구가 없는 상태, MSVC 또는 SDK만
있는 상태, 기존 Build Tools에 구성요소가 부족한 상태를 각각 시험합니다. 설치 후 새 셸에서
`-Check`와 x64 C/C++ 컴파일·링크, 프로젝트 빌드 및 `cargo test --workspace`를 확인합니다.
재실행 시 무변경 종료, 재부팅 요구 시 3010 반환과 재부팅 후 재검사도 확인합니다.
VM에서의 설치 시험은 격리된 검사 테스트와 별도로 기록해야 합니다.

설치 동작은 [Microsoft 명령행 설치 문서](https://learn.microsoft.com/en-us/visualstudio/install/use-command-line-parameters-to-install-visual-studio?view=vs-2022)와
[Build Tools 구성요소 목록](https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools?view=vs-2022)을 기준으로 합니다.

.NET 설치 스크립트는 `dependencies/lock.json`에 고정된 SDK를 다운로드하고 해시를 검증합니다.
`tools/development/dotnet.ps1`은 이 SDK를 실행하며, NuGet 패키지는
`.local/state/dotnet/nuget-packages`에 보관합니다. C# 빌드·게시의 자동 restore는
`backends/hancom-automation/NuGet.Config`에 명시된 공식 nuget.org 소스만 사용합니다.
캐시에 있는 패키지는 재사용하고 없는 패키지는 다운로드하므로, 최초 빌드나
의존성 변경 후에는 네트워크 연결이 필요합니다. 별도 패키지 다운로드 스크립트나
기존 캐시 복사는 필요하지 않습니다. SDK·캐시·빌드 결과는 Git에 포함하지 않습니다.

Release 빌드 결과는 `target/release/`의 다음 파일과 라이선스 폴더입니다.
`-Configuration Debug`를 주면 `target/debug/`에 모입니다.
개발·실행에는 이 두 폴더를 기본으로 사용합니다. `target/dist/`는 버전별 배포 파일과 ZIP의 보관용입니다.

- `md2hwp.exe`
- `md2hwp-rhwp.exe`
- `md2hwp-hancom.exe`
- `hwp2pdf.exe`
- `template.hwp`
- `README.md`
- `AGENTS.md`
- `README-MANUSCRIPT.md`
- `LICENSE`
- `THIRD-PARTY-NOTICES.txt`
- `licenses/`

일반 Debug·Release 빌드는 두 백엔드를 모두 생성하며 ZIP에도 함께 넣습니다. 기존 `-Rhwp` 옵션은 호환용으로 계속 허용합니다. 빌드는 선택한 `target/debug` 또는 `target/release` 바로 아래의 구형 `md2hwp-backend.exe` 파일만 정리하며 배포 보관 폴더는 변경하지 않습니다.

C# 백엔드는 `backends/hancom-automation/Md2Hwp.Backend.csproj`, PDF CLI는 `apps/hwp2pdf/Hwp2Pdf.csproj`입니다.
PDF CLI는 백엔드의 COM 세션·보안 검사 코드를 재사용하고, 게시 시 필요한 관리 어셈블리를 EXE 안에 묶습니다.
`hwp2pdf.exe` 실행에는 별도 백엔드 EXE나 DLL 파일이 필요하지 않습니다.
`templates/template.hwp`를 게시 폴더와 `target/debug` 또는 `target/release`에 복사합니다. 빌드 폴더의 템플릿은 매번 강제 교체합니다. 기본 템플릿은 `templates/template.hwp`에서 편집하고, 별도 사용자 템플릿은 `target/` 밖에 두어 `--template`으로 지정하십시오.
빌드 결과는 Git에서 제외합니다. 배포할 때 위 파일과 라이선스 폴더를 함께 복사합니다. 동봉 README의 원고 작성 안내 링크는 같은 폴더의 `README-MANUSCRIPT.md`로, 나머지 문서·예제 링크는 GitHub의 해당 버전으로 연결합니다.

원고 작성 지침은 `docs/manuscript/AGENTS.md`, 같은 규칙의 사람용 요약은 `docs/manuscript/README-MANUSCRIPT.md`에서 관리하며 두 문서를 함께 갱신합니다. 빌드는 두 파일을 배포 폴더 최상위에 복사합니다. 저장소 루트의 개발용 `AGENTS.md`는 배포하지 않습니다.

배포 ZIP을 만들려면 다음 명령을 사용합니다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Release -Package
```

`target/dist/md2hwp-v<버전>-windows-x64.zip`에 실행 파일 네 개, 추적 기본 템플릿, README와 원고 작성 문서 두 개, 프로젝트 라이선스·제3자 고지 및 `licenses/`를 넣고 SHA-256을 출력합니다.
패키지는 사용자 편집본 대신 `templates/template.hwp`를 사용합니다. 기존 배포 ZIP은 새 ZIP 생성에 성공한 뒤 교체합니다.
GitHub Release에는 실행용 ZIP만 첨부하고, 체크섬은 본문 끝에 표시합니다. 소스는 GitHub 자동 다운로드를 사용하며 릴리스 노트·빌드 매니페스트는 별도 첨부하지 않습니다.

AGPL 실행 파일에는 정확히 대응하는 소스를 제공해야 합니다. 배포 페이지에 동일 태그의
무료 소스 다운로드 링크를 표시하고, 태그가 실제 빌드 소스·잠금 파일·스크립트·필수 입력을
포함하는지 확인하십시오. 라이선스 적용 전의 기존 v0.4.1 태그는 이후 수정한 빌드의 대응
소스로 사용할 수 없습니다. v0.4.2는 변경된 라이선스와 고지를 포함한 새 릴리스입니다.
제3자 자료는 공개 배포 전에 권한을 확인하십시오.

의존성을 변경하면 로컬 Cargo 캐시를 준비한 뒤 고지를 갱신하고 검토합니다.

```powershell
pwsh -NoProfile -File tools/development/update-third-party-notices.ps1
pwsh -NoProfile -File tools/development/update-third-party-notices.ps1 -Check
```

빌드는 고지가 현재 잠금 버전과 일치하는지 확인합니다. 알려지지 않은 라이선스 표현이나
고지 누락은 명시적으로 실패합니다. Rust 도구 체인이나 .NET 호스트 버전을 변경할 때는
`licenses/`의 해당 원본 라이선스·고지도 실제 버전에 맞게 갱신해야 합니다.

추가 계약 검사:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
pwsh -NoProfile -File tools/development/dotnet.ps1 run --project tests/backend-contract/Md2Hwp.Backend.Tests.csproj
```

계약 테스트는 실제 한글 실행이나 시각적 배치 검증을 대신하지 않습니다.

## 생성 방식 비교

한글 백엔드의 Debug와 Release 기본 생성 방식은 IR·템플릿 XML 구성입니다. Debug 한글 백엔드는
`--legacy-com`으로 기존 COM 문단 삽입 방식을 선택할 수 있습니다. 이 옵션은
Rust 실행 파일의 원고 변환·`ir2hwp`와 백엔드 직접 실행에서 모두 전달됩니다.
레거시 옵션과 안내는 Debug 실행 파일에만 있습니다. Rust를 거칠 때는 앱과
백엔드 모두 Debug 빌드를 사용하십시오. Release에는 레거시 인자 처리·안내와
상세 계측 코드가 컴파일되지 않습니다.

속도 개선은 Debug 기본 XML 경로에서 진행하고, Debug의 기존 COM 경로를 비교 기준으로
사용합니다. Release에서는 전체 실행 시간만 측정합니다.

같은 IR과 템플릿으로 세 결과를 별도 경로에 생성하십시오. 한글 작업은 차례로 실행합니다.

```powershell
.\target\debug\md2hwp-hancom.exe source.ir.json debug-xml.hwp --template templates/template.hwp
.\target\debug\md2hwp-hancom.exe source.ir.json debug-com.hwp --template templates/template.hwp --legacy-com
.\target\release\md2hwp-hancom.exe source.ir.json release-xml.hwp --template templates/template.hwp
```

원고에서 시작할 때도 `md2hwp.exe source.md debug-com.hwp --worker .\target\debug\md2hwp-hancom.exe --legacy-com`처럼 지정합니다.
Debug의 네이티브 복제·import 추가 진단은 두 생성 방식에 유지합니다.
기존 COM 삽입의 중간 저장·재열기 검사는 `--legacy-com` 경로에 적용합니다.
실제 비교 결과는 [세 생성 방식 검증](performance/2026-10-09-generation-modes.md)에 기록합니다.

Debug의 상세 단계·작업·XML export 계측은 `MD2HWP_PROFILE=1`로 켭니다.
두 Debug 경로에 동일하게 적용되며, 출력은 stderr의 `md2hwp-profile:` JSON입니다.
Release에서는 이 환경 변수를 사용하지 않습니다.

전체 생성 시간은 Debug·Release 모두 `MD2HWP_TIMING=1`로 켤 수 있습니다.
stderr의 `md2hwp-time:` JSON에 `Completed`와 `TotalMilliseconds`만 출력합니다.
실행 파일 시작 시간을 포함하려면 PowerShell의 `Measure-Command`를 사용하십시오.

```powershell
$env:MD2HWP_TIMING = '1'
try {
    .\target\release\md2hwp.exe source.md
} finally {
    Remove-Item Env:MD2HWP_TIMING
}
```

현재 계측 구분과 출력 검증은 [Debug 전용 계측](performance/2026-10-09-debug-profiling.md)에 기록합니다.

## 기능 검증 원고

[`examples/all-features/`](../examples/all-features/)에는 짧은 검증 원고
`all-features.md`, 예상 IR `all-features.ir.json`, PNG/JPG/JPEG/EMF 리소스를 함께 보관합니다.
AURI 편집양식에 수록된 세계유산 공간관리 보고서의 논의를 요약·재구성하며,
같은 기능을 두 번 반복하는 기존 원고를 대체합니다. 장 간 참조·번호 재시작을 위해
내용이 다른 두 장을 두고, 장마다 여러 절과 한 절의 여러 하위 제목으로 each.child를 확인합니다.
제목 1~6단계, 강조·줄바꿈·링크·중첩 목록, 제목 있는/없는/제목만 있는 글상자,
그림 6종과 표, 다중 하단 설명, 각주와 제목·그림·표 번호 참조를 포함합니다.
각주 위치·참조 경계·출처 및 캡션 생략 등의 상세 목록은 해당 폴더 README에 있습니다.

YAML에는 제목·부제·저자 목록·날짜·발행처·사용자 문자열과 장 시작 번호 3을 지정합니다.
메타데이터 출력은 해당 `meta` 태그를 둔 템플릿에서 확인하며, `date-meta`의 날짜 포맷과
태그 문구를 담은 사용자 값의 비재귀 치환도 해당 태그를 넣어 확인할 수 있습니다.
헤딩 참조는 원고 순서로 계산하므로 제목의 한글 개요 설정은 필요하지 않습니다. 기본 템플릿과 `init-template`은 여섯 수준의 한 문단 참조 블록을 제공합니다. 옛 공통 `ref.heading.number` 양식은 여섯 블록으로 바꾸십시오.
각주가 있는 heading6 슬롯은 머리말·꼬리말·바탕쪽에 복제할 수 없습니다.
박스 제목을 분리하려면 `slot:code.title`이 필요합니다. 실제 각주 번호와 표·박스의
잘림·겹침은 생성된 HWP의 렌더링으로 확인해야 합니다.

저장소 루트에서 빌드한 프로그램으로 실행합니다.

```powershell
.\target\release\md2hwp.exe .\examples\all-features\all-features.md
```

원고 옆의 IR을 갱신하고 `all-features.output.hwp`를 생성합니다.
IR은 Git에 포함하며, 생성된 HWP는 Git에서 제외합니다.
IR 변환만 확인하려면 다음 명령을 사용합니다.

```powershell
.\target\release\md2hwp.exe md2ir .\examples\all-features\all-features.md --force
```

대규모 본문용 원고와 IR은 [`examples/korean-lorem/`](../examples/korean-lorem/)에 있습니다.
Ipsum 본문 11,155어절을 5장·20절로 나누었으며, 작성 당시 최대 검증 원고의
텍스트 2,231어절을 기준으로 본문 분량을 5배로 맞췄습니다. AURI 예시 보고서의 본문 대비 빈도를
참고해 그림 29개·표 39개·박스 14개·각주 66개와 목록·메타데이터·상호참조 등
현재 지원 문법을 추가했습니다. 빈도와 템플릿 조건은 해당 폴더의 README에 설명하며,
이 대규모 원고는 한글 변환·저장·재열기 검증을 통과했습니다. 최초 111쪽의 시각 문제를
확인한 뒤 강조 문법과 표 머리행 고립을 수정했으며, 보정된 112쪽 출력의 검증도 기록했습니다. 문단 스타일 조회를 선택 블록으로 바꾼 비교 실행은 779.527 → 646.964초였으며,
이전과 112쪽 PNG가 모두 동일했습니다. 이후 호출 수·시간·단축 결과는
[성능 검증 기록](performance/README.md)에 정리했습니다.

[README로 돌아가기](../README.md)

## 성능 측정 기준

성능 개선의 최종 효과·검증과 정리된 커밋의 역할은
[성능 이력·#7 해결 근거](performance/README.md)에 정리합니다.
이전 보고서의 해시는 측정 당시 이력이며, 현재 커밋 대응과 되돌리기 안내도 이 문서를 따릅니다.

현재 한글 백엔드의 Debug와 Release는 IR·템플릿 XML에서 평면 문서를 직접 구성합니다. [성능·출력 비교](performance/2026-10-09-direct-xml.md)와
[집계 JSON](performance/2026-10-09-direct-xml.json)에 일반 실행 209.830 → 121.349초,
COM 메서드 계측 35,025 → 22,360회 감소를 기록했습니다. 문단 스타일 블록 조회는
473 → 1회, 전체 XML export는 19 → 14회로 줄었습니다. 기존 Debug 생성 경로와 최종
저장·재열기 검증을 유지하며, 대규모 107쪽과 모든 기능 원고 17쪽의 구조·서식·내장 이미지·
전체 픽셀이 일치했습니다. 기존 템플릿의 긴 표 clipping과 간헐적 초기 import 높이 오류는
동등성 결과와 별도로 보고서에 기록합니다. 원시 증거는
`artifacts/performance-direct-xml-2026-10-09/`에 보관합니다.

직전 Release의 XML 재사용·반복 비교 최적화는 [성능·출력 비교](performance/2026-10-09-xml-comparison.md)와
[집계 JSON](performance/2026-10-09-xml-comparison.json)에 있습니다. 같은 대규모 원고의 일반 Release가
277.016 → 209.830초로 단축됐고, 전체 XML 조회는 37 → 19회로 줄었습니다.
캐시를 단발 조회까지 적용해 처음에는 느려졌던 실험과 적용 범위 수정도 기록했습니다.
기존 Debug와 새 Release의 구조·내장 이미지 데이터 및 107쪽 전체 픽셀이 일치했습니다.
문단 스타일 조회와 저장·재열기 검증은 유지하며, 원시 로그는
`artifacts/performance-xml-comparison-2026-10-09/`에 보관합니다.

직전 Release의 원형 복제 좌표 추적 변경은 [성능·출력 비교](performance/2026-10-09-clone-coordinates.md)와
[집계 JSON](performance/2026-10-09-clone-coordinates.json)에 있습니다. 전체 XML 조회 140회를 줄였고,
기존 Debug와 새 Release의 구조·내장 이미지 데이터 및 107쪽 전체 픽셀이 일치했습니다.
원시 로그는 `artifacts/performance-coordinates-2026-10-09/`에 보관합니다.

직전 Release의 기준 측정은 [2026-10-09 현재 Release 상세 보고서](performance/2026-10-09-current-release.md)와
[집계 JSON](performance/2026-10-09-current-release.json)에 있습니다. 실제 배포 빌드의 시간과
별도 계측 빌드의 호출별 시간을 구분했으며, 원시 로그와 CSV는
`artifacts/performance-release-2026-10-09/`에 보관합니다.

초기 Release의 액션별 기준 측정은 [2026-10-08 성능 보고서](performance/2026-10-08-baseline.md)와
[비교용 JSON](performance/2026-10-08-baseline.json)에 보관합니다. 호출별 원시 시간선과 CSV는
`artifacts/performance-2026-10-08/`에 있습니다. 대규모 원고는 네이티브 자동번호 검증에서
실패했으므로 실패 지점까지의 비용이며, 정상 완료된 작은 원고와 구분해서 비교하십시오.

각주 번호 추적을 수정한 뒤 같은 대규모 원고가 저장·재열기 검증까지 완료했습니다.
최적화 작업은 [수정 후 완료 측정](performance/2026-10-08-large-remeasured.md)과
[집계 JSON](performance/2026-10-08-large-remeasured.json)을 기준으로 진행하십시오.
전체 실행은 530.699초이며, 호출별 원시 자료는
`artifacts/performance-remeasure-2026-10-08/`에 있습니다.

첫 Release 최적화는 [Debug·Release 동등성 및 시간 비교](performance/2026-10-09-release-diagnostics.md)에
기록했습니다. 변경 전 두 빌드, 그리고 변경 전 Debug와 수정 Release의 문서 구조·서식·참조 및
107쪽 전체 렌더링이 일치했습니다. 그림·박스·출처 치환 중 검증용 전체 XML export만 Debug에
남기고 완성 문서 검증은 두 빌드에 유지하여, 같은 원고의 Release 실행이 511.752 → 383.004초로
줄었습니다. 기존 Debug 실행 파일과 세 실행 결과는 `artifacts/debug-release-equivalence/`에 보존합니다.
