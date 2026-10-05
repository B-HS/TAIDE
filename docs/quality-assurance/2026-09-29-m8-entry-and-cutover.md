# M8 Rust-native 착수·TypeScript 제거 게이트

## 대상 파일과 현재 판정

기준은 `docs/roadmap-rust-native.md` Phase 3~9, `docs/acknowledge/2026-09-23-rust-native-transition-contract.md` §2~4, `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`와 `docs/PROCESS.md` M8입니다. M1~M7 및 Phase 0 기준선은 완료됐지만 native UI 구현·기술 선정·beta·TypeScript 제거는 시작하지 않았습니다. 이번 문서는 착수 순서와 삭제 금지선을 고정할 뿐 제품 코드를 바꾸지 않습니다.

## 현재 코드 기준선

- 현재 workspace에는 `taide-ui`·`taide-editor` crate가 없고, `taide-terminal`은 세션·서비스 코드만 있습니다. native GUI 기술 의존도 아직 없습니다.
- `rg --files crates src-tauri/src`의 Rust 파일은 364개, `rg --files src`의 TypeScript 613개·TSX 262개입니다. 단순 파일 수는 Rust 사용 비율이 아닙니다.
- 테스트 경로와 생성 `src/shared/api/bindings.ts`를 제외한 임시 소스 줄 수는 Rust 82,349줄, TS/TSX 52,212줄입니다. 이 값은 공백·보조 코드까지 포함한 착수용 참고치이며 제품 런타임 비율의 공식 지표로 사용하지 않습니다.
- 기존 TS view inventory는 212개 경로였고 M7 성능 화면 1개가 추가돼 현재 census는 213개입니다. 항목별 native 대응·실기 결과는 아직 없습니다.

## 착수 순서와 통과 조건

- [ ] 기술 spike: egui/eframe·iced shell과 제한된 GPUI editor surface 후보를 공식 문서·동일 macOS Apple Silicon fixture로 검토합니다. IME 조합/취소, VoiceOver 역할·상태, 다중 창·포커스, drag/drop, 메뉴, 패키징·서명, editor 입력 지연을 먼저 비교하고 통과 후보만 별도 결정 문서에 확정합니다. 후보·의존성은 아직 선택하지 않습니다.
- [ ] Native shell vertical slice: 기존 `taide-runtime`/기능 crate를 재사용해 프로젝트·세션 복원, tree, tab/split, 명령 팔레트, 테마·로케일, 창 수명을 연결합니다. 현재 저장 데이터를 읽고 다시 기존 앱으로 복귀할 수 있는 양방향 호환을 확인합니다.
- [ ] Editor·LSP와 terminal: native editor의 transaction·undo·save/hot-exit·IME·LSP replay를 검증하고, terminal은 PTY 출력을 Rust core에서 한 번 파싱해 replay/live 순서·backpressure·selection/search·OSC·IME를 검증합니다. Monaco·xterm은 각각 동등성·성능·실기 통과와 fallback 기간 전에는 제거하지 않습니다.
- [ ] 나머지 기능·화면: 현재 213개 TS view 경로마다 native 대응 파일과 자동/실기 결과를 연결합니다. 파일·Git·검색·preview·plugin·agent·remote·IDE/CLI의 기능·보안 wire, 키보드·접근성·테마·로케일·보조 창을 0개 누락으로 확인합니다. 시각적 유사성만으로 기능 통과 처리하지 않습니다.
- [ ] Beta·cutover: 같은 기기·fixture에서 현행 v0.3.0 release 기준선보다 핵심 성능이 악화되지 않고, 장시간 세션·crash/restart·sleep/wake·대형 저장소·다중 창 회귀와 이전 앱 rollback을 통과한 뒤 native 앱을 기본값으로 전환합니다.
- [ ] TypeScript·Tauri 제거: 위 모든 게이트가 통과한 다음에만 React·TS source, Vite/Bun frontend build, Tauri command/event adapter, Specta TS 생성, Monaco/xterm·WebView 자산을 제거합니다. Rust-native 배포물의 서명·공증과 기존 데이터 migration/rollback을 다시 검증합니다.

## 99% 목표의 측정과 삭제 금지선

사용자의 “Rust 99%”는 전환 진척을 수치로 확인할 목표입니다. 구현 중에는 추적된 1차 제품 소스의 Rust LOC / (Rust LOC + TS/TSX LOC)을 같은 제외 규칙으로 비교하고, 테스트·fixture·생성 파일·문서·빌드 산출물은 분모에서 뺍니다. 첫 M8 slice에서 독립 `tools/migration-metrics`와 `tracked-product-nonblank-physical-v1` 정책을 구현했습니다. commit `2824005`의 공식 기준값은 Rust 42,530 / TS·TSX 47,336줄, 47.3260%이며 정의·명령·검증은 `docs/utils/migration-metrics.md`에 기록합니다. 위 임시 참고치와 제외 규칙이 달라 직접 비교하지 않습니다. 그러나 최종 완료 계약은 이 비율보다 엄격합니다. 앱 런타임의 TS·React·Tauri·Monaco·xterm 참조와 빌드 자산을 0건으로 만들고 native 동등성·성능·보안·패키징·beta를 통과해야 합니다. 이 조건 전에는 TypeScript 파일이나 기존 Tauri 앱을 삭제하지 않습니다.
