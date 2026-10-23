# rhwp 템플릿 검사·생성 검증

검증일: 2026-10-22. IR 0.4, rhwp 고정 revision
`1a76570e833917d15817415a53c09ad61ab3203f`, 한글 2020 11.0.0.9136.
기존 한글 백엔드는 유지하며 HWPX는 rhwp 지원 범위에 추가하지 않았다.

## 독립적인 엄격 검사

`md2hwp-rhwp validate-template [[--template] <template.hwp>]`를 추가했다.
기본 경로는 EXE 폴더의 `template.hwp`이다. 기존 launcher의 `--worker`로도 선택한다.
한글·.NET·원고가 필요하지 않으며 원본 바이트 불변을 확인한다. 확장자뿐 아니라
HWP compound document 시그니처도 검사하여 이름만 바꾼 HWPX를 거부한다.

빈 원고의 렌더링으로 대체하지 않는다. 전체 문서와 바탕쪽·중첩 컨테이너에서
메타데이터 키·NFC·fmt·제어문자 경계를 먼저 검사하고 필수 키를 수집한다.
그 복사본에서 모든 제목·조건 분기·each.child, 목록, 코드, 그림, 표, 각주와
참조 원형의 실제 판독기를 실행한다. 선언 범위, reset/body 스타일 구분,
문단·문자 모양 및 스타일 참조, 정의 밖 잔여 태그와 고아 번호 태그도 검사한다.
사용하지 않는 원형도 검사 대상이다. 결과 JSON의 필드는 한글 worker와 같다.
사용자 메타데이터 키의 colon·supplementary Unicode 치환도 한글 계약에 맞췄다.

정상 추적 템플릿은 양쪽 모두 `RequiredMetadata=[title]`,
`RequiresRecognizedDate=false`였다. 별도 파일의 판정은 다음과 같다.

| 사례 | rhwp | 한글 worker |
| --- | --- | --- |
| 정상 사용자 메타데이터와 date-meta fmt | 성공: date, md2hwp-detail, title 필요 | 동일 |
| 미지원 날짜 서식 `%Q` | 거부 | 거부 |
| heading6 선언 누락 | 거부 | 거부 |
| reset과 body의 동일 스타일 | 거부 | 거부 |
| 알 수 없는 선언 | 거부 | 거부 |
| 정의 안의 소유 범위 없는 num:heading1 문단 | 거부 | 기존 검사에서는 허용 |

마지막 행은 의도한 엄격 검사 계약을 rhwp에서 적용한 차이이며, 한글 검사기의
기존 허용 동작은 이 작업에서 변경하지 않았다. 조사 중 선언 앞에 새 문단을 넣어
의도치 않게 구역이 추가된 초기 파일들은 위 대조에서 제외했다.
2026-10-23 후속 작업에서는 한글의 엄격 검사도 고아 번호 태그를 거부하도록 맞췄다.
위 표는 변경 전 기록이다. 현재 판정과 조건부 필드 보존 결과는
[기능 동등성 검토](rhwp-parity-2026-10-23.md)를 참고한다.
미사용 원형·분기, 잘못된 서식 ID, 태그가 제어문자를 가로지르는 경우도 단위 검사했다.
검사 통과는 실제 글꼴 설치·원고별 폭·페이지 잘림을 보장하지 않는다.

## 기본 모델로 생성

`md2hwp-rhwp init-template [[--output] <template.hwp>]`를 추가했다.
기본 경로는 현재 작업 폴더의 `template.hwp`이다. 기존 파일·폴더를 거부하며,
게시 시점에도 덮어쓰지 않는 방식으로 경합을 막는다.

기존 보고서, 추적 템플릿, upstream의 빈 문서 fixture를 읽거나 복사하지 않는다.
A4·함초롬바탕 TTF 10pt·160% 문단 모델과 역할별 별도 스타일에서 모든 필수 선언을
작성한다. 그림만 rhwp 모델 생성 API에 직접 만든 1픽셀 PNG를 전달하며,
표·native 캡션과 AUTONUM·각주·참조 샘플은 명시적 모델로 구성한다.
임시 저장 후 재열기, 독립 기대 모델 대조, 내장 이미지 바이트 대조와 엄격 검사를
통과해야 게시한다. 기본 스타일은 사용자 보고서의 서식을 재현하려는 것이 아니다.

실제 한글 대조에서 다음 차이를 찾아 생성 모델에 반영했다. 검사를 완화하지 않았다.

- 구역의 전체·짝수·홀수 PAGE_BORDER_FILL 세 레코드와 유효한 1-based 테두리 참조가
  필요했다. 누락 모델은 rhwp 자체 재열기를 통과해도 한글이 열기를 거부했다.
- 단 간격 0은 한글 XML 재입력에서 1134로 바뀌므로 한글 새 문서의 기본값을 명시했다.
- TTF 유형 표시를 넣어 표 자동 폭 계산이 기본 글꼴을 HFT/미상으로 거부하지 않게 했다.
- 문자 테두리의 투명 brush와 숨은 Solid 대각선 기본값을 명시했다.
  누락하면 한글의 CharShape 재적용·상호참조 삽입 때 새 테두리 정의로 바뀌었다.
- 그림 샘플은 실제 속성 비트와 모델의 Picture 번호 유형·TreatAsChar를 함께 작성했다.
  재열기 후 이 값도 명시적으로 검사한다.

최종 새 템플릿은 양쪽 엄격 검사를 통과했다. `examples/all-features/all-features.ir.json`을
동일 템플릿으로 양쪽에서 생성해 각각 저장·재열기 검증을 통과했다.
한글 300dpi 페이지 이미지는 양쪽 모두 6쪽이다. 1·2·3·5·6쪽의 픽셀 차이는 0이다.
4쪽은 JPG·progressive JPEG 그림이 있는 페이지로 3,627,068픽셀이 달랐다.
해당 쪽을 나란히 확인했으며 배치와 캡션·출처는 같고 사진 표시가 달랐다.
이는 기존 [JPEG 재인코딩 대조](rhwp-integration-2026-10-22.md)의 차이와 같은 유형이며
전체 출력의 픽셀 일치를 주장하지 않는다. 새 템플릿 2쪽과 결과의 1·2·4·6쪽도 시각 확인했다.

## 재현과 결과

```powershell
pwsh -NoProfile -File tools/development/build.ps1 -Configuration Release -Rhwp
.\target\release\md2hwp.exe init-template 새템플릿.hwp --worker .\target\release\md2hwp-rhwp.exe
.\target\release\md2hwp.exe validate-template 새템플릿.hwp --worker .\target\release\md2hwp-rhwp.exe
.\target\release\md2hwp-rhwp.exe examples/all-features/all-features.ir.json 결과.hwp --template 새템플릿.hwp
```

Rust workspace 276개 테스트, Clippy `-D warnings`, fmt, C# 계약 검사,
정상 Debug·Release 빌드를 통과했다. Windows 임시 실행 파일 및 게시 테스트는
샌드박스에서 접근 거부가 나 호스트에서 다시 실행해 통과했다.
추적 `templates/template.hwp`의 SHA-256은 아래 값으로 유지했다.

`20BC2EC532692D09CD2A40A67DDBBC24190FD87033EFADEB884FA48D31FE6B81`

결과는 ignored `artifacts/rhwp-integration/`에 두었다. 프로그램에는 포함하지 않는다.

| 파일 | 바이트 | SHA-256 |
| --- | ---: | --- |
| `template-rhwp-final4.hwp` | 7,168 | `3cde377b991c6c12a92a81dc41d41d52e19903a761e438c204ec02ac1dd104d6` |
| `rhwp-default-all-final4.hwp` | 3,969,024 | `7a114fd4a57c9a7b51c9ad0bab6c4fdc5cecffd70710b54bc2988b3a22175361` |
| `hancom-default-all-final4.hwp` | 4,883,968 | `dab95264cb34f8a0b87124c2d4acd408ff2fb07f7e4c6b61335de6fdc60777cc` |

검사 대조 로그: `template-validation2/results.json`.
페이지 대조 로그: `default-template-page-comparison.json`.
페이지 이미지·HWPML: 각 최종 파일의 `*-pages/` 폴더.
기존 긴 TreatAsChar 박스의 쪽 넘김 잘림 등 이미 기록한 레이아웃 한계는 그대로다.
