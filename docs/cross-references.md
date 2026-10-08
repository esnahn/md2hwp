# 번호 상호참조

원고는 Pandoc `xrefs_number`와 같은 내부 링크 문법을 사용합니다.
`xrefs_number`는 ODT·OpenDocument 출력기의 기능이며, md2hwp는 같은 대상 연결
방식을 읽어 한글의 상호참조 필드로 출력합니다. Pandoc 출력 확장을 직접 실행하거나
`citations`·citeproc·pandoc-crossref를 사용하는 기능은 아닙니다.

## 원고

```markdown
# 장 제목 {#chapter-one}

## 절 제목 {#section-one}

![대상지 현황](site.png){#site}

출처: 작성자 작성.

[그림 참조](#site)를 보십시오. [개요 참조](#section-one)를 참고하십시오.
```

`attributes` 확장이 `{#ID}`를 해석합니다. `fig:`·`sec:` 같은 접두사는 필수가 아니며,
실제 대상 블록이 그림·표·헤딩 중 무엇인지로 참조 종류를 결정합니다. ID는 문서 전체에서
유일해야 합니다. 한글 ID도 허용하며 공백·제어문자·`#`를 포함할 수 없습니다.
ID 이외의 class·key-value 속성은 지원하지 않습니다.
내부 링크의 URI 이스케이프는 해제하여 ID와 비교합니다. 없는 대상·중복 ID는 오류입니다.
외부 URL 링크는 기존의 서식 있는 표기 텍스트 출력 방식을 유지합니다.
CommonMark 참조형 링크도 같은 내부 Link로 처리합니다.

```markdown
[그림 참조][site]

[site]: #site
```

`[@문헌]`에 인용·그림 참조의 의미를 부여하지 않으며, 별도 링크 정의가 없다면
일반 텍스트로 남습니다.

번호 참조에서는 링크 표시문을 템플릿 표기로 대체합니다. 표시문을 비운
`[](#site)`도 사용할 수 있습니다. 표시문의 문법과 자원 한도는
대체 전에 검증합니다. 참조 대상의 그림 캡션이나 제목은 삭제하지 않습니다.
조사는 자동으로 바꾸지 않으므로 출력 표기에 맞춰 원고에 적으십시오.
참조는 본문·제목·목록·표 셀·각주에 사용할 수 있습니다. 번호 참조의 링크 표시문 안에는
각주를 넣을 수 없으며, 각주는 링크 뒤에 붙이십시오. 그림 설명·그림 및 표 캡션과 개체 출처의
번호 참조도 지원합니다. 표 생성과 ID 지정은 [표 설명](tables.md)을 참고하십시오.

`implicit_figures` 확장으로 생성되는 Pandoc `Figure`와 독립 문단의 단일 `Image`를
같은 IR 그림으로 정규화합니다. `Figure`는 이미지 하나와 캡션 한 문단만 지원하며,
Figure 캡션과 이미지 대체 텍스트를 구분하여 보존합니다. 여러 이미지·하위 그림·짧은
캡션은 지원하지 않습니다. 독립 그림 바로 다음 `출처:` 문단은 기존 규칙으로 연결합니다.

## 템플릿

참조 양식은 `begin:template` 안에 독립된 begin/end 문단과 그 사이 **한 문단**으로 선언합니다. 참조는 이 문단의 문구를 원고 문장 안에 삽입합니다. 여러 문단·표·네이티브 제어문자·쪽 나눔은 허용하지 않습니다. 샘플 글자 서식은 사용하지 않으며 참조 위치의 서식과 강조를 유지합니다.

그림·표는 실제 한글 번호 상호참조 필드를 사용합니다. 다음 두 양식은 참조를 사용하지 않아도 필수입니다.

```text
{{md2hwp:begin:ref.figure.number}}
그림 {{md2hwp:num:heading1}}-{{md2hwp:slot:ref.figure.number}}
{{md2hwp:end:ref.figure.number}}

{{md2hwp:begin:ref.table.number}}
표 {{md2hwp:num:heading1}}-{{md2hwp:slot:ref.table.number}}
{{md2hwp:end:ref.table.number}}
```

`num:heading1`은 참조 대상 그림·표가 속한 장 번호를 생성 시 고정합니다. 실제 개체 번호는 한글 필드입니다. 표의 캡션과 Table 자동번호가 필요하며, 템플릿의 표 번호 재시작 규칙을 보존합니다. 하이퍼링크는 만들지 않습니다.

### 헤딩별 참조 블록

헤딩 참조는 한글 개요 상호참조가 아니라 **원고의 헤딩 순서로 계산한 고정 텍스트**입니다. 제목 문단에 개요 번호가 없어도 작동합니다. 한글 개요의 시작번호·재시작·구두점·표시 형식은 읽지 않습니다. 문서 생성 후 한글에서 제목 순서를 바꾸어도 자동 갱신되지 않으므로 원고를 수정하고 다시 변환하십시오.

`ref.heading1.number`부터 `ref.heading6.number`까지 여섯 양식이 모두 필요합니다. 기존 공통 `ref.heading.number`와 `slot:ref.heading.number`는 지원을 제거했습니다. 기존 양식을 삭제하고 아래 블록들을 추가하거나 `init-template`으로 새 템플릿을 생성하십시오. IR 버전은 0.4로 유지됩니다.

```text
{{md2hwp:begin:ref.heading1.number}}
제{{md2hwp:num:heading1}}장
{{md2hwp:end:ref.heading1.number}}

{{md2hwp:begin:ref.heading2.number}}
{{md2hwp:num:heading1}}.{{md2hwp:num:heading2}}절
{{md2hwp:end:ref.heading2.number}}

{{md2hwp:begin:ref.heading3.number}}
{{md2hwp:num:heading1}}.{{md2hwp:num:heading2}}.{{md2hwp:num:heading3}}항
{{md2hwp:end:ref.heading3.number}}
```

heading4~heading6도 같은 begin/end 구조로 작성합니다. 해당 양식에는 자신의 `num:headingN`이 적어도 하나 필요합니다. 상위 수준 번호를 함께 쓰거나 생략할 수 있고, 같은 번호를 여러 번 사용할 수 있습니다. 자신의 수준보다 깊은 번호와 다른 태그는 허용하지 않습니다. 예를 들어 heading2 양식을 `제{{md2hwp:num:heading1}}장 제{{md2hwp:num:heading2}}절`로 바꾸면 `제3장 제2절`이 됩니다. 마침표·괄호·장·절·항 등은 템플릿에 적은 문구만 출력합니다.

번호 규칙은 다음과 같습니다.

- heading1은 `md2hwp-heading1-start`(기본 1)부터 증가합니다.
- heading2~heading6은 각각 1부터 증가하며 상위 헤딩이 나오면 하위 번호를 0으로 초기화합니다.
- ID가 없는 헤딩도 셉니다. ID는 참조 대상을 찾는 용도로만 사용합니다.
- 생략된 상위 수준은 0입니다. heading3부터 시작하면 `0.0.1`이 됩니다.
- 다른 장과 뒤쪽 제목을 참조해도 대상 헤딩의 번호를 사용합니다.
- 제목 블록 안에서 제목이나 `each.child` 목록을 여러 번 복제해도 원고 헤딩은 한 번만 셉니다.

헤딩 참조도 본문·제목·목록·각주·표 셀·캡션·출처에 사용할 수 있습니다. [여섯 수준과 다른 장 참조 검증 원고](../examples/heading-references/heading-references.md)를 참고하십시오. 원고 제목 문단에 실제 보이는 개요 번호와 참조 번호는 서로 독립적이므로, 둘을 일치시키려면 템플릿의 제목 번호 설정도 원고 순서에 맞추십시오.

그 밖의 대상·쪽 번호 참조는 미지원입니다. 저장·재열기 구조 검증과 그림·표의 네이티브 필드 검증은 유지합니다.

## 표 ID

현재 사용하는 `commonmark+attributes+pipe_tables`에서 ID는 표 바로 앞의 독립된 줄에 둡니다. 표 제목 끝에 ID를 붙이면 표가 아닌 인라인 요소의 ID가 되므로 사용하지 않습니다.

```markdown
Table: 표 설명

{#tbl:example}
| 항목 | 값 |
| --- | --- |
| A | 1 |

[표 참조](#tbl:example)
```

표 ID도 접두어가 필수가 아니며 문서 전체에서 유일해야 합니다. 새 전용 문법을 추가하지 않습니다. [Pandoc attributes 설명](https://pandoc.org/MANUAL.html#extension-attributes)의 블록 앞 속성 문법을 사용합니다. [표 참조 검증 원고](../examples/table-references/table-references.md)는 앞뒤·다른 장·캡션·출처·각주·셀 참조를 포함합니다.

제목 블록·한 문단 제목 선언·`each.child`에서도 같은 `num:heading1`~`num:heading6` 번호를 사용합니다. 범위별 자신의 수준과 상위 수준만 지원합니다. [템플릿의 반복·번호 예시](templates.md)를 참고하십시오.
