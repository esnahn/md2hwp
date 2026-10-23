# 백엔드 선택과 배포 검증 — 2026-10-23

사용자가 rhwp의 남은 제한과 JPEG 재인코딩 차이를 수용한 뒤 기본 선택을 변경했다.
IR 0.4와 버전 0.4.4 메타데이터는 유지했다. 이번 변경은 개발 브랜치의 동작이며,
기존 v0.4.4 태그나 GitHub Release를 바꾸지 않았다.
구현 커밋은 `c6efe8e`이며, 아래 검증은 해당 변경을 포함한 빌드로 수행했다.

## 선택 규칙

- `--worker`를 명시하면 그 경로를 사용한다. 없거나 실행에 실패해도 자동 전환하지 않는다.
- 생략하고 템플릿·출력이 HWP이면 `md2hwp.exe` 폴더의 `md2hwp-rhwp.exe`를 먼저 찾는다.
  이 파일이 없을 때 `md2hwp-hancom.exe`를 찾는다.
- 템플릿 또는 출력 확장자가 HWPX이면 `md2hwp-hancom.exe`만 자동 탐색한다.
  확장자 대소문자는 구분하지 않는다. 명시한 rhwp에 HWPX를 전달하면 기존대로 거부한다.
- 변환·템플릿 생성·검사에 같은 규칙을 적용한다. `--version`은 형식 입력이 없으므로
  rhwp→Hancom 순서를 사용한다.
- `--dotnet`은 Hancom에만 적용한다. `check-runtime`은 한글 백엔드의 .NET 점검 명령으로 유지한다.
- 선택은 파일 존재 기준이다. 선택한 EXE가 손상되었거나 의존성이 없거나 문서 처리가
  실패한 경우에도 다음 백엔드로 재시도하지 않는다.

## 실행파일과 빌드

한글 백엔드의 AssemblyName과 실행파일을 `md2hwp-backend`에서 `md2hwp-hancom`으로
변경했다. 프로젝트 경로와 네임스페이스는 유지한다. hwp2pdf는 여전히 필요한 관리
어셈블리를 자체 EXE에 포함하며 별도 백엔드 파일이 필요하지 않다.

일반 Debug·Release 빌드는 두 백엔드를 모두 만든다. 기존 `-Rhwp`는 호환용으로 유지한다.
ZIP 구성은 md2hwp·rhwp·Hancom·hwp2pdf EXE 네 개와 기존 템플릿·문서·라이선스다.
이번에는 기존 버전 ZIP을 덮어쓰지 않도록 패키지 생성이나 릴리스를 실행하지 않았다.
빌드가 선택한 target/debug 또는 target/release 바로 아래의 구형 worker EXE만 정리한다.

## 검증

- Rust workspace 291개 테스트, Clippy `-D warnings`, fmt, C# 계약 검사를 통과했다.
- PowerShell 자동완성 79개 assertion을 통과했다. 옵션 집합은 바뀌지 않았다.
- 정상 Debug·Release 빌드를 통과했고 두 폴더 모두 새 이름의 EXE 네 개가 있다.
- 별도 폴더에서 각 구성당 12개 사례를 통과했다: 양쪽 존재·Hancom만 존재·rhwp만 존재·
  모두 없음, HWPX에 Hancom 없음, 명시 Hancom·명시 누락 경로·명시 rhwp/HWPX,
  기본 템플릿 검사·생성·IR 변환·Markdown 변환.
- 단독 hwp2pdf.exe의 버전 실행도 성공했다. 실제 PDF 변환·시각 비교를 다시 한 것은 아니다.
- 한글 2020으로 HWPX 템플릿 생성·검사, 기존 검증 HWPX 템플릿→HWP,
  추적 HWP 템플릿→HWPX, Hancom만 있는 폴더의 HWP 변환을 확인했다.
  실제 변환은 기존 저장·재열기 검증을 통과했고 COM은 순차 실행·정상 종료했다.
- 추적 템플릿은 변경하지 않았다. SHA-256:
  `20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`.

검증 자료는 ignored `artifacts/backend-selection-2026-10-23/`에 있다.
`check.ps1`과 debug/release의 `results.json`, `check-hancom.ps1`,
release의 `hancom-results.json`·`hancom-additional-results.json`을 보관했다.

## 별도 발견: 새 Hancom HWPX 기본 템플릿

자동 선택한 Hancom의 init-template과 validate-template은 새 HWPX 파일에서 성공했다.
그러나 이 새 템플릿으로 원고를 생성할 때, 작업용 HWP로 변환하는 첫 구조 검사에서
실패했다. 같은 입력을 md2hwp-hancom.exe에 직접 전달해도 동일하게 재현했다.

- 코드 원형 표 높이: 1282→2882
- 그림 원형 캡션 LastWidth: -1→40252
- 3행 표 원형 높이: 1282→3846

이는 `TaggedTemplateWriter.cs`의 HWPX→HWP 정규화 검사에서 원고 합성 전에 발생한다.
새 템플릿 내부 값이 해당 원형과 일치한다. DefaultTemplateWriter는 표 행 수와 캡션
내용을 작성하지만 이 레이아웃 값의 후속 HWP 변환 안정성은 검사하지 않는다.
선택·실행파일명 변경은 이 작성 로직을 수정하지 않았다.

기존 검증 HWPX 템플릿과 추적 HWP 템플릿의 HWPX 출력은 성공했다.
따라서 백엔드 선택 자체의 실패와 구분한다. 이번 변경에서는 구조 검사를 완화하거나
템플릿 생성 로직을 바꾸지 않았다. 실패한 두 시도의 결과 파일은 게시되지 않았다.
