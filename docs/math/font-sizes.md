# TeX 글자 크기 선언

변환 기준은 **LaTeX 표준 10pt 크기 체계에 대응하는 KaTeX 배율**이다.
LaTeX의 실제 크기는 문서 클래스와 10pt/11pt/12pt 옵션에 따라 달라지므로,
모든 LaTeX 문서에 고정된 배율이라고 해석하지 않는다.

| 명령 | KaTeX 배율 | 10pt 기준 크기 | 출력 SCALE(정수 반올림) |
| --- | ---: | ---: | ---: |
| `\tiny` | 0.5 | 5pt | 50 |
| `\scriptsize` | 0.7 | 7pt | 70 |
| `\footnotesize` | 0.8 | 8pt | 80 |
| `\small` | 0.9 | 9pt | 90 |
| `\normalsize` | 1 | 10pt | 100 |
| `\large` | 1.2 | 12pt | 120 |
| `\Large` | 1.44 | 14.4pt | 144 |
| `\LARGE` | 1.728 | 17.28pt | 173 |
| `\huge` | 2.074 | 20.74pt | 207 |
| `\Huge` | 2.488 | 24.88pt | 249 |

근거는 [LaTeX 표준 클래스의 크기 정의](https://raw.githubusercontent.com/latex3/latex2e/main/base/classes.dtx),
[KaTeX Options의 sizeMultipliers](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/Options.ts),
[KaTeX sizing 명령과 크기 인덱스 연결](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/functions/sizing.ts)이다.
실제 대조에는 로컬 KaTeX 0.19.1 배포본의 같은 정의를 사용했다.
수치만 참고하며 외부 파서 구현을 복사하지 않는다.

수식과 `\text{…}` 안의 선언을 지원한다. 배율은 누적해서 곱하지 않고
기본 크기 100에 대한 절대값으로 기록한다. 중괄호·셀·텍스트 범위가 끝나면
진입 전 SCALE로 복귀한다.

```tex
{\LARGE a {\tiny b} c} d
\text{\Huge a {\tiny b} c} d
```

첫 예시는 `scale 173` 아래에서 `b`만 `scale 50`이 되고, 그 뒤는 173,
범위 밖의 `d`는 100으로 돌아간다.

한글 2020 실험에서는 SCALE의 실제 크기 효과가 관찰되지 않았다.
SCALE 코드 보존을 실제 글자 크기 재현으로 간주하지 않는다.
HML→TeX의 SCALE은 기존에 정한 범위별 math-style 근사 규칙을 유지하므로
원래 크기 명령 이름까지 왕복 보존하지 않는다.
`\big`·`\Big` 등은 구분자 높이 명령이며 이 글자 크기 표와 별도다.
