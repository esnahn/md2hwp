# rhwp 무편집 저장 시험 — 2026-10-19

## 판정

**HWP→HWP 경로는 후속 생성 시험을 진행할 근거를 확보했다. HWPX를 포함한
완전 교체는 보류한다.** 현재 템플릿을 읽고 저장하는 단계에서 세로쓰기와
일부 스타일·필드 속성이 달라진다. md2hwp 제품 코드와 기본 백엔드는 변경하지 않았다.

이번 시험은 문단·개체를 새로 만들지 않았다. 한글에서의 필드 갱신 동작,
임의 사용자 템플릿, 대규모 생성 성능, rhwp 자체 화면·PDF 렌더링까지
검증한 것은 아니다.

후속 범위 결정: rhwp 백엔드는 **HWP 템플릿·HWP 출력만 지원**한다.
아래 HWPX 결과는 조사 기록으로 보존하며, HWPX 문제 수정은 이번 구현의
선행 조건이 아니다. HWPX는 기존 한글 백엔드에서 지원한다.

## 조건

- md2hwp v0.4.4 (`54caa0d`), IR 0.4.
- rhwp v0.8.7 main [`1a76570e833917d15817415a53c09ad61ab3203f`](https://github.com/edwardkim/rhwp/tree/1a76570e833917d15817415a53c09ad61ab3203f).
- rhwp 라이브러리를 직접 호출하는 시험 EXE. Rust 1.98.0, upstream Cargo.lock을
  `--locked`로 사용. Debug, debuginfo 및 incremental 비활성화. 속도 벤치마크가 아니다.
- upstream 소스는 `.local/dependencies/rhwp-source`에 sparse checkout했다.
  엔진 수정 없이 Cargo.toml에 로컬 시험 example 등록만 추가했다.
- `DocumentCore::from_bytes`로 열고 HWP는
  `export_hwp_with_adapter_snapshot_with_report`, HWPX는
  `export_hwpx_native_with_report`로 저장했다. 다시 rhwp로 파싱하고 내부 모델을 비교했다.
- 한글 2020 11.0.0.9136, 사용자 Session 1, Windows PowerShell 5.1 x64 STA.
  등록된 보안 모듈 확인 후 문서 하나씩 열었다. 경고창 수락이나 UI 조작은 없었다.
- 한글 HWPML 내보내기 및 native 300 DPI 이미지로 독립 비교했다.
  원본은 변경하지 않았고 시험 후 한글 프로세스가 종료된 것을 확인했다.

입력:

| 입력 | SHA-256 |
| --- | --- |
| `templates/template.hwp` | `20bc2ec532692d09cd2a40a67ddbbc24190fd87033efadeb884fa48d31fe6b81` |
| `artifacts/hwpx-support/template.hwpx` | `a317a186f24d3f3195e41f8d55d36569380afed48fe3251c97f995a747ec13a0` |
| `artifacts/hwpx-support/release-hwp-to-hwp.hwp` | `d2e4136f2d57aa559331c7289cce405f3c76664be13b477bcf5675a2fbc3cf1e` |
| `artifacts/hwpx-support/release-hwp-to-hwpx.hwpx` | `5606dfa063500aadbf3bd4c7d527ad1f926b0e0efda6ce129cf68e0e26040564` |

전체 기능 출력은 기존 `examples/all-features` 원고의 13쪽 결과다.
PNG/JPEG/EMF, 표, 코드, 각주, native 캡션·자동번호·상호참조를 포함한다.

## 저장 및 페이지 결과

8개 모두 저장, rhwp 재파싱 및 한글 열기에 성공했다. 한글 쪽수는 모두 유지됐다.
아래 차이 픽셀 수는 각 문서 전체 페이지를 원본과 비교하여 합산한 값이다.

| 입력 | 출력 | 쪽수 | rhwp 모델 차이 수 | 한글 차이 픽셀 |
| --- | --- | ---: | ---: | ---: |
| 템플릿 HWP | HWP | 6 | 0 | **0** |
| 템플릿 HWP | HWPX | 6 | 0 | 22,944 |
| 템플릿 HWPX | HWP | 6 | 1 | 25,248 |
| 템플릿 HWPX | HWPX | 6 | 0 | 22,840 |
| 전체 기능 HWP | HWP | 13 | 0 | **0** |
| 전체 기능 HWP | HWPX | 13 | 22 | 53,288 |
| 전체 기능 HWPX | HWP | 13 | 28 | 114,367 |
| 전체 기능 HWPX | HWPX | 13 | 0 | 52,976 |

rhwp 모델 차이 0도 원본 보존을 증명하지 않는다. 파서에서 이미 빠진 속성은
자체 재파싱 비교에 나타나지 않을 수 있다. 형식 간 모델 차이에는 파라미터 표현
변경처럼 무조건 손실이라고 할 수 없는 차이도 포함된다.

같은 형식 재저장 4건을 다시 저장했을 때 모두 첫 재저장본과 파일 바이트까지
일치했다. HWPX도 이미 발생한 손실이 안정적으로 반복될 수 있으므로, 이를
원본 보존 성공으로 해석하지 않는다.

## 독립 구조·리소스 비교

- **HWP→HWP 두 건:** 기존 body/style 검증, 글꼴 정의, 문단별 문자열,
  native 필드 identity·속성이 일치했다. 6+13쪽 화면도 픽셀 단위 일치다.
- **모든 8건:** 문단별 CHAR 문자열 배열이 일치했다. 전체 기능 문서의
  194개 문단, 18개 표, 6개 그림, 7개 각주, 22개 필드 begin/end,
  20개 AUTONUM, 7개 NEWNUM이 유지됐다.
- **그림:** 압축을 해제한 실제 payload와 문서 내 그림이 참조하는 payload 순서가
  모두 일치했다. 최초 엄격 BINDATA 비교 실패는 `Size`/`Compress`/ID 등
  저장 표현 차이도 포함한 결과다. 이를 이미지 내용 손실로 보고하지 않는다.
- HWPX 저장의 JPEG가 raw-DEFLATE로 포장되는 경우, 해제 후 원본 71,627바이트와
  SHA-256이 일치했다. HWPX ZIP의 BinData 자체도 같은 형식 왕복에서 정확히 일치했다.
- 다만 이미지 형식 메타데이터에 `EMF→OCTET-ST`, `JPG→JPEG` 변경이 있어 별도 기록했다.
  모든 속성이 동일하다는 의미는 아니다.

기존 비교기는 그대로 유지했다. 조사용 추가 분석에서 저장 표현과 실제 데이터
차이를 분류했으며, 제품의 엄격 검증을 완화하지 않았다.

## 보완해야 할 차이

1. **세로쓰기 소실 — 직접 시각 확인**
   HWPX 경유 경로에서 `PARALIST/@TextDirection`이 `2→0`으로 바뀐다.
   원본 여백에 세로로 쓰인 제목이 하단에서 가로로 겹쳐 보인다.
   템플릿 3쪽, 전체 기능 2쪽의 차이 영역을 확대하여 확인했다.
2. **스타일 정의 누락**
   HWPX 경유 경로에서 스타일 18·19·20의 글자 모양에 있던
   `<STRIKEOUT Color="0" Shape="3D" Type="Continuous"/>`가 빠진다.
   원고가 해당 스타일을 사용하지 않아 화면 차이가 작더라도 템플릿 보존 위반이다.
   HWPX→HWP에는 일부 BorderFill/FILLBRUSH 표현 차이도 있다.
3. **글상자 및 페이지 배치**
   템플릿 HWPX→HWPX에서 일부 `DRAWTEXT/@LastWidth`가 `27467→15896`으로 바뀐다.
   전체 기능 HWPX→HWP에서는 4쪽 첫 목록 `관리체계`가 3쪽 끝으로 이동한다.
   3·4쪽을 직접 확인했으며 문단 데이터는 동일하다. 삭제가 아니라 페이지 배치 차이다.
   정확한 배치 차이의 원인은 추가 추적이 필요하다.
4. **상호참조 기능 속성**
   전체 기능 HWP→HWPX에서는 22개 FIELDBEGIN의 `Fixed="true"`가 사라진다.
   HWPX→HWP에서는 여기에 22개 FIELDEND의 `Property="0"→"37"` 변경도 있다.
   대상·Command·FieldId·InstId·캐시 문자열은 유지됐다. 갱신 동작 동등성은 미검증이다.
   rhwp의 content-loss 반환값은 비어 있었지만 HWPX→HWP stderr에는 일부 필드
   파라미터 손실 경고가 있어, 반환 보고서만으로 무손실을 판정하면 안 된다.

## 다음 판단

`md2hwp-rhwp.exe`를 기존 백엔드와 같은 CLI 계약으로 만드는 방향은 유지한다.
후속 사용자 결정에 따라 HWP 전용으로 본문·제목·메타데이터 생성 시험을 진행한다.
HWPX 템플릿·출력은 명시적으로 거부하고 기존 한글 백엔드 선택을 안내한다.
현재 결과를 모든 HWP 템플릿의 호환 보장으로 확대 해석하지 않는다.

원시 증거는 ignored `artifacts/rhwp-roundtrip/`에 보관한다:

- `probe.rs`, `run-matrix.ps1`, `build*.log`: 실제 시험 API/실행·빌드 기록.
- `matrix.json`, `*.report.json`, `second-roundtrip.json`: 경로·해시·자체 비교·2차 저장.
- `*-pages/document.xml`, `page*.png`: 한글에서 얻은 구조와 이미지.
- `*.native.json`, `compare-xml/native-analysis*.json`: 독립 구조·데이터 분석.
- `pixel-comparison.json`, `regions.json`, `*-region*.png`: 76개 페이지 비교와 확대 이미지.
- `*-hwpx-package.json`: HWPX ZIP 부분·리소스 비교.

이 자료와 시험 EXE는 배포 패키지에 포함하지 않는다. 앱 변경이 없으므로
프로젝트 전체 테스트는 재실행하지 않았다. 비교 도구의 원본 자기 비교와
문자 변경 negative control은 각각 성공·실패를 확인했다.
