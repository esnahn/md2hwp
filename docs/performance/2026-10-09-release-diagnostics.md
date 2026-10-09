# Debug·Release 결과 동등성 및 첫 Release 최적화 — 2026-10-09

소스 변경 전에 `804a5bcc9b719ba039ad5cd1afe944d35fb594c4`의 Debug와 Release를 각각 빌드·변환해 비교했습니다. 내용·서식·상호참조 대상·내장 데이터가 일치했고, 144dpi로 렌더링한 107쪽 모두의 RGB 픽셀도 일치했습니다. 이 비교가 끝난 후 최적화 커밋 `fed99cd71b591bdf60b2e174ca813e0e1f98c557`를 적용했습니다.

수정 후 Release 역시 보존한 수정 전 Debug 결과와 같은 XML 구조·서식·참조 관계·내장 데이터를 가지며, 107쪽 전체 픽셀이 일치했습니다. HWP/PDF 파일의 바이너리 일치가 아니라 문서 내용·서식·참조 의미와 렌더링 일치입니다. 한글이 매 실행 부여하는 내부 ID는 문서 내 대응 위치로 비교했습니다.

## 변경 범위

- `AuriMinimalCaptionPrototype.ReplaceCloneCaption`: 치환 전 원형·임시문자 중복, 치환 중 임시문자 위치, 치환 후 문단 검사를 Debug에만 남깁니다.
- `AuriMinimalBoxPrototype.ReplaceCloneContent/ReplaceSource`: 같은 치환 단계와 출처 없는 박스의 캡션 분리 직후 XML 검사를 Debug에만 남깁니다.
- `FigureSourcePrototype.Insert`: 출처 슬롯의 임시문자 상태와 치환 직후 문단 검사를 Debug에만 남깁니다.

임시문자 생성·삽입·검색·삭제, 실제 텍스트와 개체 삽입, 하이퍼링크 제거 등 COM 생성 동작은 그대로입니다. 원형 복제 탐색에 필요한 전후 XML 조회, 삽입 구조 검사, 완성된 본문·서식·그림·박스·목록 검사, 네이티브 import 검증, 최종 저장·재열기·참조 검증은 두 구성에 모두 남습니다. 중간 저장·재열기를 제거하는 변경은 이번에 하지 않았습니다.

Debug 소스는 기존 동작을 그대로 포함합니다. 이번 비교에 사용한 Debug EXE는 변경 전 빌드 사본입니다. `target/debug`의 실행 파일도 유지하고, 수정 빌드는 `target/release`에 배치했습니다.

## 결과와 시간

| 실행 | CLI 전체(초) | 백엔드(초) | 전체 XML export 횟수 | 전체 XML export(초) | 공통 계측 선택 블록 export 횟수 |
| --- | --- | --- | --- | --- | --- |
| baseline-debug | 511.989 | 508.123 | 386 | 226.754 | 473 |
| baseline-release | 511.752 | 507.989 | 386 | 231.562 | 473 |
| optimized-release | 383.004 | 379.089 | 177 | 106.151 | 473 |

동일 구성의 수정 전 Release와 비교해 **128.748초(25.16%) 단축**되었습니다. 전체 XML export는 386 → 177회로 209회 감소했습니다. 누적 논리적 UTF-16 문자열 크기는 3.853 → 1.761 GiB입니다.

각 구성은 성공 실행 1회이며 캐시·시스템 부하의 변동을 따로 추정하지 않았습니다. 모두 별도 메서드 계측 사본 없이 일반 빌드에 기존 `MD2HWP_PROFILE=1`을 적용했습니다. 2026-10-08의 별도 상세 계측 실행 시간과 직접 성능 비율을 계산하지 않습니다. PDF export·렌더링·비교 시간은 변환 시간에서 제외했습니다. 선택 블록 횟수는 공통 RenderProfile 계측 범위이며, 상호참조의 직접 선택 export 일부를 포함하지 않습니다.

## 줄어든 조회 위치

| 위치 | 변경 전 횟수 | 변경 후 횟수 | 변경 전(초) | 변경 후(초) |
| --- | --- | --- | --- | --- |
| hwpml.export/AuriMinimalCaptionPrototype.ReadDocument | 148 | 61 | 90.371 | 35.397 |
| hwpml.export/TemplateSource.Insert | 108 | 54 | 64.375 | 31.831 |
| hwpml.export/AuriMinimalBoxPrototype.ReadDocument | 99 | 31 | 57.396 | 18.099 |
| hwpml.block.export/CurrentParagraphStyle.ReadBlock | 473 | 473 | 62.579 | 61.491 |

## 단계별 시간

| 단계 | 기존 Debug(초) | 기존 Release(초) | 수정 Release(초) |
| --- | --- | --- | --- |
| phase.insert | 331.797 | 338.607 | 200.026 |
| phase.import-native-references | 76.888 | 73.787 | 79.244 |
| phase.attach-native | 30.746 | 28.178 | 28.431 |
| phase.verify-native-reopened | 26.907 | 26.947 | 27.851 |
| phase.save-flat-reopen | 14.403 | 13.842 | 14.262 |
| phase.validate-flat | 8.328 | 8.283 | 8.993 |
| phase.verify-flat-reopened | 8.897 | 8.114 | 9.004 |
| phase.prepare | 2.457 | 2.214 | 2.499 |
| phase.save-native-reopen | 1.119 | 1.024 | 1.036 |

## 검증 및 간헐적 오류

백엔드 계약 테스트는 Debug·Release 모두 통과했습니다. Rust workspace 테스트, Clippy, Release 빌드도 통과했습니다. 세 번의 성공 변환 모두 입력 템플릿 해시 보호와 최종 저장·재열기 검증을 완료했습니다. Markdown·그림·템플릿은 동일한 사본으로 고정했습니다. IR·템플릿 계약·날짜·번호 규칙은 변경하지 않았습니다.

최초 변경 전 Debug 시도에서 기존에 보고됐던 초기 import 높이 검증 오류 `SIZE/@Height: 4160 → 2500`이 한 번 발생했습니다. 소스를 바꾸거나 이 검사를 무시하지 않고 실패 로그를 보존한 뒤 재실행했으며, 이후 성공했습니다. 실패 시도는 위 성공 실행 시간에서 제외했으며 `baseline-debug/attempt1/`에 남아 있습니다. 이 간헐적 오류를 해결했다고 주장하지 않습니다.

## 자료 및 되돌리기

집계·페이지별 해시·환경·비교 결과는 [JSON](2026-10-09-release-diagnostics.json)에 함께 보관합니다. 호출 위치·작업별 기존 프로파일, 완성 HWP/XML/PDF, 전체 페이지 PNG와 비교 스크립트는 `artifacts/debug-release-equivalence/`에 있으며 Git에서 제외됩니다.

- `baseline-debug/`: 변경 전 Debug EXE·결과·107쪽 PNG.
- `baseline-release/`: 변경 전 Release EXE·결과·107쪽 PNG.
- `optimized-release/`: 수정 Release EXE·결과·107쪽 PNG.
- `baseline-structure.json`, `baseline-pages.json`: 변경 전에 완료한 비교.
- `optimized-structure.json`, `optimized-pages.json`: 기존 Debug와 수정 Release 비교.

코드 변경은 단독 커밋으로 분리했습니다. 동작을 되돌리는 명령은 다음과 같습니다.

```powershell
git revert fed99cd
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Release
```

다음 단축 후보는 여전히 원형 복제 위치 조회와 현재 문단 스타일 조회입니다. 최종 검증은 계속 유지하며 이 결과를 다음 비교 기준으로 사용합니다.
