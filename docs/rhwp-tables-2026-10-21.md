# rhwp 표 생성 시험 — 2026-10-21

글꼴 측정 기반은 `a720452`, 표 생성 구현은 `0c1059e`이다.
IR 0.4와 고정 rhwp revision을 유지했다. 생성에는 한글 COM이나 .NET을 사용하지
않으며, 독립 재열기·페이지 비교에는 한글 2020 11.0.0.9136을 사용했다.
기본 백엔드와 릴리스 ZIP 구성은 변경하지 않았다.

## 구현 범위

- `begin:table/end:table`의 3행·2열 원형에서 머리행·본문행의 두 셀과 병합
  출처 셀을 읽는다. 원형 크기를 무시하고 IR의 행·열로 다시 구성한다.
- 왼쪽 샘플의 서식·가로 테두리와 두 샘플의 안팎 세로 테두리를 사용한다.
  한 열에서는 양쪽 외곽선을 결합한다. 명시적 정렬만 덮어쓰고 기본 정렬은 유지한다.
- 캡션은 실제 Table AUTONUM과 샘플 옵션을 유지하며, 원고에 없으면 생략한다.
  문서·구역 시작 번호와 기존 NEWNUM을 따라 세며 별도의 장별 재시작을 추가하지 않는다.
- 여러 출처 문단은 투명한 병합 셀에 복제한다. 출처가 없으면 마지막 행을 생략한다.
  머리행만 있는 표의 출처 상단선은 머리행 하단선에 맞춘다.
- 빈 셀, 강조·기울임·링크 표시 문구, 원형 앞뒤 빈 문단을 보존한다.
  각주와 상호참조는 아직 rhwp에서 지원하지 않는다.
- 머리행 플래그는 `Cell::set_header`로 native LIST_HEADER와 함께 갱신한다.
  원형의 텍스트 폭 보정값, 반복 머리행·셀 쪽 나눔 설정, raw 개체 헤더와
  provenance seal을 보존·갱신한다. 임시 저장 후 엄격한 자체 모델 재열기를 유지한다.

## 열 폭과 Windows 의존성

`md2hwp-font-measurement`는 Windows GDI 핸들을 소유하고 안전한 Rust API를 제공한다.
필요한 unsafe FFI는 이 작은 crate 안에만 있으며, rhwp 실행 파일의 unsafe 금지는
유지한다. 글꼴 핸들은 다른 스레드로 보내지 않고 DC와 함께 정상 해제한다.

기존 C#과 같은 4배 논리 배율로 설치된 TTF/OTF의 실제 폭을 측정한다.
언어별 글꼴·크기·장평·자간·강조와 grapheme 단위를 사용한다. HFT 또는 GDI의
다른 글꼴 대체는 거부한다. 문단 여백은 URC에서 한 번만 변환하고 셀 여백,
1% 폭 여유와 최소/선호 폭 배분을 적용한다. `table.width-mm`가 최소 폭보다
작으면 오류다. 생성 위치의 실제 SECDEF/COLDEF를 따라 가용 폭도 검사한다.
같지 않은 폭의 다단은 거부한다.

첫 페이지 비교에서는 일부 열 폭이 달랐다. 원인은 native 문단 attr1 bit7과
upstream HWPX 문자열 해석의 방향 차이였다. 한글 HWPML 기준으로 bit7=0은
비라틴 단어를 유지하고, bit7=1은 단어 안에서 나눔을 허용한다. 이 규칙으로
고친 뒤 GDI 독립 계산 및 한글 기준 출력과 열 폭이 정확히 같아졌다.
이 비트 방향에 대한 회귀 테스트를 추가했다.

| 표 | 계산된 열 폭, HWPUNIT |
| --- | --- |
| 지역별 현황 | 4282 / 10967 / 25003 |
| 캡션 없는 표 | 20387 / 19865 |
| 한 열 | 40252 |
| 머리행만 있음 | 20126 / 20126 |
| 56행 긴 표 | 3554 / 9101 / 27597 |

이는 추적 템플릿에서 얻은 값이다. 다른 글꼴·서식의 템플릿은 그 서식으로 다시
측정하므로 열 폭이 달라지는 것이 정상이다. 표 생성은 현재 Windows와 설치된
글꼴에 의존하지만 .NET 또는 한글 프로세스를 실행하지 않는다.

## 저장·표시 비교

[검증 원고](../tests/fixtures/rhwp-tables.md)는 5개 표에 정렬·빈 셀·강조·링크 표시
문구·한자·여러 출처·캡션 생략·한 열·머리행만 있음·56행 긴 표를 포함한다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
New-Item -ItemType Directory -Force artifacts/rhwp-tables
target/debug/md2hwp.exe md2ir tests/fixtures/rhwp-tables.md --output artifacts/rhwp-tables/source.ir.json
target/debug/md2hwp-rhwp.exe artifacts/rhwp-tables/source.ir.json artifacts/rhwp-tables/rhwp.hwp --template templates/template.hwp
target/debug/md2hwp-backend.exe ir2hwp --ir artifacts/rhwp-tables/source.ir.json --output artifacts/rhwp-tables/hancom.hwp --template templates/template.hwp
```

Pandoc·.NET·보안 모듈의 준비 조건은 기존과 같다.

| 검사 | 결과 |
| --- | --- |
| 임시 저장·디스크 재읽기·독립 기대 모델과 재열기 비교 | 통과 |
| 표 크기·위치·열 폭·행/열 주소·병합·머리행 플래그 | 5개 모두 일치 |
| 캡션 옵션·자동번호·셀 텍스트·스타일·출처 문단 | 5개 모두 일치 |
| 추적 템플릿 / 한글 300dpi 페이지 비교 | 양쪽 6쪽, 전부 차이 0픽셀 |
| 별도 기본 템플릿 / 한글 300dpi 페이지 비교 | 양쪽 4쪽, 전부 차이 0픽셀 |
| 최소 폭을 넘는 긴 단어 | 명확한 오류, 기존 출력 SHA-256 유지 |
| Rust / Clippy / fmt / C# 계약 테스트 | 195개 통과 / 통과 / 통과 / 통과 |

추적 템플릿 결과의 2·5·6쪽과 별도 기본 템플릿의 3·4쪽을 직접 확인했다.
원본 템플릿은 변경하지 않았다. 최초 C# 기준 생성에서 기존의 간헐적 제목 표
높이 오류 `4160 → 2500`이 한 번 발생했고, 동일 입력 재실행은 통과했다.
이 오류를 무시하도록 검사 코드를 변경하지 않았다.

## 페이지 나눔의 구분과 한계

**현재 추적 템플릿의 표는 TreatAsChar=true이다.** 이 설정에서 긴 표는 두 백엔드
모두 22번 행 부근에서 페이지 아래로 잘리며, 나머지 행과 출처는 XML에 있어도
보이지 않는다. 6쪽의 픽셀 일치는 이 문제까지 같다는 뜻이며 조판 안전성은 아니다.

별도 `init-template` 결과는 TreatAsChar=false이며 표가 여러 페이지로 이어진다.
같은 원고로 56번 마지막 행, 두 출처 문단과 다음 본문까지 확인했다. 반복 머리행과
셀 내부 페이지 분할의 실제 표시도 두 출력이 같았다. 셀이 페이지 경계에서
분할될 때의 반복 머리행 표시는 한글의 native 동작을 따른다.

```powershell
target/debug/md2hwp-backend.exe init-template --output artifacts/rhwp-tables/paged-template.hwp
target/debug/md2hwp-rhwp.exe artifacts/rhwp-tables/source.ir.json artifacts/rhwp-tables/rhwp-paged.hwp --template artifacts/rhwp-tables/paged-template.hwp
target/debug/md2hwp-backend.exe ir2hwp --ir artifacts/rhwp-tables/source.ir.json --output artifacts/rhwp-tables/hancom-paged.hwp --template artifacts/rhwp-tables/paged-template.hwp
```

한글 백엔드의 `NativeTablePagination`은 첫 머리행만 앞 페이지에 남는 경우를
실제 셀 좌표로 탐지해 표 앞 쪽 나눔을 보정한다. 이 보정은 아직 rhwp에 이식하지
않았다. 이번 원고의 비교 성공을 모든 원고의 고립 머리행 방지나 임의 템플릿의
조판 보장으로 확대하지 않는다. 전체 HWPML 동일성도 주장하지 않는다.

## 보관 근거

추적 템플릿 SHA-256:
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`

추적 템플릿 rhwp 출력:
`C7AA1F479FA990725214295A583EEF4D296B1D1F461F5E01616D4AF1225F0E91`

별도 기본 템플릿:
`FE0945F6FC861053BB57F78769EB6B3E3B2ED1CC25FEFCF44427647D071C1747`

별도 기본 템플릿 rhwp 출력:
`6A3525D91CAF25F82C749928F8B239C79EDDB68C30A5C9CB9351AB0C2E0BC1C3`

HWP·XML·페이지 PNG·픽셀 비교·구조 요약·실패 보존 결과는 ignored
`artifacts/rhwp-tables/`에 보관한다. Git에는 원고·회귀 테스트와 이 기록을 남긴다.
