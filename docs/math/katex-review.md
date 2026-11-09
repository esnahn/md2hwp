# KaTeX 예제별 처리 방침

[비교 HTML](katex.html)의 입력은 사용자 승인에 따라 060번 이후의 `\KaTeX`
로고 항을 제거한다. 바로 앞의 덧셈 기호도 함께 제거하며, 해당 항목의
`original_tex`와 `adaptation_reason`에 원문과 수정 사유를 보존한다. 096은 사용자 승인에 따라 rule만 있는 마지막 행을 삭제하고 원문과 사유를 남긴다. 나머지 입력은 수정하지 않는다. 명령 지원과 예제 전체의
변환 성공은 구별한다. 한 예제에 다음 미지원 명령이 남아 있으면 오류로 표시한다.

기존에 제외하기로 한 016(angl 포함 전체 예제), 018(CD 도식), 038(사용자 정의
매크로)도 비교 자료의 `support_policy`에 제외 사유를 명시한다. 이는 새 제외
결정이 아니며 일반 `\boxed` 지원은 유지한다.

| 예제 | 처리 방침 |
| --- | --- |
| 013 BinomTest | `\scriptscriptstyle`은 `scale 50`, 범위 종료 시 이전 크기 복원. 한글 SCALE의 크기 효과는 확인되지 않았다. |
| 014 BoldSpacing | 일반 `*`는 인용된 한글 `"*"`. `\mathscr`의 영문 A–Z/a–z는 수학 스크립트 유니코드로 옮기고 역변환은 mathscr로 복원한다. e/g/o는 별도 Letterlike Symbols 영역의 ℯ/ℊ/ℴ를 사용한다. 중첩 수식은 일반 파서로 읽으며 대응하지 않는 숫자·그리스 문자 등은 기본 표기로 남긴다. |
| 016 Boxed | `\angl`은 보험수리 표기의 위·오른쪽 테두리다. bar+오른쪽 천장 괄호 조합은 선이 분리되어 표시되므로 미지원 유지. 일반 ANGLE로 바꾸지 않는다. |
| 017 Cases | rcases는 `LEFT . matrix {행과 셀} RIGHT }`로, 기존 cases의 반대쪽 괄호를 만든다. 행과 셀 구조를 유지하며 역변환의 숨은 왼쪽 구분자도 다시 입력받는다. |
| 018 CD | 도식 환경은 미지원 유지. |
| 019 Colors | CSS named color 148개를 ASCII 대소문자 구분 없이 받는다. textcolor의 green은 #008000이며 #0f0는 #00ff00이다. KaTeX 색상 명령 56개는 별도의 대소문자 구분 이름이며 blue는 #6495ED, red는 #DF0030, green은 #28AE7B다. blueA–E, grayA–I, kaBlue, kaGreen 등도 받는다. #RGB/#RRGGBB, `[RGB]`, `[HTML]`도 유지한다. |
| 020 ColorImplicit | 중괄호·배열 셀·LEFT/RIGHT의 범위 안에서 선언 색을 유지한다. textcolor는 인수만 칠하며, color 선언은 닫는 RIGHT 괄호도 칠한다. 예: `LEFT ( black {color {255,0,0} {red RIGHT )}}`. 색상 선언이 있는 RIGHT 범위 자체의 중위 분수·외부 줄바꿈 조합은 명시적으로 거부한다. |
| 022 Colorbox | colorbox와 fcolorbox 모두 배경색은 생략하고 한글 box 테두리를 남긴다. 테두리 색도 생략한다. |
| 023 DashesAndQuotes | 텍스트 안 it·rm·tt, texttt와 수식의 mathtt를 받는다. 백틱 두 개/작은따옴표 두 개는 “/”, 하나씩은 ‘/’로 변환한다. 고정폭에서도 따옴표 두 개는 하나의 겹따옴표로 통일한다. --/---는 일반 텍스트에서 –/—로 옮기며 고정폭 텍스트에서는 각각 남긴다. 그룹·명령 경계를 넘겨 합치지 않는다. 수식에서도 백틱 두 개는 “로 열고 같은 범위 안의 작은따옴표 두 개로 ”를 닫는다. 인용 밖의 f'' 프라임과 간격 명령은 유지한다. 원문에 이미 유니코드로 적힌 개별 따옴표를 사후 병합하지 않는다. 고정폭 영문·숫자는 수학 고정폭 유니코드로 옮긴다. 문장부호에는 별도 고정폭 글꼴을 강제하지 않는다. |
| 024 DeepFontSizing | 크기 선언을 정수 `scale`로 보존한다. 기본 100 기준 tiny 50, scriptsize 70, footnotesize 80, small 90, large 120, Large 144, LARGE 173, huge 207, Huge 249. 수식·text 내부에서 모두 범위 종료 시 이전 크기를 복원한다. [출처와 전체 표](font-sizes.md). |
| 025 DelimiterSizing | big/Big/bigg/Bigg의 고정 크기는 각각 scale 120/180/240/300이며 l/r/m 변형도 같다. 보이지 않는 점 구분자는 생략한다. Set·Braket은 각각 `LEFT { … RIGHT }`, `LEFT < … RIGHT >`로 옮긴다. 내부 세로선은 일반 구분자이며 자동 높이를 재현하지 않는다. |
| 029 Equation | equation/equation* 안 수식을 유지하고 번호를 생략한다. split은 eqalign으로 옮긴다. |
| 031 ExtensibleArrows | TeX→HML은 위·아래 주석 유무와 관계없이 `rel 화살표 {위} {아래}`를 출력한다. 없는 주석은 빈 그룹이다. 단방향 xrightarrow/xleftarrow는 rarrow/larrow, 두 줄 단방향 xRightarrow/xLeftarrow는 RARROW/LARROW로 방향을 유지한다. 양방향 xleftrightarrow는 lrarrow, xLeftrightarrow는 LRARROW, 교차 xrightleftharpoons는 EXARROW로 정규화한다. xhookrightarrow/xhookleftarrow는 hookright/hookleft, xtwoheadrightarrow/xtwoheadleftarrow는 유니코드 ↠/↞를 REL에 넣으며 역변환도 지원한다. hook·twohead는 주석이 길어져도 늘어나지 않는 고정 길이 근사다. BUILDREL 입력은 계속 읽지만 새로 생성하지 않는다. 이 목록 밖의 x명령은 계속 거부한다. |
| 032 FractionTest | `\brace`·`\brack`은 중위 atop과 양쪽 괄호로 처리한다. 일반 그룹과 overset/underset의 주석 그룹에서 같은 규칙을 사용한다. genfrac도 여섯 인자를 읽으며, 분수선 유무·괄호·스타일을 변환한다. 양수 선 두께는 한글 기본 두께로 정규화한다. above도 같은 범위의 중위 분수로 처리하며 두께를 정규화한다. 원본 전체 예제의 변환과 한글 렌더링을 확인했다. |
| 034 Gather | gather/gather*를 pile로 옮기고 번호를 생략한다. |
| 037 GreekUnicode | HML→TeX의 boldrm/rmbold는 의도상 다음 항의 굵은 직립체. TeX→HML은 rm bold 순서를 사용한다. rm/it의 모양 전환은 지속되지만 bold는 다음 항/그룹만 굵게 하며, bold {rm a}는 굵은 직립체로 내보낸다. Ω의 한글 표시 차이는 별도 교정하지 않는다. |
| 038 GroupMacros | 사용자 정의 매크로와 외부 매크로 설정은 거부한다. |
| 040 HorizontalBrackets | 지원 대상 제외. 수평 대괄호 모양에 직접 대응하는 한글 native 수식 명령이 없으며, 모양이 달라지는 OVERBRACE로 대체하지 않는다. 원문과 변환 오류를 유지한다. |
| 041 HTML | 지원 대상 제외. HTML/CSS 확장 예제이며 의도적 문법 오류가 아니다. KaTeX의 trust=false 설정에 따른 HTML 확장 거부와 HML 변환 미지원을 따로 기록하고, 원문과 변환 오류를 유지한다. |
| 043 Integrands | int/oint/iint/iiint/oiint/oiiint 바로 뒤의 limits를 underover로 옮겨 위·아래 첨자 배치를 유지한다. 기존 mathop의 limits 처리와 동일하며, limits 없는 적분의 기본 배치는 바꾸지 않는다. |
| 044 KaTeX | 지원 대상 제외. KaTeX 제품 로고 명령은 변환하지 않으며 원문과 변환 오류를 유지한다. 알 수 없는 명령 전체를 빈 문자열로 지우지 않는다. |
| 045 Kern | 양수 길이를 한글 빈칸으로 근사하고, 음수는 0으로 정규화한다. em/ex 및 물리 단위를 명목 10pt 기준으로 환산한다. 원본 045와 같은 명령을 쓰는 125의 한글 렌더링을 확인했다. |
| 046 Lap | Lap 계열은 내부 내용을 일반 배치로 변환한다. 이 전체 예제에는 아직 미지원인 phantom이 남아 있어 원문과 변환 오류를 유지한다. |
| 047 LargeRuleNumerator | 지원 대상 제외. 분자에 rule로 실제 파란 직사각형을 그리는 예제이므로 사각형을 생략하거나 크기 명령으로 취급하지 않는다. |
| 048 LaTeX | LaTeX·TeX 로고 시험은 지원 대상에서 제외한다. 로고 명령을 빈 문자열로 지우거나 일반 텍스트 내용을 삭제하지 않는다. |
| 051 LeftRightMiddle | middle의 구분자는 보통 크기의 구분자로 남긴다. 주변 LEFT/RIGHT 범위는 유지하며, middle 구분자의 자동 높이는 재현하지 않는다. |
| 052 LeftRightStyleSizing | 전체 예제는 지원 대상 제외. right. 자체는 보이지 않는 닫힘 구분자 `RIGHT .`으로 지원한다. rule은 실제 사각형을 그리므로 미지원이며 크기만의 지시자로 취급하거나 조용히 지우지 않는다. 원문과 변환 오류를 유지한다. |

TeX 명령의 의미와 예시는 [KaTeX 공식 지원 함수 목록](https://katex.org/docs/supported.html)을
참조했다. 위의 한글 대응 및 생략 정책은 이 변환기의 정책이다.

## 크기 선언의 근거

KaTeX 0.19.1의 `dist/katex.js` 9059줄은 big/Big/bigg/Bigg의 목표 높이를
1.2/1.8/2.4/3.0 em으로 정의한다. 이 수치는 **구분자의 높이**이며 본문 글꼴
크기 배율이 아니다. 6599줄의 글꼴 크기 배율에서 Huge는 2.488배다.
현재 변환기는 이를 정수 scale 249로 근사한다. SCALE의 한글 표시 효과가 없으므로
TeX의 실제 조판 크기를 재현한다고 해석하지 않는다.

색상 근거는 같은 파일 15530줄의 blue 별칭과 9434–9468줄의 rightColor 전달이다.
CSS green과 xcolor green의 값 차이를 구분하며, 이전 xcolor 값 적용을 수정했다.

CSS 148개와 KaTeX 색상 명령 56개의 이름·RGB 전체는
[색상 검증 자료](../../tests/katex/named-colors.json)에 있다. CSS 값은 Chromium의
computed style과 별도로 대조했으며 Rust 검사는 그 자료를 사용한다.
`transparent`, `currentColor`와 시스템 색은 고정된 불투명 RGB named color가
아니므로 거부한다. 팔레트 명령은 사용자 정의 매크로를 실행하는 기능이 아니다.

023번의 따옴표·대시 구분은 [KaTeX symbols의 문자 대응](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/symbols.ts)과
[Parser의 텍스트 합자 처리](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/Parser.ts)를
참고했다. TeX 백틱을 그대로 SCRIPT에 쓰면 한글의 간격 명령과 충돌하므로
보이는 따옴표 유니코드로 내보낸다. 이 처리는 악센트 명령이나 수식 프라임을 치환하지 않는다.
사용자 지정에 따라 고정폭 및 수식의 TeX 겹따옴표 표기도 한 글자의 “/”로
정규화한다. 고정폭에서 둘로 나누는 KaTeX의 기본 표시와는 이 부분이 다르다.

twohead는 `REL ↠ {위} {아래}`와 `REL ↞ {위} {아래}`로 구현한다.
화살촉 두 개와 방향, 위·아래 주석을 유지하며, 주석 폭에 맞춘 늘이기는 하지 않는다.
일반 twoheadrightarrow/twoheadleftarrow 및 유니코드 ↠/↞ 입력도 같은 기호로
정규화하고 역변환한다.

042 Includegraphics는 수식 내부 그림 삽입이므로 지원 대상에서 제외한다. 원본의 사용자 정의 매크로도 기존대로 거부하며, 비교 HTML에 제외 이유를 표시한다.

053 LimitControls는 연산자 바로 뒤의 nolimits를 그룹으로 감싼 한글 연산자에 대응한다. 예: `\sum\nolimits_i^n` → `{sum} _{i}^{n}`. 합·곱·적분 및 지원하는 큰 연산자와 극한 계열에 적용하고, 일반 문자 뒤나 독립적인 nolimits는 거부한다. limits의 위·아래 배치와 구별한다.

054 LineBreak: nobreak/allowbreak는 자동 줄 나눔 금지·허용 지시로, 변환에서는 생략한다. 수식의 글자·연산자와 강제 줄바꿈은 유지한다. hspace/hspace*는 kern과 같은 간격 근사를 사용한다. 별표의 줄 시작 공백 유지 차이는 구별하지 않는다. 전체 원본은 한 줄이 길어 페이지 이미지의 오른쪽이 잘리며, 공백은 짧은 보조 예제로도 확인한다.

055 LowerAccent의 아래 화살표 3종은 underover의 아래 첨자 기호로 옮긴다. 곡선·물결은 underover의 아래 첨자 기호로 근사하며 본체 폭에 따라 늘이지 않는다. underlinesegment는 underover의 아래에 수평선 문자 ─를 놓으며 양 끝의 세로획은 재현하지 않는다. text 내부 underline도 지원한다. 원본 전체 변환과 HML→TeX 재출력을 검사한다.

056 Mapsfrom는 mapsfrom과 직접 입력한 ↤를 같은 한글 유니코드 기호로 옮긴다. 역변환은 mapsfrom으로 정규화한다. 반대 방향 ↦는 기존 mapsto에 대응한다.

underlinesegment의 원본 끝획은 수평선을 위아래로 가로지른다. U+23B5 ⎵는 위로만 향하는 끝획이라 대응하지 않으며 사용하지 않는다. 정확히 대응하는 유니코드 기호는 아직 확인하지 못했다.

106·107 StretchyAccent: overleftarrow·Overrightarrow·위쪽 작살표·overlinesegment는 underover 위 첨자 기호로 근사한다. 선분 양 끝 세로획은 생략한다. overgroup은 arch, widehat/widetilde/widecheck는 hat/tilde/check에 대응한다. 기존 overrightarrow→vec, overleftrightarrow→dyad는 유지한다. 고정 폭 기호의 너비는 본체에 맞춰 늘이지 않는다.

060 MathDefaultFonts의 breve는 단일 문자에 Unicode NFC 조합 문자가 있으면 ă 같은 한 글자로 옮긴다. 없으면 본체 뒤에 독립 기호 ˘를 연달아 놓는다. 결합형 U+0306과 underover를 출력하지 않는다. 조합표는 Unicode 17.0의 33개 단일 문자 조합 사실을 사용한다.

059 MathChoice: 크기와 별개로 TeX의 display/text/script/scriptscript 문맥을 추적하여 해당 분기만 남긴다. 모든 분기를 검사하되 버려진 분기는 HML에 저장하지 않으며 역변환은 일반 수식이다. 기본 시작 스타일은 text이고 원본 선택 결과는 D T S SS X_{S_{SS}}다.

061·063·064·066의 수학 알파벳은 중첩 수식을 읽도록 확장한다. mathbb의 대문자와 k, mathcal 대문자, mathfrak 영문 대소문자에 해당 글꼴을 적용하고 나머지는 기본 수학 글꼴로 남긴다. mathnormal은 기본 알파벳으로 복원한다. Fraktur 숫자와 유니코드 글꼴의 세부 모양은 KaTeX 글꼴과 같지 않을 수 있다. 012번은 짝 없는 일반 괄호도 재출력하며 LEFT/RIGHT 제어의 짝 검사는 유지한다.

067 이후의 진행 내역과 남은 결정은 [미지원 사례 검토](katex-triage.md)에 정리한다.
067의 mathop은 그룹과 명시적 limits가 둘 다 있을 때만 기존 큰 연산자/underover로
옮긴다. 나머지는 내부 기호와 내용을 일반 기호로 보존한다.
071 mathscr는 중첩 수식을 지원하고 072 matrix*/pmatrix*는 l/c/r 열 정렬을 무시한다.
076의 mathbin에는 display/text 스타일에서 4mu 양쪽 간격을 native 단위로
근사하여 넣는다. script 스타일에서는 이 간격을 넣지 않는다.
081의 단순 영문 묶음 `\not{abc}`는 `not abc`로 옮겨 첫 글자의 사선을 유지한다.
083의 기존 글꼴 선언은 범위가 끝나면 복원하며 bf는 범위 전체를 굵게 한다.
sans-serif는 별도 글꼴로 표현하지 않는다. 084의 operatorname은 단일 토큰,
빈 이름, 지원되는 기호 이름과 limits를 받으며 알 수 없는 명령을 거부한다.
080의 일반 수학 그룹 안 단순 줄바꿈은 [TeX 구현 조사](line-breaks.md)에 따라
한 줄로 처리한다. 직접 지원하는 직전 glue는 제거하지만 kern은 남긴다.
명시적 행렬·정렬 환경과 바깥 수식의 줄바꿈은 유지한다. 그룹 안 별표·선택 길이는 거부한다.
084의 operatorname* 및 085의 displaylimits 연산자는 text 스타일에서 오른쪽,
display 스타일에서 위·아래에 첨자를 둔다. 명시적 limits/nolimits가 우선한다.
int는 기본 오른쪽이고 intop는 displaylimits를 따른다. substack은 script 스타일의
가운데 정렬 pile로 옮긴다. native 위·아래 첨자의 TeX 재출력은 필요한 limits를
명시하여 이 배치를 잃지 않도록 하며 nolimits는 새로 생성하지 않는다.
089는 각도 기호와 내부 내용만 표시하며 긴 밑줄은 생략한다. [검토했던 대안](phase-options.html)도 보존한다.
090 pmb는 일반 수학 bold로 정규화하며 겹쳐 찍는 조판 효과는 재현하지 않는다.
074–076의 mod 간격은 로컬 KaTeX 0.19.1 `dist/katex.js`의 bmod/mod/pmod/pod
매크로를 기준으로 기존 2.25mu 단위에 양자화한다. bmod 양쪽 5mu,
mod 앞 display 18mu/나머지 12mu와 뒤 6mu, pod/pmod 앞 display 18mu/나머지 8mu,
pmod 안 mod 뒤 6mu를 적용한다. 한글의 실제 서체 간격과 TeX의 조판 간격이
같다는 보장은 아니다. 075의 작은 글씨도 기존 SCALE 표시 한계가 남는다.

093·095는 미지원으로 유지한다. 097·101의 mathllap/mathrlap/mathclap, llap/rlap/clap, smash는
배치 효과를 생략하고 내부 내용을 일반 배치로 보존한다. smash의 t/b 옵션도 배치에는 적용하지 않는다.
phantom 계열의 숨은 내용을 표시하는 변경은 하지 않는다. 096은 원본 마지막 행이 rule만으로
구성되어 있어 해당 행을 삭제하며, 변환기 자체의 rule 미지원은 그대로다.

094의 xrightleftarrows, xrightequilibrium, xleftequilibrium은 사용자 결정에 따라
모두 native `rel EXARROW`로 옮긴다. xrightequilibrium과
xleftequilibrium에 대응하는 U+1F8D2/U+1F8D3은
[Unicode 17.0 반응 화살표 목록](https://www.unicode.org/charts/PDF/Unicode-17.0/U170-1F800.pdf)에 있지만
실제 한글 2020/HancomEQN에서는 빈 사각형으로 표시된다. 해당 Unicode 직접 입력도
`EXARROW`로 옮긴다. 주석은 REL에 유지하며 원래 화살표 모양, 비대칭 선의
길이·우세 방향은 보존하지 않는다. 역변환은 세 명령을 구별하지 않고
`\rightleftarrows`에 주석을 붙인 형태로 출력한다. 빈 사각형 증거는
artifacts/katex-math-review/089-101-native에 남고 최종 HTML에는 EXARROW 이미지를 사용한다.
