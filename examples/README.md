# 예시와 검증 자료

처음 시험할 때는 짧은 [`all-features/all-features.md`](all-features/all-features.md)를
사용하십시오. 전체 지원 기능을 서로 다른 두 장에 나누며 같은 원고를 반복하지 않습니다.
기존 `all-features-twice` 통합 원고를 대체합니다.

| 목적 | 자료 |
| --- | --- |
| 전체 기능 빠른 검증 | [`all-features/`](all-features/) — 원고·예상 IR·PNG/JPG/JPEG/EMF |
| 대용량·긴 표·성능 | [`korean-lorem/`](korean-lorem/) — 기존 large-5x 원고·IR·이미지와 배치 문제 재현 자료 |
| 각주·메타데이터 회귀 | [`footnotes/`](footnotes/), [`metadata/`](metadata/) — Rust 테스트가 예상 IR을 직접 포함 |
| 번호·캡션·하단 설명 회귀 | [`cross-references/`](cross-references/), [`heading-references/`](heading-references/), [`caption-references/`](caption-references/), [`table-references/`](table-references/), [`all-features/object-sources.md`](all-features/object-sources.md) |
| 표 회귀 | [`tables/`](tables/) |
| 이미지 형식별 회귀·생성기 | [`jpeg/`](jpeg/), [`emf/`](emf/) |
| IR 정규화·거부 규칙 | 이 폴더 루트의 JSON — 일부는 Rust 테스트가 직접 포함 |

MD·예상 IR·그림·유지 중인 생성기는 Git으로 추적합니다. 생성한 HWP/PDF와 실행 로그·페이지
이미지는 제외합니다. 기능별 회귀 자료를 전체 기능 원고와 별도로 두는 이유는 특정 오류를
작은 입력으로 재현하고 자동 테스트의 입력을 유지하기 위해서입니다.
