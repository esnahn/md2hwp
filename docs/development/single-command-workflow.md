# 원고에서 HWP까지 한 명령으로 실행

`apps/md2hwp/convert.ps1`은 기존 Rust `md2ir`와 C# `render-tagged`를 순서대로
실행하는 조사용 진입점이다. 파싱·IR 검증·템플릿 바인딩·문서 생성 구현은 기존
각 계층에 남는다. 외부 프로필 JSON이나 새 작업자 프로토콜을 도입하지 않는다.

## 실행

잠긴 Rust·Pandoc·.NET 도구와 등록된 한글 보안 모듈이 필요하다.
[환경 설정](environment.md)을 마친 로그인 사용자 세션에서 한글 문서를 모두
닫고 실행한다. PowerShell 7이 빌드와 원고 변환을 실행하며, 한글 작업은 지정된
Windows PowerShell 5.1 x64 STA 자식 프로세스에서 창을 숨겨 실행한다.

실행 시 Rust가 설치된 **.NET 10 런타임 x64**를 먼저 확인한다. 없으면 설치 URL을
안내하고 중단한다. 저장소 로컬 SDK의 런타임을 쓸 때는 아래 명령에
`-RuntimeHostPath .\.local\dependencies\dotnet\10.0.400\dotnet.exe`를 추가한다.
배포 시 런타임은 포함하지 않는다. [검사·실행 계약](dotnet-runtime-launch.md) 참고.

C#은 `FrameworkDependent` 게시 프로필로 단일 EXE를 만든다. DLL과 실행 설정
JSON은 EXE에 포함되므로 별도 배포하지 않는다. `-SkipBuild`도 해당 구성의
게시된 EXE가 있어야 한다. 스키마는 Rust에 내장돼 있다. 태그 템플릿 생성은
lock 파일 없이 실행하며, 호출 작업 디렉터리를 그림 리소스의 기준 범위로 사용한다.

```powershell
pwsh -NoProfile -File .\apps\md2hwp\convert.ps1 `
  -InputPath .\examples\report-workflow-v0.2.md `
  -Template .\tests\fixtures\templates\minimal-tagged-v1.hwp `
  -Output .\artifacts\my-report.hwp
```

기본값은 CommonMark 입력과 Debug 빌드다. 이미 빌드한 코드를 반복 실행할 때만
`-SkipBuild`를 추가한다. `-Configuration Release`는 Rust와 C# 모두 Release로
빌드한다. `-From pandoc-json`은 Rust 앱의 기존 직접 Pandoc JSON 입력을 사용한다.

입력 경로는 호출한 디렉터리 기준이다. 출력 디렉터리는 먼저 만들어 두어야 하며,
기존 출력 파일은 덮어쓰지 않는다. 템플릿은 현재 `minimal-1` 선언을 가진 HWP만
지원한다. HWPX, reStructuredText 입력, 생성 표는 이 명령의 지원 범위가 아니다.
그림은 현재 작업자가 허용하는 저장소 내부 PNG 파일이어야 한다.

## 실행 순서와 실패 처리

1. 입력·템플릿·출력 경로를 확인하고 Rust 앱을 빌드한다.
2. Rust 앱이 Pandoc을 실행하고 IR 0.2로 정규화·검증한다.
3. 유효한 IR만 `artifacts/convert-<고유값>/document.ir.json`에 쓴다.
   그림 경로는 이 임시 IR 위치 기준으로 환산한다.
4. 별도 프로세스에서 C#을 빌드한 뒤 기존 한글 작업자를 실행한다.
   .NET 빌드의 환경 변수 변경은 한글 프로세스로 전달되지 않는다.
5. 작업자가 템플릿 사본을 편집하고 저장·재열기 검사를 통과해야 HWP를 게시한다.
6. 임시 IR과 빈 작업 디렉터리는 성공·실패 모두 정리한다.

지원하지 않는 원고는 한글 실행 전에 실패한다. 템플릿·보안 모듈·기존 한글
프로세스 문제는 작업자가 진단하고 실패하며, 설치나 등록을 자동으로 수행하지 않는다.
실패한 명령은 0이 아닌 종료 코드를 반환한다. 직접 IR 생성과 재생이 필요하면
기존 `md2ir`와 [`render-tagged`](minimal-tagged-template.md)를 각각 사용한다.

## 검증 원고

`examples/report-workflow-v0.2.md`는 실제 조사 결과가 아닌 가상 보고서다.
긴 문단, 2~4단계 제목, 번호 목록과 중첩 글머리 목록, 명시적 줄바꿈, 중첩 강조,
그림 두 개, 빈 줄이 있는 박스, 그림·박스 출처, URL·이메일을 함께 다룬다.
실제 업무 원고의 검증을 대체하지 않는다.

COM을 실행하지 않는 실패 경로 검사는 다음 명령으로 재현한다.

```powershell
pwsh -NoProfile -File .\tools\smoke\test-convert-command.ps1
```

생성 후 페이지 검토용 PNG는 기존 작업자의 `export-images`로 내보낸다.
구조 검증 통과와 별개로 긴 문단의 줄바꿈, 제목 위치, 객체와 출처의 페이지
분리 여부를 눈으로 확인한다. rhwp 호환성은 이 검증의 통과 기준에 포함하지 않는다.

## 2026-09-23 검증 기록

위 명령으로 `artifacts/report-workflow-v0.2.hwp`를 생성했다. Rust·C# 빌드부터
저장·재열기까지 성공했고, 임시 IR 디렉터리는 제거됐다. 원본 템플릿은 변경되지
않았다. `artifacts/report-workflow-v0.2-pages/`로 4쪽을 내보내 확인했다.

- 그림 2개와 자동번호·출처, 박스와 출처가 각각 같은 쪽에 유지됐다.
- 긴 문단, 중첩 번호/글머리 목록, 강제 줄바꿈, 중첩 강조와 마지막 문단이 보존됐다.
- 생성 영역의 하이퍼링크가 없고 URL·이메일 문자가 보존됐다.
- 표지 이미지 SHA-256은 기존과 같은
  `8B12E3BC4909E9201617D853643F789BD7128F3ED67EA0B298BF7C6D5908D56B`였다.

시각적으로 남은 사항: 2쪽의 고정 제목 아래 큰 여백은 기존 템플릿 내용이다.
4쪽 맨 위에는 앞 문단의 마지막 짧은 줄인 `확인합니다.`가 홀로 넘어온다.
본문 내용 손실은 없지만, 실제 보고서의 편집 품질을 위해서는 템플릿의 문단
보호/외톨이줄 정책을 별도 검토해야 한다. 이 명령이 해당 설정을 임의로 바꾸지는 않는다.
