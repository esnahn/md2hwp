# 2026-10-09 현재 Release 상세 성능 측정

현재 커밋 `e03700a`의 Release를 `tools/development/build.ps1 -Configuration Release`로 다시 빌드하여 측정했습니다. 생성·검증 코드는 수정하지 않았습니다. 일반 Release 실행과 COM 메서드별 계측을 추가한 별도 Release 사본을 각각 한 번 실행했습니다. 계측 사본은 `artifacts/`에만 있으며 `target/release` 실행 파일을 교체하지 않습니다.

동일한 대규모 원고(그림 29개, 코드 박스 14개, 표 39개, 각주 66개)와 템플릿의 별도 사본을 사용했습니다. 승인된 사용자·세션의 Windows PowerShell 5.1 x64 STA에서 순차 실행했고, 기존 한글 프로세스 검사와 보안 모듈 파일·등록·RegisterModule 검사를 유지했습니다. 입력 원고·템플릿을 변경하지 않았습니다.

## 실행 시간

| 측정 | CLI 전체(초) | 백엔드(초) | 성공 |
| --- | ---: | ---: | --- |
| 일반 Release | 378.341 | 374.990 | True |
| 상세 계측 Release | 385.164 | 381.648 | True |

표에는 로그 저장까지 완료된 실행을 구성별로 1회씩 기록했습니다. 앞선 일반 Release 실행도 변환은 성공했으나(백엔드 383.612초), PowerShell 5.1이 정상 stderr 프로파일 출력을 오류로 취급해 CLI 시간과 로그 저장이 중단되었습니다. 해당 HWP와 사유는 ordinary/attempt1에 보존했고, 변환 코드 수정 없이 측정 스크립트만 고쳐 다시 실행했습니다. 상세 계측의 기록 비용과 실행 시 시스템 변동이 있으므로 일반 Release 시간을 기준으로 하고, 상세 기록은 호출 수와 비용 분포를 조사하는 데 사용합니다. 단계와 그 안의 세부 항목 시간을 중복 합산하지 않습니다.

## 단계별 시간

| 단계 | 일반 Release(초) | 상세 계측(초) |
| --- | ---: | ---: |
| phase.insert | 201.377 | 205.301 |
| phase.import-native-references | 76.822 | 79.639 |
| phase.attach-native | 28.024 | 27.999 |
| phase.verify-native-reopened | 26.939 | 27.744 |
| phase.save-flat-reopen | 13.797 | 13.916 |
| phase.validate-flat | 8.510 | 8.331 |
| phase.verify-flat-reopened | 8.293 | 9.014 |
| phase.prepare | 2.162 | 2.649 |
| phase.save-native-reopen | 1.100 | 1.083 |

## 단계 안의 계측된 COM 비용

| 단계 | 기록한 메서드 호출 수 | 직접 COM 시간(초) | 단계 전체(초) |
| --- | ---: | ---: | ---: |
| phase.prepare | 44 | 1.773 | 2.649 |
| phase.insert | 12802 | 169.259 | 205.301 |
| phase.validate-flat | 55 | 7.320 | 8.331 |
| phase.save-flat-reopen | 3 | 13.907 | 13.916 |
| phase.verify-flat-reopened | 10 | 7.903 | 9.014 |
| phase.import-native-references | 11344 | 18.261 | 79.639 |
| phase.save-native-reopen | 3 | 1.083 | 1.083 |
| phase.verify-native-reopened | 10848 | 4.402 | 27.744 |

이 표는 상세 계측 실행 안에서 비교한 값입니다. 단계 전체와 직접 COM 시간의 차이는 XML 파싱·복사·대조 등 관리 코드와 계측되지 않은 COM 속성/out·ref 메서드, 기록 비용을 포함합니다. 이를 전부 C# 계산 시간이나 제거 가능한 검증 비용으로 해석하지 않습니다.

## 개체 및 문단 처리

그림·코드 박스가 상대적으로 느린 이유는 [개체별 호출 비용 분석](2026-10-09-object-costs.md)에 정리했습니다.

| 종류 | 횟수 | 일반 Release(초) | 개체당 평균(초) |
| --- | ---: | ---: | ---: |
| operation.figure/body | 29 | 99.257 | 3.423 |
| operation.code/code | 14 | 29.172 | 2.084 |
| operation.table/body | 39 | 5.744 | 0.147 |
| operation.text/body | 565 | 57.808 | 0.102 |

이 항목들은 `phase.insert`에 포함됩니다. 최종 네이티브 구조 조립·가져오기·저장·재열기는 별도 단계입니다.

## 상세 COM 호출

기록한 메서드 호출은 **35,113회**입니다. COM 속성 읽기·쓰기와 out/ref 인자를 가진 메서드(예: GetPos)는 이 수에 포함하지 않습니다. `GetTextFile` 공통 래퍼는 직접 계측하여 전체/선택 XML 내보내기를 포함합니다. Release에서 실행되지 않는 Debug 검사 구문은 계측 지점 목록에 있더라도 실행 횟수에 포함되지 않습니다.

| 행동 | 횟수 | 직접 소요 시간(초) |
| --- | ---: | ---: |
| hwp.GetTextFile:HWPML2X:full | 177 | 108.614 |
| hwp.GetTextFile:HWPML2X:saveblock | 493 | 64.706 |
| hwp.SaveAs | 2 | 14.807 |
| hwp.HAction.Run:Cancel | 1072 | 5.188 |
| hwp.SetTextFile:HWP:insertfile | 70 | 3.982 |
| candidate.Item | 12168 | 3.810 |
| hwp.SetTextFile:HWPML2X | 3 | 3.378 |
| hwp.InsertPicture | 29 | 3.091 |
| Activator.CreateInstance:HWPFrame.HwpObject | 1 | 2.510 |
| hwp.HAction.GetDefault:InsertText | 1796 | 1.998 |
| hwp.HAction.Execute:InsertText | 1796 | 1.844 |
| control.GetAnchorPos | 6474 | 1.635 |
| hwp.GetTextFile:UNICODE:saveblock | 18 | 1.535 |
| hwp.HAction.Run:BreakPara | 692 | 1.196 |
| hwp.HAction.GetDefault:StyleEx | 794 | 0.813 |
| hwp.HAction.Run:TableRightCell | 392 | 0.764 |
| hwp.HAction.Run:MoveDocEnd | 1099 | 0.761 |
| hwp.GetTextFile:HWP:saveblock | 3 | 0.654 |
| hwp.HAction.Execute:StyleEx | 794 | 0.638 |
| parentAnchor.Item | 1096 | 0.338 |

## 전체/선택 XML 내보내기

| 호출 위치 | 횟수 | 일반 Release(초) |
| --- | ---: | ---: |
| hwpml.block.export/CurrentParagraphStyle.ReadBlock | 473 | 60.995 |
| hwpml.export/AuriMinimalCaptionPrototype.ReadDocument | 61 | 36.497 |
| hwpml.export/TemplateSource.Insert | 54 | 32.200 |
| hwpml.export/AuriMinimalBoxPrototype.ReadDocument | 31 | 18.155 |
| hwpml.export/HancomPreviewWriter.ReadParagraphs | 7 | 4.877 |
| hwpml.export/TaggedTemplateWriter.RenderTaggedTemplate | 7 | 4.864 |
| hwpml.export/TaggedTemplateWriter.RangeRoots | 7 | 4.356 |
| hwpml.export/TaggedTemplateWriter.ImportFigureDocument | 3 | 1.859 |
| hwpml.export/NativeCrossReferences.Insert | 2 | 1.829 |
| hwpml.export/HancomPreviewWriter.VerifyLists | 2 | 1.324 |
| hwpml.export/HancomPreviewWriter.VerifyText | 1 | 0.767 |
| hwpml.export/HancomPreviewWriter.PrepareInsertionTarget | 2 | 0.096 |

## 원시 자료와 검증 범위

원시 로그는 `artifacts/performance-release-2026-10-09/`에 보관했습니다. `ordinary/profile.json`은 실제 배포 Release의 단계/내보내기 집계이고, `large.detailed.json`은 호출별 시간선과 작업 범위를 포함합니다. CSV 파일은 `large.calls.csv`(호출 순서), `large.actions.csv`(행동별), `large.callsites.csv`(위치별), `large.operations.csv`(개체별), `large.spans.csv`(단계별), `large.xml.csv`(XML 조회별)입니다. 해시와 두 실행의 집계는 동명의 JSON에 보관했습니다.

두 실행 모두 기존 전체 구조·서식·이미지 데이터·각주·참조·표 배치 검증과 최종 저장·재열기를 통과했습니다. 이번 측정에서는 별도의 PDF 렌더링이나 페이지 픽셀 비교를 수행하지 않았습니다. 이전 변경의 시각 동등성 검증은 [Debug·Release 비교](2026-10-09-release-diagnostics.md)에 기록되어 있습니다.

최적화 후보의 호출 수는 실제 비용 위치를 나타내며 안전한 제거 가능 횟수를 자동으로 증명하지 않습니다. 현재 문단 스타일 조사는 캐시 무효화 조건을 설계해야 하고, 복제 전후 XML 조회는 삽입 위치 탐색을 좌표 방식으로 대체해야 합니다. 최종 저장 결과 검증을 제거 대상으로 세지 않습니다.
