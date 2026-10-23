# rhwp 추가 제한의 출처 — 2026-10-23

앞선 [동등성 검토](rhwp-parity-2026-10-23.md)는 추가 거부 목록을 나열했지만,
그 원인이 rhwp 자체인지 md2hwp 구현인지 충분히 구분하지 않았다.
**대부분은 HWP 저장 자체의 불가능이 아니라 어댑터의 구현 범위 또는 정확성 정책이다.**
그렇다고 거부문만 삭제하면 올바르게 작동한다는 뜻도 아니다.

코드만 읽은 감사다. 이번 분류 작업에서 COM 실행·새 native 입력 시험·빌드는 하지 않았다.
upstream 근거는 고정 커밋 `1a76570e833917d15817415a53c09ad61ab3203f`의
`src/` 기준이다. 아래 행 번호는 작성 당시 기준이다.

## 분류 기준

- **upstream 특성 대응:** 실제 파서·표시기·저장 모델의 동작을 보완한다. 고정 버전의
  구현 한계이며 영구적인 HWP 형식 한계와 같지 않다.
- **어댑터 미구현:** rhwp가 표현할 수 있지만 md2hwp가 변환·측정 규칙을 구현하지 않았다.
- **어댑터 정책:** 검증 범위 밖 결과를 내보내지 않으려고 저희가 추가한 거부다.
  정책의 근거와 적용 범위를 검토할 수 있다. rhwp가 원천적으로 금지한다고 설명하면 안 된다.

한 항목이 두 분류에 걸칠 수 있다. 특히 upstream 표시기의 불완전한 fallback을
그대로 믿지 않겠다는 것은 실제 upstream 동작을 고려한 **어댑터의 정책 선택**이다.

## 항목별 판정

| 제한 | 분류 | 실제 이유와 판단 |
| --- | --- | --- |
| 일부 목록 번호 형식 거부 | upstream 특성 대응 + 미구현 | raw 번호 형식은 저장할 수 있다. upstream 표시기가 10종 이외를 십진수로 대체하므로, 그 문자열로 native 표식 폭을 재면 틀릴 수 있다. 추가 formatter 구현 대상이다. |
| 원문자 20·로마자 3999·가나다/자모 14 초과 거부 | upstream 특성 대응 + 정책 | upstream이 범위 밖을 십진수로 바꾼다. 이 fallback이 한글의 실제 표시와 같다는 증거가 없어 거부했다. 이 수들이 HWP 문서 전체의 최대 번호는 아니다. |
| 목록 시작/표시 번호 65535 제한 | 저장 필드·upstream API에 결합한 구현 제한 | 시작 번호의 기본 필드는 u16이지만 수준별 시작과 restart는 u32도 있다. 표시기의 formatter는 u16으로 잘라 쓴다. 모든 native 목록 번호의 본질적 상한이라고 단정하면 안 된다. |
| 이미지 글머리표의 후속 문단 거부 | 미구현 + upstream 표시 한계 대응 | 모델과 serializer에 이미지 bullet이 있다. native 이미지 표식의 실제 폭 계산이 없고, upstream 표시 경로도 이를 완전하게 그리는 근거가 없다. 고정 폭 후속 문단까지 막는 것은 더 보수적인 정책이다. |
| PUA 글머리표 전면 거부 | 어댑터 측정 미구현 + 정책 | upstream에는 일부 PUA의 Symbol/한컴 문자 매핑이 있다. 저희 GDI 실측은 그 매핑·글꼴 선택을 재현하지 않는다. PUA 자체가 rhwp에서 금지된 것은 아니다. |
| 보충평면 문자가 든 표식 문자열 거부 | 어댑터 검사 API 선택 | GetGlyphIndicesW를 쓴 coverage 검증이 해당 문자를 확인하지 못해 거부한다. 번호 형식 문자열은 surrogate pair 저장이 가능하다. 단일 HWP5 bullet 문자 슬롯은 별도 WCHAR 제약이 있으므로 구분한다. |
| 선택 글꼴에 없는 표식 glyph 거부 | 어댑터 정확성 정책 | fallback을 사용한 실제 표시와 측정 글꼴이 달라지는 것을 막는다. rhwp 모델의 저장 불가는 아니다. 정확한 fallback·shaping 측정 없이는 삭제를 권하지 않는다. |
| 표 원형의 cell-zone 거부 | 어댑터 변환 미구현 + 정책 | upstream은 읽기·저장·표시를 지원한다. 저희가 3×2 원형을 임의 격자로 늘릴 때 zone 좌표·적용 범위를 어떻게 반복할지 정의하지 않아 막았다. |
| 숨은 설명 안의 원고 각주 거부 | 어댑터 정책 + upstream 번호 순회 한계 | 문단과 각주를 표현·직렬화할 수 있고 어댑터도 순회한다. 다만 upstream 번호 계산은 이 컨테이너를 빠뜨린다. native 허용성과 표시도 아직 시험하지 않았다. |
| 필드 메모 안의 원고 각주 거부 | 실제 upstream HWP 재열기 부족 + 정책 | 모델과 출력 경로는 있지만 HWP5 reader의 MEMO_LIST 본문 연결 구현을 찾지 못했다. 숨은 설명과 달리 기본 roundtrip 보강이 먼저 필요하다. |
| AUTONUM 원본 레코드 대응 실패 시 거부 | upstream 특성 대응 + 무손실 검증 정책 | reader가 저장 번호를 파생 번호로 덮어쓰므로 raw에서 원래 값을 복원한다. 원본 대응을 증명하지 못하면 멈춘다. 특정 native 컨트롤 전체의 미지원을 뜻하지 않는다. |
| 특수 컨테이너의 목록 후속 문단 실측 거부 | 어댑터 번호 흐름 미구현 | 캡션·머리말·꼬리말·바탕쪽·주석·각주 각각의 독립/공유 카운터와 표시 순서를 정의하지 않았다. 저장 모델 금지가 아니다. |
| 컨테이너 중첩 128 제한 | 어댑터 자원 정책 | 재귀 폭주를 막는 예산이다. 128이라는 native 형식 상한을 확인한 것이 아니다. |

## 적용 범위를 과장하지 말 것

번호 형식·표시 범위·PUA·글리프 검사는 보통 **한 목록 항목에 본문 문단이 둘 이상이고
UseInstWidth=true여서 실제 표식 폭에 맞춰 후속 문단을 정렬할 때** 작동한다.
한 문단 목록 전체나 일반 원고의 Unicode·이모지를 금지하는 규칙이 아니다.
이미지 bullet은 그보다 앞에서 검사하므로 UseInstWidth=false인 다문단 항목도 막는다.
시작 번호 65535 제한은 새 번호 정의를 만들 때 적용되어 후속 문단에 한정되지 않는다.

HWPX 미지원은 사용자가 정한 rhwp 백엔드의 범위다. upstream 자체가 HWPX를
못 읽는다는 의미가 아니다. 제목 블록 그림·OLE·구역 정의, 여러 SECTION 등
양쪽 공통 계약은 rhwp 추가 제한으로 세지 않는다.
JPEG 재인코딩 차이는 사용자 승인으로 이미 허용했다.

## 핵심 코드 근거

### 목록 표시와 측정

어댑터 `backends/rhwp/src/list_markers.rs:304–333`의 검사와
upstream `renderer/layout/utils.rs:229–278`을 대조했다.
upstream도 같은 10종만 매핑하고 나머지는 Digit로 바꾸며, u32 번호를 u16으로
변환해 `format_number`에 넘긴다. `renderer/mod.rs:2900–3008`의 formatter는
범위 밖 원문자·로마자·가나다/자모를 decimal로 대체한다.

`model/style.rs:428`의 Numbering에는 u16 `start_number`와 u32
`level_start_numbers`가 함께 있고, `serializer/doc_info.rs:695` 이후 각각
해당 폭으로 저장한다. `model/paragraph.rs:248`의 NewStart도 u32다.
따라서 기본 시작 필드의 실제 16비트 폭, 저희의 동일 시작값 대입 방식,
formatter의 u16 API 제한을 하나의 형식 상한으로 합쳐 설명하면 부정확하다.

이미지 bullet은 upstream `model/style.rs:464`, `serializer/doc_info.rs:723`에
저장 표현이 있다. 반면 `renderer/layout/paragraph_layout.rs:592`는 이미지 표식
U+FFFF를 건너뛴다. 같은 파일 `:11018`의 `map_pua_bullet_char`는 일부 PUA를
실제 표시 문자로 바꾸므로, raw PUA를 그대로 GDI 측정하는 것과 같지 않다.

`crates/md2hwp-font-measurement/src/lib.rs:109–146`의 supplementary/missing
glyph 거부는 저희 코드다. 번호 형식 문자열의 UTF-16 저장과 달리 단일 bullet은
upstream `serializer/doc_info.rs:720`에서 `char_to_wchar`를 쓴다.
`serializer/byte_writer.rs:8`은 보충평면 단일 문자를 U+FFFD로 바꾼다.
이 두 입력의 지원 가능성은 별개로 판단해야 한다.

### 표의 영역 서식

upstream `model/table.rs:97`, `parser/control.rs:296`,
`serializer/control.rs:699`에 TableZone의 좌표·border/fill 읽기와 쓰기가 있다.
`renderer/layout/table_layout.rs:3374`, `:3539`에는 배경·대각선 표시도 있다.
거부는 저희 `backends/rhwp/src/tables.rs:139`에서 추가했다.

단순히 guard를 지우면 기존 3×2 좌표가 새 표의 일부에만 적용될 수 있다.
header/body/source 소유권, 반복 행, 한 열 출력, 병합 출처행과 zone 우선순위를
정해야 한다. zone을 무시하거나 전부 복사하는 것이 자동으로 올바른 변환은 아니다.

### 숨은 설명·메모·저장 번호

어댑터 `note_plan.rs:198`의 두 컨테이너 금지와 달리
`model_tools.rs:427–428`은 이미 둘 다 순회한다.
upstream HiddenComment는 `model/control.rs:381`, `parser/control.rs:623`,
`serializer/control.rs:1014`에 문단 읽기·쓰기가 있다.
그러나 `parser/mod.rs:1303–1504`의 번호 재계산은 이 컨테이너를 방문하지 않는다.

메모는 더 제한적이다. `model/control.rs:862`에 본문 배열이 있지만
HWP5 `parser/control.rs:135`는 빈 배열로 생성한다. parser 전체에서 본문 할당은
HWPX `parser/hwpx/section.rs:6118`에서만 찾았다.
HWP5 `serializer/body_text.rs:171–260`의 MEMO_LIST 출력은 root의 Memo Field를
모으며 중첩 필드를 재귀 수집하지 않는다. 현재 HWP 전용 backend에서 지원하려면
HWP5 메모 파싱·index 연결과 필요한 출력 범위를 먼저 보강해야 한다.
이는 정적 코드 감사 결과이며 실제 메모 문서로 새 시험을 한 결과는 아니다.

AUTONUM은 upstream `parser/control.rs:634`가 저장 번호를 읽은 뒤
`parser/mod.rs:1336`의 파생 계산이 number/assigned_number를 다시 쓴다.
어댑터 `note_storage.rs:65–147`은 원본 컨트롤의 순서·레벨·비번호 payload를
대조한 후 저장값을 복원한다. 최종 저장 레코드는 독립 기대값으로 검사하며,
reader cache 비교는 그 뒤 별도로 처리한다. 이 검사를 없애면 보존 보장이 약해진다.
복원에 쓰는 순서 대응 알고리즘 자체는 개선할 수 있으나 원본 저장값 대조는 유지해야 한다.

## 후속 작업 판단

저희가 정한 제한 중 재검토 우선 후보는 PUA 일괄 금지와
UseInstWidth=false 이미지 bullet 금지다. 실제 필요한 매핑·기하만 구현하면
허용 범위를 좁게 넓힐 수 있는지 확인할 수 있다. 표 zone도 adapter 구현 과제지만
먼저 원형에서 새 격자로 적용하는 의미를 정해야 한다.

번호 형식 확장은 native 표시 규칙을 확인하면서 진행하고, 글꼴 대체는 단순 허용보다
동일한 표시·측정 경로를 확보해야 한다. 숨은 설명의 각주는 native 허용성부터 시험하며,
메모는 upstream HWP roundtrip 보강을 선행한다.
이번 작업은 **분류와 근거 정리만** 했으며 어떤 거부 조건이나 검증도 변경하지 않았다.
