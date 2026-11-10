# Rust 수식 변환

`crates/md2hwp-math`는 한글 수식 SCRIPT와 TeX를 양방향으로 변환하는
독립 Rust 라이브러리와 개발용 CLI다. `hml`은 전체 XML 파일이 아니라
`EQUATION/SCRIPT`의 수식 코드 문자열이다. Markdown/IR 문서 생성에는
연결 중이다. core는 Markdown 수식을 IR 0.6의 `inline_math`·`display_math`로 변환하고, 한글·rhwp 백엔드가 모두 수식을 생성한다. rhwp는 한글 실측에 맞춘 크기 계산기를 쓴다. 설계는 [수식의 IR 표현](math/ir-design.md)에 있다.

## 검토 자료

- [툴바 수식 76개](math/toolbar.html)는 3칸 구성이다. 위에는 원본 한글과
  `hml_to_tex → tex_to_hml`의 실제 한글 왕복 출력을 나란히 두고, 아래에는
  `hml_to_tex` 결과 한 칸을 둔다.
  왕복 입력은 수작업 대응 TeX가 아니라 아래에 표시된 실제 변환 출력이다.

이 HTML은 이미지와 코드를 포함한 2026-10-30 스냅샷이며 인터넷 없이 열린다.
수식 이미지는 PNG로 고정했고 렌더링 라이브러리나 폰트 파일을 포함하지 않는다.
툴바 예제의 대응 TeX는 사용자가 검토·승인한 제안이다.
툴바 수식 76개는 2026-10-30 사용자 검토를 완료했다.
한글 설치본의 내장 메뉴 예시 수식은 재배포 권한이 확인되지 않아 저장소에 포함하지 않는다.

[명세 예시 수식](math/specification.html)은 수식 명세 revision 1.2의
물리적 7–16쪽에서 입력 예시, 개별 기호·별칭과 문법 표기를 수집한 별도 자료다.
페이지 위치와 원문 표기를 보존하며, 미지원·불완전 입력·빈 렌더링·각 변환 단계
실패를 숨기지 않는다. 문법표의 피연산자 없는 명령은 완성 수식과 구별한다.
명세의 `bamtrix` 오타 등도 보정하지 않고 기록한다. 기본 그리스 소문자 전체
목록은 해당 PDF에 없으므로 명세에서 추출한 것처럼 추가하지 않았다.
테스트 입력은 크레이트의 `tests/fixtures/toolbar-equations.json`과
`specification-equations.json`에 보존한다.

## 한글 SCRIPT → TeX

`hml_to_tex`는 묶음·인수·연산자·배열을 직접 읽는 파서다.
`scale <양수>`는 가장 가까운 지원 크기에 따라 TeX 스타일 선언으로 변환한다.
`0 < scale < 60`은 `\scriptscriptstyle`, `60 ≤ scale < 85`는
`\scriptstyle`, `85 ≤ scale`은 `\textstyle`이다. 경계는 기준 크기
50·70·100의 중간값이며 동률은 큰 쪽을 택한다. 100을 넘는 값도
현재는 `\textstyle`로 대응한다.
TeX의 `\scriptstyle`·`\scriptscriptstyle`은 각각 `scale 70`·`scale 50`,
`\textstyle`·`\displaystyle`는 `scale 100`으로 출력한다. 선언은 현재
그룹·셀 범위에 적용하고 범위가 끝나면 이전 크기로 복귀한다. 반복 선언은
배율을 곱하지 않는다. 한글 11.0.0.9166에서 SCALE의 실제 크기 효과는
관찰되지 않았지만, 변환에서는 이 선언과 대응 스타일을 보존한다.
`Equation` 같은 일반 단어도 이탤릭 수학 문자로 처리하며, 알려진 미지원 명령과
잘못된 명령어 대소문자는 계속 구분해 거부한다.
왼쪽 첨자 `LSUP`/`LSUB`, `UNDEROVER` 상하 배치, 장식, 큰 연산자,
다중 적분, `REL` 관계 화살표, 괄호, `cases`/`pile`/행렬을 지원한다.
`lim`과 `Lim`을 구별하며 `UNDERBRACE {a} {b}`는 실제 한글 배치에 맞춰
`\underbrace{b}_{a}`로 출력한다.

중괄호 없는 `a over b^c`의 첨자는 한글과 같이 분모에 붙는다.
분수 전체에 첨자를 붙이려면 `{{a} over {b}}^c`로 묶는다.
일반 분수 `over`는 `\frac`로 출력하여 `cases`·첨자·중첩 분수 등
주변 TeX 스타일에 맞춰 크기가 결정되게 한다.
다중 폐곡면 적분 `odint`/`otint`는 `\oiint`/`\oiiint`로 출력한다.
일반 LaTeX 문서에서는 해당 명령을 제공하는 적분 패키지나 수학 글꼴이 필요하다.
일부 큰 연산자는 기본 기호와 `\mathop`으로 표현한다. `BIGSQCAP`,
`BIGOMINUS`, `BIGODIV`는 각각 `\mathop{\sqcap}\limits`,
`\mathop{\ominus}\limits`, `\mathop{\oslash}\limits`를 사용한다.
상하한 배치는 유지하지만 기호 크기는 한글과 다를 수 있다.
텍스트 크기 명령인 `\Large`는 수식 안에 출력하지 않는다.
명세의 `coth`, `lg`, `max`, `min`, `arcsin`, `arccos`, `arctan`, `det`,
`gcd`, `hom`, `ker`, `deg`, `arg`, `dim`, `Pr`는 같은 이름의 표준 TeX 명령을
사용한다. 표준 단축 명령이 없는 `cosec`, `Exp`, `arc`, `mod`, `if`, `for`,
`and`는 철자를 보존하는 `\operatorname{...}`로 출력한다. `cosec`를 `csc`로
개명하지 않으며, 별도 인수 문법을 가진 `\mod`도 사용하지 않는다.
기존 기호 출력인 `\wedge`, `\ll`, `\gg`, `\ggg`도 각각 한글 `WEDGE`,
`<<`, `>>`, `GGG`로 역변환한다.
표준 함수 정의는 [LaTeX kernel](https://github.com/latex3/latex2e/blob/develop/base/ltmath.dtx#L99-L151)을 참고한다.
붙여 쓴 ASCII 단어의 시작에서는 가장 긴 기본 함수 이름을 인식한다.
`logx`·`sinx`는 `\log x`·`\sin x`, `arcsinx`는 `\arcsin x`다.
인용 문자열과 `xsinx`처럼 함수가 단어 처음에 없는 경우는 그대로 유지한다.
단어 전체의 결합도 보존하여 `sinx over y`는 `\frac{\sin x}{y}`, 공백으로
나뉜 `sin x over y`는 `\sin \frac{x}{y}`로 구분한다.
일부 `REL` 화살표는 `\overset`과
`\underset`을 사용하여 긴 설명에 따른 자동 확장이 다를 수 있다.
이 근사들은 76개 검토 자료의 설명에 명시했다.

입력 256 KiB, 토큰 32,768개, 재귀 깊이 96, 누적 출력 4 MiB로 제한한다.
잘못된 구조나 알려진 미지원 명령은 입력 바이트 위치와 함께 거부한다.
일반 수학 식별자는 허용하므로 모든 철자 오류를 검출하는 문법 검사기는 아니다.

## TeX → 한글 SCRIPT

`tex_to_hml`은 TeX 문자열을 직접 읽어 SCRIPT를 생성한다.
툴바 76개를 `hml_to_tex`로 변환한 모든 결과를 지원한다.
일반 한글은 따옴표 없이 원래 단어로 출력한다. 인수·첨자 경계와 명시적 공백을
넘어 합치지 않으며, 분자·분모는 중괄호로 묶어 단어 전체를 보존한다.

`\frac`·`\dfrac`·`\tfrac`·`\cfrac`는 모두 분자·분모를 보존하여
한글 `over`로 변환한다. TeX의 표시 크기와 연분수 전용 간격은 한글의
기본 분수 배치로 통일하며, 다시 TeX로 내보내면 `\frac`가 된다.
`\cfrac[l]`·`\cfrac[r]`의 분자 정렬 옵션은 받아들이되 무시한다.
옵션 없는 형태와 `[]`도 같은 한글 분수로 변환한다. 잘못된 옵션은 거부한다.

분수선이 없는 `x atop y`는 `{x \atop y}`로 출력하며 역변환도 지원한다.
TeX의 중위 분수 `a\over b`도 같은 그룹·셀 범위의 앞뒤 수식을 분자와
분모로 묶어 한글 `over`로 변환한다. 중위 분수 명령의 중복은 오류로 처리한다.
`a\above1pt b`와 `a\above{1pt} b`도 지원하며, 두께의 부호·분수선
정규화는 `\genfrac`과 같다. 그룹·셀·주석 인자 안에서도 같은 범위를 적용한다.
`\atop`은 현재 그룹·구분자·표 셀 안의 앞뒤 수식을 각각 위·아래 항으로
받는다. 괄호가 있는 기존 이항계수는 계속 `\binom`으로 처리한다.
`\genfrac{left}{right}{thickness}{style}{numerator}{denominator}`를 지원한다.
괄호 인자는 빈 그룹 또는 단일 구분자이며, `.`도 보이지 않는 구분자로 받는다.
두께가 비어 있으면 기본 분수선, 0 이하이면 `atop`, 양수이면 `over`로
정규화한다. 정확한 선 두께는 보존하지 않는다. 두께는 유한한 십진수와
TeX 단위 pt/mm/cm/in/bp/pc/dd/cc/sp/em/ex/mu를 받는다.
스타일은 빈 값이면 현재 크기를 유지하고, 0·1은 scale 100, 2는 70,
3은 50으로 변환한 뒤 이전 크기로 복귀한다. SCALE의 실제 크기 제한은
기존 규칙과 같다. 잘못된 구분자·두께·스타일은 거부한다.
TeX→HML 전반에서 이미 기본 크기일 때의 `scale 100` 선언은 생략한다.
다른 크기에서 100으로 돌아오는 명령과 그룹 종료 시 복귀 명령은 유지한다.

한글의 `sqrt 2`, `sqrt 12`, `root 3 of 27`처럼 중괄호 없는 한 항도 받는다.
숫자 낱말은 통째로 읽으며, `sqrt {a+b}`처럼 여러 항을 묶는 기존 문법은
유지한다. `acute A`, `hat AA` 같은 악센트·장식과 위·아래 중괄호 명령도
한 항이면 중괄호 없이 읽는다. 여러 항을 묶을 때는 기존 그룹을 사용한다.

출력 표기는 자동완성 목록에 맞춰 `box`, `color`, `lsub`, `lsup`, `nabla`,
`notin`, `rel`, `underover`, `CDOT`, `EXIST`를 사용한다. 역변환은 기존
검토 자료의 대소문자 별칭도 읽으며 인용 문자열은 명령어로 바꾸지 않는다.
`\Rightarrow`, `\Leftarrow`, `\Leftrightarrow`는 각각 `RARROW`, `LARROW`,
`LRARROW`로 출력한다. `<=`는 왼쪽 화살표가 아니라 작거나 같음이다.
기존 `=>`와 `<=>`는 한글에서 화살표가 아닌 개별 관계 기호로 표시되므로
이들의 별칭으로 취급하지 않는다.

자동완성 목록의 대소문자 철자는 한글 실측으로 동일함을 확인한 경우만
명시적 별칭으로 받는다. 그리스 대문자 `ALPHA`–`OMEGA` 24개는 각각 `Alpha`–
`Omega` 계열로, `Downarrow`·`Larrow`·`Lrarrow`·`Rarrow`·`Udarrow`·`Uparrow`·
`Vert`는 해당 대문자 명령으로 받는다. `centigrade`·`fahrenheit`·`laplace`도
기존 대문자형과 같다. 소문자 `omicron`은 별도로 `o`를 출력하며,
`Omicron`·`OMICRON`의 `O`와 구분한다. 전체 대소문자를 무시하지 않으므로
`alpha`와 `Alpha`, `lim`과 `Lim`, `vert`와 `VERT`의 구별은 유지한다.
인용 문자열은 명령으로 바꾸지 않는다.

자동완성의 직접 입력 실측에서 확인한 기호·장식 40개도 대응한다.
`ASSERT`, `CUP`, `del`, `dline`, 점 기호 계열, `INFINITY`·`INFTY`,
`LAND`·`NEG`·`NI`, `overline`·`underline` 등도 같은 의미의 TeX 기호로
변환한다. 역변환은 아래의 출력 표기로 정규화하므로 입력 별칭의 철자를
그대로 복원하지 않는다. TeX 명령의 동의어와 대응 유니코드 기호 입력도
같은 출력 표기를 사용한다.

| TeX 입력 | 한글 출력 표기 |
| --- | --- |
| `\bigvee` / `\bigwedge` | `BIGVEE` / `BIGWEDGE` |
| `\cup` | `CUP` |
| `\dashv` | `DASHV` |
| `\ggg` | `GGG` |
| `\ni` / `\owns` | `NI` |
| `\infty` | `INFTY` |
| `\underline` | `underline` |
| `\bar` / `\overline` | `bar` |
| `\lnot` / `\neg` | `LNOT` |

예를 들어 `SMALLUNION → \cup → CUP`, `OWNS → \ni → NI`,
`INF → \infty → INFTY`, `under {a} → \underline{a} → underline {a}`로
회귀한다. `bar`와 `LNOT`는 현재 TeX 출력인 `\bar`와 `\lnot`에 맞춘다.
`VDASH`, `WEDGE`, `nabla`, `VERT` 등은 기존 출력 표기를 유지한다.

`MHO`, `SDOTS`, `TRIANGLEL`·`TRIANGLER`, 부정 집합 관계와 작은 합·곱
기호도 읽는다. `SDOTS`는 위로 올라가는 점 `⋰`로 출력하여 `DDOTS`와
방향을 구별한다. `SMALLSUM`·`SMALLPROD`는 해당 연산자에만 적용되는
`{\textstyle\sum}`·`{\textstyle\prod}`로 출력한다.
`SMALLCOPROD`도 사용자 결정에 따라 명목상 작은 쌍대곱 기호로 지원하며,
`{\textstyle\coprod}`와 양방향 변환한다. 실제 한글 직접 입력에서는
일반 글자로 표시되었으므로, 이 지원을 한글의 실제 기호 해석과 동일하다고
간주하지 않는다. `\coprod`는 [MathJax의 TeX 명령 목록](https://docs.mathjax.org/en/latest/input/tex/macros/index.html)에 있는 명령이다.
`HDOWN`·`HRIGHT`·`HUP`·`HCROSS`·`PHOR`·`PVER`·`ROUND`와
`DASHV`·`MASSERT` 등은 같은 의미의 기존 TeX 기호로 정규화하므로
한글의 원래 글리프 크기·획·간격과 역변환 결과가 다를 수 있다.
지원 판정은 기호의 의미와 방향을 기준으로 하며 픽셀 일치를 뜻하지 않는다.

`boldrm`·`rmbold`는 HML→TeX에서 명령의 의도에 따라 굵은 직립체로 옮긴다.
사용자가 확인한 `boldrm` 표시와 별개로, HML→TeX에서는 의도상 굵은 직립체로 취급한다.
TeX→HML은 실제로 굵은 직립체가 되는 `rm bold` 순서를 사용한다.
`bold {rm a}`도 굵은 직립체이며, `rm`·`it`의 상태 복귀는 별도로 유지한다.

시험한 문맥에서 일반 글자로 표시된 `dirac`·`ldirac`·`rdirac`,
`exrel`·`ldrel`·`lrdrel`·`lrel`·`rdrel`·`rrel`, `string` 10개는 명령 지원으로
추가하지 않았다. [MathJax의 braket 확장](https://docs.mathjax.org/en/latest/input/tex/extensions/braket.html)은
`\bra`·`\ket`·`\braket`을 제공하지만, 확인한 문서에 같은 이름의
`dirac`·`ldirac`·`rdirac` 명령은 없다. 자동완성 표시명만으로 실제 삽입
구문을 확정할 수 없어 이들을 `\bra`·`\ket`·`\braket`에 임의로 대응하지 않는다.

별도 구조인 `longdiv`는 `ladder`·`sladder`와 함께 구조상 지원 범위에서
제외한 무시 목록에 포함한다. 괄호·부정 약식 15개도 사용자가 직접 입력하여
일반 문자로 확인했으므로 아래 무시 목록에 포함한다. `dirac`·`ldirac`·
`rdirac`, 관계식 약식 `exrel`·`ldrel`·`lrdrel`·`lrel`·`rdrel`·`rrel`,
`string`도 사용자 결정에 따라 구현 후보에서 제외하고 무시 목록에 포함한다.

`VecA`–`VecZ`는 한글의 고정 이중선 대문자이며 `\mathbb{A}`–
`\mathbb{Z}`로 변환한다. 화살표 장식 `vec {A}`와는 별개다. 역변환의
`\mathbb`는 일반 수식 인수를 읽으며, 대문자는 각 native 이중선 기호로
보존한다. 지원되는 소문자 k는 유니코드 이중선 글자로, 그 밖의 소문자·숫자·
기호는 기본 수학 글꼴로 유지한다. 중첩된 악센트와 수식 구조도 처리하며
`\mathbb A`처럼 중괄호 없는 한 글자도 받는다.

붙여 쓴 `vec`·`Vec`·`VEC` 장식 접두어도 읽는다. 예를 들어 `Veca`·`veca`는
`\vec{a}`, `VECA`는 `\vec{A}`, `vecab`는 `\vec{ab}`다. 정확한 이중선
기호가 접두어보다 우선하므로 `VecAB`는 이중선 A와 일반 B이며,
`vecAB`의 AB 전체 위 화살표와 다르다. 등록하지 않은 혼합 표기 `veCalpha`는
일반 문자다. 한글의 `Veca_i`와 `vec {a}_i`는 `\vec{a_{i}}`로,
전체 장식을 묶은 `{vec {a}}_i`는 `\vec{a}_{i}`로 구분한다.
인용된 `"VecA"`는 그대로 문자로 취급하고, TeX에서 온 일반
문자열도 한글의 기호·장식 명령으로 다시 해석되지 않도록 보호한다.

`\overset{\frown}{a}`는 TeX 인수 구조를 읽어 `arch {a}`로 변환하며,
단독 `\frown`은 거부한다. 내부 판별용 문자열을 SCRIPT에 출력하지 않는다.

`\overset{a}{x}`와 `\underset{b}{x}`는 임의의 밑항 위·아래에 주석을
놓는 일반 구문이며 `underover {x}`에 해당 첨자를 붙인다. 서로 반대쪽
주석은 중첩 순서와 불필요한 묶음에 관계없이 한 상하 구조로 합친다.
같은 쪽 주석이 반복되거나 밑항에 별도 첨자·추가 항이 있으면 그 구조를
보존하며 안쪽 내용을 버리거나 화살표 하나로 축약하지 않는다.
특수처리: 밑항이 실제 TeX 화살표 명령 하나인 경우에만 한글의 주석 화살표
`rel`로 정규화한다. 예를 들어
`\overset{a}{\underset{b}{\Longleftrightarrow}}`는
`{rel LRARROW {a} {b}}`다. 이는 네이티브 화살표와 주석 배치를 보존하기
위한 규칙으로, 일반 문자나 인용된 `LRARROW` 이름에는 적용하지 않는다.

내보낼 때는 원래 한글 구조를 구별한다. 전용 `Lim`은 직립체 연산자 이름인
`\operatorname*{Lim}`, 일반 `underover {x}`는 `\mathop{x}\limits`,
주석 화살표 `rel`은 `\overset`·`\underset` 또는 확장 화살표로 출력한다.
첨자 있는 native 위·아래 연산자는 필요한 `\limits`를 명시하여 inline TeX로
재변환해도 배치를 유지한다. `\nolimits`를 새로 생성하지 않는다.

`\operatorname*{foo}_{a}^{b}`는 현재 TeX 스타일이 display이면
`underover {rm "foo" it} _{a} ^{b}`로, text이면 오른쪽 첨자로 변환한다.
명시적 `\limits`는 위·아래, `\nolimits`는 오른쪽 배치를 강제한다.
생성된 위·아래 밑항이 정확히 `underover {rm "Lim" it}`일 때만 전용 `Lim`으로
정규화한다. 대소문자나 글꼴 복귀가 다르면 특수처리하지 않는다.
`\operatorname*{Lim}\limits_{a} c`와
`\overset{a}{\underset{b}{Lim}}`는 같은 식이 아니다. 후자의 Lim은
일반 수학 문자이며 위 주석 a, 아래 주석 b가 있고 뒤의 c도 없다.

별표 없는 operatorname의 cosec, Exp, arc, mod, if, for, and는 일반 이름
처리 결과가 정확히 `{rm "이름" it}`일 때만 기존 한글 키워드로 정규화한다.
별표 없는 일반 이름은 오른쪽 첨자를 사용하며 명시적 limits를 줄 수도 있다.
이름은 그룹 또는 단일 토큰으로 받으며, 지원하는 기호 명령도 읽는다.
빈 이름은 생략하되 첨자를 붙이면 거부한다. 알 수 없는 명령·중첩 그룹·따옴표·
달러·제어문자는 거부한다. 역변환에서는 직립체 인용 이름을 구조로 보존하고
일반 수식이나 내부 첨자를 이름으로 오인하지 않는다.

한글의 `rm`·`it`는 중괄호가 끝나도 유지되는 글꼴 모양 전환 명령이다.
기본 영문은 일반 이탤릭체이며, 따옴표 자체도 로만체를 지정하지 않는다.
`bold`는 다음 기본 항이나 그룹에만 적용한다. `bold a b`는 a만 굵고,
`bold {a b} c`는 그룹의 a·b만 굵다. 그룹 안의 `rm`·`it`는 모양을 바꾸지만
그룹 전체에 걸린 굵기를 취소하지 않는다. `bold {rm a} b`는 a가 굵은 직립,
b가 보통 직립이다. 반면 `bold rm a`는 항에 들어가기 전에 rm이 굵기 지정을
취소하므로 보통 직립이다. 따라서 `rm bold`는 다음 항의 굵은 직립체, `it bold`와
기본 상태의 `bold`는 다음 항의 굵은 이탤릭체다. 영문·인용 텍스트를 내보낼 때
굵은 직립은 `\mathbf`, 굵은 이탤릭은 `\boldsymbol`로 구별한다.

역변환은 `\mathbf{abc}`를 `{rm bold {a b c} it}`, 기본 상태의
`\boldsymbol{abc}`를 `{it bold {a b c} it}`로 생성한다. 중첩 범위를 마치면
이전 기울기와 굵기를 모두 복원한다. 이전 상태가 굵은 직립이면 `rm bold`,
굵은 이탤릭이면 `it bold`를 써야 하므로 복귀에 `bold`만 사용하지 않는다.
한글의 굵은 그룹 내부에서는 rm/it로 굵기를 해제할 수 없다. 복잡한 TeX 중첩
글꼴 선택의 굵기 해제까지 원문과 같은 표시를 보장하지 않는다.
중괄호·근호·분수·행렬을 지나는 전환 상태와 복귀 명령 없는 입력도 보존한다.

자동완성 분석의 [무시 목록](math/autocomplete-ignore.json)은 구현 후보에서
제외하기로 한 항목을 기록한다. 현재 `longdiv`·`ladder`·`sladder`와 일반 문자로 확인된
괄호·부정 약식 15개를 분석·구현 후보에서 제외하며,
이 설정은 실제 SCRIPT 입력을 삭제하거나 오류 처리를 바꾸지 않는다.

자동완성 표시명과 실제 삽입 구문은 구분한다. 예를 들어 `root`의 실제
구문에는 `of`가 필요하고, 괄호 삽입 구문에는 `LEFT`와 `RIGHT`가 포함된다.
이런 구조 토큰과 일반 변수·인용 텍스트를 자동완성 표시명과 단순 대조하여
일괄 거부하지 않는다.

분수·근호·첨자·함수·그리스 문자, 장식·큰 연산자·다중 적분,
`cases`/`aligned`/`gathered`와 기본 행렬, RGB 색상 및 명시적 간격을 지원한다.
빈 밑항의 첨자는 실제 한글 `LSUP`/`LSUB`로, 상하 배치와 주석 화살표는
`UNDEROVER`/`REL`로 복원한다. 중괄호를 포함한 정상 쌍의 가변 괄호,
바닥·천장 괄호, `Lim`, 위·아래 중괄호의 인수 역할을 구분한다.
일반 빈칸 `~`는 TeX의 명시적 공백 `\ `로, `~~`는 `\quad`로 출력하며
역변환도 같은 간격으로 받는다. 백틱 간격은 `\,`로 출력하고 역변환한다.
`\mkern`은 0–72mu의 음이 아닌 십진수를 받아 한글 간격으로 근사한다.
변환 기준은 백틱 2.25mu, `~` 9mu이며, 가장 가까운 2.25mu 배수로 반올림한다
(정확히 중간이면 큰 쪽). `~`와 남은 백틱을 조합하고 0으로 반올림되면 생략한다.
예를 들어 2mu는 백틱 하나, 4mu는 두 개, 9mu는 `~`, 18mu는 `~~`다.
소수·선행 `+`·단위 앞 공백을 허용하되, 음수·범위 초과·다른 단위·지수 표기·
매크로 길이는 거부한다. 실제 한글 너비와의 일치를 보장하는 측정값은 아니다.
`align`·`align*`·`aligned`와 `alignat`·`alignat*`·`alignedat`는 한글 `eqalign`로
변환한다. `align` 계열은 독립 수식, `aligned` 계열은 수식 안에 넣는 정렬 구조지만
한글 수식 코드에서는 같은 정렬 구조를 사용한다. `alignat`·`alignedat`의 열 쌍
개수 인수는 읽고, `&`로 지정한 빈 셀도 보존한다. 행마다 셀 수가 다르면
부족한 마지막 셀을 빈칸으로 채워 같은 열 수를 만든다.
수식 내부 번호·참조 이름인 `\tag`·`\tag*`·`\label`의 인수와
`\nonumber`·`\notag`는 출력에서 생략한다.

매크로 프로그램, 사용자 패키지·설정, 임의의 `array` 열 프로그램, `multline`,
텍스트 안의 중첩 수식, 행 간격 옵션 등은 지원하지 않는다.
역방향 중괄호·짝이 없는 중괄호와 `aligned` 위치 옵션도 거부한다.
`\mathop`은 그룹과 명시적 `\limits`가 둘 다 있을 때 기존 큰 연산자 배치를
사용하며, 그 밖에는 일반 기호로 표시한다. 크기는 지원하는 선언만 받는다. TeX 전체 문법이나 패키지를 지원한다는 뜻은 아니다.

입력 256 KiB, 깊이 96, 항 32,768개로 제한한다. 미지원 명령과 환경은
입력 바이트 위치와 함께 오류를 반환한다.

## 빌드와 실행

```powershell
cargo build -p md2hwp-math
target/debug/md2hwp-math.exe hml-to-tex 'root {3} of {27}'
target/debug/md2hwp-math.exe tex-to-hml '\sqrt[3]{27}'
```

표현식을 생략하면 표준 입력을 읽는다. `batch`는 `cases` 배열을 가진 JSON을
받으며 역변환 입력 키는 `hancom_script`, 정변환은 `tex`다.

```powershell
target/debug/md2hwp-math.exe batch hml-to-tex crates/md2hwp-math/tests/fixtures/toolbar-equations.json artifacts/toolbar-result.json
target/debug/md2hwp-math.exe batch tex-to-hml tests/katex/cases.json artifacts/katex-result.json
```

결과에 원본 사례·입력·출력 또는 오류를 남긴다. 출력 경로가 입력 파일을
가리키면 거부한다. Release 빌드는 `cargo build -p md2hwp-math --release`이며,
개발용 EXE는 기존 배포 ZIP에 자동 추가하지 않는다.

## 검증 범위

툴바 76개 모두 원본 SCRIPT → TeX → SCRIPT 변환 후 한글 2020에서 실제로
렌더링했다. 한글에서 읽어 온 SCRIPT도 제출한 문자열과 모두 같았다.
원본 한글 이미지와 비교하면 76개 전부 픽셀까지 같다. `Lim`은 일반 규칙 이후
전용 명령으로 정규화하여 왕복을 보존한다.
아래쪽 TeX 이미지는 MathJax 4.1.3으로 렌더링하므로 한글 서체와의 차이는 남는다.

회귀 검사는 툴바 76개 왕복, 중첩 첨자·분수·행렬·관계 화살표, 인수 순서,
분모 첨자 결합, 잘못된 입력과 누적 자원 제한을 포함한다.

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

새 한글 문서에서 실제 모양을 확인하는 도구는
`tools/development/render_hancom_equation_gallery.py`다. 별도 한글 프로세스만
소유·정리하며, 기존 보안 모듈의 등록을 확인할 뿐 설치나 레지스트리를 바꾸지 않는다.
검토 HTML은 고정 자료이므로 변환기가 바뀌면 해당 이미지를 다시 검토해야 한다.

## 참고 명세

`reference/hancom-api/한글문서파일형식_수식_revision1.2.pdf`를 참고했다.
이 파일의 SHA-256은
`d5a99c24c64bb74ecc782a0f2f51cc6923b00b524ff24d0c0a62a01fee3d993f`다.

본 제품은 한글과컴퓨터의 글 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.
프로젝트 라이선스가 한컴의 서체 등에 대한 권리를 대신하지 않는다.

### 예약 함수와 인용 문자열

명세의 `si n`, `s i n` 예시는 원문 그대로 남긴다. 명세는 공백으로 예약어를
나누면 이탤릭체가 된다고 설명하지만, 한글 2020(11.0.0.9166)의 이번
`EquationCreate` 시험에서는 공백이 SCRIPT에 보존되어도 직립체 `sin`으로
표시됐다. HML→TeX는 명세의 설명보다 실제 한글의 동작에 맞춰 이 예시를 직립체
함수로 출력한다. `si nx`도 `\sin x`로 인식하지만, `si n over y`는
이탤릭체 `si`와 분수 `n/y`로 나뉜다. 공백을 일괄 삭제해 분수의 피연산자를
바꾸지 않으며, 인용 문자열과 명시적 간격은 예약어 결합을 막는다. 원본·왕복 한글 픽셀 일치만으로 가운데 TeX의
시각적 일치를 보장하지 않는다.

보충 예시 `"sin"`은 실제 한글에서 이탤릭체다. HML→TeX는
`sin`으로 출력하며, TeX→HML은 예약 함수로 재해석될 일반 문자에만
따옴표를 남긴다. `abc`나 `xsin` 같은 짧은 일반 낱말은 그대로 출력한다.
알 수 없는 생성자와 구분해야 하는 낱말 뒤 그룹 및 대문자 낱말은 현재
HML 읽기 문법과의 왕복을 위해 인용한다. 명시적인 텍스트·연산자 이름의
인용 의미도 유지한다. TeX 명령 `\sin`은 한글 함수 `sin`으로
변환하고, 명시적인 `\mathit{sin}`도 이탤릭체 리터럴로 받는다.
TeX 입력의 첨자·인수는 한 문자씩 파싱하며, 첨자 없는 일반 문자들은 표시되지
않는 TeX 공백·탭·줄바꿈을 사이에 두어도 한 낱말로 합친다.
명령·그룹·명시적 간격·첨자 경계를 넘어 합치지 않는다.
`a b c`는 `abc`, `abc^2`는 `ab c^{2}`가 된다.
한글 2020에서 9글자 낱말은 그대로 유지되지만, 10글자 이상을 인용하지 않으면
분수 앞에서 첫 9글자와 나머지가 갈라지는 것을 확인했다. 따라서 10글자
이상은 하나의 인용 문자열로 보존한다. 인용 문자열을 9글자씩 나누지 않는다.
예약어 판단은 개별 조각뿐 아니라 단순 그룹·공백 너머의 문자 흐름도 살펴,
`s{i}n`처럼 합쳐서 예약어가 되는 경우도 보호한다. 이 보호는 기본 이탤릭체뿐 아니라 다른 글꼴 범위에도 적용한다. 공백으로 나누거나 `it sin`으로 바꾸는
것만으로는 이 보존이 되지 않는다.

### 일반 문자 낱말 뒤 첨자

한글 `E=mc^2`는 TeX `E = mc^{2}`로 출력한다. 왼쪽 첨자와 명시적 그룹이 없는 ASCII 문자
낱말 뒤 오른쪽 첨자는 마지막 문자에 붙이며, `mc` 전체를 자동으로 한 밑판으로
묶지 않는다. 아래첨자도 같은 규칙이다. `sinx^2`도 `\sin x^{2}`로 구분한다.
원문에 명시한 `{mc}^2` 같은 그룹, 숫자 낱말과 인용 문자열의 범위는 유지한다.

### 한글 입력에서 붙여 쓴 피연산자의 범위

중괄호가 없는 한글 분수의 피연산자는 원문에서 공백 없이 이어진 범위를
읽는다. `10a^3 over b^2`는 `\frac{10 a^{3}}{b^{2}}`,
`b^3 over 2a`는 `\frac{b^{3}}{2 a}`가 된다.
`a+b over c+d`는 전체가 분수지만, `a + b over c + d`는 `b/c`만 분수다.
명시적인 그룹은 독립된 경계이므로 `x+{a} over {b}+y`의 `x+`·`+y`는
분수에 포함하지 않는다.

중괄호 없는 첨자도 공백·다음 첨자 명령·컨테이너 끝 전까지 읽는다.
`sum_n=1^N`은 `\sum_{n = 1}^{N}`, `lim_N->inf`는
`\lim_{N \to \infty}`로 변환한다. `x^n+1`과 `x^n + 1`은 서로 다르다.
명시적 간격과 새 생성자는 첨자 경계로 유지하고 근호·장식의 인수 규칙을
임의로 확장하지 않는다.

`2^k)`의 닫는 괄호까지 지수로 올리는 한글의 입력 해석은 따라 하지 않는다.
명세의 작은 괄호 예시에서는 괄호를 수식 밖에서 닫는 정상 TeX를 출력하며,
사용자가 승인한 원본·왕복 차이로 별도 표시한다.

### BIGG 크기 정규화

한글 `bigg /`·`BIGG /`는 고정 크기의 TeX `\Bigg/`로 단순 치환한다.
TeX→HML은 big/Big/bigg/Bigg를 각각 `scale 120/180/240/300`으로 옮기고
기호 뒤에서 이전 크기를 복원한다. l/r/m 변형도 같은 크기다.
기호는 인용해 독립된 괄호가 문법상의 그룹으로 해석되지 않게 하며,
보이지 않는 점 구분자는 생략한다. 일반 연산자는 구분자로 받지 않는다.
한글 BIGG는 앞서 나온 식의 높이에 맞춰 크기를 조절하지만 TeX의 크기는 고정이며,
한글 SCALE의 실제 크기 효과는 확인되지 않았다. 원래 크기의 왕복 보존을 보장하지 않는다.

### NOT 사선 겹치기

한글 `not =`·`NOT =`와 TeX `\not=`를 양방향 변환한다.
직접 쓴 ASCII 영숫자·한글·지원 문장부호와 지원 기호 명령을 대상으로 한다.
`not ab`는 `\not a b`처럼 첫 글자에만
적용한다. `not ab over c`도 `\not a \frac{b}{c}`로 항 범위를 보존한다.
`not IN`·`not LEQ`는 이 접두 문맥에서 각각 `\not\in`·`\not\leq`로 읽는다.
기존 단일 기호 `notin`·`!=`의 변환은 그대로 유지한다.

한글 2020은 `not int`·`not oint` 같은 적분 명령어 위에 사선을 그리지 않고
작은 점만 남긴다. 그래서 `\not\oint`, `\cancel{\oint …}` 등의 적분은
Unicode 문자 `∫`·`∬`·`∭`·`∮`·`∯`·`∰`를 사용해 `not ∮`처럼 출력하고,
한글의 `not ∮`는 `\not\oint`로 역변환한다. 이 문자는 일반 적분 기호보다
작게 표시되며 첨자가 있으면 사선이 뒤의 항까지 지나간다. 사선이 보이지 않는
적분 명령어 형태 `not int`·`not INT`와 `not sum` 같은 큰 연산자는 계속 거부한다.

TeX의 한 글자 그룹 `\not{a}`는 한글 `not a`로 출력한다. 한글의
`not {a}`는 중괄호 자체에 사선을 붙이므로 그룹 인수로 받지 않는다.
인용 문자열·여러 항·함수·생성자·중첩 NOT도 지원 대상으로 추측하지 않고
명시적으로 거부한다. 글꼴은 `rm not a`처럼 바깥에서 지정한 경우를 보존하며,
NOT 뒤에 바로 글꼴 명령을 넣는 형태는 받지 않는다.

### 기존 문법의 별칭과 기호 표기

명세의 대문자 생성자·연산자 표기는 이미 구현된 같은 문법으로 연결한다.
예를 들어 `OVER`, `SQRT`, `MATRIX`, `PILE`, `SUM`, `COPROD`와
소문자 `left`·`right`를 받는다. 인용 문자열 안에는 적용하지 않으며,
`Lim`·`lim`, 대소문자 그리스 문자와 화살표의 차이는 유지한다.
TeX 명령 이름을 임의로 한글 명령으로 해석하지 않는다.

기존 기호 대응의 누락도 연결한다. 관계·집합 기호, 대문자 그리스 문자 중
이미 대응 명령이 있는 것, `VEE`·`LOR`, `LLL`·`<<<`, `vert`·`VERT`,
천장·바닥 기호가 해당한다. `VERT`와 원시 `||`는 한글에서 간격이 달라
역변환은 `VERT`를 사용한다.

명세의 피연산자 없는 표기는 원문을 유지하고 `test_script`에 시험 문맥을
보충한다. 이것만으로 새 문법을 지원한다고 간주하지 않는다.
새 기호·서체 대응 등 실제 구현이 필요한 항목은 별도로 구분한다.

### BUILDREL 관계식

`BUILDREL 화살표 {위쪽 내용}`은 REL의 아래쪽 내용을 생략한 문법이다.
`A BUILDREL <-> {+2} B`를 `A \overset{+ 2}{\longleftrightarrow} B`로
변환한다. `buildrel` 소문자 표기와 기존 REL 화살표도 받는다.
TeX→HML은 위 주석만 있어도 `rel 화살표 {위} {}`로 생성하며 BUILDREL을 새로 출력하지 않는다.
위쪽 내용은 중괄호로 묶으며, 뒤의 본문은 추가 인수로 소비하지 않는다.
역변환은 기존 REL·UNDEROVER 표현을 사용하므로 명령어 철자 자체의 일치는
보장하지 않는다.

### 세로 쌓기·조합·행렬 열 정렬

`pile{a#bbb}`는 가운데, `lpile{a#bbb}`는 왼쪽,
`rpile{a#bbb}`는 오른쪽으로 맞춘다. 가운데 쌓기는 기존 `gathered`,
왼쪽·오른쪽 쌓기는 단일 열 `array`의 `l`·`r` 정렬로 변환한다.

`n CHOOSE k`와 `BINOM n k`는 모두 `\binom{n}{k}`로 변환한다.
복합 피연산자는 중괄호로 묶는다. 역변환은 `binom {전체항} {선택항}`을
사용하므로 원문 명령의 철자나 접두·중위 표기까지 보존하지는 않는다.

행렬은 기존 행 기준 입력 외에 `matrix{lcol{a#bbb} rcol{ccc#d}}`처럼
열 기준 입력도 받는다. `col`은 가운데, `lcol`은 왼쪽, `rcol`은 오른쪽
맞춤이다. 각 열의 행 수는 같아야 한다. TeX에서는 `array`의 `l/c/r` 열
지정으로 나타내며, 기존 행렬 괄호도 유지한다. TeX에서 한글로 변환할 때 여러
열의 `array`는 열별 정렬을 무시하고 행 단위 `matrix`를 생성한다. 열 지정의
`|`·`:` 구분선도 생략한다. 예를 들어 `{|rl:c||}`는 세 열로 읽는다.
폭·삽입·반복 프로그램이 들어간 임의의 `array` 열 문법은 이 범위에 포함하지 않는다.
`\hline`·`\hdashline`도 생략한다. `\cr`은 `\\`와 같은 행 구분자로 받는다.
`\\[1ex]` 같은 행 간격 옵션은 생략하고 행 구분만 유지한다. 옵션 안의
치수나 매크로는 해석하지 않으며, 닫히지 않은 옵션은 오류로 처리한다.
배열 밖의 `\\`는 바깥 수식을 `pile {첫 행 # 다음 행}`로 묶어 줄을 나눈다.
내부 배열의 행 구분은 해당 배열 안에 유지하며, 분수의 범위는 각 행에서 끝난다.
`smallmatrix`는 `scale 70`을 붙인 `matrix`로, `subarray`는 단일 열의
`c`·`l` 지정에 따라 `pile`·`lpile`로 변환한다. 두 환경을 벗어나면 진입 전
크기 선언으로 복귀한다. 현재 한글에서 `scale`의 실제 크기 효과는 관찰되지
않았으므로 TeX의 작은 행렬·첨자 배열보다 크게 표시될 수 있다.

### 명세의 개별 기호

명세의 그리스 대문자부터 마지막 특수 기호까지 모두 대응한다. 단순히
명령 이름의 대소문자를 바꾸지 않고 실제 한글 기호와 대응시킨다.
화살표의 단선·복선, 그리스 문자 변형과 서로 다른 점의 위치를 유지한다.
그리스 대문자는 한글처럼 기울임꼴로 출력하고 `OHM`의 직립체와 구분한다.
라틴 글자와 같은 모양인 대문자는 Overleaf의 기본 수학 모드에서도 쓸 수
있도록 `A`, `B`, `E`, `Z`, `H`, `I`, `K`, `M`, `N`, `O`, `P`, `T`, `X`, `Y`로
정규화한다. 이 문자들은 기본이 기울임꼴이므로 `\mathit`을 붙이지 않는다.
역변환도 라틴 글자를 사용하므로 원래 그리스 문자와 한글 서체·폭의 차이가
남을 수 있다. 나머지 대문자는 `\mathit{\Gamma}` 등 ASCII LaTeX 명령을 쓴다.

| 한글 입력 | 대응 기호·TeX |
| --- | --- |
| `Alpha`·`Beta` 등 라틴 자형의 그리스 대문자 | `A`·`B` 등 기본 수학 문자 |
| `Gamma`·`Delta` 등 별도 자형의 그리스 대문자 | `\mathit{\Gamma}`·`\mathit{\Delta}` 등 ASCII LaTeX 명령 |
| `OHM` / `Omega` | `\Omega` / `\mathit{\Omega}` |
| `varupsilon` / `Upsilon` | `\Upsilon` / `Y` |
| `DEG` | `{}^{\circ}` |
| `LAPLACE` | `\mathcal{L}` |
| `CENTIGRADE` / `FAHRENHEIT` | `{}^{\circ}\mathrm{C}` / `{}^{\circ}\mathrm{F}` |
| `ANGSTROM` | `\text{\AA}` |
| `STAR` | `\bigstar` |
| `IDENTICAL` / `EQUIV` | `::` / `\equiv` |
| `image` / `REIMAGE` / `IMAG` | `\fallingdotseq` / `\risingdotseq` / `\Im` |
| `ASYMP` / `ISO` | `\asymp` / `\Bumpeq` |
| `RTANGLE` | `∟` |
| `well` / `base` / `benzene` | `⌗` / `⌂` / `⌬` |
| `att` / `hund` / `thou` | `\text{\textreferencemark}` / `\text{\textperthousand}` / `\text{\textpertenthousand}` |

LaTeX 문서에는 다음 패키지를 준비한다. 텍스트 기호는 textcomp를 사용하며
`\text`는 amsmath 문법이다. `IDENTICAL`의 `::`에는 별도 패키지가 필요하지 않다.
MathJax 검토 환경도 [textcomp](https://docs.mathjax.org/en/latest/input/tex/extensions/textcomp.html) 확장을 함께 읽는다.

```latex
\usepackage{amsmath,amssymb,textcomp}
```

역변환은 직립체 `\Omega`와 명시적인 `\mathit{\Omega}`를 각각 OHM과
Omega로 구분한다. `\mathcal`은 대문자 스크립트 글꼴로 옮기며 L은
기존 LAPLACE를 사용한다. 텍스트 명령도 위의 단일 기호 조합을 지원한다. 임의의 매크로를
실행하거나 확장하지 않는다. 유니코드 입력 별칭은 계속 받을 수 있다.

`RTANGLE`·`well`·`base`는 MathJax에서 지원하지 않는 명령 대신 각각
`∟`·`⌗`·`⌂`로 출력한다. `benzene`도 기존 `⌬` 출력을 유지한다.
역변환은 이 유니코드 기호들과 기존 `⊾` 입력을 받으며, `\rightangle`·`\viewdata`·
`\house` 명령도 계속 받는다. NOT 아래의 알려진 유니코드 기호도 같은 명령으로
복원한다. 일반 pdfLaTeX에서는 이 유니코드 수학 기호를 별도로 설정해야 하며,
유니코드 수학 환경을 사용할 수 있다.
전체 대응 코드는 명세 예시 HTML의 각 기호 아래에 표시한다.

## KaTeX 테스트 수식 비교

`\text`·`\textrm`·`\mbox` 안에서는 일반 문자, 중괄호 그룹과 지원하는 문자
악센트 명령을 읽는다. `\'`·`\acute`, ``\` ``·`\grave`, `\.`·`\dot`,
`\"`·`\ddot`, `\^`·`\hat`, `\~`·`\tilde`, `\v`·`\check`, `\=`·`\bar`를
한글의 같은 악센트 명령에 대응한다. 텍스트 안의 `\i`·`\j`는 점 없는
문자 `ı`(U+0131)·`ȷ`(U+0237)로 처리하며,
문자는 로만체로 표시하고 범위를 마치면 이전 글꼴로 돌아간다.
`\H`의 double acute와 `\r`의 고리는 각각 문자 바로 뒤에 `˝`·`˚`를 붙인다.
예를 들어 `ı˝`, `A˚`로 표시한다. 첨자나 상하 배치 구조를 만들지 않는다.
`\underbar`의 내용도 같은 텍스트 처리와 로만체 적용·복귀 규칙을 사용한다.
`\textbf{…}`는 `\mathbf`와 같은 `{rm bold {…} it}`로, 텍스트 명령 안의
`\textbf{…}`는 텍스트 안 `\bf`와 같은 `bold {…}`로 옮긴다. `\textit{…}`는
`{it {…} it}`로 옮기고, 텍스트 안에서는 `{it {…} rm}`처럼 범위가 끝나면
이전 글꼴로 되돌린다. 한글 글꼴 명령은 중괄호로 범위가 닫히지 않기 때문이다.
텍스트 안의 `\textrm`·`\textsf`는 로만으로 바꾸고 고정폭 서체도 해제하며,
`\text`·`\mbox`는 바깥 서체를 유지한다. 이탤릭 범위의 굵은 글씨는 굵은 이탤릭이다.
한글 `it`은 숫자를 기울이지 않으므로 `\textit{123}`의 숫자는 똑바로 표시된다.
역변환은 `\mathrm`·`\mathbf`·`\mathit`·`\boldsymbol` 같은 수학 서체 명령이며,
중괄호 인수가 없거나 내부에 `$…$` 수식이 있으면 거부한다.
여러 열의 `array`는 열별 `l`·`c`·`r` 정렬을 무시하고 행 단위의 한글
`matrix {a & b # c & d}`로 만든다. 한 행의 셀들이 공통 높이를 사용하여
악센트 높이가 다른 열도 같은 행에 놓이도록 한다.

[KaTeX 테스트 비교 HTML](math/katex.html)은 upstream 시각 테스트 128개를
원래 순서로 수록한다. 사용자 요청에 따라 060번 이후는 `\KaTeX` 로고 항과 앞의 덧셈 기호를
제거하며, 원래 수식과 변경 이유를 함께 보존한다. 왼쪽에는 KaTeX로 표시한 TeX, 오른쪽에는 HML 변환 코드와
실제 한글 렌더링을 둔다. 미지원 입력과 재출력 오류도 남겨 검색·필터로 확인할 수 있다.
[예제별 처리 방침](math/katex-review.md)은 구현된 대응과 아직 제안 중인 처리를 구별한다.
[067 이후 미지원 사례 검토](math/katex-triage.md)는 기존 제외 방침과 판단이 필요한 항목을 구별한다.
[글자 크기 선언](math/font-sizes.md)은 10개 명령의 출처·배율·정수 SCALE 대응을 정리한다.
현재 HML 코드를 생성한 항목은 109개이고, 109개 모두 한글에서 렌더링했다.
현재 HML→TeX 재출력 오류는 없다. 렌더링 성공은 모양의 일치를 뜻하지 않는다.
원본·MIT 고지·변환 확인 방법은 [테스트 자료 안내](../tests/katex/README.md)에 있다.
비교 HTML은 2026-11-09 시점의 고정 기록이며 생성 스크립트는 저장소에 남기지 않았다.

### TeX kern 간격

`\kern{1em}`·`\kern1em`처럼 그룹 및 단일 길이 표기를 받는다.
양수는 기존 한글 공백 단위로 근사하고 음수는 0으로 정규화한다.
10pt 기준에서 em=18mu, ex=9mu로 근사하며, pt/pc/in/bp/cm/mm/dd/cc/sp도
같은 기준으로 환산한다. 글꼴별 실제 ex 높이나 정확한 물리 간격을 재현하지 않는다.
환산한 길이의 절댓값은 72mu 이하로 제한하고, 잘못된 단위·숫자는 거부한다.
기존 `\mkern`의 mu 전용 처리와 음수 거부 정책은 유지한다.

### 적분의 limits 배치

`\int\limits_0^1`은 `underover {int} _{0} ^{1}`로 옮긴다.
oint/iint/iiint/oiint/oiiint도 같은 규칙으로 처리하며, 기존
`\mathop{\int}\limits`와 동일한 한글 구조를 만든다.
limits가 없는 적분의 기본 첨자 배치는 유지한다. 일반 문자 뒤의 limits를
버리지 않으며, 현재는 적분 명령 바로 뒤에 오는 limits만 추가로 지원한다.

### 중간 구분자와 생략한 닫힘 구분자

`\left…\middle/…\right`의 middle은 보통 크기의 구분자로 옮긴다.
슬래시·세로줄 등 눈에 보이는 구분자는 유지하되 바깥 괄호 높이에 맞춰
늘이지 않는다. middle은 left/right 범위 안에서만 받으며 middle.은
보이지 않는 구분자다. `\left\{…\right.`는 `LEFT { … RIGHT .`로
옮기고 역변환한다. rule은 여전히 미지원이다. Lap 계열과 smash는 사용자 방침에 따라
겹침·폭·높이 효과를 생략하고 내부 내용을 일반 배치로 보존한다.

연산자 바로 뒤의 `\nolimits`는 한글 연산자를 그룹으로 감싸 첨자를
오른쪽에 놓는다. 예를 들어 `\sum\nolimits_{i=1}^n`은
`{sum} _{i = 1}^{n}`이 된다. 지원하는 합·곱·큰 연산자·적분·극한
명령에 적용하며, 일반 문자 뒤의 nolimits를 무시하지 않는다. 기존
mathop은 그룹과 명시적 limits가 둘 다 있을 때만 큰 연산자 배치를 사용하고,
그 밖에는 일반 기호로 정규화한다.

역변환에서는 `{sum}`의 오른쪽 첨자를 `{\sum}_{…}^{…}`처럼
연산자 그룹을 유지하여 보존한다. `\nolimits`를 추가하지 않는다.

`\nobreak`·`\allowbreak`는 자동 줄 나눔 위치를 제어하는 지시이므로
TeX→HML에서 생략한다. 해당 줄 나눔 제약은 보존하지 않으며, 수식 내용이나
강제 줄바꿈 `\\`와는 구별한다. HML→TeX에서 이 지시를 새로 추가하지 않는다.

### 아래쪽 장식 근사

underleftarrow/underrightarrow/underleftrightarrow는 underover의 아래
첨자에 방향 기호를 놓는다. undergroup은 ⌣, utilde는 SIM을 아래 첨자로 둔다. 고정된 기호를
사용하므로 본체 폭에 맞춰 늘이지 않는다. underlinesegment는 underover의
아래에 수평선 문자 ─를 놓으며 양 끝의 세로획은 생략한다. text 내부 underline은 직립 텍스트를
유지한 under로 옮긴다. 역변환은 일반적인 underset/underline으로 정규화한다.

`\hspace`·`\hspace*`도 `\kern`과 같은 길이 단위·근사·제한을 사용한다.
별표가 있는 형태의 줄 시작 공백 유지 속성은 별도로 표현하지 않는다.
역변환은 한글에 남은 공백의 `\ `·`\quad`·`\,` 표기로 정규화한다.

`\mapsfrom`과 유니코드 `↤`는 한글의 인용된 `"↤"` 기호로 옮기며,
HML→TeX에서는 `\mapsfrom`으로 출력한다. `↦`는 기존 mapsto와 같다.
첨자 안에서도 방향과 기호를 유지한다.

### 위쪽 장식

overleftarrow/Overrightarrow/overleftharpoon/overrightharpoon/overlinesegment는
underover의 위 첨자에 방향·작살표·수평선 기호를 놓는다. 고정 폭이며 선분의
양 끝 세로획은 생략한다. overgroup은 arch로, widehat/widetilde/widecheck는
hat/tilde/check로 정규화한다. 기존 overrightarrow의 vec와 overleftrightarrow의
dyad 대응은 유지한다. TeX의 SVG 장식과 폭·간격·곡선 형태가 같다는 보장은 없다.

`\breve{a}`는 미리 조합된 Unicode 문자 `ă`로 옮긴다. 단일 리터럴 문자와
기본 그리스 문자 명령의 NFC 조합이 있으면 이를 우선하고, 나머지는
`본체 + ˘`처럼 U+02D8 독립 기호를 연달아 놓는다. 결합형 U+0306과
underover는 출력하지 않는다. 예를 들어 `\breve{x}`는 `{x "˘"}`다.
단일 문자 조합표 33개는 Unicode 17.0의 canonical composition 사실이며
외부 런타임이나 새 의존성 없이 사용한다. 복합 수식과 내부에 서식 명령이 있는
본문은 인접 기호 방식으로 처리한다. 바깥의 글꼴 서식과 첨자는 유지한다.

### mathchoice와 논리적인 수식 스타일

`\mathchoice{display}{text}{script}{scriptscript}`는 TeX→HML에서 현재
스타일에 해당하는 한 분기만 출력한다. 네 분기는 모두 중괄호로 묶어야 하며,
선택되지 않은 분기도 문법·지원 범위·누적 리소스 제한을 검사한다.
기본 단독 변환기의 시작 스타일은 text다. `tex_to_hml_with_display`는 초기
display 문맥을 지정하며, 배치 변환은 각 입력의 `options.displayMode`를 따른다.
displaystyle/textstyle/scriptstyle/
scriptscriptstyle 선언과 그룹 범위를 추적한다. 이는 한글 SCALE 및 글꼴 크기
선언과 독립적이므로 display와 text는 크기 100이어도 서로 다른 분기다.
첨자는 display/text→script, script/scriptscript→scriptscript로 진행한다.
분수의 항은 display→text→script→scriptscript로 한 단계 낮아진다.
dfrac/dbinom/cfrac은 display, tfrac/tbinom은 text를 기준으로 항을 계산하고,
genfrac은 지정한 스타일을 사용한다. 중위 over/atop/brace/brack/above도
같은 규칙을 적용한다. 근호 본체는 현재 스타일, 근 지수는 scriptscript다.
위아래 주석·화살표 주석은 첨자 스타일로 처리한다. 배열·행렬은 text,
smallmatrix/subarray는 script, 정렬·gather 계열은 display를 기본으로 쓴다.

HML에는 선택된 수식만 남는다. 따라서 HML→TeX는 일반 수식을 출력하며
mathchoice나 버려진 분기를 복원하지 않는다. 059번의 내용 선택은
`D T S SS X_{S_{SS}}`가 된다. 이 처리는 한글 SCALE의 시각적 크기 재현을
보장하는 기능이 아니다.

### 중첩된 수학 글꼴

mathbb/mathcal/mathfrak은 일반 수식 인수를 읽으며, 내부의 악센트·분수·
첨자와 글꼴 범위를 유지한다. mathbb 대문자는 VecA–VecZ, 소문자 k는 𝕜,
mathcal 대문자는 스크립트 유니코드(L은 native LAPLACE), mathfrak의
영문 대소문자는 Fraktur 유니코드로 옮긴다. 해당 글꼴에 대응하지 않는
문자·숫자는 기본 수학 글꼴로 남기며 Fraktur 숫자 모양은 재현하지 않는다.
mathnormal은 현재 수학 알파벳을 기본 글꼴로 복원한다. 중첩 명령을
텍스트로 해석하거나 지원하지 않는 매크로를 실행하지 않는다.

일반 괄호·대괄호는 짝이 없어도 보이는 문자로 내보낸다. 제대로 짝지어진
괄호의 기존 그룹/이항계수 처리는 유지하며, native LEFT/RIGHT의 구조적
짝 검사는 유지한다. 012번의 `3 TIMES )`도 일반 수식으로 재출력한다.
