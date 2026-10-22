# rhwp의 동일 이미지 리소스 공유

2026-10-22. 구현은 `6998d81`이다. IR 0.4, 템플릿 문법, 기본 백엔드와
배포 ZIP 구성은 변경하지 않았다. HWP 전용 실험 백엔드의 저장 방식만 바꿨다.
이전 문제와 한글 재저장 결과는 [통합 비교](rhwp-integration-2026-10-22.md)에 있다.

## 공유 규칙

`figures::embed`는 같은 확장자와 **원본 파일 바이트 전체**를 가진 기존
Embedding을 먼저 찾는다. 선언과 content의 확장자가 모두 같고 storage ID의
연결이 유일할 때, 기존 BinData 선언의 1-based 순번을 반환한다. 실제 storage ID는
그림 참조 번호와 별개다. 해시 충돌이나 픽셀 유사도를 이용한 병합은 하지 않는다.

기존 템플릿에 들어 있던 정상 리소스도 같은 조건이면 재사용한다. 압축 리소스는
해제한 파일 바이트를 비교하되, 검증된 입력 파일 길이를 상한으로 로드한다.
기존 항목의 순서·ID·raw 데이터·압축 방식·provenance는 변경하거나 제거하지 않는다.
재사용하지 못할 때만 기존처럼 새 storage ID를 할당한다.

Link/OLE, 실패·무시 상태, content 누락, 선언/content 확장자 불일치와 중복 storage
연결은 후보에서 제외한다. 서로 다른 바이트나 확장자는 따로 저장한다. 원본 파일
경로별 변경 검사와 최종 저장 바이트·독립 예상 모델 검증은 그대로 유지했다.

공유 대상은 이미지 바이트 저장소다. 각 그림의 native 식별자, 크기·배치·자르기,
캡션·출처·장별 번호와 ZOrder는 이전과 같이 개별 생성한다. 한글로 다시 저장하는
단계를 생성 과정에 추가하지 않았다.

## 대규모 원고 결과

같은 `examples/korean-lorem/large-5x.ir.json`과 `templates/template.hwp`를 사용했다.
원고는 같은 `image.png`를 그림 29개로 사용한다. 앞선 측정 결과를 보존하고 별도
`rhwp-large-dedup.hwp`로 생성했다.

| 검사 | 이전 rhwp | 공유 적용 후 |
| --- | ---: | ---: |
| HWP 크기 | 110,928,384바이트 | **3,938,816바이트** |
| 원고 PNG의 저장 복사본 | 29개 | **1개** |
| 전체 BinData 스트림 | 30개 | 2개 |
| Release 프로세스 전체 시간 | 7.819초 | **4.065초** |
| 한글 페이지 수 | 106쪽 | 106쪽 |
| 원본 rhwp 출력과 픽셀 차이 | 기준 | **106쪽 전부 0** |

남은 한 스트림은 저장 크기 113바이트인 기존 템플릿 샘플이다. 이를 임의로
삭제하지 않았다. CFB를 직접 읽어 새 PNG 스트림 3,790,382바이트가 원본과
바이트·SHA-256까지 같음을 확인했다. native 한글 XML에서도 캡션이 있는 그림
29개와 서로 다른 개체 식별자 29개가 하나의 이미지 리소스를 참조함을 확인했다.
한글 XML의 리소스 번호는 열기·내보내기 때 재배열될 수 있으므로 저장 CFB의
물리 ID와 같은 값이라고 가정하지 않았다.

Release는 `tools/development/build.ps1 -Configuration Release -Rhwp`로 빌드했다.
다른 빌드·테스트가 끝난 뒤 이전과 같은 외부 Stopwatch로 프로세스 전체를 한 번
측정했다. 시작·IR 검증·생성·직렬화·저장·엄격한 재열기를 포함하고, 빌드·Pandoc·
페이지 이미지 출력은 제외한다. 이전 7.819초는 앞선 시험의 단일 실행 값이다.
반복 통계나 시간 항목별 분해를 하지 않았으므로 시간 차이를 보편적인 개선율로
보장하지 않는다.

새 결과는 한글 2020에서 300dpi PNG 106쪽으로 출력했다. 변경 전 rhwp 페이지와
전체 픽셀을 대조한 `dedup-pixels.json`은 모든 페이지에서 차이 0이었다. 따라서
이번 원고의 표시를 유지하면서 저장 중복을 제거했다. 이는 임의 템플릿의 조판
안전성이나 rhwp 자체 렌더러와 한글의 동등성을 입증하지 않는다. 기존 긴
TreatAsChar 표·상자의 잘림 제한도 그대로다.

## 검증과 재현

전체 Rust 테스트 **257개(rhwp 142개 포함)**, Clippy, rustfmt와 고정 .NET SDK의
백엔드 계약 테스트가 통과했다. Debug·Release 빌드 모두 완료했다. 회귀 검증은
logical 순번과 storage ID의 분리, ID 한도에서의 재사용, 다른 바이트·확장자,
누락·충돌 리소스, 기존 템플릿 항목의 불변성을 다룬다. 실제 템플릿의 PNG 반복
그림을 직렬화·재열기해 같은 리소스 참조, 별도 개체 ID와 캡션 번호 1·4를 확인했다.

기존 원고 폴더에서 생성한다. 원고 이미지와 템플릿은 수정하지 않는다.

```powershell
New-Item -ItemType Directory -Force artifacts/rhwp-integration | Out-Null
Push-Location examples/korean-lorem
try {
    ../../target/release/md2hwp-rhwp.exe ir2hwp --ir large-5x.ir.json --output ../../artifacts/rhwp-integration/rhwp-large-dedup.hwp --template ../../templates/template.hwp
} finally {
    Pop-Location
}
```

| 파일 | SHA-256 |
| --- | --- |
| 입력 IR | `C2641AE63A170275AB44D2C638301D3B1499462484EDFD741E2B9EC96B290ACB` |
| 템플릿 | `20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81` |
| 원본 PNG 및 저장 PNG | `39BB81EABC91B69C6FB209F9FE6FF81A787E4EB6E9EFC23711D3840C8577A47B` |
| Release rhwp EXE | `53A89ADFBFDFFB17B5D7423724C00A56D5D721547E6B04841D6810BCDB0540FE` |
| 생성 HWP | `95496B47AF33A459B1B0F27561C7A43A89F0FC5FC89AC0599CDF7449FA1ECFFC` |

원시 자료는 ignored `artifacts/rhwp-integration/`에 있다:
`benchmark-dedup.ps1`, `rhwp-large-dedup-timing.json`, 실행 stdout/stderr,
`rhwp-large-dedup.hwp`, `dedup-resources.json`, `dedup-picture-references.json`,
`rhwp-large-dedup-pages/`와 `dedup-pixels.json`. 이전의 큰 HWP와 한글 재저장 파일도
덮어쓰지 않고 유지했다.
