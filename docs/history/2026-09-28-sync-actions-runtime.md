# Sync application의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/sync_actions.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`
- `src-tauri/src/domain/sync/commands.rs`, `github.rs`, `tests/sync_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

Sync status/connect/disconnect/upload/download 5개의 application 정책을 runtime으로 이전했습니다. 같은 SecretStore·lazy gist factory·SettingsApplyPort callback·EventSink를 주입하며 공개 IPC, guard/오류 우선순위, 보호 설정과 파일 저장/이벤트 순서를 보존했습니다. 기존 native GistClient 요청·응답·인증 헤더·마스킹·페이지 검색과 SettingsApplyPort의 실제 integration callback은 유지합니다.

## 상세

1. status의 secret get 2회와 snapshot 순서를 유지합니다. 미연결이어도 tuple 평가의 두 번째 get 오류가 반환되는 기존 정책을 고정했습니다. 연결·gist가 모두 있어야 client/fetch를 호출하며 fetch 오류는 status snapshot으로 반환합니다.
2. connect는 blank 입력 gate→API client factory→previous gist snapshot→discover→guard→live gist 재검증→secret set→settings 저장/state→event 순서입니다. 같은 gist만 기존 last_synced_at을 유지하고 다른 gist/없음은 지웁니다. disconnect도 guard 아래 secret delete→저장/state→event이며 저장 실패 뒤 secret을 복구하지 않는 기존 정책을 보존합니다.
3. upload는 guard 아래 token/settings/theme/locale snapshot·같은 payload/pretty JSON을 만든 뒤 client를 취득합니다. 기존 gist update만 guard를 놓고 round-trip 후 재취득하며 최초 create는 await 동안 유지합니다. snapshot/live gist 불일치에서는 write-back/event를 생략하고, 일치하면 bookkeeping만 갱신해 live 설정을 보존합니다. connected는 같은 guard 아래 실제 주입 store의 상태입니다.
4. download는 token/gist snapshot→client/fetch→guard→gist 변경 retry→다른 sync 완료 retry→conflict→payload/schema→보호 설정 strip→settings apply await→theme/locale 적용→sync event 순서입니다. force는 conflict만 건너뛰며 retry를 무시하지 않습니다. 실제 settings apply callback은 이전의 AppHandle/State를 빌린 같은 포트를 호출합니다.
5. SyncGistPort는 기존 DownloadFileIo처럼 `impl Future + Send`를 반환하고 native adapter는 async method로 구현합니다. [Rust Reference의 return-position impl Trait](https://doc.rust-lang.org/reference/types/impl-trait.html#return-position-impl-trait-in-traits-and-trait-implementations)를 확인했습니다. runtime에 기존 workspace taide-sync와 설치된 serde_json의 직접 연결만 추가했습니다. Cargo.lock 변경은 runtime dependencies 2줄이며 package/version/install은 변경하지 않았습니다.

## 검증 기록

새 모듈/API 부재 E0432 RED(exit 101)를 확인했습니다. NoGist의 concrete Ready 반환 경고는 trait에 맞는 async method로 바꿨고, PartialEq 없는 SettingsPatch 비교는 직렬화된 Value로 대조했습니다. 새 HTTP adapter의 Default와 test callback type alias를 추가해 clippy 경고를 해결했습니다. 검사 억제나 제품 타입/동작 변경은 하지 않았습니다.

서로 다른 검사 49건의 성공을 사용합니다. 같은 입력의 성공은 재사용하며 재실행 건수를 합산하지 않습니다.

| 명령                                                                                                                                                                                   | 실제 결과                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `cargo test -p taide --test sync_actions_runtime`                                                                                                                                      | 새 memory/file/race/source 18건, exit 0                                |
| `cargo test -p taide --test sync_actions_runtime sync_commands는_같은_공유_secret_lazy_http_apply와_events를_주입한다 -- --exact`                                                      | native adapter Default 보완 뒤 영향 source 1건, exit 0; 중복 합산 제외 |
| `cargo test -p taide --lib domain::sync::commands::tests`                                                                                                                              | 실제 기존 decision/상태/메모리 secret 12건, exit 0                     |
| `cargo test -p taide --test taide_sync_extraction --test taide_model_sync --test settings_actions_runtime --test domain_boundaries`                                                    | service 1·model wire 2·settings apply 5·boundary 3건, exit 0           |
| `cargo test -p taide --lib tests::typescript_바인딩을_생성한다 -- --exact`                                                                                                             | 실제 생성 1건, exit 0                                                  |
| `cargo test -p taide --test rust_native_phase0_contract`                                                                                                                               | IPC contract 7건, exit 0                                               |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                           | exit 0                                                                 |
| `cargo clippy -p taide --lib --test sync_actions_runtime --test taide_sync_extraction --test taide_model_sync --test settings_actions_runtime --test domain_boundaries -- -D warnings` | 최종 exit 0                                                            |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                                      | exit 0                                                                 |
| `cargo tree -p taide-runtime --edges normal --prefix none`                                                                                                                             | 실제 551줄, Tauri package 0개, exit 0                                  |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                       | exit 0                                                                 |

Fixture는 자기 UUID AppPaths와 InMemorySecretStore를 사용하며 합성 값만 전달했습니다. 실제 keyring·credential·GitHub·HTTP listener·사용자 home/파일·프로세스·앱은 사용하지 않았습니다. 첫 upload를 create await에서 pending으로 만들고 두 번째 upload가 guard에서 기다리는 것을 확인했습니다. 첫 완료 뒤 두 번째는 같은 gist update를 호출해 create 1회·update 1회입니다. 저장 실패는 자기 settings.json 경로를 디렉터리로 만들어 재현했습니다. 실제 settings apply는 runtime 공통 apply를 자기 파일로 호출하고 SettingsChanged→SyncStateChanged와 theme/locale의 적용 전후를 확인했습니다. Drop은 자기 UUID 디렉터리만 정리합니다.

공개 command 5개의 signature/Rustdoc와 순수 helper 4개의 body는 byte 동일합니다. application 5개의 body는 named host-port 치환을 제외하고 공백 정규화 동일합니다. 기존 github 전체 구현/tests도 새 adapter/import를 제외하면 공백 정규화 동일합니다. 실제 binding 생성 뒤 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`·manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 등록/remote/wire·SettingsApplyPort·공유 state를 변경하지 않았습니다.

## 남은 경계

신규 history는 docs ignore를 해제한 Prettier로 실제 검사합니다. PROCESS/architecture 전체 포맷 실패는 직전 단위에서 변경 전 HEAD에도 동일한 exit 1로 확인한 baseline이며 무관한 전체 재포맷은 하지 않습니다.

5개 command entry를 P→F로 옮겨 정적 배치는 F189/S0/A13/P4입니다. 남은 application entry는 project 4개이며 전체 M6 완료 판정이 아닙니다. 실제 native HTTP·OS keyring·AppHandle integration observer 실기와 cancellation·guard admission·supervisor/root 회수·직접 Exit·강제 bounded 종료는 이번 단위에서 검증하지 않았습니다. 기존 secret 먼저 변경/partial local apply/후속 best-effort 파일 적용 정책을 보존하며 트랜잭션이나 rollback을 새로 보장하지 않습니다. locale 보안 결정과 M6/M7/M8는 미완료이며 전체 M6 완료 뒤 일반 push 조건을 유지합니다.
