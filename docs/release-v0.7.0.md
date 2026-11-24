# md2hwp v0.7.0

Markdown 원고를 HWP·HWPX 문서로 변환합니다.
AURI 보고서 양식의 서식과 문서 구조를 구현·출력하는 것을 기준으로 합니다.
프로그램 버전은 **0.7.0**, IR은 **0.6** 그대로입니다. v0.6.0·v0.6.1에서 만든 IR 파일과 템플릿을 그대로 쓸 수 있습니다.

## 이번 버전의 변경사항

- 기본 rhwp 백엔드가 HWPX 템플릿과 HWPX 출력을 지원합니다(#12). 한글·.NET·보안 모듈 없이 HWPX 문서를 만들 수 있습니다.
- `--worker`를 생략하면 HWPX도 rhwp 백엔드로 만듭니다. 이전에는 템플릿이나 출력이 HWPX이면 한컴 백엔드를 골랐습니다.
- rhwp 라이브러리를 0.8.7에서 0.9.1로 올렸습니다.

### 버그 수정

- rhwp 백엔드 `init-template`으로 만든 템플릿에서, 쪽 아래쪽에 온 표가 한글에서 그 쪽의 각주 위에 겹쳐 그려지던 문제를 고쳤습니다. 새 템플릿의 표 원형은 '쪽 영역 안으로 제한'을 켭니다. 한컴 백엔드의 `init-template`은 한글 기본값으로 이 설정이 켜져 있었고 이제 명시적으로 켭니다. 기본 템플릿 `template.hwp`의 표는 글자처럼 취급하는 표라 이 설정과 무관합니다.
- 이전 버전의 rhwp `init-template`으로 만든 템플릿을 계속 쓰려면, 한글에서 '글자처럼 취급'이 꺼진 표 원형의 개체 속성에서 '쪽 영역 안으로 제한'을 켜거나 템플릿을 새로 만드십시오.
- 한컴 백엔드 `init-template`으로 만든 HWPX 템플릿을 한컴 백엔드로 변환할 때, HWPX→HWP 정규화 검사에서 원형 높이·캡션 폭 차이로 실패할 때가 있던 문제를 고쳤습니다. 이 템플릿은 한글이 배치하지 않은 값을 저장하는데, 검사가 한글이 배치를 마치기 전에 원본을 읽으면 실패했습니다. 이제 배치를 마친 뒤 비교합니다. 기본 rhwp 백엔드도 이 템플릿을 변환합니다.

### HWPX (rhwp 백엔드)

- 출력 확장자가 `.hwpx`이면 HWPX로, `.hwp`이면 HWP로 저장합니다. HWP·HWPX 템플릿과 출력은 서로 조합할 수 있습니다. 기본 템플릿과 출력은 계속 HWP입니다.
- `init-template`과 `validate-template`도 `.hwpx` 경로를 받습니다.
- 문서는 HWP로 조합합니다. HWPX 템플릿은 먼저 HWP로 바꿔 원본과 대조하고, HWPX 출력은 HWP 검증을 마친 뒤 HWPX로 내보내 다시 열어 그 HWP와 비교합니다. 어느 단계에서 실패해도 기존 결과 파일은 그대로 남습니다.
- 처리 순서와 검증 범위는 [rhwp HWPX 템플릿·출력](https://github.com/esnahn/md2hwp/blob/v0.7.0/docs/rhwp-hwpx-2026-11-12.md)에 있습니다.

```powershell
.\md2hwp.exe 원고.md 결과.hwpx --template 내템플릿.hwpx
.\md2hwp.exe init-template 새템플릿.hwpx
.\md2hwp.exe validate-template 새템플릿.hwpx
```

이전처럼 한컴 백엔드로 HWPX를 만들려면 백엔드를 지정합니다.

```powershell
.\md2hwp.exe 원고.md 결과.hwpx --template 내템플릿.hwpx --worker .\md2hwp-hancom.exe
```

### rhwp 0.9.1

- 0.9.1은 HWPX 템플릿을 HWP로 바꿀 때 바탕쪽(양쪽·짝수·홀수) 종류가 바뀌던 문제를 고친 판입니다. md2hwp는 바뀐 종류를 고치지 않고, 종류가 원본과 다르면 변환을 멈춥니다.
- rhwp를 올리는 각 단계에서 기본 템플릿 예제의 한글 2020 쪽 이미지를 이전 단계와 비교해 차이가 없었습니다. 전체 기능 예제를 `init-template` 템플릿으로 만든 결과는 v0.6.1과 같이 8쪽이며 1–7쪽이 픽셀까지 같습니다.
- SVG를 PNG로 넣을 때 파일이 작아졌습니다(resvg 0.47). matplotlib 갤러리의 EMF 결과는 바이트까지 그대로입니다.

## 실행

`md2hwp-v0.7.0-windows-x64.zip`을 풀고 같은 폴더에서 다음 명령을 실행합니다.

```powershell
.\md2hwp.exe setup-pandoc
.\md2hwp.exe path\to\원고.md
```

원고 옆에 `원고.ir.json`과 `원고.output.hwp`가 생성됩니다.

기존 한컴 백엔드를 지정하려면 다음 명령을 실행합니다.

```powershell
.\md2hwp.exe 원고.md --worker .\md2hwp-hancom.exe
```

## 필요한 환경

Windows x64 환경에서 실행됩니다. 기본 rhwp 백엔드를 통한 HWP·HWPX 문서 생성에는 Pandoc과 템플릿이 쓰는 Windows TTF/OTF 글꼴 외에 추가 환경이 필요하지 않습니다.
한컴 백엔드, `hwp2pdf.exe`, `hwpeqrecalc.exe`에는 한컴 한글, .NET 10 x64 런타임과 등록된 한글 보안 모듈이 추가로 필요합니다.
보안 모듈은 [한컴 공식 안내](https://developer.hancom.com/hwpautomation)에 따라 설치·등록하십시오.

> [!TIP]
> 저장소 소스코드를 다운받으면 [설치 스크립트](https://github.com/esnahn/md2hwp/tree/main/dependencies/install-hancom-security-module.ps1)를 통해 자동 설치가 가능합니다.
> [lock.json](https://github.com/esnahn/md2hwp/tree/main/dependencies/lock.json)을 같은 폴더에 놓은 뒤,
> 한글을 모두 닫고 그 폴더에서 실행할 수 있습니다.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-hancom-security-module.ps1
> ```
>
> 스크립트는 다운로드 파일의 해시를 확인하고 DLL을 설치·등록합니다. 설치된 DLL은 이후에도 그 위치에 유지하십시오.
> 실행용 배포 ZIP에는 스크립트와 `lock.json`이 포함되지 않으며 프로그램 실행에 `lock.json`은 필요하지 않습니다.

## 이전 버전 사용자

- v0.6.0·v0.6.1 사용자는 IR 파일과 템플릿을 그대로 쓸 수 있습니다.
- HWPX 템플릿이나 HWPX 출력을 쓰던 경우 이제 rhwp 백엔드가 변환합니다. 결과를 이전과 같은 백엔드로 만들려면 `--worker .\md2hwp-hancom.exe`를 지정하십시오. 두 백엔드의 결과가 모든 템플릿에서 같다고 보장하지는 않습니다.
- v0.5.0 이하에서 올라오는 경우 IR 재생성과 템플릿 선언 반영이 필요합니다. [v0.6.0 릴리스 노트](https://github.com/esnahn/md2hwp/blob/v0.7.0/docs/release-v0.6.0.md)를 참고하십시오.
- v0.6.1의 변경사항(SVG 그림, `svg2emf.exe`, HTML 주석, `setup-pandoc` 수정)은 [v0.6.1 릴리스 노트](https://github.com/esnahn/md2hwp/blob/v0.7.0/docs/release-v0.6.1.md)에 있습니다.

## 배포 파일 구성

Release 빌드 ZIP은 `md2hwp.exe`, `md2hwp-rhwp.exe`, `md2hwp-hancom.exe`, `hwp2pdf.exe`, `hwpeqrecalc.exe`, `svg2emf.exe` 여섯 실행 파일을 포함합니다. ZIP에는 기본 HWP 템플릿, README, 원고 작성 안내 두 문서, PowerShell 자동완성과 `LICENSE`, `THIRD-PARTY-NOTICES.txt`, `licenses/`도 포함합니다.
프로젝트 코드는 AGPL-3.0-only이며 제3자 자료는 각 원저작자의 조건을 따릅니다.

`md2hwp-v0.7.0-windows-x64.zip` SHA-256:
```
7C91668FEAFC0CD4B48E8F8BA7ABD996C3F903031080AD30982AE31AB8FF9EBB
```

## 지원 범위와 알려진 제한

- rhwp HWPX는 한글이나 rhwp가 저장한 HWPX 템플릿으로 확인했습니다. 다른 프로그램이 저장한 HWPX 템플릿, 한글 외 프로그램에서의 HWPX 표시는 확인하지 않았습니다.
- rhwp의 수식 크기는 추정값입니다. 한글과 정확히 같은 크기가 필요하면 `hwpeqrecalc.exe`를 사용하십시오. 목록 항목 안의 별도 문단 수식, 그리고 제목·표 셀·캡션·출처의 수식은 지원하지 않습니다.
- 저장·재열기 구조 검증만으로 넘침·겹침이나 표시 번호를 판정할 수 없으므로 생성된 문서의 배치를 확인하십시오.
