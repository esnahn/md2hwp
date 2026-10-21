# rhwp 각주 생성 시험 — 2026-10-22

각주 생성·저장 번호 검증 구현은 `9497a98`이다.

IR 0.4와 고정 rhwp revision을 유지하며 기존 각주 계약을 HWP 전용 rhwp 백엔드에
구현했다. 생성에는 한글 COM이나 .NET을 사용하지 않는다. 한글 백엔드는 기본값과
표시 비교 기준으로 유지하며, 릴리스 ZIP 구성은 변경하지 않는다.

## 구현 범위

- 본문·제목·중첩 목록·표 머리행과 본문 셀에서 native FOOTNOTE를 생성한다.
  강조·기울임·링크 표시 문구·Unicode·강제 줄바꿈과 여러 각주 본문 문단을 유지한다.
- `{{md2hwp:footnote}}`는 정의 구간의 평문 루트 샘플 또는 평문 루트 앵커에
  연결된 native 각주 샘플에서 읽는다. 첫 문단의 전체 서식과 슬롯 첫 글자 서식,
  native 번호 서식·탭 데이터·공백·각주 LIST_HEADER 옵션을 보존한다.
  평문 샘플에서 새로 만드는 각주 헤더는 구역의 번호 형식·장식 문자를 사용하며,
  native 샘플의 헤더는 알려지지 않은 상위 플래그까지 그대로 유지한다.
- 선택적 `{{md2hwp:footnote.next}}`는 이어지는 모든 본문 문단에 적용한다.
  리터럴·탭·AUTONUM 접미/접두 요소도 문단마다 복제한다. 샘플이 없으면 첫 문단의
  서식을 사용하되 첫 문단에만 속하는 번호·공백·탭은 반복하지 않는다.
- 중복·혼합·잘못된 위치의 샘플, 추가 본문 샘플 문단, 지원하지 않는 제어문자와
  쪽 나눔은 거부한다. 원본 템플릿을 바꾸지 않고 작업 복사본의 정의 구간만 제거한다.
- 제목 슬롯·each.child 복제와 동일한 원고 참조의 반복은 각각 독립 native 각주와
  식별자를 만든다. 원고·메타데이터의 리터럴을 템플릿 태그로 다시 해석하지 않는다.
- 내부 참조 표식은 실제 본문에서 native 8단위 간격으로 치환한다. 가짜 공백이나
  개체 문자를 남기지 않고 문자 서식·탭·다른 개체·필드와 UTF-16 위치를 유지한다.
  표 폭 계산에는 임시 ASCII 표식의 폭 대신 실제 각주 참조를 전달한다.

각주 본문은 문단만 허용한다. 중첩 각주, 그림 대체문구·그림/표 캡션·개체 출처의
각주와 상호참조는 기존 IR 또는 rhwp 지원 범위에 따라 명시적으로 거부한다.
제목 슬롯이 실제로 복사되는 컨테이너도 검사하므로 native 캡션·머리말·꼬리말·
바탕쪽·주석·다른 각주에 들어가는 참조는 오류다. 평범한 표 셀에서 허용된다는
이유로 표 출처 셀의 각주까지 허용하지 않는다.

## 저장 번호와 파서 캐시의 구분

고정 rhwp 파서는 읽기 후 AUTONUM의 `number`와 `assigned_number`를 자체 순회
카운터로 덮어쓴다. 이어지는 각주 문단의 AUTONUM도 카운터에 포함하며 native
OnPage/OnSection 배치 규칙을 구현하지 않는다. 이 파생값은 저장된 번호의 증거가 아니다.

템플릿 샘플을 복제하기 전에 원본 native 레코드에서 저장 AUTONUM 값을 복원한다.
컨트롤 종류·순서·레벨과 숫자 외 형식 바이트가 맞아야 하며, 임의 위치나 표식 문구로
값을 추정하지 않는다. 최종 저장·재읽기에서는 독립적으로 합성한 기대 모델과
실제 native AUTONUM 전체 payload, 생성 FOOTNOTE 헤더와 고유 식별자를 비교한다.
이어지는 문단의 AUTONUM 값도 이 검사에 포함한다.

이 검사를 통과한 뒤에만 검증용 기대 모델 복사본에서 고정 파서와 같은 순회로
번호 캐시를 계산하고 재열기 모델과 비교한다. 실제 저장 모델이나 재열기 바이트를
기대값에 맞게 고치지 않는다. 원본 SECDEF의 번호·구분선·배치 옵션은 유지하며,
저장 바이트와 구조 검사만으로 한글의 실제 표시 번호나 페이지 배치를 보장하지 않는다.

생성 각주의 첫 AUTONUM은 문서·구역 시작 설정과 native NEWNUM 재시작에 따라
계산한다. 이어지는 문단의 AUTONUM 접두/접미 값과 기존 정적 각주는 생성 번호를
배정하기 위해 고치지 않는다. 번호 회귀 테스트는 문서 시작 7, 구역 시작 20,
NEWNUM 12, SECDEF 시작 30, 이어지는 문단의 저장 번호 77 보존과 정적 영역·
머리말 제외를 포함한다. 이 단위 테스트와 실제 한글 페이지 비교는 구분한다.

## 확인한 비교 결과

[검증 원고](../tests/fixtures/rhwp-footnotes.md)는 제목·본문·강조·기울임·목록·중첩
목록·표 머리행·표 셀, 같은 원고 참조의 반복, 여러 각주 문단과 리터럴 태그를 포함한다.
추적 템플릿에서는 each.child 제목 복제까지 포함해 11개 native 각주를 생성했다.
최종 비교 원고의 CommonMark 강조 경계는 실제 Strong/Emph 내부에 각주 참조가
들어가도록 확인했다.

| 검사 | 결과 |
| --- | --- |
| Rust workspace / rhwp 백엔드 | 218개 / 104개 통과 |
| Clippy / rustfmt / C# 계약 테스트 | 통과 |
| 번호 설정 회귀 테스트 | 5개 통과 |
| 임시 저장·재읽기·독립 기대 모델 검증 | 통과 |
| 저장 FOOTNOTE 헤더와 AUTONUM payload 검증 | 통과 |
| 최종 원고 / 추적 템플릿 / 한글 페이지 비교 | 양쪽 3쪽, 전부 차이 0픽셀 |
| 문서 각주 시작 7 / 별도 native 샘플 템플릿 / 한글 페이지 비교 | 양쪽 1쪽, 차이 0픽셀 |
| 문서 각주 시작 7 / 첫 본문 AUTONUM의 HWPML 번호 | 양쪽 7–16 |

추적 템플릿과 별도 기본 템플릿의 HWPML `NOTENUMBERING`은 `Type=Continuous`,
`NewNumber=1`이었다. 시작 7의 별도 템플릿은 `DOCSETTING/BEGINNUMBER`의
`Footnote=7`을 사용했다. OnPage의 실제 페이지별 번호 재시작은 이번에 시험하지 않았다.
위 픽셀 결과는 두 템플릿과 이 원고에 한정한다. 임의 템플릿의 각주 번호,
페이지 분할·잘림·겹침이나 모든 번호 설정의 동등성을 입증하지 않는다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
target/debug/md2hwp.exe md2ir tests/fixtures/rhwp-footnotes.md --output artifacts/rhwp-footnotes/source.ir.json
target/debug/md2hwp-rhwp.exe artifacts/rhwp-footnotes/source.ir.json artifacts/rhwp-footnotes/rhwp-final.hwp --template templates/template.hwp
target/debug/md2hwp-backend.exe ir2hwp --ir artifacts/rhwp-footnotes/source.ir.json --output artifacts/rhwp-footnotes/hancom-final.hwp --template templates/template.hwp
```

Pandoc·한글·.NET·보안 모듈 준비는 기존 조건을 따른다. 한글은 비교 문서 생성과
독립 재열기·페이지 출력에 사용하며 rhwp 생성의 런타임 의존은 아니다.

## 보관 근거

추적 템플릿 SHA-256:
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`

최종 원고·추적 템플릿 rhwp 출력 (`rhwp-final.hwp`):
`799B2D6A842659E5F68E61EFCE20C5DB2BAC9754FBD28EF643F65224D81CD6D9`

최종 원고·추적 템플릿 한글 기준 출력 (`hancom-final.hwp`):
`8FFC15FA9C3C57F205FC58592F364C50E0F54A47FFBE68F8EBBF9130870ACA83`

문서 각주 시작 7·별도 native 샘플 rhwp 출력 (`rhwp-start7.hwp`):
`B1384702672A040B1F8AF68C5792F731BA10F94F0CAEFD6942BC588732DDE27D`

문서 각주 시작 7·별도 native 샘플 한글 기준 출력 (`hancom-start7.hwp`):
`D6415F94E0907DFACE817348744867B59432E2FAFB4CBB9192691A406F717158`

HWP·HWPML·페이지 PNG와 픽셀 비교는 ignored `artifacts/rhwp-footnotes/`에 보관한다.
최종 비교는 `final-pixel-comparison.json`, 시작 7 비교는
`start7-pixel-comparison.json`에 기록했다. 초기 native 샘플 비교 자료도 남아 있지만,
위 표와 해시는 CommonMark 강조 경계를 확인한 최종 원고의 결과를 기준으로 한다.
Git에는 원고·회귀 테스트와 이 기록을 남긴다. 원본 추적 템플릿과 과거 시험 기록은
변경하지 않았다.
