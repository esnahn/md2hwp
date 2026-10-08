# 빌드와 검증

## 빌드

Rust 버전은 `rust-toolchain.toml`, .NET SDK는 `global.json`에 고정되어 있습니다.
.NET SDK가 없으면 `dependencies/install-dotnet-sdk.ps1`로 개발용 SDK를 준비합니다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1
cargo test --workspace
```

Release 빌드 결과는 `target/release/`의 다음 네 파일입니다.
`-Configuration Debug`를 주면 `target/debug/`에 모입니다.
개발·실행에는 이 두 폴더를 기본으로 사용합니다. `target/dist/`는 버전별 배포 파일과 ZIP의 보관용입니다.

- `md2hwp.exe`
- `md2hwp-backend.exe`
- `template.hwp`
- `README.md`

C# 단독 프로젝트는 `backends/hancom-automation/Md2Hwp.Backend.csproj`입니다.
`templates/template.hwp`를 게시 폴더와 `target/debug` 또는 `target/release`에 복사합니다. 빌드 폴더의 템플릿은 매번 강제 교체합니다. 기본 템플릿은 `templates/template.hwp`에서 편집하고, 별도 사용자 템플릿은 `target/` 밖에 두어 `--template`으로 지정하십시오.
빌드 결과는 Git에서 제외합니다. 배포할 때 위 네 파일을 같은 폴더에 복사합니다. 동봉 README의 문서·예제 링크는 GitHub의 해당 버전으로 연결합니다.

배포 ZIP을 만들려면 다음 명령을 사용합니다.

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Release -Package
```

`target/dist/md2hwp-v<버전>-windows-x64.zip`에 실행 파일 두 개, 추적 기본 템플릿과 README를 넣고 SHA-256을 출력합니다.
패키지는 사용자 편집본 대신 `templates/template.hwp`를 사용합니다. 기존 배포 ZIP은 새 ZIP 생성에 성공한 뒤 교체합니다.
GitHub Release에는 실행용 ZIP만 첨부하고, 체크섬은 본문 끝에 표시합니다. 소스는 GitHub 자동 다운로드를 사용하며 릴리스 노트·빌드 매니페스트는 별도 첨부하지 않습니다.

추가 계약 검사:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
pwsh -NoProfile -File tools/development/dotnet.ps1 run --project tests/backend-contract/Md2Hwp.Backend.Tests.csproj
```

계약 테스트는 실제 한글 실행이나 시각적 배치 검증을 대신하지 않습니다.

## 기능 검증 원고

[`examples/all-features-twice/`](../examples/all-features-twice/)에는 검증 원고
`all-features.md`, 예상 IR `all-features.ir.json`, 그림 `image.png`를 함께 보관합니다.
제목 1~6단계, 본문 강조와 줄바꿈, 링크, 중첩 목록, 출처가 있는/없는 박스·그림·표를
두 장에서 반복합니다. 각주는 본문·제목·목록·표 셀·강조 및 링크 표시문에 넣고,
여러 문단·강제 줄바꿈·같은 정의의 반복 사용을 포함합니다. 헤딩·그림 번호 상호참조는
장 안팎의 앞뒤 대상, 한글 ID와 URI 이스케이프, 빈 표시문·참조형 링크, 강조·목록·표 셀·각주를 확인합니다.
박스는 제목과 본문, 제목 없는 본문, 제목만 있는 구성을 포함합니다.
표는 캡션·출처 유무, 위/아래 캡션, 정렬·서식·빈 셀을 포함합니다.
각 장에 2단계 제목을 세 개씩 두어 장 표지의 절 목록도 확인합니다.

YAML에는 제목·부제·저자 목록·날짜·발행처·사용자 문자열과 장 시작 번호 3을 지정합니다.
메타데이터 출력은 해당 `meta` 태그를 둔 템플릿에서 확인하며, `date-meta`의 날짜 포맷과
태그 문구를 담은 사용자 값의 비재귀 치환도 해당 태그를 넣어 확인할 수 있습니다.
개요 참조 대상인 heading2에는 실제 한글 개요 번호가 필요합니다. 동봉 템플릿의
heading2~heading4에는 개요 번호가 설정되어 있습니다. `init-template`으로 만든 일반
제목 문단은 별도 사본에 개요 번호를 설정하고 `--template`으로 지정하십시오.
각주가 있는 heading6 슬롯은 머리말·꼬리말·바탕쪽에 복제할 수 없습니다.
박스 제목을 분리하려면 `slot:box.title`이 필요합니다. 실제 각주 번호와 표·박스의
잘림·겹침은 생성된 HWP의 렌더링으로 확인해야 합니다.

저장소 루트에서 빌드한 프로그램으로 실행합니다.

```powershell
.\target\release\md2hwp.exe .\examples\all-features-twice\all-features.md
```

원고 옆의 IR을 갱신하고 `all-features.output.hwp`를 생성합니다.
IR은 Git에 포함하며, 생성된 HWP는 Git에서 제외합니다.
IR 변환만 확인하려면 다음 명령을 사용합니다.

```powershell
.\target\release\md2hwp.exe md2ir .\examples\all-features-twice\all-features.md --force
```

대규모 본문용 원고와 IR은 [`examples/korean-lorem/`](../examples/korean-lorem/)에 있습니다.
Ipsum 본문 11,155어절을 5장·20절로 나누었으며, 작성 당시 최대 검증 원고의
텍스트 2,231어절을 기준으로 본문 분량을 5배로 맞췄습니다. AURI 예시 보고서의 본문 대비 빈도를
참고해 그림 29개·표 39개·박스 14개·각주 66개와 목록·메타데이터·상호참조 등
현재 지원 문법을 추가했습니다. 빈도와 템플릿 조건은 해당 폴더의 README에 설명하며,
이 대규모 원고는 한글 변환·저장·재열기 검증을 통과했습니다. 최초 111쪽의 시각 문제를
확인한 뒤 강조 문법과 표 머리행 고립을 수정했으며, 보정된 112쪽 출력의 검증도 기록했습니다. 문단 스타일 조회를 선택 블록으로 바꾼 비교 실행은 779.527 → 646.964초였으며,
이전과 112쪽 PNG가 모두 동일했습니다. 호출 수·시간·단축 후보는
[대규모 문서 성능 측정](render-performance.md)에 정리했습니다.

[README로 돌아가기](../README.md)
