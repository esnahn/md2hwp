---
title: 캡션과 출처의 상호참조 검증
subtitle: 여러 출처 문단과 앞뒤 참조
author: md2hwp
date: "2026-10-08"
publisher: md2hwp 예제
md2hwp-report-number: "검증 2026-02"
md2hwp-literal: "문자 치환"
md2hwp-heading1-start: 3
---

# 캡션 검증

## 참조할 절 {#sec-target}

본문의 [절](#sec-target)과 [그림](#second-figure), 각주[^note]를 확인합니다.

![첫 그림 — [절](#sec-target), **[다음 그림](#second-figure)**](sample.png){#first-figure}

출처: 첫 문단의 [그림](#second-figure).

참고: 두 번째 문단의 *[절](#sec-target)* 및 **[그림](#first-figure)**.

```
제목: 코드 박스
원문 한 줄입니다.
두 번째 줄입니다.
```

출처: 코드 출처의 [절](#sec-target).

비고: 코드의 두 번째 출처에는 **[그림](#second-figure)**.

Table: 표 캡션의 [절](#sec-target)과 *[그림](#second-figure)*

| 항목 | 값 |
| --- | --- |
| 셀의 [그림](#first-figure) | 셀의 [절](#sec-target) |

출처: 표 출처의 [그림](#second-figure).

참고: 표의 두 번째 출처에는 **[절](#sec-target)**.

![참조 대상 두 번째 그림](sample.png){#second-figure}

출처: 두 번째 그림의 [첫 그림](#first-figure).

[^note]: 본문 각주 안의 [그림](#second-figure)과 [절](#sec-target).
