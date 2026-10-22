# rhwp와 한컴 백엔드 통합 비교

검증일: 2026-10-22. HWP 전용 실험 백엔드 `md2hwp-rhwp.exe`와 기존 한컴
백엔드의 생성 결과를 같은 한글 렌더러에서 비교했다. 기본 백엔드는 아직
전환하지 않았다. IR 0.4와 기존 원고·템플릿 문법도 그대로 사용했다.

이 기록은 아래 입력과 템플릿으로 실제 생성·저장·재열기한 결과의 진단이다.
각 백엔드의 최종 strict 구조 검증 통과와 페이지 픽셀 비교를 구분한다.
strict 통과는 각 백엔드가 독립적으로 구성한 예상 구조를 저장·재열기 결과와
대조했다는 뜻이며, 두 백엔드의 XML이나 저장 바이트가 같다는 뜻은 아니다.
1·2절은 당시 코드 변경 없이 검증한 결과이며, 3절에는 이후 표 머리행 고립을
보정한 구현과 그 결과를 기록했다. 특정 입력의 일치 결과를 모든 원고의 페이지 나눔·넘침 안전성이나
HWPX 지원으로 확대하지 않는다.

생성 HWP, 내보낸 XML, 페이지 PNG, 해시와 픽셀 분석 JSON, 임시 분석 스크립트는
로컬 `artifacts/rhwp-integration/`에 보관했다. `artifacts/`는 `.gitignore`의
제외 대상이며 원시 증거를 저장소에 추적하지 않는다. 아래 수치는 그 증거를
요약한 것이다.

## 1. 전체 기능 원고

`examples/all-features/all-features.ir.json`과 추적 기본 템플릿
`templates/template.hwp`로 두 백엔드의 HWP를 생성했다. 한컴·rhwp 모두
최종 strict 구조 검증을 통과했다. 결과를 한글에서 재열어 300dpi PNG로
렌더링했으며, 두 결과 모두 13쪽이다.

| 페이지 | 픽셀 비교 결과 |
| --- | --- |
| 1–9, 12–13쪽 | 11쪽 모두 차이 0픽셀 |
| 10쪽 | JPG 사진 영역 안에서만 1,813,534픽셀 차이 |
| 11쪽 | Progressive JPEG 사진 영역 안에서만 1,813,534픽셀 차이 |

사진 밖의 차이는 10·11쪽 각각 0픽셀이다. 사진 경계는 두 페이지의 각 이미지를
독립적으로 읽어 검출했으며, 두 백엔드에서 같은 좌표와 크기를 얻었다.
좌표는 페이지 왼쪽 위를 원점으로 하는 0 기반 픽셀이다.

| 페이지 | 사진 경계 `(x, y, 폭, 높이)` | 사진 밖 차이 |
| --- | --- | ---: |
| 10쪽 | `(449, 1041, 1677, 1118)` | 0 |
| 11쪽 | `(472, 331, 1677, 1118)` | 0 |

비백색 픽셀이 1,000개보다 많은 행의 가장 긴 구간을 찾고, 그 구간에서 비백색
점유율이 90%보다 높은 열로 사진 범위를 정했다. 그 범위는 차이 경계 상자와
같았다. native 그림 크기 `40252×26835 HWPUNIT`의 300dpi 환산값
`1677.17×1118.125`와도 맞으며 페이지 축소본을 직접 확인했다.
페이지 RGB 채널의 최대 차이는 16이었다.

출력에서 내보낸 HWPML의 BINDATA를 Base64 및 필요한 Deflate 해제 후 원본
그림과 대조했다. SHA-256은 디코딩 전 리소스 바이트를 비교했고, 픽셀은
System.Drawing으로 디코딩해 별도로 비교했다.

| 원본 그림 | 한컴 결과 XML 리소스 | rhwp 결과 XML 리소스 |
| --- | --- | --- |
| `image.png` | 바이트는 달라졌으나 디코딩 픽셀 차이 0 | 원본 바이트·SHA-256 일치, 픽셀 차이 0 |
| `image.jpg` | 재인코딩된 JPG, 원본 대비 144,317픽셀 차이 | 원본 바이트·SHA-256 일치, 픽셀 차이 0 |
| `image.jpeg` | JPG와 동일한 리소스로 병합, 원본 대비 144,317픽셀 차이 | 원본 바이트·SHA-256 일치, 픽셀 차이 0 |
| `EmfOnly.emf` | 원본 바이트·SHA-256 일치 | 원본 바이트·SHA-256 일치 |
| `EmfPlusDual.emf` | 원본 바이트·SHA-256 일치 | 원본 바이트·SHA-256 일치 |
| `EmfPlusOnly.emf` | 원본 바이트·SHA-256 일치 | 원본 바이트·SHA-256 일치 |

한컴 결과의 JPG/JPEG 사진은 같은 `BinItem 5`를 참조한다. 해당 리소스의
SHA-256은 `92546EC4F0A8D2264529C40767EB228FCCE411C7ED3B2614AAF1EE82D194286B`다.
원본 JPG와 JPEG는 서로 바이트가 다르지만 디코딩 픽셀은 같다. 한컴 결과의
리소스는 각 원본의 `480×320 = 153,600`픽셀 중 144,317픽셀이 다르고,
RGB 채널 최대 차이는 21, 전체 RGB 채널 평균 절대 차이는 2.148129다.
rhwp는 각각 다른 리소스를 사용하며 두 원본의 바이트를 유지한다.

따라서 이번 13쪽 렌더링의 모든 차이는 두 JPEG 사진 영역 안에만 있으며,
결과 XML에서 확인한 JPEG 재인코딩·중복 병합과 일치한다. 본문·캡션·EMF 등
사진 밖에서 렌더링 차이를 나타내는 픽셀은 없었다. 이 비교는 저장 후 내보낸
XML을 읽었으므로 한컴 처리의 어느 단계에서 리소스가 바뀌었는지까지 특정하지
않는다. 원본 원고와 그림 파일은 수정하지 않았다.

원시 증거: `hancom-all.hwp`, `rhwp-all.hwp`,
`hancom-all-pages/`, `rhwp-all-pages/`, `all-pixels.json`,
`image-comparison.json`, `image-comparison.md`, `analyze-images.ps1`.

## 2. 페이지별 각주 번호 재시작

기존 `tests/fixtures/rhwp-footnotes.md`에서 만든 검증된 IR을 사용했다.
추적 템플릿은 별도 `onpage-template.hwp`로 저장하고, 그 작업본의
`FOOTNOTESHAPE/NOTENUMBERING`에서 `Type`만 `OnPage`로 바꾸었다.
추적 템플릿에 번호 옵션이나 선언을 채택하지 않았다.

이 작업본으로 생성한 한컴·rhwp 결과는 모두 최종 strict 구조 검증을
통과했다. 같은 한글 렌더러의 결과는 각각 3쪽이었고, 모든 페이지에서
픽셀 차이가 0이었다.

| 페이지 | 차이 픽셀 |
| --- | ---: |
| 1쪽 | 0 |
| 2쪽 | 0 |
| 3쪽 | 0 |

2쪽에 각주 번호 1–9가 표시되고 3쪽에서 1로 재시작하는 것을 페이지
이미지로 직접 확인했다. 이 증거는 native `OnPage` 설정이 이번 입력에서
실제 표시 번호에 반영됨을 확인한다. XML의 AUTONUM 숫자만으로 표시 번호를
판정하지 않았다.

`templates/template.hwp`의 작업 전후 SHA-256은
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`로
같았다. 원고와 그림을 포함한 원본 입력은 유지했다.

원시 증거: `onpage-template.ps1`, `onpage-template.hwp`,
`hancom-onpage.hwp`, `rhwp-onpage.hwp`, `hancom-onpage-pages/`,
`rhwp-onpage-pages/`, `onpage-pixels.json`.

## 3. 표 머리행 고립 보정

`tests/fixtures/rhwp-table-pagination.md`는 기존 `layout-issues.md` 원고에
`title`과 `md2hwp-heading1-start: 3` 메타데이터를 추가한 회귀 입력이다.
추적 템플릿의 별도 작업본 `paged-tracked-template.hwp`를 사용했다.
이 작업본은 `slot:table.header`를 포함하는 TABLE의
`SHAPEOBJECT/POSITION`에서 `TreatAsChar`만 `false`로 바꾸었다.
추적 기본 템플릿을 변경하거나 사용자의 배포 템플릿에 이를 채택하지 않았다.

보정 전 결과를 한글에서 열어 native 셀 위치를 확인했을 때 rhwp 결과의
원래 머리행은 2쪽, 첫 본문행은 3쪽에 있었다. 한컴 결과는 두 행이 모두
3쪽이었다. 반복된 머리행을 원래 머리행으로 오인하지 않고 실제 셀의 위치를
대조했다.

| 결과 | 원래 머리행 | 첫 본문행 |
| --- | ---: | ---: |
| 보정 전 rhwp | 2쪽 | 3쪽 |
| 한컴 | 3쪽 | 3쪽 |
| 보정 후 rhwp | 3쪽 | 3쪽 |

`backends/rhwp/src/pagination.rs`에 보정을 추가했다. 대상은 이번 원고에서
생성한 표 중 IR의 본문 `rows`가 비어 있지 않고 native `TreatAsChar=false`인
표다. 정적 템플릿 표, 코드 상자, 글자처럼 취급하는 표와 본문 없는 표는
대상에서 제외한다. 본문 없는 머리행·출처 2행 표를 잘못 포함하지 않도록
전체 native 행 수와 별개로 IR의 본문 존재 여부를 전달한다.

표를 원고 순서대로 확인하면서 rhwp 자체 `page-items`의 원래 머리행과 첫
본문행이 처음 시작하는 쪽·단을 비교한다. 행 조각의 `startRow`/`endRow`,
continuation과 셀별 cut을 읽어 반복 머리행 및 내용이 아직 시작하지 않은
빈 분할 영역을 구별한다. 두 행이 다른 쪽·단에서 시작하면 해당 생성 표의
앵커 문단에 쪽 나눔을 넣고 단 나눔 플래그를 지운다. 다시 조판한 뒤 그 표를
재판정하고, 뒤따르는 표도 바뀐 배치를 기준으로 차례로 확인한다.

위치가 없거나 중복·모호하고, 행 조각의 연속성이나 초기 cut을 해석할 수
없으면 실패한다. 이미 쪽 나눔이 있는데도 고립되어 있거나 한 번 보정한 뒤에도
두 행이 함께 시작하지 않으면 실패하며 반복해서 빈 쪽을 추가하지 않는다.
저장·재열기한 문서에도 같은 위치 검증을 적용한다. 기존의 독립 예상 구조
대조, 내장 리소스 원본 바이트 검증, 입력 불변 확인은 유지하며, 이 검증을
모두 마치기 전에는 임시 출력을 기존 출력에 게시하지 않는다.

새 회귀 입력의 한컴 결과 `hancom-pagination.hwp`와 보정 후 rhwp 결과
`rhwp-pagination-after.hwp`는 모두 strict 구조 검증을 통과했다. 한글에서
렌더링한 결과는 각각 3쪽이며 `pagination-pixels.json`의 모든 페이지 차이는
0픽셀이다. 보정 후 native 위치 기록에서도 머리행과 첫 본문행이 모두
3쪽으로 확인되었다. 3쪽 페이지 이미지를 직접 열어 표 마지막 행과 출처까지
표시되는 것을 확인했다.

| 페이지 | 보정 후 두 백엔드의 차이 픽셀 |
| --- | ---: |
| 1쪽 | 0 |
| 2쪽 | 0 |
| 3쪽 | 0 |

이 보정은 rhwp 자체 조판의 표 머리행 고립을 대상으로 한다. 자체 조판의
전체 배치가 한글 native 배치와 항상 일치한다는 보장은 아니며, 이 입력의
픽셀 일치는 별도의 실제 한글 렌더링 증거다. `TreatAsChar=true`인 긴 표·상자의
클리핑은 해결하지 않았다. 별도 기본 템플릿의 기존 긴 표 시험도 새 코드로 다시
생성했다. 56번 마지막 행과 두 출처 문단을 포함하는 4쪽 모두 기존 한컴 기준과
픽셀이 같았다(`long-tables-pixels.json`). 이 결과는 표 전체를 한 쪽에 묶는
보정이 아님을 확인하는 회귀 증거다. 최종 Rust workspace 테스트 255개(rhwp 140개
포함), Clippy, rustfmt와 고정 SDK의 C# 계약 테스트가 통과했다. Debug·Release를
모두 `tools/development/build.ps1 -Rhwp`로 빌드했고 배포 템플릿은 원본과 같은
해시를 유지했다. 빌드 중의 incremental hard-link 경고는 파일 복사로 대체됐으며
컴파일이나 검증 실패가 아니었다.

첫 두 단계의 검증은 `f7fb83e`, 표 보정과 회귀 원고는 `1c8501e`에 기록했다.

원시 증거: `paged-tracked-template.ps1`, `paged-tracked-template.hwp`,
`rhwp-layout-positions.json`, `hancom-layout-positions.json`,
`rhwp-layout-tracked-page-items.json`, `rhwp-layout-tracked-table-fragments.json`,
`pagination.ir.json`, `hancom-pagination.hwp`, `rhwp-pagination-after.hwp`,
`hancom-pagination-pages/`, `rhwp-pagination-after-pages/`,
`rhwp-pagination-after-positions.json`, `pagination-pixels.json`.
