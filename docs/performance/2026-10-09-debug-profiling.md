# Debug 전용 상세 계측과 Release 전체 시간 측정

구현 커밋 `faa8dcf` 기준입니다. 앞으로 속도 개선은 Debug 기본 XML 생성 경로에서
진행하고, Debug의 `--legacy-com` 경로를 비교 기준으로 사용합니다. Release는
상세 계측 없이 전체 시간만 측정합니다.

## 구성 분리

- `RenderProfile`과 단계·작업별 타이머 호출은 `#if DEBUG` 안에만 컴파일합니다.
- Debug의 XML 읽기는 기존 export·parse 시간과 횟수, XML 크기를 기록합니다.
- Release의 XML 읽기는 직접 export·parse하며 계측용 호출 위치 인자도 없습니다.
- 레거시 옵션·안내·렌더링 인자는 Rust·C# 모두 Debug에만 있습니다.
- 저장·재열기와 최종 구조 검증은 두 구성에 유지합니다.

`MD2HWP_PROFILE=1`은 Debug의 상세 계측을 켭니다. Release는 이 변수를 사용하지 않습니다.
`MD2HWP_TIMING=1`은 두 구성의 전체 생성 시간을 stderr에 출력합니다.

```text
md2hwp-time: {"Completed":true,"TotalMilliseconds":34167.078}
```

`Completed`와 `TotalMilliseconds`만 포함하며 단계별 데이터는 없습니다.
이 시간은 백엔드 생성 함수의 범위입니다. 실행 파일 시작·Rust 원고 변환까지 포함한
시간은 PowerShell `Measure-Command` 등 외부 도구로 측정합니다.

## 검증

Rust 전체 테스트·Clippy, Release CLI 테스트와 Debug·Release 백엔드 계약 테스트를 통과했습니다.
계약 검사는 Release 어셈블리에 상세 계측 클래스와 레거시 인자가 없음을 확인하고,
전체 시간 출력의 성공·실패 상태와 Debug 상세 계측 유지도 검사합니다.

실제 배포 실행 파일의 도움말 및 오류 출력에서 Release 레거시 안내가 없음을 확인했습니다.
두 Release 실행 파일의 UTF-8·UTF-16 데이터에서도 `--legacy-com`, `MD2HWP_PROFILE`,
`md2hwp-profile:`, `phase.compose-flat`, `ExportedUtf16CodeUnits` 문자열이 없습니다.

같은 기능 원고와 템플릿으로 Debug XML과 Release XML을 새로 생성했습니다.
두 실행 모두 `MD2HWP_PROFILE=1`과 `MD2HWP_TIMING=1`을 설정했습니다.
Debug에서는 상세·전체 시간 로그가 모두 나오고, Release에서는 전체 시간 로그만 나옵니다.

| 방식 | 백엔드 전체 시간(초) | 실행 파일 시작·종료 포함(초) | 상세 계측 |
| --- | ---: | ---: | --- |
| Debug XML | 43.912 | 44.626 | 유지, 29개 항목 |
| Release XML | 34.167 | 34.451 | 없음 |

단일 실행의 확인용 수치이며 성능 개선률을 산정한 반복 측정은 아닙니다.
이번에는 17쪽 기능 원고를 재검증하며, 대규모 원고는 다시 생성하지 않았습니다.
이전 107쪽 비교는 [세 방식 검증](2026-10-09-generation-modes.md)에 보존합니다.

출력은 앞선 검증에서 보존한 Debug 레거시 결과를 기준으로, 새 Debug·새 Release와
새 두 출력 사이를 각각 비교합니다. 구조·서식·참조 대상·내장 이미지와 144dpi 17쪽
전체 페이지 픽셀이 일치합니다. 원고·IR·템플릿은 보존했습니다.

[검증 집계 JSON](2026-10-09-debug-profiling.json)에 타이밍·Debug 프로파일·비교 결과와
실행 파일 해시를 저장합니다. 원시 자료는 `artifacts/debug-only-profiling-2026-10-09/`에 있습니다.

현재 실행·환경 변수 설정 방법은 [개발 안내](../development.md#생성-방식-비교)를 참고하십시오.
