# rhwp 번호 상호참조 구현 — 2026-10-22

구현과 회귀 테스트는 `3f6e0e5`이다.

HWP 전용 rhwp 백엔드에 제목·그림·표 번호 참조를 구현했다. IR 0.4와 기존
`[표시문](#ID)` 입력, 템플릿 선언은 유지한다. 기존 한글 백엔드가 기본값이며,
rhwp 실행 파일은 계속 별도 `-Rhwp` 빌드 대상으로 릴리스 ZIP에 포함하지 않는다.

## 제목 참조

`begin:ref.heading1.number/end:ref.heading1.number`부터 heading6까지 여섯 개의
한 문단 원형이 모두 필요하다. 원형에는 자기 수준의 `num:headingN`이 반드시 있어야
하며 자기·상위 수준 번호 태그만 사용할 수 있다. 이전 `ref.heading.number` 선언,
제어문자·쪽 나눔·여러 문단 원형과 하위 수준 태그는 거부한다.

제목 번호는 원고의 제목 순서로 계산한다. ID 없는 제목도 세며,
`md2hwp-heading1-start`, 상위 제목에 의한 하위 카운터 초기화와 생략된 부모의
0을 기존 계약대로 반영한다. 뒤쪽 제목 참조도 각주 분리와 폭 측정 전에 해석한다.
원형의 문자 서식 대신 원고 참조의 강조·기울임 등 서식을 유지한다.

결과는 고정 텍스트다. native 개요 필드나 하이퍼링크를 추가하지 않으며, HWP에서
제목을 직접 수정한 뒤 자동 갱신되지 않는다. 원고를 다시 생성해야 번호가 바뀐다.

## 그림·표 참조

`ref.figure.number`와 `ref.table.number`의 한 문단 원형을 사용하고 번호 슬롯은
native Crossref 필드로 생성한다. 원형의 리터럴·`num:heading1`은 대상 개체가
속한 장을 따르며, 참조가 놓인 장으로 바꾸지 않는다. 표 참조 대상에는 native
Table AUTONUM이 있는 캡션이 필요하다.

대상 개체가 모두 합성된 뒤 실제 native 개체 번호·재시작과 식별자를 바인딩하므로
앞쪽 참조도 처리한다. 필드 명령은 모델의 식별자 별칭이 아닌 대상 개체의 정확한
raw common-object ID를 사용한다.

| 대상 | native Crossref 명령 |
| --- | --- |
| 그림 | `?#ID;1;1;0;0` |
| 표 | `?#ID;0;1;0;0` |

필드 `properties=2`, 번호 표시 1, 하이퍼링크 0을 사용한다. 참조별 식별자와
로컬 begin/end 필드 짝, 표시 캐시, 원고 문자 서식과 주변 native 제어문자를 유지한다.
그림/표 번호를 일반 텍스트로 축소하지 않는다.

## 위치·폭·검증 계약

본문·제목·중첩 목록·여러 각주 본문 문단·표 셀·그림/표 캡션과 코드/그림/표의
모든 출처 문단에서 제목·그림·표 번호를 처리한다. 그림 alt는 설명 메타데이터이므로
참조 대상을 검증하지만 렌더링 문단에서 그 필드가 나와야 한다고 요구하지 않는다.
각주 자체가 허용되는 위치와 참조 필드가 허용되는 위치는 각각의 계약을 따른다.
예를 들어 캡션의 상호참조가 가능하더라도 캡션의 각주를 허용하지 않는다.

원고·메타데이터·코드 본문의 리터럴은 재해석하지 않는다. 내부 표식은 충돌을 피한
어댑터 전용 문자열이며 실제 문자 폭 계산이나 최종 출력에 남지 않아야 한다.

표는 합성 중 임시 열 폭을 사용하고, 뒤쪽 개체까지 참조 관계가 확정된 뒤
Windows TTF/OTF 글꼴 측정으로 최종 폭을 다시 계산한다. 이때 그림·표 참조에는
기존 한글 백엔드와 같은 보수적 예약 폭을 사용한다. 원형의 각 조각은 자체 문자
서식으로 측정하고 원고의 강조·기울임을 추가로 적용한다. 장 번호와 개체 번호를
각각 `999`로 놓으며, 확장된 참조 전체를 나눌 수 없는 한 단위로 취급한다.
native 필드의 실제 짧은 표시 캐시 번호나 내부 ASCII 표식의 폭을 대신 사용하지 않는다.

생성 표의 열·셀 폭과
native 텍스트 영역 폭만 갱신하며 식별자·내용·서식·캡션·저장 높이는 유지한다.
병합 출처 셀은 전체 열 폭의 합을 사용한다. 달라진 셀 문단의 줄 캐시만 비우고
빈 셀의 파서 표현은 최종 폭으로 다시 구성한다. 잘못된 그리드나 native 폭
오버플로는 기존 결과를 바꾸기 전에 오류로 처리한다.

최종 임시 저장·디스크 재읽기·재열기는 독립적으로 합성한 기대 모델과 비교한다.
native 필드 대상·명령·캐시·서식·주변 구조를 유지해야 한다. 한글 HWPML의 필드
시작 요소는 `FIELDBEGIN`이며, 확인할 때 다른 이름의 요소로 세지 않는다.
재열기 결과를 기대값으로 삼거나 검사를 완화하지 않는다. 저장 번호와 고정 파서의 파생 캐시를 구분하는
[각주 검증 계약](rhwp-footnotes-2026-10-22.md)도 유지한다. OnPage 각주의 실제
페이지별 번호 재시작은 아직 한글 페이지로 시험하지 않았다.

## 저장·표시 비교

[검증 원고](../tests/fixtures/rhwp-references.md)는 Unicode ID, 뒤쪽 그림·제목 참조,
다른 장의 개체, 강조·기울임과 emoji 옆의 연속 참조, 목록·각주·표 셀·캡션·여러
출처 문단, 스스로를 가리키는 개체 참조와 코드 리터럴을 포함한다.

| 검사 | 결과 |
| --- | --- |
| 임시 저장·재읽기·독립 기대 모델 비교 | 통과 |
| 한글 HWPML native FIELDBEGIN 개수 | 양쪽 23개 |
| 실제 대상 개체·캡션 카운터·그림/표 종류 | 모두 일치 |
| 표시 캐시와 대상 번호 | 모두 일치 |
| 고유 InstId·로컬 FieldId begin/end 짝 | 통과 |
| 한글 페이지 비교 | 양쪽 5쪽, 전부 차이 0픽셀 |
| Rust workspace / rhwp 백엔드 | 239개 / 125개 통과 |
| Clippy·C# 계약 테스트·fmt 검사 | 통과 |

두 출력의 2·3·5쪽을 직접 확인했다. 픽셀 비교와 별도로 HWPML의 필드 대상·
캐시·식별자를 검사했다. 이 결과를 임의 원고·템플릿의 배치 안전성으로 확대하지
않으며, 그림·표의 기존 페이지 나눔 한계도 그대로 적용한다.

초기 구현에서는 인접한 참조를 확장할 때 앞 필드의 끝 경계가 뒤 참조까지 늘어나는
문제가 있었다. native 필드 경계를 보존하는 치환으로 수정했다. 이어진 페이지
비교에서는 3쪽에만 7,540픽셀 차이가 남았다. 실제 짧은 참조 번호로 열 폭을
측정한 것이 원인이었다. 원형 조각 서식·원고 강조와 장/개체 번호 999를 사용하는
기존 한글 백엔드의 예약 폭으로 맞춘 뒤 다섯 페이지가 모두 일치했다.
구조 검사나 기대 모델 비교를 완화해 해결하지 않았다.

`all-features` 원고의 기존 UseInstWidth=true 여러 문단 목록 항목은 계속 명시적으로
거부한다. 상호참조 지원으로 이 목록 표식 측정 제한을 해제하지 않았다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
target/debug/md2hwp.exe md2ir tests/fixtures/rhwp-references.md --output artifacts/rhwp-references/source.ir.json
target/debug/md2hwp-rhwp.exe artifacts/rhwp-references/source.ir.json artifacts/rhwp-references/rhwp-final.hwp --template templates/template.hwp
target/debug/md2hwp-backend.exe ir2hwp --ir artifacts/rhwp-references/source.ir.json --output artifacts/rhwp-references/hancom.hwp --template templates/template.hwp
```

rhwp 생성에는 COM·.NET을 사용하지 않는다. Markdown→IR에는 Pandoc이 필요하며,
한글 기준 출력과 독립 페이지 비교에는 기존 한글·.NET·보안 모듈 준비 조건을 따른다.
HWPX 템플릿·출력은 기존 한글 백엔드를 사용한다.

## 보관 근거

원본 추적 템플릿 SHA-256은 시험 전후 동일하다.
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`

rhwp 최종 출력 (`rhwp-final.hwp`):
`D084752C7337E13D057B3E1B83ABD872C0D93D46A540FA6ABEC0B47E0521B022`

한글 기준 출력 (`hancom.hwp`):
`5D13422D9C5A4ADA30D4B6BD5FC41DE4891A594C0A7FC84AC221E94F9BFBA1CA`

HWP·HWPML·페이지 PNG는 ignored `artifacts/rhwp-references/`에 보관한다.
픽셀 결과는 `final-pixel-comparison.json`, 독립 필드 검사는
`verify-fields.py`와 `field-verification.json`에 있다. 원본 템플릿과 과거 기능별
시험 기록은 변경하지 않았다. Git에는 검증 원고·회귀 테스트와 이 기록을 남긴다.
