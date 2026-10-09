# EMF 검증 예제

`emf.md`는 일반 EMF·EMF+ Dual·EMF+ Only를 두 번씩 삽입합니다.
장 번호 재시작, 같은 원본의 재사용, 다중 출처, 본문·출처·각주의 상호참조를 확인합니다.
세 그림은 이 저장소의 `generate.ps1`로 만든 재현용 자료이며 외부에서 가져온 그림이 아닙니다.

```powershell
.\target\release\md2hwp.exe .\examples\emf\emf.md
```

원고 옆에 `emf.ir.json`과 `emf.output.hwp`가 생성됩니다.
실행 결과와 그림의 기준 해시는 [지원·검증 기록](../../docs/emf-figures.md)에 있습니다.

다른 폴더에 시험 그림과 GDI+ 기준 PNG를 다시 만들려면 Windows에서 실행하십시오.

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\examples\emf\generate.ps1 -Directory .\artifacts\emf-fixtures
```

맑은 고딕을 사용하며 EMF 기록 장치에 따라 파일 헤더와 바이트가 달라질 수 있습니다.
위 명령은 원고나 기본 템플릿을 수정하지 않습니다.
