# rhwp/Hancom 기능 동등성 검토 — 2026-10-23

사용자가 요청한 2번까지의 작업 이후 기능 동등성을 판단하기 위한 코드 대조 기록이다.
현재 확인한 범위만으로 두 백엔드의 **완전한 기능 동등성을 선언할 수 없다**.
공통 템플릿 계약에 따른 거부와 rhwp에만 있는 추가 거부를 구분해야 한다.

이 문서의 코드 행 번호는 작성 당시 기준이며, 이후 수정으로 이동할 수 있다.
“C#에 대응 거부 없음”은 코드 대조 결과다. 해당 입력이 한글에서 실제로 삽입·저장·재열기되고
정확하게 표시된다는 입증은 아니다. 구조 검증과 시각적 검증도 구분한다.

## 공통 계약과 제한

다음 항목을 rhwp만의 미구현 기능으로 세거나, 동등성을 맞추기 위해 무조건 허용해서는 안 된다.

| 항목 | rhwp 근거 | Hancom 근거 | 판단 |
| --- | --- | --- | --- |
| 제목 블록의 그림·OLE | `backends/rhwp/src/headings.rs:309` | `backends/hancom-automation/TemplateHeadingBlocks.cs:68` | 두 백엔드 모두 명시적으로 거부한다. 중첩된 그림·OLE도 포함한다. |
| 제목 블록의 SECDEF/COLDEF | `backends/rhwp/src/headings.rs:309` | `backends/hancom-automation/TemplateHeadingBlocks.cs:66` | 구역·단 정의는 제목 블록 안에서 허용하지 않는다. 기존 `begin:template` 예외를 다른 선언으로 확대하지 않는다. |
| 여러 SECTION 템플릿 | `backends/rhwp/src/template.rs:30` | `backends/hancom-automation/TaggedTemplateWriter.cs:565`, `AuriMinimalBoxPrototype.cs:319` | 두 백엔드 모두 하나의 SECTION을 요구한다. |
| 목록 후속 문단 정렬의 상대 문자 단위 여백 | `backends/rhwp/src/lists.rs:439` | `backends/hancom-automation/NativeListContinuations.cs:42` | 두 백엔드 모두 홀수 URC 여백을 거부한다. 고정 HWPUNIT만 계산한다. |
| 실측에 필요한 Windows TTF/OTF와 실제 선택 글꼴 확인 | `backends/rhwp/src/table_widths.rs` 및 `crates/md2hwp-font-measurement/src/lib.rs` | `backends/hancom-automation/TableAutoWidths.cs:258`, `TableAutoWidths.cs:288` | HFT와 글꼴 대체를 허용하는 것으로 동등성을 맞추지 않는다. 표식의 개별 glyph 검증은 아래 추가 제한과 구분한다. |
| 원고 각주를 native 캡션·머리말·꼬리말·바탕쪽·다른 각주/미주에 배치 | `backends/rhwp/src/note_plan.rs:192` | `backends/hancom-automation/NativeFootnotes.cs:198` | 공통으로 거부한다. 제목 슬롯을 해당 컨테이너로 복사한 경우도 확인해야 한다. |
| 각주 샘플의 위치·문단 수·AUTONUM/TAB·continuation 규칙 | `backends/rhwp/src/footnotes.rs:70`, `footnotes.rs:94`, `footnotes.rs:248` | `backends/hancom-automation/NativeFootnotes.cs:35`, `NativeFootnotes.cs:52`, `NativeFootnotes.cs:78` | 공통 샘플 계약이다. 두 문단 샘플은 첫 본문과 선택적 `footnote.next`를 뜻한다. |

Hancom은 제목 블록 VIDEO도 `TemplateHeadingBlocks.cs:68`에서 거부한다.
고정 rhwp 의존성 `1a76570e833917d15817415a53c09ad61ab3203f`의
`src/model/control.rs`와 `src/model/shape.rs`에는 Video 모델이 없다.
이를 rhwp 영상 지원 또는 영상 보존의 근거로 삼을 수 없으며, 공통 금지를 해제할 대상도 아니다.

## rhwp의 추가 제한

아래 표는 어댑터의 추가 거부 목록이며, rhwp 자체가 저장할 수 없는 기능 목록이 아니다.
각 항목의 upstream 특성·미구현·정책 구분과 해제 조건은
[추가 제한의 출처](rhwp-limit-classification-2026-10-23.md)에 정리했다.

| 항목 | rhwp의 명시적 제한 | C# 대조 근거와 판정 |
| --- | --- | --- |
| 목록 후속 문단의 번호 표시 형식 | `backends/rhwp/src/list_markers.rs:321`은 형식 코드 0–5, 8, 10, 12, 13만 허용하며, 나머지는 `:333`에서 거부한다. | `backends/hancom-automation/NativeListContinuations.cs:27`은 `GetHeadingString()`으로 표시 문자열을 얻는다. 대응 형식 whitelist가 없으므로 rhwp의 추가 제한이다. |
| 목록 표시 번호 범위 | `backends/rhwp/src/list_markers.rs:311`은 원문자 1–20, 로마자 1–3999, 가나다/자모 1–14 등의 범위 밖을 거부한다. `:304`는 표시 번호의 UINT16 범위도 확인한다. | 위 C# 경로에는 동일 범위 guard가 없다. 각 형식의 범위 밖 native 동작은 별도 입증이 필요하다. |
| 그림 글머리표를 사용하는 목록 후속 문단 | `backends/rhwp/src/lists.rs:353`, `list_markers.rs:231`에서 거부한다. | C#은 native 목록 원형을 적용하고 `NativeListContinuations.cs:27`에서 표시 문자열을 얻는다. 동일 명시적 거부가 없다. 실제 그림 표식의 정확한 폭과 후속 문단 위치는 live 검증 전까지 미입증이다. |
| PUA 글머리표를 사용하는 목록 후속 문단 | `backends/rhwp/src/list_markers.rs:234`에서 private-use 범위를 확인하고 `:236`에서 거부한다. | C#에는 같은 범위 guard가 없다. 특정 글꼴의 PUA 매핑과 표시를 입증한 것은 아니다. |
| 목록 표식의 보충평면 문자와 없는 glyph | `backends/rhwp/src/table_widths.rs:43`에서 glyph 검사를 요구한다. `crates/md2hwp-font-measurement/src/lib.rs:118`은 보충평면 문자를, `:146`은 없는 glyph를 거부한다. | C# `TableAutoWidths.cs:293`은 `GetTextExtentPoint32W`로 측정하며 대응 glyph coverage 검사가 없다. rhwp의 안전 검사에 따른 추가 거부다. 일반 원고 Unicode나 emoji 전체를 금지하는 규칙이 아니다. |
| 표 원형의 cell-zone overlay | `backends/rhwp/src/tables.rs:140`에서 일괄 거부한다. | C# `TemplateTables.cs:166`은 전체 원형을 복제하고 `:223`에서 ROW만 교체한다. 대응 zone 거부가 없으므로 코드상 추가 제한이다. zone의 좌표·서식이 새 격자에서 올바르게 적용되는지는 C#도 native 검증이 필요하다. |
| 숨은 설명·필드 memo 안으로 복사된 원고 각주 | `backends/rhwp/src/note_plan.rs:198`의 `HiddenComment`와 `:199`의 `memo_paragraphs`도 금지 컨테이너로 센다. | C# `NativeFootnotes.cs:198`의 금지 목록에는 이 두 컨테이너가 없다. rhwp의 추가 명시적 guard이며, C#의 native 성공까지 입증된 기능 차이로 단정하지 않는다. |

목록의 번호 형식·범위·glyph 제한은 주로 `UseInstWidth=true`인 여러 문단 항목의
표식 실측 경로에 적용된다. 한 문단 목록 전체나 일반 원고 텍스트의 지원 범위로 확대해서는 안 된다.
그림 글머리표는 `Lists::marker_head`에서도 거부하므로 `UseInstWidth=false`인 후속 문단 경로와도 구분해서 시험해야 한다.

## 원시 레코드 및 정적 컨테이너의 미입증 범위

`backends/rhwp/src/note_storage.rs:99`부터 원본과 재직렬화한 컨트롤의 수·순서·레벨을
대조하고, `:118`에서는 AUTONUM의 비수치 payload까지 일치시킨다. 원본의 정적 컨트롤을
손실 없이 대응시킬 수 없으면 거부한다. C#에는 같은 원시 HWP 레코드 비교 경로가 없다.
이 차이만으로 특정 정적 필드·컨테이너가 rhwp에서 미지원이라고 확정해서는 안 된다.
실제 실패 입력과 보존 차이를 확보한 뒤 판정해야 하며, 비교를 완화해서 통과시킬 대상도 아니다.

`note_storage.rs:351` 부근은 고정 reader가 순회하지 않는 바탕쪽·숨은 설명·필드 memo·
묶음 도형 자식의 derived AUTONUM cache를 별도로 취급한다. 이는 저장된 원본 값의 손실을
허용한다는 뜻이 아니다. 저장 레코드 검증과 reader cache 비교를 분리한 구현을 그대로 확인해야 한다.

## 구현 및 시험 결과

### 엄격한 템플릿 검사

한글 `validate-template`에도 고아 번호 태그 검사를 적용했다. 여섯 제목 참조,
평문/블록 제목, 중첩 each.child와 once/except.once의 미사용 분기를 먼저 검사하고,
남은 번호 태그는 그림·표 native 캡션의 caption 슬롯과 함께 있는 heading1만 허용한다.
변환 경로는 변경하지 않았다. 검사 입력 XML도 수정하지 않는다.

실제 HWP 일곱 개를 두 검사기로 대조했다. 추적 템플릿과 정상 metadata/fmt는 모두
성공했고, 잘못된 fmt, heading6 누락, reset/body 동일 스타일, 알 수 없는 선언,
고아 num:heading1은 모두 거부했다. 모든 원본 해시는 유지했다.
이는 [이전 검사 도구 기록](rhwp-template-tools-2026-10-22.md)의 마지막 판정 차이를 해소한다.

### 조건부 범위 안의 native 필드

rhwp의 once/except.once 편집을 표시 문자 위치가 아닌 native UTF-16 위치로 처리한다.
TAB·필드 시작/끝·제목 위치·범위 태그를 함께 옮기고, 문단을 합칠 때 cross-paragraph
필드를 같은 문단의 필드로 연결한다. 전체 필드를 삭제할 수 있지만 경계 한쪽만
삭제하는 입력은 거부한다. 제목 블록 재작성은 원래 존재하지 않은 필드 메모 문단을
만들지 않는다. 기존 셀의 내용을 모두 제거한 경우에는 원래 서식의 빈 문단을 유지한다.

한글에서도 복제 필드의 InstId가 큰 무작위 값일 때 import에서 재할당되는 사례를 발견했다.
실제 실패 값은 3971622632, 4231234618, 3366119921이었다. 복제 식별자를 기존 값과
충돌하지 않는 양수 31비트 값으로 결정적으로 할당했다. 이는 관측된 한글 2020 동작에
맞춘 adapter 정책이며, 공식 형식이 모든 ID를 signed 31비트로 제한한다는 주장은 아니다.
기존/static 식별자와 FieldId·Command·시작/끝 연결은 보존한다.

추적 템플릿의 heading6를 별도 복사본에서 native Clickhere 필드가 있는 블록으로 바꾸고,
같은 수준 제목을 세 번 사용했다. 한글 2020 11.0.0.9136에서 원형을 저장·재열기한 뒤
양쪽 백엔드로 생성했다. 각 백엔드의 독립 기대값 대조와 저장·재열기 검증을 유지했다.

| 사례 | 내용 | 한글/rhwp 필드 수 | 한글 300dpi 페이지 비교 |
| --- | --- | ---: | --- |
| join-field | 필드 안의 once 범위를 제거하면서 두 문단 결합 | 3 / 3 | 2쪽 모두 차이 0픽셀 |
| drop-field | once 범위가 여러 문단 필드 전체를 포함 | 1 / 1 | 2쪽 모두 차이 0픽셀 |
| join-except | except.once로 첫 사용에만 필드 내부 문단 결합 | 3 / 3 | 2쪽 모두 차이 0픽셀 |

재열기 XML에서도 Clickhere 유형·Command·FieldId를 확인했고 복제 필드의 InstId는
각 출력에서 고유했다. join-field의 2쪽은 직접 시각 확인했다. 생성·페이지 출력 중
원본 해시와 한글 프로세스 정상 종료를 확인했다. UI 조작이나 프로세스 강제 종료는 하지 않았다.

### 기존 원고 회귀

추적 템플릿으로 전체 기능 원고와 대규모 원고를 rhwp에서 다시 생성했다.
저장·재열기와 독립 모델 대조를 통과했으며, 각각 이전 검증본과 파일 바이트까지 동일했다.
따라서 이 두 결과에 대해서는 기존 페이지 비교 증거가 그대로 적용된다.
기존 JPEG 재인코딩 차이까지 해소되었다는 뜻은 아니다.
사용자는 이 재인코딩에 따른 JPEG 표시 차이를 허용했다. 이는 동등성 판단의
차단 사유에서 제외하며, 픽셀 일치를 주장하거나 구조·배치·캡션·원본 불변 검사를
완화하지 않는다. 다른 이미지 형식의 검증에도 이 예외를 적용하지 않는다.

| 결과 | 바이트 | SHA-256 (이전 검증본과 동일) |
| --- | ---: | --- |
| rhwp-all.hwp | 3,985,920 | `9d4b5f77fb775056cb7fa1abd1e886a2eeefb0e4539508e1500daefd1db5b29d` |
| rhwp-large.hwp | 3,938,816 | `95496b47af33a459b1b0f27561c7a43a89f0fc5fc89ac0599cdf7449fa1ecffc` |

대규모 원고에는 제목 59개, 그림 29개, 표 39개, 코드 박스 14개, 각주 66개,
상호참조 36개가 있다. 이전 검증의 페이지 수는 106쪽이다.

### 재현 및 보관 위치

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Release -Rhwp
.\target\release\md2hwp-rhwp.exe 원고.ir.json rhwp.hwp --template 시험템플릿.hwp
.\target\release\md2hwp-backend.exe 원고.ir.json hancom.hwp --template 시험템플릿.hwp
```

원형·결과·HWPML·300dpi 페이지는 ignored `artifacts/rhwp-parity-2026-10-23/`에 있다.
`create-fixtures.ps1`은 별도 템플릿 원형 생성 절차, `fields.md`/`fields.ir.json`은 입력이다.
`*-page-comparison.json`, `field-evidence.json`, `template-validation-results.json`,
`regression-results.json`에 대조 결과를 보관했다. 이 자료는 배포 파일에 포함하지 않는다.
고정 회귀 검사는 `conditional_ranges.rs`, `headings.rs`,
`TemplateValidationNumberScopeTests.cs`, `CopiedNativeIdentityTests.cs`에 추가했다.
Rust workspace 286개 테스트, Clippy `-D warnings`, fmt와 C# 계약 검사를 통과했다.
정상 Debug·Release 빌드도 `-Rhwp`로 통과했다.
Windows 임시 실행 파일과 PDF 게시 검사는 호스트에서 실행했다. Clippy에는 Rust 코드
경고가 없었고, Windows 증분 캐시의 hard-link 실패 후 복사 사용 안내만 있었다.

추적 `templates/template.hwp`는 변경하지 않았다. SHA-256:
`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`.

구현 커밋은 `ebde8b9`(엄격 검사), `cf730c1`(한글 복제 식별자),
`684b82e`(rhwp 조건부 필드와 제목 블록)로 나눴다. 뒤의 커밋 본문에서 앞의 관련
커밋과 실측 근거를 언급했다. main·next·원격 브랜치·릴리스는 변경하지 않았다.

## 동등성 판정

요청한 엄격 검사 판정 차이와 조건부 native 필드 보존은 해결했다.
지원 기능의 기존 통합 원고와 새 필드 사례는 위 범위에서 동등성을 확인했다.
그러나 임의 사용자 템플릿까지 포함한 완전 동등 판정은 보류한다.
위 추가 제한과 미입증 native 컨테이너는 남아 있다. 공통 레이아웃 한계도 그대로다.
JPEG 재인코딩에 따른 표시 차이는 사용자 승인으로 허용하며 동등성 판단을 막지 않는다.
HWPX 미지원은 사용자가 정한 rhwp의 의도적 범위다.
기본 백엔드·릴리스 구성은 변경하지 않았으며 백엔드 선택은 사용자 검토 후 결정한다.
