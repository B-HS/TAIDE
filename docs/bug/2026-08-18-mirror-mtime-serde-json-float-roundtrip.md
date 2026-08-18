# mirror conflict 오탐 — serde_json f64 유손실 파싱 (2026-08-18)

## 대상 파일
- `src-tauri/Cargo.toml`(serde_json 피처) — 수정
- `src-tauri/src/domain/file/service.rs`(`mirror_dirty`·`list_mirrors`·conflict 비교 :199·:204·:236·:242) — 결함 발현 지점

## 증상
- 테스트 `mirror_dirty는_호출_시점의_실제_디스크_mtime을_baseline으로_반환한다`(service.rs:688 conflict assert)가
  `cargo test --lib` 에서 산발 실패(~5~7%). 세션 내내 "병렬 mtime flaky·무관"으로 치부돼 verify 게이트를
  간헐 파손. **단독 `--test-threads=1` 200회 중 14회(7%) 실패 — 병렬성과 무관**(진단이 "단독은 항상 통과"를
  표본 착시로 반증).
- **프로덕션 영향(잠재 데이터 무결성 버그)**: 앱 재시작 후 미러 복원(`list_mirrors`) 시 디스크가 전혀 바뀌지
  않았어도 약 5% 확률로 가짜 "디스크 변경됨" conflict 배지.

## 근본 원인 (진단 wf_1798253b-8ad, fable+high 읽기전용 — 메인 실물 재검증)
- serde_json 을 `float_roundtrip` 피처 없이 사용(Cargo.toml:28 `serde_json = "1"`, 실측 피처
  raw_value/std/alloc 만). 기본 fast-path 파서는 f64 를 약 5.5% 확률로 **1 ULP 낮게** 파싱(라이브러리
  문서에 명시된 best-effort 동작). 실측: mtime 형태 f64 10만 개 왕복 시 exact 88,887 / lower 5,563 / higher 5,550.
- 체인: `mirror_dirty` 가 stat 으로 baseline f64 `b` 저장 → `write_json`(ryu 최단표현) 직렬화 → `list_mirrors`
  의 `from_slice` 가 `b' = b − 2^-12ms` 반환 → conflict 비교 `현재 stat(정확한 b) > baseline(b')` → conflict=true.
- 파일시스템 mtime 드리프트 기각(stat→write→stat 16,000회 드리프트 0). temp_dir 병렬 공유 기각(UUID 유니크).

## 수정
- `Cargo.toml`: `serde_json = { version = "1", features = ["float_roundtrip"] }`. 파싱만 정확해지고
  직렬화 불변, 성능 비용은 이 워크로드에서 무의미. **신규 크레이트 아님**(기존 serde_json 피처 활성화 —
  승인 규칙 무관). 테스트 epsilon 우회·정수 절삭(IPC 계약 변경)은 차선으로 기각(HACK 금지·근본 해결).

## 검증
- 수정 후 해당 테스트 30회 반복 전부 통과(수정 전 200회 중 14회 실패). `float_roundtrip` 왕복 10만 개 전부 exact.
- 회귀 표면: 앱 전체 serde_json 파싱(settings.json·session.json·mirror·플러그인). "f64 파싱 정확해짐"뿐,
  기존 통과 테스트(conflict 마진 1일 규모)는 무영향.
