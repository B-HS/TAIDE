# Rust-native 전체 기능 crate 분리 실행 계약

> 브랜치: `to_rust_native`
> 상태: M1~M5 코드·결합 분리 완료, M6~M8과 GUI·실제 재시작 실기 미완료; Git commands/watch/plugin overlay·layout flush/이벤트/IDE·terminal 조립은 Tauri 경계, platform adapter는 M6 소유
> 상태 정본: `docs/PROCESS.md`의 「Rust-native 이전을 위한 전체 기능 crate 분리」
> 선행 근거: `docs/roadmap-rust-native.md`, `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`, `docs/architecture.md`

## 목표와 순서

현재 사용자 기능의 Rust 서비스와 자원 관리를 기능별 crate로 분리하고, 현행 Tauri 실행 경로·테스트를 보존한 다음 native UI를 시작합니다. 기존 `docs/acknowledge/2026-08-28-no-crate-split-decision.md`의 단일 소비자 전제는 이번 native 앱 추가 요청으로 바뀌었습니다. 전체 이동은 한 번에 하지 않습니다. 각 변경은 자체 테스트·기존 공개 API·wire 계약의 green 상태에서 끝납니다.

2026-09-24 사용자 추가 결정: 특정 crate 수·이름을 미리 강제하지 않고 **기능별 책임·실제 의존 DAG에 맞게** 나눕니다. `taide-model`은 공통 순수 DTO 경계이지 모든 기능 구현의 단일 도착지가 아닙니다. UI와 독립적인 기존 기능 서비스·데이터·프로토콜·자원 경계를 이관·검증하고 Phase 0 기능/데이터/성능 baseline과 M1~M7이 통과하기 전에는 native UI 구현을 시작하지 않습니다. 화면에만 존재하는 기능은 이 단계에서 제거하지 않고 TS view inventory·동작 계약으로 고정합니다. UI 착수 뒤에는 현재 TS view 전수 inventory의 각 화면·상태·상호작용·단축키·멀티윈도·테마/로케일·접근성·시각적 구성 요소를 빠짐없이 native 화면으로 대응시킵니다. 자동·실기 parity가 완료되기 전까지 기존 TS/Tauri view는 유지합니다.

로드맵의 `taide-core`/`taide-infra`는 목표 역할이지 무조건 한 파일 묶음으로 고정된 crate 이름이 아닙니다. 구체 crate는 실제 의존 방향이 DAG가 되도록 기능별로 확정합니다. `src-tauri` 패키지 `taide`, 라이브러리 `taide_lib`, CLI `taide-cli`와 기존 frontend/IPC 계약은 전환 중 그대로 유지합니다.

## 소유권 지도와 의존성 차단

| 현재 영역 | 목표 책임·선행 조건 |
| --- | --- |
| `ids.rs`, `error.rs`, `paths.rs`, 도메인 `types.rs`, persistence schema | `taide-model`부터 시작해 경량 직렬화·ID·error crate로 이전. `events.rs`의 Tauri `Event` derive는 adapter에 남김. |
| `infra/*` | Tauri 미의존 infra crate. `asset_protocol`·`navigation_guard`는 platform adapter. `root_guard`·`watcher`·`self_write`·`asset_protocol`의 domain 역참조를 제거한 후 이동. |
| `project`, `layout`, `app`, `search` | 상호 DTO 의존을 model에서 먼저 해소하고 workspace core로 묶음. service의 `AppState`·Tauri 결합은 port로 절단. |
| `file`, `tree`, `git` | 파일·트리·Git 서비스 crate. root guard·persist·watcher 및 plugin overlay 소비를 port로 정의. |
| `settings`, `theme`, `locale`, `snippet`, `sync` | 설정·테마·언어·snippet 서비스와 sync aggregation을 하위 서비스 의존 방향으로 분리. `include_str!` 리소스는 소유 crate와 함께 이동. |
| `plugin`, `vsix`, `ai`, `agent`, `task`, `system`, `font`, `notification` | 기능별 정책·서비스 crate, 프로세스/OS adapter 분리. agent↔terminal 참조는 조립 계층에서 주입. |
| `lsp`, `terminal`, `ide`, `remote`, `window` | LSP/PTY process 및 protocol/UI lifecycle을 개별 port·adapter로 추출. layout↔ide, layout↔window의 실제 순환은 이전에 절단. `remote/dispatch.rs`의 전 도메인 호출은 상위 조립 계층에 둠. |
| `state.rs`, `events.rs`, `lib.rs` | `AppServices`·task/event/window port 및 composition root. 기존 Tauri 등록·이벤트 wire와 sidecar 경로는 유지. |

현재 `src-tauri/tests/domain_boundaries.rs`는 `src/domain`/`src/infra`를, `capability_symmetry.rs`는 등록 source를, `rust_native_phase0_contract.rs`는 Rust 원천 4곳을 텍스트로 스캔합니다. 파일 이동 시 scan root와 manifest를 같은 변경에서 갱신하고 빈 scan은 실패시킵니다. `bindings.ts` hash, 203 command, 30 event, raw channel 3종과 error wire code 6종은 불변 게이트입니다.

## 모든 이동의 테스트 계약

1. 새로운 경계·동작에는 가능한 한 실패하는 테스트를 먼저 작성하고 예상 실패를 기록합니다. 기존 기능 단순 이동은 이동 전 characterization test의 green을 확보한 뒤 그대로 이전합니다.
2. 구현과 기존 unit test를 함께 옮기고, 필요한 통합 테스트·직렬화 fixture를 같은 변경에서 추가합니다. 코드 변경에 테스트가 없는 wave는 완료 처리하지 않습니다.
3. facade는 기존 `crate::`/`taide_lib::` 공개 경로를 재수출합니다. Tauri command/event 타입·순서, 영속 데이터, 보안 정책을 변경하지 않습니다.
4. 변경 위험에 직접 맞는 새 crate unit·관련 integration/contract test부터 실행합니다. wave 종료 시 workspace 전체 tests·fmt·clippy와 필요한 frontend contract 검사로 확인합니다. 동일한 변경 상태에서 이미 성공한 검사는 반복하지 않습니다.
5. 앱 실행·재시작과 수동 IME/접근성/멀티윈도 확인은 실제 결과가 없으면 미완료로 남깁니다. 어떤 phase도 미래 앱의 동등성이나 성능을 추정해 완료하지 않습니다.

## M1 세부 checklist — `taide-model` 첫 vertical slice

- [x] A. 기존 Rust workspace 테스트와 생성 bindings digest를 측정했습니다: `cargo test --workspace` exit 0, 총 1,782개(taide lib 1,735개·통합 30개·CLI 17개), 실패 0·ignored 0. `bindings.ts` SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49`.
- [x] B. 경계·파사드 계약 테스트를 먼저 추가하고 의도한 red를 기록했습니다. `cargo test -p taide --test taide_model_extraction` exit 101(`taide_model` crate 미존재, 15개 컴파일 오류). 의존 검사 보강 중 구 파서의 dev/target 누락도 별도 red로 확인했습니다.
- [x] C. `ids.rs`와 `error.rs`를 원본 바이트 동일하게 `crates/taide-model`로 이전해 unit test 12개를 동반했습니다. 기존 모듈은 `pub use` 파사드이며 crate-local `rustfmt.toml`은 기존 설정과 같습니다.
- [x] D. Phase 0 error source scan·manifest 경로를 새 원천으로 이전했습니다. error code 6종·bindings digest `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변을 확인했습니다.
- [x] E. 신규 경계 6건·model 12건·기존 전체 Rust 1,782건 중 변경 후 총 1,787건(초기 +5)을 통과했고, 의존 gate 보강으로 경계 테스트가 1개 더 추가됐습니다. 최종 변경 상태에서 경계 테스트 6건, fmt·clippy(exit 0)를 다시 확인했습니다. 같은 상태의 기존 workspace 성공 증거는 재사용합니다. 독립 검토 PASS(구현 바이트 동등, Cargo 의존, lock, facade, wire 원천 확인). 프론트 소스·생성 bindings에 변경이 없으므로 frontend 전체 검사는 이번 M1의 독립 위험을 덮지 않아 실행하지 않았습니다. 앱 실행은 사용자 몫입니다.
- [x] F. 검증 결과와 남은 위험을 기록하고 관련 파일만 선별 commit·현재 브랜치 push합니다.

### M1 검증·이전 기록

| 항목 | 이전 | 현재 |
| --- | --- | --- |
| Rust workspace 테스트 | 1,782개(taide lib 1,735·통합 30·CLI 17), exit 0 | 첫 이동 후 1,787개, exit 0. 이후 경계 테스트 1개만 추가해 총 1,788개 예상; 최종 변경에서는 전용 경계 6건·clippy·fmt를 확인했고 workspace 전체 중복 실행은 생략. |
| model unit | 없음 | 12개 이동·통과 (원본 구현과 바이트 동일) |
| IPC contract | 7개 | 7개 통과, error 원천 경로만 변경 |
| bindings SHA-256 | `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` | 동일 |

실기·미완료: Tauri 앱 실행, 저장 데이터 실제 재시작·GUI 기능은 이번 순수 모델 이동에서 실행하지 않았습니다. Phase 0의 나머지 fixture와 성능 측정은 여전히 미완료입니다.

M2 이후는 각 기능의 실제 파일·테스트·자원 경계가 확정될 때 하위 checklist를 추가합니다. M1을 끝냈다는 사실만으로 crate 분리 전체나 native UI 착수 gate를 완료로 간주하지 않습니다.

## M2 첫 slice — `AppPaths` 경계 (자동 검증 완료)

- [x] A. 이전 전 `src-tauri/src/paths.rs:1-69`의 `AppPaths`는 표준 라이브러리 `PathBuf`와 이미 분리된 `ProjectId`만 사용했고, 같은 파일 `:71-108`에 경로 unit test 4개가 있었습니다. `src-tauri/src/lib.rs:7,34`의 공개 모듈·앱 호출과 `src-tauri/tests/session_restore.rs:10`의 기존 import는 facade로 보존했습니다. `src-tauri/src/state.rs:32-44`의 `FlushScope`는 별도 후속 slice로 유지합니다.
- [x] B. `src-tauri/tests/taide_model_paths.rs`에 두 공개 경로의 타입 동일성과 모든 경로 메서드 14개의 기존 출력 검증을 먼저 추가했습니다. `cargo test -p taide --test taide_model_paths`는 `taide_model::paths` 미존재(E0432)로 예상대로 exit 101이었습니다. 서버 ID·버전 입력을 새 정책으로 정규화하거나 경로를 변경하지 않습니다.
- [x] C. 기존 `paths.rs` 구현·unit 4개를 바이트 동일하게 `crates/taide-model/src/paths.rs`로 이전하고 `src-tauri/src/paths.rs`는 `AppPaths`를 재수출합니다. 기존 unit 4개는 이전 전·후 모두 통과했고, 새 crate 16개·경계 2개·세션 복원 8개·추출 경계 6개·IPC 계약 7개가 통과했습니다. 파일시스템 접근이나 새 dependency를 추가하지 않았습니다.
- [x] D. 새 crate unit 16개(이전 unit 4개 포함), 경계 2개, `session_restore` 8개, Phase 0 IPC 7개 및 기존 추출 경계 6개가 통과했습니다. `cargo test --workspace` exit 0, 총 1,790개(taide lib 1,719·통합 38·CLI 17·model 16), 실패·ignored 0; `cargo fmt --all --check` 및 `cargo clippy --workspace --all-targets -- -D warnings` exit 0. 경계·source-scan 테스트는 그대로 통과했고 bindings SHA-256은 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49`로 불변입니다. 생성 bindings·IPC 등록·Cargo 의존성·저장 경로는 변경하지 않았습니다.
- [x] E. `FlushScope`는 `ProjectId`·serde·specta만 의존함을 확인해 두 번째 slice로 이전했습니다. 이어 `theme/types.rs`(103줄), `locale/types.rs`(36줄), `snippet/types.rs`(50줄)는 서로·Tauri를 의존하지 않고 표준 `BTreeMap`·serde·specta만 사용하는 저장 데이터 DTO임을 확인해 세 번째 slice로 확정했습니다. 그 밖의 DTO와 M2 전체는 미완료입니다.

첫 slice 당시 제약: 사용자 지정 모델 `ollama-cloud/deepseek-v4.1-flash#max`의 `opencode run --agent explore-pen --model ollama-cloud/deepseek-v4.1-flash#max` 호출이 `Permission denied: shell`로 거부돼 메인이 직접 수행했습니다. 앱 실행·재시작은 사용자 몫입니다. M2 전체는 후속 slice가 끝나기 전까지 미완료입니다.

## M2 두 번째 slice — `FlushScope` 경계 (진행 중)

- [x] A. `src-tauri/src/state.rs:19-44`의 `FlushScope`는 이미 model crate에 있는 `ProjectId`·serde·specta만 사용합니다. `events.rs:505-528`의 all/window/project 직렬화·왕복 테스트와 생성 bindings의 타입 설명까지 확인했습니다. 이벤트 구조체와 Tauri `Event` derive는 기존 adapter에 남기고 `state::FlushScope` 경로를 보존합니다.
- [x] B. 기존 `events::tests::플러시_스코프` 2개 green을 먼저 확인했습니다. `src-tauri/tests/taide_model_flush_scope.rs`에 model↔facade 타입 동일성·Hash·all/window/project 레거시 wire fixture를 추가했고, `cargo test -p taide --test taide_model_flush_scope`는 `taide_model::flush` 부재(E0432, exit 101)로 의도대로 실패했습니다.
- [x] C. enum과 기존 문서 속성을 바이트 동일하게 `crates/taide-model/src/flush.rs`로 옮기고 `state::FlushScope`에서 재수출했습니다. variant·serde rename·specta 설명을 바꾸지 않았습니다. 신규 타입·wire 2건, 기존 이벤트 wire 2건, Phase 0 IPC 7건이 통과했습니다.
- [x] D. 새 모델↔facade 경계 2건, 기존 event wire 2건, Phase 0 IPC 7건, Rust workspace 총 1,792개(taide lib 1,719·통합 40·CLI 17·model 16)가 통과했습니다. fmt는 신규 테스트의 긴 행 때문에 첫 검사에서 실패해 해당 행만 고쳤고 재검사는 통과했습니다. clippy `--workspace --all-targets -- -D warnings` exit 0; 생성 bindings SHA-256은 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49`로 불변이며 source-scan 계약도 통과했습니다. 앱 실행은 사용자 몫입니다.
- [x] E. 검증된 파일 6개만 선별 commit `1b17864`·현재 브랜치에 일반 push했습니다.

이번 재개에서도 지정 모델의 `opencode run --agent explore-pen --model ollama-cloud/deepseek-v4.1-flash#max` 호출이 `Permission denied: shell`로 거부됐습니다. 모델·호출 방식을 바꾸지 않고 메인이 직접 진행합니다.

## M2 세 번째 slice — 저장 데이터 DTO(theme·locale·snippet, 자동 검증 완료)

- [x] A. `src-tauri/src/domain/{theme,locale,snippet}/types.rs`에 저장 파일용 타입·기본값·직렬화 속성이 있고, 이 세 파일 사이 의존·Tauri import·원천 경로 고정 source-scan은 없습니다. `docs/data-model.md:75-79`의 `themes/`, `snippets/`, `locales/` 저장 구역에 대응합니다. 세 facade의 기존 공개 경로는 보존합니다.
- [x] B. `src-tauri/tests/taide_model_persistence.rs`에 세 도메인의 model↔facade 동일 타입과 구버전 언어팩·테마의 기본값, 스니펫의 단일/배열·선택 필드 wire fixture를 먼저 추가했습니다. `cargo test -p taide --test taide_model_persistence`는 새 model 모듈 부재 13건(E0433, exit 101)으로 의도한 red였습니다.
- [x] C. 세 파일을 바이트 동일하게 model crate로 옮겼습니다. 원본에 unit은 없었고, `src-tauri/src/domain/{theme,locale,snippet}/types.rs`는 기존 API 전체를 재수출합니다. 신규 타입·구버전 wire 3건과 model unit 16건이 통과했습니다.
- [x] D. `cargo test --workspace --quiet` exit 0, 총 1,795개(taide lib 1,719·통합 43·CLI 17·model 16)가 통과해 기존 도메인 서비스·source-scan·IPC 계약도 포함합니다. fmt는 신규 테스트 행 길이 5곳 수정 뒤 재검사 통과, clippy `--workspace --all-targets -- -D warnings` exit 0. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변이고 원천 코드·저장 경로에 동작 변경이 없습니다.
- [x] E. 검증된 파일 10개만 선별 commit `f8bdd24`·현재 브랜치에 일반 push했습니다.

## M2 네 번째 slice — project/session 영속 타입 (진행 중)

- [x] A. `src-tauri/src/domain/project/types.rs:1-291`은 `SplitDir`(layout/types.rs:16-21)과 model crate에 있는 ID·serde·specta만 참조합니다. layout 방향 enum `SplitDir`·`DropEdge`는 다른 타입·Tauri를 참조하지 않아 선행 이동 가능한 DAG 경계입니다. `SessionState`의 기존 기본값·project.json의 필드 기본값·ShellSlotTree의 wire를 먼저 고정합니다. `layout::types`와 `project::types`의 공개 경로는 파사드로 유지하며 layout의 나머지 타입·unit 2개는 아직 이동하지 않습니다.
- [x] B. `src-tauri/tests/taide_model_project.rs`에 기존 소비 경로↔model 타입 동일성, 구버전 session/project/group 기본값과 슬롯 트리 wire를 먼저 작성했습니다. `cargo test -p taide --test taide_model_project`는 새 layout/project 모듈 부재 E0432/E0433(exit 101)로 의도한 red였습니다.
- [x] C. layout의 `SplitDir`·`DropEdge`를 model crate로 옮기고 layout 공개 경로를 재수출했습니다. 뒤이어 project/types.rs 전체 구현을 model crate로 이전해 `SplitDir` import만 새 crate 내부 경로로 갱신하고 rustfmt에 맞게 순서를 정리했습니다. project 공개 경로도 재수출합니다. 프로젝트 원본에는 unit이 없고 layout unit 2개는 원래 모듈에 유지했습니다.
- [x] D. 신규 project 경계·구버전 fixture 3건, `session_restore` 8건, IPC 7건, Rust workspace 총 1,798개(taide lib 1,719·통합 46·CLI 17·model 16) 통과했습니다. fmt의 project import 순서 1건을 고친 뒤 fmt·clippy `--workspace --all-targets -- -D warnings` exit 0. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다.
- [x] E. 검증된 파일 8개만 선별 commit `b131fe6`·현재 브랜치에 일반 push했습니다.

## M2 다섯 번째 slice — search 직렬화 타입 (자동 검증 완료)

- [x] A. `src-tauri/src/domain/search/types.rs:1-147`은 serde·specta만 사용하고 `SearchQuery`는 `src-tauri/src/domain/layout/types.rs:85`에서 저장 레이아웃의 검색 탭이 사용합니다. 검색/치환 결과·채널 wire도 이 파일 소유입니다. type 파일 자체에는 unit이 없습니다. 기존 `search::types::*` 공개 경로와 검색 동작은 보존합니다.
- [x] B. `src-tauri/tests/taide_model_search.rs`의 구버전 SearchQuery 기본값·치환 실패 사유·검색 채널 wire·model↔facade 타입 동일성 테스트가 신규 모듈 부재 E0432/E0433(exit 101)로 의도대로 실패했습니다.
- [x] C. `src-tauri/src/domain/search/types.rs` 전체 구현·속성을 바이트 동일하게 `crates/taide-model/src/search.rs`로 이전하고 기존 search facade에서 재수출합니다. 기존 파일에는 unit이 없었습니다.
- [x] D. 신규 검색 경계·구버전 fixture 2건, IPC 7건, `cargo test --workspace --quiet` 총 1,800개(taide lib 1,719·통합 48·CLI 17·model 16), fmt 및 clippy `--workspace --all-targets -- -D warnings` 통과. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다.
- [x] E. 검증된 파일 6개만 선별 commit `3355ae5`·현재 브랜치에 일반 push했습니다.

## M2 여섯 번째 slice — app 파일 대상 타입·프롬프트 ID (자동 검증 완료)

- [x] A. `src-tauri/src/domain/app/types.rs:1-84`는 serde·specta만 쓰지만 `PromptTemplateId::as_str`이 `src-tauri/src/domain/ai/prompt.rs:10-12`의 세 문자열 상수를 참조합니다. 해당 상수를 model app 타입과 함께 소유시키고 기존 ai::prompt 경로에서 재수출하면 Tauri 의존·cycle 없이 layout의 `AppFileTarget` 선행 DTO가 됩니다. 타입/IPC·프롬프트 파일 경로·기존 bindings 설명은 유지합니다.
- [x] B. `src-tauri/tests/taide_model_app.rs`의 enum wire·기존 프롬프트 ID 문자열·model↔facade 타입 동일성 테스트가 새 model app 모듈 부재 E0432/E0433(exit 101)로 의도대로 실패했습니다.
- [x] C. app/types.rs의 타입·원본 문서 속성을 model crate로 옮기고 세 prompt ID 상수의 문자열을 같은 모듈로 이전했습니다. 기존 app/AI 공개 경로는 각각 재수출로 유지하고 `PromptTemplateId::as_str`은 이전한 같은 상수를 참조합니다. `domain_boundaries.rs`의 제거된 `app/types.rs → ai::prompt` 허용 항목을 정리했습니다.
- [x] D. 신규 app 경계 2건·app service 9건·domain boundary 3건·IPC 7건·`cargo test --workspace --quiet` 총 1,802개(taide lib 1,719·통합 50·CLI 17·model 16) 통과했습니다. fmt는 신규 테스트 행 3곳 정리 뒤 통과, clippy `--workspace --all-targets -- -D warnings` 통과, bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다.
- [x] E. 검증된 파일 8개만 선별 commit `640d2b1`·현재 브랜치에 일반 push했습니다.

## M2 일곱 번째 slice — 레이아웃 영속 타입 (자동 검증 완료)

- [x] A. `src-tauri/src/domain/layout/types.rs`의 잔여 `TabKind`·`ProjectLayout`·2개 unit은 model에 앞서 이전한 `AppFileTarget`·`SearchQuery`·`SplitDir`·`DropEdge`·ID만 사용합니다. `layout.json`의 구버전 기본값·Diff 필드·AppFile 및 SearchEditor 변형을 먼저 고정합니다. 기존 `layout::types::*` 공개 경로와 레이아웃 서비스는 그대로 둡니다.
- [x] B. `src-tauri/tests/taide_model_layout.rs`에 구버전 `layout.json` 기본값·Diff/앱 파일/검색 탭 wire와 model↔facade 타입 동일성 테스트를 추가했고 `cargo test -p taide --test taide_model_layout`은 새 model 타입 부재(E0432, exit 101)로 의도한 red였습니다.
- [x] C. 선행 분리된 방향 enum에 나머지 `layout/types.rs` 구현·unit 2개를 합쳐 model `layout.rs`로 이전하고 기존 domain/types.rs는 재수출만 유지했습니다. 모델 파일의 나머지 구현은 이전 전 원본과 바이트 동일함을 확인했습니다.
- [x] D. 신규 구버전 layout/탭 fixture 2건, model unit 18건(이전 layout unit 2건 포함), `session_restore` 8건, IPC 7건, `cargo test --workspace --quiet` 총 1,804개(taide lib 1,717·통합 52·CLI 17·model 18) 통과했습니다. 신규 테스트 포맷 1건 수정 뒤 fmt·clippy `--workspace --all-targets -- -D warnings` 통과, bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다.
- [x] E. 검증된 파일 5개만 선별 commit `6fdd84f`·현재 브랜치에 일반 push했습니다.

## M2 여덟 번째 slice — file·tree·font 순수 wire 타입 (완료)

- [x] A. `src-tauri/src/domain/{file,tree,font}/types.rs`는 serde·specta·순수 타입만 참조하고 새 의존성 없이 model crate로 이동할 수 있습니다. file의 `FsChange`는 infra watcher/self_write가 기존 `file::types` 경로로 소비하므로 facade를 보존하고, tree/font도 기존 IPC 이름·필드·문서 속성을 유지합니다. 기존 파일들에 unit은 없습니다.
- [x] B. `src-tauri/tests/taide_model_file_tree_font.rs`에 file change·editorconfig·tree page·font wire와 양쪽 타입 동일성 3건을 먼저 추가했고 model 모듈 부재 E0432/E0433(exit 101) red를 확인했습니다.
- [x] C. file·tree·font 구현을 바이트 동일하게 model crate로 옮기고 각각의 domain/types.rs에서 재수출합니다. 기존 unit은 없었고 infra가 쓰는 `file::types` 경로를 유지했습니다.
- [x] D. 신규 경계 3건·domain boundary 3건·IPC 7건, `cargo test --workspace --quiet` 총 1,807개(taide lib 1,717·통합 55·CLI 17·model 18)와 fmt·clippy `--workspace --all-targets -- -D warnings` 통과. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다.
- [x] E. 검증된 파일 10개를 commit `52274d2`로 현재 브랜치에 반영했고 원격 HEAD와 일치합니다.

## M2 아홉 번째 slice — task·system 순수 wire 타입 (완료)

- [x] A. `task/types.rs`(TaskSource·Task)와 `system/types.rs`(SystemUsage·AppDataPathKind·SystemUsageProcessKind·SystemUsageProcess)의 기존 serde camelCase·공개 경로와 task/system 서비스 소비를 확인했습니다. 두 타입 파일은 serde·specta 외 Tauri·도메인 import가 없습니다.
- [x] B. model↔facade 타입 동일성·기존 작업 목록, `cpuPercent: null` 시스템·프로세스 사용량, app data 경로 enum wire 테스트 3개를 먼저 작성했고 model 모듈 부재 E0432/E0433(exit 101) red를 확인했습니다.
- [x] C. 타입 구현·속성을 원본 바이트 동일하게 model crate로 옮기고 기존 `domain::{task,system}::types::*` 경로를 재수출했습니다. 신규 테스트 3건과 IPC 계약 7건 통과. command, 서비스 로직, bindings는 변경하지 않았습니다.
- [x] D. 새 경계 3건·관련 task/system 서비스를 포함한 `cargo test --workspace --quiet` 총 1,810개, IPC 계약 7건, fmt·clippy `--workspace --all-targets -- -D warnings`가 통과했습니다. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변. GUI 실행·실제 재시작은 수행하지 않았습니다.
- [x] E. 검증된 파일만 선별 commit·현재 브랜치에 일반 push합니다.

## M2 열 번째 slice — VSIX 결과·보조 창 정보 DTO (완료)

- [x] A. `vsix/types.rs`의 첫 6개 경로·크기 상수는 파서에서 사용하고 결과 DTO 4종은 serde·specta만 사용합니다. `window/types.rs`는 창 크기·라벨 정책 상수와 `ProjectId`가 든 `AuxiliaryWindowInfo`로 나뉩니다. 기존 공개 경로와 command·service 소비를 확인했습니다.
- [x] B. VSIX 확장 정보·include chain과 보조 창 `projectId`·`windowSlot` wire/타입 동일성 테스트 2건을 먼저 작성했고 model 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 상수는 원래 도메인 파일에 남기고 DTO 본문은 원본 바이트 동일하게 model crate로 이전했습니다. 기존 `domain::{vsix,window}::types::*` 경로를 재수출하고 도메인 서비스·Tauri 창 로직을 수정하지 않았습니다. 새 경계 2건·IPC 계약 7건·도메인 경계 3건이 통과했습니다.
- [x] D. 전용 경계 2건·도메인 서비스 포함 `cargo test --workspace --quiet` 총 1,812개·IPC 계약 7건·domain boundaries 3건·fmt·clippy `--workspace --all-targets -- -D warnings` 통과. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다. 앱 실행·재시작은 수행하지 않았습니다.
- [x] E. 관련 파일만 선별 commit·현재 브랜치에 일반 push합니다.

## M2 열한 번째 slice — plugin manifest·상태 DTO (완료)

- [x] A. `src-tauri/src/domain/plugin/types.rs`의 상수 3개는 manifest 버전·파일 이름·grammar 크기 정책이며 같은 파일의 DTO 7개는 serde·specta·표준 타입만 사용합니다. `PluginErrorCode`는 kebab-case, 선택 기여와 오류는 `#[serde(default)]`입니다. 기존 공개 경로·service·IPC 소비를 확인했습니다.
- [x] B. 플러그인 구버전 manifest 기본값·언어/LSP optional 속성·로드 상태 오류 wire·타입 동일성 테스트 2건을 먼저 작성했고 새 model 모듈 부재 E0432/E0433(exit 101) red를 확인했습니다.
- [x] C. 지정 모델 sub-pen이 DTO 7개를 model crate로 옮기고 도메인 상수 3개·공개 경로를 보존했습니다. 첫 구현 호출은 출력 상한 초과, 재시도는 최종 JSON 파싱 오류로 FAILED여서 완료로 간주하지 않았습니다. 파일 실물·검증을 메인이 직접 확인하고 같은 모델 인수 작업 `taide-m2-plugin-adopt-20260924`의 수정 없는 DONE으로 종료 상태를 확정했습니다. 공유 트리는 에이전트 실행 중 메인이 수정하지 않았습니다.
- [x] D. DTO 구현 본문 2,187B 원본 바이트 동일·전용 경계 2건·IPC 7건·도메인 경계 3건·`cargo test --workspace --quiet` 총 1,814개·fmt·clippy `--workspace --all-targets -- -D warnings` 통과. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다. GUI 실기는 미실행입니다.
- [x] E. 관련 파일만 선별 commit·현재 브랜치에 일반 push합니다.

## M2 열두 번째 slice — AI provider·프롬프트 wire 타입 (완료)

- [x] A. `ai/types.rs`는 serde·specta만 import하고 `AiProviderId`를 `settings/types.rs`가 소비합니다. 요청/응답·저장 프롬프트 타입·`AiPromptVars` 2종과 원본 unit 3개는 같은 파일에 있습니다. 모델/토큰/요청 기본값 및 기존 공개 경로를 확인했습니다.
- [x] B. model↔기존 도메인 타입 동일성·구버전 provider/owner 요청·프롬프트 저장 wire 경계 테스트 2건을 먼저 작성했고 model 모듈 부재 E0432/E0433(exit 101) red를 확인했습니다.
- [x] C. 지정 모델 `sub-pen`(task `taide-m2-ai-implement-20260924`, session `ses_f30e49303ffeERgGBKB08G3G6M`)이 AI 타입·기존 unit 3개를 원본 바이트 동일하게 model crate로 옮겼습니다. 메인이 소유한 새 경계 테스트·문서와 Git은 수정하지 않았고 기존 facade/등록을 보존했습니다.
- [x] D. 메인이 AI 원본 208줄 바이트 동일성과 `cargo test --workspace --quiet` 총 1,816개(taide lib 1,714·model 21·CLI 17·기타 통합 64), fmt·clippy `--workspace --all-targets -- -D warnings`·diff 검사를 직접 통과했습니다. IPC 계약은 workspace의 기존 7건을 포함하며 bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변입니다. 앱 실기는 미실행입니다.
- [x] E. 관련 파일만 선별 commit·현재 브랜치에 일반 push합니다.

## M2 열세 번째 slice — settings 선택지 enum 경계 (완료)

- [x] A. `settings/types.rs:25-83`의 에디터 렌더 공백·커서 스타일·커서 깜빡임·터미널 커서 스타일 enum 4개는 serde·specta만 의존하며, 같은 파일의 `Settings`/`SettingsPatch`가 이를 소비합니다. 상수·default 함수·영속 필드·binding parity unit은 기존 파일에 유지합니다.
- [x] B. model↔facade enum 타입 동일성·camelCase wire/default fixture와 `Settings`/`SettingsPatch` 소비 테스트 2개가 새 model 모듈 부재 E0432(exit 101)로 red인 것을 확인했습니다.
- [x] C. 네 enum을 model의 settings 모듈로 분리하고 기존 공개 경로를 재수출했습니다. 구 도메인 service rustdoc 링크만 plain path로 정정했고 `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-model --no-deps`가 통과했습니다.
- [x] D. 전용 경계 2건, Phase 0 IPC 계약 7건, Rust workspace 1,818건·fmt·clippy·model rustdoc를 확인했습니다. `file.rs`의 선행 rustdoc 수정과 enum 문서 경로 변경은 생성된 `bindings.ts`의 설명 2줄만 변경하여 Phase 0 manifest 해시를 `0554f681b1f02cc1959439e451464799b071a2781524d6fa916e536ec2639c33`으로 갱신했습니다. JSON 파싱 오류로 구현 sub-pen 호출 2회가 실패했지만 소유 파일 구현이 남아 메인이 직접 diff·검증을 확인했습니다. 별도 인수 `taide-m2-settings-adopt-20260924`(session `ses_f2e87a7bcffe959QLmtgKQx7u2`)는 읽기 전용으로 변경 없이 DONE을 반환했습니다.
- [x] E. 관련 구현·경계 테스트·생성 bindings·Phase 0 manifest 7파일만 선별 commit `1159884`로 반영하고 현재 브랜치에 일반 push했습니다. 계약·체크리스트 문서는 별도 커밋에서 현행화합니다.

## M2 열네 번째 slice — notification 순수 wire 타입 (완료)

- [x] A. `src-tauri/src/domain/notification/types.rs`의 3개 enum은 serde·specta만 쓰며 서비스의 완료 정책과 별도입니다. 기존 서비스·명령·공개 경로와 도메인 doc 링크를 확인했습니다.
- [x] B. category·suppression·delivery의 model↔facade 타입 동일성, tag/content·camelCase wire와 구버전 데이터를 2개 테스트로 고정하고 model 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. enum과 설명을 model crate로 이전하고 도메인 파일은 재수출했습니다. stale 도메인 rustdoc 링크 2건만 일반 코드 경로로 고쳤습니다.
- [x] D. 새 경계 2건·`cargo test --workspace --quiet` 총 1,820건(IPC 계약 7건 포함)·fmt·clippy·strict model rustdoc가 통과했습니다. 생성 bindings의 설명 2줄만 바뀌어 manifest 해시를 `56f31885f4920d663972a80b1db640634d2fae126347e4914f1ecd262c6eeccc`으로 갱신했습니다. 구현 sub-pen 2회는 JSON 파싱·완료 근거 오류로 FAILED였지만 소유 파일 diff를 메인이 대조했고 읽기 전용 인수 `taide-m2-notification-adopt-20260924`(session `ses_f2e5ef48affekRM7q1hekuBzcR`)가 변경 없이 DONE을 반환했습니다.
- [x] E. 검증된 구현 6파일을 commit `36ef8e3`으로 현재 브랜치에 반영했습니다. 샌드박스 DNS 제한 해제 후 기록 commit `e05807d`와 함께 원격 `to_rust_native`에 일반 push했습니다.

## M2 열다섯 번째 slice — settings 영속 DTO (완료)

- [x] A. `Settings`·`SettingsPatch`는 선행 이전한 settings enum 4개와 `AiProviderId`만 참조하고, 상수·serde 기본값 함수도 표준 라이브러리 외 의존이 없습니다. 서비스의 sanitize·migration·apply 로직은 도메인에 남깁니다. `bindings.ts` field parity unit은 frontend 계약을 읽으므로 기존 settings facade에 유지합니다.
- [x] B. model↔facade 타입 동일성, 구버전 기본값과 patch wire fixture를 먼저 작성했고 model DTO 부재 E0432(exit 101) red를 기록했습니다.
- [x] C. 순수 DTO·상수·기본값을 model crate로 이전하고 기존 `domain::settings::types::*` 공개 경로를 재수출했습니다. frontend `bindings.ts` field parity unit은 facade에 남겼습니다.
- [x] D. 전용 경계 3건·settings 서비스 70건·`cargo test --workspace --quiet` 총 1,821건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. 생성 bindings는 이동된 rustdoc 경로 9곳만 바뀌어 manifest SHA-256을 `14c2b3af63b4222a5fabbdb41aae2827eacf8c0ea7cfb0f695509aaee51f3af2`로 동기화했습니다. sandbox 안 workspace 실행은 프로세스 조회·로컬 소켓·macOS 휴지통 권한으로 9건 실패했고, 제한 밖 동일 명령에서는 전부 통과했습니다.
- [x] E. 관련 7파일을 선별 commit `34deeae`로 현재 브랜치에 반영하고 기록 commit과 함께 일반 push합니다.

## M2 열여섯 번째 slice — sync 영속·wire DTO (완료)

- [x] A. `sync/types.rs`는 model로 이전된 `SettingsPatch`와 serde·specta만 참조합니다. GitHub API·secret·Tauri command/event 로직은 다른 파일에 있어 sync payload·status·download result와 두 상수를 독립적으로 옮길 수 있습니다.
- [x] B. model↔facade 타입 동일성, 구버전 payload 기본값과 status/download wire fixture를 먼저 작성했고 model sync 모듈 부재 E0432(exit 101) red를 기록했습니다.
- [x] C. sync 상수·DTO·기존 unit 2건을 model crate로 이전하고 기존 `domain::sync::types::*` 공개 경로를 재수출했습니다.
- [x] D. 전용 경계 2건·model unit 23건·`cargo test --workspace --quiet` 총 1,823건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. 생성 bindings와 SHA-256 `14c2b3af63b4222a5fabbdb41aae2827eacf8c0ea7cfb0f695509aaee51f3af2`는 불변입니다.
- [x] E. 관련 6파일을 선별 commit `599e98d`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 열일곱 번째 slice — Git wire DTO (완료)

- [x] A. `src-tauri/src/domain/git/types.rs` 214줄은 serde·specta 외 도메인 의존이 없고, Git 서비스·IPC가 기존 공개 경로를 사용합니다. Git 서비스 정책과 git2 의존은 원위치에 둡니다.
- [x] B. Git status·diff·commit legacy wire와 model↔facade 타입 동일성 테스트를 먼저 추가했고 model 모듈 부재 E0432(exit 101) red를 기록했습니다.
- [x] C. Git DTO를 model crate로 이전하고 기존 `domain::git::types::*` 경로를 재수출했습니다. 서비스 rustdoc 링크 1곳은 경로 텍스트로 보존했습니다.
- [x] D. 전용 경계 2건·`cargo test --workspace --quiet` 총 1,825건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. 타입 본문은 rustdoc 링크 1곳 외 원본 바이트 동일하며 생성 bindings의 해당 설명 1줄만 달라 manifest SHA-256을 `17a94672163a91185d30df392d94313025188817fccf728c1f2e90e12317c9d8`로 동기화했습니다.
- [x] E. 관련 8파일을 commit `e199ad4`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 열여덟 번째 slice — IDE 통신 DTO (완료)

- [x] A. `ide/types.rs`의 정책·timeout 상수는 기존 domain 파일에 두고 serde·specta·`ProjectId`만 의존하는 DTO 5종을 model로 옮기는 경계를 확인했습니다.
- [x] B. IDE status·diagnostic·selection legacy wire와 model↔facade 타입 동일성 테스트를 먼저 추가했고 model 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. DTO 5종을 model crate로 옮기고 기존 `domain::ide::types::*` 경로를 재수출했습니다. IDE 실행 상수는 기존 파일에 유지했습니다.
- [x] D. 전용 경계 2건·`cargo test --workspace --quiet` 총 1,827건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. DTO 본문은 rustdoc 링크 2곳 외 원본 바이트 동일하며 생성 bindings도 해당 설명 2줄만 달라 manifest SHA-256을 `0085288be0f5948e5570273ccf795f5cbee8318ba130568ad79e21b20bdbed17`로 동기화했습니다.
- [x] E. 관련 8파일을 commit `51c3f8c`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 열아홉 번째 slice — agent 상태·설치 DTO (완료)

- [x] A. `agent/types.rs`의 scanner 재수출과 정책 상수는 도메인에 유지하고, serde·specta·`ProjectId`만 의존하는 하단 DTO 8종을 model로 옮기는 경계를 확인했습니다.
- [x] B. 상태·hook·외부 열기 legacy wire와 model↔facade 타입 동일성 테스트를 먼저 추가했고 model 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. DTO 8종을 model crate로 옮기고 기존 `domain::agent::types::*` 경로를 재수출했습니다. scanner 재수출·정책 상수는 도메인에 유지했습니다.
- [x] D. 전용 경계 2건·`cargo test --workspace --quiet` 총 1,829건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. DTO 본문은 rustdoc 링크 2곳 외 원본 바이트 동일하며 생성 bindings도 해당 설명 2줄만 달라 manifest SHA-256을 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`로 동기화했습니다.
- [x] E. 관련 8파일을 commit `cc5e105`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 스무 번째 slice — terminal wire·scrollback 타입 (완료)

- [x] A. `terminal/types.rs`는 `ProjectId`·serde·specta와 표준 계산만 사용합니다. 기존 스크롤백 unit 5건과 타입 구현을 함께 model로 옮기고 기존 공개 경로를 유지합니다.
- [x] B. spawn/attach/session legacy wire와 model↔facade 타입 동일성 테스트를 먼저 추가했고 model 모듈 부재 E0432/E0433(exit 101) red를 확인했습니다.
- [x] C. 타입·스크롤백 계산·기존 unit 5건을 model crate로 옮기고 기존 `domain::terminal::types::*` 경로를 재수출했습니다.
- [x] D. 전용 경계 2건·기존 model unit 5건·`cargo test --workspace --quiet` 총 1,831건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. 타입 본문은 rustdoc 링크 1곳 외 원본 바이트 동일하며 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다.
- [x] E. 관련 6파일을 commit `ab56d30`으로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 스물한 번째 slice — remote wire DTO (완료)

- [x] A. `remote/types.rs`의 인증·호스트·dispatch 상수는 도메인에 두고, serde·specta·JSON Value만 의존하는 DTO 3종을 model로 옮기는 경계를 확인했습니다.
- [x] B. status·link·request legacy wire와 model↔facade 타입 동일성 테스트를 먼저 추가했고 model 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. DTO 3종을 model crate로 옮기고 기존 `domain::remote::types::*` 경로를 재수출했습니다. 인증·호스트·dispatch 상수는 도메인에 유지했습니다.
- [x] D. 전용 경계 2건·`cargo test --workspace --quiet` 총 1,833건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. DTO 본문은 원본 바이트 동일하며 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다.
- [x] E. 관련 6파일을 commit `4da998a`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 스물두 번째 slice — LSP manifest·session DTO (진행 중)

- [x] A. `lsp/types.rs`의 bundled manifest `include_str!`와 restart 상수는 도메인에 유지하고, 표준 `BTreeMap`·serde·specta·`ProjectId`만 의존하는 나머지 타입과 기본값 함수를 model로 옮기는 경계를 확인했습니다.
- [x] B. LSP ID·manifest 기본값·spawn/session wire와 model↔facade 타입 동일성 테스트를 먼저 추가했고 model 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 순수 타입을 model crate로 옮기고 기존 `domain::lsp::types::*` 경로를 재수출했습니다. 전용 경계 테스트 2건이 통과했습니다.
- [x] D. 전용 경계 2건·`cargo test --workspace --quiet`·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. DTO 본문은 원본 바이트 동일하며 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다.
- [x] E. 관련 4파일을 commit `12f3e0a`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 스물세 번째 slice — 서비스 공개 IPC 결과 DTO (진행 중)

- [x] A. 네 공개 결과 DTO는 기존 model의 `Project`·`TabId`와 serde·specta만 의존하고 command는 `service::*` 경로를 소비함을 확인했습니다.
- [x] B. model↔service 타입 동일성 및 기존 camelCase 응답 직렬화 테스트를 먼저 추가해 model 타입 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 네 결과 DTO를 model crate로 옮기고 서비스 공개 경로를 재수출했습니다. 전용 경계 2건이 통과했습니다.
- [x] D. 전용 경계 2건·Rust workspace 1,837건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. 네 DTO 본문은 원본 바이트 동일하고 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`도 불변입니다.
- [x] E. 구현·테스트 7파일을 commit `5f97a1f`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M2 스물네 번째 slice — IDE lockfile·agent hook 입력 DTO (진행 중)

- [x] A. `IdeLockfileContent`는 외부 lockfile 영속 스키마, `HookPayload`는 agent hook 입력 wire이며 둘 다 serde·표준 타입만 의존합니다. 남은 private provider·GitHub·VSIX·package 응답은 M4/M5 서비스·프로토콜 소유, mirror 파일은 M3 persist 소유, Tauri `Event` 파생 payload는 M6 adapter 소유로 분류했습니다.
- [x] B. 기존 lockfile·hook wire 및 model↔기존 경로 타입 동일성 테스트를 먼저 추가해 model DTO 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 두 DTO를 model crate로 옮기고 기존 공개 경로를 재수출했습니다. 전용 경계 테스트 2건이 통과했습니다.
- [x] D. 전용 경계 2건·Rust workspace 1,839건·fmt·clippy·strict model rustdoc·IPC 계약이 통과했습니다. 두 DTO 본문은 원본 바이트 동일하고 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`도 불변입니다.
- [x] E. 구현·테스트 5파일을 commit `7974e31`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다. 남은 private 직렬화 구조체는 해당 adapter·서비스와 함께 M3~M6에서 이전하며 M2 공통 model에 잘못 합치지 않습니다.

## M3 첫 slice — infra→domain 역참조 제거 (완료)

- [x] A. `asset_protocol`·`root_guard`는 `Project`, `self_write`·`watcher`는 `FsChange`/`FsChangeKind`를 이미 분리된 model의 도메인 facade로 import합니다. 경계 테스트의 4항목 허용 목록을 비우면 역참조가 명시적으로 검출됩니다. Tauri HTTP가 필요한 `asset_protocol`은 이후 platform adapter에 남깁니다.
- [x] B. infra→domain 허용 목록을 없애고 경계 검사를 강화해 기존 네 참조가 모두 검출되는 red(exit 101)를 확인했습니다.
- [x] C. infra 4파일의 `Project`·`FsChange` 타입 import와 `self_write` unit import를 model 직접 경로로 돌렸고 경계 테스트 3건이 통과했습니다. 타입 정의·함수 시그니처·동작은 변경하지 않았습니다.
- [x] D. 경계 3건·`cargo test --workspace --quiet` 1,839건·fmt·clippy·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다.
- [x] E. 코드·경계 테스트 5파일을 commit `819f037`로 반영하고 기록 commit과 함께 현재 브랜치에 일반 push합니다.

## M3 두 번째 slice — 독립 infra crate 첫 모듈 (완료)

- [x] A. 6모듈은 Tauri·domain·infra sibling을 import하지 않고 표준 라이브러리와 `redact`의 기존 `regex`만 사용합니다. 기존 unit을 함께 옮길 수 있고 domain/infra 소비는 `crate::infra::*` facade를 통해 유지합니다. `asset_protocol`·`navigation_guard`는 Tauri adapter에 남깁니다.
- [x] B. 새 crate와 기존 facade의 타입·동작 경계 테스트를 먼저 추가했고 `taide_infra` 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 6모듈과 기존 unit 40건을 `taide-infra`로 이전하고 기존 `infra::*` 경로를 재수출했습니다. 전용 경계 2건이 통과했습니다. 엄격 rustdoc의 기존 private 링크·HTML 표기 3곳만 경로 텍스트로 고쳤습니다.
- [x] D. 전용 경계 2건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. 새 crate의 6모듈 원천은 기존 파일과 rustdoc 표기 3곳만 다릅니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 21파일을 commit `960a71e`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다. 현재 M3의 자원 이전은 계속 진행 중입니다.

## M3 세 번째 slice — self-write tracker 이전 (완료)

- [x] A. `self_write.rs`는 `taide_model::file::FsChange`와 기존 `parking_lot`만 외부에서 소비하고, `state.rs`·`file/capability.rs`가 기존 `infra::self_write` 경로를 사용합니다. 기존 unit 9건을 함께 옮기고 facade로 경로를 유지합니다.
- [x] B. crate 직접 경로와 기존 facade의 타입·배치 소비 경계를 추가했고 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. tracker·배치 판정·unit 9건을 infra crate로 옮기고 기존 facade를 재수출했습니다. 전용 경계 3건과 infra unit 49건이 통과했습니다. 원본 구현은 private rustdoc 링크 표기 1곳만 다릅니다.
- [x] D. 전용 경계 3건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 8파일을 commit `63858e0`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 네 번째 slice — root/symlink guard 이전 (완료)

- [x] A. `root_guard.rs`는 이미 분리한 model의 error·ID·Project와 표준 파일시스템만 사용합니다. 기존 unit 10건과 `user-bug-regressions` symlink 회귀가 있으며, `state`·`plugin`이 쓰는 `canonicalize_lenient`의 crate-private facade 가시성은 유지해야 합니다.
- [x] B. crate 직접 경로와 기존 facade의 타입·안전 컴포넌트 경계를 추가했고 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 10건을 infra crate로 옮기고 공개 API·crate-private facade 경로를 보존했습니다. 전용 경계 4건과 infra unit 59건이 통과했습니다. 원본 구현은 model import와 crate 간 가시성만 바뀌었습니다.
- [x] D. 기존 root/symlink 회귀를 포함한 `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 8파일을 commit `d9eaba6`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 다섯 번째 slice — 원자적 persist 이전 (완료)

- [x] A. `persist.rs`는 model `AppError`와 기존 serde·serde_json·uuid 및 표준 파일시스템만 사용합니다. 기존 unit 11건은 임시 파일 정리·파일 모드 보존·owner-only 쓰기·JSON 왕복을 검증합니다. `file/service.rs`·`watcher.rs`가 사용하는 `temp_sibling`의 crate-private facade 경로를 유지합니다.
- [x] B. crate 직접 경로와 기존 facade의 쓰기 타입·임시 파일 판별 경계를 추가했고 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 11건을 infra crate로 옮기고 공개 API·crate-private facade 경로를 보존했습니다. 전용 경계 5건과 infra unit 70건이 통과했습니다. 원본 구현은 model error import와 crate 간 가시성만 바뀌었습니다.
- [x] D. 원자적 쓰기·권한·임시 파일 회귀를 포함한 `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 8파일을 commit `ee0073a`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 여섯 번째 slice — watcher 자원 이전 (완료)

- [x] A. `watcher.rs`는 model `FsChange`와 기존 notify·notify-debouncer-full·log, 이전한 infra `persist`, `constants.rs`의 무시 디렉터리 정책을 사용합니다. 기존 watcher unit 24건과 constants unit 1건, `WatchScope`별 필터·debounce·cache 검사를 함께 옮기고 `infra::watcher` 및 `constants::*` 공개 경로를 유지합니다.
- [x] B. crate 직접 경로와 기존 facade의 watcher 타입·빈 루트 거부·무시 디렉터리 정책을 추가했고 두 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 정책·watcher 구현·기존 unit 25건을 infra crate로 옮기고 두 기존 공개 경로를 재수출했습니다. 새 경계 6건이 통과했고 실제 핸들 종료 테스트 1건을 추가·단독 실행해 통과했습니다. private rustdoc 링크 2곳만 일반 경로 표기로 고쳤습니다.
- [x] D. 실제 핸들 종료와 기존 배치·필터 회귀를 포함한 infra unit 96건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 10파일을 commit `692a24b`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 일곱 번째 slice — range·URL 보안 유틸 이전 (완료)

- [x] A. `range_file.rs`는 표준 라이브러리만 쓰고 원격 파일 route·asset adapter가 동일한 범위·MIME·CSP 정책을 소비합니다. `external_url.rs`는 model `AppError`만 필요하고 system command·navigation adapter가 소비합니다. 기존 unit은 각각 10건·9건입니다.
- [x] B. crate 직접 경로와 기존 facade의 범위 파싱·URL 거부 경계를 추가했고 두 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 두 구현·unit 19건을 infra crate로 옮기고 기존 공개 경로를 재수출했습니다. 전용 경계 8건과 infra unit 115건이 통과했습니다. 원본 구현은 model error import와 rustdoc 경로 표기만 바뀌었습니다.
- [x] D. 범위 상한·CSP·URL 위장 회귀를 포함한 infra unit 115건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 8파일을 commit `6a15a79`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 여덟 번째 slice — archive 보안 추출 이전 (완료)

- [x] A. `archive.rs`는 model `AppError`와 기존 `zip` 외에 Tauri·domain 의존이 없습니다. plugin 설치와 VSIX 서비스가 보안 예산을 소비하며, 기존 unit 10건은 경로 탈출·크기·모드 제한을 검증합니다.
- [x] B. crate 직접 경로와 기존 facade의 압축 해제 타입·상한 경계를 추가했고 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 10건을 infra crate로 옮기고 기존 공개 경로를 재수출했습니다. 전용 경계 9건과 infra unit 125건이 통과했으며 원본 구현은 model error import만 바뀌었습니다.
- [x] D. zip-slip·용량·권한 회귀를 포함한 infra unit 125건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 8파일을 commit `a5e16e2`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 아홉 번째 slice — LSP 설치 자원 이전 (완료)

- [x] A. `lsp_install.rs`는 model error·기존 HTTP/async/압축 의존만 사용합니다. LSP 설치·서비스와 plugin 서비스가 소비하며 unit 16건을 보유합니다. 원격 LSP archive는 `commands.rs`의 SHA-256 검증 성공 뒤 해제하고, 사용자 입력 plugin zip은 별도 hardened archive 경계를 유지합니다.
- [x] B. crate 직접 경로와 기존 facade의 타입·해시·설치 경계를 테스트했고 `taide_infra::lsp_install` 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 16건을 infra crate로 옮기고 기존 공개 경로를 재수출했습니다. 새 경계 10건과 infra unit 141건이 통과했으며 model error import만 바뀌었습니다.
- [x] D. 경계 10건·infra unit 141건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 6파일을 commit `4b19f58`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 열 번째 slice — HTTP 클라이언트 자원 이전 (완료)

- [x] A. `http.rs`는 기존 reqwest·표준 OnceLock만 사용하고 AI/sync API 요청과 LSP 다운로드가 소비합니다. 프로필별 단일 연결 풀과 기존 unit 2건을 확인했습니다.
- [x] B. crate 직접 경로와 기존 facade의 타입·반환 경계 테스트에서 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 2건을 infra crate로 옮기고 단일 프로세스 클라이언트의 소유권을 유지했습니다. 기존 공개 경로를 재수출했고 경계 11건·infra unit 143건이 통과했습니다. strict rustdoc에서 private 함수 링크 표기만 수정했습니다.
- [x] D. 경계 11건·infra unit 143건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 4파일을 commit `09ccdfd`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 열한 번째 slice — 성능 계측 레지스트리 이전 (완료)

- [x] A. `perf.rs`는 표준 라이브러리만 사용하고 앱 초기화·명령 집계와 Git/search/terminal 계측이 소비합니다. process-global 레지스트리, 슬롯·카운터 wire 이름과 기존 unit 18건을 확인했습니다.
- [x] B. crate 직접 경로와 기존 facade의 타입·전역 인스턴스 동일성 테스트에서 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 18건을 infra crate로 옮기고 공개 경로·단일 전역 레지스트리를 유지했습니다. 새 경계 12건과 infra unit 161건이 통과했으며 이전 crate를 가리키던 rustdoc 링크 표기만 수정했습니다.
- [x] D. 경계 12건·infra unit 161건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 4파일을 commit `33a9d2d`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 열두 번째 slice — 셸 통합·터미널 스캐너 이전 (완료)

- [x] A. shell_integration은 model·기존 persist/quote/log/uuid, terminal_scan은 marker 타입만 의존합니다. 기존 unit 18·42건과 OSC payload 4096B·title 512B·agent 1024B 상한을 확인했습니다. secret은 keyring·cfg(test) test_support가 도메인 unit에 교차 crate로 쓰여 별도 설계가 필요하므로 이 slice 밖에 둡니다.
- [x] B. crate 직접 경로와 기존 facade의 marker·scanner 타입·OSC 경계 테스트에서 두 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 결합된 구현·unit 60건을 infra crate로 옮기고 공개 경로를 유지했습니다. 새 경계 13건과 infra unit 221건이 통과했으며 이전 crate·private 항목을 가리키던 rustdoc 표기만 수정했습니다.
- [x] D. 경계 13건·infra unit 221건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 6파일을 commit `0fff8cd`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 열세 번째 slice — PTY 세션 자원 이전 (완료)

- [x] A. PTY는 model error·이전된 shell_integration·기존 parking_lot/portable-pty만 의존하고 terminal 명령이 소비합니다. unit 16건에 paused drop의 자식 종료·셸 임시 디렉터리 정리 테스트가 포함됨을 확인했습니다.
- [x] B. crate 직접 경로와 기존 facade의 config·session 타입 테스트에서 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 16건을 infra crate로 옮기고 공개 경로를 유지했습니다. 새 경계 14건과 paused 자식 종료·임시 디렉터리 정리를 포함한 infra unit 237건이 통과했습니다. 이전 crate import·private rustdoc 링크 표기만 바뀌었습니다.
- [x] D. paused 자식 종료·임시 디렉터리 정리를 포함한 infra unit 237건·경계 14건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 6파일을 commit `9a216f6`으로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 열네 번째 slice — LSP 프로세스 자원 이전 (완료)

- [x] A. LSP 프로세스는 model error·기존 parking_lot/tokio/sysinfo만 의존하고 LSP 명령이 소비합니다. unit 18건에 프레이밍·실프로세스 종료·PID 재사용 보호·stderr tail 상한이 포함됨을 확인했습니다.
- [x] B. crate 직접 경로와 기존 facade의 config·handle·프레이밍 경계 테스트에서 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 18건을 infra crate로 옮기고 공개 경로를 유지했습니다. 새 경계 15건과 실프로세스 회귀를 포함한 infra unit 255건이 통과했으며 model import·private rustdoc 링크 표기만 바뀌었습니다.
- [x] D. 실프로세스 종료·PID 재사용·stderr 상한을 포함한 infra unit 255건·경계 15건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 6파일을 commit `1d1912b`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 열다섯 번째 slice — 키체인 자원 이전 (완료)

- [x] A. secret은 model error·기존 keyring/parking_lot만 의존하고 AI/sync/remote가 소비합니다. 기존 unit 4건과 AI/sync 도메인 unit이 cfg(test) InMemorySecretStore를 교차 crate로 쓰는 계약을 확인했습니다. [Cargo resolver 2 기능 계약](https://doc.rust-lang.org/cargo/reference/features.html#feature-resolver-version-2)을 따릅니다.
- [x] B. crate 직접 경로와 기존 facade의 account·store 타입 및 in-memory helper 경계 테스트에서 모듈 부재 E0432(exit 101) red를 확인했습니다.
- [x] C. 구현·unit 4건을 infra crate로 옮기고 공개 경로를 유지했습니다. test-support 기능은 taide dev-dependency에서만 활성화해 AI/sync unit의 메모리 저장소 경로를 보존했습니다. 새 경계 16건·infra unit 259건이 통과했고 `cargo tree -p taide -e normal,features -i taide-infra`에 test-support가 없습니다.
- [x] D. 실제 키체인 값을 건드리지 않는 경계 16건·infra unit 259건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict infra rustdoc·IPC 계약이 통과했습니다. 일반 빌드 의존성 그래프에는 test-support가 없고 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 실행하지 않았습니다.
- [x] E. 관련 7파일을 commit `7dd075c`로 반영하고 원격 `to_rust_native`에 일반 push했습니다.

## M3 종료 판정 — Git 소유권과 adapter 잔여 (완료)

- [x] A. `docs/architecture.md`는 별도 `infra/repo.rs`가 없고 git2가 `domain/git/service.rs`에 있다고 명시합니다. 실제 3,641줄 구현도 Git DTO·정책, libgit2 호출, timeout subprocess를 함께 소유하며 이 계약의 소유권 지도는 file/tree/git 서비스 crate를 M4로 배치합니다. 얇은 repo 래퍼를 임의로 만들지 않고 Git 구현·테스트를 M4에서 서비스 단위로 이전합니다.
- [x] B. `src-tauri/src/infra`의 나머지는 21개 facade와 Tauri `asset_protocol`·`navigation_guard` adapter 두 파일입니다. `taide-infra` 소스와 정상 의존 그래프에 Tauri/domain 역의존은 없고 마지막 코드 변경에서 경계 16건·infra unit 259건·workspace 전체·fmt·clippy·strict infra rustdoc·IPC/bindings 계약이 통과했습니다. GUI 실기는 실행하지 않았습니다.
- [x] C. M3를 완료 처리하고 Git 서비스는 M4, 두 Tauri adapter는 M6으로 소유권을 기록해 문서를 선별 commit·일반 push합니다.

## M4 첫 번째 slice — font 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. font 서비스는 fontdb와 model `FontFamily`만 의존합니다. fontdb의 다른 src-tauri 소비자는 없고 Tauri `font_list` command는 서비스의 공개 `list_families`만 호출합니다. 기존 unit 3건은 빈 DB, 프로세스 수명 캐시 1회 스캔, 가족명 정렬·중복 제거를 검증합니다.
- [x] B. 새 `taide_font::service`와 기존 `taide_lib::domain::font::service`가 같은 함수 진입점·DTO를 공유하는 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 구현·unit 3건을 새 crate로 옮기고 기존 경로는 재수출 facade로 유지했습니다.
- [x] C. 새 crate unit 3건·경계 1건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-font --no-deps`가 통과했습니다. 제한된 sandbox의 기존 프로세스·소켓·휴지통 테스트 9건은 동일 명령을 권한 허용 환경에서 재실행해 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. GUI 실기는 사용자 몫이며 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `5fc420a`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 두 번째 slice — notification 정책 이전 (완료, M4 전체는 진행 중)

- [x] A. notification 서비스는 model `NotificationCategory`·`NotificationDelivery`·`NotificationSuppressionReason`·`Settings`만 참조합니다. Tauri command는 focus와 OS 알림 표시를 소유하며 정책 함수 `decide_delivery`만 호출합니다. 기존 unit 7건은 master/category/focus와 억제 사유 우선순위를 검증합니다.
- [x] B. 새 crate와 기존 facade의 함수 진입점·master 억제 사유를 비교하는 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 정책·unit 7건을 새 crate로 옮기고 기존 서비스 경로는 재수출 facade로 유지했습니다.
- [x] C. 새 crate unit 7건·경계 1건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-notification --no-deps`가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. OS 알림 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `c0dfca7`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 세 번째 slice — snippet 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. snippet 서비스는 model의 DTO/error/paths, infra persist, log·serde_json을 사용합니다. 기존 unit 12건은 저장·목록·삭제, JSON 호환, 잘못된 파일의 허용적 스캔, 경로 구분자·Windows drive-relative·확장자 거부를 검증합니다.
- [x] B. 새 crate와 기존 facade의 세 공개 진입점 및 위험 파일명 거부를 비교하는 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 구현·unit 12건을 새 crate로 옮기고 기존 서비스 경로는 재수출 facade로 유지했습니다.
- [x] C. 새 crate unit 12건·경계 1건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-snippet --no-deps`가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. snippet UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `cda8f6d`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 네 번째 slice — system 사용량 정책 이전 (완료, M4 전체는 진행 중)

- [x] A. system 서비스는 model 사용량 DTO만 참조합니다. Tauri command는 sysinfo 샘플링·프로세스 상태를 소유하고 서비스는 CPU 정규화·PID 자손·프로세스 종류·라벨·정렬 정책을 소유합니다. 기존 unit 13건을 확인했습니다.
- [x] B. 새 crate와 기존 facade의 ProcessRecord 타입·사용량 결과 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 정책·unit 13건을 새 crate로 옮기고 기존 서비스 경로는 재수출 facade로 유지했습니다.
- [x] C. 새 crate unit 13건·경계 1건·`cargo test --workspace --quiet` 전체가 통과했습니다. 경계 테스트의 과도한 함수 포인터 타입 표기로 처음 clippy가 실패한 뒤 중복 표기를 제거하고 전용 테스트·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`를 재확인했습니다. `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-system --no-deps`도 통과했고 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. 사용량 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `ac6aa13`으로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 다섯 번째 slice — task 탐지 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. task 서비스는 model `Task`·`TaskSource`, infra shell_quote, regex·serde_json만 참조합니다. 기존 unit 16건은 package manager 우선순위, Make target 판별, Cargo 고정 명령, 셸 인용, 다중 소스 병합을 검증합니다.
- [x] B. 새 crate와 기존 facade의 탐지 함수·Bun lockfile·공백 스크립트 명령 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 구현·unit 16건을 새 crate로 옮기고 기존 서비스 경로는 재수출 facade로 유지했습니다.
- [x] C. 새 crate unit 16건·경계 1건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-task --no-deps`가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. task 실행 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `df75bf2`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 여섯 번째 slice — tree 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. tree 서비스는 model error/DTO와 표준 라이브러리만 참조합니다. Tauri command는 프로젝트별 store·락·비동기 prefetch를 소유하고 서비스는 디렉터리 스캔·symlink·확장/접기·페이지 정책을 소유합니다. 기존 unit 26건을 확인했습니다.
- [x] B. 새 crate와 기존 facade의 `TreeState` 타입·상태 생성·페이지 함수 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 구현·unit 26건을 새 crate로 옮기고 기존 서비스 경로는 재수출 facade로 유지했습니다.
- [x] C. 새 crate unit 26건·경계 1건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-tree --no-deps`가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. 트리 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `a91beb8`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 일곱 번째 slice — locale 서비스·번들 카탈로그 이전 (완료, M4 전체는 진행 중)

- [x] A. locale 서비스는 model DTO/error/paths, infra persist, serde_json만 참조하며 번들 en/ko/ja JSON 3개를 include_str로 사용합니다. 기존 unit 18건은 카탈로그 키·번역 변수·커스텀 언어 저장/상속·fallback을 검증합니다. 다른 Rust 원천의 카탈로그 파일 직접 참조는 없습니다.
- [x] B. 새 crate와 기존 facade의 내장 카탈로그·필수 키 경계 테스트에서 crate 부재 E0433(exit 101) red를 확인했습니다. 구현·unit 18건·리소스 3개를 새 crate로 옮기고 기존 서비스 경로는 재수출 facade로 유지했습니다. 리소스 각각의 SHA-256은 원본과 동일합니다.
- [x] C. 새 crate unit 18건·경계 1건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`가 통과했습니다. strict rustdoc은 공개 함수가 private 파서를 링크한 표기 한 곳에서 실패했고 이를 코드 텍스트로 바꾼 뒤 `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-locale --no-deps`가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. locale UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `50a19fc`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 여덟 번째 slice — theme 서비스·번들 카탈로그 이전 (완료, M4 전체는 진행 중)

- [x] A. theme 서비스는 model DTO/error/paths와 infra persist/root_guard, serde_json만 참조합니다. 번들 JSON 47개·기존 unit 49건·프론트 테마 품질 게이트 3파일 20건, 경로를 참조하는 정비 스크립트 3개·라이선스 문서를 확인했습니다. 기존 프론트 게이트 20건 green 후 새 crate 경계 테스트는 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 구현·unit 49건·번들 JSON 47개를 taide-theme로 옮기고 기존 서비스 경로를 재수출 facade로 유지했습니다. Rust 원천의 include_str 51곳, 프론트 게이트·스크립트·라이선스와 관련 코드의 경로 표기를 새 소유 위치로 갱신했습니다. 새 crate unit 49건·경계 1건·프론트 게이트 20건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-theme --no-deps`·`bun run typecheck`·변경 파일 Prettier가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. 파일을 수정하는 테마 정비 스크립트와 테마 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `4f6e774`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 아홉 번째 slice — settings 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. settings 서비스는 model DTO/error/paths, infra persist, theme 서비스, remote wildcard 문법 상수에 의존합니다. 기존 unit 69건은 저장·마이그레이션·입력 보정을 검증합니다. 새 경계 테스트에서 settings crate 부재 E0433과 model wildcard 상수 부재 E0425(exit 101)를 확인했습니다.
- [x] B. wildcard 상수를 taide-model로 옮기고 Tauri의 기존 remote 경로는 재수출했습니다. settings 구현·unit 69건을 taide-settings로 이전하고 기존 서비스 경로도 재수출 facade로 유지했습니다. 새 crate unit 69건·경계 1건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·settings/model strict rustdoc가 통과했습니다. 첫 workspace 검사는 이전 경계 화이트리스트의 미사용 항목 1건에서 실패해 항목 제거 후 전부 재검증했습니다. strict settings rustdoc의 private 링크 표기 1건도 코드 텍스트로 바로잡았습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. settings UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `db16b6d`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열 번째 slice — sync 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. sync 서비스는 model DTO/error/paths, 이전된 locale/settings/theme 서비스, serde_json·log에 의존합니다. 기존 unit 31건은 UTC·버전 게이트, 비동기화 설정 필드, 레거시 payload, 테마/로케일 적용을 검증합니다. 새 crate 경계 테스트에서 crate 부재 E0433(exit 101)를 확인했습니다.
- [x] B. 구현·unit 31건을 taide-sync로 이전하고 기존 Tauri 공개 서비스 경로를 재수출 facade로 유지했습니다. 더 이상 존재하지 않는 sync 서비스의 도메인 간 엣지 3개를 화이트리스트에서 제거했습니다. 새 crate unit 31건·경계 1건·도메인 경계 3건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-sync --no-deps`가 통과했습니다. 생성 bindings SHA-256 `99ab778ed7f7b8a92aebec3afc94492c5b8f26e8ed4c97d63122f23194284763`은 불변입니다. 실제 동기화 업로드·다운로드 GUI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `89ff027`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열한 번째 slice — search 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. search 서비스는 model Search DTO/error, infra persist/root_guard/watch_policy, ignore/regex와 공유 파일 크기 상수에 의존합니다. 기존 unit 57건은 검색·교체·취소·ignore·큰 파일 정책을 검증합니다. 새 경계 테스트에서 search crate 부재 E0433과 model 파일 크기 상수 부재 E0425(exit 101)를 확인했습니다.
- [x] B. 공유 파일 크기 상수 4개를 model file 소유로 옮기고 기존 Tauri constants 경로는 재수출했습니다. search 구현·unit 57건을 taide-search로 이전하고 기존 서비스 경로를 재수출 facade로 유지했습니다. 새 crate unit 57건·경계 1건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·search/model strict rustdoc가 통과했습니다. strict search rustdoc의 private 링크 3곳은 코드 텍스트로 바로잡고 재검증했습니다. model search 설명 문구 1줄이 다음 slice의 bindings 재생성에서 반영되어 commit `d6511a8`로 생성 bindings와 manifest 해시를 동기화했습니다. 검색 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `bf3d7fd`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열두 번째 slice — plugin 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. plugin 서비스는 model plugin DTO/error, infra archive/language/lsp_install/root_guard, parking_lot·serde_json·uuid와 VSIX에서도 사용하는 manifest 상수에 의존합니다. 기존 unit 26건은 경로·manifest·grammar·install/staging/rollback을 검증합니다. 새 경계 테스트에서 plugin crate 부재 E0433과 model 상수 부재 E0425(exit 101)를 확인했습니다.
- [x] B. manifest 상수 3개를 model plugin 소유로 옮기고 기존 types 경로는 재수출했습니다. plugin 구현·unit 26건을 taide-plugin으로 이전하고 기존 서비스 경로를 재수출 facade로 유지했습니다. 새 crate unit 26건·경계 1건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·plugin/model strict rustdoc가 통과했습니다. 직전 search 문서 1줄을 bindings로 재생성한 뒤 manifest 해시를 commit `d6511a8`로 맞췄고 Phase 0 계약 7건·권한 허용 `cargo test -p taide --lib --quiet` 1,122건·`bun run typecheck`가 통과했습니다. 제한된 sandbox에서 같은 lib 명령의 프로세스·루프백·휴지통 권한 관련 기존 9건 실패는 권한 허용 재실행에서 모두 통과했습니다. 생성 bindings SHA-256은 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`이며 타입·명령 계약 변화는 없습니다. plugin 설치 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `d4474f9`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열세 번째 slice — VSIX 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. VSIX 서비스는 model VSIX/plugin DTO/error, infra archive/persist/root_guard, regex·serde·zip·log·uuid와 VSIX 경로·크기 상수 6개에 의존합니다. 기존 unit 32건은 경로 탈출·압축 크기 제한·include 순환·manifest/grammar 적용을 검증합니다. 새 경계 테스트에서 VSIX crate 부재 E0433과 model 상수 부재 E0425(exit 101)를 확인했습니다.
- [x] B. VSIX 상수 6개를 model vsix 소유로 옮기고 기존 types 경로는 재수출했습니다. 구현·unit 32건을 taide-vsix로 이전하고 기존 서비스 경로를 재수출 facade로 유지했습니다. 새 crate unit 32건·경계 1건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·VSIX/model strict rustdoc가 통과했습니다. 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. VSIX import UI와 실제 외부 확장 파일 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `c0ce737`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열네 번째 slice — file 서비스·editorconfig 이전 (완료, M4 전체는 진행 중)

- [x] A. file 서비스는 model DTO/error/ids/paths/파일 크기 상수, infra clock/language/persist, 외부 trash에 의존하고 editorconfig는 model DTO만 참조합니다. `save_file_within_open_projects` 한 함수는 AppState의 열린 프로젝트·self-write·mirror를 조립하므로 Tauri adapter로 남겨야 합니다. 기존 file unit 44건(guarded save 1건 포함)·editorconfig unit 17건과 새 crate 부재 E0433 경계 red(exit 101)를 확인했습니다.
- [x] B. 순수 file 구현·unit 43건과 editorconfig 구현·unit 17건을 taide-file로 옮기고 기존 서비스·editorconfig 경로를 재수출했습니다. guarded save는 Tauri에 남겨 루트 가드 → 원자 저장(모드 보존) → self-write 표시 → hot-exit mirror 정리 순서를 유지하고 회귀 unit을 경계 테스트로 옮겼습니다. 새 crate unit 60건·경계/adapter 2건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·strict file rustdoc가 통과했습니다. strict rustdoc의 private 링크 3곳은 코드 텍스트로 바로잡고 재검증했습니다. 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 파일 저장·editorconfig UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `89cbb29`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열다섯 번째 slice — AI 서비스·프롬프트·provider 이전 (완료, M4 전체는 진행 중)

- [x] A. AI service/prompt/provider 5개 모듈은 model DTO/error/paths·infra persist/redact/secret·reqwest/futures-util/serde/uuid와 번들 프롬프트 JSON 3개에 의존합니다. 기존 unit 86건을 확인했고 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 구현·unit 86건·번들 JSON 3개를 taide-ai로 이전하고 Tauri의 service/prompt/providers 공개 경로를 재수출 facade로 유지했습니다. 테스트에서만 쓰던 Tauri async runtime은 Tokio 테스트 전용 런타임으로 교체하고 in-memory secret 저장소 기능은 dev-dependency에만 뒀습니다. 새 crate unit 86건·경계 1건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-ai --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 프롬프트 JSON 3개의 SHA-256도 원본과 동일합니다. 실제 provider 네트워크·키체인·AI UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 선별 commit하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열여섯 번째 slice — app 파일·성능 서비스 이전 (완료, M4 전체는 진행 중)

- [x] A. app 서비스의 파일 경로·프롬프트 fallback·저장·성능 스냅샷은 model DTO/paths/settings, 이전된 AI prompt, infra persist/perf에 의존합니다. `app_info`만 컴파일 시점 `CARGO_PKG_VERSION`을 사용하므로 Tauri 패키지 버전을 보존하도록 기존 어댑터에 남깁니다. 기존 unit 9건 green 후 새 crate 경계 테스트의 crate 부재 E0433(exit 101)를 확인했습니다.
- [x] B. 나머지 구현·unit 9건을 taide-app으로 옮기고 기존 app service 경로에서 재수출했습니다. `app_info`와 `APP_NAME`은 Tauri 패키지에 남겨 버전·이름·플랫폼 응답을 보존했습니다. 이동으로 사라진 app→AI 도메인 경계 화이트리스트를 제거하고 새 crate unit 9건·경계 1건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-app --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. app 파일 편집·성능 표시 UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `a0395d7`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열일곱 번째 slice — project 그룹·셸 슬롯 순수 정책 이전 (완료, project 서비스는 진행 중)

- [x] A. `groups.rs`와 `shell_slots.rs`는 model의 ID/project/layout/error만 참조하고 Tauri·infra 자원에 의존하지 않습니다. 기존 unit 9건·15건 green 후 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 두 구현과 unit 24건을 taide-project로 옮기고 기존 project 공개 경로는 재수출 facade로 유지했습니다. 새 crate unit 24건·경계 1건이 통과했고 project service/commands의 저장·복원·OS 조립은 아직 Tauri 도메인에 남습니다.
- [x] C. `session_restore` 8건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-project --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 2곳은 코드 텍스트로 바로잡고 재검증했습니다. normal feature graph는 model만 참조하고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. project UI·실제 재시작 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `67d796d`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열여덟 번째 slice — project 서비스 이전 (완료, project command 조립은 유지)

- [x] A. project 서비스는 model project/layout/error/ids/paths, infra clock/home/persist, log·serde_json에 의존하고 Tauri 호출은 없습니다. 기존 unit 74건 green 후 새 서비스 경계 테스트는 `taide_project::service` 모듈 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 구현·unit 74건을 기존 taide-project crate로 옮기고 Tauri 공개 서비스 경로를 재수출 facade로 유지했습니다. 새 crate unit 총 98건·경계 1건·session restore 8건이 통과했습니다. Tauri commands·capability는 기존 조립 경계에 남습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-project --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 5곳은 코드 텍스트로 바로잡고 재검증했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. project UI·실제 재시작 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `0c93b13`으로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 열아홉 번째 slice — agent 정책 서비스·공유 상수 이전 (완료, hooks 조립은 유지)

- [x] A. agent 서비스는 model agent DTO/error/ids, infra terminal_scan/crypto, serde/serde_json과 기존 Tauri types의 공유 정책 상수에 의존합니다. 기존 unit 142건 green 후 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 정책 구현·unit 142건과 공유 상수를 taide-agent로 이전하고 기존 service/types 경로를 재수출 facade로 유지했습니다. 새 crate unit 142건·경계 1건이 통과했습니다. hooks 서버·Tauri commands는 기존 조립 경계에 남습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-agent --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 2곳은 코드 텍스트로 바로잡고 재검증했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 agent 프로세스·hook 설치·UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `0d8f9da`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 스무 번째 slice — Git 서비스 이전 (완료, Tauri 조립은 유지)

- [x] A. Git 서비스는 model Git DTO/error/file 크기 상수, infra language/redact, git2/trash에 의존합니다. 기존 unit 88건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 서비스 구현·unit 88건을 taide-git로 이전하고 기존 Tauri 서비스 경로는 재수출 facade로 유지했습니다. 새 crate unit 88건·경계 1건이 통과했고 Tauri commands·watch·plugin overlay 조립은 기존 경계에 남습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-git --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 표기 5곳은 코드 텍스트로 바로잡고 재검증했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 저장소·원격·Git UI 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `48a0290`으로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 스물한 번째 slice — layout 정책·저장 서비스 이전 (완료, Tauri 조립은 유지)

- [x] A. layout 서비스의 탭·패널 정책과 저장/복원은 model DTO/error/ids/paths, infra persist, log에 의존합니다. flush·이벤트 발신·IDE pending diff 해소·terminal 세션 회수는 AppState/AppHandle에 묶여 Tauri 조립 경계입니다. 기존 unit 108건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 정책·저장/복원 구현과 unit 104건을 taide-layout로 이전하고 기존 서비스 경로를 재수출 facade로 유지했습니다. flush/이벤트/IDE·terminal 조립과 unit 4건은 Tauri 경계에 남겼습니다. 새 crate unit 104건·경계 1건·adapter unit 4건·session restore 8건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-layout --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 표기 7곳은 코드 텍스트로 바로잡고 재검증했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 레이아웃 GUI·재시작 실기는 실행하지 않았고 M4 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `fb966e1`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M4 종료 판정 — 기능별 서비스 코드 분리 완료

- [x] 계약의 project/layout/file/tree/search/git, settings/theme/locale/snippet, plugin/vsix/sync, ai/agent/task/system/font/notification 서비스가 각각 Tauri 없는 기능 crate에 있으며 추가 app 서비스도 분리됐습니다. 기존 Tauri 공개 service 경로는 재수출 facade 또는 명시적 조립 adapter로 보존했습니다.
- [x] 기존 기능 unit·번들 fixture/resource는 소유 crate와 함께 이전했습니다. AppState/AppHandle이 필요한 guarded save, layout flush/이벤트/IDE·terminal 후처리, project·agent·Git commands/capability/hooks/watch/plugin overlay 취득은 Tauri 조립 경계에 남겼습니다. root_guard·persist는 taide-infra 소유이며 서비스는 검증된 경로·overlay를 입력으로 소비합니다.
- [x] 마지막 layout slice에서 `cargo test --workspace --quiet` 전체, fmt, clippy, strict layout rustdoc, Phase 0 IPC 계약 7건, TypeScript typecheck, normal feature graph 및 생성 bindings 해시 불변을 확인했습니다. 이는 M4의 코드 분리 완료 근거이며 실제 GUI·재시작·원격/OS 통합 parity를 통과했다는 뜻이 아닙니다. M5~M7에서 결합 절단·adapter·실기 검증을 계속합니다.

## M5 첫 번째 slice — terminal scrollback·shell·경로 정책 이전 (완료, PTY 조립은 유지)

- [x] A. terminal 서비스는 model ShellProfile/error, infra home만 의존하며 scrollback·shell profile·경로 정책을 소유합니다. 기존 unit 21건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다. PTY command/capability·구독 채널·세션 수명주기는 Tauri 조립 경계입니다.
- [x] B. 정책 구현과 unit 21건을 taide-terminal로 이전하고 기존 service 경로를 재수출 facade로 유지했습니다. 새 crate unit 21건·경계 1건·PTY command 테스트 22건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. 경계 테스트의 타입 복잡도 지적은 중복 타입 표기를 줄인 뒤 해당 테스트·fmt·clippy를 재검증했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 PTY 프로세스·셸 프로필 OS·GUI 실기는 실행하지 않았고 M5 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `1390e41`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 두 번째 slice — IDE 토큰·경로·열린 편집기 정책 이전 (완료, MCP 조립은 유지)

- [x] A. IDE 서비스는 model layout/project/ide/ids, infra language/root_guard/crypto, layout 서비스와 uuid에 의존합니다. 기존 unit 12건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다. MCP server/store/command와 세션 수명주기는 Tauri 조립 경계입니다.
- [x] B. 서비스 정책·unit 12건과 전용 포트 범위 상수를 taide-ide로 이전하고 새 crate가 taide-layout에 단방향 의존하게 했습니다. 기존 service/types 공개 경로는 재수출 facade로 유지했습니다. 새 crate unit 12건·경계 1건·IDE 전체 lib 테스트 39건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-ide --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. 첫 workspace 검사는 더 이상 존재하지 않는 ide/service→layout/service 화이트리스트 1건에서 실패해 항목·설명을 정리한 뒤 경계 3건과 workspace 전체를 재검증했습니다. normal feature graph는 ide→layout→model/infra 단방향이며 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 MCP server·세션·GUI 실기는 실행하지 않았고 M5 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `82d2b6f`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 세 번째 slice — window label·복원 계획 정책 이전 (완료, OS 창 조립은 유지)

- [x] A. window 서비스의 label·복원 계획은 model Project/Layout/ID만 의존하며 AppHandle·hot-exit mirror·창 닫기 후 탭 복귀는 Tauri 조립 경계입니다. 기존 unit 12건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다.
- [x] B. 계획 구현·unit 12건과 label 상수 2개를 taide-window로 이전하고 기존 service/types 공개 경로를 재수출 facade로 유지했습니다. 새 crate unit 12건·경계 1건·Tauri window 테스트 13건·layout 테스트 12건이 통과했습니다. taide-layout은 테스트 전용 의존으로 두고 normal feature graph는 taide-model만 참조합니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-window --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. normal feature graph는 model만 참조하고 Tauri·test-support가 없으며 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 OS 다중창 생성·닫기·복원 GUI 실기는 실행하지 않았고 M5 전체는 미완료입니다.
- [x] D. 관련 코드를 commit `321754e`로 선별 반영하고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 네 번째 slice — LSP 서비스·매니페스트 이전 (완료, 프로세스 조립은 유지)

- [x] A. LSP service·manifest는 model LSP DTO, infra lsp_install, log·serde_json과 번들 서버 JSON에 의존합니다. 기존 Tauri LSP unit 69건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다. 프로세스 생성·세션/재시작·설치 command는 Tauri 조립 경계입니다.
- [x] B. 서비스·매니페스트 구현과 unit 45건, 번들 JSON을 taide-lsp로 이전하고 기존 Tauri service/manifest/types 경로를 재수출했습니다. crate unit 45건·경계 1건·Tauri LSP 명령 24건이 통과했습니다. `toolchain_binary`만 기존 Tauri 명령 소비를 위해 공개했습니다. JSON SHA-256은 `e5b35e2727633bcd8e7fdc21f044c299c1ed25902e37eee24f3dd701b33565f3`로 이전 전후 동일합니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 2곳은 코드 텍스트로 바꾸고 재검증했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 LSP 프로세스·세션·GUI 실기는 실행하지 않았고 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `f85c1ed`로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 다섯 번째 slice — remote 인증·호스트 정책 이전 (완료, 서버 조립은 유지)

- [x] A. remote service의 토큰·비밀번호 digest, Origin/Host·와일드카드 판단은 infra crypto, model wildcard 상수, sha2·uuid 및 전용 상수 3개에만 의존합니다. 기존 unit 39건 green 뒤 새 crate 경계 테스트는 crate 부재 E0433(exit 101)으로 의도대로 실패했습니다. 서버·WebSocket·dispatch와 비밀 저장·로그인 세션 수명주기는 Tauri 조립 경계입니다.
- [x] B. 기존 정책·unit 39건과 서비스 공유 상수 3개를 taide-remote로 이전하고 Tauri service/types 공개 경로를 재수출했습니다. 새 crate unit 39건·경계 1건·기존 remote 도메인 테스트 96건이 통과했습니다. 비밀번호 해시 방식·Host/Origin·허용 목록 판정 동작은 변경하지 않았습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-remote --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. strict rustdoc의 private 링크 1곳은 코드 텍스트로 바꿔 재검증했고 새 crate 상수 문서에서 Tauri 명령 경로를 명시했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 브라우저 로그인·세션·WebSocket/dispatch 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `c5d6a59`로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 여섯 번째 slice — remote 로그인 페이지 렌더러 이전 (완료, 서버 조립은 유지)

- [x] A. 로그인 페이지는 locale 공개 팩·조회와 remote 로그인 경로 상수만 사용하며 Tauri 호출은 없습니다. 기존 unit 8건 green 뒤 새 crate 경계 테스트는 login_page 모듈·경로 상수 부재 E0433/E0425(exit 101)로 의도대로 실패했습니다.
- [x] B. 렌더러·unit 8건과 로그인 경로 상수를 taide-remote로 이전하고 기존 Tauri 공개 경로를 재수출했습니다. remote→locale 도메인 경계 화이트리스트 1건과 더 이상 맞지 않는 설명을 제거했습니다. 새 crate unit 총 47건·경계 1건·도메인 경계 3건·Tauri remote 88건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-remote --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. 경계 테스트는 기존 HTML과 새 HTML의 동일성, CSP의 script/connect 차단과 로그인 form action을 확인했고 추가된 두 CSP 단언도 전용 테스트·clippy·fmt가 통과했습니다. normal feature graph는 remote→locale→infra/model 단방향이며 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 브라우저 로그인·세션·WebSocket/dispatch 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `3204c14`로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 일곱 번째 slice — remote WebSocket 프레임 프로토콜 이전 (완료, 수명주기 조립은 유지)

- [x] A. `ws.rs`의 channel/response binary·JSON 프레임 함수 3개는 serde_json과 태그 상수만 사용합니다. 기존 ws unit 4건 green 뒤 새 byte/JSON wire fixture 경계 테스트는 protocol 모듈·상수 부재 E0433/E0425(exit 101)로 의도대로 실패했습니다.
- [x] B. 프레임 함수 3개와 태그/채널 접두사 상수 3개를 taide-remote protocol/types로 이전하고 Tauri WebSocket adapter에서 호출하게 했습니다. binary 헤더 용량은 u32 크기에서 계산하며 출력 바이트는 기존 wire와 동일합니다. 새 경계 1건·기존 ws unit 4건·crate unit 47건이 통과했습니다. writer·channel 종료와 세션 수명주기는 Tauri에 남겼습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-remote --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. byte fixture는 태그·big-endian ID/순번·raw payload를, JSON fixture는 응답 필드를 확인합니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 WebSocket 연결·writer/channel 종료 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `e002730`으로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 여덟 번째 slice — remote 채널 JSON 프레임 이전 (완료, 송신 조립은 유지)

- [x] A. `ws.rs`의 `chan`·`chanEnd` JSON 객체 생성은 serde_json과 채널 ID·순번만 사용합니다. 직전 slice의 ws unit 4건 green을 재사용하고 새 JSON wire 경계 테스트는 두 함수 부재 E0425(exit 101)로 의도대로 실패했습니다.
- [x] B. 채널 JSON·종료 프레임 함수 2개만 taide-remote protocol로 이전하고 실제 송신·`ChannelEndGuard` Drop·writer 수명주기는 Tauri에 유지했습니다. 기존 살아있는 채널 unit이 송신 프레임과 sink 해제 뒤 종료 프레임의 순번까지 확인하도록 강화했습니다.
- [x] C. 새 JSON wire 경계 2건·taide-remote unit 47건·Tauri remote 88건·`cargo clippy --workspace --all-targets -- -D warnings`·`cargo fmt --all --check`·strict remote rustdoc가 통과했습니다. 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 직전 slice의 전체 workspace 테스트 성공은 확인했지만 두 함수 이전 뒤 전체 workspace 테스트는 재실행하지 않았습니다. 실제 WebSocket 연결·채널 전송/종료 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `1367d84`로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 아홉 번째 slice — LSP workspace-folder 알림 프로토콜 이전 (완료, 세션 조립은 유지)

- [x] A. Tauri LSP 명령의 workspace-folder 알림은 LSP 서비스의 URI 인코딩을 사용해 JSON-RPC 알림을 만들고 기존 관련 unit 1건이 있습니다. 새 JSON wire 경계 테스트는 `taide_lsp::protocol` 부재 E0433(exit 101)으로 의도대로 실패했습니다. 직전 LSP 명령 테스트 24건 green은 동일 코드 상태의 결과를 재사용했습니다.
- [x] B. 폴더 JSON과 알림 직렬화를 taide-lsp protocol로 옮기고 실제 Tauri 세션 전송은 기존 명령 경로에 유지했습니다. 기존 private 함수 설명은 공개 API의 영어 rustdoc으로 정리했습니다. 새 경계 1건·Tauri LSP 명령 24건·crate unit 45건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. fixture는 공백·한글 경로 URI와 JSON-RPC method/added/removed/name을 확인합니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 LSP 프로세스·세션·재시작·GUI 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `4ae91a2`로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 열 번째 slice — remote 설정 보안 필터 이전 (완료, dispatch 조립은 유지)

- [x] A. remote dispatch의 `settings_update` patch와 `app_file_write` 전체 설정 필터는 taide-model의 Settings/SettingsPatch만 사용합니다. 새 독립 crate 경계 2건은 policy 모듈 부재 E0433(exit 101)으로 의도대로 실패했습니다. 직전 workspace 전체 green은 동일 remote 코드 상태의 결과로 재사용했습니다.
- [x] B. 두 필터를 taide-remote policy로 옮기고 실제 remote dispatch의 호출 위치는 유지했습니다. patch에서는 `remote_password_only_login`·`remote_allowed_hosts`·`shell_override`·`ai_omlx_base_url`을 제거하고, 전체 설정 쓰기에서는 네 필드를 현재값으로 복원합니다. 앞의 두 필드는 접속 게이트의 자기 확장을, 셸 경로는 세션 종료 후 지속되는 실행 경로 변경을, OMLX 주소는 저장된 API 키의 향후 전송 대상 변경을 방지합니다. `remote_access_enabled`의 자가 차단과 무관한 설정은 계속 통과합니다. 새 경계 2건·기존 dispatch unit 37건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-remote --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 브라우저 로그인·원격 세션의 설정 쓰기 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `f774471`로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 열한 번째 slice — remote 세션 owner 강제 정책 이전 (완료, dispatch 조립은 유지)

- [x] A. WebSocket의 `dispatch`/`dispatch_raw`는 인증된 원격 요청의 클라이언트 제어 JSON을 받아 명령에 전달하며, `owner`는 IDE 선택·검색·AI 요청·LSP 채널의 창별 격리에 사용됩니다. 기존 top-level·중첩·배열·무변경 회귀 5건을 확인했습니다. 새 crate 경계 2건은 owner 함수·라벨 부재 E0425(exit 101)로 의도대로 실패했습니다.
- [x] B. 모든 JSON 객체의 `owner` 키를 깊이에 관계없이 고정 `remote` 라벨로 바꾸는 함수를 taide-remote policy로, 공유 라벨을 taide-remote types로 옮겼습니다. Tauri의 기존 types 공개 경로는 재수출하고 두 dispatch 진입점의 호출 위치를 유지했습니다. 클라이언트가 `main`·`editor-*`를 보내도 데스크톱 창 소유자로 위장하지 못하도록 하는 신뢰 경계는 동일합니다. 새 경계 2건·기존 dispatch unit 37건이 통과했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-remote --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 WebSocket 연결·원격/데스크톱 owner 분리 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현은 commit `39a4523`으로 선별 반영했고 이 기록을 별도 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다.

## M5 열두 번째 slice — LSP 설치 수명주기 이전 (완료, installer 조립은 유지)

- [x] A. 대상 파일은 `src-tauri/src/domain/lsp/commands.rs`의 `LspInstallStore`·`LspInstallGuard`, `crates/taide-lsp/src/install.rs`, `src-tauri/tests/taide_lsp_install_store_extraction.rs`입니다. 동일 server ID의 중복 설치는 거부하고 취소 토큰은 현재 작업에만 전달하며, 정상 종료·패닉·대기 중 future 폐기에는 Drop이 슬롯을 해제해야 합니다. 기존 Tauri unit 2건을 확인한 뒤 새 독립 crate 경계 1건은 `taide_lsp::install` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. 설치 슬롯·Drop 가드와 기존 unit 2건을 taide-lsp로 옮기고 Tauri 명령은 installer 실행·취소만 조립합니다. 슬롯 동기화에 기존 workspace에서도 쓰는 `parking_lot`을 taide-lsp 의존성으로 추가해 패닉 시 독성 잠금이 없는 기존 동작을 유지했습니다. `begin`이 가드를 반환해 호출자가 슬롯 해제를 빠뜨릴 수 없으며, Tauri의 공개 store 경로와 `lsp_install`·`lsp_install_cancel` IPC는 그대로입니다. 대기 중 future를 poll한 다음 버릴 때도 슬롯이 풀리는 unit 1건을 추가했습니다.
- [x] C. 새 경계 1건·taide-lsp unit 48건·Tauri LSP 명령 22건·`cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `cd90578bdfeab4fc79aad8968bb489f822c34849f55322c0b2108b57566e016d`은 불변입니다. 실제 installer 프로세스·UI 취소 실기는 미실행이며 M5 전체는 미완료입니다.
- [x] D. 구현·테스트·기록은 하나의 논리 단위로 선별 commit하고 원격 `to_rust_native`에 일반 push합니다.

## M5 열세 번째 slice — LSP 메시지 구독 정책 이전 (완료, Tauri 채널·프로세스 조립은 유지)

- [x] A. `SessionEntry`의 owner별 구독 교체·명시 제거·송신 실패 정리는 `Channel<String>` 저장소에 묶여 있었고, `find_reusable_entry`는 같은 owner만 재사용 후보로 허용했습니다. 새 독립 crate 경계 2건은 `taide_lsp::session` 부재 E0432(exit 101)로 의도대로 실패했습니다. Tauri 공식 [채널 API](https://docs.rs/tauri/latest/tauri/ipc/struct.Channel.html)는 `send`의 실패 결과와 `Channel<String>`의 `Send`·`Sync`를 확인하는 근거입니다.
- [x] B. `LspMessageSubscribers`가 owner별 sink 저장·교체·제거·실패 정리를 소유하고 Tauri 명령은 `Channel<String>`을 sink로 연결합니다. 기존 `lsp_spawn`·`lsp_stop` IPC, JSON-RPC 프로세스·세션 조립은 유지합니다. 다른 창 owner가 기존 세션을 재사용하지 못하는 단언을 추가하고 `docs/ipc-contract.md`의 이전 `channels.contains_key` 표기를 현재 `subscribers.contains`로 고쳤습니다. 경계 2건·crate unit 48건·Tauri LSP 명령 22건이 통과했습니다.
- [x] C. 새 경계 2건·crate unit 48건·Tauri LSP 명령 22건·권한 허용 `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. 제한된 sandbox에서 기존 ps/로컬 소켓 관련 6건은 OS 권한 오류로 실패했고 권한 허용 재실행에서 통과했습니다. 명령 rustdoc 정리로 재생성된 `bindings.ts`의 주석만 달라져 manifest SHA-256을 `7c016bf8af6c09cdc8ac9f63daa68b9a4db748f888a75c9d0bb8cd4b431f5f6b`로 동기화하고 전체 테스트를 다시 통과했습니다. normal feature graph에 Tauri·test-support가 없습니다. 실제 LSP 프로세스·다중창 GUI 실기는 미실행입니다. 기존 `lsp_stop`은 남은 root를 판단하기 전에 subscriber를 제거하므로, 같은 owner의 여러 root가 세션을 공유할 때 잔여 root의 메시지가 끊길 수 있습니다. 이 기존 위험은 별도 M5 수명주기 검증·수정 대상으로 남깁니다.
- [x] D. 코드·테스트·IPC 문서는 commit `5a63f4c`로 선별 반영했습니다. 검증·미실기 위험을 기록한 이 문서와 PROCESS 상태를 선별 commit하고 원격 `to_rust_native`에 일반 push합니다.

## M5 열네 번째 slice — LSP root 참조 수 정책 이전 (완료, 세션·프로세스 조립은 유지)

- [x] A. `src-tauri/src/domain/lsp/commands.rs`의 root 목록은 같은 root 재사용 시 참조 수를 늘리고, 새 root에만 workspace-folder 알림을 보내며, 마지막 참조 제거 시에만 폴더 제거를 알립니다. 없는 root 해제는 기존 목록을 유지합니다. 새 독립 crate 경계 2건은 `LspSessionRoots` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `LspSessionRoots`·`LspRootRelease`가 목록·참조 수·부분/최종 해제를 소유하고 Tauri 명령은 `should_reuse_session` 판단, owner 구독, workspace-folder 알림과 프로세스 종료를 조립합니다. 직전 `docs/bug/2026-09-25-lsp-shared-root-subscriber-release.md`에서 수정한 부분 종료 후 메시지 수신 회귀를 보존했습니다. 새 경계 2건·Tauri LSP 명령 25건·crate unit 48건이 통과했습니다.
- [x] C. `cargo clippy --workspace --all-targets -- -D warnings`·`cargo fmt --all --check`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·Phase 0 IPC 계약 7건이 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `2bb2a35885f128ea9d13d7464c358fccee469f9f82c5d3c066b79190401ecd38`은 불변입니다. 전체 workspace 테스트와 TypeScript typecheck는 이 slice에서 재실행하지 않았습니다. 실제 언어서버·다중창 GUI 실기는 M7 검증입니다. LSP 세션·프로세스 전체 수명주기 이전은 M5에 남습니다.
- [x] D. 관련 코드·테스트·계약 기록은 commit `57400f9`로 선별 반영했습니다. PROCESS 상태를 문서 commit으로 남기고 원격 `to_rust_native`에 일반 push합니다.

## M5 열다섯 번째 slice — LSP 세션 상태 전이 이전 (프로세스 조립은 유지)

- [x] A. `SessionEntry`의 `status`·`last_error`·`generation`·`restart_count`·`stopping`이 Tauri 명령에 분산돼 있고, 재초기화의 세대 검사와 상태 갱신도 별개 잠금으로 수행됐습니다. 새 crate 경계 테스트 3건은 `LspSessionLifecycle` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `LspSessionLifecycle`이 상태·오류·세대의 일관된 snapshot, 종료 중 재사용 차단, 자동 복구 횟수, 수동 재시작 초기화, 현재 세대의 `Crashed` 상태에서만 성공·실패 확인 적용을 소유합니다. Tauri는 프로세스 spawn/kill, Channel, 이벤트 방출을 유지합니다. 기존 Tauri 세대·상태 테스트 5건은 새 경계 테스트로 이전했고, Tauri LSP 명령 테스트 20건과 새 경계 3건이 통과했습니다. 수동 재시작이 세대를 올리지 않는 기존 wire 의미는 보존했습니다.
- [x] C. `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·Phase 0 계약 7건·`bun run typecheck`가 통과했습니다. 제한된 sandbox의 첫 전체 테스트는 기존 `ps`·로컬 소켓 권한 6건으로 실패했고 권한 허용 재실행에서 전체가 통과했습니다. 생성 bindings는 공개 설명 주석만 변경됐고 명령·DTO wire는 동일하며 SHA-256은 `a040030bf4528b7b78031f9631484a0099bbe959dcc2a2440e248d895bea2d44`입니다. 실제 언어서버 crash/restart, 다중창·원격 세션과 오래된 프로세스 exit callback의 교차 실행은 아직 실기 검증하지 않았습니다. 프로세스 슬롯·이벤트 방출 순서는 Tauri에 남아 있으므로 전체 `LspCoordinator` 이전 완료로 보지 않습니다.
- [x] D. 구현·테스트·생성 bindings·해시는 commit `79e9688`로 선별 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 문서 commit으로 남기고 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 열여섯 번째 slice — LSP 이전 프로세스 콜백 격리 (프로세스 조립은 유지)

- [x] A. 이전 언어서버의 exit callback은 `session_id`만으로 현재 `SessionEntry`를 찾았고, 수동 재시작 뒤 `stopping`이 해제되면 새 프로세스의 종료로 오인할 수 있었습니다. 이전 exit의 backoff 작업도 대기 뒤 현재 프로세스 여부를 확인하지 않았습니다. 새 process epoch 경계 테스트는 메서드 부재 E0599와 기존 시그니처 불일치 E0061(exit 101)로 의도대로 실패했습니다.
- [x] B. `LspSessionLifecycle`이 프로세스별 비공개 epoch를 소유하고, Tauri는 spawn마다 발급한 epoch를 메시지·종료 callback에 연결합니다. 종료 callback과 backoff 재시작은 전역 mutation guard 아래에서 현재 epoch·종료 상태를 확인하며, 이전 프로세스의 늦은 메시지는 구독자에게 보내지 않습니다. 수동 재시작은 종료 중에 epoch를 먼저 교체한 뒤 새 시작 상태로 전환합니다. 프론트엔드에 공개되는 재초기화 `generation`과 IPC wire는 그대로입니다.
- [x] C. 새 경계 4건·Tauri LSP 명령 20건·`cargo fmt --all --check`·workspace all-target clippy·strict LSP rustdoc·Phase 0 계약 7건이 통과했습니다. 생성 bindings SHA-256 `a040030bf4528b7b78031f9631484a0099bbe959dcc2a2440e248d895bea2d44`는 불변입니다. 전체 workspace 테스트와 TypeScript typecheck는 이 slice에서 재실행하지 않았습니다. 실제 언어서버 프로세스의 crash/restart와 다중창·원격 세션은 아직 실기 검증하지 않았고, 프로세스 spawn/kill과 이벤트 방출 자체는 Tauri에 남습니다.
- [x] D. 코드·테스트는 commit `d8516f1`로 선별 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 문서 commit으로 남기고 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 열일곱 번째 slice — LSP 세션 저장소 소유권 이전 (프로세스 실행은 유지)

- [x] A. Tauri 명령의 `LspStore`·`SessionEntry`가 프로젝트/서버/owner별 검색, 종료 중 재사용 차단, root·구독·수명주기·프로세스 슬롯 보관, 프로젝트별 세션 snapshot과 PID 조회를 맡고 있었습니다. 새 독립 crate 경계 테스트는 `taide_lsp::store` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `LspSessionEntry`·`LspStore`가 세션 상태와 프로세스 슬롯, 검색/재사용·프로젝트 snapshot·전체 종료·PID 조회를 소유하고, 기존 Tauri `domain::lsp::commands::LspStore` 공개 경로는 재수출로 유지합니다. Tauri는 Channel 연결, 프로세스 spawn/개별 shutdown, IPC 에러 변환·이벤트 방출을 유지합니다. `lsp_sessions`의 저장소 잠금 중 snapshot 생성 순서도 보존했습니다. 삭제된 `SessionEntry`·`channels` 문서 참조는 현재 Rust/TypeScript 소스와 생성 bindings에서 실제 소유 경로로 갱신했습니다.
- [x] C. 새 경계 1건·Tauri LSP 명령 20건·권한 허용 `cargo test --workspace --quiet` 전체·`cargo fmt --all --check`·workspace all-target clippy·strict model/infra/LSP rustdoc·Phase 0 계약 7건·`bun run typecheck`·수정 TS 4파일 Prettier 검사가 통과했습니다. 생성 bindings는 공개 설명 주석만 변경됐고 SHA-256은 `db8e919fe65816b0a1666038a91ef1375c4e4168be495def84075b7cddf46d50`입니다. 실제 언어서버/다중창·원격 실기는 아직 미검증이며 `LspCoordinator`의 전체 process/JSON-RPC/replay 소유권 이전과 native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 소스 표기·생성 bindings·해시는 commit `de2fe0e`로 선별 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 문서 commit으로 남기고 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 열여덟 번째 slice — 터미널 출력 세션 정책 이전 (PTY·Tauri 채널은 유지)

- [x] A. 기존 Tauri `SessionOutput`은 스크롤백 기록·재생, 여러 구독자의 전달/제거와 구독 ID를 한 잠금으로 직렬화했습니다. 새 독립 crate 경계 테스트는 `taide_terminal::session` 부재 E0432(exit 101)로 의도대로 실패했습니다. 재부착에서 스냅샷과 구독 등록 사이에 출력이 끼어들지 않아야 하며, 리플레이 바이트 수에는 SGR 리셋 프리앰블도 포함됩니다.
- [x] B. `TerminalSessionOutput`을 `taide-terminal`로 이전하고 기존 `parking_lot` 잠금 동작과 `ScrollbackRing`의 두 조각 재생, 실패 sink의 다음 라이브 청크 정리, 구독 ID의 wrapping 증가를 보존했습니다. Tauri는 `Channel<InvokeResponseBody>`의 바이너리 송신 결과를 bool sink로 변환하고 PTY 프로세스·IPC·이벤트 조립을 유지합니다. 새 crate 경계 6건은 재생/실시간/해제, 실패 정리, 빈 스크롤백, wrap 후 바이트 수, attach 잠금, 스트리밍 경합을 검증하며 Tauri 명령 15건은 바이너리 채널 연결과 실패 변환을 포함해 통과했습니다. 현재 기능 문서의 삭제된 타입 이름도 새 경계로 갱신했습니다.
- [x] C. `cargo test --workspace --quiet` 전체(exit 0)·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 IPC 계약 7건·`bun run typecheck`·생성 bindings Prettier가 통과했습니다. normal feature graph에 Tauri·test-support가 없으며 생성 bindings는 `pty_detach` 설명 주석만 바뀌어 manifest SHA-256을 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`으로 맞췄습니다. 실제 PTY 자식 프로세스의 연속 출력·다중창 재부착 GUI 실기는 실행하지 않았습니다. 터미널 세션 저장소와 PTY 실행 자원은 아직 Tauri에 있어 M5 전체와 native UI는 미완료입니다.
- [x] D. 관련 코드·테스트·생성 bindings·해시는 commit `11acbf2`로 선별 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 문서 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 열아홉 번째 slice — 터미널 OSC 133 명령 시계 이전 (이벤트 발행은 유지)

- [x] A. 기존 Tauri `take_timed_command`는 시작 없는 종료를 무시하고, 중복 시작에서 최근 시각을 채택하며, 종료는 시작을 한 번만 소비하고, `u32`를 넘는 경과를 포화시켰습니다. 새 독립 crate 경계 테스트는 `taide_terminal::command_clock` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `TerminalCommandClock`과 `TimedCommand`를 `taide-terminal`로 옮겨 기존 `parking_lot::Mutex<Option<Instant>>` 계약을 유지했습니다. Tauri는 출력 스캔 후 현재 시각 입력, 세션 cwd 조회와 `TerminalCommandFinished` 이벤트 발행을 계속 담당합니다. 새 경계 5건은 누락·상태/경과·중복 시작/종료·연속 명령·포화를 검증하고 Tauri 터미널 명령 8건이 통과했습니다. 현재 기능 문서의 이전 `command_started_at` 표기도 갱신했습니다.
- [x] C. `cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 계약 7건이 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 이전 slice와 같습니다. 전체 workspace 테스트와 TypeScript typecheck는 코드 이전 후 재실행하지 않았습니다. 실제 셸의 OSC 133 이벤트·태스크 완료 알림 GUI 실기는 미검증이며 터미널 저장소/PTY 자원은 Tauri에 남습니다.
- [x] D. 관련 코드·테스트·현재 기능 문서는 commit `5ada297`로 선별 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 문서 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 스무 번째 slice — 터미널 세션 메타데이터 이전 (PTY 저장소는 유지)

- [x] A. 기존 Tauri `SessionEntry`의 프로젝트·cwd·셸·실행 상태는 PTY 핸들/출력과 같은 구조체에 있었고, `report_cwd_change`는 동일 cwd 보고를 무시한 뒤 저장소 잠금을 놓고 이벤트를 발행했습니다. 종료 callback은 공유 원자 실행 상태를 false로 바꿨고 `terminal_sessions`는 프로젝트별 snapshot을 만들었습니다. 새 crate 경계 테스트는 `taide_terminal::metadata` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `TerminalSessionMetadata`가 불변 프로젝트/셸, 잠금으로 보호하는 cwd, 원자 실행 상태와 `TerminalSession` snapshot을 소유합니다. Tauri는 PTY 핸들·출력·`TerminalStore` 지도/잠금, 프로세스 종료 이벤트와 cwd/명령 완료 이벤트 발행을 유지합니다. 종료 callback과 세션 진입점은 같은 `Arc` 메타데이터를 사용하고, 기존 저장소→세션 잠금 순서 및 spawn/kill/attach IPC는 유지했습니다. 새 경계 2건과 Tauri 터미널 명령 8건이 통과했습니다.
- [x] C. 권한 허용 `cargo test --workspace --quiet` 전체(exit 0)·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 IPC 계약 7건이 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. TypeScript는 수정하지 않아 typecheck를 재실행하지 않았습니다. 실제 PTY 조기 종료·cwd 변경 이벤트/GUI 실기는 미검증이고 `TerminalStore` 및 PTY 회수는 여전히 Tauri에 남아 M5 전체와 native UI는 미완료입니다.
- [x] D. 코드·테스트는 commit `2bbed92`로 선별 반영했습니다. 검증 기록과 PROCESS 상태를 별도 문서 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 스물한 번째 slice — 터미널 세션 저장소 이전 (프로세스 spawn·이벤트는 Tauri 유지)

- [x] A. 기존 `TerminalStore`는 PTY 핸들·메타데이터·출력의 지도, 프로젝트별 PID 조회와 회수, 개별 종료·resize·pause, attach/detach를 보관했습니다. writer 핸들은 저장소 잠금 안에서 복제하고 실제 블로킹 쓰기 전에 잠금을 놓으며, attach는 저장소→출력 잠금 순서 아래 재생/구독을 수행합니다. 새 독립 crate 경계 테스트는 `taide_terminal::store` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `TerminalStore`와 `TerminalSessionEntry`를 `taide-terminal`로 이전하고 기존 `domain::terminal::commands::TerminalStore` 경로는 재수출했습니다. 누락 세션의 `NotFound`, 없는 detach의 성공, 프로젝트별 회수 시 다른 세션 유지, PTY writer/resize/pause/kill과 snapshot/출력 연결 계약을 유지했습니다. Tauri에는 프로세스 spawn, mutation guard, Channel 바이너리 sink, cwd/명령 완료/생성/종료 이벤트 조립을 남겼습니다. 실제 `/bin/sh` PTY 2개를 생성하는 새 경계 2건과 Tauri 터미널 명령 8건이 통과했고 `docs/ipc-contract.md`의 삭제된 `SessionEntry.cwd` 표기를 현재 메타데이터로 갱신했습니다.
- [x] C. 권한 허용 `cargo test --workspace --quiet` 전체(exit 0)·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 IPC 계약 7건이 통과했습니다. 마지막 테스트의 누락 세션 오류를 `AppError::NotFound`로 강화한 뒤 경계 2건·해당 test clippy·fmt를 다시 통과했으며, 전체 workspace는 이 테스트 전용 단언 뒤 재실행하지 않았습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 추가로 시도한 strict Tauri rustdoc은 수정하지 않은 AI·Git·layout·LSP·remote·sync·window 등의 공개 문서가 private 항목에 링크한 기존 오류(exit 101)로 실패했고 터미널 경로 오류는 없었습니다. TypeScript는 수정하지 않았고 실제 다중창·PTY GUI 실기는 미검증입니다. M5의 다른 도메인 결합과 M6 adapter, native UI는 계속 미완료입니다.
- [x] D. 코드·테스트·현재 IPC 문서는 commit `f38f42b`로 선별 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 문서 commit으로 반영해 원격 `to_rust_native`에 일반 push합니다. M5 전체는 미완료로 유지합니다.

## M5 스물두 번째 slice — 터미널 PTY 실행·출력 스캔 조립 이전 (Tauri async·이벤트 유지)

- [x] A. 기존 `pty_spawn`의 출력 콜백은 청크별 계측→스크롤백 기록·구독 전달→세션별 OSC 스캔→스캔 계측·Tauri 이벤트 순서였고, 종료 콜백은 세션 실행 상태와 종료 이벤트를 갱신했습니다. `pty::spawn`의 출력 `Fn`·종료 `FnOnce` 경계를 확인했으며 새 독립 crate 프로세스 테스트는 `taide_terminal::runtime` 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. `spawn_terminal_session`이 실제 PTY 생성과 세션별 `OutputScanner`를 소유하며 기존 출력 순서를 유지합니다. Tauri는 `spawn_blocking` 스케줄링, 계측·`AppHandle` 이벤트, IPC 및 mutation guard를 계속 조립합니다. 실제 `/bin/sh` PTY에 OSC 7을 써서 계측·리플레이 완료 뒤 스캔 콜백에 cwd가 전달되는 경계 1건과 기존 Tauri 터미널 명령 8건이 통과했습니다. 현재 터미널 기능 문서의 스캐너 소유 표기도 갱신했습니다.
- [x] C. 권한 허용 `cargo test --workspace --quiet` 전체(exit 0)·`cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 계약 7건이 통과했습니다. normal feature graph에 Tauri·test-support가 없고 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. TypeScript는 수정하지 않아 typecheck를 재실행하지 않았습니다. 실제 GUI·다중창 PTY 실기는 미검증이며 M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 기능 문서는 commit `4fb6dfb`로, 이 검증 기록과 PROCESS 상태는 commit `2847b62`로 각각 선별 로컬 반영했습니다. 원격 `to_rust_native` 일반 push는 안전 검토에서 목적지·payload 승인 부족으로 거절되어 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료로 유지합니다.

## M5 스물세 번째 slice — 터미널 링크 경로 인가 정책 이전 (IPC·프로젝트 snapshot은 Tauri 유지)

- [x] A. Tauri `guard_terminal_path`는 터미널 출력의 경로와 자식 프로세스가 제어하는 cwd를 정규화한 뒤 열린 프로젝트 루트 소유권을 확인합니다. 루트 밖 경로와 부재 경로를 모두 `NotFound`로 접고, 행별 후보는 입력 순서를 유지하면서 16개 초과를 `None`으로 답합니다. 기존 Tauri 테스트 6건을 확인했고 새 독립 crate 경계 3건은 API 부재 E0432(exit 101)로 의도대로 실패했습니다.
- [x] B. 경로 인가·후보 상한을 `taide-terminal::service`로 옮기고 기존 `taide-infra::root_guard`와 정규화 경로를 그대로 사용했습니다. Tauri의 두 IPC 명령은 열린 프로젝트 snapshot 추출과 서비스 호출만 유지하며 반환 wire와 원격 허용 분류는 바꾸지 않았습니다. 새 경계 3건과 기존 Tauri 터미널 명령 8건이 통과했고 현재 기능 문서의 소유 표기도 갱신했습니다.
- [x] C. `cargo test -p taide-terminal --test path_policy` 3건·`cargo test -p taide --lib domain::terminal::commands` 8건·`cargo fmt --all --check`·`cargo clippy -p taide-terminal --all-targets -- -D warnings`·`cargo clippy -p taide --lib -- -D warnings`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-terminal --no-deps`·Phase 0 계약 7건이 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 변경 경계 밖 전체 workspace 테스트와 TypeScript typecheck는 재실행하지 않았으며 실제 GUI 링크 클릭·원격 미러 실기는 미검증입니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 검증된 코드·테스트·현행 기능 문서는 commit `11b6d8d`로 선별 로컬 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기고 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료로 유지합니다.

## M5 스물네 번째 slice — layout 탭 닫기 후처리 조립 분리 (Tauri 생명주기 유지)

- [x] A. 기존 `layout::service::close_tab_and_finish`는 mutation guard 아래 탭을 닫고 layout snapshot을 기록한 뒤 IDE pending Claude diff 응답을 해소하고 터미널 PTY를 회수했습니다. layout command와 IDE MCP 도구가 이 경로를 공유합니다. 기존 domain boundary 3건을 확인한 뒤 layout→IDE·terminal 화이트리스트 2개를 제거하자 참조 2건이 정확히 검출되어 집중 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. `LayoutTabClosedObservers`를 layout 서비스의 조립 port로 두고 `lib.rs`에서 IDE 저장소 반응과 터미널 세션 회수를 기존 순서로 등록했습니다. IDE pending diff 해소는 `IdeStore::reconcile_closed_tab` 메서드로 옮겼고, layout writeback 뒤 observer를 실행합니다. 기존 탭 닫기 IPC·이벤트·mutation guard 순서는 변경하지 않았으며 IDE 저장소의 해당 탭만 해소·중복 닫기 회귀와 조립 순서 계약을 추가했습니다. layout→IDE·terminal 직접 참조는 없어졌습니다.
- [x] C. `cargo test -p taide --test domain_boundaries` 3건·`cargo test -p taide --lib domain::ide::store` 14건·`cargo test -p taide --lib domain::layout::service` 4건·조립 순서 집중 테스트 1건·Phase 0 계약 7건·`cargo fmt --all --check`·`cargo clippy -p taide --lib -- -D warnings`·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 전체 workspace 테스트·TypeScript typecheck는 재실행하지 않았습니다. 실제 GUI 탭 닫기·PTY 종료는 미실기이며, layout↔window의 허용 순환과 IDE→layout 조립 의존은 다음 M5 작업에 남습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 아키텍처 문서는 commit `598b914`로 선별 로컬 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기고 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 스물다섯 번째 slice — layout·window 명령 조립 상향 (창 닫힘 탭 복귀는 유지)

- [x] A. `layout::commands::layout_move_tab_to_window`는 mutation guard 안에서 OS 보조 창을 먼저 연 뒤 layout 탭을 옮기고, 실패한 이동의 창을 닫으며 빈 보조 창을 정리했습니다. 반대 방향의 `window::service`는 닫힌 보조 창 탭을 layout에 복귀시킵니다. 기존 domain boundary 3건 green 뒤 layout→window 허용 항목을 제거하자 해당 참조 하나만 검출되어 집중 테스트가 의도대로 실패했습니다(exit 101). [Tauri command 공식 문서](https://v2.tauri.app/develop/calling-rust/)는 조립부로 옮겨도 command의 Rust 함수 경로와 프론트 호출 이름이 분리됨을 확인하는 근거입니다.
- [x] B. 탭의 OS 창 생성·layout 이동·실패 rollback·빈 창 정리를 `lib.rs`의 `layout_move_tab_to_window` command로 옮기고 Specta 등록 순서를 유지했습니다. Tauri 도메인 간 layout→window 참조가 없어져 이전 순환은 끊겼고, 창 닫힘 탭 복귀의 window→layout 참조는 그대로입니다. Phase 0 원천 검사기는 도메인 경로뿐 아니라 조립부에 직접 등록된 명령도 선언 순서대로 읽도록 고쳤습니다. 처음 Phase 0 실행의 `spectaCommands` 한 건 stale 실패(exit 101)는 이 스캐너 가정 때문이었으며 검사기 수정 뒤 7건이 통과했습니다. 조립부의 mutation guard→창 생성→탭 이동→rollback→정리→layout 기록 순서를 소스 계약 테스트로 고정했습니다.
- [x] C. `cargo test -p taide --test domain_boundaries` 3건·조립 순서 집중 테스트 1건·Tauri window 명령 6건·layout 명령 8건·`cargo test -p taide-layout --lib` 104건·Phase 0 계약 7건·`cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 전체 workspace 테스트·TypeScript typecheck와 실제 다중 창 GUI 생성·닫기는 이번 slice에서 실행하지 않았습니다. 남은 window→layout 창 닫힘 탭 복귀와 IDE→layout 단방향 참조, M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 아키텍처 문서는 commit `abd7d81`로 선별 로컬 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기고 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 스물여섯 번째 slice — 보조 창 닫힘의 mirror·layout 조립 상향

- [x] A. 기존 window 서비스는 `CloseRequested`의 scoped flush 뒤 재요청 또는 `Destroyed`에서 창 등록을 한 번만 해제하고, Tauri async 작업 안에서 mirror 조회→유령 dirty 정리→layout 탭 복귀→기록·이벤트를 수행했습니다. mirror 조회 실패는 빈 목록으로 접고, 이미 닫힌 슬롯·프로젝트는 멱등 no-op입니다. 기존 domain boundary 3건 green 뒤 window→file/layout 허용 두 항목을 제거하자 두 참조가 정확히 검출되어 집중 테스트가 의도대로 실패했습니다(exit 101). [Tauri `WindowEvent` 문서](https://docs.rs/tauri/latest/tauri/enum.WindowEvent.html)는 `CloseRequested`와 `Destroyed`가 별도 이벤트임을 확인하는 근거입니다.
- [x] B. window command는 기존 flush handshake와 `WindowStore::forget` 결과 `(project_id, slot)`만 반환합니다. `lib.rs`의 두 창 이벤트 분기가 같은 탭 복귀 함수를 호출하고, 그 함수가 기존 mirror 실패 정책·mutation guard·layout 기록과 `LayoutChanged` 발행 순서를 유지합니다. window→file/layout 직접 참조 두 건을 제거하고 상위 조립 순서 계약 테스트를 추가했습니다. `window/service.rs`에는 독립 `taide-window` 서비스 재수출만 남습니다.
- [x] C. `cargo test -p taide --test domain_boundaries` 3건·조립 순서 집중 테스트 1건·Tauri window 명령 6건·layout 서비스 4건·Phase 0 계약 7건·`cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·`git diff --check`가 통과했습니다. 첫 clippy는 `Option` 조기 반환 표현을 지적해 실패(exit 101)했고 `?`로 수정한 뒤 같은 검사가 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 전체 workspace 테스트·TypeScript typecheck와 실제 다중 창 GUI 닫기·flush timeout은 이번 slice에서 실행하지 않았습니다. IDE→layout 및 remote gateway 등 남은 결합, M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 아키텍처 문서는 commit `e041b83`로 선별 로컬 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기고 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 스물일곱 번째 slice — IDE MCP 탭 수명주기 조립 분리

- [x] A. IDE MCP의 `openFile`·`close_tab`·`closeAllDiffTabs`는 Tauri layout 서비스의 open/close를 직접 호출했고, 열린 탭 탐색·diff ID 추출은 그 서비스가 재수출하는 독립 layout crate의 순수 함수였습니다. 코드 변경 전 IDE 서버 12건은 제한 샌드박스에서 루프백 소켓 3건만 `Operation not permitted`로 실패했으며 권한 허용 동일 명령에서 모두 통과했습니다. 기존 domain boundary 3건 green 뒤 IDE→layout 서비스 허용 항목을 제거하자 참조 한 건이 정확히 검출되어 집중 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. IDE 서버의 순수 탐색은 `taide_layout::service`를 직접 사용하고, `IdeLayoutActions`의 open/close 함수 포트는 `lib.rs`에서 기존 `layout::service::open_tab_and_finish`·`close_tab_and_finish`에 배선했습니다. 앱 상태에 포트를 등록한 뒤 IDE 자동 시작 경로가 실행됩니다. `openFile`의 열린 프로젝트 루트·파일 존재 선검증, MCP 텍스트/JSON 응답, 닫힘 뒤 `IdeCloseTabRequested`와 pending diff 해소·PTY 회수는 기존 경로와 순서를 유지했습니다. IDE→layout 직접 실행 참조를 제거하고 세 MCP 경로가 포트를 사용하는 조립 계약 테스트를 추가했습니다.
- [x] C. 권한 허용 `cargo test -p taide --lib domain::ide::server` 12건·domain boundary 3건·조립 계약 1건·Phase 0 계약 7건·`cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 전체 workspace 테스트·TypeScript typecheck와 실제 Claude Code MCP 클라이언트·GUI 조작은 이번 slice에서 실행하지 않았습니다. M5의 remote gateway·잔여 결합, M6 adapter와 native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 아키텍처 문서는 commit `3c9cbe9`로 선별 로컬 반영했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기고 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 스물여덟 번째 slice — remote 전 도메인 command 게이트웨이 조립 상향

- [x] A. 기존 `src-tauri/src/domain/remote/dispatch.rs`는 전 도메인 command 테이블·기본 거부 정책·JSON/raw dispatch와 정책/wire 단위 테스트 37건을 함께 보유했고, `remote/ws.rs`가 이 구현을 직접 호출했습니다. 변경 전 gateway unit 37건이 통과했고 직전 Phase 0 계약 7건을 재사용했습니다. domain boundary의 유일한 import 형태 예외를 제거하자 `domain/remote/dispatch.rs` 한 파일만 정확히 검출되어 집중 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. 전체 게이트웨이 구현과 37개 테스트를 `src-tauri/src/remote_gateway.rs`로 옮기고 remote 도메인에는 `ChannelFactory`·JSON/raw 함수 포인터 시그니처의 `RemoteDispatchPort`만 남겼습니다. `lib.rs`가 포트를 관리 상태에 등록하고 WebSocket은 이 포트로 양쪽 dispatch를 호출합니다. 이전 파일과 새 파일의 직접 diff는 import·채널 타입 소유·`include_str!` 경로 변경만 보여 명시 허용/거부 표, owner 강제, raw 응답 처리와 명령 본문이 불변임을 확인했습니다. Phase 0 manifest의 원천 경로와 현행 아키텍처·IPC·AI 기능 문서의 파일 표기를 갱신했습니다. [Tauri 관리 상태 API](https://v2.tauri.app/develop/state-management/)의 `manage`·`AppHandle::state` 계약에 맞춰 조립했습니다.
- [x] C. 이동한 gateway unit 37건·remote 도메인 unit 51건(그중 WebSocket 4건)·domain boundary 3건·Phase 0 계약 7건·조립 배선 1건·`cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·`git diff --check`가 통과했습니다. 첫 clippy는 JSON/raw 함수 포인터의 복잡한 타입 2건으로 실패(exit 101)했고 명명된 공개 타입 별칭으로 분리한 뒤 같은 검사가 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 전체 workspace 테스트·TypeScript typecheck와 실제 원격 WebSocket 연결·인증/인가 실기는 이번 slice에서 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현재 아키텍처/IPC/AI 문서를 `5836ccc`로 선별 로컬 commit하고 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남깁니다. 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 스물아홉 번째 slice — native 메뉴 조회의 도메인 경계 정리

- [x] A. `domain/window/menu.rs`의 locale·project 서비스 직접 참조 두 건과 `lib.rs`의 클릭 시점 재조회 계약을 확인했습니다. 변경 전 메뉴 단위 7건·경계 3건이 통과했고 두 허용 항목을 제거하자 정확히 `locale::service`·`project::service` 두 참조만 검출되어 경계 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. `MenuSources`를 window 메뉴의 조회 포트로 두고 `lib.rs`가 최근 프로젝트 목록과 내장 번역 fallback을 공급합니다. 메뉴의 상위 10건·표시 라벨 우선·부재 루트 비활성 정책은 그대로이며, `Open Recent` 클릭 시 디스크 레코드를 다시 읽어 프로젝트를 여는 경로는 `lib.rs` 조립부로 옮겼습니다. 조회 실패 시 빈 메뉴 경고·메뉴 이벤트 비동기 실행·갱신의 blocking 작업 분리·언어 변경 시 전체 재구성 순서는 보존했습니다. `window/menu.rs`의 교차 실행 참조 두 건을 경계 화이트리스트에서 제거하고 현행 아키텍처·window 기능 문서를 갱신했습니다.
- [x] C. 메뉴 단위 7건·window command 단위 6건·도메인 경계 3건·조립 배선 1건·Phase 0 계약 7건, `cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `3ce82f3fda029e700a8f26ad5a08869b7452d337c32e11df82f670b9d33f0993`은 불변입니다. 실제 OS 메뉴 생성·언어 전환·최근 프로젝트 클릭 GUI 실기와 전체 workspace·TypeScript 검사는 이번 slice에서 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `fa17de4`로 선별 로컬 commit하고 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남깁니다. 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 서른 번째 slice — agent 전경 PID 공급 경계 분리

- [x] A. `agent_list`와 `poll_agents`의 `TerminalStore::foreground_pids` 조회를 확인했습니다. 제한 환경의 기존 agent 집중 7건 중 `ps`가 자기 PID를 찾지 못한 1건은 실패(exit 101)했으나 권한 허용 동일 명령은 7건 모두 통과했습니다. 기존 경계 3건 green 뒤 agent→terminal 허용 항목을 제거하자 그 한 참조만 정확히 검출되어 경계 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. `AgentForegroundPids` 함수 포트를 agent 도메인에 두고 `lib.rs`가 관리 중인 터미널 Store의 read-only 전경 PID 조회를 배선했습니다. `agent_list`와 polling이 이 포트를 공유하고 원격 게이트웨이의 직접 호출에도 AppHandle·상태 인자를 전달합니다. 첫 컴파일은 게이트웨이의 기존 인자 목록으로 실패(exit 101)했으며 이를 고친 뒤 기존 PID 이름 탐침·캐시·상태 이벤트와 `agent_list(projectId)` IPC wire를 유지했습니다. agent→terminal 허용 항목을 제거하고 현행 아키텍처·기능 문서를 갱신했습니다.
- [x] C. 권한 허용 agent 집중 7건·remote gateway 37건·도메인 경계 3건·조립 배선 1건·Phase 0 계약 7건과 bindings 생성 1건, `cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·생성 bindings Prettier·`git diff --check`가 통과했습니다. 재생성된 bindings는 IPC 타입·시그니처가 불변이고, 앞선 remote 파일 이동·layout 명령 조립 상향·터미널 경로 명령의 소스 설명 주석을 반영했습니다. 설명의 공백 줄이 생성 파일에 trailing whitespace를 만들던 원천 주석을 정리하고 조립 명령의 공개 IPC 설명을 현행 순서로 맞췄습니다. manifest SHA-256은 `7930246598910fd63595a0d64301e45e1f5a032e48478d5e1f1bb8b635d6be21`입니다. 실제 에이전트 프로세스·원격 클라이언트·GUI 실기와 전체 workspace·TypeScript typecheck는 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·생성 bindings·현행 문서를 `b12455b`로 선별 로컬 commit하고 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남깁니다. 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 서른한 번째 slice — IDE diff 보호 저장 경계 분리

- [x] A. IDE `ide_resolve_diff`의 `Saved` 분기는 mutation guard 안에서 파일 도메인의 `save_file_within_open_projects`를 직접 호출했습니다. 그 함수의 root guard·원자 쓰기·self-write·미러 정리와 IDE의 `Forbidden` 경고 후 완료·기타 오류 전파를 확인했습니다. 기존 IDE store 14건·파일 추출 2건·경계 3건이 통과했고 허용 항목을 제거하자 `domain/ide/commands.rs → file::service` 한 참조만 정확히 검출되어 경계 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. `IdeSaveFile` 함수 포트를 IDE 도메인에 두고 `lib.rs`가 동일한 파일 저장 함수를 배선했습니다. 직접 IPC와 원격 게이트웨이의 `ide_resolve_diff` 호출에 같은 포트 상태가 주입되며 기존 파일 저장 함수·mutation guard·오류 분기는 바꾸지 않았습니다. 첫 배선 테스트는 게이트웨이 소스 줄바꿈에 맞지 않는 종료 마커 때문에 실패(exit 101)했고 수정 뒤 통과했습니다. IDE→file 실행 참조 허용 항목을 제거하고 현행 아키텍처·기능 문서의 IDE open/close 조립 표기도 맞췄습니다.
- [x] C. IDE store 14건·파일 추출 2건·도메인 경계 3건·조립 배선 1건·remote gateway 37건·Phase 0 계약 7건과 bindings 생성 1건, `cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·생성 bindings Prettier·`git diff --check`가 통과했습니다. bindings는 IDE 공개 설명 문구만 바뀌고 IPC 시그니처는 불변이며 manifest SHA-256은 `64e86f9482c2c2feee794dfd79156d3528cf6a0cbabea98725d852c82e0d44af`입니다. 실제 Claude MCP diff 저장·원격 클라이언트·GUI 실기와 전체 workspace·TypeScript typecheck는 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·생성 bindings·현행 문서를 `ca5e831`로 선별 로컬 commit하고 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남깁니다. 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 서른두 번째 slice — app·sync 설정 적용 경계 분리

- [x] A. app 파일 저장·설정 적용과 sync 연결·해제·업로드·다운로드의 설정 parse·save·apply 순서, mutation guard 및 원격 gated 필드 정책을 확인했습니다. 변경 전 sync 단위 12건·설정 분리 경계 1건·도메인 경계 3건이 통과했고 허용 항목 4개를 제거하자 `app→settings::commands/service`와 `sync→settings::commands/service` 참조 4건만 검출되어 경계 테스트가 의도대로 실패했습니다(exit 101).
- [x] B. app의 JSON parse와 sync의 설정 저장 3곳은 기존 재수출의 원천인 `taide-settings` 함수를 직접 호출합니다. 공통 설정 적용은 루트 조립부에 등록한 `SettingsApplyPort`로 연결해 기존 `apply_and_broadcast`를 유지했습니다. 직접 IPC·원격 게이트웨이의 app 파일 저장/원격 sanitized 적용/sync 다운로드가 같은 포트를 사용하며 기존 guard·오류·이벤트 순서는 바꾸지 않았습니다. [Tauri 관리 상태 API](https://v2.tauri.app/develop/state-management/)의 `manage`와 command `State` 계약에 맞춰 조립했고, 경계 허용 4건을 제거했습니다.
- [x] C. sync 단위 12건·remote gateway 37건·도메인 경계 3건·새 조립 배선 1건·Phase 0 계약 7건·bindings 생성 1건, `cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·bindings Prettier·`git diff --check`가 통과했습니다. 첫 fmt 검사는 새 배선 테스트 줄바꿈만 지적했고 포맷 후 통과했습니다. 생성 bindings는 `app_file_write` 공개 설명만 변경되어 IPC 시그니처는 불변이며 manifest SHA-256은 `4d2128775d42b25797a5c38b8ede59dca59d1b26caceca3d0d3e43915ab3e96c`입니다. 전체 workspace 테스트·TypeScript typecheck·실제 원격/Gist/GUI 실기는 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·생성 bindings·현행 아키텍처 문서는 `b64efea`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 서른세 번째 slice — 플러그인 캐시·VSIX 설치 조립 경계 분리

- [x] A. file·git·IDE의 플러그인 언어 조회 3곳은 `PluginStore` read-through 캐시와 overlay 변환을 공유하고, VSIX 임포트는 아카이브 스테이징 후 mutation guard 안에서 설치를 확정해 store를 재로드합니다. file/git은 원격 gateway에서 직접 호출되고 VSIX 임포트는 원격 거부 대상입니다. 기존 도메인 경계 3건·`taide-plugin` 단위 26건이 통과했으며 허용 항목 4개를 제거하자 해당 네 참조만 검출되어 경계 테스트가 의도대로 실패했습니다(exit 101). Store를 crate 경로로 우회하지 않고 조립부 포트로 옮기기로 했습니다.
- [x] B. 루트 `PluginRuntimePort`가 기존 `taide-plugin` 서비스로 같은 Store의 언어 overlay를 조회하고 VSIX staged install 확정·재로드를 수행합니다. file/git/IDE/VSIX 도메인은 관리 상태의 포트만 참조합니다. VSIX 스테이징과 guard 순서, 설치 오류 코드/문구, 원격의 VSIX 거부 정책을 유지했으며 Git diff는 인자 상한을 넘지 않도록 `AppHandle`에서 기존 AppState를 조회합니다. 도메인 허용 4건을 제거하고 현행 아키텍처·플러그인 기능 문서를 갱신했습니다.
- [x] C. 기존 `taide-plugin` 단위 26건·IDE MCP 12건·Git 명령 20건·원격 gateway 37건·파일 분리 2건·도메인 경계 3건·조립 배선 1건·Phase 0 계약 7건·bindings 생성 1건이 통과했습니다. 첫 fmt는 줄바꿈·모듈 순서를 지적해 포맷 후 통과했고 첫 clippy는 Git diff 인자 8개를 지적해 상태 조회 경로를 고친 뒤 `cargo clippy -p taide --all-targets -- -D warnings`가 통과했습니다. 생성 bindings는 VSIX 공개 설명 한 줄만 변경되어 IPC 시그니처는 불변이고 manifest SHA-256은 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`입니다. bindings Prettier·`git diff --check`도 통과했습니다. 전체 workspace 테스트·TypeScript typecheck·실제 플러그인 설치/GUI·원격 실기는 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·생성 bindings·현행 문서는 `2c500da`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 서른네 번째 slice — 프로젝트 부팅 복구의 도메인 경계 분리

- [x] A. 부팅 setup은 세션·layout·settings를 첫 창 전에 동기 로드하고 복원 프로젝트의 watcher를 활성 프로젝트 우선 순서로 백그라운드 재부착합니다. file/git watcher build는 mutation guard 밖, 프로젝트 열림·기존 watcher 재검증과 등록은 guard 안, 합성 변경 이벤트는 guard 해제 뒤이며 종료·중복/닫힘 skip 정책을 유지해야 합니다. 기존 project 명령 11건·경계 3건이 통과했고 허용 항목 4건을 제거하자 `project→file::capability/git::watch/layout::service/settings::service` 4건만 검출되어 경계 검사가 의도대로 실패했습니다(exit 101).
- [x] B. project 도메인은 복구 대상 선정과 guard·경합·이벤트 흐름을 유지합니다. layout/settings 로드는 기존 재수출의 원천인 `taide-layout`/`taide-settings` 함수로 바꾸고, file/git watcher build/register 함수는 `lib.rs`가 관리 상태에 등록한 `ProjectRestoreWatchers`로 공급합니다. 등록은 기존 `restore_project_watchers` 시작보다 먼저 수행하며 배선 테스트가 build→guard→file/git 등록 순서를 확인합니다. 도메인 경계 허용 목록은 비웠고 현행 아키텍처의 오래된 예외 설명을 갱신했습니다.
- [x] C. project 명령 11건·Git watcher 9건·layout/settings 추출 각 1건·도메인 경계 3건·조립 배선 1건·Phase 0 계약 7건·bindings 생성 1건, `cargo fmt --all --check`·`cargo clippy -p taide --all-targets -- -D warnings`·`git diff --check`가 통과했습니다. 첫 fmt는 새 줄바꿈만 지적해 포맷 후 통과했습니다. 생성 bindings SHA-256 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`는 불변입니다. 전체 workspace 테스트·TypeScript typecheck·실제 다중 프로젝트 부팅/워처 경합 GUI 실기는 실행하지 않았습니다. M5의 다른 조립/자원 결합, M6 adapter, native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 아키텍처 문서는 `191e6ee`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다.

## M5 서른다섯 번째 slice — IDE MCP wire의 독립 crate 이전

- [x] A. `domain/ide/server.rs`의 JSON-RPC envelope·초기화/도구 목록·선택/진단 직렬화는 Tauri 비의존이고 WebSocket 인증·전송·도구 실행은 AppHandle 의존임을 확인했습니다. 기존 `taide-ide` 단위 12건·권한 허용 IDE server 12건이 통과했고, 신규 wire 경계 2건은 `taide_ide::protocol` 부재 E0432(exit 101)로 의도대로 실패했습니다. [MCP 기본 프로토콜](https://modelcontextprotocol.io/specification/2025-03-26/basic)·[도구 wire](https://modelcontextprotocol.io/specification/2025-03-26/server/tools)와 serde 공식 API를 확인했습니다.
- [x] B. 순수 MCP wire와 선택 스냅샷을 `taide-ide::protocol`로 이전했습니다. IDE 서버는 Tauri WebSocket·인가·도구 실행을 유지하며, 초기화 `serverInfo.version`은 기존 앱 패키지 버전을 인자로 전달합니다. IDE 명령은 crate의 알림 생성 함수를 사용합니다. 현행 아키텍처·에이전트 연동 문서에 소유 경계를 반영했습니다.
- [x] C. `taide-ide` 단위 20건·신규 wire 경계 2건·Tauri IDE 영역 32건·Phase 0 계약 7건·bindings 생성 1건, crate/Tauri clippy·fmt·독립 crate rustdoc·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`는 불변입니다. 전체 workspace 테스트·TypeScript typecheck와 실제 Claude MCP 연결/GUI는 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `871d7ea`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 서른여섯 번째 slice — IDE lockfile 자원 정책의 독립 crate 이전

- [x] A. lockfile 경로·권한·원자 쓰기·stale PID 판정은 Tauri `AppHandle`에 의존하지 않고 `taide-model` DTO, `taide-infra::persist`, sysinfo·log만 사용합니다. 기존 lockfile 14건이 통과했고 신규 crate 경계 2건은 `taide_ide::lockfile` 부재 E0432(exit 101)로 의도대로 실패했습니다. Rust `DirBuilder`·serde_json 공식 문서를 확인했습니다.
- [x] B. lockfile 구현·식별 상수와 기존 14개 테스트를 `taide-ide::lockfile`로 옮기고 Tauri lockfile/types 공개 경로는 facade로 유지했습니다. 토큰의 private atomic write(Unix 디렉터리 0700·파일 0600), 살아 있는 PID·타 IDE·깨진 JSON 보존, 죽은 TAIDE PID 정리 정책과 command의 기동·갱신·종료 순서는 바꾸지 않았습니다. 현행 아키텍처·에이전트 연동 문서의 소유 경로를 갱신했습니다.
- [x] C. `taide-ide` 단위 34건·신규 경계 2건·Tauri IDE 영역 18건·기존 model 외부 wire 2건·Phase 0 계약 7건·bindings 생성 1건, crate/Tauri clippy·fmt·독립 crate rustdoc·`git diff --check`가 통과했습니다. 첫 fmt는 이동 파일 import·줄바꿈만 지적해 포맷 후 통과했습니다. 생성 bindings SHA-256 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`는 불변입니다. 전체 workspace·TypeScript typecheck·실제 Claude MCP 연결/GUI는 실행하지 않았습니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `9c6ee72`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태를 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 서른일곱 번째 slice — LSP 프로세스 기동의 독립 crate 이전

- [x] A. `domain/lsp/commands.rs::spawn_process`가 실행 파일·인자 템플릿 해석과 실제 자식 프로세스 spawn을 소유하고 있었습니다. 기존 Tauri LSP 명령 20건이 권한 허용 환경에서 통과했고, 새 crate 프로세스 경계 3건은 `taide_lsp::process` 부재 E0432(exit 101)로 의도대로 실패했습니다. Rust `var_os`·Tokio 프로세스 공식 문서를 확인했습니다.
- [x] B. 관리 설치 경로·실행 파일·인자 템플릿 해석과 실제 프로세스 기동을 `taide-lsp::process`로 이전했습니다. Tauri에는 AppHandle 세션 epoch 확인, 메시지 구독 전송 및 종료 시 mutation guard·이벤트 콜백만 남겼습니다. 기존 실행 파일 부재·미해결 템플릿 오류와 spawn 로그·콜백 순서를 유지하고 새 경계 3건이 통과했습니다. 현행 아키텍처·LSP 기능 문서의 소유 표기를 갱신했습니다.
- [x] C. 권한 허용 `cargo test --workspace --quiet` 전체(exit 0)에 새 crate 프로세스 경계 3건·Tauri LSP·Phase 0 IPC 계약이 포함되어 통과했습니다. `cargo clippy --workspace --all-targets -- -D warnings`·`cargo fmt --all --check`·`RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp --no-deps`·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`는 불변입니다. TypeScript 파일은 변경하지 않아 typecheck를 재실행하지 않았고 실제 LSP 서버·GUI 세션 실기는 미검증입니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `394d2fb`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 서른여덟 번째 slice — LSP 종료 정책의 독립 crate 이전

- [x] A. Tauri `shutdown_entry`가 stopping 설정 후 `shutdown` 요청→조기 종료 폴링(2초)→`exit` 알림→조기 종료 폴링(2초)→kill을 가드 밖에서 실행했습니다. 기존 조기 종료·타임아웃 테스트 2건을 확인했고, 새 crate 종료 API 테스트는 `shutdown_process` 부재 E0432(exit 101)로 의도대로 실패했습니다. [LSP 3.17 생명주기 사양](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)과 [Tokio 시간 API](https://docs.rs/tokio/latest/tokio/time/fn.sleep.html)를 확인했습니다.
- [x] B. 종료 순서와 폴링 상수를 `taide-lsp::process::shutdown_process`로 이전하고, 기존 조기 종료·타임아웃 테스트 2건을 crate 소유로 옮겼습니다. Tauri는 세션 stopping·프로세스 snapshot·최종 상태 이벤트와 `lsp_stop`/`lsp_restart`의 mutation guard 분리를 그대로 유지합니다. 실제 자식 프로세스가 `shutdown` 수신 후 빨리 종료하는 새 경계 테스트를 추가하고 현행 LSP 기능 문서의 소유 표기를 갱신했습니다.
- [x] C. 권한 허용 `taide-lsp` 단위 50건·새 경계 4건·Tauri LSP 명령 18건·Phase 0 계약 7건과 crate/Tauri clippy·fmt·독립 crate rustdoc·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`는 불변입니다. 전체 workspace·TypeScript typecheck는 이번 slice 뒤 재실행하지 않았고 실제 GUI/서버 세션 실기는 미검증입니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `91d4b22`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 서른아홉 번째 slice — LSP 재시작 정책의 독립 crate 이전

- [x] A. Tauri `handle_process_exit`의 재시작 한도 3회·500ms 선형 backoff·30초 건강 판정은 Tauri 비의존 프로세스 정책이고 AppHandle mutation guard·재기동·상태 이벤트는 조립 책임입니다. 새 crate 정책 경계 테스트는 API 부재 E0432(exit 101)로 의도대로 실패했습니다. Rust [`Arc::ptr_eq`](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.ptr_eq)와 [`Duration`](https://doc.rust-lang.org/std/time/struct.Duration.html#method.from_millis) 공식 문서를 확인했습니다.
- [x] B. 재시작 한도·지연·건강 판정 시간과 프로세스 생존/현재 슬롯 동일성 확인을 `taide-lsp::process`로 이전했습니다. Tauri `types` 상수 경로는 facade로 유지하고 기존 실패·상태 이벤트와 재기동 순서를 변경하지 않았습니다. 기존 건강 판정 테스트 3건을 crate 소유로 옮기고 현행 LSP 기능 문서의 소유 표기를 갱신했습니다. 첫 fmt 파싱 오류는 `let ... else` 세미콜론 누락으로, 수정 후 통과했습니다.
- [x] C. 권한 허용 `taide-lsp` 단위 53건·새 경계 5건·Tauri LSP 명령 15건·Phase 0 계약 7건과 crate/Tauri clippy·fmt·독립 crate rustdoc·`git diff --check`가 통과했습니다. 생성 bindings SHA-256 `2c7b4343878cacea8351733fb04435c6ab0519db15c84dc1dd4d6a4d69d06bda`는 불변입니다. 전체 workspace·TypeScript typecheck는 이번 slice 뒤 재실행하지 않았고 실제 GUI/서버 세션 실기는 미검증입니다. M5 전체·M6 adapter·native UI는 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `725d211`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 실행하지 않습니다. M5 전체는 미완료입니다.

## M5 종료 게이트 — 도메인 결합 절단과 독립 crate 경계

- [x] A. `domain_boundaries.rs`의 도메인 간 실행 참조 허용 목록은 비어 있고 infra→domain 참조·우회 import를 거부합니다. remote 명령 dispatch는 `lib.rs::remote_dispatch_port`가 조립하며 `remote/ws.rs`는 포트만 사용합니다. LSP·terminal·IDE·remote·window의 Tauri `service.rs`는 독립 crate facade이고 manifest·login page·lockfile도 같은 경계로 이전됐습니다. 다섯 crate의 전체 normal `cargo tree`에 Tauri 패키지가 없으며 소스의 Tauri import도 0건입니다.
- [x] B. 권한 허용 `cargo test --workspace --quiet` 전체(exit 0)·`cargo clippy --workspace --all-targets -- -D warnings`·`cargo fmt --all --check`·`git diff --check`가 통과했습니다. 테스트에는 도메인 경계 3건·Phase 0 계약 7건과 LSP/terminal/IDE/remote/window의 세션·보안·자원 수명주기 검사가 포함됩니다. 전체 실행이 갱신한 `bindings.ts`는 `lsp_stop` 공개 설명 주석만 달라졌고 IPC 시그니처는 불변입니다. manifest SHA-256을 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`로 동기화한 뒤 Phase 0 7건과 bindings Prettier 검사를 다시 통과시켰습니다. TypeScript typecheck는 주석만 변경돼 재실행하지 않았습니다.
- [x] C. M5의 코드·결합·테스트 계약이 충족되어 M5만 완료합니다. 실제 GUI·외부 LSP/PTY/MCP/원격 연결 실기, M6 adapter 분리, M7 전체 기능·데이터 동등성 및 M8 native UI gate는 미완료입니다. 원격 push는 기존 목적지·payload 승인 거절 때문에 사용자 승인 전까지 재시도하지 않습니다.
- [x] D. 생성 bindings·manifest 동기화를 `12c4341`로 선별 로컬 commit했습니다. 이 M5 종료 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 일반 push는 사용자 승인 전까지 재시도하지 않습니다.

## M6 첫 slice — 웹뷰 navigation guard의 platform adapter 이전

- [x] A. `infra/navigation_guard.rs`는 Tauri 웹뷰 builder의 navigation/new-window 콜백과 공유 외부 URL 검증기를 사용합니다. 메인·보조 창이 동일 정책을 부착하며 기존 URL 허용/거부 단위 테스트 5건이 있습니다. 새 `taide_lib::platform` 경계 테스트는 모듈 부재 E0432(exit 101)로 의도대로 실패했습니다. [Tauri WebviewWindowBuilder 공식 API](https://docs.rs/tauri/latest/x86_64-apple-darwin/tauri/webview/struct.WebviewWindowBuilder.html)를 확인했습니다.
- [x] B. 구현과 단위 테스트를 `src-tauri/src/platform/navigation_guard.rs`로 옮기고 이전 `infra/navigation_guard.rs` 경로는 재수출 facade로 유지했습니다. 메인 창 `create_main_window`와 보조 창 `open_auxiliary_window`는 platform 모듈을 직접 호출합니다. 기존 스킴·호스트·dev 오리진 허용 목록과 `window.open()` 외부 URL 검증/거부 동작은 바꾸지 않고 현행 아키텍처 문서의 소유 경로를 갱신했습니다.
- [x] C. 새 platform/기존 facade·두 창 부착 경계 2건, navigation 정책 5건·window 명령 6건·도메인 경계 3건·Phase 0 계약 7건과 Tauri all-target clippy·fmt·`git diff --check`가 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 웹뷰/OS 브라우저 GUI 실기는 미검증입니다. M6의 AppServices·EventSink·WindowRegistry·TaskSupervisor 및 asset adapter, M7/M8은 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `77553f9`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기고 원격 push는 사용자 승인 전까지 실행하지 않습니다. M6 전체는 미완료입니다.

## M6 두 번째 slice — asset URI 프로토콜의 platform adapter 이전

- [x] A. 기존 `infra/asset_protocol.rs`는 열린 프로젝트 집합으로 경로를 인가하고 공유 range 처리기로 바이트·CSP·캐시 헤더를 구성하며 `lib.rs`가 Tauri asset URI scheme을 등록합니다. 새 platform 공개 경계 테스트 2건을 추가한 뒤 구현 전 E0432(exit 101)를 확인했고 [Tauri Builder 공식 API](https://docs.rs/tauri/latest/x86_64-apple-darwin/tauri/struct.Builder.html)의 URI scheme 등록 계약을 확인했습니다.
- [x] B. 구현과 기존 단위 테스트를 `src-tauri/src/platform/asset_protocol.rs`로 옮기고 이전 `infra/asset_protocol.rs` 경로는 재수출 facade로 유지했습니다. `lib.rs` 등록 클로저는 platform 응답 함수를 직접 호출하며 인가·range·CSP·MIME·캐시 정책은 바꾸지 않았습니다. 현행 아키텍처 문서의 소유 경로를 갱신했습니다.
- [x] C. 새 platform/기존 facade·등록 경계 2건, 기존 asset 정책 10건·도메인 경계 3건·Phase 0 계약 7건과 Tauri all-target clippy·fmt·`git diff --check`가 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. Phase 0 테스트 대상 이름을 처음 잘못 지정했으나 실제 대상 `rust_native_phase0_contract`로 7건을 확인했습니다. 전체 workspace·TypeScript typecheck와 실제 미디어 webview GUI 실기는 미검증이며 M6 runtime/DI·M7/M8도 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `545e890`으로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 세 번째 slice — 보조 창 WindowRegistry 경계 이전

- [x] A. 기존 `domain/window/commands.rs::WindowStore`는 보조 창 label에서 project/slot을 찾고 역조회·멱등 해제를 수행합니다. `lib.rs`가 Tauri 관리 상태로 등록하고 창 명령이 생성·닫힘·부팅 복원에 소비합니다. 새 platform 공개 경계 테스트 2건은 구현 전 모듈 부재 E0432(exit 101)로 실패했습니다. [Tauri 상태 관리](https://v2.tauri.app/develop/state-management/)와 [parking_lot Mutex](https://docs.rs/parking_lot/latest/parking_lot/type.Mutex.html)의 공식 계약을 확인했습니다.
- [x] B. 매핑 구조와 기존 단위 테스트 6건을 `platform/window_registry.rs`로 옮겼습니다. 기존 `domain::window::commands::WindowStore`는 같은 타입의 재수출 facade이며 `lib.rs`와 창 명령은 `WindowRegistry`를 직접 사용합니다. 창 생성·flush·탭 복귀·복원 순서는 변경하지 않았고 현행 아키텍처 문서의 소유 경로를 갱신했습니다.
- [x] C. 새 platform/facade·앱 배선 2건, 기존 registry 정책 6건·창 수명주기 순서 2건·도메인 경계 3건·Phase 0 계약 7건과 Tauri all-target clippy·fmt·`git diff --check`가 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 다중창 GUI 실기는 미검증이며 M6 EventSink/AppServices/TaskSupervisor·M7/M8도 미완료입니다.
- [x] D. 코드·테스트·현행 문서를 `87fc1c5`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 네 번째 slice — LayoutChanged의 EventSink 첫 경계

- [x] A. `events.rs`의 Tauri 이벤트 30개는 `lib.rs::collect_events!`와 원격 `fanout_remote_events!`에 등록됩니다. `LayoutChanged` 발행은 `domain/layout/service.rs::finish_mutation`과 `lib.rs::plan_return_of_auxiliary_window_tabs` 두 곳입니다. model/runtime 공개 경계 테스트는 구현 전 `app_event` 모듈과 `taide-runtime` crate 부재 E0432(exit 101)로 실패했습니다. [Tauri 상태 관리](https://v2.tauri.app/develop/state-management/)와 [Rust trait object](https://doc.rust-lang.org/book/ch18-02-trait-objects.html) 공식 계약을 확인했습니다.
- [x] B. `taide-model::app_event::AppEvent`의 layout variant와 Tauri 미의존 `taide-runtime::EventSink`를 만들고, `platform::event_sink::TauriEventSink`가 기존 `LayoutChanged` 타입을 발행하게 했습니다. `finish_mutation`은 `&dyn EventSink`를 명시적으로 받으며 보조 창 탭 복귀는 동일 어댑터를 호출합니다. 처음 관리 상태에 `AppHandle`을 보관한 형태는 불필요한 장기 보유를 피하도록 발행 시점에 핸들을 빌리는 어댑터로 바로 수정했습니다. 기존 30개 등록·원격 `listen_any` fanout·IPC payload는 바꾸지 않았고, 새 crate의 normal dependency graph에 Tauri가 없습니다.
- [x] C. 새 경계 3건에는 기록용 sink를 주입한 `finish_mutation`의 revision·dirty 표시 검증이 포함됩니다. 권한 허용 Tauri lib 332건, Phase 0 IPC 계약 7건·도메인 경계 3건, model/runtime/Tauri all-target clippy·fmt·strict runtime rustdoc·`git diff --check`가 통과했습니다. 제한된 sandbox의 lib 6건은 기존 ps·로컬 소켓 권한 오류로 실패했지만 권한 허용 동일 테스트 332건은 모두 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. TypeScript 파일은 변경하지 않아 typecheck를 재실행하지 않았고 전체 workspace·실제 GUI/원격 세션 실기는 미검증입니다. 나머지 29개 이벤트와 M6 AppServices·TaskSupervisor, M7/M8은 미완료입니다.
- [x] D. 구현·테스트·현행 문서를 `7b82061`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 다섯 번째 slice — Git status/refs EventSink 경계

- [x] A. Git 명령의 두 발행 helper와 파일시스템 watcher 콜백은 모두 `GitStore::invalidate_status`를 이벤트 발행 전에 호출합니다. watcher가 둘 다 감지하면 status를 refs보다 먼저 발행하고, 기존 Tauri 이벤트를 로컬 cache 구독과 원격 fanout이 소비합니다. 새 두 variant·배선 테스트는 구현 전 `AppEvent` variant 부재 E0599(exit 101)로 실패했습니다.
- [x] B. model `AppEvent`에 Git status/refs 두 variant를 추가하고 Tauri platform adapter에서 기존 `GitStatusChanged`·`GitRefsChanged`로 변환했습니다. 명령 helper와 watcher 콜백은 빌린 AppHandle의 EventSink를 호출하며 cache 무효화, status→refs 순서, 기존 IPC payload·원격 구독·로컬 listener는 바꾸지 않았습니다. 현행 아키텍처 문서의 이전 상태를 3/30 이벤트로 갱신했습니다.
- [x] C. EventSink 5건·Git 집중 29건·Phase 0 IPC 계약 7건·도메인 경계 3건과 model/Tauri all-target clippy·fmt·`git diff --check`가 통과했습니다. 경계 테스트는 명령/워처 소스의 cache 무효화 선행과 Tauri adapter mapping도 검사합니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 Git watcher·원격 클라이언트·GUI 실기는 미검증이며 나머지 27개 이벤트와 M6 AppServices·TaskSupervisor, M7/M8도 미완료입니다.
- [x] D. 구현·테스트·현행 문서를 `af5f38c`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 여섯 번째 slice — terminal 세션 EventSink 경계

- [x] A. terminal spawned/exited/cwd/command-finished는 각각 세션 등록, 종료 metadata 갱신, 실제 cwd 변경, 측정된 command marker 뒤에 발행됩니다. 기존 이벤트는 `collect_events!`와 원격 fanout에 등록됩니다. 네 AppEvent variant와 adapter 배선 테스트는 구현 전 variant 부재 E0599(exit 101)로 실패했습니다.
- [x] B. model `AppEvent`에 terminal 네 variant를 추가하고 Tauri platform adapter에서 기존 TerminalSpawned/Exited/CwdChanged/CommandFinished로 변환했습니다. terminal 명령의 네 발행점은 빌린 AppHandle의 EventSink를 호출하며 상태 갱신 순서, 기존 IPC payload·원격 fanout·raw output channel은 바꾸지 않았습니다. 현행 문서의 이전 상태를 7/30 이벤트로 갱신했습니다.
- [x] C. EventSink 경계 7건·Tauri terminal 8건·Phase 0 IPC 계약 7건·도메인 경계 3건과 Tauri/model all-target clippy·fmt·`git diff --check`가 통과했습니다. 경계 테스트는 상태 갱신·명령 측정 선행과 adapter mapping도 검사합니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 PTY·원격·GUI 실기는 미검증이며 나머지 23개 이벤트와 M6 AppServices·TaskSupervisor, M7/M8도 미완료입니다.
- [x] D. 구현·테스트·현행 문서를 `28b924a`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 일곱 번째 slice — 설정·테마 EventSink 경계

- [x] A. `apply_and_broadcast`는 설정 저장·공유 상태 갱신·integration observer 완료 뒤 SettingsChanged를 발행하며, `settings_set_theme`은 그 경로가 끝난 뒤 ThemeChanged를 발행합니다. 기존 이벤트는 로컬 설정 리스너와 원격 fanout이 소비합니다. 두 AppEvent variant·adapter 배선 테스트는 구현 전 variant 부재 E0599(exit 101)로 실패했습니다.
- [x] B. model AppEvent에 설정·테마 두 variant를 추가하고 Tauri platform adapter에서 기존 이벤트 타입으로 변환했습니다. Settings 페이로드는 AppEvent 안에서만 Box로 보유해 enum 크기 차이를 줄이고 IPC payload·bindings는 그대로 유지합니다. [Rust Clippy의 enum variant 크기 계약](https://doc.rust-lang.org/clippy/lint_configuration.html#enum-variant-size-threshold)을 확인했습니다. 발행 순서·원격 fanout·구독 경로는 변경하지 않았고 아키텍처 문서의 이전 상태를 9/30 이벤트로 갱신했습니다.
- [x] C. EventSink 경계 9건·taide-settings 정책 69건·Tauri settings 1건·Phase 0 IPC 계약 7건·도메인 경계 3건과 Tauri/model all-target clippy·fmt·`git diff --check`가 통과했습니다. 첫 clippy는 큰 Settings variant 때문에 실패했고 Box 적용 후 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 원격·GUI 실기는 미검증이며 나머지 21개 이벤트와 M6 AppServices·TaskSupervisor, M7/M8도 미완료입니다.
- [x] D. 구현·테스트·현행 문서를 `9a782e2`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 여덟 번째 slice — 동기화 상태 EventSink 경계

- [x] A. SyncStateChanged는 connect/disconnect/upload/download의 성공 경로 네 곳에서 설정·테마·로케일 등 각 경로의 상태 반영을 마친 뒤 발행됩니다. 기존 Tauri 이벤트는 `collect_events!`와 원격 fanout에 등록됩니다. model variant·adapter 배선 테스트는 구현 전 variant 부재 E0599(exit 101)로 실패했습니다.
- [x] B. model AppEvent에 SyncStatus variant를 추가하고 Tauri platform adapter에서 기존 SyncStateChanged로 변환했습니다. 네 발행점은 빌린 AppHandle의 EventSink를 호출하며 기존 IPC payload·원격 fanout·성공 경로 정책은 변경하지 않았습니다. 현행 아키텍처 문서의 이전 상태를 10/30 이벤트로 갱신했습니다.
- [x] C. EventSink 경계 11건·권한 허용 Tauri sync 16건·Phase 0 IPC 계약 7건·도메인 경계 3건과 Tauri/model all-target clippy·fmt·`git diff --check`가 통과했습니다. 제한된 sandbox의 sync lib 2건은 로컬 소켓 권한 오류였고 권한 허용 동일 16건은 모두 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 GitHub·원격·GUI 실기는 미검증이며 나머지 20개 이벤트와 M6 AppServices·TaskSupervisor, M7/M8도 미완료입니다.
- [x] D. 구현·테스트·현행 문서를 `e2db6d6`으로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.

## M6 아홉 번째 slice — 원격 서버 상태 EventSink 경계

- [x] A. RemoteStateChanged는 서버 시작에서 RemoteStore 등록 뒤, 중지에서 shutdown 신호 뒤 기본 상태로 발행됩니다. 기존 이벤트는 `collect_events!`와 원격 fanout에 등록됩니다. model variant·adapter 배선 테스트는 구현 전 variant 부재 E0599(exit 101)로 실패했습니다.
- [x] B. model AppEvent에 RemoteStatus variant를 추가하고 Tauri platform adapter에서 기존 RemoteStateChanged로 변환했습니다. 시작·중지 발행점은 빌린 AppHandle의 EventSink를 호출하며 기존 IPC payload·원격 fanout·수명주기 순서는 변경하지 않았습니다. 현행 아키텍처 문서의 이전 상태를 11/30 이벤트로 갱신했습니다.
- [x] C. EventSink 경계 13건·Tauri remote 51건·Phase 0 IPC 계약 7건·도메인 경계 3건과 Tauri/model all-target clippy·fmt·`git diff --check`가 통과했습니다. bindings SHA-256 `1b30c770469188b6a568bdde7e6479cd1f0c43934cb8bb577562d5cc4060eebf`는 불변입니다. 전체 workspace·TypeScript typecheck와 실제 서버·원격 클라이언트·GUI 실기는 미검증이며 나머지 19개 이벤트와 M6 AppServices·TaskSupervisor, M7/M8도 미완료입니다.
- [x] D. 구현·테스트·현행 문서를 `56479e5`로 선별 로컬 commit했습니다. 이 검증 기록과 PROCESS 상태는 별도 로컬 문서 commit으로 남기며 원격 push는 기존 승인 거절로 재시도하지 않습니다.
