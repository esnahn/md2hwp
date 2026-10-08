# YAML 메타데이터와 템플릿 태그

v0.4.0 / IR 0.4에서 지원합니다. 이전 IR은 원고에서 다시 생성해야 합니다.
변수 입력 위치는 Markdown 원고 맨 앞의 YAML 블록으로 정했습니다.
지원 키는 title·subtitle·author·date·publisher와 비어 있지 않은 최상위 `md2hwp-<이름>` 키입니다.
템플릿 태그는 `{{md2hwp:meta:<키>}}`로 정했습니다. 아래 치환 규칙을 사용합니다.
추가 후보와 자료형은 [Pandoc 메타데이터 변수 조사](pandoc-metadata-variables.md)에 정리했습니다.

## 이름과 태그

YAML 자체는 문서 제목·저자 등의 키 이름을 정하지 않습니다.
문서 정보는 [Pandoc 메타데이터 변수](https://pandoc.org/MANUAL.html#metadata-variables)의 이름을 사용합니다.
`title`, `subtitle`, `author`, `date`, `publisher`를 사용하고 별칭을 만들지 않습니다.
`publisher`는 Pandoc EPUB 메타데이터에서도 사용하는 발행기관 이름입니다.
`lang`, `rights`, `identifier`, `abstract`, `keywords` 등의 키는 이번 지원 범위에 포함하지 않습니다.

| YAML 경로 | 값 | 템플릿 태그 | 의미 |
| --- | --- | --- | --- |
| `title` | 한 줄 문자열 | `{{md2hwp:meta:title}}` | 문서 전체 제목 |
| `subtitle` | 한 줄 문자열 | `{{md2hwp:meta:subtitle}}` | 문서 부제 |
| `author` | 한 줄 문자열 또는 문자열 목록 | `{{md2hwp:meta:author}}` | 저자. 목록이면 입력 순서대로 쉼표와 공백으로 연결 |
| `date` | 한 줄 문자열 | `{{md2hwp:meta:date}}` | 발행일 표기. 입력 문구를 유지 |
| `date-meta` | 입력 키가 아닌 파생 ISO 날짜 | `{{md2hwp:meta:date-meta}}` | Rust core에서 date를 인식하면 생성 |
| `publisher` | 한 줄 문자열 | `{{md2hwp:meta:publisher}}` | 발행기관. 저자의 소속과 구별 |

## 최상위 md2hwp- 임의 키

고정된 문서 정보 외의 값도 사용자가 입력하여 템플릿에서 치환할 수 있도록 합니다.
임의 키는 YAML 최상위에 `md2hwp-<이름>`으로 작성하고 값은 한 줄 문자열로 받습니다.
`md2hwp` 아래에 중첩된 객체를 만드는 방식은 사용하지 않습니다.
템플릿에서는 전체 키 이름을 그대로 사용합니다.

```yaml
md2hwp-report-number: "기본 2026-01"
```

```text
{{md2hwp:meta:md2hwp-report-number}}
```

임의 키가 존재한다는 이유만으로 프로그램의 동작 설정으로 해석하지 않습니다.
단, `md2hwp-heading1-start`는 아래의 장 번호 설정으로 예약되어 있습니다.

## 원고 예시

```markdown
---
title: 도시 공간 연구
subtitle: 보고서 부제
author:
  - 홍길동
  - 김연구
date: "2026-10-02"
publisher: 건축공간연구원
md2hwp-report-number: "기본 2026-01"
---

# 첫 번째 장 제목

본문입니다.

# 두 번째 장 제목

다음 장의 본문입니다.
```

문서 제목 `title`과 본문의 장 제목 `# ...`는 각각의 용도로 사용합니다.
출력 형식별 차이와 예외 옵션은 [Pandoc 제목 처리 조사](pandoc-title-writers.md)를 참고합니다.
날짜는 문자열로 입력하고 date에는 원문을 보존합니다.
Rust core에서 인식한 날짜는 date-meta에 정규화해 저장하며 현재 날짜로 채우지 않습니다.
날짜에 YAML의 따옴표를 사용하면 입력 의도가 분명합니다.

## 날짜 정규화와 표시 서식

Pandoc의 날짜 입력 형식 전체와 한국어·점·슬래시 구분자 형식을 지원합니다.
템플릿에서는 `{{md2hwp:meta:date-meta:fmt:%Y년 %-m월 %-d일}}`로 표시 형식을 지정합니다.
입력 형식, 부분 날짜 보완과 인식 실패 규칙은 [date-meta와 fmt 계약](date-metadata.md)을 따릅니다.
`date-meta`는 파생 값이므로 YAML에서 직접 지정할 수 없습니다.

## 장 번호와의 관계

문서 제목 치환과 장 번호 추적은 별개입니다. 시작 번호는 md2hwp-heading1-start로 지정하고,
현재 장 번호는 num:heading1로 표시합니다. 아래 장 시작 번호 절을 참고하십시오.

## 치환 규칙

- 문서 변수는 표지, 일반 문단, 표 셀, 글상자, 머리말·꼬리말과 반복 원형에서 사용할 수 있습니다.
- 태그는 문단 안에서 주변 문구와 함께 사용할 수 있으며, 같은 변수를 여러 곳에서 사용할 수 있습니다.
- 값은 태그 자리의 글자 서식과 문단·개체 구조를 따릅니다. 변수는 템플릿의 서식 설정을 덮어쓰지 않습니다.
- 한 태그가 여러 글자 서식 구간에 나뉘어 있어도 같은 문단 안에서 읽습니다. 치환값은 태그 첫 글자의 서식을 따릅니다.
- 변수 값은 일반 텍스트로 삽입합니다. 태그처럼 보이는 값도 다시 해석하지 않으며 자동 하이퍼링크를 만들지 않습니다.
- 한 줄 문자열과 저자 문자열 목록을 지원합니다. 값에 여러 문단이나 개체가 있으면 위치를 명시해 오류를 냅니다.
- `author`의 반복 배치는 목록 연결과 별도의 확장입니다. 기존 `each.child`의 의미를 바꾸지 않습니다.
- 템플릿이 참조한 문서 변수가 없거나 비어 있으면 변수 이름과 위치를 포함한 오류를 냅니다. 문단 전체를 자동 삭제하지 않습니다.
- 템플릿이 참조하지 않은 문서 정보는 생략할 수 있습니다. 변수 태그 자체는 템플릿의 필수 스타일 선언으로 요구하지 않습니다.
- 고정 키 5개와 최상위 `md2hwp-<이름>` 임의 키 외의 YAML 키는 거부합니다.
- 지원하지 않는 자료형과 존재하지 않는 변수를 참조하는 태그는 명시적으로 거부합니다.

## 입력과 IR의 흐름

```text
Markdown 앞의 YAML
    → Pandoc JSON의 meta
    → Rust core에서 이름·자료형·문자열 검증
    → IR 0.4의 metadata
    → C# 백엔드에서 템플릿 변수 치환
```

Pandoc 호출은 Rust 앱에 두고, Pandoc AST 해석과 IR 검증은 core에서 처리합니다.
백엔드는 YAML을 다시 읽지 않고 검증된 IR과 HWP 템플릿을 사용합니다.
IR에는 고정 문서 정보와 지원하는 임의 키의 값, 인식한 날짜의 date-meta를 저장합니다.
문자열은 기존 IR의 Unicode NFC 규칙을 따릅니다.

[원고 YAML 블록](https://pandoc.org/MANUAL.html#extension-yaml_metadata_block)은
`commonmark+yaml_metadata_block`으로 읽을 수 있음을 로컬 Pandoc 3.10.1에서 확인했습니다.
0.4 앱은 `commonmark+yaml_metadata_block+footnotes+attributes+implicit_figures+pipe_tables`를 사용합니다. 이전 IR 0.2는 메타데이터를 지원하지 않습니다.

## 버전 전환

IR과 프로그램 버전은 각각 0.4, 0.4.0입니다.
이전 원고의 IR을 다시 생성하고, 템플릿의 IR 버전도 0.4와 일치시켜야 합니다.
변수 태그는 필요한 자리에서만 사용하며 필수 원형 선언을 대체하지 않습니다.

## 장 시작 번호

```yaml
---
md2hwp-heading1-start: 3
---
```

첫 `heading1`을 3으로 시작하고 다음 `heading1`마다 4, 5로 증가합니다.
생략하면 1입니다. 1~2147483647의 양의 십진 정수만 받으며 전체 장 번호가 범위를 넘으면 오류입니다.
Pandoc JSON에서 전달된 값을 Rust가 검증하여 IR metadata에 문자열로 보존합니다.
`meta:md2hwp-heading1-start`는 시작 값 자체이며 현재 장 번호를 추적하지 않습니다.

현재 장 번호는 헤딩 블록이나 그림·표 캡션 안의 인라인 태그
`{{md2hwp:num:heading1}}`로 표시합니다. 첫 태그 글자의 서식을 유지합니다.

```text
{{md2hwp:begin:heading1}}
제{{md2hwp:num:heading1}}장
{{md2hwp:slot:heading1}}
{{md2hwp:end:heading1}}
```

그림 캡션에는 `[그림 {{md2hwp:num:heading1}}-<그림 자동번호>]`처럼 배치합니다.
`<그림 자동번호>`는 설명용 표기이며 실제 한글 자동번호 제어를 사용하십시오.
장마다 첫 그림에 한글의 `NEWNUM` 제어를 넣어 그림 번호를 1로 재시작합니다.
첫 heading1 앞의 그림에 장 번호 슬롯을 쓰면 오류입니다. 고정 표지 등 지원 범위 밖의 슬롯도 오류입니다.
기존 템플릿의 `#`는 자동 치환하지 않습니다. 이 설정은 md2hwp 전용이며 Pandoc 출력 옵션이 아닙니다.
