# HWPX 템플릿과 출력

> **미출시 개발 기능:** HWPX 템플릿과 HWPX 출력은 개발 버전에서 지원합니다.
> 공개된 v0.4.3 실행 파일에는 포함되지 않습니다.

HWPX는 템플릿과 변환 결과 형식으로 사용할 수 있습니다. 원고 직접 입력은 Markdown 또는
Pandoc JSON이며, HWPX·RST 원고 입력은 지원하지 않습니다. 이 기능은 IR 0.4를 바꾸지 않으며
별도의 `--format` 옵션도 추가하지 않습니다.

## 변환

출력 경로의 확장자로 저장 형식을 정하고, `--template`에는 사용할 템플릿을 지정합니다.
HWP와 HWPX 템플릿·출력은 서로 조합할 수 있습니다.

```powershell
.\md2hwp.exe 원고.md 결과.hwpx --template 내템플릿.hwpx
.\md2hwp.exe 원고.md --output 결과.hwpx --template 내템플릿.hwp
.\md2hwp-backend.exe 원고.ir.json --output 결과.hwpx --template 내템플릿.hwpx
```

위치 인자와 `--output`은 기존처럼 출력 경로를 받습니다. 기본값은 바뀌지 않습니다.
템플릿을 지정하지 않으면 EXE 옆의 `template.hwp`를, 출력 경로를 생략하면 원고 옆의
`원고.output.hwp`를 사용합니다. HWPX를 원할 때는 출력 경로를 `.hwpx`로 지정하십시오.

결과는 HWPX로 저장한 뒤 다시 열어 최종 엄격 검증을 통과해야 기존 출력 파일을 교체합니다.
실패하면 이전 결과를 보존합니다. 한글 2020의 HWPX 저장에서 생기는 변환행렬 단정밀도
반올림과 그림 데이터 ID 재배치는 비교용 XML에서만 정규화합니다. 그림 참조는 실제 데이터에
연결해 비교하고, 내장 데이터 바이트·내용·서식·개체 구조 검사는 유지합니다.

## 템플릿 만들기와 검사

`init-template`의 출력 경로를 `.hwpx`로 지정하면 네이티브 HWPX 템플릿을 만듭니다.
`validate-template`도 명시한 `.hwpx` 템플릿을 검사합니다. 기본 템플릿 자동 선택은 계속
`template.hwp`입니다.

```powershell
.\md2hwp.exe init-template 새템플릿.hwpx
.\md2hwp-backend.exe init-template --output 새템플릿.hwpx
.\md2hwp.exe validate-template 새템플릿.hwpx
```

`hwp2pdf.exe` 입력은 HWP 문서로 제한됩니다. 이 기능은 HWPX에서 PDF로 변환하는 기능을
추가하지 않습니다. [설치·실행 안내](usage.md)와 [템플릿 안내](templates.md)도 참고하십시오.


## 개발 검증 기록 — 2026-10-19

한글 2020 HWP 11.0.0.9136에서 다음을 확인했습니다.

- HWPX `init-template`, 생성한 템플릿과 기존 HWP를 변환한 HWPX 템플릿의 검사.
- [전체 기능 원고](../examples/all-features/)의 HWP/HWPX 템플릿 × HWP/HWPX 출력 네 조합.
  모든 결과가 저장·재열기 구조·서식·내장 데이터 검증을 통과했으며 원본 템플릿은 보존됐습니다.
- 네 조합의 결과는 각각 13쪽이고, 네이티브 300 DPI 페이지 이미지가 13쪽 모두 픽셀 단위로 일치했습니다.
- 별도 작은 원고의 Debug XML·Debug legacy COM·Release XML 결과도 각각 4쪽이며 모든 쪽의 픽셀이 일치했습니다.
- HWPX 출력 13쪽의 모음 이미지와 2·8쪽을 직접 확인해 제목, 본문, 각주 번호·본문, 표 캡션·출처의 배치를 검토했습니다.
- 메타데이터 오류로 생성이 실패한 경우 기존 HWPX 결과의 해시가 유지되고 임시 문서와 COM이 정리됐습니다.
- Rust 전체 테스트·Clippy와 Debug/Release 백엔드 계약 테스트를 통과했습니다.

결과 HWP/HWPX, 전후 XML, 페이지 이미지와 `pixel-comparison.json`, `verification.json`은
개발 환경의 `artifacts/hwpx-support/`에 보관했습니다. 이 경로는 Git 추적·배포 대상이 아닙니다.
픽셀 비교는 같은 한글 엔진에서 이 원고를 비교한 결과이며, 다른 뷰어의 표시나 모든 원고의 배치를 보장하지 않습니다.
