# rhwp 목록 생성 시험 — 2026-10-21

IR 0.4와 고정 rhwp 커밋을 유지한 목록 생성 단계이다. 기본 백엔드와 배포 ZIP은
바꾸지 않았다. rhwp 생성에는 한글 COM이나 .NET을 사용하지 않는다.
독립 표시 검증에만 한글 2020 11.0.0.9136을 사용한다.

## 서식과 번호

목록 문단의 본문·문단 서식은 `body`에서 가져온다. `list.bullet`과
`list.ordered`에서는 native 표식 정의만 가져온다. 표식의 글자 서식이 생략되어
있으면 목록 샘플의 글자 서식으로 고정한다. 원래 정의는 수정하지 않는다.

`list.indent-hwp`를 원래와 같이 깊이마다 더한다. 이 값과 문단 여백의 단위는
URC이며, 표식 폭·크기·본문과의 거리는 HWPUNIT이다. 두 단위를 혼용하지 않는다.
깊이는 0부터 `list.max-depth`까지이며, 최대 설정 6이면 총 7수준이다.

번호 목록은 원고의 시작 번호를 사용한다. 하위 목록이나 이어지는 문단 뒤에
상위 목록을 재개할 때에는 해당 항목 번호로 새 native 정의를 만든다.
별도 목록의 번호도 독립적이다. native 한글·괄호·원문자 번호 양식을 유지한다.
새 번호 구간의 시작값은 현재 HWP 저장 형식의 16비트 제한 때문에 65535까지이며,
이를 넘으면 오류를 반환한다.

여러 문단 항목의 두 번째 이후 문단에는 표식을 붙이지 않고 첫 문단의 본문
시작점에 맞춘다. `UseInstWidth=false`에서는 글자 높이를 쓰는 기존 한글 백엔드와
같은 계산식을 적용한다. `UseInstWidth=true`이면 실제 표시된 표식의 글꼴 폭이
필요하므로 현재는 명확한 미지원 오류를 반환한다. 한글 백엔드 사용을 안내하며
자동으로 전환하지 않는다. 이미지 글머리표의 이어지는 문단도 지원하지 않는다.

현재 기본 템플릿의 글머리표는 false, 번호 목록은 true이다. 따라서 기본 템플릿에서
여러 문단 글머리표는 지원하고 여러 문단 번호 항목은 거부한다.
긴 한 문단의 자연 줄바꿈은 이 제한과 별개이며 두 목록 모두 지원한다.

## 재현 원고와 방법

[rhwp-lists.md](../tests/fixtures/rhwp-lists.md)에 다음 사례를 모았다.

- 글머리표 6수준, 하위 목록에서 상위로 복귀.
- 7부터 시작하는 번호 목록, 한글 양식의 중첩 번호와 상위 9번 재개.
- 별도 99·100번 목록.
- 강조·기울임·링크 표시·이모지·자동 줄바꿈.
- 여러 문단 글머리표의 이어지는 문단과 다음 항목, 목록 뒤 일반 본문.

템플릿은 변경하지 않은 `templates/template.hwp`이다. SHA-256은
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`이다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
.\target\debug\md2hwp.exe md2ir tests\fixtures\rhwp-lists.md --output artifacts\rhwp-lists\source.ir.json
.\target\debug\md2hwp-rhwp.exe artifacts\rhwp-lists\source.ir.json artifacts\rhwp-lists\rhwp.hwp --template templates\template.hwp
.\target\debug\md2hwp-backend.exe ir2hwp --ir artifacts\rhwp-lists\source.ir.json --output artifacts\rhwp-lists\hancom.hwp --template templates\template.hwp
```

Pandoc 경로와 .NET·한글 보안 모듈은 기존 실행 조건을 따른다. 두 결과를 한글에서
순차적으로 재열고, 300dpi 손실 없는 페이지 이미지와 HWPML을 비교한다.

## 실제 표시 비교에서 발견한 차이

첫 구현은 문단 모양의 attr1에만 수준을 썼다. rhwp 파서는 이 값으로 재열기하여
모델 검사를 통과했지만, 한글은 같은 레코드의 별도 수준 값도 읽어 중첩 `가·나`를
`1·2`로 표시했다. 차이는 두 번째 페이지의 18,121픽셀이었다.
58바이트 레코드의 수준 값도 함께 갱신하도록 수정하고, 모든 7수준에서 두 값과
나머지 원본 바이트의 보존을 검사하는 회귀 테스트를 추가했다.

이 사례는 rhwp 자체 모델 재열기가 한글의 실제 표시 검증을 대체하지 못함을 보여 준다.
제품의 저장·재열기 구조 검증은 완화하지 않았다.

## 최종 결과

| 검사 | 결과 |
| --- | --- |
| rhwp 임시 저장·디스크 재읽기·자체 재열기 모델 비교 | 통과 |
| 한글 재열기 | 두 결과 모두 3쪽 |
| 한글 300dpi 페이지 비교 | 3쪽 모두 다른 픽셀 0개 |
| 실제 2·3쪽 시각 검토 | 중첩 표식·가/나 번호·7/99 시작·상위 재개·이어지는 문단 정렬 일치 |
| 루트 텍스트 문단 22개의 문단 속성·여백·표식 종류/수준/양식/시작값 비교 | 일치 |
| 원래 스타일·글꼴 정의, CHAR 텍스트, 필드, 내장 데이터 비교 | 일치 |
| 기존 전체 XML 구조 비교 | 차이 있음, 전체 일치로 보고하지 않음 |
| UseInstWidth=true인 여러 문단 번호 항목 | 명확한 오류, 기존 출력 보존 |
| 전체 Rust 테스트 / Clippy / Debug 빌드 | 152개 통과 / 통과 / 통과 |

전체 XML 비교는 [제목 블록 시험](rhwp-headings-2026-10-21.md)에서 확인한 도형
행렬 목록 표현 차이부터 불일치한다. 별도의 문단·목록 비교는 서식 식별자를 제외한
실제 문단 속성과 여백, 선택된 native 표식 양식과 번호 시작값을 검사했다.
표식 글자의 실제 표시는 페이지 픽셀 비교로 함께 확인했다.
이 별도 비교를 제품의 엄격한 검증을 대체하는 규칙으로 추가하지 않았다.

산출물은 ignored `artifacts/rhwp-lists/`에 저장했다.
`rhwp-final-pages/`와 `hancom-final-pages/`가 최종 페이지·XML이며,
`pixel-comparison.json`, `list-structure-comparison.json`,
`structure-comparison-final.json`, `negative-test.json`에 결과를 기록했다.
실패한 첫 표시 비교도 `rhwp-pages/`에 남겼다.
최종 HWP SHA-256:
`7910976705286172741504F0F8D3D920D3456153A989BA24B71F9A3AA3B0A419`.
