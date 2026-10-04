---
title: 상호참조 검증
md2hwp-heading1-start: 3
---

# 첫 장 {#chapter-one}

다음 장의 [그림 참조](#overview)를 먼저 참조합니다. [개요 참조](#section-one)와 [한글 ID 참조](#절-둘)도 확인합니다.

## 첫 절 {#section-one}

![첫 번째 그림](image.png){#first}

출처: 작성자 작성.

![두 번째 그림](image.png){#second}

![세 번째 그림](image.png){#third}

![네 번째 그림](image.png){#fourth}

같은 장의 [그림 참조](#fourth)를 참조합니다.
표시문 없이 [](#fourth)와 [](#section-one)를 참조할 수도 있습니다.

- 목록에서도 [그림 참조](#first)를 참조합니다.
- **강조한 [그림 참조](#second)** 와 *[개요 참조](#section-one)* 를 확인합니다.

각주에서도 참조합니다.[^reference]

[^reference]: 다른 장의 [그림 참조](#overview)와 [개요 참조](#section-one)를 확인합니다.

# 다음 장 {#chapter-two}

## 다음 절 {#절-둘}

![다음 장의 그림](image.png){#overview}

앞 장의 [그림 참조](#fourth)와 [개요 참조](#section-one)를 참조합니다.
외부 [링크](https://example.net)는 기존처럼 표기 텍스트로 출력합니다.
