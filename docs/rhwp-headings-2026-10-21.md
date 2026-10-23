# rhwp 제목 블록 시험 — 2026-10-21

IR 0.4와 rhwp 0.8.7 고정 커밋
`1a76570e833917d15817415a53c09ad61ab3203f`를 유지한다.
생성에는 rhwp만 사용하고, 비교 결과의 독립 재열기와 렌더링에는
한글 2020 11.0.0.9136을 사용했다. 기본 백엔드는 변경하지 않았다.

## 구현 범위

- 평문·번호 문단과 heading1–heading6 블록, 표·글상자·머리말·꼬리말 내부의 여러 제목 슬롯.
- 한 컨테이너 안의 인라인·여러 문단 once/except.once와 중첩 each.child.
- IR 순서에서 계산하는 own/ancestor 번호, ID 없는 제목, 생략된 상위 수준의 0,
  md2hwp-heading1-start, 자식별 번호와 기존 native 번호 양식의 형제 순번.
- 제목 슬롯의 글자 서식과 원고의 강조, 독립된 접두·접미 서식, 쪽 나눔.
- 제목 블록 내부 메타데이터. 원고·메타데이터의 태그 모양 리터럴은 재해석하지 않는다.
- 복제된 개체 식별자·raw payload·유효한 provenance seal 갱신과 필드·연결선 참조 연결.

원고의 목록·코드·그림·표·각주·상호참조 생성은 아직 지원하지 않는다.
템플릿 안 제목 장식용 표·글상자를 복제하는 것은 원고의 표 생성과 별개다.
제목 블록의 그림·OLE·구역 정의와 일부 필드가 포함된 조건부 문단 합치기는 거부한다.
이는 당시 상태다. 그림·OLE·구역 정의 금지는 한글 백엔드와 공통 계약이며,
조건부 필드 합치기는 [2026-10-23 후속 작업](rhwp-parity-2026-10-23.md)에서 지원했다.
IR·템플릿 문법과 버전, 기본 배포 구성은 바꾸지 않았다.

## 입력과 재현

원고는 [rhwp-headings.md](../tests/fixtures/rhwp-headings.md)이다.
장 두 개, 장마다 절 두 개, heading3–heading6, 강조·이모지·리터럴을 포함하며
장 번호는 3부터 시작한다. 템플릿은 수정하지 않은 `templates/template.hwp`이다.

템플릿 SHA-256:
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
.\target\debug\md2hwp.exe md2ir tests\fixtures\rhwp-headings.md --output artifacts\rhwp-headings\source.ir.json
.\target\debug\md2hwp-rhwp.exe artifacts\rhwp-headings\source.ir.json artifacts\rhwp-headings\rhwp.hwp --template templates\template.hwp
.\target\debug\md2hwp-backend.exe ir2hwp --ir artifacts\rhwp-headings\source.ir.json --output artifacts\rhwp-headings\hancom.hwp --template templates\template.hwp
```

Pandoc이 PATH에 없으면 `md2ir`에 `--pandoc` 경로를 지정한다.
한글 백엔드는 등록된 보안 모듈과 .NET이 필요하며, 두 문서는 순차 처리한다.

## 독립 검증 결과

| 검사 | 결과 |
| --- | --- |
| rhwp 임시 저장·디스크 재읽기·자체 재열기 모델 비교 | 통과 |
| 한글에서 두 결과 재열기 | 각각 7쪽 |
| 한글 300dpi 페이지 이미지 비교 | 7쪽 모두 다른 픽셀 0개 |
| 7쪽 시각 검토 | 장 표지·절 목록·각 수준 제목·쪽 나눔·강조·머리말 일치 |
| 원래 스타일 정의·글꼴 정의·CHAR 텍스트·필드·내장 데이터 | 일치 |
| 기존 전체 XML 구조 비교 | 차이 있음, 완전 일치로 보고하지 않음 |
| HWPX·미지원 원고·원본 덮어쓰기 거부 | 오류 반환, 기존 출력과 템플릿 보존 |
| 전체 Rust 테스트 / Clippy | 145개 통과 / 통과 |

원본 XML 비교를 통과시키기 위해 제품의 검증 규칙을 완화하지 않았다.
차이는 다음과 같이 조사했다.

1. 도형 16개의 변환 행렬 목록에서 한글 백엔드에 항등 행렬 36개가 더 있었다.
   항등 행렬을 제외한 계수·종류·순서와 유리수로 계산한 합성 행렬은 모두 같았다.
2. 같은 서식의 ID가 다르게 배정되었다. 번호 대신 실제 서식 정의를 비교했다.
3. 복제 개체 식별자는 두 생성기의 할당 방식에 따라 달랐다.
4. 제목 글상자 두 문단의 끝에 한글 백엔드만 빈 TEXT 요소를 남겼다.
5. 짝수 쪽 머리말 두 곳의 Page AUTONUM 저장값은 1과 2로 달랐다.
   native 번호 양식과 실제 7쪽 표시는 같았다. 이를 일반적인 캐시 정규화 규칙으로
   제품에 추가하지 않았다.
6. 마지막 빈 문단의 실제 문단·스타일·글자 서식 차이는 수정했다.

페이지 일치는 이 원고 길이와 설치된 글꼴에 대한 결과다. 긴 제목·많은 절로 인한
고정 글상자의 넘침을 보장하지 않으며, 새로운 native 개체 생성 성공을 뜻하지 않는다.

실행 산출물은 ignored `artifacts/rhwp-headings/`에 두었다.
최종 구조 비교는 `structure-comparison-final.json`, 상세 행렬 조사는
`matrix-investigation.json`, 페이지 비교는 `pixel-comparison.json`에 기록했다.
최종 페이지 이미지는 `rhwp-final-pages/`, 비교 기준은 `hancom-pages/`에 있다.
최종 rhwp 결과 HWP SHA-256은
`C8E7AA6B3CD4AC37890BF4BE0D459A5EAA4C9EE3AF4409ADBF76767EEE177316`이다.
