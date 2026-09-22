# Markdown 그림·박스 출처

그림 또는 박스 바로 다음에 빈 줄을 두고 `출처: …` 문단을 쓰면 해당 객체의
출처로 연결된다. 원고에는 `출처:`를 한 번 쓰고, 출력 접두어는 템플릿이 결정한다.

````markdown
![도시 **맥락**](../assets/sample-urban-context.png "그림 설명")

출처: **작성자** · https://example.com

```
박스 첫째 줄

빈 줄 다음 줄
```

출처: *현장 조사* 2026
````

- 그림은 독립된 문단에 하나만 둔다. `![…]` 안의 내용이 캡션이 된다.
- 출처의 강조·링크는 IR에 보존된다. 현재 HWP 조사는 링크 레이블을 출력한다.
  URL을 직접 표시하려면 예제처럼 링크 문법 없이 URL을 적는다.
- 출처가 없으면 기존처럼 템플릿의 `출처:` 자리만 남는다.
- 일반 본문의 `출처:`는 변경하지 않는다. 객체 직후 한 문단만 출처로 소비한다.
- `출처:`만 쓰면 오류다. 접두어 뒤에는 공백과 내용이 필요하다.
- 이미지 경로는 Markdown 파일 기준이며 IR 저장 위치가 달라지면 자동 환산한다.
  현재 HWP 경로는 저장소 안의 실제 PNG만 사용한다.
- 표도 같은 인접 출처 규칙을 사용하도록 정했다. **표 생성 자체는 아직 지원하지 않는다.**
  기본 CommonMark의 파이프 표는 일반 문장으로 해석되며 HWP 표가 되지 않는다.

## 실행

저장소 루트에서 다음 명령을 실행한다. 출력은 새 경로를 사용한다.

```powershell
cargo run -p md2hwp -- md2ir --from commonmark `
  --input .\examples\commonmark-sources-v0.2.md `
  --output .\artifacts\my-sources.ir.json

pwsh -NoProfile -File .\tools\development\dotnet.ps1 build `
  .\tools\investigation\hancom-automation\ir-preview\Md2Hwp.HancomIrPreview.csproj `
  --configuration Debug

& 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' `
  -NoProfile -ExecutionPolicy Bypass -Sta `
  -File .\tools\investigation\hancom-automation\ir-preview\run-ir-preview.ps1 `
  -Mode render-tagged -Ir .\artifacts\my-sources.ir.json `
  -Template .\tests\fixtures\templates\minimal-tagged-v1.hwp `
  -Output .\artifacts\my-sources.hwp -Configuration Debug
```

한글 단계는 기존 [검증된 사용자 세션과 보안 모듈](environment.md)을 사용한다.
창을 숨겨 실행하며 서비스 실행은 지원하지 않는다. 기존 태그 템플릿 파일을
그대로 사용한다. 외부 프로필 JSON이나 템플릿 재작성은 필요하지 않다.

## 검증 기록

2026-09-23에 예제 Markdown → Pandoc 3.10.1 → IR 0.2 → HWP를 실행했다.
Windows PowerShell 5.1.26100.9444 x64 STA, MOLIT 사용자 세션 1에서
열기 전용 probe가 성공했으며 모듈·템플릿은 변경하지 않았다.

`artifacts/commonmark-sources-v0.2.hwp`를 재열고 다음을 검사했다.

- 그림 2개, 자동번호 캡션 1·2, 출처 유무와 강조.
- 박스 2개, 빈 줄 보존, 출처 기울임, 출처가 없는 박스의 자리 표시.
- 출처와 별개인 본문 문단 및 일반 본문의 `출처:` 문자열 보존.
- 기존 표지·고정 내용·스타일과 태그 템플릿 해시 보존.

전체 3쪽 PNG를 내보내 표지와 콘텐츠를 확인했다. 표지는 기존 PNG와 동일한
SHA-256 `8B12E3BC4909E9201617D853643F789BD7128F3ED67EA0B298BF7C6D5908D56B`다.
박스 출처는 객체의 기존 한글 출처 문단에 삽입하며, 문단의 스타일·구조를
보존하고 문자별 굵게·기울임을 별도로 검증한다.

`tests/fixtures/ir/box-source-slots-v0.2.json`도 생성·재열기·페이지 확인을
통과했다. 박스 본문의 `출처:`와 본문/출처의 `{{md2hwp:slot:box.content}}`
문자열은 슬롯으로 재해석되지 않고 그대로 보존된다.

자동 검사는 아래 명령으로 재현한다. 마지막 명령은 COM을 사용하지 않는다.

```powershell
cargo test --workspace
pwsh -NoProfile -File .\tools\smoke\test-contracts.ps1
pwsh -NoProfile -File .\tools\smoke\test-commonmark-sources.ps1
```

닫힌 규약은 [IR 0.2](../specifications/ir-v0.2.md)를 참고한다.
