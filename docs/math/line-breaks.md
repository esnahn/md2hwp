# 080번: 일반 그룹 안의 줄바꿈

입력의 마지막 행은 `a+b+c+{d+\\e}+f+g`이며 KaTeX 설정은
`displayMode: false`다. 이전 HML 변환은 `{d+\\e}`를 `pile {d+ # e}`로
만들었지만 KaTeX HTML은 d와 e를 가로로 표시한다. 현재 변환은 일반 수학 그룹
내부의 단순 줄바꿈을 가로로 처리한다.

## KaTeX가 한 줄로 표시하는 이유

로컬 KaTeX 0.19.1 `dist/katex.js`의 줄바꿈 함수는 `\\`를 `cr` 노드로
읽고 `katex-newline` 요소를 만든다. 다만 전체 수식의 HTML 빌더는 최상위
표현식에서 발견한 줄바꿈만 `.katex-html`의 직계 자식으로 올린다.
일반 중괄호 그룹 내부의 줄바꿈은 `.mord` 안에 남는다.

같은 배포본 `dist/katex.css`는 `.katex-html > .katex-newline`에만
`display: block`을 적용한다. 따라서 그룹 안의 줄바꿈은 파싱되었어도
HTML에서 별도 행을 만들지 않는다. MathML에는 `linebreak="newline"`이
남으므로 HTML의 모습만으로 원문 줄바꿈이 삭제되었다고 설명하면 부정확하다.

그룹 유무에 따른 원본 HTML과 경고는
`artifacts/katex-math-review/080-newline-probe.json`에 기록했다.
전체 080 비교 이미지는 [비교 HTML](katex.html)에서 확인한다.

[KaTeX 옵션 문서](https://katex.org/docs/options)의 `newLineInDisplayMode`는
display 모드에서 배열 밖 줄바꿈을 제한하는 항목이다. 이 예제는 inline 모드여서
그 경고가 원인이 아니다. 표시 설정과 그룹 위치를 구별해야 한다.

## LaTeX와의 관계

[LaTeX 커널 소스](https://latex-project.org/help/documentation/source2e.pdf)의
`ltspace.dtx`는 일반 `\\`를 `\@normalcr`·`\@xnewline`·`\@gnewline`으로
처리한다. 배열은 별도의 `\@arraycr`를 사용한다. 일반 중괄호는 범위를 만들며
그 자체로 배열이나 행 정렬 환경을 만들지 않는다. 따라서 그룹 안 줄바꿈을
자동으로 두 행짜리 수학 개체로 만드는 현재 변환은 LaTeX의 구조에서 도출되지 않는다.
이 조사에서 실제 TeX 엔진을 실행하지 않았으므로 특정 오류 메시지가 발생한다고
단정하지 않는다.

## 반영한 변환 규칙

일반 수학 그룹 안의 단순 `\\`와 `\newline`은 행을 만들지 않는다.
직접 지원하는 공백 명령이 바로 앞에 있으면 `\unskip`에 따라 그 공백을 제거한다.
`\kern`은 glue가 아니므로 지우지 않는다. 분수의 스타일 미리 읽기도 이 줄바꿈에서
끊지 않는다. 명시적인 배열·정렬 환경의 행 구분과 바깥 수식의 줄바꿈은 유지한다.
그룹 안의 별표·선택 길이는 아직 거부하며 임의로 생략하지 않는다.

## 실제 TeX 매크로와 엔진에서의 처리

추가 조사 결과, 일반 LaTeX의 `\\`는 내용이 없는 명령이 아니다.
`ltspace.dtx`의 `\@normalcr`에서 `\@xnewline`을 거쳐 `\@gnewline`으로
이어지고, 기본 경로는 다음 노드를 만든다.

```tex
\unskip ... \nobreak \hfil \break
```

즉 앞의 glue를 제거하고, 줄바꿈 penalty와 자연 폭 0의 늘어나는 glue를 넣는다.
수학 모드라는 이유로 매크로 전체를 즉시 버리는 분기는 없다.
[매크로 정의](https://github.com/latex3/latex2e/blob/develop/base/ltspace.dtx#L515-L520)

핵심은 [TeX 엔진의 일반 수학 그룹 처리](https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/tex.web#L13603-L13616)다.
일반 그룹의 sub_mlist는 내부 수식 목록을 변환한 후 `hpack(..., natural)`로
하나의 가로 상자가 된다. 이 과정은 문단 줄바꿈 알고리즘을 실행하지 않는다.
따라서 그 안의 강제 줄바꿈 penalty는 행을 만들지 못하고, hfil의 자연 폭은 0이다.

**이 예제의 `{d+\\e}`에 한정하면 `\\`를 빼고 한 줄로 만드는 것이
최종 표시와 대응한다.** 매크로가 아무 일도 하지 않는다는 의미는 아니다.
일반 그룹 밖의 inline 수식은 문단 줄바꿈에 노출될 수 있고, 배열은 별도의
행 처리 매크로를 쓰므로 모두 무시하는 규칙으로 일반화하면 안 된다.
별표·선택 길이·앞선 명시적 glue가 있는 변형도 별도 검토가 필요하다.

이 결론은 매크로와 엔진 소스의 실행 경로를 분석한 결과다. 이 환경에는
TeX 실행기가 확인되지 않아 실제 TeX 컴파일 비교는 수행하지 않았다.
위 규칙을 변환기에 반영하고 080 전체 예제와 한글 이미지로 확인한다.
