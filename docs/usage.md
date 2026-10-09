# 설치와 실행

## 사용자 환경

- 한글 설치 및 로그인된 대화형 Windows 세션이 필요합니다. 기준 환경은
  한글 2020 HWP 11.0.0.9136입니다. 실행 중인 한글 문서는 먼저 닫습니다.
- .NET 10 x64 런타임이 필요합니다. Rust는 실행 전에 확인하고 없으면
  [공식 설치 페이지](https://dotnet.microsoft.com/ko-kr/download/dotnet/10.0)를 안내합니다.
  백엔드 직접 실행 시 별도 설치 런타임은 `DOTNET_ROOT_X64`로 지정합니다.
- 한글 보안 모듈의 설치·등록이 필요합니다. 아래 [설치 안내](#한글-보안-모듈-설치등록)를
  따라 현재 사용자 계정에 등록하십시오. 앱은 RegisterModule 성공을 요구합니다.
- Pandoc은 `md2hwp setup-pandoc`으로 공식 배포본을 다운로드할 수 있습니다.
  `%LOCALAPPDATA%\md2hwp\pandoc`에 원본 ZIP·문서·저작권 고지를 보관합니다.
  탐색 순서는 `--pandoc`, 관리 다운로드, PATH입니다. 변환 중 자동 설치는 하지 않습니다.
  다운로드 명령은 빌드 시 lock의 버전을 우선하고, 없으면 공식 최신 안정판을 확인합니다.
  버전 번호 대신 Pandoc JSON 계약의 호환성을 검사합니다.

### 한글 보안 모듈 설치·등록

[한컴 공식 한글 오토메이션 페이지](https://developer.hancom.com/hwpautomation)에서
**보안모듈(Automation).zip**을 내려받아 압축을 풀고, 동봉된 안내에 따라 현재 사용자
계정에 등록하십시오. DLL을 둘 폴더는 자유롭게 선택할 수 있습니다.

- 레지스트리 키: `HKEY_CURRENT_USER\Software\HNC\HwpAutomation\Modules`
- 값 이름: `FilePathCheckerModuleExample`
- 값 형식: `REG_SZ`
- 값 내용: DLL의 절대 경로

등록 후 DLL은 해당 위치에 유지하십시오. DLL을 옮기면 등록 경로도 수정해야 합니다.
프로그램은 보안 모듈을 자동으로 설치하거나 등록하지 않습니다. 등록이 없거나 등록된
DLL을 찾을 수 없으면 안내와 함께 HWP 생성을 중단하고 기존 결과 파일을 보존합니다.

> [!TIP]
> 다운로드·설치·등록을 돕는 [설치 스크립트](../dependencies/install-hancom-security-module.ps1)를
> 사용할 수 있습니다. 저장소 루트에서 한글을 모두 닫고 다음 명령을 실행하십시오.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\dependencies\install-hancom-security-module.ps1
> ```
>
> 소스 ZIP 전체를 받을 필요는 없습니다. 스크립트와 [lock.json](../dependencies/lock.json)을
> 같은 폴더에 저장했다면, 그 폴더에서 다음 명령을 실행하십시오.
>
> ```powershell
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-hancom-security-module.ps1
> ```
>
> 스크립트는 옆의 `lock.json`에서 다운로드 URL과 SHA-256 해시를 읽어 파일을 검증하고,
> 같은 폴더에 DLL을 설치하여 현재 사용자 레지스트리에 등록합니다. 마지막으로 한글의
> `RegisterModule` 성공 여부를 확인합니다. 등록된 DLL은 이후에도 그 위치에 유지하십시오.
>
> 실행용 배포 ZIP에는 스크립트와 `lock.json`이 포함되지 않습니다. 이들은 설치 보조 도구이며,
> 프로그램 실행에 외부 `lock.json`은 필요하지 않습니다. 프로그램이 스크립트를 자동으로
> 실행하지도 않습니다.

IR 0.4의 YAML 메타데이터와 템플릿 태그는 [메타데이터 문서](yaml-variables.md)에 정리했습니다.

## 실행

```powershell
.\md2hwp.exe 원고.md
.\md2hwp.exe 원고.md 결과.hwp
.\md2hwp.exe 원고.md --output 결과.hwp --template 내템플릿.hwp
.\md2hwp-backend.exe 원고.ir.json
.\md2hwp-backend.exe 원고.ir.json --output 결과.hwp --template 내템플릿.hwp
```

첫 명령은 원고 옆에 `원고.ir.json`, `원고.output.hwp`를 생성합니다.
백엔드는 기존 IR에서 HWP만 생성합니다. 두 번째 인자는 선택적인 HWP 출력 경로입니다.
검증된 IR은 교체하며, HWP는 생성·재열기 검증 성공 후 기존 결과를 교체합니다.
서식 보존 검사는 저장 중 바뀔 수 있는 문단·글자 모양의 내부 번호 대신 실제 설정을 비교합니다.
한글이 병합하는 동일 서식의 텍스트 조각과 명세 기본값 `TextFlow="BothSides"`의 생략도
동등하게 비교합니다. 실제 문자·공백·줄바꿈·탭·서식·개체 크기 변경은 계속 오류로 처리합니다.
작업 문서의 박스 원형 보존 검사에도 같은 비교를 적용합니다. 한글이 다시 계산하는 박스 표의
양수 높이는 비교에서 제외하며, 너비·셀 높이·위치·내용·서식은 계속 검증합니다.
실패한 HWP 생성은 이전 HWP를 보존합니다. 원고와 템플릿은 덮어쓰지 않습니다.
간편 실행의 이미지 경로는 원고/IR 폴더 안에서 해석합니다.
간편 실행은 출력 경로를 위치 인자 또는 `--output`으로 받습니다. 두 방식을 중복 지정할 수는 없습니다.
Rust 간편 실행은 `--template`, `--worker`, `--dotnet`도 받으며, 백엔드는 `--template`을 받습니다.
변환 작업별 목록은 기본적으로 숨깁니다. 간편 실행이나 `ir2hwp`에 `--verbose`를 붙이면 표시합니다. 오류와 최종 결과는 항상 출력합니다.
옵션 입력 순서는 자유롭습니다. 명시한 경로는 실행한 작업 폴더 기준이며,
이미지의 기준 폴더만 원고/IR 폴더로 유지됩니다.

`md2hwp --version`은 앱 자체와 실제로 사용할 백엔드의 버전을 함께 표시합니다.
`md2hwp-backend --version`은 백엔드 자체 버전만 표시합니다.

```powershell
.\md2hwp.exe --version
.\md2hwp.exe --version --worker 다른폴더\md2hwp-backend.exe --dotnet 경로\dotnet.exe
.\md2hwp-backend.exe --version
```

v0.4.0의 첫 명령 출력은 다음과 같습니다.

```text
md2hwp 0.4.0
md2hwp-backend 0.4.0
```

`--worker`를 생략하면 md2hwp.exe 옆의 백엔드를 확인합니다. `--worker`와 `--dotnet`은
현재 작업 폴더 기준이며 옵션 순서는 자유롭습니다. 백엔드 확인에는 .NET 런타임이 필요하며,
한글이나 보안 모듈은 사용하지 않습니다. 백엔드 확인에 실패하면 앱 자체 버전을 표시하고
오류와 함께 종료 코드 1을 반환합니다. 버전이 달라도 확인한 값을 그대로 표시합니다.

기존 옵션 방식도 사용할 수 있습니다.

```powershell
.\md2hwp.exe md2ir 원고.md --force
.\md2hwp.exe ir2hwp --ir 원고.ir.json --output 원고.output.hwp
.\md2hwp-backend.exe ir2hwp --ir 원고.ir.json --output 원고.output.hwp
```

`md2ir`도 `md2ir 원고.md [결과.ir.json]`처럼 위치 인자를 받으며, 기존 `--input`·`--output` 방식도 지원합니다. 같은 항목의 중복 지정은 거부합니다.

`md2ir`에서 `--output`을 생략하면 입력 옆의 `원고.ir.json`, `--from`을 생략하면 `commonmark`를 사용합니다. Pandoc JSON 입력은 `--from pandoc-json`으로 지정합니다.

`--template`과 Rust의 `--worker`를 생략하면 각 EXE 옆의 `template.hwp`,
`md2hwp-backend.exe`를 찾습니다. 옵션 방식의 이미지 기준 폴더는 작업 폴더입니다.
현재 IR `0.4`, 정규화 규칙과 스키마는 실행 파일에 포함됩니다.

입력 IR과 템플릿의 IR 버전은 프로그램의 현재 IR 버전과 정확히 같아야 합니다.
변환 규칙과 템플릿에는 별도의 계약 버전을 두지 않습니다.

개발 참고 자료의 파일명·SHA-256·출처는 [reference/README.md](../reference/README.md)에 기록합니다.

IR 0.2 입력은 원고에서 다시 생성하고, 기존 템플릿은 새 선언을 직접 채택하거나 별도 파일로 재생성하십시오. 사용자 템플릿은 자동 갱신하지 않습니다.

[README로 돌아가기](../README.md)
