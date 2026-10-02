# 개발 참고 자료

IR 0.2 / v0.2.0 개발에 참고하는 ignored 파일의 보존 목록입니다.
자료를 별도 압축본으로 전달하거나 다시 준비할 때 파일과 SHA-256을 확인하세요.
해시는 현재 로컬 파일의 식별값이며, 프로그램 실행을 제한하는 버전 검사가 아닙니다.
이 문서와 Git에 포함된 소스만으로 빌드할 수 있습니다. 아래 참고 자료는 양식·서식과 API를 확인하는 데 사용합니다.

기준일: 2026-10-02. 아래 표의 경로는 이 문서가 있는 `reference/` 기준입니다.

## AURI 보고서 자료

출처: 로컬에 보관된 AURI 연구보고서 편집양식(2026) 자료. 원본 다운로드 주소는 미확인입니다.

| 파일 | 용도 | SHA-256 |
| --- | --- | --- |
| `auri/01 auri 기본연구보고서 작성양식.hwp` | 기본 보고서의 구조·서식 확인 | `0BA84133775B182C76ACE779082ED77E4BCE0743C00733FB6C7AF7CD43B3BAF1` |
| `auri/★ auri 연구보고서 작성 매뉴얼.pdf` | 보고서 작성·편집 규칙 확인 | `584253C98926CC3CA1D04E4844B6D59451040A12CD0BCE7149E555329736ADA1` |
| `auri/★ 연구보고서 스타일 목록표.hwp` | 이름 있는 스타일의 서식 확인 | `D9EA39ED3FD7D9C50D71FD83FBE097DD018361AF29BDDDE50C05B40AF6571F16` |

같은 세 파일이 `auri/auri 연구보고서 편집양식(2026)/`에도 동일한 해시로 있습니다.
`01 auri 기본연구보고서 작성양식 - 복사본.hwp`도 기본 양식과 동일합니다. 보존할 때 한 세트만 있으면 됩니다.

## KoPubWorld 서체

로컬 출처: AURI 편집양식(2026)의 `서체/` 폴더.
관련 배포 페이지: [한국출판인회의 KoPubWorld](https://www.kopus.org/biz-electronic-font2/).
확인 시 해당 페이지가 HTTP 403을 반환했습니다. 개별 파일의 원본 다운로드 주소와 배포 버전은 미확인입니다.
로컬에 보관된 KoPubWorld 바탕체·돋움체의 TTF 6개와 Pro OTF 6개를 함께 기록합니다.
기본 템플릿의 선언만으로 실제 사용 여부를 판정하지 않으며, 아래 표는 보존할 서체 파일의 목록입니다.

| 파일 | 서체 | SHA-256 |
| --- | --- | --- |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Batang Bold.ttf` | 바탕체 Bold | `72122F815CAD001C97269BB99337647336B5597D2C3F749A49FB879C9E6AE9FF` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Batang Light.ttf` | 바탕체 Light | `E3EE21A86B6A6728C567A95AAEBD8883480F27CE4F230207B0D7266B5CB3FB18` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Batang Medium.ttf` | 바탕체 Medium | `F2C31FFD195EA0FE74C9E720592930ED528C52EE7876BAFFAEFF24D42C133D6D` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Dotum Bold.ttf` | 돋움체 Bold | `C9DC58E806CF639AD33D7C59B06848D2F4C3CF3B367CE52CD456F79A68407635` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Dotum Light.ttf` | 돋움체 Light | `069494CCE21A4222C88E537F256B6F46FEE209375ABA769F82431B2D382BC84F` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Dotum Medium.ttf` | 돋움체 Medium | `6269624BD0C5AE8746A8E731B8087F056088AF930A7BBC0EFB76902BF732A293` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Batang_Pro Bold.otf` | 바탕체 Pro Bold (OTF) | `E7B2E5AB08D0D39B5A09417A1970FD8E0D7FD5464821339ACCD40AA61583B4DD` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Batang_Pro Light.otf` | 바탕체 Pro Light (OTF) | `895FDC6DE0FF0FE24B1A63AE16601C174C810B24DAA23ADE78115B7E134C4C0A` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Batang_Pro Medium.otf` | 바탕체 Pro Medium (OTF) | `2D385FFBB351F41CBE82D981A0BC10315E01F4E113424A0ED1B7291A801479BB` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Dotum_Pro Bold.otf` | 돋움체 Pro Bold (OTF) | `650F21FD744674BE266F7A40095FC8838A2C2FEF05A3F4634362C022D2F81216` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Dotum_Pro Light.otf` | 돋움체 Pro Light (OTF) | `529B2F02B96276D9209124A72181FCD7BFC656A567718670D0C3934F6C11ADEA` |
| `auri/auri 연구보고서 편집양식(2026)/서체/KoPubWorld Dotum_Pro Medium.otf` | 돋움체 Pro Medium (OTF) | `073A3426827351E393BA290A59A072AECD830A5DAF2E9692828CA20F3D5FF4A0` |

## 한글 자동화 API 문서

[한컴 공식 한글 오토메이션 안내](https://developer.hancom.com/hwpautomation)에서 2504 문서의 다운로드 주소를 확인했습니다.
아래 해시는 로컬 PDF의 값입니다. 공식 다운로드 주소에 있는 현재 파일의 해시와는 대조하지 않았습니다.
개별 주소를 확인하지 못한 구형 문서는 미확인으로 표시합니다.

| 파일 | 용도 | SHA-256 | 다운로드 출처 |
| --- | --- | --- | --- |
| `hancom-api/ActionObject.pdf` | 액션 객체 | `D3D4434D6CD9E14BFE3D085A346D7A1B23290192DBF594678E69E517B12A2A29` | 원본 다운로드 주소 미확인 |
| `hancom-api/HwpAutomation.pdf` | 기존 자동화 객체 문서 | `C787C69D46F26A9FCA49AC9D34106BB4F146FB29DA9F1CAF92C6E77D46CEF45B` | 원본 다운로드 주소 미확인 |
| `hancom-api/ParameterSetObject.pdf` | 파라미터 세트 객체 | `7CBF536A8814CF8165A1614EDFD9816173BB1BD7C9E585821D776563E2BDB674` | 원본 다운로드 주소 미확인 |
| `hwpautomation/ActionTable_2504.pdf` | 액션 목록 | `BB876A95EA4E0F4B052C94188A1390B5F981161223759F59F52FC324C8130A87` | [한컴 공식 페이지 연결](https://github.com/hancom-io/devcenter-archive/raw/main/hwp-automation/ActionTable_2504.pdf) |
| `hwpautomation/HwpAutomation_2504.pdf` | 자동화 객체 문서(2504) | `917142DE2704AE037B0132F5A344CE5331B1FD96CC19DD574666AAACFBF0268C` | [한컴 공식 페이지 연결](https://github.com/hancom-io/devcenter-archive/raw/main/hwp-automation/HwpAutomation_2504.pdf) |
| `hancom-api/ParameterSetTable_2504.pdf` | 파라미터 세트 목록(2504) | `6FA10E0AE69711AF99A03E54C321C6C922796BD943BADF8C21B2C7DFB780AD6E` | [한컴 공식 페이지 연결](https://github.com/hancom-io/devcenter-archive/raw/main/hwp-automation/ParameterSetTable_2504.pdf) |
| `hwpautomation/한글오토메이션EventHandler추가_2504.pdf` | 이벤트 처리 문서(2504) | `ED58788419119D2C73E2ABDC8195EDEB2439C5BD1A14A2C91EC034C5D25B3AFE` | [한컴 공식 페이지 연결](https://github.com/hancom-io/devcenter-archive/raw/main/hwp-automation/%ED%95%9C%EA%B8%80%EC%98%A4%ED%86%A0%EB%A9%94%EC%9D%B4%EC%85%98EventHandler%EC%B6%94%EA%B0%80_2504.pdf) |

`hwpautomation/ParameterSetTable_2504.pdf`는 위 표의 `hancom-api/ParameterSetTable_2504.pdf`와 동일한 해시입니다.
PDF에서 추출한 `.txt` 파일은 검색 편의를 위한 자료이며 PDF로 다시 만들 수 있습니다.

## 해시 확인

저장소 루트의 PowerShell에서 다음처럼 확인합니다.

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath './reference/auri/01 auri 기본연구보고서 작성양식.hwp'
```

다른 해시의 자료를 받았다면 변경된 판본인지 확인하고 이 목록을 갱신하세요.
