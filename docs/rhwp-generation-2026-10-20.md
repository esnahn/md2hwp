# rhwp 최소 생성 시험 — 2026-10-20

브랜치 `codex/rhwp-backend`, IR 0.4, rhwp 0.8.7의 고정 커밋
`1a76570e833917d15817415a53c09ad61ab3203f` 기준이다.
HWP 전용 `md2hwp-rhwp.exe`가 직접 문서를 생성하며 한글 COM을 호출하지 않는다.
아래 독립 검증에서만 한글 2020 11.0.0.9136을 사용했다.

## 입력과 방법

기존 `init-template`으로 별도 평문 제목 템플릿을 만들고, 첫 정적 문단에
title, author, date-meta fmt 태그를 추가했다. 변경 전후 저장·재열기로
표 구조와 기존 native 개체 수를 확인했다. 저장소 기본 템플릿은 변경하지 않았다.

원고는 heading1 두 개와 heading2–heading6 각 하나, 일반 문단,
Strong/Emph, 링크 표시 문구, 명시적 줄바꿈, 한글·영문·이모지,
태그처럼 생긴 본문 리터럴을 포함한다. 메타데이터에는 제목, 저자 목록,
한국어 날짜와 publisher를 넣었다. 같은 IR과 같은 템플릿으로 두 백엔드를 실행했다.

## 결과

| 검사 | 결과 |
| --- | --- |
| rhwp 임시 저장 → 디스크 재읽기 → rhwp 재열기 모델 비교 | 통과 |
| 기존 `--worker`를 통한 생성 및 버전 조회 | 통과 |
| 한글에서 두 출력 재열기 | 모두 성공, 각각 1쪽 |
| 본문 구조와 실제 서식 정의 비교 | 통과 |
| 원래 스타일·글꼴·CHAR 텍스트·필드·내장 데이터 비교 | 통과 |
| 한글 300dpi 페이지 이미지 비교 | 2480×3508, 다른 픽셀 0개 |
| 전체 1쪽 시각 검토 | 제목·본문·강조·줄바꿈·이모지·메타데이터 표시 일치 |
| HWPX 템플릿/출력, 복합 제목 블록, 원본 덮어쓰기 거부 | 통과, 기존 출력 보존 |

최초 생성에서는 수정한 문단에 샘플의 옛 줄 배치 캐시가 남아 모델 비교가 실패했다.
upstream serializer는 수정된 텍스트의 오래된 줄 좌표를 저장하지 않는다.
생성 모델에서도 수정 문단의 해당 캐시를 비우도록 고쳤다. 비교 조건을 완화하지
않았으며, 수정 없는 정적 문단의 캐시는 보존한다. 회귀 테스트에 두 경우를 포함했다.

Rust 전체 121개 테스트, `cargo clippy --workspace --all-targets -- -D warnings`,
기존 C# 계약 테스트, 제3자 고지 검사(308개 패키지)가 통과했다.
빌드는 `tools/development/build.ps1 -Configuration Debug -Rhwp`로 수행했다.
Release 빌드와 전체 기능·대용량 원고 생성은 이번 시험 범위에 없다.

## 자료

로컬 결과는 ignored 경로 `artifacts/rhwp-generation/`에 있다.
`plain.md`, `plain.ir.json`, `meta-template.hwp`, `hancom.hwp`, `rhwp.hwp`,
`hancom-pages/`, `rhwp-pages/`, `structure-comparison.json`,
`pixel-comparison.json`, `negative-tests.json`을 보존했다.

| 파일 | SHA-256 |
| --- | --- |
| plain.md | `DA7A9F003D2431327A7CB9C076FD159F59EE48A282662EB8502FFC3A96394EA0` |
| plain.ir.json | `33BC4F8C9E189CE3DA1C106BF4CE4969383043C5C6877D0CF0A0A8502656F83F` |
| meta-template.hwp | `3DABDC2588257CDC833CCB7A851AF3278F606BE0CE5D2394E9743C84B92423A3` |
| hancom.hwp | `11D597F8D123FEB34CCFF019AFF8483DB82B2E976A494789B8F0CED1283CD323` |
| rhwp.hwp | `DF8728D3C1C664F331BB9532B09E64CBA37559A62D10A9E0DA5A86C6057505BD` |

이 결과는 최소한의 평문 제목·본문 생성에 대한 증거다. 복합 제목, 목록,
코드, 그림·표, 각주, 상호참조는 아직 생성하지 않고 명시적으로 거부한다.
여러 페이지의 쪽 나눔 안전성은 미검증이다. 사용하지 않는 native 원형의 상세
검증도 기존 validate-template과 동등하지 않다. 현재 AURI 기본 템플릿의
지원 완료를 의미하지 않는다.
