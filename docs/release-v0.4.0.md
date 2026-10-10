# md2hwp v0.4.0

Markdown 원고를 HWP 문서로 변환합니다.
AURI 보고서 양식의 서식과 문서 구조를 구현·출력하는 것을 기준으로 합니다.
IR 버전은 **0.4**입니다.

## 이번 버전의 변경 사항

- 그림·글상자·표의 출처와 하단 설명을 여러 문단으로 붙일 수 있습니다. `출처:`, `주.1:`, `단위:`, `Source:` 등 지원 접두어의 표기와 순서를 보존합니다.
- JPG·JPEG·EMF 그림을 추가로 지원합니다. 그림 파일은 HWP에 내장하며, EMF는 물리적 프레임 비율로 크기를 정합니다.
- 표 번호 상호참조를 지원합니다. 그림·표 캡션과 개체의 하단 설명 안에서도 번호 상호참조를 사용할 수 있습니다.
- 장·절 번호를 원고 순서로 계산합니다. 헤딩 1~6단계별 참조 양식을 템플릿에서 지정하고, 제목 블록과 소속 절 목록에도 자신과 상위 단계의 번호를 넣을 수 있습니다. 헤딩 참조는 고정 텍스트이며 HWP 편집 후 자동 갱신되지 않습니다.
- 글상자·그림·표 앞뒤의 템플릿 빈 문단을 서식과 쪽·단 나눔과 함께 복제합니다.
- 문서 XML을 먼저 구성하는 방식으로 생성 속도를 개선했습니다. 최종 저장·재열기 검증은 유지합니다.
- 기존 HWP를 PDF로 출력하는 별도 실행 파일 `hwp2pdf.exe`를 추가했습니다. 한글 자체 `PrintToPDFEx` 액션과 `GraphicQuality=100`을 사용합니다.

## 실행

`md2hwp-v0.4.0-windows-x64.zip`을 풀고 동봉 파일을 같은 폴더에 두십시오.
ZIP에는 `md2hwp.exe`, `md2hwp-backend.exe`, `hwp2pdf.exe`, `template.hwp`,
`README.md`, 원고 작성 지침 `AGENTS.md`와 `README-MANUSCRIPT.md`가 들어 있습니다.

```powershell
.\md2hwp.exe setup-pandoc
.\md2hwp.exe 원고.md
.\hwp2pdf.exe 원고.output.hwp
```

원고 옆에 `원고.ir.json`, `원고.output.hwp`가 생성되며 PDF 명령은 `원고.output.pdf`를 만듭니다.
다른 템플릿은 `--template 내템플릿.hwp`, 출력 경로는 `--output`으로 지정할 수 있습니다.

동봉 템플릿을 사용할 때는 원고 맨 앞에 문서 제목을 적으십시오. 문서 제목과 첫 장 제목은 별개입니다.

```yaml
---
title: 보고서 제목
---
```

`hwp2pdf.exe`는 단독으로 옮겨 사용할 수도 있으며 템플릿·Pandoc·별도 백엔드 EXE는 필요하지 않습니다.
원본 HWP는 저장하지 않고, PDF 완료 확인과 한글 정상 종료 후 결과 파일을 교체합니다.

## 필요한 환경

Windows x64, 한글, .NET 10 x64 런타임, 설치·등록된 한글 보안 모듈이 필요합니다.
기준 검증 환경은 한글 2020입니다. 실행 중인 한글 문서는 먼저 닫아 주십시오.
Markdown 변환에 필요한 Pandoc은 위 `setup-pandoc` 명령으로 준비합니다.

보안 모듈은 [한컴 공식 한글 오토메이션 페이지](https://developer.hancom.com/hwpautomation)에서
**보안모듈(Automation).zip**을 내려받아 동봉된 안내에 따라 현재 사용자 계정에 등록하십시오.
등록 이름은 `FilePathCheckerModuleExample`이며 `REG_SZ` 값에 DLL의 절대 경로를 지정합니다.
등록된 DLL은 해당 위치에 유지하십시오. 프로그램은 보안 모듈을 자동으로 설치하거나 등록하지 않습니다.

> [!TIP]
> 저장소의 [설치 스크립트](https://github.com/esnahn/md2hwp/blob/v0.4.0/dependencies/install-hancom-security-module.ps1)와
> [lock.json](https://github.com/esnahn/md2hwp/blob/v0.4.0/dependencies/lock.json)을 같은 폴더에 저장한 뒤,
> 한글을 모두 닫고 그 폴더에서 실행할 수 있습니다.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-hancom-security-module.ps1
> ```
>
> 스크립트는 다운로드 파일의 해시를 확인하고 DLL을 설치·등록합니다. 설치된 DLL은 이후에도 그 위치에 유지하십시오.
> 실행용 배포 ZIP에는 스크립트와 `lock.json`이 포함되지 않으며 프로그램 실행에 외부 `lock.json`은 필요하지 않습니다.

## 이전 버전 사용자

IR 0.3은 지원하지 않으므로 Markdown 원고에서 IR을 다시 생성하십시오.
사용자 템플릿도 IR 0.4의 다중 하단 설명과 현재 참조 선언을 반영해야 합니다.
출처 샘플에는 접두어·내용 슬롯이 필요하며, 글상자 범위 이름은 `code`, 그림 범위 이름은 `figure`입니다.
헤딩 참조는 헤딩 1~6단계별 한 문단 범위, 표 번호 참조는 별도 샘플이 필요합니다.

기존 템플릿을 백업하고 [템플릿 안내](https://github.com/esnahn/md2hwp/blob/v0.4.0/docs/templates.md)에 따라
수정하거나 `md2hwp init-template 새템플릿.hwp`로 별도 파일을 생성하십시오.
사용자 템플릿은 자동으로 갱신하지 않습니다.

## PDF 화질과 지원 범위

PNG·JPG는 **최종 삽입 크기를 기준으로 500dpi로 저장**하는 것을 권장합니다.
한글 2020의 현재 출력 설정에서 500dpi 입력은 픽셀 크기를 유지했고 510dpi 이상은 약 500dpi로 축소됐습니다.
EMF는 PDF에서 약 600dpi로 래스터화됩니다. Flate 압축이어도 원본 픽셀 일치나 벡터 보존을 보장하지 않습니다.

HWPX·RST 직접 입력, 쪽 번호 상호참조, 캡션·하단 설명 안의 각주는 지원하지 않습니다.
일반 링크는 클릭 가능한 연결 없이 표시문으로 출력합니다.
변환 후 긴 제목·표·글상자의 잘림이나 겹침과 각주 표시 번호를 확인하십시오.
동봉 템플릿에 원래 있는 첫 빈 페이지도 유지됩니다.

자세한 내용은 [설치·실행](https://github.com/esnahn/md2hwp/blob/v0.4.0/docs/usage.md),
[원고 작성](https://github.com/esnahn/md2hwp/blob/v0.4.0/docs/manuscript/README-MANUSCRIPT.md),
[PDF 출력](https://github.com/esnahn/md2hwp/blob/v0.4.0/docs/hwp2pdf.md)을 참고하십시오.
