# 성능 개선 이력과 #7 해결 근거

[#7: release build 속도 향상](https://github.com/esnahn/md2hwp/issues/7)의 구현·검증을 마쳤습니다.
속도 개선은 Debug 기본 XML 경로에서 진행하고, Debug 레거시 COM 경로를 비교 기준으로 사용합니다.
Release에는 레거시 옵션·안내와 상세 계측 코드가 없으며 전체 생성 시간만 측정합니다.
최종 저장·재열기와 구조·서식·참조 검증은 유지합니다.

## 효과와 검증

같은 107쪽 원고의 세 방식 비교입니다. 단일 실행의 전체 시간이며,
Debug와 Release 사이에는 추가 진단 비용 차이가 있습니다.

| 방식 | 전체 시간 | 레거시 대비 단축 |
| --- | ---: | ---: |
| Debug 레거시 COM | 501.159초 | 기준 |
| Debug 기본 XML | 168.714초 | 66.3% |
| Release XML | 112.511초 | 77.5% |

생성 경로 자체의 효과는 Debug끼리 비교한 66.3% 단축으로 평가합니다.
별도의 동일 Release 구성 비교에서도 XML 직접 구성은 209.830 → 121.349초로 42.2% 단축됐습니다.
계측 구분 변경 자체의 추가 속도 개선은 확인되지 않았습니다.

17쪽·107쪽 세 쌍의 구조·서식·참조·내장 이미지와 144dpi 전체 페이지 픽셀이 일치했습니다.
최종 계측 분리 후 새 Debug·Release 출력도 기존 레거시 기준의 17쪽 결과와 일치했습니다.
Rust 전체 테스트·Clippy, Release CLI 테스트, Debug·Release 백엔드 계약 검사와 공식 빌드를 통과했습니다.

- [XML 직접 구성](2026-10-09-direct-xml.md)
- [세 생성 방식 비교](2026-10-09-generation-modes.md)
- [현재 Debug 계측·Release 시간 측정](2026-10-09-debug-profiling.md)
- [실행 방법](../development.md#생성-방식-비교)

## 커밋 정리

성능 작업과 해결 요약을 네 개의 변경 단위로 묶었습니다. 개수를 먼저 정하지 않고,
정확성 확보 → 기존 생성 경로의 조회 비용 축소 → 생성 방식 전환 → 개발용 비교·계측 구분이라는
실제 구현 경계를 기준으로 나눴습니다. 각 커밋에 해당 구현·계약 검사·측정·출력 검증을 함께 담습니다.
진단 제외·좌표 추적·스냅샷과 캐시는 기존 COM 경로의 반복 XML 처리 비용을 줄이는 하나의 변경으로 묶고,
XML 구성기와 실제 생성 경로 연결도 하나로 합쳤습니다.

| 순서 | 변경 단위 | 측정 당시 커밋 | 현재 커밋 |
| --- | --- | --- | --- |
| 1 | 각주 오류 수정·정상 기준 측정 | `dcc8035`, `d9d595f`, `804a5bc` | [7992340](https://github.com/esnahn/md2hwp/commit/7992340843de481ed930f1b3c0be8e10a118a50a) |
| 2 | 반복 XML 조회·비교 비용 축소 | `fed99cd`, `e03700a`, `c06a100`, `009b9a5`, `7a2f6a6`, `4e4a24f`, `db96687`, `1b428c6`, `2450311`, `7fdb4c7`, `55318d6` | [91696d0](https://github.com/esnahn/md2hwp/commit/91696d04773b1d7a002d5e93d85f3976385c1839) |
| 3 | IR·템플릿 XML 직접 구성과 생성 연결 | `52c05a4`, `5d97050`, `96cf50a` | [e32f0ce](https://github.com/esnahn/md2hwp/commit/e32f0ce568510ed9ffe913972f8ee7af5750ea68) |
| 4 | Debug 비교 모드·Release 계측 구분·해결 요약 | `adbd437`, `a3ceed2`, `faa8dcf`, `d2b4751` | [최종 커밋](https://github.com/esnahn/md2hwp/commits/codex/v0.4.0): `refactor(backend): separate Debug comparison modes from Release timing` |

마지막 변경은 해결 요약(`81eab41`)도 포함하며 `Closes #7`을 기록합니다.
현재 네 커밋은 `git log --oneline c7cfb17..HEAD`로 확인할 수 있습니다.
게시된 개발 브랜치의 성능 구간만 다시 정리하며 main과 버전 태그는 변경하지 않습니다.
최종 제품 코드·IR·템플릿·테스트·측정 자료는 정리 전과 정확히 같고, 이 대응 안내만 갱신합니다.

## 측정 기록과 되돌리기

기존 보고서·JSON의 커밋 해시는 정확한 측정 당시 상태를 뜻하므로 그대로 보존합니다.
위 대응표로 현재 이력의 역할을 찾으십시오. 보고서의 과거 `git revert` 명령은
측정 당시 이력 기준이며, 현재 이력에서 되돌릴 때는 위 커밋과 이후 의존 변경을 역순으로 검토해야 합니다.
여러 개선이 합쳐진 커밋은 한 번에 되돌아갑니다. 개별 실험의 정확한 상태는 원본 백업을 사용하십시오.
느려진 중간 캐시 시도의 코드는 최종 구현과 다르므로 해당 측정 해시를 최종 해시로 바꾸지 않았습니다.

원본 측정 이력은 로컬 `artifacts/performance-history-2026-10-09/before-organization.bundle`에,
17개로 정리했던 이력과 해결 요약은 같은 폴더의 `before-consolidation.bundle`에 보존합니다.
전체 해시 대응과 제품 변경 없음의 증거는 `mapping.json` 및 `consolidated-mapping.json`에 있습니다.
이 원시 백업은 ignored입니다. 커밋 메시지·현재 역할 대응·측정 근거는 추적 문서에 보존합니다.
