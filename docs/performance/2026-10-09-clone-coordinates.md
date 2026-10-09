# Release 원형 복제 좌표 추적: 성능과 출력 비교

`4e4a24f`는 [좌표 실험](2026-10-09-clone-position.md)에서 확인한 동작을 Release에 적용합니다. 캡션·코드 박스·출처 원형 복제의 전후 전체 XML 조회를 좌표 추적으로 바꾸고 Debug는 기존 XML 탐색과 대조를 유지합니다. 원형 삽입·내용 치환·최종 검증을 유지하며, 스타일 조사와 중간 저장·재열기는 변경하지 않았습니다.

삽입 전 본문 끝 List=0·Offset=0, 삽입 직후 동일 위치, MoveDocEnd 후 문단 번호 +1·Offset=0을 요구합니다. 불일치하면 오류로 중단하며 빈 문단을 자동 추가하지 않습니다. 좌표 검사 자체는 서식 보존을 증명하지 않으므로 최종 문서 검증을 유지합니다.

## 실행 시간

| 측정 | CLI 전체(초) | 백엔드(초) |
| --- | ---: | ---: |
| 변경 전 일반 Release | 378.341 | 374.990 |
| 변경 후 일반 Release | 277.016 | 275.328 |
| 변경 후 상세 계측 Release | 276.049 | 272.511 |

일반 Release 기준으로 **101.325초(26.78%) 단축**했습니다. 동일한 원고·이미지·템플릿 사본을 사용했으며 각 표의 일반/상세 측정은 성공 실행 1회입니다. 시스템 변동과 별도 계측의 기록 비용을 분리 추정하지 않았습니다. PDF 내보내기·렌더링·비교 시간은 변환 시간에 포함하지 않았습니다.

## 단계와 개체 처리

| 항목 | 변경 전(초) | 변경 후(초) |
| --- | ---: | ---: |
| phase.insert | 201.377 | 95.192 |
| phase.import-native-references | 76.822 | 80.999 |
| phase.attach-native | 28.024 | 29.383 |
| phase.verify-native-reopened | 26.939 | 27.711 |
| phase.save-flat-reopen | 13.797 | 14.275 |
| operation.figure/body | 99.257 | 14.754 |
| operation.code/code | 29.172 | 3.176 |
| operation.table/body | 5.744 | 6.283 |
| operation.text/body | 57.808 | 61.257 |

개체 처리 시간은 phase.insert에 포함됩니다. 표 항목은 임시 문단 입력이며 실제 표 조립·너비 계산·페이지 배치는 이후 단계에 포함됩니다.

## XML 조회와 COM 호출

| 항목 | 변경 전 횟수 | 변경 후 횟수 | 변경 전(초) | 변경 후(초) |
| --- | ---: | ---: | ---: | ---: |
| hwpml.export/AuriMinimalCaptionPrototype.ReadDocument | 61 | 3 | 36.497 | 1.365 |
| hwpml.export/AuriMinimalBoxPrototype.ReadDocument | 31 | 3 | 18.155 | 1.376 |
| hwpml.export/TemplateSource.Insert | 54 | 0 | 32.200 | 0.000 |
| hwpml.block.export/CurrentParagraphStyle.ReadBlock | 473 | 473 | 60.995 | 63.376 |
| 전체 XML 조회 | 177 | 37 | 106.824 | 22.801 |

새 상세 계측에서 COM 메서드 **35,043회**를 기록했습니다. 속성 읽기·쓰기와 out/ref 메서드(좌표 GetPos 포함)는 이 수에 포함하지 않습니다. 좌표 GetPos는 계측 사본에서 별도 구간으로 기록하여 **210회·0.050초**를 측정했습니다. XML 조회 제거와 별개로 좌표 추적에서 MoveDocEnd 호출이 추가되므로 전체 메서드 호출 수 감소와 XML 조회 감소는 같지 않습니다.

## 출력 비교

보존한 변경 전 Debug 출력과 변경 후 일반 Release 출력을 비교했습니다. 원고·이미지·템플릿은 이전 동등성 시험의 고정 사본과 같습니다. 비교용 Debug 백엔드 해시도 기록했습니다. 전체 HWP/PDF 파일 바이트의 일치를 주장하지 않습니다.

- 본문·중첩 구조·사용 서식·스타일·개체 및 참조 대상 그래프를 기존 비교기로 대조했습니다. 생성된 내부 식별자는 대상 문서 경로로 정규화하고 실제 내용·번호·기하·서식은 유지했습니다.
- XML의 내장 BINDATA를 디코딩하여 이미지 데이터 해시를 비교했습니다.
- PDF를 같은 Poppler 설정의 144dpi PNG로 렌더링하여 **107쪽 전체 RGB 픽셀이 일치**했습니다.

비교 결과 원문은 아래 자료에 보관합니다. 페이지 픽셀 비교는 이번 고정 입력의 출력 동등성을 확인하며, 임의로 길어진 제목이나 사용자 템플릿의 배치 안전성을 보장하지 않습니다.

## 검증·파일·롤백

Debug·Release 백엔드 계약 테스트, cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings를 통과했습니다. 계약 테스트에 비어 있지 않은 끝 문단, 내부 리스트, 커서 이동, 잘못된 문단 증가, 남은 텍스트, 정수 경계의 오류 조건을 추가했습니다. 라이브 변환은 기존 저장·재열기·구조·서식·각주·참조·표 검증을 통과했습니다.

빌드는 tools/development/build.ps1 -Configuration Release로 수행했습니다. target/release에 수정 빌드를 두며, 계측 사본은 artifacts에만 있습니다. 원본 템플릿은 수정하지 않았습니다. 변경 전 Debug 출력은 artifacts/debug-release-equivalence/baseline-debug에 보존되어 있습니다.

원시 자료는 artifacts/performance-coordinates-2026-10-09에 있습니다. ordinary/profile.json은 실제 Release 측정, large.detailed.json 및 large.calls.csv/large.callsites.csv/large.actions.csv는 상세 기록입니다. structure-comparison.json과 page-comparison.json은 비교 결과, ordinary/large.output.hwp는 새 출력입니다.

롤백은 `git revert 4e4a24f` 후 Release 재빌드입니다. 이 변경은 IR·템플릿 계약을 변경하지 않습니다.
