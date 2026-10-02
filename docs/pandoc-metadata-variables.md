# Pandoc 메타데이터 변수 조사

조사일: 2026-10-02. Pandoc 3.10.1의 `commonmark+yaml_metadata_block → json`으로 자료형을 확인했습니다.
목적은 보고서의 표지·판권·머리말 등에 사용할 문서 변수 후보를 고르는 것입니다.
아래 후보 평가는 조사 당시의 제안입니다. 공개 배포된 IR 0.2에서는 지원하지 않으며, 이후 선택한 항목은 0.3 개발 구현에 반영했습니다.

조사 이후 정한 0.3 지원 범위는 title·subtitle·author·date·publisher와 md2hwp로 시작하는 임의 키입니다.
아래 표는 조사 당시의 후보 평가이며 확정 목록이 아닙니다. 최신 범위는 [YAML 메타데이터 문서](yaml-variables.md)을 따릅니다.

## meta에 들어가는 이름은 고정 목록이 아님

Pandoc의 `meta`는 이름과 값을 담는 맵입니다. 사용자 정의 키도 들어갑니다.
“Pandoc이 읽을 수 있음”, “특정 출력기가 의미를 알고 사용함”, “md2hwp가 지원함”은 각각 다른 조건입니다.
자료형은 `MetaInlines`, `MetaBlocks`, `MetaList`, `MetaMap`, `MetaBool`, `MetaString`으로 구분됩니다.
[공식 AST 정의](https://hackage-content.haskell.org/package/pandoc-types-1.23.1.1/docs/Text-Pandoc-Definition.html)

## 보고서에 쓸 만한 후보

다음 용도와 우선순위는 md2hwp를 위한 판단입니다.
“공통”은 Pandoc 매뉴얼의 일반 메타데이터 항목, “EPUB”은 EPUB 메타데이터에서 정의한 이름입니다.
어느 분류도 모든 출력기가 같은 형태로 표시한다는 뜻은 아닙니다.

| 이름 | 이름의 근거 | 보고서에서 사용할 위치 | 검토 의견 |
| --- | --- | --- | --- |
| `title` | 공통 | 표지, 속표지, 머리말 | 기본 채택 후보. 문서 전체 제목 |
| `subtitle` | 공통 | 표지, 속표지 | 기본 채택 후보. 문서 부제 |
| `author` | 공통 | 표지, 집필진, 판권 | 기본 채택 후보. 문자열과 저자 목록 지원을 구분 |
| `date` | 공통 | 발행일, 제출일 표기 | 기본 채택 후보. 표시 문자열을 유지하고 두 날짜가 필요하면 별도 키 |
| `lang` | 언어 변수 | 문서 언어 정보 | 기본 채택 후보. 예: `ko-KR`; HWP 서체 변경과는 별개 |
| `publisher` | EPUB | 발행기관, 판권 | 보고서용으로 유용. 저자의 소속기관과 구별 |
| `rights` | EPUB | 저작권·이용조건 | 판권에 유용. 임의의 라이선스로 자동 해석하지 않고 입력 문구 사용 |
| `identifier` | EPUB | ISBN, DOI 등 식별번호 | 유용하지만 번호 종류·복수 값의 표현을 먼저 정해야 함 |
| `keywords` | 공통 | 초록 아래 주제어 | 목록 지원을 정하면 유용. 구분자는 템플릿 또는 명시적인 치환 규칙에서 결정 |
| `abstract` | 공통 | 국문초록, 요약 | 유용하지만 여러 문단을 지원하려면 인라인 변수 치환보다 범위가 커짐 |
| `abstract-title` | 공통 | 초록의 제목 | 초록 지원 시 함께 검토. 예: 국문초록, 요약 |
| `subject` | 공통 | 주제·분야 표기 | 필요 시 채택. keywords와 별개 |
| `description` | 공통 | 짧은 소개문, 설명 | 필요 시 채택. 초록과 동시에 쓸 경우 용도를 명확히 정의 |
| `category` | 공통 | 기본연구·정책연구 등의 분류 | 양식에 분류가 있다면 유용 |

이름의 의미는 [일반 메타데이터 변수](https://pandoc.org/MANUAL.html#metadata-variables),
[언어 변수](https://pandoc.org/MANUAL.html#language-variables),
[EPUB 메타데이터](https://pandoc.org/MANUAL.html#epub-metadata)를 확인했습니다.
HWP에서 위 값을 표시하거나 문서 속성에 기록하는 동작은 별도로 구현해야 합니다.

## 사용자 정의가 필요한 보고서 정보

다음은 보고서에 유용하지만 위 공통 항목과 EPUB 항목만으로 이름과 의미가 정해지지 않습니다.
Pandoc이 임의 키를 받아 준다는 이유만으로 공식 공통 변수라고 소개하면 안 됩니다.

| 정보 | 용도 | 이름을 정할 때의 고려사항 |
| --- | --- | --- |
| 연구보고서 번호 | 표지·판권의 `기본 2026-01` 등 | ISBN과 분리. 숫자로 변환하지 않고 표시 문자열 유지 |
| 연구기관·저자 소속 | 집필진 소개 | publisher는 발행기관이므로 같은 기관이라고 가정하지 않음 |
| 연구책임자·공동연구자·편집자 | 역할별 이름 표시 | author를 한 줄로 합치는 방식만으로는 역할 구분 불가 |
| 발행인·발행처 연락처 | 판권 | 저자와 구별하고 주소·전화번호를 문자열로 유지 |
| 영문 제목·영문 초록 | 이중언어 표지·초록 | subtitle을 영문 제목의 별칭으로 사용하지 않음 |
| 과제명·과제번호·지원기관 | 사업 정보 | 보고서 번호 및 식별번호와 구별 |
| 판·쇄·문서 개정번호 | 판권·개정 이력 | IR 버전·프로그램 버전과 구별 |
| 제출일·발행일 | 서로 다른 날짜 | date 하나의 의미를 문서마다 바꾸지 않음 |

위 정보는 먼저 실제 양식에서 필요한 항목을 고른 뒤 키 이름을 정하는 편이 좋습니다.
저자 소속은 `author`를 이름·소속의 객체 목록으로 확장하는 방법도 있으나,
현재 초안의 “문자열 또는 문자열 목록” 계약과 템플릿 반복 규칙을 변경해야 합니다.
조사 이후 사용자 정의 문서 정보는 최상위 `md2hwp-<이름>` 문자열 키와 `{{md2hwp:meta:<키>}}` 태그를 사용하기로 정했습니다. 개별 키 이름은 사용자가 정합니다.

## 별도 기능으로 검토할 항목

| 항목 | 판단 |
| --- | --- |
| `bibliography`, `references`, `csl`, `nocite`, `reference-section-title` | 참고문헌·인용 처리 기능. 단순히 태그에 값을 넣는 것으로 완성되지 않음 |
| `toc`, `toc-title`, `lof`, `lot` | 목차·그림목차·표목차 생성 기능. 현재 each.child 절 목록과 구별 |
| `title-meta`, `author-meta`, `date-meta`, `pagetitle` | 화면 제목과 파일 속성용 값을 분리할 때 검토. 초기 HWP 변수에 반드시 넣을 필요는 없음 |
| `mainfont`, `fontsize`, `geometry`, `documentclass`, 여백 변수 | 출력 형식별 서식 설정. 현재 md2hwp의 템플릿 서식 소유 원칙에 따라 초기 YAML 기능에서 제외하는 제안 |
| `header-includes`, `include-before`, `include-after` | 출력 코드·본문 주입. HWP 머리말·꼬리말의 일반 문서 변수와 동일하게 취급하지 않음 |
| `dir` | 좌우 쓰기 방향. 한국어 보고서 초기 범위에서는 우선순위 낮음 |
| `cover-image` | 이미지 자원과 표지 배치 기능이 필요하므로 텍스트 변수와 별도로 검토 |

인용은 [Pandoc 인용 처리](https://pandoc.org/MANUAL.html#citations)의 별도 처리 과정을 따릅니다.
어떤 키를 meta에서 읽었다는 사실만으로 HWP에서 해당 기능을 지원하게 되는 것은 아닙니다.

## 실제 JSON 자료형 확인

로컬 Pandoc에 표의 후보와 사용자 정의 키를 넣어 다음 결과를 확인했습니다.
한 문단으로 읽힌 문자열은 `MetaInlines`이며, 이름만으로 자료형이 고정되지는 않습니다.

| 입력 예 | 관찰한 AST |
| --- | --- |
| title, subtitle, date, lang, publisher, rights 등 한 문단 문자열 | `MetaInlines` |
| `author: [홍길동, 김연구]` | `MetaList` 안의 `MetaInlines` 두 개 |
| `keywords: [도시, 주거, 공간정책]` | `MetaList` 안의 `MetaInlines` 세 개 |
| abstract에 빈 줄로 구분된 두 문단 | `MetaBlocks` 안의 `Para` 두 개 |
| `identifier: {scheme: ISBN-13, text: "urn:isbn:..."}` | `MetaMap`; 두 값은 `MetaInlines` |
| 사용자 정의 `author-detail: {name: 홍길동, affiliation: 연구기관}` | `MetaMap`; 두 값은 `MetaInlines` |
| `md2hwp: {chapter-start: 3, enabled: true}` | `MetaMap`; 숫자 3은 `MetaInlines`, true는 `MetaBool` |
| 사용자 정의 `report-number: "2026-01"` | `MetaInlines` |
| 본문 `# 첫 번째 장` | meta와 분리된 `Header 1` |

따라서 chapter-start는 AST에서 정수형으로 바로 꺼내는 방식이 아니라,
허용한 인라인 텍스트를 정수로 파싱하고 범위를 검증해야 합니다.
문자열 안에 Markdown 서식이 있으면 `Str` 하나만 온다고 가정해서도 안 됩니다.
날짜·식별번호는 YAML 문자열로 명시하고, 지원할 인라인 서식 범위를 IR 설계에서 정해야 합니다.

## 초기 범위에 대한 제안

1. 기존 초안의 title·subtitle·author·date·lang을 유지합니다.
2. 표지·판권에 바로 필요한 publisher·rights를 우선 추가 후보로 검토합니다.
3. category·keywords·identifier는 실제 보고서 양식의 필요와 값 표현을 확인해서 채택합니다.
4. abstract는 여러 문단을 넣을 수 있는 별도 설계 단계로 다룹니다.
5. 연구보고서 번호·영문 제목·역할별 연구진은 사용자 정의 후보 중 먼저 검토합니다.

태그는 이후 합의한 `{{md2hwp:meta:<키>}}` 틀을 사용할 수 있지만,
목록·객체·여러 문단을 어느 자리에서 어떻게 출력할지는 추가 규칙이 필요합니다.
예를 들어 identifier 객체를 단일 문자열처럼 치환해서는 ISBN 종류와 번호를 안정적으로 구분할 수 없습니다.

[YAML 메타데이터 문서](yaml-variables.md)과 [출력기별 제목 처리 조사](pandoc-title-writers.md)를 함께 참고하십시오.
이번 변경은 조사 문서 추가이며 코드·IR 스키마·템플릿은 변경하지 않았습니다.
