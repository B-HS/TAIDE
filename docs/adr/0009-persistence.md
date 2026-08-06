# ADR-0009 — 영속화: Rust 자체 serde JSON(원자적 쓰기) + window-state 플러그인

- 상태: 승인 (2026-08-06)
- 관련: `docs/data-model.md`, ADR-0004, `docs/research/tauri-v2.md`

## 맥락

세션·레이아웃·설정을 디스크에 저장해야 한다(FR-A5·B4). 후보: tauri-plugin-store(KV JSON),
자체 serde JSON 파일, SQLite.

## 결정

1. **도메인 영속 상태(session/project/layout/settings/buffers)는 자체 구현**: `infra/persist.rs` 에서
   serde JSON + 원자적 쓰기(tmp→fsync→rename) + `version` 마이그레이션 (`data-model.md` §4·§5 규격).
2. **윈도우 크기·위치·최대화는 `tauri-plugin-window-state` 에 위임**한다(멀티 모니터 엣지 케이스가
   검증된 공식 플러그인). `data-model.md` 의 `SessionState.window` 필드는 두지 않는다.
3. tauri-plugin-store 는 채택하지 않는다.

## 근거

- 우리 영속 상태는 typed struct(스플릿 트리 등)라 KV 스토어 API(store 플러그인)와 형태가 맞지 않고,
  마이그레이션·원자성 규격을 어차피 자체 정의해야 한다. serde 직렬화가 가장 직접적이다.
- 쓰기 주체가 Rust 뿐(ADR-0004)이므로 store 플러그인의 JS API 표면은 불필요하다 — 렌더러 노출
  최소화(NFR-7)에도 부합.
- SQLite 는 현 데이터 규모(수 KB~수백 KB JSON)에 과잉. 검색 인덱스 등 실수요가 생기면 그때 도입.

## 결과

- `persist.rs` 는 (경로, 타입, 마이그레이션 체인)을 받는 제네릭 로더/세이버 하나로 구현하고
  전 도메인이 공유한다.
- 저장 트리거는 도메인 상태 변경 시 debounce(기본 2s) + 앱 종료 flush. 단위 테스트로
  "파손 파일 → bak 복구 → 기본값" 경로를 검증한다.
