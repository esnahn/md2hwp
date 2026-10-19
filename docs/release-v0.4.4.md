# md2hwp v0.4.4

HWPX 템플릿과 HWPX 출력을 지원합니다. IR과 Markdown 원고 계약은 **0.4** 그대로입니다.

## 변경 사항

- 템플릿과 출력 경로의 `.hwp`·`.hwpx` 확장자로 각각 네이티브 형식을 선택합니다. HWP/HWPX 템플릿과 출력을 서로 조합할 수 있으며 별도의 `--format` 옵션은 없습니다.
- `init-template 새템플릿.hwpx`로 네이티브 HWPX 템플릿을 만들고, `validate-template 새템플릿.hwpx`로 검사할 수 있습니다.
- 기본 템플릿은 EXE 옆 `template.hwp`, 기본 출력은 원고 옆 `원고.output.hwp`입니다. HWPX 출력도 저장·재열기와 최종 엄격 검증이 성공한 뒤 기존 결과를 교체하며, 실패하면 이전 결과를 보존합니다.
- 원고 직접 입력은 계속 Markdown 또는 Pandoc JSON입니다. HWPX·RST 원고 입력과 `hwp2pdf.exe`의 HWPX 입력은 지원하지 않습니다.

```powershell
.\md2hwp.exe 원고.md 결과.hwpx --template 내템플릿.hwpx
.\md2hwp.exe 원고.md --output 결과.hwpx --template 내템플릿.hwp
.\md2hwp.exe init-template 새템플릿.hwpx
.\md2hwp.exe validate-template 새템플릿.hwpx
```

세부 범위와 저장·검증 방식은 [HWPX 안내](hwpx.md), 설치와 옵션은 [사용법](usage.md)을 참고하십시오.

## 검증 기록

[2026-10-19 개발 검증 기록](hwpx.md#개발-검증-기록--2026-10-19)에서 한글 2020 HWP 11.0.0.9136의 다음 결과를 확인했습니다.

- 전체 기능 원고의 HWP/HWPX 템플릿 × HWP/HWPX 출력 네 조합은 저장·재열기 구조·서식·내장 데이터 검증을 통과했습니다. 각 결과는 13쪽이며 네이티브 300 DPI 페이지 이미지가 모두 픽셀 단위로 일치했습니다.
- 별도 작은 원고의 Debug XML·Debug legacy COM·Release XML 결과는 각각 4쪽이며 모든 쪽의 픽셀이 일치했습니다.
- HWPX 결과의 모음 이미지와 2·8쪽에서 제목·본문·각주·표 캡션·출처 배치를 검토했습니다. 실패 시 기존 HWPX 결과 보존과 임시 문서·COM 정리도 확인했습니다.

이는 기록된 예제와 같은 한글 엔진에서의 결과이며, 모든 원고의 배치나 다른 뷰어의 표시를 보장하지 않습니다.

## 실행 파일과 대응 소스

실행용 ZIP에는 실행 파일 세 개, 기본 HWP 템플릿, README, 원고 작성 문서 두 개와
`LICENSE`, `THIRD-PARTY-NOTICES.txt`, `licenses/`가 들어 있습니다.
프로젝트 코드는 AGPL-3.0-only이며 제3자 자료는 각 원저작자의 조건을 따릅니다.

실행 파일과 정확히 대응하는 소스는 동일한 태그의
[v0.4.4 자동 소스 다운로드](https://github.com/esnahn/md2hwp/archive/refs/tags/v0.4.4.zip)로 제공합니다.
의존성 소스와 도구 체인 출처는 `THIRD-PARTY-NOTICES.txt`와 `licenses/README.md`에 기록돼 있습니다.
