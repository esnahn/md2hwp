# 한국어 Ipsum 대규모 원고

[`large-5x.md`](large-5x.md)와 검증된 [`large-5x.ir.json`](large-5x.ir.json)을 함께 보관합니다.

2026-10-05 작성 당시, 생성 HWP까지 검증한 원고 중 텍스트 분량이 가장 컸던
`artifacts/tables/long.md`를 기준으로 합니다. 이 원고는 본문 74행 표였고,
한글 출력 6쪽을 확인했습니다. 기준 원고의 SHA-256은 다음과 같습니다.

```text
e12cbb64f95626239f8a5dc8908a3af2b36ec6241e2bdb41ab96a79c8b373bf1
```

규모 단위는 공백으로 구분한 어절입니다. 기준 IR의 제목·표 셀·캡션·출처에
담긴 텍스트가 2,231어절이므로, 새 원고의 Ipsum 본문을 정확히 5배인
**11,155어절**로 만들었습니다. YAML 정보와 새 장·절 제목은 이 본문 수에 포함하지 않습니다.
쪽수는 표·그림·장 표지와 서식에 따라 달라지므로 5배 기준으로 사용하지 않습니다.

| 항목 | 규모 |
| --- | ---: |
| Ipsum 본문 | 11,155어절 |
| 본문 문단 | 118개 |
| 장 / 절 | 5장 / 20절 |
| IR 루트 블록 | 143개 |
| 본문 문자 수 | 50,541자 |
| Markdown UTF-8 파일 | 119,847바이트 |

본문은 저장된 후보 데이터를 사용하는 생성기로 작성하고, 문단 순서를 유지해
장과 절에 배분했습니다. 다음 명령은 같은 어절 수의 새 본문을 표준 출력으로 만듭니다.
난수를 사용하므로 문장은 매번 달라집니다. 동일한 원고를 비교할 때는 동봉된 파일을 사용하십시오.

```powershell
pwsh -NoProfile -File .\tools\development\generate-korean-lorem.ps1 11155
```

저장소 루트에서 IR을 다시 검증할 수 있습니다.

```powershell
.\target\release\md2hwp.exe md2ir .\examples\korean-lorem\large-5x.md --force
```

원고의 Markdown escape를 해제한 본문과 IR의 118개 문단이 문자 단위로 일치하며,
본문은 모두 일반 문단으로 해석됩니다. 의도치 않은 목록·표·링크·각주 노드는 없습니다.
이 원고로 HWP 생성·쪽수·COM 성능과 시각 배치는 아직 검증하지 않았습니다.
한글 문서를 닫은 뒤 다음 명령으로 HWP를 생성할 수 있습니다.

```powershell
.\target\release\md2hwp.exe .\examples\korean-lorem\large-5x.md --template .\templates\template.hwp
```

생성 HWP는 Git에서 제외하고, 원고와 IR은 추적합니다.
