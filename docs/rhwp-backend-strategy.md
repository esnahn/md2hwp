# rhwp 백엔드 교체 검토

검토일: 2026-10-19. md2hwp 기준은 v0.4.4 (`54caa0d`), 작업 브랜치는
`codex/rhwp-backend`이다. 최초 소스 검토 뒤 무편집 저장 시험을 수행했다.
실행·한글 재열기·페이지 비교 결과는 [시험 보고서](rhwp-roundtrip-2026-10-19.md)에 있다.
사용자 결정에 따라 rhwp 백엔드는 HWP 템플릿과 HWP 출력만 지원한다.
HWPX 보존 문제는 이번 구현 범위에서 제외하며, 기존 한글 백엔드가 HWPX를 계속 담당한다.

## 결론

rhwp를 Rust 라이브러리로 연결하는 HWP 전용 백엔드를 만든다.
무편집 저장 시험 결과를 바탕으로 HWP 최소 생성 시험을 진행한다.
C# 백엔드는 비교 기준이자 HWPX 지원 경로로 유지한다.
IR 0.4와 원고 문법, 템플릿 태그는 엔진 교체만을 이유로 변경하지 않는다.

## 확인한 upstream

공식 GitHub API에서 확인한 최신 릴리스는 v0.8.7(2026-10-06), main은
`1a76570e833917d15817415a53c09ad61ab3203f`이다. 다음 근거는 이 커밋에 고정한다.

- [Cargo.toml](https://github.com/edwardkim/rhwp/blob/1a76570e833917d15817415a53c09ad61ab3203f/Cargo.toml): rlib/cdylib 제공. 브라우저/WASM을 거치지 않는 Rust 연결 후보이다.
- [공개 라이브러리](https://github.com/edwardkim/rhwp/blob/1a76570e833917d15817415a53c09ad61ab3203f/src/lib.rs): DocumentCore, parser, model, serializer 노출.
- [저장 API](https://github.com/edwardkim/rhwp/blob/1a76570e833917d15817415a53c09ad61ab3203f/src/serializer/mod.rs): HWP/HWPX serializer 및 content-loss report 제공. API 존재만으로 우리 개체의 보존을 보장하지 않는다.
- [지원 범위](https://github.com/edwardkim/rhwp/blob/1a76570e833917d15817415a53c09ad61ab3203f/README.md): HML은 제한 지원이며 그림·리소스 등 미지원 요소의 손실 저장을 차단한다. 따라서 기존 HWPML 합성 결과를 그대로 입력하는 경로는 우선안에서 제외한다.
- [MIT 라이선스](https://github.com/edwardkim/rhwp/blob/1a76570e833917d15817415a53c09ad61ab3203f/LICENSE): 저작권·허가문 보존 필요. 실제 도입 시 선택 feature의 전이 의존성과 자산까지 별도로 점검하고 배포 고지를 갱신한다.

## 현재 코드에서 가져갈 것

Release는 이미 `DirectXmlFlatDocument.cs`에서 IR과 템플릿으로 HWPML을
합성한다. COM은 주로 템플릿 해석, 그림 리소스와 개체 identity 생성,
native 상호참조, 일부 목록 측정, 표 페이지 보정, 저장·재열기에 남아 있다.

| 현재 역할 | 이전 방안과 주의점 |
| --- | --- |
| Rust Pandoc→IR 검증 | 그대로 유지. rhwp의 Document 모델과 공개 IR을 분리한다. |
| 템플릿 선언·범위·서식 복제 | rhwp 어댑터에 포팅. 정적 문서 영역과 미사용 서식을 함께 보존한다. |
| 제목 카운터·고정 제목 참조 | 기존 의미와 테스트를 재사용한다. 템플릿 의존 코드 제목 처리를 IR로 옮기지 않는다. |
| 그림·캡션·각주 | native 개체와 리소스 ID 할당을 구현한다. 그림·표 번호와 참조를 일반 텍스트로 축소하지 않는다. |
| 표 폭·페이지 분할 | `TableAutoWidths.cs`의 Windows GDI 측정과 `NativeTablePagination.cs`의 실제 페이지 보정은 별도 대체 과제다. |
| 최종 검증 | HWPML 문자열 일치에 종속되지 않는 구조 비교가 필요하다. 한글에서 다시 열어 보는 독립 검증을 개발 단계에 유지한다. |
| hwp2pdf | 별도 과제. md2hwp 생성 백엔드 교체가 PDF 출력 동등성을 의미하지 않는다. |

## 제안 구조

`Markdown/Pandoc JSON → md2hwp IR → 템플릿 바인딩/생성 계획 → rhwp 어댑터 → HWP`

공개 IR에 rhwp 타입을 노출하지 않는다. 어댑터만 upstream 모델/API에 의존하며
revision을 고정한다. 사용자가 동의한 구조는 rhwp 라이브러리를 직접 링크하는
별도 `md2hwp-rhwp.exe`이며, 기존 `--worker`로 선택하고 백엔드 단독 실행도
지원한다. 기존 백엔드와 IR+템플릿→출력 계약 및 CLI를 맞춘다.
rhwp 백엔드에서 .hwpx 템플릿 또는 출력 경로를 받으면 생성 전에 명확한
미지원 오류를 반환한다. 기존 한글 백엔드 선택을 안내하며 자동 전환하지 않는다.
이번 무편집 시험 EXE는 이 제품용 백엔드와 구분한다.
HWP 기능을 옮긴 후 rhwp 실행 경로의 .NET/한글 의존을 제거할 수 있다.
글꼴 측정과 PDF까지 포함한 플랫폼 독립 여부는 따로 판단한다.

## 단계와 통과 조건

1. **무편집 왕복 저장 시험**
   - 기본 템플릿과 전체 기능 출력 문서를 고정 입력으로 사용한다.
   - HWP→HWP를 통과 대상으로 한다. 이미 수행한 HWPX·교차 저장 시험은 조사 기록으로만 남긴다.
   - 텍스트, 실제 서식 정의, 페이지/구역 설정, 머리말·꼬리말·바탕쪽,
     그림 데이터, native 캡션·각주·필드와 참조 관계를 비교한다.
   - rhwp 자체 재열기뿐 아니라 한글 재열기와 같은 한글 렌더러의 페이지 비교를 한다.
   - 실패하면 파서/serializer 보존 문제를 먼저 해결한다. 이 단계 전에 본문 생성기를 포팅하지 않는다.
2. **최소 생성 시험**
   - 본문·제목·메타데이터를 HWP 템플릿 샘플에서 복제하고 HWP로 저장한다.
   - 태그가 여러 글자 run에 걸친 경우, 서식 ID 재매핑, 정적 영역 보존을 검증한다.
3. **기능별 확대**
   - 목록 → 코드 → 그림·캡션/출처 → 표 → 각주 → native 상호참조 순으로 확대한다.
   - once/except.once, each.child, 제어문자, 개체 앞뒤 빈 문단도 기존 계약 그대로 확인한다.
   - PNG/JPEG/EMF 내장과 표시를 각각 검사한다. EMF 데이터 보존과 rhwp의 화면 렌더링은 별개 판정이다.
4. **동등성 및 성능 판정**
   - 짧은 전체 기능 원고와 대규모 원고를 기존 백엔드와 비교한다.
   - 구조·리소스·필드 검증과 실제 줄바꿈/페이지/각주 번호/표 분할/잘림 검증을 나눈다.
   - 템플릿 읽기, 생성, 조판, 저장, 검증 시간을 각각 측정한다. 속도 개선은 측정 전 단정하지 않는다.
5. **기본값 전환**
   - HWP 계약을 통과한 뒤 기본 백엔드 변경 여부를 판단한다. 미지원 구조는 명시적으로 실패한다.
   - 사용자에게 알리지 않는 COM 자동 fallback은 두지 않는다.
   - C# 백엔드의 HWPX 지원과 해당 런타임 안내는 유지한다. hwp2pdf도 별도 과제다.

## 가장 먼저 확인할 질문

현재 `templates/template.hwp`를 rhwp로 읽고 아무것도 수정하지 않은 채 저장한
파일이 한글에서 동일한 구조와 모양으로 열리는가? 시험에서 HWP→HWP는
본문·서식·필드와 19쪽의 픽셀이 일치했다. HWPX 관련 경로에서는 세로쓰기,
스타일 및 일부 필드 속성·페이지 배치 차이가 확인됐으나 HWPX는 지원 범위에서
제외했다. 이어 수행한 [최소 생성 시험](rhwp-generation-2026-10-20.md)에서도
평문 제목·본문·메타데이터의 구조와 한 페이지 픽셀이 일치했다.
이 결과를 아직 미지원인 복합 제목·개체 생성 성공으로 확대 해석하지 않는다.

## 실험용 실행 파일 빌드와 사용

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug -Rhwp
.\target\debug\md2hwp.exe 원고.md --worker .\target\debug\md2hwp-rhwp.exe
.\target\debug\md2hwp-rhwp.exe 원고.ir.json
```

`-Rhwp`는 별도 네이티브 실행 파일을 추가 빌드한다. 기본 백엔드와 배포 ZIP 구성은
변경하지 않는다. rhwp 실행에는 .NET/한글 COM이 필요하지 않으며, 이 worker와
`--dotnet`을 함께 지정하면 오류를 반환한다. Markdown을 IR로 바꾸는 단계에는
기존과 같이 Pandoc이 필요하다.
`init-template`은 아직 기존 한글 백엔드의 기능이므로 템플릿을 새로 만들 때에는
한글과 .NET이 필요하다. 이미 만들어진 HWP 템플릿을 읽어 생성하는 rhwp 경로와 구분한다.

현재 지원 범위는 일반 문단, heading1–heading6의 평문·번호 문단 및 복합 제목 블록,
Text/Space/LineBreak, Strong/Emph/Link 표시 문구이다. 제목 블록에서는 표·글상자·머리말·
꼬리말의 여러 제목 슬롯, once/except.once, 중첩 each.child, own/ancestor 번호 태그와
쪽 나눔을 처리한다. 루트 정적 문단과 제목 블록 내부에서 meta 및 date-meta fmt를
치환한다. 기본 템플릿으로 제목·본문 원고를 생성한 비교 결과는
[제목 블록 시험](rhwp-headings-2026-10-21.md)에 기록한다.
HWPX와 원고의 목록·코드·그림·표·각주·상호참조 생성은 명시적 미지원 오류다.
제목 블록의 그림·OLE·구역 정의와 일부 필드가 포함된 조건부 문단 합치기도 거부한다.
이 단계의 선언 검사는 기존 validate-template의 모든 native 구조 검사를 대체하지 않는다.

저장은 임시 파일에 수행한 뒤 rhwp 재열기와 모델 비교를 통과해야 결과를 교체한다.
이 검사는 한글의 실제 페이지 배치와 시각적 동등성을 보장하지 않는다.
