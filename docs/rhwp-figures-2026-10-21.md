# rhwp 그림 생성 시험 — 2026-10-21

구현 커밋은 `da293bd`이다. IR 0.4, 기본 템플릿과 고정 rhwp revision을 유지했다. 생성 자체는 COM·.NET 없이
수행하며, 독립 재열기와 페이지 비교에는 한글 2020 11.0.0.9136을 사용했다.
기본 백엔드와 릴리스 ZIP 구성은 변경하지 않았다.

## 지원 범위

- PNG/JPG/JPEG/EMF 원본 바이트를 HWP 내부에 내장한다. 샘플 그림의 이미지·크기·
  자르기·변환을 복제하지 않고 `figure.max-width-mm`와 원본 비율로 새 그림을 만든다.
- `begin:figure/end:figure`의 native 캡션 옵션과 두 샘플 문단을 사용한다.
  캡션의 실제 Figure AUTONUM 및 글자 서식, 출처 접두어·내용의 개별 서식과 문단
  모양을 보존한다. 출처는 문단마다 복제하며 없으면 해당 문단을 생략한다.
- `num:heading1`은 대상 그림의 장 번호로 치환한다. 첫 그림에 native NEWNUM을
  넣어 장별 그림 번호를 다시 시작한다. 장 번호 슬롯이 있는데 앞선 heading1이
  없으면 오류다. 문구와 원고의 태그 모양 리터럴을 재해석하지 않는다.
- 원형 앞뒤의 빈 문단과 쪽 나눔을 복제한다. 개체 식별자와 그림·코드 박스의
  ZOrder를 새로 할당하며 정적 템플릿 개체의 순서는 바꾸지 않는다.
- 파일별 64 MiB, JPEG SOF0/1/2 및 EMF 헤더·레코드·EOF 경계를 검사한다.
  이는 전체 이미지 디코더나 EMF 그리기 명령의 의미 검증은 아니다.
- 경로는 IR 폴더에서 해석한다. 단독 축약 호출은 IR 폴더 안으로, 명시적
  `ir2hwp` 호출은 실행 cwd 안으로 제한한다. 저장 직전 경로·원본 바이트를 다시
  확인하고 원본 이미지에 결과를 덮어쓰지 않는다.

템플릿 해석은 원고에서 그림을 사용하지 않더라도 수행한다. 각주·상호참조는
기존 rhwp 미지원 범위이며, 한글 백엔드로 자동 전환하지 않는다.

## 실제 표시에서 발견한 문제

첫 자체 저장·재열기 검사는 통과했으나 한글 페이지에서는 그림이 꼬여 보였다.
고정 upstream의 `border_x`와 `border_y`는 각각 독립된 x/y 좌표 배열이 아니라,
HWP5에 연속 저장되는 좌표 8개를 네 개씩 나눈 모델이다. 실제 저장 순서는
`x0,y0,x1,y1,x2,y2,x3,y3`이다. 이 순서로 사각형을 구성했다.

또한 캡션 AUTONUM 값만 설정하면 한글이 모든 그림을 1번으로 표시했다.
개체 공통 속성의 native 그림 번호 범주(bits 26–28 = 1)와 모델의 Picture 범주를
함께 지정해 해결했다. 줄 간격 영향·본문 흐름·상대 기준도 한글 백엔드의
TreatAsChar 그림 배치와 맞췄다. 자체 모델 비교를 완화하지 않았으며,
실제 템플릿의 PNG/JPEG/EMF export/reparse 및 원본 리소스 바이트 회귀 검사를 추가했다.

## 검증 결과

[검증 원고](../tests/fixtures/rhwp-figures.md)는 3장부터 시작하는 두 장에 그림 5개,
강조 캡션, 반복 PNG, 출처 없음·여러 출처, JPG/JPEG와 일반 EMF를 포함한다.
기존 tracked 예제 이미지를 참조하므로 아래 명령은 저장소 루트에서 실행한다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
New-Item -ItemType Directory -Force artifacts/rhwp-figures
target/debug/md2hwp.exe md2ir tests/fixtures/rhwp-figures.md --output artifacts/rhwp-figures/source.ir.json
target/debug/md2hwp-rhwp.exe ir2hwp --ir artifacts/rhwp-figures/source.ir.json --output artifacts/rhwp-figures/rhwp.hwp --template templates/template.hwp
target/debug/md2hwp-backend.exe ir2hwp --ir artifacts/rhwp-figures/source.ir.json --output artifacts/rhwp-figures/hancom.hwp --template templates/template.hwp
```

Pandoc·.NET·한글 보안 모듈의 준비 조건은 기존과 같다.

| 검사 | 결과 |
| --- | --- |
| rhwp 임시 저장·디스크 재읽기·엄격 모델 재열기 | 통과 |
| 저장된 리소스 ID·확장자·원본 바이트 | 모든 리소스 일치 |
| 한글 재열기 | 양쪽 모두 6쪽 |
| 그림 크기·위치·사각형, 캡션 옵션·스타일·텍스트·번호 | 5개 모두 일치 |
| 그림 번호 / 출처 문단 수 | 3-1·3-2·3-3·4-1·4-2 / 2·0·1·1·0 |
| 한글 300dpi 페이지 픽셀 | 1·2·4·5쪽: 차이 0개 |
| 3·6쪽 | 각 1,813,534개 차이, JPEG 그림 영역 안에만 존재 |
| 손상된 JPEG 입력 | 오류, 기존 출력 SHA-256 유지 |
| Rust / Clippy / fmt / C# 계약 테스트 | 176개 통과 / 통과 / 통과 / 통과 |

2·3·5·6쪽의 그림·번호·캡션·출처와 다음 본문을 직접 확인했다.
전체 HWPML 동일성을 주장하지 않는다. native 식별자·ZOrder·행렬 표현과 일부
캐시는 백엔드마다 다르며, 겹치는 정적 개체가 있는 모든 템플릿의 배치를 보장하지 않는다.

JPEG 차이는 단순 허용 오차로 숨기지 않았다. 한글 기준 파일은 JPEG를
재인코딩하고 두 원고 파일을 하나로 합쳤다. rhwp 파일은 각 원본 JPEG를 그대로
보존했다. 한글에서 내보낸 XML의 BINDATA를 디코딩해 PNG·JPEG·EMF 원본과
바이트 비교했다. JPEG 영역 밖의 캡션·번호·여백·본문은 픽셀이 일치한다.
rhwp는 PNG의 중복도 합치지 않으므로 파일 크기가 한글 결과와 다를 수 있다.

일반 EMF를 이번 원고에서 렌더링했다. 이 시험만으로 모든 EMF+ 또는 임의의
EMF 그리기 명령에 대해 독립 엔진 표시 동등성을 주장하지 않는다.

템플릿 SHA-256:
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`

검증 출력 SHA-256:
`C1403E627C0DCBC2CA634C6933C033114C8758EB08F27E1230713335848B3E8E`

HWP·페이지 PNG·XML·픽셀 경계·리소스 비교·실패 보존 검증 결과는 ignored
`artifacts/rhwp-figures/`에 있으며 Git에는 시험 원고·회귀 테스트와 이 기록을 남긴다.
