# EMF PostScript 출력 검토 — 2026-10-09

한글 2020에 원본 EMF를 내장한 HWP에서 PS 파일 출력을 실제 수행했습니다. **전체 그림을 JPEG로 바꾸지 않고 벡터·글자·비트맵을 혼합해 전달했습니다. 다만 모든 글자가 벡터로 유지되지는 않았습니다.** 실제 인쇄 파일과 실행 명령 분석, 독립 렌더링 페이지를 근거로 합니다.

## 시험 환경

- Windows, 한글 2020 HWP 11.0.0.9136, 사용자 DESKTOP-BRTN48S\MOLIT / Session 1 / Windows PowerShell 5.1 x64 STA.
- 시스템에 이미 있는 Microsoft PS Class Driver 패키지를 등록하고 `md2hwp PS verification`이라는 전용 프린터를 FILE: 포트에 만들었습니다. 등록 작업은 현재 실행 권한에서 성공했습니다. 별도 제조사 드라이버 다운로드·관리자 설정 우회는 하지 않았습니다.
- 이 드라이버는 **v4, XPS 기반**입니다. 앞서 설명한 v3 Pscript 경로와 동일한 드라이버라고 취급하지 않습니다. 드라이버의 최종 PS 결과를 검증했습니다.
- PostScript Level 3, 컬러, A4, 600dpi. 한글 Print 액션의 PrinterName·Device=0·PrintToFile=1·FileName을 다시 읽어 확인한 뒤 실행했습니다. 실제 프린터로 전송하지 않았습니다.
- 원본 EMF/HWP는 수정하지 않았으며 기존 세 문서의 해시를 출력 후 확인했습니다. 서체 추가 시험은 별도의 새 EMF/HWP에서 수행했습니다.
- 보안 모듈 파일·REG_SZ를 읽어 확인하고 RegisterModule 성공 후 문서를 열었습니다. 보안 모듈 설치·레지스트리 변경은 하지 않았습니다. 기존 한글 프로세스를 종료하지 않았습니다.
- 독립 PS 렌더링은 별도 PS 해석기로 수행했습니다. 설치 프로그램을 실행하거나 시스템에 서체를 추가하지 않고 도구의 기본 리소스로 렌더링했습니다. 도구 버전·해시·실행 설정은 ignored 원시 실행 기록에 보존했습니다. LibreOffice는 사용하지 않았습니다.
- 원본 `.ps`에는 PJL 앞뒤 포장이 있습니다. 분석·렌더링용 `*-payload.ps`는 `%!PS-Adobe`부터 `%%EOF`까지의 바이트만 복사한 것입니다. 원본 PS는 보존했습니다.

## 결과

명령 수는 PS 본문이나 공통 prolog에서 키워드를 검색한 수가 아닙니다. PS 해석기의 실제 `stroke`, `fill`, `eofill`, `show`, `image`, `filter`, `defineresource` 실행을 감싼 계측으로 얻었습니다. 서체 자원 수는 복합 서체의 Font/CMap/CIDFont를 각각 따로 세지 않고 CIDFont 자원 수로 표시합니다.

| 샘플 | PS 바이트 | stroke | fill / eofill | show | 이미지 | CIDFont 자원 | 실제 FlateDecode | DCTDecode |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 일반 EMF | 128,414 | 7 | 2 / 4 | 45 | 2 | 1 | 2 | 0 |
| EMF+ Dual | 69,468 | 7 | 10 / 2 | 6 | 5 | 1 | 5 | 0 |
| EMF+ Only | 69,468 | 7 | 10 / 2 | 6 | 5 | 1 | 5 | 0 |
| 서체 / 일반 EMF | 188,705 | 0 | 0 / 1 | 66 | 4 | 3 | 4 | 0 |
| 서체 / EMF+ Dual | 59,827 | 0 | 1 / 0 | 0 | 13 | 0 | 13 | 0 |

EMF+ Dual과 Only의 원본 HWP는 다르지만 이 PS 출력 바이트는 정확히 같습니다. 모든 샘플에서 실제 이미지 데이터의 압축은 **FlateDecode**이며 JPEG용 DCTDecode는 없었습니다. 이 결과를 모든 PS 드라이버·그림에 확대하지 않습니다.

기본 도형 샘플의 일반 EMF에는 403×404와 16×16 이미지 두 개가 있습니다. 앞의 이미지는 반투명 겹침 영역, 뒤는 원본 체크무늬 비트맵에 해당합니다. 나머지 선·도형과 텍스트의 명령은 남았습니다.

EMF+에는 747×72, 498×90, 754×96 등 글자 영역 이미지도 추가됐습니다. 회전 글자에는 show와 CIDFont 자원이 남지만, 다른 글자 영역은 비트맵으로 전달됐습니다. 따라서 **벡터 유지와 텍스트 유지 여부는 요소별로 구분해야 합니다.**

`show` 호출은 글자·문단 수와 같지 않습니다. `fill`에는 도형뿐 아니라 드라이버가 채우는 배경도 포함될 수 있으므로 표의 숫자를 도형 개수로 해석하지 않습니다. Type 42/CIDFontType 2, sfnts와 GlyphDirectory 데이터가 PS에 포함된 것도 확인했습니다.

## 서체와 화면

별도 EMF에는 실제 GDI 선택 결과를 확인한 다음 네 서체를 사용했습니다.

- Malgun Gothic
- KoPubWorldBatang_Pro Light
- KoPubWorldDotum_Pro Light
- Times New Roman

일반 EMF와 EMF+ Dual을 각각 한글에 내장·저장·재열기하고 네 줄의 한글/영문/숫자를 PS로 출력했습니다. PS를 렌더링한 전체 페이지와 한글의 lossless 600dpi 페이지를 비교해 글자 모양과 내용이 표시되는 것을 확인했습니다. 일반 EMF에서도 일부 서체/글자 영역은 이미지로 전달됐으며, 이 EMF+ 서체 샘플은 **모든 글자가 이미지 데이터**로 전달됐습니다. 표시 성공을 전부 글꼴 다운로드나 검색 가능한 텍스트의 증거로 쓰지 않습니다.

기본 도형·회전·클리핑·반투명·원본 비트맵은 PS 페이지에서 표시됐습니다. 일반 EMF의 굵은 녹색 점선 모양은 한글 원래 페이지에도 있으며 PS에서 새로 생긴 문제가 아닙니다. 확대 비교에서는 선 끝 모양·안티앨리어싱·글자 간격 차이가 남습니다. 한글과 PS의 완전 픽셀 일치를 주장하지 않습니다.

한글 페이지는 4961×7016, PS 렌더링 페이지는 4958×7017픽셀입니다. 드라이버의 A4 595×842pt와 한글 페이지 계산의 차이가 있습니다. 비교 그림은 각 페이지의 실제 비백색 내용 경계로 잘라 나란히 붙였습니다. 확대·축소·왜곡 정합은 하지 않았습니다. 경계 정렬은 시각적 비교를 돕기 위한 것이며 수치적 픽셀 동일성 검증이 아닙니다. 모든 PS 페이지는 600dpi png16m, TextAlphaBits=4, GraphicsAlphaBits=4로 렌더링했습니다.

## 파일

실측 자료는 ignored 작업 경로 `artifacts/emf-2026-10-09/ps/`에 있습니다.

- [명령·이미지·서체·페이지 집계](../artifacts/emf-2026-10-09/ps/analysis.json)
- [일반 EMF PS 원본](../artifacts/emf-2026-10-09/ps/EmfOnly.ps)
- [EMF+ PS 원본](../artifacts/emf-2026-10-09/ps/EmfPlusOnly.ps)
- [EMF+ PS 전체 페이지](../artifacts/emf-2026-10-09/ps/EmfPlusOnly-page-600.png)
- [일반 EMF의 한글/PS 확대 비교](../artifacts/emf-2026-10-09/ps/EmfOnly-lines-compare.png)
- [서체 EMF+ PS 전체 페이지](../artifacts/emf-2026-10-09/ps/Fonts-EmfPlusDual-page-600.png)
- [서체의 한글/PS 비교](../artifacts/emf-2026-10-09/ps/Fonts-EmfPlusDual-compare.png)
- [서체 원본 내장 HWP](../artifacts/emf-2026-10-09/ps/native-Fonts-EmfPlusDual/picture.hwp)

각 시험의 원본 PS, payload PS, 실행 trace, 렌더링 PNG, 원본 HWP/EMF 해시와 도구 해시를 보존했습니다. `capture.json`, `fonts-native-capabilities.json`, `*-trace.log`가 원시 근거입니다.

## 정리와 범위

시험 후 전용 프린터와 이번에 추가한 미사용 PS 드라이버 등록을 제거했습니다. 기존 프린터의 이름·드라이버·포트 목록이 전후 일치합니다. 원본 HWP/EMF·템플릿·프로그램 코드는 변경하지 않았습니다. 실제 종이 출력, 다른 PS 드라이버, 모든 EMF 명령/서체의 호환성은 시험하지 않았습니다.

PS 파일은 인쇄 시점에 만들어지는 출력물입니다. HWP에 원본 EMF를 내장하는 기존 구현은 유지되며, 사용자에게 PS 드라이버 설치를 요구하거나 PS 변환을 프로그램에 추가하지 않았습니다.

## 도구 출처

- 드라이버 설명: [Microsoft Pscript capabilities](https://learn.microsoft.com/en-us/windows-hardware/drivers/print/pscript-capabilities). 이 문서는 일반 v3 Pscript의 기능 설명이며, 이번 v4 MSxpsPS 실측을 대신하지 않습니다.
## PS → PDF 추가 확인

위 EMF+ Only PS를 외부 변환 도구로 PDF로 변환했습니다. [변환 PDF](../artifacts/emf-2026-10-09/ps/EmfPlusOnly-from-ps.pdf)는 A4 한 페이지, 20,208바이트입니다. [PDF 페이지 이미지](../artifacts/emf-2026-10-09/ps/EmfPlusOnly-from-ps-page.png)를 Poppler 150dpi로 렌더링해 그림·한글·회전 글자 표시를 확인했습니다.

PDF에 stroke/fill/곡선·직선 경로와 포함된 Type0 서체가 남습니다. 일반 이미지 XObject 네 개의 필터는 모두 FlateDecode이고, 16×16 체크무늬는 인라인 이미지로 전달됩니다. EMF 그림 전체를 JPEG 이미지로 바꾸는 한글의 앞선 기본 품질 PDF 저장과 다른 결과입니다. 본문 텍스트까지 페이지 전체를 이미지로 바꾸었다는 뜻은 아닙니다. Downsample을 끄고 Color/Gray 이미지를 FlateEncode로 지정했으며 실행 인자를 함께 보존했습니다.

PS 단계에서 이미 비트맵으로 바뀐 글자는 PDF에서도 비트맵입니다. 남은 회전 글자 서체에는 ToUnicode 매핑이 없으며, 표시되는 ROTATE의 추출 텍스트는 올바르지 않습니다. Poppler도 원본 PS에서 유래한 synthetic character collection 경고를 냈지만 페이지는 정상적으로 표시됐습니다. 따라서 이 결과는 시각적 출력용 PDF이며 텍스트 검색·복사·접근성 정보를 완전하게 보존한다고 주장하지 않습니다.

[PDF 구조 검증 JSON](../artifacts/emf-2026-10-09/ps/ps-to-pdf-verification.json)과 `ps-to-pdf-arguments.json`에 실측 및 변환 설정을 저장했습니다. 원본 PS는 변경하지 않았습니다.
## PS → PDF의 본문 검색 추가 시험

그림만 있는 문서와 별개로 기존 9쪽 `release-xml/emf.output.hwp` 전체를 같은 Microsoft PS Class Driver로 출력했습니다. [PS 경유 원고 PDF](../artifacts/emf-2026-10-09/ps-body/manuscript-from-ps.pdf)는 9쪽, 121,917바이트이며 본문·제목·캡션·각주의 글자가 화면에 표시됩니다. Poppler로 모든 페이지를 렌더링하고 본문 페이지를 확인했습니다.

한컴 자체 PDF는 한글 261자가 정상 추출됐습니다. 본문/제목/각주의 검색어 `다음 장의 그림`, `현재 장의 그림`, `각주 안에서`, `첫 번째 장`, `일반 EMF`, `그림 3-1`은 모두 있었습니다. 동일 HWP의 PS 경유 PDF에서는 해당 검색어 여섯 개가 모두 없었고, 추출 텍스트에 정상 한글 음절은 0개였습니다. 출력된 글자는 실제 텍스트/서체로도 남지만 Type0 서체의 ToUnicode 매핑이 모두 없어서 엉뚱한 문자로 추출됩니다. 이 환경에서는 그림 외의 본문도 검색 가능한 PDF로 보존되지 않았습니다. 이를 모든 PS 드라이버·PS 파일의 한계로 일반화하지 않습니다.

[검색·서체 비교 JSON](../artifacts/emf-2026-10-09/ps-body/search-verification.json)과 각 PDF의 추출 텍스트를 작업 폴더에 저장했습니다. 전용 프린터와 추가 드라이버 등록을 제거했고 기존 프린터 목록은 전후 일치합니다.

전체 문서 출력은 비동기여서 Print 실행 직후 생긴 파일에 곧바로 해시를 계산하면 파일 잠금으로 실패했습니다. 완료된 PS의 EOF·읽기 가능 여부를 확인한 후 변환을 진행했습니다. 원본 HWP를 바꾸거나 인쇄를 재실행하지 않았습니다. 향후 출력 도구에는 완료 대기와 실패 시 결과 보존이 필요합니다.