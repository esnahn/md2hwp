# Pandoc 출력 형식별 문서 제목 처리

조사일: 2026-10-02. 로컬 Pandoc 3.10.1 기준입니다.
YAML의 `title`·`subtitle`과 본문의 1단계 헤딩이 어떻게 출력되는지 확인했습니다.
아래 결과는 기본 템플릿과 기본 옵션에 대한 관찰이며 사용자 템플릿이나 참조 문서에 따라 달라질 수 있습니다.

## 결론

일반 변환에서는 문서 제목과 1단계 헤딩이 서로 대체되지 않습니다.
`title`과 `subtitle`은 Pandoc JSON의 `meta`에, `# 장 제목`은 `blocks`의 `Header`에 들어갑니다.
출력기는 이 서로 다른 입력을 제목 영역과 본문 구조에 배치합니다.
HTML처럼 같은 요소 이름을 사용하거나 일반 텍스트처럼 구조 구분을 잃는 형식도 있지만, 그것이 값의 경합을 뜻하지는 않습니다.

다만 출력 전 변환 옵션은 예외입니다. `--shift-heading-level-by=-1`은 원고 맨 앞의 헤딩1을 문서 제목으로 옮기며 기존 YAML `title`도 대체합니다.
따라서 “제목과 헤딩의 관계는 출력기에서만 결정된다”는 설명은 이 옵션까지 포함하면 정확하지 않습니다.
[공식 설명](https://pandoc.org/MANUAL.html#reader-options)

## 출력 형식별 기본 동작

표는 `--standalone`을 적용한 결과입니다. DOCX·ODT·PPTX·EPUB는 독립 문서로 출력했습니다.
“생략”은 이번 기본 출력에서 표시하거나 보존하지 않았다는 뜻이며, 해당 형식으로 부제를 표현할 수 없다는 뜻은 아닙니다.

| 형식 | YAML title | YAML subtitle | 본문 헤딩1과의 관계 |
| --- | --- | --- | --- |
| HTML4 / HTML5 | 페이지 `<title>` 및 `<h1 class="title">` | `<p class="subtitle">` | 본문은 별도 `<h1>`. 제목과 요소 수준은 같지만 노드와 클래스가 다름 |
| DOCX | `Title` 스타일 문단 | `Subtitle` 스타일 문단 | 헤딩1은 `Heading1` 스타일 문단 |
| ODT | `Title` 스타일의 일반 문단 | `Subtitle` 스타일의 일반 문단 | 헤딩1은 `text:h`, 개요 수준 1 |
| LaTeX | `\title` 및 `\maketitle` | 기본 템플릿의 `\subtitle` 처리 | 기본 article에서는 헤딩1이 `\section` |
| ConTeXt | 문서 정보와 별도 제목 영역 | 제목 영역의 별도 부제 | 본문은 section 구조 |
| Typst | 문서 정보와 별도 제목 텍스트 | 별도 부제 텍스트 | 본문은 `= ...` 헤딩 |
| EPUB3 | OPF 문서 제목 및 제목 페이지 | 제목 페이지의 부제 | 본문 헤딩1은 별도 본문 XHTML에 배치 |
| RST | 위·아래 `=` 선으로 둘러싼 문서 제목 | 위·아래 `-` 선으로 둘러싼 부제 | 헤딩1은 아래 `=` 선만 사용. 문서 제목 구문과 구분 |
| Markdown / GFM | YAML 메타데이터로 보존 | YAML 메타데이터로 보존 | 본문 `# ...`와 분리. 별도 표시용 제목 문단은 만들지 않음 |
| CommonMark | 생략 | 생략 | 본문 `# ...`만 유지 |
| Plain | 문서 앞 제목 텍스트 | 생략 | 헤딩도 일반 텍스트이므로 구조 구분은 사라짐 |
| RTF | 가운데 정렬한 제목 문단 | 생략 | 헤딩1은 별도 문단과 개요 수준 0 |
| AsciiDoc | 문서 제목 `= ...` | 생략 | 헤딩1은 `== ...` |
| DocBook5 | `info/title` | `info/subtitle` | 본문은 `section/title` |
| JATS | 앞부분 `article-title` | 같은 제목 그룹의 `subtitle` | 본문은 `sec/title` |
| TEI | 문서 헤더의 `titleStmt/title` | 생략 | 본문은 `div`의 `head` |
| Org | `#+title:` | 생략 | 본문 헤딩1은 `* ...` |
| Textile | 생략 | 생략 | 본문 헤딩1은 `h1. ...` |
| Beamer | 제목 프레임의 title | 제목 프레임의 subtitle | 본문 헤딩은 슬라이드 수준에 따라 프레임·절·블록으로 배치 |
| PPTX | 별도 메타데이터 슬라이드의 제목 | 그 슬라이드의 부제 | 본문 헤딩은 슬라이드 수준에 따라 구분 슬라이드·내용 슬라이드·내부 제목으로 배치 |
| reveal.js / Slidy | HTML 제목 및 제목 슬라이드 | 제목 슬라이드의 부제 | 본문 헤딩은 별도 슬라이드 구조 |
| JSON | `meta.title` | `meta.subtitle` | 본문 `Header`를 그대로 보존 |
| Native | `Meta`에 보존 | `Meta`에 보존 | 본문 `Header`와 분리. 비 standalone 출력에서는 메타데이터를 생략 |

DOCX는 생성한 ZIP의 `word/document.xml`에서 스타일 식별자를 확인했고,
ODT는 `content.xml`의 문단 종류·스타일·개요 수준을 확인했습니다.
PPTX와 EPUB도 내부 XML/XHTML을 열어 제목 영역과 본문 영역이 분리되는지 확인했습니다.

## 제목이 보이거나 사라지는 조건

텍스트 형식의 일반 출력은 본문 조각입니다. 이번 시험에서 JSON을 제외한 텍스트 형식은
`--standalone` 없이 출력하면 YAML 제목·부제가 나오지 않았습니다.
독립 문서 출력에서는 출력기와 기본 템플릿이 지원하는 정보만 배치됩니다.
따라서 YAML에 부제가 있다고 모든 기본 출력에서 부제가 보이는 것은 아닙니다.
[독립 문서 출력과 템플릿](https://pandoc.org/MANUAL.html#general-writer-options)

`--top-level-division=chapter`는 본문 헤딩1의 역할을 장으로 바꿉니다.
로컬 LaTeX 시험에서는 헤딩1이 `\chapter`, 헤딩2가 `\section`이 되었으며 문서 제목은 그대로 별도 처리됐습니다.
`-V documentclass=book`을 사용한 시험에서도 같은 본문 계층을 확인했습니다.
슬라이드 형식의 `--slide-level` 역시 본문 헤딩 배치를 결정하며 메타데이터 제목과 합치지 않습니다.
[슬라이드 구조](https://pandoc.org/MANUAL.html#structuring-the-slide-show)

PDF는 위 표의 단일 출력 규칙을 추가한 형식으로 취급하면 안 됩니다.
LaTeX·ConTeXt·HTML·Typst 등 선택한 중간 출력과 PDF 엔진의 규칙을 따릅니다.
이번 조사에서는 PDF 엔진 실행이나 GUI 렌더링 검증을 하지 않았습니다.

## md2hwp 설계에 적용할 판단

- `title`은 문서 전체 제목, `subtitle`은 문서 부제로 정의하고 헤딩1과 독립적으로 유지합니다.
- 헤딩1은 기존처럼 본문의 장 구조에 사용합니다. 첫 헤딩1을 제목으로 자동 승격하지 않습니다.
- 문서 제목은 `{{md2hwp:meta:title}}`, 부제는 `{{md2hwp:meta:subtitle}}` 자리에서만 치환하는 방향이 적합합니다.
- 제목의 서체·크기·배치는 HWP 템플릿이 결정합니다. YAML 제목의 존재만으로 Heading 1 스타일이나 제목 영역을 자동 생성할 필요는 없습니다.
- 같은 문구를 YAML title과 헤딩1 양쪽에 작성하면 서로 다른 위치에 두 번 나올 수 있습니다. 이는 입력과 템플릿 배치의 결과입니다.

이 부분은 조사 결과를 바탕으로 한 설계 제안이며 현재 구현된 기능은 아닙니다.
조사 당시의 IR 0.2는 비어 있지 않은 메타데이터를 거부했습니다. 이후 0.3 개발 구현에 별도 메타데이터 기능을 추가했습니다.
[YAML 메타데이터 문서](yaml-variables.md)에서 입력·IR·치환 규칙을 별도로 검토합니다.

## 조사 방법과 근거

서로 다른 식별 문자열을 YAML title·subtitle과 본문 헤딩1·헤딩2에 넣은 원고로
주요 26개 출력 형식에서 기본 변환 48회를 실행했습니다.
텍스트 형식은 본문 조각과 독립 문서를 비교했고 바이너리 형식은 압축 내부 문서 구조를 확인했습니다.
추가로 헤딩 이동·장 구분·슬라이드 수준 옵션을 시험했습니다.
시험 산출물은 ignored `artifacts/pandoc-title-writers/`에 있으며 배포나 빌드에 필요한 파일은 아닙니다.

공식 매뉴얼 외에 Pandoc 3.10.1 태그의 출력기 코드와 로컬 기본 템플릿을 확인했습니다.

- [DOCX 출력기](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/Docx/OpenXML.hs): 문서 제목·부제의 스타일과 본문 헤딩 스타일을 구분.
- [OpenDocument 출력기](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/OpenDocument.hs): 일반 제목 문단과 개요 헤딩을 구분.
- [LaTeX 출력기](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/LaTeX.hs): 문서 메타데이터와 본문 절 계층을 따로 처리.
- [PowerPoint 슬라이드 구성](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/Powerpoint/Presentation.hs): 메타데이터 슬라이드와 본문 헤딩에서 만드는 슬라이드를 구분.
- [RST 출력기](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/RST.hs): 제목·부제의 titleblock과 본문 헤딩을 따로 작성.
- [HTML 출력기](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/HTML.hs), [EPUB 출력기](https://github.com/jgm/pandoc/blob/3.10.1/src/Text/Pandoc/Writers/EPUB.hs): 문서 제목 컨텍스트·제목 페이지와 본문 헤딩 처리.

XML 구조 및 텍스트 출력의 확인 결과입니다. 화면에서의 간격·겹침·글꼴 등 시각적 품질을 검증한 결과는 아닙니다.
