# Rust-native 전체 기능 crate 분리 실행 계약

> 브랜치: `to_rust_native`
> 상태: M1 완료; M2 `AppPaths`·`FlushScope` 및 첫 저장 DTO 묶음 자동 검증 완료, 전체 M2 진행 중
> 상태 정본: `docs/PROCESS.md`의 「Rust-native 이전을 위한 전체 기능 crate 분리」
> 선행 근거: `docs/roadmap-rust-native.md`, `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`, `docs/architecture.md`

## 목표와 순서

현재 사용자 기능의 Rust 서비스와 자원 관리를 기능별 crate로 분리하고, 현행 Tauri 실행 경로·테스트를 보존한 다음 native UI를 시작합니다. 기존 `docs/acknowledge/2026-08-28-no-crate-split-decision.md`의 단일 소비자 전제는 이번 native 앱 추가 요청으로 바뀌었습니다. 전체 이동은 한 번에 하지 않습니다. 각 변경은 자체 테스트·기존 공개 API·wire 계약의 green 상태에서 끝납니다.

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

현재 제약: 사용자 지정 모델 `ollama-cloud/deepseek-v4.1-flash#max`는 모델 목록에서 확인했으나, 이번 재개에서도 `opencode run --agent explore-pen --model ollama-cloud/deepseek-v4.1-flash#max` 호출이 `Permission denied: shell`로 거부됐습니다. 모델·호출 방식을 변경하지 않고 메인이 첫 slice를 직접 수행했습니다. 앱 실행·재시작은 사용자 몫이므로 이번 자동 검증에서 다루지 않았습니다. E와 M2 전체는 후속 slice가 끝나기 전까지 미완료입니다.

## M2 두 번째 slice — `FlushScope` 경계 (진행 중)

- [x] A. `src-tauri/src/state.rs:19-44`의 `FlushScope`는 이미 model crate에 있는 `ProjectId`·serde·specta만 사용합니다. `events.rs:505-528`의 all/window/project 직렬화·왕복 테스트와 생성 bindings의 타입 설명까지 확인했습니다. 이벤트 구조체와 Tauri `Event` derive는 기존 adapter에 남기고 `state::FlushScope` 경로를 보존합니다.
- [x] B. 기존 `events::tests::플러시_스코프` 2개 green을 먼저 확인했습니다. `src-tauri/tests/taide_model_flush_scope.rs`에 model↔facade 타입 동일성·Hash·all/window/project 레거시 wire fixture를 추가했고, `cargo test -p taide --test taide_model_flush_scope`는 `taide_model::flush` 부재(E0432, exit 101)로 의도대로 실패했습니다.
- [x] C. enum과 기존 문서 속성을 바이트 동일하게 `crates/taide-model/src/flush.rs`로 옮기고 `state::FlushScope`에서 재수출했습니다. variant·serde rename·specta 설명을 바꾸지 않았습니다. 신규 타입·wire 2건, 기존 이벤트 wire 2건, Phase 0 IPC 7건이 통과했습니다.
- [x] D. 새 모델↔facade 경계 2건, 기존 event wire 2건, Phase 0 IPC 7건, Rust workspace 총 1,792개(taide lib 1,719·통합 40·CLI 17·model 16)가 통과했습니다. fmt는 신규 테스트의 긴 행 때문에 첫 검사에서 실패해 해당 행만 고쳤고 재검사는 통과했습니다. clippy `--workspace --all-targets -- -D warnings` exit 0; 생성 bindings SHA-256은 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49`로 불변이며 source-scan 계약도 통과했습니다. 앱 실행은 사용자 몫입니다.
- [x] E. 검증된 파일 6개만 선별 commit `1b17864`·현재 브랜치에 일반 push했습니다.

이번 재개에서도 지정 모델의 `opencode run --agent explore-pen --model ollama-cloud/deepseek-v4.1-flash#max` 호출이 `Permission denied: shell`로 거부됐습니다. 모델·호출 방식을 바꾸지 않고 메인이 직접 진행합니다.

## M2 세 번째 slice — 저장 데이터 DTO(theme·locale·snippet, 진행 중)

- [x] A. `src-tauri/src/domain/{theme,locale,snippet}/types.rs`에 저장 파일용 타입·기본값·직렬화 속성이 있고, 이 세 파일 사이 의존·Tauri import·원천 경로 고정 source-scan은 없습니다. `docs/data-model.md:75-79`의 `themes/`, `snippets/`, `locales/` 저장 구역에 대응합니다. 세 facade의 기존 공개 경로는 보존합니다.
- [x] B. `src-tauri/tests/taide_model_persistence.rs`에 세 도메인의 model↔facade 동일 타입과 구버전 언어팩·테마의 기본값, 스니펫의 단일/배열·선택 필드 wire fixture를 먼저 추가했습니다. `cargo test -p taide --test taide_model_persistence`는 새 model 모듈 부재 13건(E0433, exit 101)으로 의도한 red였습니다.
- [x] C. 세 파일을 바이트 동일하게 model crate로 옮겼습니다. 원본에 unit은 없었고, `src-tauri/src/domain/{theme,locale,snippet}/types.rs`는 기존 API 전체를 재수출합니다. 신규 타입·구버전 wire 3건과 model unit 16건이 통과했습니다.
- [x] D. `cargo test --workspace --quiet` exit 0, 총 1,795개(taide lib 1,719·통합 43·CLI 17·model 16)가 통과해 기존 도메인 서비스·source-scan·IPC 계약도 포함합니다. fmt는 신규 테스트 행 길이 5곳 수정 뒤 재검사 통과, clippy `--workspace --all-targets -- -D warnings` exit 0. bindings SHA-256 `092a866cf053f7ed81518d045ac3b332c42722dc542031e4c9bd46b3f7450e49` 불변이고 원천 코드·저장 경로에 동작 변경이 없습니다.
- [ ] E. 검증된 파일만 선별 commit·현재 브랜치에 일반 push합니다.
