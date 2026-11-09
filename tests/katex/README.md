# KaTeX 시각 테스트 수식

`cases.json`은 KaTeX의 `test/screenshotter/ss_data.yaml` 128개 항목을 원래 순서로
추출한 자료입니다. 사용자 승인에 따라 060번 이후의 `\KaTeX` 로고 항과 바로 앞 덧셈 기호를
제거했습니다. 해당 항목의 `original_tex`와 `adaptation_reason`에 원문과
수정 사유를 보존합니다. 096은 사용자 승인으로 rule만 있는 마지막 행을 제거했으며,
이 역시 원문과 사유를 보존합니다. 나머지 TeX와 미지원 사례는 유지합니다.
입력별 매크로·display 설정과 나머지 원본 옵션도 보존했습니다.

- 원본 커밋: `65d1e725206dbbd278704df641491cf877413b6b`
- [원본 파일](https://github.com/KaTeX/KaTeX/blob/65d1e725206dbbd278704df641491cf877413b6b/test/screenshotter/ss_data.yaml)
- 원본 YAML SHA-256: `a5b1fb7b123dea398d04ffddef530a26af9e5f099244be2675f11aceb3f5baba`
- 원본 저작권: Copyright (c) 2013-2020 Khan Academy and other contributors
- 라이선스: [MIT 원문](LICENSE). 이 자료의 라이선스를 프로젝트의 AGPL로 바꾸지 않습니다.

[비교 HTML](../../docs/math/katex.html)은 원본 TeX를 KaTeX 0.19.1로 표시하고,
현재 변환기의 HML 코드와 한글 2020 실제 렌더링을 나란히 보여 줍니다.
모든 PNG를 포함하므로 HTML을 열 때 외부 스크립트나 한글이 필요하지 않습니다.
매크로는 KaTeX 렌더링에만 적용하며 HML 변환기는 원문을 받습니다.
pre/post HTML·원본 CSS는 실행하지 않았습니다. 외부 리소스·HTML 확장은 허용하지
않았고, 원본 noThrow 오류 사례도 엄격한 오류로 표시합니다.
변환 실패와 HML 재출력 실패를 그대로 기록하며 시각적 동일성을 자동 판정하지 않습니다.

현재 결과는 HML 생성 109/128, 한글 렌더링 109/109, HML→TeX 재출력 109/109입니다.
BinCancellation의 일반 괄호 재출력 오류도 해결하여 현재 재출력 오류는 없습니다.
첫 검토 이후 `\underbar`를 한글 `under`로 변환하도록 추가했습니다.
그 내용은 로만체로 처리합니다. AccentsText의 `\H`와 `\r`은 각각 문자 뒤에
`˝`·`˚`를 붙여 `ı˝`·`A˚`처럼 표시합니다.
여러 열의 array는 열별 정렬 지정을 무시하고 행 순서로 생성하여 행 높이를 공유합니다.
스타일 크기 선언은 `scale 70/50/100`으로 보존합니다. 현재 한글에서는
SCALE 크기 변화가 관찰되지 않았으며, 역변환은 숫자 구간에 맞는 TeX 스타일을 출력합니다.

비교 HTML은 2026-11-09 시점의 고정 기록입니다. 이를 만든 YAML 추출·TeX 렌더링·
HTML 생성 스크립트는 저장소에 남기지 않았으므로 HTML을 다시 생성할 수 없습니다.
변환기가 바뀐 뒤의 HML 변환 결과는 다음 명령으로 확인합니다.

```powershell
md2hwp-math batch tex-to-hml tests/katex/cases.json artifacts/katex-math-review/conversions.json
```

개별 SCRIPT의 한글 표시는 `tools/development/render_hancom_equation_gallery.py`로
확인합니다. 캡처 전에 기존 한글 프로세스가 없는지 확인해야 합니다.
이 확인 결과로 비교 HTML이 갱신되지는 않습니다.
기존 수식 자료와 섞지 않은 독립 검토 모음입니다.

`named-colors.json`은 CSS named color 148개와 KaTeX 0.19.1의 내장 색상 명령
56개의 이름·RGB 검증 자료입니다. CSS RGB는 브라우저의 computed style로
대조했으며, KaTeX 팔레트 자료에도 이 폴더의 MIT 고지가 적용됩니다.
Rust의 `named_colors` 테스트는 전체 이름, RGB, 대소문자와 인수 범위를 검사합니다.
