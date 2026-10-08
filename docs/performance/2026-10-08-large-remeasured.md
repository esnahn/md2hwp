# 대규모 원고 수정 후 재측정 — 2026-10-08

수정 대상은 백엔드입니다. 원고·이미지·템플릿을 바꾸지 않고 [실패했던 기준 측정](2026-10-08-baseline.md)과 같은 대규모 원고의 Release 전체 처리를 측정했습니다. 수정 커밋은 `d9d595f2c8f5cc429d4aef636a54f2bde20ad71d`입니다. 검증 생략이나 COM 호출 감소는 적용하지 않았습니다.

## 원인과 수정

실패 당시 XML의 자동번호 142개를 비교한 결과, 생성 각주 2번과 35번만 달랐습니다. 둘 다 `NumberType="Footnote"`이며 예상값이 원형의 `1`에 머물렀고, 한글은 실제 순서인 `2`, `35`를 반환했습니다. 쪽 번호나 그림·표 번호는 이 오류의 원인이 아니었습니다.

같은 서식의 인접 `TEXT`를 합칠 때 LINQ to XML이 부모가 있는 노드를 복제합니다. `XElement` 참조로 추적하던 생성 각주가 복제되면 연속 각주 번호 재계산에서 누락됐습니다. 이미 부여하고 있던 첫 본문 문단 식별자 `InstId` / `InstID`로 추적하도록 바꿨습니다. 기존 각주·연속 번호 검증, 이어지는 문단의 자동번호와 OnPage에 한정된 정규화 규칙은 유지했습니다.

같은 서식의 분리된 `TEXT`에 각주를 넣는 회귀 테스트를 추가했습니다. 이전 구현에서 실패하고 수정 후 통과했습니다. 백엔드 계약 테스트, `cargo test --workspace`, Clippy, Release 빌드도 통과했습니다.

## 조건과 결과

Windows x64 / 한글 2020 11.0.0.9136 / .NET 10.0.11 / SDK 10.0.400 / interactive Session 1. 입력은 Ipsum 본문 11,155어절에 제목 1~6, 그림 29개, 표 39개, 코드 박스 14개, 각주 66개 위치를 포함한 최상위 462블록 IR입니다.

| 항목 | 결과 |
| --- | --- |
| 종료 코드 | 0 |
| 저장·재열기 검증 | True |
| CLI 전체(초) | 530.699 |
| 백엔드 렌더(초) | 528.663 |
| 계측 COM 메서드 호출 | 35322 |
| COM 메서드 합계 / 중첩 제외(초) | 359.982 |
| 전체 XML export | 386회 / 237.013초 |
| 선택 블록 XML export | 493회 / 66.850초 |
| 전체 XML 누적 논리적 UTF-16 크기 | 3.853 GiB |

CLI 시간은 Pandoc·IR 처리, 프로세스 시작과 계측 파일 기록을 포함합니다. 백엔드 시간은 RenderTaggedTemplate 구간이며 마지막 계측 JSON 직렬화·기록은 제외합니다. COM 속성 get/set은 개별 계측하지 않았습니다. 동일한 132개 동적 메서드 호출 지점과 추가 XML/COM 시작 계측을 사용했습니다. XML 크기는 문자열 코드 단위 합계로, 디스크 파일 크기가 아닙니다. 선택 블록 직접 export 일부는 크기 집계에 포함되지 않습니다.

실행은 `target/release/md2hwp.exe`에 `--worker artifacts/performance-remeasure-2026-10-08/instrumented-bin/md2hwp-backend.exe`를 지정했습니다. 일반 Release 백엔드도 빌드했지만, 아래 실행 파일 해시는 일반 빌드 기록입니다. 실제 계측 worker·DLL·IR·그림의 해시는 집계 JSON의 `MeasurementFiles`에 있습니다.

각 조건은 1회 측정입니다. 실패했던 426.620초와 이번 완료 시간을 성능 개선율로 비교하면 안 됩니다. 이번 실행은 앞선 실패 지점 이후의 상호참조 삽입·표 페이지 보정·최종 저장·재열기 검증까지 포함합니다. 각 단계와 COM 합계는 포함 관계가 있으므로 합산하지 마십시오.

## 단계별 시간

| 단계 | 횟수 | 시간(초) |
| --- | --- | --- |
| phase.insert | 1 | 347.665 |
| phase.import-native-references | 1 | 82.556 |
| phase.attach-native | 1 | 28.253 |
| phase.verify-native-reopened | 1 | 28.030 |
| phase.save-flat-reopen | 1 | 14.827 |
| phase.validate-flat | 1 | 8.675 |
| phase.verify-flat-reopened | 1 | 8.412 |
| phase.prepare | 1 | 2.322 |
| phase.save-native-reopen | 1 | 1.061 |

상호참조 import 단계 안의 `table.pagination`은 28.249초였습니다.

| 생성 작업 | 횟수 | 시간(초) |
| --- | --- | --- |
| 그림 | 29 | 196.887 |
| 코드 박스 | 14 | 74.801 |
| 텍스트·제목·목록 문단 | 624 | 69.741 |
| 표 | 39 | 6.229 |

이 값은 원형 복제·검증을 포함한 본문 삽입 단계의 비용입니다. 표 폭 계산, 네이티브 캡션·각주 조립과 페이지 보정은 뒤 단계에 들어 있습니다.

## 저장 결과 확인

저장한 HWP에서 각주 번호 1~66이 연속됨을 XML로 확인했습니다. PDF는 107쪽이며, 문제였던 2번·35번 각주가 있는 PDF 5쪽·59쪽을 이미지로 확인했습니다. 각주 표시도 `2)`, `35)`로 정상입니다. PDF 내보내기와 시각 확인 시간은 위 성능 측정에 포함하지 않았습니다. 전체 107쪽의 시각 검토를 수행한 것은 아닙니다.

## 액션별 상세

시간 합계는 중첩 계측을 제외한 COM 메서드 비용입니다. 평균·P50·P95·최대는 개별 호출의 원래 구간입니다.

| 액션 | 횟수 | 합계(초) | 평균(ms) | P50(ms) | P95(ms) | 최대(ms) |
| --- | --- | --- | --- | --- | --- | --- |
| hwp.GetTextFile:HWPML2X:full | 386 | 237.013 | 614.024 | 592.773 | 814.623 | 1402.95 |
| hwp.GetTextFile:HWPML2X:saveblock | 493 | 66.850 | 135.598 | 129.268 | 168.218 | 349.377 |
| hwp.SaveAs | 2 | 15.691 | 7845.724 | 953.714 | 14737.734 | 14737.734 |
| hwp.HAction.Run:Cancel | 1072 | 4.787 | 4.466 | 1.462 | 19.124 | 58.687 |
| hwp.SetTextFile:HWP:insertfile | 70 | 4.124 | 58.912 | 56.933 | 71.479 | 90.281 |
| candidate.Item | 12168 | 4.054 | 0.333 | 0.353 | 0.617 | 402.16 |
| hwp.SetTextFile:HWPML2X | 3 | 3.652 | 1217.173 | 1552.546 | 2019.916 | 2019.916 |
| hwp.InsertPicture | 29 | 3.265 | 112.574 | 100.176 | 140.287 | 351.672 |
| hwp.HAction.GetDefault:InsertText | 1796 | 2.705 | 1.506 | 0.898 | 1.659 | 558.924 |
| Activator.CreateInstance:HWPFrame.HwpObject | 1 | 2.001 | 2000.68 | 2000.68 | 2000.68 | 2000.68 |
| hwp.HAction.Execute:InsertText | 1796 | 1.961 | 1.092 | 0.854 | 2.366 | 35.162 |
| control.GetAnchorPos | 6474 | 1.795 | 0.277 | 0.2 | 0.333 | 391.529 |
| hwp.GetTextFile:UNICODE:saveblock | 18 | 1.655 | 91.965 | 88.632 | 117.268 | 117.268 |
| hwp.HAction.Run:MoveDocEnd | 1099 | 1.420 | 1.292 | 0.409 | 8.639 | 43.748 |
| hwp.HAction.Run:BreakPara | 692 | 1.186 | 1.714 | 1.401 | 3.196 | 68.381 |
| hwp.HAction.GetDefault:StyleEx | 794 | 0.826 | 1.04 | 1.018 | 1.647 | 7.633 |
| hwp.HAction.Execute:StyleEx | 794 | 0.633 | 0.797 | 0.733 | 1.119 | 4.388 |
| hwp.GetTextFile:HWP:saveblock | 3 | 0.549 | 182.992 | 182.526 | 206.857 | 206.857 |
| hwp.HAction.Run:TableRightCell | 392 | 0.437 | 1.114 | 1.011 | 1.503 | 24.415 |
| hwp.HAction.GetDefault:RepeatFind | 182 | 0.396 | 2.174 | 1.589 | 2.76 | 42.898 |
| hwp.HAction.Run:MoveDocBegin | 18 | 0.366 | 20.357 | 0.7 | 261.683 | 261.683 |
| hwp.HAction.Execute:RepeatFind | 182 | 0.326 | 1.793 | 1.145 | 6.183 | 34.377 |
| parentAnchor.Item | 1096 | 0.322 | 0.294 | 0.349 | 0.621 | 1.982 |
| hwp.HAction.GetDefault:ParagraphShape | 221 | 0.313 | 1.416 | 1.162 | 2.01 | 29.668 |
| hwp.SetPos | 251 | 0.278 | 1.107 | 0.383 | 2.711 | 16.43 |
| hwp.HAction.Run:DeleteBack | 473 | 0.261 | 0.551 | 0.503 | 0.824 | 2.309 |
| hwp.HAction.Execute:ParagraphShape | 221 | 0.245 | 1.11 | 0.749 | 3.264 | 6.308 |
| hwp.HAction.Run:MoveListBegin | 548 | 0.232 | 0.423 | 0.402 | 0.663 | 1.761 |
| hwp.Open | 3 | 0.222 | 73.862 | 65.581 | 96.199 | 96.199 |
| hwp.SelectText | 473 | 0.206 | 0.435 | 0.387 | 0.604 | 4.798 |
| hwp.HAction.Run:MoveParaBegin | 185 | 0.189 | 1.021 | 0.854 | 1.65 | 10.305 |
| hwp.CreateAction:CharShape | 18 | 0.173 | 9.588 | 0.432 | 47.906 | 47.906 |
| hwp.RegisterModule | 1 | 0.155 | 155.265 | 155.265 | 155.265 | 155.265 |
| hwp.HAction.Run:BreakLine | 97 | 0.136 | 1.399 | 1.423 | 1.705 | 2.327 |
| parent.GetAnchorPos | 548 | 0.125 | 0.229 | 0.207 | 0.347 | 2.981 |
| hwp.HAction.Run:CharShapeBold | 268 | 0.117 | 0.438 | 0.394 | 0.701 | 1.244 |
| parameters.SetItem | 402 | 0.112 | 0.279 | 0.108 | 0.424 | 39.646 |
| hwp.HAction.Run:ShapeObjTableSelCell | 78 | 0.100 | 1.288 | 0.54 | 1.795 | 38.765 |
| hwp.HAction.Run:CharShapeItalic | 224 | 0.095 | 0.424 | 0.373 | 0.686 | 1.021 |
| action.CreateSet | 156 | 0.094 | 0.602 | 0.476 | 0.924 | 6.899 |
| hwp.Clear | 6 | 0.094 | 15.593 | 11.345 | 29.051 | 29.051 |
| hwp.HAction.Run:Delete | 88 | 0.082 | 0.932 | 0.8 | 1.804 | 4.304 |
| action.Execute | 142 | 0.079 | 0.559 | 0.255 | 2.296 | 4.028 |
| hwp.HAction.Run:TableLowerCell | 78 | 0.079 | 1.016 | 0.971 | 1.307 | 1.468 |
| subset.Merge | 112 | 0.062 | 0.554 | 0.489 | 0.746 | 5.04 |
| parameters.CreateItemSet | 112 | 0.061 | 0.544 | 0.465 | 0.814 | 4.6 |
| set.SetItem | 198 | 0.055 | 0.305 | 0.109 | 0.447 | 16.522 |
| hwp.FindCtrl | 78 | 0.051 | 0.652 | 0.357 | 0.625 | 18.368 |
| action.GetDefault | 156 | 0.045 | 0.291 | 0.163 | 0.468 | 9.01 |
| hwp.SetPosBySet | 78 | 0.037 | 0.479 | 0.397 | 0.67 | 5.151 |
| hwp.HAction.Run:MoveSelNextParaBegin | 29 | 0.037 | 1.272 | 0.418 | 0.88 | 22.799 |
| hwp.CreateAction:ParagraphShape | 138 | 0.036 | 0.26 | 0.177 | 0.431 | 3.134 |
| hwp.FindDir | 182 | 0.036 | 0.196 | 0.117 | 0.194 | 4.377 |
| characters.CreateSet | 18 | 0.025 | 1.369 | 0.522 | 11.803 | 11.803 |
| parameters.Item | 26 | 0.024 | 0.907 | 0.373 | 8.096 | 8.439 |
| hwp.HAction.Run:MoveParaEnd | 29 | 0.022 | 0.759 | 0.62 | 1.03 | 3.893 |
| hwp.CreateAction:InsertCrossReference | 18 | 0.022 | 1.201 | 0.19 | 18.062 | 18.062 |
| hwp.Quit | 1 | 0.015 | 15.448 | 15.448 | 15.448 | 15.448 |
| ((dynamic)nativeShape!).Item | 48 | 0.011 | 0.231 | 0.114 | 0.161 | 3.85 |
| hwp.HAction.Run:MoveSelParaEnd | 14 | 0.009 | 0.617 | 0.547 | 0.849 | 0.849 |
| characters.Execute | 18 | 0.008 | 0.441 | 0.237 | 3.526 | 3.526 |
| characters.GetDefault | 18 | 0.008 | 0.427 | 0.192 | 4.157 | 4.157 |
| hwp.GetHeadingString | 2 | 0.006 | 3.196 | 1.566 | 4.826 | 4.826 |
| subset.Clone | 2 | 0.005 | 2.298 | 0.643 | 3.954 | 3.954 |
| hwp.HAction.Run:SelectCtrlFront | 2 | 0.004 | 2.158 | 0.832 | 3.485 | 3.485 |
| hwp.HAction.Run:ShapeObjDetachCaption | 2 | 0.002 | 1.182 | 0.474 | 1.89 | 1.89 |

## 자료와 후속 작업

생성 문서와 호출별 자료는 `artifacts/performance-remeasure-2026-10-08/`에 있습니다.

- `large.output.hwp`: 검증을 통과한 결과.
- `large.detailed.json`: 액션별 시작 시각·소요 시간·호출 지점·문서 작업·단계.
- `large.calls.csv`, `large.actions.csv`, `large.callsites.csv`: 모든 호출과 액션·호출 지점 집계.
- `large.spans.csv`, `large.operations.csv`, `large.xml.csv`: 단계·원고 작업·XML export 집계.
- `large.expected.xml`, `large.actual.xml`: 수정 전 실패를 진단한 XML.
- `regression-before.stderr.log`: 이전 구현의 회귀 테스트 실패.

원시 자료는 ignored 경로에 보관하며 [집계 JSON](2026-10-08-large-remeasured.json)은 Git으로 추적합니다. 이전 실패 기준 자료도 그대로 유지했습니다.

이 완료 실행을 이슈 #7의 새 기준으로 사용합니다. 다음 최적화에서는 전체 export/import·저장·재열기 호출 수와 액션별 시간을 비교해야 합니다. 이번 수정 자체는 성능 최적화가 아닙니다.

우선 조사할 호출 위치는 다음과 같습니다. 앞의 세 위치가 전체 XML export 386회 중 355회·216.733초를 차지합니다. `CurrentParagraphStyle.ReadBlock`까지 합치면 약 280.715초입니다. 원형·삽입 위치 조회와 검증이 섞여 있으므로 모두 제거할 수 있다고 가정하지 않습니다.

| 호출 위치 | 횟수 | 시간(초) |
| --- | --- | --- |
| AuriMinimalCaptionPrototype.ReadDocument / 전체 XML | 148 | 91.601 |
| TemplateSource.Insert / 전체 XML | 108 | 66.037 |
| AuriMinimalBoxPrototype.ReadDocument / 전체 XML | 99 | 59.094 |
| CurrentParagraphStyle.ReadBlock / 선택 블록 XML | 473 | 63.982 |

표 페이지 조회의 `candidate.Item`은 12,168회지만 합계 4.054초입니다. 호출 횟수만으로 우선순위를 정하면 XML export의 큰 비용을 놓칠 수 있습니다.

## 입력 해시

| 파일 | SHA-256 |
| --- | --- |
| examples/korean-lorem/large-5x.md | e1f5735152995d0891c8af96af2da9468263131f975c25b10cc46a1c433799cd |
| templates/template.hwp | e7c678d3014a82c34b565e860e42526ba6d75c688b5d3b33491bafe64430fde9 |
| target/release/md2hwp.exe | 51479db2c70667b9b9ba720db14d1c9e3c351c49aed8dd8450b12a9573d00252 |
| target/release/md2hwp-backend.exe | e5ba596d69fbad46e9515b4f3f4244d968cd5eece3ab84ccfa35630407f3faed |
