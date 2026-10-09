# Release XML 조회 공유와 비교 비용 감소

`1b428c6`는 비교 문서별 서식 정의 색인·해석 결과 캐시와 이미 소유한 문단 사본의 정규화를 도입합니다. `2450311`은 문서 변경 사이의 평면 문서 검사를 한 XML 사본으로 묶고, 네이티브 검증에서 이미 복사한 문서의 재복사를 제거합니다. 후속 `7fdb4c7`는 첫 성능 회귀를 조사하여 기본 Copy를 두 빌드 모두 기존 방식으로 복구하고, 색인·캐시는 불변인 XML을 반복 비교하는 범위에만 적용합니다. 이 구현들은 [직전 좌표 추적 측정](2026-10-09-clone-coordinates.md)의 Release(`4e4a24f`) 이후 변화입니다.

검사는 유지합니다. 하이퍼링크 제거 후 snapshot을 읽고, 템플릿 정의 삭제 후에는 새 XML로 삭제 결과를 확인합니다. 저장·재열기 뒤에는 새 snapshot을 사용하며, 문서가 변경되는 Attach/import 경계를 넘어 캐시를 재사용하지 않습니다. Debug는 기존 반복 조회와 비교 경로를 유지합니다. 현재 문단 스타일 조사는 보류한 그대로이며 IR 0.4·템플릿 계약은 변경하지 않았습니다.

## 첫 실험의 성능 회귀와 수정

| 일반 Release | CLI 전체(초) | attach-native(초) | import-native-references(초) | 최종 native 검증(초) |
| --- | ---: | ---: | ---: | ---: |
| 변경 전 | 277.016 | 29.383 | 80.999 | 27.711 |
| 처음 적용 | 322.670 | 136.462 | 39.063 | 17.720 |

처음 적용한 실행은 저장·재열기 등 기존 라이브 검증을 통과했지만 전체 시간이 늘었습니다. 일부 비교 구간이 빨라져도 native attachment의 비용 증가가 더 컸습니다. 조사 결과 ImportParagraph가 계속 변하는 destination의 서식 후보마다 기본 Copy를 호출했고, 그때마다 새로운 reader가 전체 서식 정의 색인을 만들었습니다. 반복 비교를 위한 색인이 단발성 복사에까지 적용된 것이 문제였습니다.

후속 수정은 기본 Copy를 기존 탐색 방식으로 되돌리고, TemplateRangeStructure와 이름 있는 스타일 정의 비교처럼 같은 불변 문서의 많은 요소를 검사하는 경우에만 reader를 공유합니다. 처음 실험은 initial/run.json·profile.json에 보존했으며 아래 최종 일반 측정과 섞지 않습니다. 처음 실험에 대해 전체 페이지 픽셀 일치를 주장하지 않습니다.

## 실행 시간

| 측정 | CLI 전체(초) | 백엔드(초) |
| --- | ---: | ---: |
| 변경 전 일반 Release | 277.016 | 275.328 |
| 변경 후 일반 Release | 209.830 | 206.509 |
| 변경 후 상세 계측 Release | 216.756 | 213.316 |

일반 Release 기준 **67.186초(24.25%) 단축**했습니다. 각 측정은 성공 실행 1회이며 시스템 변동과 상세 계측 비용을 분리 추정하지 않았습니다. 같은 원고·이미지·템플릿을 사용했고 SHA-256이 일치합니다. PDF 내보내기·렌더링·비교 시간은 변환 시간에 포함하지 않습니다.

| 단계 | 변경 전(초) | 변경 후(초) |
| --- | ---: | ---: |
| phase.prepare | 2.573 | 2.561 |
| phase.insert | 95.192 | 92.653 |
| phase.validate-flat | 8.425 | 2.933 |
| phase.save-flat-reopen | 14.275 | 15.277 |
| phase.verify-flat-reopened | 8.626 | 1.545 |
| phase.attach-native | 29.383 | 28.562 |
| phase.import-native-references | 80.999 | 37.637 |
| table.pagination | 28.034 | 16.108 |
| phase.save-native-reopen | 1.147 | 1.060 |
| phase.verify-native-reopened | 27.711 | 16.969 |

table.pagination은 phase.import-native-references에 포함됩니다. 하위 시간은 상위 시간에 다시 더하지 않습니다.

## 상세 비교 함수와 XML 조회

| 상세 계측 구간 | 횟수 | 누적(초) |
| --- | ---: | ---: |
| comparison.native-document | 6 | 9.892 |
| comparison.original-styles | 7 | 0.097 |
| comparison.structure | 231 | 1.409 |
| comparison.format-index | 476 | 0.745 |

comparison.native-document는 문서 사본·정규화·스타일·본문·이미지 검증 전체입니다. original-styles와 structure는 그 안에서도 호출되며 format-index는 서식 해석에 포함됩니다. 표의 누적 시간은 중첩 구간을 포함하므로 합산하지 않습니다. 변경 전에는 같은 세부 구간을 계측하지 않아 함수별 단축률을 주장하지 않습니다.

| XML 조회 | 변경 전 횟수 | 변경 후 횟수 | 변경 전(초) | 변경 후(초) |
| --- | ---: | ---: | ---: | ---: |
| 전체 문서 XML | 37 | 19 | 22.801 | 11.107 |
| 선택 영역 XML | 473 | 473 | 63.376 | 62.282 |
| 현재 문단 스타일 조사 XML | 473 | 473 | 63.376 | 62.282 |

선택 영역 표는 공통 프로파일러의 XML 조회입니다. 상세 COM 계측에는 네이티브 참조의 직접 saveblock 호출도 기록되므로 횟수가 다를 수 있습니다.

## 저장·가져오기 동작

| 상세 COM 행동 | 변경 전 횟수 | 변경 후 횟수 | 변경 전(초) | 변경 후(초) |
| --- | ---: | ---: | ---: | ---: |
| hwp.SaveAs | 2 | 2 | 14.825 | 14.800 |
| hwp.Open | 3 | 3 | 0.206 | 0.204 |
| hwp.Clear | 6 | 6 | 0.078 | 0.074 |
| hwp.SetTextFile:HWPML2X | 3 | 3 | 3.608 | 3.226 |
| hwp.SetTextFile:HWP:insertfile | 70 | 70 | 3.931 | 4.062 |
| hwp.InsertPicture | 29 | 29 | 3.170 | 3.274 |

상세 COM 메서드는 35,025회입니다. 속성 읽기·쓰기와 out/ref 메서드는 이 수에 포함되지 않습니다. 생성·저장·재열기·참조 삽입 동작의 제거를 이번 변경의 근거로 삼지 않습니다.

## 출력과 검증

보존한 원래 Debug 출력과 새 일반 Release 출력을 비교했습니다. 고정 입력뿐 아니라 원래 Debug 백엔드의 SHA-256도 확인했습니다. HWP/PDF 전체 파일 바이트 일치를 주장하지 않습니다.

비교에는 재빌드하지 않은 기존 비교기(artifacts/debug-release-equivalence/compare/bin/Release/net10.0-windows/compare.dll)를 사용했습니다. 기존 코드 사용 여부는 verification.json의 provenance 기록이며 비교기 바이너리 해시도 함께 기록합니다.

- 본문·중첩 구조·사용 서식·이름 있는 스타일·번호·개체 기하·참조 대상 그래프가 비교 규칙에 따라 일치했습니다. 생성 식별자는 대상 경로로 정규화했습니다.
- 내장 BINDATA를 디코딩한 이미지 데이터가 일치했습니다.
- 같은 Poppler 설정의 144dpi PNG에서 **107쪽 전체 RGB 픽셀이 일치**했습니다.

이는 고정 시험 입력의 동등성 검증입니다. 임의로 길어진 제목이나 사용자 템플릿의 clipping·겹침 안전성을 보장하지 않습니다.

| 검사·빌드 | 결과 | 명령 |
| --- | --- | --- |
| Backend Debug contracts | 통과 | `pwsh -NoProfile -File tools/development/dotnet.ps1 run --project tests/backend-contract/Md2Hwp.Backend.Tests.csproj` |
| Backend Release contracts | 통과 | `pwsh -NoProfile -File tools/development/dotnet.ps1 run --project tests/backend-contract/Md2Hwp.Backend.Tests.csproj --configuration Release` |
| Rust workspace tests | 통과 | `cargo test --workspace` |
| Rust Clippy | 통과 | `cargo clippy --workspace --all-targets -- -D warnings` |
| Debug build | 통과 | `pwsh -NoProfile -File tools/development/build.ps1 -Configuration Debug` |
| Release build | 통과 | `pwsh -NoProfile -File tools/development/build.ps1 -Configuration Release` |

테스트·빌드 표는 세션에서 확인한 종료 결과를 verification.json에 기록한 것입니다. 독립적인 원시 테스트·빌드 로그 파일은 별도로 보관하지 않았습니다. 서식 검사에는 중첩 참조 해석, 누락·중복 ID, BORDERFILL 0, 문서별 cache 격리, 새 scope의 정의 변경 탐지, 입력 XML 불변과 실제 문자·서식·기하 변경 거부를 포함합니다. 공유 snapshot 검사는 서식·문자 강조·목록·박스·그림 캡션 검사의 정상/오류 판정을 확인합니다. 라이브 변환은 기존 저장·재열기·구조·서식·각주·참조·표 검증을 통과했습니다.

## 입력 해시와 자료·롤백

| 입력 | SHA-256 |
| --- | --- |
| large-5x.md | `e1f5735152995d0891c8af96af2da9468263131f975c25b10cc46a1c433799cd` |
| image.png | `39bb81eabc91b69c6fb209f9fe6ff81a787e4eb6e9efc23711d3840c8577a47b` |
| template.hwp | `e7c678d3014a82c34b565e860e42526ba6d75c688b5d3b33491bafe64430fde9` |

원시 자료는 artifacts/performance-xml-comparison-2026-10-09에 있습니다. initial/profile.json·run.json은 성능 회귀가 있었던 첫 실험이고, ordinary/profile.json과 run.json은 수정 후 일반 측정입니다. large.detailed.json, large.spans.csv, large.calls.csv, large.callsites.csv, large.actions.csv는 상세 기록입니다. structure-comparison.json·page-comparison.json은 비교 결과, ordinary/large.output.hwp는 새 출력입니다. target/debug·target/release는 개발 빌드이고, 계측 사본은 artifacts에만 있습니다.

전체 롤백은 `git revert 7fdb4c7` 다음 `git revert 2450311`, `git revert 1b428c6` 후 Release 재빌드입니다. 후속 수정만 되돌리면 성능 회귀가 있었던 첫 적용 상태로 돌아갑니다. 원본 템플릿과 원고는 수정하지 않았습니다.
