# SVG 그림 검토 — 2026-10-09

한글 2020 11.0.0.9136에서 SVG 원본을 내장해 저장·재열기하고, 전체 페이지 이미지와 PDF·PCL을 확인했습니다. 독립 SVG 렌더러 resvg와 비교했습니다. **삽입 성공과 원본 내장은 확인했지만, 표시 누락과 특정 로컬 파일 참조의 한글 종료가 재현됐습니다. 현재 md2hwp의 SVG 미지원은 유지합니다.** 이 검토에서 변환기·IR·템플릿은 변경하지 않았습니다.

## 환경과 방법

- Windows, Hancom Office 2020 HWP 11.0.0.9136, 문서 한 개씩 숨김 COM 실행.
- `RegisterModule` 성공을 확인한 뒤 원본 SVG를 `InsertPicture`로 내장했습니다. 삽입 폭 142mm, 높이 79.875mm, SVG viewBox 800×450입니다.
- HWP 저장·재열기 후 HWPML의 그림 개수·SVG 형식·내장 데이터를 확인했습니다. 압축된 내장 SVG를 추출해 원본 SHA-256과 비교했습니다.
- 재열기한 HWP에서 `CreatePageImage`로 300/600dpi 전체 페이지를 만들었습니다. 비교 이미지는 이 페이지에서 그림 위치를 잘라 얻었습니다. SVG만 별도로 그린 이미지를 한글 페이지 이미지로 대체하지 않았습니다.
- 비교 렌더러는 [resvg 0.47.0 공식 Windows 배포](https://github.com/linebender/resvg/releases/tag/v0.47.0)입니다. 동일 시스템 서체를 사용하고 그림 폭·배경·출력 해상도를 맞췄습니다. LibreOffice는 사용하지 않았습니다. Edge를 이용한 비교는 완료되지 않았으며 결과 근거에 포함하지 않았습니다.
- PDF는 한글 자체 저장, PCL은 SINDOH D420/CM Series PCL 드라이버의 PrintToFile로 얻었습니다. 실제 프린터로 전송하지 않았고 프린터 포트 설정도 변경하지 않았습니다. PCL은 [Artifex의 PCL XL 분석 도구](https://github.com/ArtifexSoftware/ghostpdl/blob/master/pcl/tools/pxldis.py)로 명령·압축 방식을 분석했습니다.

실측 파일은 저장소의 ignored 경로 `artifacts/svg-2026-10-09/`에 있습니다. 아래 상대 링크는 해당 작업 폴더를 보유한 환경에서 열립니다. 원시 수치: [analysis.json](../artifacts/svg-2026-10-09/analysis.json). 충돌 로그: [hancom-crash-events.json](../artifacts/svg-2026-10-09/hancom-crash-events.json).

## 화면·저장 결과

| 시험 | 한글 재열기 후 화면 | resvg 비교 및 판정 |
| --- | --- | --- |
| 선·점선·도형·클리핑·회전·반투명 겹침 | 주요 요소 표시 | 글자·선 경계 등 픽셀 차이가 있어 완전 일치는 아님 |
| 선형/방사형 그라데이션·패턴·화살표·블러·로컬 use·마스크 | 주요 요소 표시 | 이 샘플에서는 대체로 같은 모양. 모든 조합의 지원을 보장하지 않음 |
| PNG data URI, xlink:href | 이미지 누락 | resvg에는 표시됨 |
| PNG data URI, SVG 2 href | 이미지 누락 | 최소 재현에서도 동일 |
| JPEG data URI, xlink:href | 이미지 표시 | SVG 내부 비트맵을 전부 지원하지 않는다는 결론은 아님 |
| 상대 경로 PNG | 이미지 누락 | resvg에는 표시됨 |
| 절대 file URI PNG | 삽입 중 한글 종료 | 복합 샘플과 단일 이미지 샘플에서 재현 |
| textPath 곡선 배치 글자 | 글자 누락 | resvg에는 표시됨 |
| 설치된 KoPub/KoPubWorld 서체 | 올바른 CSS family/weight로 표시 | 서체 이름과 weight를 구분해야 함 |
| 없는 서체 | 대체 서체로 표시 | 렌더러마다 대체 결과가 다름 |

독립 PNG 파일의 한글 직접 삽입은 성공했습니다. 따라서 PNG 누락은 이 환경의 SVG 내부 이미지 처리에서 관찰된 문제입니다. 단순 PNG 삽입 전체의 문제로 확대하지 않습니다.

기본·효과·서체 세 종류와 서체 이름 확인용 두 변형, 총 다섯 주요 샘플은 그림 한 개를 내장하고 저장·재열기했습니다. **이 다섯 샘플의 내장 SVG 바이트는 모두 원본과 일치했습니다.** 원본 보존이 표시 정확성을 보장하지는 않습니다. 원본 그대로 내장된 SVG에서도 PNG와 textPath가 누락됐습니다.

절대 파일 URI의 최소 재현 파일은 [image-absolute.svg](../artifacts/svg-2026-10-09/fixtures/image-absolute.svg)입니다. Windows Application 이벤트 1000에 18:42:01과 18:45:41 두 번의 장애가 기록됐습니다. 오류 모듈은 한글 SVG 필터의 `ImgFilters/SVG/gio-2-vs9.dll`, 예외는 `0xc0000417`입니다. COM 호출은 RPC 실패로 끝났습니다. 모든 파일 URI·한글 버전에서 같은 장애가 발생한다고 일반화하지 않습니다. resvg도 이 file URI를 해석하지 않았으므로 해당 사례의 화면 정답으로 사용하지 않았습니다.

## 픽셀 비교

600dpi 그림 영역은 3354×1887입니다. 한글 전체 페이지에서 물리적 위치로 자른 영역과 resvg 결과를 비교했으며, 내용에 맞춘 이동·확대·왜곡 정합은 하지 않았습니다. 300dpi에서 resvg 높이는 943픽셀, 한글 영역은 944픽셀이므로 공통 높이만 비교했습니다.

| 주요 샘플 | 600dpi 채널 평균 절대 차이 / 255 | 한 채널 이상 차이 >32인 픽셀 |
| --- | ---: | ---: |
| 기본 도형 | 5.770155 | 208,920 |
| 효과 | 10.097294 | 353,619 |
| 올바른 서체 family | 11.007652 | 319,522 |

숫자는 안티앨리어싱·글자 폭·배치 차이도 포함합니다. 실제 PNG와 곡선 글자 누락은 확대 비교로 별도 확인했습니다. 차이를 모두 안티앨리어싱으로 설명할 수 없습니다.

- 효과 샘플: [원본 SVG](../artifacts/svg-2026-10-09/fixtures/effects.svg), [내장 HWP](../artifacts/svg-2026-10-09/native-effects/picture.hwp), [한글 전체 페이지 600dpi](../artifacts/svg-2026-10-09/native-effects/native-page-600.png).
- [효과 비교: resvg / 한글 / 차이](../artifacts/svg-2026-10-09/native-effects/compare-600.png).
- [PNG 누락 확대 비교](../artifacts/svg-2026-10-09/native-basic/compare-bitmap.png).
- [서체 비교](../artifacts/svg-2026-10-09/native-fonts-family/compare-600.png).
- [SVG 내부 JPEG 표시 페이지](../artifacts/svg-2026-10-09/native-image-jpeg-data-xlink/native-page-300.png).

## 서체

CSS family 이름과 전체 face 이름은 다릅니다. 첫 시험에서 `KoPubWorldBatang_Pro Light` 등의 face 표기를 family로 사용하면 양쪽 렌더러 모두 의도한 서체를 고르지 못했습니다. 이는 한글만의 장애로 판정하지 않았습니다.

`KoPubWorldBatang_Pro`·`KoPubWorldDotum_Pro`에 `font-weight="300"`을 지정한 시험에서는 양쪽 모두 해당 Light 서체를 표시했습니다. 설치된 `KoPubBatang Light`, Malgun Gothic, Times New Roman도 확인했습니다. 같은 서체라도 글자 폭·배치의 차이는 남습니다. 없는 서체의 한글 대체 결과는 렌더러마다 달랐습니다. Malgun Gothic의 합성 이탤릭 처리도 달랐습니다.

시험 SVG에는 서체 파일을 내장하지 않았습니다. 따라서 다른 PC에서 HWP 안의 SVG를 다시 렌더링할 때 같은 서체가 설치돼 있어야 합니다. PDF·PCL의 글자는 이번 출력에서 이미지에 고정됐으므로 이후 보기에는 원본 SVG 서체가 필요하지 않습니다. 웹폰트·내장 폰트·네트워크 폰트는 시험하지 않았습니다.

## PDF

다섯 주요 샘플 모두 SVG 전체가 **3354×1887 JPEG 한 개**로 출력됐습니다. 그림 배치 크기 기준 약 600dpi이며 SVG의 벡터·텍스트 명령은 PDF에 남지 않았습니다. 삽입된 SVG 원본의 해상도가 600dpi라는 뜻이 아니라, 한글이 이 출력에서 그 크기로 래스터화한 결과입니다.

PDF 페이지를 렌더링해 기본·효과·올바른 서체 샘플을 시각적으로 확인했습니다. 한글 화면에서 빠진 PNG·textPath는 PDF에서도 빠졌습니다. 한글 페이지 이미지와 PDF에서 추출한 JPEG 사이에도 픽셀 차이가 있습니다. 평균 채널 절대 차이는 기본 1.408064, 효과 1.797532, 서체 0.951355 / 255입니다. PDF 저장이 누락을 복구하거나 무손실 픽셀 일치를 보장하지 않습니다.

예: [효과 샘플 PDF](../artifacts/svg-2026-10-09/native-effects/picture.pdf), [PDF 전체 페이지 렌더](../artifacts/svg-2026-10-09/native-effects/pdf-page.png).

## PCL

PCL XL 2.1, 600dpi입니다. 다섯 샘플 모두 SVG 전체를 래스터 밴드 일곱 개로 출력했습니다. 해당 단독 문서에는 `Text`, `PaintPath`, `LineRelPath`, 서체 다운로드 명령이 없습니다. 앞선 EMF 시험의 벡터·텍스트 명령 유지와 다른 결과입니다.

| 샘플 | PRN 바이트 | ReadImage | DeltaRow 압축 | JPEG 압축 |
| --- | ---: | ---: | ---: | ---: |
| 기본 도형 | 325,865 | 94 | 90 | 4 |
| 효과 | 417,286 | 65 | 60 | 5 |
| 초기 서체 이름 | 133,190 | 210 | 210 | 0 |
| 전체 face 이름 변형 | 133,190 | 210 | 210 | 0 |
| 올바른 CSS family | 130,916 | 195 | 195 | 0 |

ReadImage 수는 그림 개수와 다릅니다. 밴드 데이터를 여러 명령으로 전달하며 내용에 따라 압축 방식을 선택합니다. **SVG의 PCL 출력도 JPEG를 사용할 수 있습니다.** 서체 샘플에서 JPEG가 없었다는 사실을 모든 SVG에 확대할 수 없습니다.

이 검토는 인쇄 중간 파일의 구조를 확인한 것입니다. 실제 종이 출력이나 PCL 재렌더링의 픽셀 비교는 하지 않았습니다. [효과 PRN](../artifacts/svg-2026-10-09/native-effects/picture.prn), [명령 분석](../artifacts/svg-2026-10-09/native-effects/picture.disassembly.txt).

## 현재 프로그램과 판단

Markdown→IR에서는 SVG 경로가 기존 그림 경로로 전달됩니다. 실제 Release 백엔드는 COM 삽입 전에 `Figures support PNG, JPG, JPEG and EMF files only.`로 거부합니다. [실행 로그](../artifacts/svg-2026-10-09/backend-local-runtime.stderr.log). SVG 지원 구현, 재빌드, Rust·백엔드 계약 테스트는 이번 검토에 포함하지 않았습니다.

직접 SVG 지원을 확장하기에는 이 환경의 표시 누락과 종료 재현이 실질적인 장애입니다. 다음 검토를 진행한다면 독립 렌더러를 이용한 SVG→PNG 또는 SVG→EMF 변환을 별도로 평가해야 합니다. 그 경우 원본 SVG 직접 내장과는 다른 지원 방식이며, 서체·크기·변환 품질의 새 기준이 필요합니다. 이번에는 변환 기능을 추가하지 않았습니다.

샘플의 기본 도형·효과·이미지 참조·설치 및 누락 서체·PDF·PCL은 확인했습니다. 애니메이션·스크립트·네트워크 참조·foreignObject·전체 SVG 규격 준수는 확인하지 않았습니다. 다른 한글 버전·드라이버에도 이 결과가 동일하다고 주장하지 않습니다.

## 비교 도구 출처

resvg Windows ZIP: `https://github.com/linebender/resvg/releases/download/v0.47.0/resvg-win64.zip`.
SHA-256: `5684e59ceaa53ce720b49efb441b0918ae99d04e8ce3f6f753664524592d67f1`.
실행은 `resvg.exe --dpi 600 -w 3354 -h 1887 --background white input.svg output.png`이며, 별도 설치나 프로그램 의존성 추가는 하지 않았습니다. 도구는 [MIT 또는 Apache-2.0 라이선스](https://github.com/linebender/resvg)입니다. resvg 자체도 [미지원 기능](https://github.com/linebender/resvg/blob/main/docs/unsupported.md)이 있으므로 비교 결과를 전체 SVG 표준의 정답으로 취급하지 않습니다.
