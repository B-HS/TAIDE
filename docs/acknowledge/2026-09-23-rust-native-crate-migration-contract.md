# Rust-native 전체 기능 crate 분리 실행 계약

> 브랜치: `to_rust_native`
> 상태: M1·M2·M3 완료, M4 font부터 AI까지 15개 서비스 slice 검증 완료; Git 서비스는 M4, Tauri platform adapter는 M6 소유, M4~M8 미완료
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
