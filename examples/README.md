# 예시와 검증 자료

처음 시험할 때는 짧은 [`all-features/all-features.md`](all-features/all-features.md)를
사용하십시오. 전체 지원 기능을 서로 다른 두 장에 나누며 같은 원고를 반복하지 않습니다.
기존 `all-features-twice` 통합 원고를 대체합니다.

| 목적 | 자료 |
| --- | --- |
| 전체 기능 빠른 검증 | [`all-features/`](all-features/) — 원고·예상 IR·PNG/JPG/JPEG/EMF/SVG, 수식 |
| 대용량·긴 표·성능 | [`korean-lorem/`](korean-lorem/) — 기존 large-5x 원고·IR·이미지와 배치 문제 재현 자료 |
| 각주·메타데이터 회귀 | [`footnotes/`](footnotes/), [`metadata/`](metadata/) — Rust 테스트가 예상 IR을 직접 포함 |
| 하단 설명 회귀 | [`all-features/object-sources.md`](all-features/object-sources.md) |
| EMF 시험 그림 생성기 | [`all-features/generate-emf.ps1`](all-features/generate-emf.ps1) |
| IR 정규화·거부 규칙 | 이 폴더 루트의 JSON — 일부는 Rust 테스트가 직접 포함 |

MD·예상 IR·그림·유지 중인 생성기는 Git으로 추적합니다. 생성한 HWP/PDF와 실행 로그·페이지
이미지는 제외합니다. 전체 기능 원고가 다루는 기능의 개별 예제(상호참조·표·JPEG·EMF·SVG)는
삭제했고 Git 히스토리에서 볼 수 있습니다. 남은 기능별 자료는 자동 테스트의 입력이거나
전체 기능 원고가 다루지 않는 범위입니다.
