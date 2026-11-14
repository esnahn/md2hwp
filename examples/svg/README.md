# SVG 검증 예제

`svg.md`는 EMF로 바뀌는 `vector.svg`(두 번 사용)와 PNG로 바뀌는 `gradient.svg`를 넣습니다.
두 그림은 이 저장소에서 직접 작성한 자료입니다.

```powershell
.\target\release\md2hwp.exe .\examples\svg\svg.md
```

원고 옆에 `svg.ir.json`, `vector.svg.emf`, `gradient.svg.png`, `svg.output.hwp`가 생성됩니다.
변환 규칙과 검증 기록은 [SVG 그림](../../docs/svg-figures.md)에 있습니다.
PNG는 설치된 글꼴(맑은 고딕)에 따라 바이트가 달라질 수 있습니다.
