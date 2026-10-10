# 한글 2020의 PDF 저장과 Hancom PDF 인쇄 옵션

검토일: 2026-10-09. 설치된 한글 2020 도움말, 공식 온라인 도움말,
오토메이션 파라미터 문서와 설치된 Hancom PDF 드라이버를 읽어 확인했습니다.
최초 옵션 검토에서는 COM 문서 열기, PDF 생성 또는 시각적 비교를 수행하지 않았습니다.
후속 실측은 아래에 따로 기록했습니다.
최초 옵션 검토에서는 프린터 및 사용자 설정을 변경하지 않았습니다.
후속 그림 용량 시험에서는 해당 옵션만 일시 변경하고 원래 값으로 복원했습니다.

## 권장 PDF 저장 방법: PrintToPDFEx + GraphicQuality=100

PDF 저장은 PrintToPDFEx 액션에 GraphicQuality=100을 지정한 뒤
Action.Execute로 직접 출력하는 방법을 권장합니다.
2026-10-10에 비교한 세 경로와 품질값 가운데 이 조합만 시험한
PNG/JPEG/EMF 그림을 JPEG 재압축 없이 무손실 FlateDecode 압축으로
저장했습니다. 따라서 품질값과 저장 방법을 함께 권장합니다.

SaveAs(path,"PDF",arg) 및 FileSaveAsPdf에서는 같은 GraphicQuality 키로
시험한 모든 값이 기본 출력과 같았고 그림은 JPEG로 압축됐습니다.
이는 시험한 인수/항목과 환경에서 얻은 결과이며, 다른 미공개 설정이나
다른 한글 버전에서도 무손실 출력을 할 수 없다는 결론은 아닙니다.
JPEG로 저장되는 값 중 최고값을 판정한 것도 아닙니다.

```powershell
$action = $hwp.CreateAction('PrintToPDFEx')
$set = $action.CreateSet()
$action.GetDefault($set) | Out-Null
$set.SetItem('FileName', $pdfPath)
$set.SetItem('GraphicQuality', 100)
$action.Execute($set)
```

EMF는 여전히 래스터화되고, 그림 축소·색상 처리 등으로 원본 픽셀도
달라질 수 있습니다. Flate 압축만으로 원본 화질 보존을 보장하지 않습니다.
UI의 매우 높음과 이 API 값의 대응도 아직 확정하지 않았습니다.
1000/10000은 실제 동적 세트의 8비트 값으로 잘리므로 사용하지 않습니다.
FileSaveAsPdf 및 SaveAs 문자열 인수로 같은 키를 전달한 시험에서는 효과가
없었습니다. 위 설정은 PrintToPDFEx.Execute에 직접 지정하며, 그 뒤 같은
PDF 경로에 SaveAs를 호출하면 기본 품질로 다시 저장될 수 있습니다.

## 두 경로와 기존 시험의 범위

| 경로 | 설정 위치 | 자동화에서 확인한 진입점 |
| --- | --- | --- |
| 파일 → PDF로 저장하기 | 저장 대화 상자 → 저장 설정 | 기존 시험의 `SaveAs(path, "PDF", "")`; 한컴의 공식 답변도 이 호출을 안내 |
| 파일 → 인쇄 → Hancom PDF | 한글 인쇄 설정 + 프린터 설정 | Print 액션의 PrinterName 및 Print 파라미터 |

앞선 EMF/PDF 시험은 첫 번째 SaveAs 경로였습니다.
`artifacts/emf-2026-10-09/export-modes.ps1` 및 `native-capabilities.ps1`에
호출이 남아 있습니다. Hancom PDF 프린터 경로에 대한 검색 가능 여부,
EMF 벡터 유지, 그림 압축 방식 비교는 아직 없습니다.
메뉴 이름이 다르다는 사실만으로 내부 엔진의 독립성이나 품질 차이를 단정하지 않습니다.

인쇄 창에 있는 별도의 PDF 저장 항목은 환경 설정의 PDF 드라이버 선택과
연결됩니다. 파일 메뉴의 자체 PDF 저장과 이 항목을 혼동하지 않아야 합니다.

## 자체 PDF 저장 옵션

설치된 Hwp.chm의 `file/pdf_save.htm` 및 공식 [PDF 저장 설정](https://help.hancom.com/hoffice110/ko-KR/Hwp/file/pdf_save.htm)에서 확인했습니다.

- 범위: 문서 전체, 현재 쪽, 현재 구역, 현재부터, 현재까지, 일부분.
- 범위 해석: 사용자가 입력한 순서대로 저장, 문서 쪽 번호 기준으로 저장.
- 그림 품질: 자동, 매우 높음, 높음, 보통, 낮음.

매우 높음은 도움말에서 원본과 동일한 품질이라고 설명합니다.
이는 원본 파일 바이트 보존, 무손실 압축 또는 EMF의 벡터 보존을
검증한 결과가 아닙니다. 이 옵션의 실제 출력은 별도로 확인해야 합니다.

설치된 `file/to_pdf.htm` 및 [공식 한글 도움말](https://help.hancom.com/hoffice/multi/ko_kr/hwp/file/to_pdf.htm)은
PDF/A-1b 저장, HFT 텍스트 변환을 설명하면서 OTF 대체와 사용자 정의 용지
제한을 기재합니다. 해당 설명을 현재 출력 파일의 적합성 검증으로 간주하지 않습니다.
다른 제품인 한/쇼 도움말의 암호/권한 옵션을 한/글의 현행 옵션으로 전용하지 않았습니다.

## 설치된 Hancom PDF 프린터 옵션

드라이버 설정 DLL은 `C:/Windows/System32/spool/DRIVERS/x64/3/HNCE2PPRUIP.dll`이며
파일 버전은 0.3.6000.550입니다. 설치 원본의 동일 버전 DLL 및 한국어
리소스 `HNCE2PPRUIP.kor`를 실행 없이 Windows 리소스로 읽었습니다.
실제 화면에서 각 옵션의 활성화 여부를 시험한 것은 아닙니다.

| 분류 | 읽은 대화 상자 및 문자열 리소스 |
| --- | --- |
| 그림 출력 품질 | 자동, 매우 높음, 높음, 보통, 낮음 |
| 해상도 | 100, 150, 300, 600dpi |
| 그림 효과 | 컬러, 회색조, 흑백 |
| 글꼴 | HFT 폰트를 TTF 폰트로 변환 |
| 바인더 | 없음 및 A4의 2공/3공/4공/혼합 배치 |
| 저장 | 기본 저장 경로 사용, 저장 폴더 |
| 문서 보기 | 기본/쪽/책갈피/전체 화면 보기; 한 쪽/연속/두 쪽/맞쪽 등 |

품질은 DIALOG 109/112, 추가 설정은 DIALOG 110,
문서 보기는 DIALOG 111, dpi는 STRING 230–233에서 확인했습니다.

Get-PrintConfiguration으로 읽은 현재 프린터 PrintTicket은
A4, 컬러, 세로, 단면, 1부, 600×600dpi입니다.
그림 품질 단계와 HFT 변환 체크 상태는 이 표준 PrintTicket만으로 확정하지 않았습니다.

한글 자체의 인쇄 옵션에는 범위, 자동/용지 맞춤/모아 찍기/나눠 찍기,
배율, 역순, 그림·그리기 개체, 배경, 메모, 형광펜, 자동 머리말·꼬리말,
워터마크 등이 있습니다. 프린터의 그림 품질 설정과 별도의 계층입니다.
[인쇄 기본 도움말](https://help.hancom.com/hoffice/multi/ko_kr/hwp/file/print/print_general.htm).

## 자동화와 다음 비교의 기준

공식 ParameterSetTable의 Print에는 PrinterName, FileName, Device,
PrintMethod, PrintImage, PrintDrawObj 등의 인쇄 항목이 있지만,
위 Hancom PDF 전용 품질/HFT/보기 옵션의 직접 제어 방법은 확인하지 못했습니다.
SaveAs의 세 번째 문자열 인수에 임의의 quality/dpi 키를 넣는 방식도
문서화된 지원으로 확인하지 못했습니다. GetDefault와 SaveAs가 사용자 설정을
어디까지 이어받는지는 다음 실행 시험에서 확인해야 합니다.

같은 HWP로 자체 PDF 저장의 매우 높음과 Hancom PDF 인쇄의 매우 높음/600dpi를
비교하는 것이 다음 시험입니다. 양쪽 모두 문서 전체, 원래 용지/배율,
모아 찍기 없음, 그림·그리기 개체 포함, 추가 자동 머리말·꼬리말 없음으로
맞춘 뒤 검색/복사, 서체/ToUnicode, EMF 경로/이미지 구조, 압축/해상도,
렌더링과 크기를 비교합니다. 600dpi나 매우 높음만으로 검색 가능성과
벡터 보존을 보장하지 않습니다.

[SaveAs PDF 공식 답변](https://forum.developer.hancom.com/t/insertpicture-hwp-pdf/2247).


## 후속 실측: PrintToPDFEx의 GraphicQuality

같은 날 원격 상태에서 시험했습니다. 화면의 매우 높음 설정을 사용자가
직접 지정할 수 없어 해당 이름과 API 값의 대응은 확정하지 않았습니다.
먼저 0–4를 시험했지만 이 값들은 5단계 열거형이 아니었습니다.
JPEG 양자화 표에 실제로 매우 낮은 품질률로 반영됐습니다.
75와 100을 추가 시험했고, 100에서만 이번 표본의 모든 이미지 스트림이
FlateDecode로 출력됐습니다. 100을 화면의 매우 높음이라고 단정하지 않습니다.

문서를 열지 않은 초기 GetDefault에서는 GraphicQuality가 없었으나
PrintToPDFEx에는 GraphicQuality=0, Resolution=88이 있었습니다.
후자는 600dpi 같은 정확한 의미를 문서화된 값으로 확인하지 못했으며 변경하지 않았습니다.
일반 Print 액션의 Device 값 설명을 새 PrintToPDFEx의 기본 Device=5에
그대로 적용하지 않았습니다. 이 시험은 PrintToPDFEx의 직접 PDF 저장이며,
Device=0으로 Hancom PDF 프린터에 인쇄하는 별도 경로의 검증이 아닙니다.

### 직접 액션과 SaveAs의 차이

PrintToPDFEx.Execute에 FileName을 지정하면 실제 PDF 파일이 만들어졌습니다.
GraphicQuality를 바꿔 직접 출력한 PDF는 서로 다른 결과였습니다.
GetDefault의 GraphicQuality는 계속 0을 반환했으므로 그 반환값만으로
요청한 품질이 적용되지 않았다고 판정할 수 없습니다.

그 다음 같은 경로에 SaveAs(path, PDF, 빈 문자열)를 호출하면 다시 기본
품질로 저장됐습니다. 이 흐름의 0–4 출력은 페이지 내용/내장 이미지가
같았으며 모두 372,514바이트였습니다. 중간 액션의 품질을 SaveAs가
이어받는다는 앞선 가정은 이번 시험에서 성립하지 않았습니다.

### 품질 100의 결과

5쪽 시험 HWP에 PNG, JPEG, 일반 EMF, EMF+ Dual, EMF+ Only를 각각 넣었습니다.
직접 출력 PDF는 539,051바이트, 5쪽이며 모든 페이지를 Poppler로 렌더링해
그림과 본문 표시를 확인했습니다.

| 항목 | 실측 |
| --- | --- |
| PNG | HWP 안에서는 원본 1024×512 픽셀이 완전히 일치. PDF에서는 853×426으로 리샘플링 후 Flate 무손실 압축 |
| JPEG | InsertPicture/HWP 저장 단계에서 이미 재압축됨. PDF에서도 1024×512에서 853×426으로 리샘플링 후 Flate 압축. PDF 단계의 손실과 앞선 삽입 손실을 구분 |
| EMF 3종 | 3359×1890 래스터 이미지 및 마스크로 표현. 그림 안의 벡터 경로/텍스트를 PDF 벡터/텍스트로 유지하지 않음 |
| 본문 | 각 페이지의 한글/영문 시험 문장이 모두 정상 추출됨. 그림 내부 글자의 검색을 보장하는 결과는 아님 |

따라서 이번 자동화 값 100에서는 JPEG 압축 손실을 피하지만,
그림의 원래 픽셀 수와 EMF 벡터 구조까지 보존하지는 않습니다.
원본 화질이라는 말을 원본 데이터/픽셀/벡터의 동일성으로 해석하면
충족하지 않습니다. 화면상의 매우 높음 옵션과 정확히 같은 조건이라고
확정하는 검증은 남아 있습니다.

[시험 PDF](../artifacts/pdf-quality-2026-10-09/direct-api-100.pdf),
[전체 페이지](../artifacts/pdf-quality-2026-10-09/quality100-contact.png),
[직접 출력 분석](../artifacts/pdf-quality-2026-10-09/direct-analysis.json),
[SaveAs 재저장 분석](../artifacts/pdf-quality-2026-10-09/quality-analysis.json)에
출력, 이미지 필터/해상도/본문/픽셀 검사와 호출별 차이를 보존했습니다.
시험 HWP의 해시는 변환 전후 일치했습니다. 사용자 원고/템플릿을 열거나
수정하지 않았으며 보안 모듈 등록 및 프린터 설정을 변경하지 않았습니다.
시험 종료 시 한글 프로세스가 남지 않은 것을 확인했습니다.


## 후속 실측: HWP 저장 시 그림 용량 줄이기

사용자 요청에 따라 이 시험의 분석 범위는 HWP 내장 그림으로 제한했습니다.
PDF 품질이나 출력 경로에 대한 추가 분석은 하지 않았습니다.
한글 2020 11.0.0.9136에서 자체 생성한 PNG 및 JPEG를 각각
142×79.875mm로 삽입하고, 삽입 직후 HWPML 내장 데이터와 저장한
HWP의 BinData 스트림을 원본과 비교했습니다. 사용자 원고나 템플릿은
열거나 변경하지 않았습니다.

기존 PictureSaveAsOption 값은 ResizeImage=1, DelCutting=0,
SaveDpiX=600, SaveDpiY=600, SaveType=2였습니다.
비활성 시험에서는 ResizeImage=0, DelCutting=0, SaveType=0으로 지정했습니다.
SaveType=2를 특정 UI 체크 상태와 직접 대응시키지는 않았습니다.
또한 ResizeImage와 SaveType을 함께 끈 시험이므로 각 필드의 개별 효과를
분리한 결과는 아닙니다. 공식 [그림 용량 줄이기 도움말](https://help.hancom.com/hoffice110/ko-KR/Hwp/insert/figure/figure%28sizedown%29.htm)은
저장 시 모든 그림에 적용, 삽입 크기 및 해상도 기준 줄이기를 설명합니다.

| 원본 | 기존 그림 용량 설정으로 HWP 저장 | 그림 용량 줄이기를 끄고 HWP 저장 |
| --- | --- | --- |
| PNG 2560×1440, 50,316바이트 | 2560×1440 유지, 528,602바이트로 재인코딩. RGB 픽셀은 전부 일치 | 원본 파일 바이트와 SHA-256, RGB 픽셀 모두 일치 |
| JPEG 2560×1440, 품질 100·4:4:4, 1,236,338바이트 | 크기는 유지, 363,705바이트로 재압축. 3,686,400픽셀 중 1,868,563픽셀 변화, 채널 평균 절대 오차 1.72994 | 원본 파일 바이트와 SHA-256, RGB 픽셀 모두 일치 |
| JPEG 8192×4608, 품질 100·4:4:4, 9,059,213바이트 | 3354×1887, 517,219바이트로 축소·재압축. 삽입 폭 기준 약 600dpi | 8192×4608 및 원본 파일 바이트와 SHA-256, RGB 픽셀 모두 일치 |

삽입 직후에는 기존 설정과 비활성 설정 모두 세 원본의 바이트가 그대로
들어 있었습니다. 차이는 HWP 저장 후에 발생했습니다. 따라서 앞선 JPEG
시험에서 삽입 단계와 저장 단계를 구분하지 못했던 문제를 이번 표본에서는
저장 시 그림 처리로 좁힐 수 있습니다.

대조군으로 HWP SaveAs의 compress:true/false도 교차 시험했습니다.
FileHeader의 실제 압축 플래그까지 확인했으며, 같은 그림 설정의 두 HWP에서
추출한 그림 바이트는 모두 같았습니다. HWP 문서 컨테이너 압축을 끄는 것은
이번 그림 재압축이나 해상도 축소를 막지 못했습니다.

시험 후 PictureSaveAsOption을 원래 값으로 복원하고 IsEquivalent로 확인했습니다.
한글 프로세스도 남지 않았습니다. 변환기 코드나 사용자 설정의 영구 변경은
하지 않았습니다. 이 결과는 위 세 표본과 해당 한글 버전에 대한 결과이며,
모든 이미지 형식·한글 버전의 원본 보존을 보장하지 않습니다.

상세 해시, 픽셀 비교, 삽입 직후/저장 후 대조 및 컨테이너 압축 대조는
[그림 압축 분석 JSON](../artifacts/pdf-quality-2026-10-09/raster-compression/hwp-picture-analysis.json)에,
실행 옵션과 설정 복원 결과는 같은 폴더의 exports.json 및 settings-restored.json에
보존했습니다. 시험 스크립트와 자료는 ignored artifacts에 있으며 제품 구성 요소가 아닙니다.


## 후속 실측: HWP 그림 처리 비활성화 후 PDF 원본 화질

위 그림 용량 줄이기 비활성 HWP에는 세 원본의 바이트가 모두 보존됐습니다.
이를 PrintToPDFEx의 GraphicQuality=100으로 이미 출력한 PDF를 추가 분석했습니다.
UI의 매우 높음 옵션과 100의 대응은 여전히 확정하지 않았습니다.
이 분석을 위해 새 COM 세션이나 설정 변경, PDF 재생성을 하지 않았습니다.

| 원본 | PDF 내장 그림 | 원본 RGB 픽셀 보존 |
| --- | --- | --- |
| PNG 2560×1440 | 2560×1440, FlateDecode | 불일치. 3,686,400픽셀 중 2,583,995픽셀 변화, 채널 평균 절대 오차 3.08028, 최대 채널 오차 14 |
| JPEG 품질 100·4:4:4, 2560×1440 | 2560×1440, FlateDecode | 불일치. 3,686,400픽셀 중 2,999,676픽셀 변화, 채널 평균 절대 오차 3.99487, 최대 채널 오차 173 |
| JPEG 품질 100·4:4:4, 8192×4608 | 2795×1572, FlateDecode | 해상도 축소로 불일치 |

8192픽셀 폭의 그림은 삽입 폭 142mm 기준 약 500dpi에 해당하는 크기로 줄었습니다.
PDFの画像ストリームはFlate無損失圧縮でも、圧縮前の画像処理まで無損失とは
言えません.今回はPNGの画素も変わっており、その具体的な色変換・処理の
原因はまだ切り分けていません.以前の1024×512の別表本で853×426に
縮小された結果を、今回のすべての画像に一般化しません.

3ページすべてをPopplerでレンダリングして確認しました.本文の
「본문 화질 검증 ABC123」は全ページから抽出できました.
詳細は [PDF画像分析](../artifacts/pdf-quality-2026-10-09/raster-compression/pdf-original-quality-analysis.json)、
全ページの検討画像は同じフォルダーのpdf-reviewed-contact.pngに保存しました.


## RHWP 저장소 추가 조사

2026-10-09에 edwardkim/rhwp의 main 커밋
`1a76570e833917d15817415a53c09ad61ab3203f`를 기준으로 조사했습니다.
Python·PowerShell 도구 397개와 PDF/인쇄 관련 문서 22개, 총 419개를
읽어 GraphicQuality, PrintToPDFEx, FileSaveAsPdf 및 최고/원본 화질 표현을
검색했습니다. 이는 전체 저장소·이슈·외부 MCP 서버에 대한 전수 검색은 아닙니다.

- [tools/hwp_oracle_pdf.ps1](https://github.com/edwardkim/rhwp/blob/1a76570e833917d15817415a53c09ad61ab3203f/tools/hwp_oracle_pdf.ps1#L120-L127)은
  FileSaveAsPdf 액션에 FileName, Format=PDF, Attributes=0을 넣어 직접 실행합니다.
  그림 품질이나 DPI는 지정하지 않습니다.
- scripts/oracle_stage3_windows_canary.ps1 및
  scripts/oracle_stage4_windows_interactive.ps1도 같은 액션을 사용합니다.
- 위 검색 범위에서는 GraphicQuality의 값 정의나 UI 단계 대응, PDF 해상도
  파라미터 정의를 찾지 못했습니다. PrintToPDFEx로 외부 MCP에서 기준 PDF를
  만들었다는 기록은 있으나 그 기록에는 PrintMethod=0 외 품질 옵션이 없습니다.
- FileSaveAsPdf는 이번 프로젝트에서 아직 출력 화질을 비교하지 않은 후보입니다.
  이름만으로 SaveAs 또는 PrintToPDFEx와 같은 결과라고 단정하지 않습니다.

검색 대상·줄별 결과 및 원문 사본은 ignored
`artifacts/rhwp-pdf-review/search-results.json`과 `sources/`에 보존했습니다.
저장소 코드를 실행하거나 한글 COM을 구동하지 않았습니다.


## GetDefault 후 정수 Item 조회

한글 2020 11.0.0.9136에서 PrintToPDFEx.GetDefault가 채운 세트의 Count는
43이었습니다. Item에 정수 0부터 43까지 전달하면 호출 오류는 없지만
모두 null이 반환됐습니다. 이름으로 조회한 GraphicQuality는 존재했고 값은 0이었습니다.
따라서 이 시험에서 정수 호출은 순번 기반 항목 조회 기능을 제공하지 않았습니다.
문서의 ParameterSet.Item 인수는 BSTR itemid이며 ParameterArray.Item의 long index와
구분됩니다. 정수가 내부에서 정확히 어떻게 처리되는지까지 입증한 시험은 아닙니다.
복제 세트에 숫자 이름 1을 추가하는 대조는 항목이 실제 생성되지 않아 성립하지 않았고,
이를 문자열 변환의 증거로 사용하지 않았습니다.
문서 열기와 액션 Execute를 수행하지 않았고 사용자 설정을 변경하지 않았습니다.
결과는 ignored artifacts/pdf-quality-2026-10-09/integer-item-results.json에 보존했습니다.


## ParameterArray 정수 인덱스 대조

한글 2020에서 독립 TableCreation 세트에 RowHeight 배열을 3개 만들었습니다.
ParameterSet.IsSet은 true, ParameterArray.IsSet은 false였습니다.
배열의 SetItem 및 Item에 정수 0, 1, 2를 넣어 11111, 22222, 33333이
각각 정상 왕복됐습니다. 보관된 구형 HwpAutomation 매뉴얼의 1부터 시작한다는
설명과 달리 이번 배열은 0부터 시작했습니다.

음수/범위 밖 조회는 RPC_E_SERVERFAULT였고 인덱스 3의 쓰기도 예외였습니다.
특히 실패한 인덱스 3 쓰기 이후 Count가 4로 변하는 부수 효과가 있어,
이를 안전한 범위 검사나 성공한 배열 확장으로 간주하지 않았습니다.
정상 0,1,2 대조는 별도의 배열에서 먼저 수행했습니다.
문서 열기 및 액션 Execute는 수행하지 않았고 사용자 설정을 변경하지 않았습니다.
이 결과는 ParameterArray의 동작이며 PrintToPDFEx의 ParameterSet 항목 열거와는
구분됩니다. 실측은 ignored artifacts/pdf-quality-2026-10-09/parameter-array-results.json에
보존했습니다.


## 실제 ParameterSet의 열거 인터페이스와 형식 라이브러리

한글 2020 11.0.0.9136의 PrintToPDFEx.GetDefault 세트(Count=43)를 읽어 확인했습니다.
IEnumVARIANT에 대한 QueryInterface는 0x80004002(E_NOINTERFACE),
_NewEnum 속성 조회는 0x80020006(DISP_E_UNKNOWNNAME)였고,
.NET IEnumerable도 구현하지 않았습니다. 조회 전후 세트의 IsEquivalent가 true였습니다.

설치된 HwpAutomation.tlb를 LoadTypeLibEx(REGKIND_NONE)로 읽은
IDHwpParameterSet 인터페이스에도 _NewEnum이나 항목 이름 열거 메서드는 없습니다.
Item의 인수 형식은 VT_BSTR, IDHwpParameterArray.Item의 인수는 VT_I4입니다.
이 결과는 해당 버전의 공개 인터페이스 및 표준 COM 열거 여부이며,
미공개 내부 구현의 가능성을 전부 부정하는 결과는 아닙니다.

형식 라이브러리의 HPrint에는 GraphicQuality 속성이 실제로 있습니다.
DISPID는 16475, setter 인수 형식은 VT_UI2(부호 없는 16비트 정수)이며
설명은 property GraphicQuality입니다. 이름 후보를 추정하는 대신 설치된
형식 라이브러리의 속성 선언을 직접 읽어 찾을 수 있었습니다.
이 메타데이터에도 유효 값 범위나 UI의 매우 높음과의 대응은 없습니다.
형식 라이브러리에서 고정 인터페이스의 메서드/속성을 열거하는 것과
GetDefault 세트에 현재 들어 있는 동적 항목 43개를 열거하는 것은 구분합니다.

처음 작성한 수동 IDispatch 조사 래퍼는 PowerShell CLR에서 접근 위반으로
종료됐습니다. 정상 COM 종료가 불가능했던 해당 시험 소유의 숨겨진 빈 한글
PID 16460만 생성 시각·세션·창 상태로 확인해 정리했습니다. 이후 정적 형식
라이브러리 읽기 및 CLR 표준 속성 조회 방식의 시험은 정상 완료됐습니다.
한글 문서를 열거나 사용자 설정을 변경하지 않았습니다.

결과는 ignored artifacts/pdf-quality-2026-10-09/parameter-set-enumeration.json,
형식 정의는 같은 폴더의 parameter-set-typelib.json에 보존했습니다.


## PrintToPDFEx 기본 세트 43개 이름 확인

HPrint 형식 정의에는 HSet을 제외한 속성 이름 65개가 있습니다.
이 선언에서 얻은 이름들을 PrintToPDFEx.GetDefault 결과에 ItemExist로 대조했습니다.
빈 문서의 GetDefault Count=43에 대해 존재하는 이름도 정확히 43개였으며,
누락 없이 현재 기본 세트의 모든 이름과 값을 확인했습니다. 한글 2020의
이 상태에 대한 스냅샷이고 다른 문서/설정/버전에서 세트가 같다는 보장은 아닙니다.
현재 채워지지 않은 FileName, RangeCustom, ImageResampling 등도 HPrint 선언에는
있으므로 기본 세트 43개를 액션이 수용하는 모든 옵션의 목록으로 간주하지 않습니다.

이름과 값은 ignored artifacts/pdf-quality-2026-10-09/print-to-pdf-ex-defaults.json에
65개 후보의 존재 여부와 함께 저장했습니다. 값의 형식/이름을 확인한 것과
GraphicQuality, Resolution 등 미문서화된 항목의 값 범위·UI 의미를 이해한 것은
구분합니다. 문서를 열거나 액션을 실행하지 않았고 세트는 조회 전후 일치했습니다.

## 서로 다른 용지 크기의 두 쪽 PDF 출력 (2026-10-10)

한글 2020 11.0.0.9136에서 구역 1은 A4 세로(210 × 297mm),
구역 2는 A3 세로(297 × 420mm)인 두 쪽 시험용 HWP를 만들었습니다.
PageSetup의 공식 CreateItemSet("PageDef", "PageDef") 예제를 사용했습니다.
HWP 저장·재열기 후 PAGEDEF의 Width/Height가 각각 59528/84189,
84189/119055 HWPUNIT이고 실제 두 쪽임을 확인한 뒤 출력했습니다.
초기 Item("PageDef") 방식은 용지 변경이 실제 저장되지 않았으므로 그 출력은
혼합 용지 검증 근거에서 제외했습니다. Execute의 true만으로 설정 적용을
판정하지 않습니다.

| 경로 | 첫째 쪽 MediaBox | 둘째 쪽 MediaBox | 결과 |
| --- | --- | --- | --- |
| SaveAs(path, "PDF", "") | 595 × 841pt | 841 × 1190pt | 서로 다른 용지 크기 유지 |
| PrintToPDFEx | 595 × 841pt | 841 × 1190pt | 서로 다른 용지 크기 유지 |

CropBox도 각 MediaBox와 같습니다. 환산 크기는 첫째 쪽 약
209.903 × 296.686mm, 둘째 쪽 약 296.686 × 419.806mm입니다.
한글이 내보낸 정수 포인트 크기이므로 HWP의 정확한 mm 값과는 소폭 차이가 있습니다.
양쪽 결과에서 각 쪽의 한국어/영어 본문을 텍스트로 추출할 수 있고,
페이지 content stream의 SHA-256이 쪽별로 같습니다. 90dpi Poppler 렌더링도
각각 744 × 1052px, 1052 × 1488px이며 경로 간 픽셀이 같았습니다.
네 페이지를 모두 시각 확인했고 내용 잘림이나 겹침은 발견하지 않았습니다.
PDF 파일 전체 해시는 서로 다르므로 파일 바이트 일치를 주장하지 않습니다.

PrintToPDFEx는 기본 Device=5, PrinterName="Hancom PDF"를 유지하고,
PrintMethod=0, ZoomX/ZoomY=100, Range=6으로 실행했습니다.
GraphicQuality=0, Resolution=88도 기본값입니다.
이 시험은 PrintToPDFEx 경로에 대한 것으로, Device=0의 실제 프린터 드라이버
스풀 출력이나 임의 프린터의 혼합 용지 지원을 검증한 것은 아닙니다.
각 출력 전에 같은 HWP를 다시 열었고 출력 후 원본 HWP 해시는 변하지 않았습니다.
사용자 문서·템플릿은 사용하거나 변경하지 않았고 시험 소유 한글을 정상 종료했습니다.

재현 스크립트, 원본 HWP, 두 PDF, 재열기 XML, exports.json, analysis.json,
전체 페이지 PNG와 comparison.png는 ignored
artifacts/mixed-paper-pdf-2026-10-10/verified/에 보존했습니다.
## FileSaveAsPdf의 GetDefault 조회 (2026-10-10)

설치된 한글 2020 11.0.0.9136에서 CreateAction/CreateSet/GetDefault만
호출했습니다. 새 빈 문서와 저장된 A4/A3 두 쪽 시험용 HWP 모두에서
FileSaveAsPdf 및 FileSaveAs의 GetDefault 반환값은 1(성공),
ParameterSet.Count는 0이었습니다. 동일한 저장된 문서에서 양성 대조
PrintToPDFEx는 Count=43이었고 형식 라이브러리의 후보 이름으로 43개를
모두 확인했습니다. 조회 전후 모든 세트는 IsEquivalent=true입니다.

이 버전/시험 상태에서는 GetDefault로 PDF 저장 옵션 목록을 얻을 수 없습니다.
빈 세트는 액션이 옵션을 수용하지 않는다는 증거가 아닙니다.
FileName/Format/Attributes/Argument를 외부에서 공급하는 것과
GetDefault가 이를 미리 채우는 것은 별개입니다. SaveAs의 문자열 arg에 대한
PDF 품질 키도 이 시험으로 확인되지 않았습니다.

액션 Execute나 PDF/HWP 저장은 수행하지 않았습니다. 사용자 문서 대신
앞서 만든 시험 HWP를 사용했고 원본 해시는 유지됐으며 시험 소유 한글은
정상 종료했습니다. 실측 및 재현 스크립트는 ignored
artifacts/pdf-quality-2026-10-09/save-as-pdf-defaults.json,
save-as-pdf-opened-defaults.json, read-save-as-pdf-defaults.ps1,
read-save-as-pdf-opened-defaults.ps1에 보존했습니다.
## FileSaveAsPdf 실제 저장 (2026-10-10)

같은 A4/A3 시험 HWP를 열고 FileSaveAsPdf의 빈 기본 세트에
FileName(별도 출력 경로), Format="PDF", Attributes=0만 지정해
Action.Execute로 실행했습니다. 반환값 true, Execute 약 401.4ms,
PDF 21,550바이트가 생성됐습니다. Argument/GraphicQuality/Resolution은
지정하지 않았습니다. Attributes=0의 세부 비트 의미는 이 시험으로 확정하지 않습니다.
실행 후 새 세트를 만들어 GetDefault를 다시 조회해도 반환값 1, Count=0입니다.

결과 PDF는 두 쪽이며 MediaBox/CropBox가 각각 595×841pt,
841×1190pt로 A4/A3 구역 크기를 유지했습니다. 한국어/영어 본문이 추출되며
기존 SaveAs/PrintToPDFEx 결과와 각 페이지 content stream, 본문, 페이지 크기가
같았습니다. 90dpi 전체 페이지 렌더링도 두 경로와 픽셀 일치했고 모든 페이지를
시각 확인했습니다. 이 시험은 텍스트와 혼합 용지에 대한 것으로 그림 화질이나
설정 공유 메커니즘이 동일하다는 증거는 아닙니다.
원본 HWP 해시는 유지됐고 사용자 파일이나 설정을 변경하지 않았습니다.
산출물·스크립트·실행 및 분석 JSON은 ignored
artifacts/mixed-paper-pdf-2026-10-10/verified/의
mixed-a4-a3-filesaveaspdf.pdf, execute-filesaveaspdf.ps1,
filesaveaspdf-execution.json, filesaveaspdf-analysis.json에 보존했습니다.
## EMF 3종의 세 PDF 저장 경로 비교 (2026-10-10)

기존 quality-fixtures.hwp를 동일 원본으로 사용했습니다. PNG/JPEG 대조군
각 한 쪽, 일반 EMF/EMF+ Dual/EMF+ Only 각 한 쪽으로 총 5쪽이며
EMF 삽입 크기는 142×80mm입니다. 각 경로 실행 전에 원본을 다시 열었고
SaveAs, PrintToPDFEx, FileSaveAsPdf로 각각 별도 PDF를 만들었습니다.
사용자 원고·템플릿 및 원본 시험 HWP는 수정하지 않았습니다.

- SaveAs: format="PDF", arg="".
- PrintToPDFEx: FileName, Range=6, RangeIncludeLinkedDoc=0,
  PrintMethod=0, ZoomX/ZoomY=100, PrintImage/PrintDrawObj=1,
  PrintAutoHeadNote/PrintAutoFootNote=0, NumCopy=1, ReverseOrder=0.
  GraphicQuality=0, Resolution=88, Device=5, PrinterName="Hancom PDF"는
  GetDefault 값을 유지했습니다. 품질 100이나 UI의 매우 높음을 시험한 것이 아닙니다.
- FileSaveAsPdf: FileName, Format="PDF", Attributes=0.

세 PDF 모두 372,514바이트, 5쪽입니다. 모든 경로에서 각 EMF는
3359×1890px, 8비트 RGB, DCTDecode(JPEG)인 단일 이미지로 표시됩니다.
PDF상의 이미지 배치는 약 141.910×79.923mm로 환산 DPI는
가로 약 601.22, 세로 약 600.66입니다. 약 600dpi라는 표현은 이 배치에 대한
실측이며 Resolution=88의 의미를 확정한 것이 아닙니다.
그림 내부 글자·도형은 PDF의 검색 가능한 텍스트나 벡터로 유지되지 않았습니다.
일반 EMF 스트림은 156,190바이트, EMF+ Dual/Only는 127,807바이트이며
EMF+ 두 변형은 이번 표본에서 같은 JPEG 개체를 재사용합니다.

모든 경로 쌍에서 각 페이지 content stream, 추출 본문, 디코딩한 이미지
픽셀이 같습니다. 120dpi로 세 PDF의 전체 15페이지를 렌더링한 결과도
모든 쌍에서 변경 픽셀 0입니다. 전체 페이지와 3×3 EMF 상세 비교를
시각 확인했으며 경로 간 표시 차이, 잘림, 누락은 발견하지 않았습니다.
일반 EMF 반투명 영역의 점무늬는 세 경로에서 같습니다.
이전 GDI+ 비교와 별개인 경로 간 동일성 시험이므로 원본 벡터/픽셀의
보존이나 모든 EMF의 정확한 렌더링을 보장하지 않습니다.
본문 한국어/영어는 추출됩니다. PDF 파일 전체 해시는 서로 다르므로
파일 바이트 전체가 같다는 주장은 하지 않습니다.

PrintToPDFEx 시험은 Device=5의 직접 PDF 출력으로, Device=0 프린터
스풀 경로를 시험하지 않았습니다. 원본 HWP SHA-256을 출력 전후 대조해
일치를 확인했고 시험 소유 한글은 정상 종료했습니다.
ignored artifacts/emf-pdf-three-routes-2026-10-10/에 세 PDF,
export-three-routes.ps1, execution.json, analysis.json, 전체 페이지 PNG,
all-pages.png 및 emf-details.png를 보존했습니다.
### 위 PDF의 JPEG 양자화표에서 확인한 품질

각 DCTDecode 이미지의 JPEG를 읽고 밝기/색차 양자화표 전체를
Pillow/libjpeg 기본 quality=1..100의 표와 대조했습니다.
EMF 3종의 표는 quality=35 표와 오차 없이 일치했고,
PNG/JPEG 대조군을 변환한 PDF JPEG의 표는 quality=47과 일치했습니다.
모두 4:2:0 색상 서브샘플링, 비점진 JPEG입니다.

이는 JPEG 헤더에 보편적인 quality 필드가 저장돼 있다는 뜻이 아니라,
해당 인코더의 표준 품질값과 양자화표가 정확히 일치한다는 결과입니다.
한글 내부의 숫자 설정, UI 품질 단계 또는 화질 손실률을 확정하지 않습니다.
특히 GraphicQuality=0은 이 출력에서 JPEG 품질 0을 의미하지 않습니다.
한 파일의 양자화표를 조사했고 앞서 세 경로의 내장 JPEG 스트림과 픽셀이
동일함을 확인했습니다. 실측 원시 표, 후보별 오차 및 서브샘플링은
ignored artifacts/emf-pdf-three-routes-2026-10-10/jpeg-quality-analysis.json에
보존했습니다. 출력이나 한글 설정은 변경하지 않았습니다.
## 사전 고화질 JPEG의 기본 PDF 출력 (2026-10-10)

libjpeg-turbo 공식 usage.txt는 quality 상한을 100으로 명시하며,
100은 모든 양자화 항목을 1로 만들지만 서브샘플링과 반올림 손실까지 없애지는
않는다고 설명합니다. https://github.com/libjpeg-turbo/libjpeg-turbo/blob/main/doc/usage.txt

기존 reduction-off-compressed.hwp를 SaveAs(path,"PDF","")로 새로 출력했습니다.
HWP 내장 그림이 원본 바이트와 일치하는 기존 검증을 바탕으로 PDF 단계의
차이를 조사했습니다. 두 입력 JPEG 모두 양자화표가 libjpeg quality=100과
정확히 일치하고 4:4:4입니다. 삽입 크기는 142×79.875mm입니다.

- 2560×1440 JPEG: 기본 PDF에서도 2560×1440이나 DCTDecode JPEG,
  quality=35 표와 완전 일치, 4:2:0으로 재압축됩니다.
- 8192×4608 JPEG: 기본 PDF에서 2795×1572로 축소되고 같은 quality=35,
  4:2:0 JPEG가 됩니다. 약 500dpi입니다.
- PNG 대조군도 2560×1440, quality=35, 4:2:0 JPEG로 변환됩니다.

2560 JPEG의 원본 디코딩 픽셀 대비 평균 채널 절대 차이는 5.224/255,
변경 픽셀 3,073,911/3,686,400입니다. 이는 전처리 및 JPEG 재압축의
합산 결과이며 단순 quality 숫자를 손실률로 해석하지 않습니다.
앞선 작은 PNG/JPEG 대조군의 quality=47과 달라 기본/자동 출력 품질이
모든 입력에 고정 47이라고 일반화할 수 없습니다.

기존 PrintToPDFEx GraphicQuality=100 결과도 재분석했습니다.
두 JPEG는 JPEG 스트림으로 재압축되지 않고 FlateDecode RGB가 되므로
출력 JPEG 품질값을 붙일 수 없습니다. 2560 입력의 크기는 유지되지만
픽셀은 달라지고 평균 채널 절대 차이는 3.995/255입니다. 8192 입력은
이 경로에서도 2795×1572로 축소됩니다. 무손실 스트림 압축과 원본 픽셀
보존은 별개입니다. GraphicQuality=100의 UI 품질 대응은 여전히 미확정입니다.

새 PDF는 3쪽/352,162바이트이며 전체 페이지를 렌더링해 시각 확인했습니다.
원본 시험 HWP 해시는 유지됐고 사용자 문서·설정은 변경하지 않았습니다.
새 결과는 ignored artifacts/high-quality-jpeg-pdf-2026-10-10/의
saveas-default.pdf, export-default.ps1, execution.json, analysis.json,
전체 페이지 PNG와 all-pages.png에 보존했습니다.
## GraphicQuality 10개 값 × 세 저장 경로 (2026-10-10)

품질값 1,2,3,4,5,6,10,100,1000,10000을 기존 5쪽 EMF/PNG/JPEG 시험
HWP에 적용했습니다. 30개 조합 모두 저장 호출이 true였고 읽을 수 있는 PDF를
만들었습니다. 각 호출 전 원본을 다시 열었고 출력 전후 HWP 해시는 같습니다.

PrintToPDFEx의 GetDefault가 만든 GraphicQuality 항목은 런타임 System.Byte입니다.
SetItem에 Int32로 전달한 1~10/100은 그대로 읽혔으나 1000은 232,
10000은 16으로 읽혔습니다. 각각 256으로 나눈 나머지와 일치합니다.
이는 앞서 형식 라이브러리 HPrint의 setter에 나온 VT_UI2 선언과 구별되는
실제 동적 세트 저장 형식입니다. 큰 값이 상한 100으로 보정된 것으로 해석하지 않습니다.

| 전달값 | 세트에서 읽힌 값 | 실제 PDF 그림 표현 |
| --- | --- | --- |
| 1,2,3,4,5,6,10 | 각각 동일 | 모든 PNG/JPEG/EMF 그림의 양자화표가 해당 libjpeg 품질과 일치 |
| 100 | 100 | 모든 그림이 FlateDecode RGB 및 필요한 마스크로 표현됨. JPEG 품질값 없음 |
| 1000 | 232 | 기본값 0과 내용 및 페이지 픽셀 동일. PNG/JPEG는 품질47, EMF는 품질35 |
| 10000 | 16 | 모든 그림의 양자화표가 libjpeg 품질16과 일치 |

100의 Flate 압축은 스트림 압축에 관한 것으로 EMF 벡터 또는 원본 픽셀 보존을
뜻하지 않습니다. EMF는 모든 조합에서 3359×1890 래스터로 유지됩니다.

FileSaveAsPdf에는 FileName/Format="PDF"/Attributes=0과 함께
SetItem("GraphicQuality", Int32값)을 시도했습니다. GraphicQuality의 Item 조회는
null이었고 모든 값의 결과가 기본값 0과 구조·스트림·이미지 및 페이지 픽셀에서
동일했습니다. 실행 성공만으로 추가 항목이 수용된다고 판정하지 않습니다.
SaveAs는 PDF arg="GraphicQuality:<값>"으로 시도했고 역시 모든 결과가 기본값0과
동일했습니다. 해당 키/표기 방식에서 효과가 없다는 결과이며 다른 미공개 옵션의
가능성을 전부 부정하지 않습니다.

150쪽 전체를 90dpi Poppler로 렌더링했습니다. 기본 출력과 동일한 경우에는
페이지별 픽셀 일치도 확인했습니다. 세 경로 각각 50쪽 접촉시트와 모든 값의
EMF 상세 비교를 시각 확인했고 페이지·그림 누락은 발견하지 않았습니다.
낮은 품질값의 JPEG 압축 열화는 실제로 표시되며 이를 오류나 원본 손상으로
혼동하지 않습니다. 보조 마스크의 FlateDecode를 RGB 그림의 JPEG 여부와 구분했습니다.

그림 품질/프린터/보안 모듈 사용자 설정과 사용자 문서는 변경하지 않았습니다.
세트의 설정은 해당 출력 호출에만 공급했고 시험 소유 한글은 정상 종료했습니다.
Device=0 물리 프린터 경로와 UI의 매우 높음 대응은 이 시험에서도 검증하지 않았습니다.
ignored artifacts/pdf-quality-sweep-2026-10-10/README.md에 전체 30개 결과 표,
execution.json에 전달값/읽힘값/반환값, analysis.json에 양자화표·스트림·픽셀 비교,
export-sweep.ps1 및 analyze-sweep.py에 재현 코드를 보존했습니다.
PDF 30개와 PNG 150개, 경로별 전체 접촉시트 및 emf-quality-details.png도 같은 폴더입니다.