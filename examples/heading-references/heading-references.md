---
title: 헤딩별 고정 번호 참조 검증
md2hwp-heading1-start: 3
---

앞쪽 참조: [장](#chapter4), [절](#section32), [3단계](#level3), [4단계](#level4), [5단계](#level5), [6단계](#level6).

# 첫 장 {#chapter3}

## ID 없는 첫 절

이 절도 번호에 포함합니다.

## 두 번째 절 {#section32}

### 세 번째 수준 {#level3}

#### 네 번째 수준 {#level4}

##### 다섯 번째 수준 {#level5}

###### 여섯 번째 수준 {#level6}

다른 장의 **[절](#section41)을** 참조하고, *[장](#chapter4)도* 참조합니다.[^heading]

[^heading]: 각주에서 [절](#section32)과 [하위 제목](#level6)을 참조합니다.

![그림 캡션에서 [절](#section32) 참조](sample.png){#fig:heading}

출처: [다른 장](#chapter4).

참고: [여섯 번째 수준](#level6).

Table: [절](#section32)과 [다른 장](#chapter4)의 표

{#tbl:heading}
| 참조 | 설명 |
| --- | --- |
| [절](#section41) | [장](#chapter3) |
| [하위 제목](#level6) | 각주[^cell] |

출처: [세 번째 수준](#level3).

[^cell]: 셀 각주에서 [절](#section32)을 참조합니다.

```
제목: 번호 참조 검증 박스
원문은 그대로 보존합니다.
```

출처: [절](#section32).

# 다음 장 {#chapter4}

## 첫 절 {#section41}

뒤쪽 참조: [이전 장](#chapter3), [이전 절](#section32), [그림](#fig:heading), [표](#tbl:heading).
