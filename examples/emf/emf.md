---
title: EMF 그림 검증
subtitle: 내장·재열기·캡션·출처·상호참조
author: md2hwp
date: "2026-10-09"
publisher: md2hwp 검증 예제
md2hwp-report-number: "EMF-2026-01"
md2hwp-literal: "{{md2hwp:meta:title}}"
md2hwp-heading1-start: 3
---

# 첫 번째 장

## 일반 EMF {#emf-section}

다음 장의 [그림](#emf-only-repeat)과 현재 장의 [그림](#emf-only)을 참조합니다.[^emf-note]

![일반 EMF — 선·한글·클리핑·회전·투명도·비트맵](EmfOnly.emf){#emf-only}

출처: md2hwp에서 생성한 재현용 그림

주.1: **한글**과 Windows GDI+의 표현을 비교합니다. [다른 그림](#emf-dual)도 확인합니다.

## EMF+ Dual

![EMF+ Dual — GDI와 GDI+ 명령](EmfPlusDual.emf){#emf-dual}

Source: md2hwp fixture generator

note2: [절 번호](#emf-section)와 [그림 번호](#emf-only)를 확인합니다.

## EMF+ Only

![EMF+ Only — GDI+ 명령](EmfPlusOnly.emf){#emf-plus}

출처: md2hwp에서 생성한 재현용 그림

# 두 번째 장

## 일반 EMF 재사용

[앞 장의 그림](#emf-only)과 [현재 장의 그림](#emf-only-repeat)을 참조합니다.

![같은 일반 EMF를 다시 내장](EmfOnly.emf){#emf-only-repeat}

출처: 같은 원본 파일의 반복 삽입

## EMF+ Dual 재사용

![같은 EMF+ Dual을 다시 내장](EmfPlusDual.emf){#emf-dual-repeat}

출처: 같은 원본 파일의 반복 삽입

## EMF+ Only 재사용

![같은 EMF+ Only를 다시 내장](EmfPlusOnly.emf){#emf-plus-repeat}

출처: 같은 원본 파일의 반복 삽입

[^emf-note]: 각주 안에서 [그림](#emf-plus)과 [절](#emf-section)을 참조합니다.
