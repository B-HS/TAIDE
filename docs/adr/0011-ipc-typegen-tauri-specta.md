# ADR-0011 — IPC 타입 계약: tauri-specta 로 Rust→TS 자동 생성

- 상태: 승인 (사용자 확정, 2026-08-06 — RC 핀 고정 감수)
- 관련: ADR-0004, `docs/ipc-contract.md`, `docs/research/tauri-v2.md` §7

## 맥락

TAIDE 는 모든 도메인 상태를 IPC 로 오가게 하므로 command·event 가 수십 개 이상이다.
TS 타입을 손으로 미러링하면 반드시 드리프트가 난다(개인 acknowledge 의 `as unknown as never` 사례와
같은 계열의 위험). Rust 타입을 단일 출처로 TS 를 생성하는 도구가 필요하다.

## 결정

**tauri-specta `=2.0.0-rc.25`** (+ `specta =2.0.0-rc.25`, `specta-typescript 0.0.12`)를 채택하고
버전을 `=` 로 핀 고정한다.

- command: `#[specta::specta]` 부착, `collect_commands!` 로 수집, debug 빌드에서 `src/bindings.ts` export.
- event: `tauri_specta::Event` derive 로 이벤트 payload 까지 타입 계약에 포함.
- 에러: `thiserror + Serialize + Type` enum → 프론트에서 태그드 유니온 `Result` 로 수신.
- 생성물 `bindings.ts` 는 커밋한다(리뷰에서 계약 변화가 diff 로 보이게).

## 근거

- Rust 가 정본(ADR-0004)이므로 생성 방향(Rust→TS)이 아키텍처와 일치한다.
- Channel 타입·이벤트까지 생성 범위에 포함되어 3종 IPC 패턴 전부를 커버한다.

## 리스크와 완화

| 리스크 | 완화 |
|--------|------|
| 2.x 가 아직 RC(stable 없음) | `=` 핀 고정. rc 간 breaking 은 업그레이드를 명시적 작업으로만 수행 |
| `cargo add tauri-specta` 기본이 v1(1.0.2)을 설치 | 버전 명시 설치를 셋업 문서·스크립트에 고정 (research 함정 §1) |
| main 브랜치 예제가 미공개 rc.26 기준 | docs.rs rc 문서와 `docs/research/tauri-v2.md` §7 의 rc.25 예제만 참조 |
| 프로젝트 중단 가능성 | 생성물이 평범한 TS 파일이라 최악의 경우 생성만 멈추고 수동 유지로 전환 가능 |

## 기각한 대안

- **수동 타입 미러링**: 규모상 반드시 드리프트. 기각.
- **taurpc**: 유지보수 상태 미확인(research), 생태계가 더 작음.
- **zod 런타임 파싱**: 이중 정의(스키마+Rust)로 드리프트 문제가 그대로 남고 런타임 비용 추가.
