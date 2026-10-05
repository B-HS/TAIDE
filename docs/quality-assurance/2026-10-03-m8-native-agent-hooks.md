# M8 native agent hooks HTTP·설정 반응 포트

## 대상 파일

- `native/taide-native-app/src/{agent-hooks,agent-hooks-tests,lib}.rs`
- `native/taide-native-app/{Cargo.toml,Cargo.lock}`
- 원본 계약: `src-tauri/src/domain/agent/hooks.rs`, `src-tauri/src/lib.rs`의 settings_toggle_observers
- 재사용 코어: `crates/taide-runtime/src/{agent_hook_reconcile,agent_hook_server,agent_probe,agent_actions}.rs`

## 리포트

Tauri AppHandle 없이 실제 loopback HTTP 서버를 시작·중지하고 기존 hook payload/설정 toggle 재조정 경로를 실행하는 native 포트를 구현했습니다. 합성 프로젝트/home에서 실제 소켓·인증·상태 이벤트와 설치된 파일만 갱신하는2개 검사를 통과했습니다. NativeApplication startup/전체 Settings apply·IDE→hooks→remote 순서에는 아직 조립하지 않았으며 M8 전체 완료가 아닙니다.

## 상세

1. ensure_started는 기존 start_hooks_server의 cached/중복 시작·lease·등록 실패·shutdown·store 정책을 재사용합니다. `127.0.0.1:0`에만 바인딩하고 원본처럼 UUID v4 simple 형식의32자 토큰을 발급합니다. accept/connection은 기존 TaskSupervisor의 transient worker이며 stop은 기존 store.take_server/override clear/accept abort를 사용합니다. 이미 받아 둔 연결은 원본처럼 읽은 뒤 현재 store 정보를 검사하고503을 받습니다.
2. path/query·constant_time_eq·HookPayload 파싱·기본 Claude/명시 agent 선택과 상태 override/diff/event 발행은 원본 계약 및 기존 runtime apply_hook_payload를 사용합니다. 미인증 경로는403, 잘못된 JSON은400, 수락된 payload는200입니다. 알 수 없는 agent/event가200이지만 상태를 바꾸지 않는 원본 정책을 임의로 새401/400 계약으로 바꾸지 않습니다.
3. apply_toggle/reconcile_installed는 기존 lazy home/emitter/start_server 포트를 실제 native 서버에 연결합니다. emitter는 원본처럼 OnceLock과3초 `claude --version` supervised probe를 사용합니다. 검사에는 설치된 Claude project hook이 없어 이 OS probe가 호출되지 않았습니다. production home resolution 함수는 연결됐지만 테스트는 private apply_toggle_with_home에 합성 home만 제공합니다. 실제 사용자 hook·home·키 파일을 읽거나 쓰지 않았습니다.
4. 기존 CLI target 경로를 그대로 사용합니다. 새 CLI 설치/소유 변경은 없습니다. 설치되지 않은 hook을 새로 만들지 않으며 원본 runtime이 관리 표시가 있는 JSON과 owned 파일만 재조정합니다. 합성 Gemini hook URL/command 갱신 및 비소유 Pi 파일 보존을 확인했습니다.
5. taide-agent는 runtime의 기존 transitive 로컬 crate이며 constants/service/store 계약을 직접 재사용하려고 native app에 edge를 추가했습니다. UUID는 기존 lock의 v4 package를 원본 인증 토큰 방식에 직접 사용합니다. offline metadata로 native lock의 app dependency 목록을 갱신했고 새 package/version·root lock/MSRV를 이 작업에서 변경하지 않았습니다.

Tokio의 [TcpListener 공식 API](https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html), [AsyncReadExt](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html), [JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)를 확인했습니다. bind port0/local_addr·stream read/write·abort 수명은 고정된 실제 프로젝트 코드와 함께 확인하고 사용했습니다.

## 실제 최소 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다.

- `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib native_agent_hooks -- --nocapture`: 최초 fixture의 build_hook_url 인자/command builder 이름이 실제 API와 달라 E0061/E0425 compile exit101이었습니다. 실제 `build_hook_url(&HooksServerInfo, agent)`와 `build_command_hook_shell_command`로 정정했습니다. 제품 API를 바꾸거나 검사를 억제하지 않았습니다.
- 같은 검사의 정정 실행은 compile8.96초·suite0.05초, sandbox의 소켓 bind `Operation not permitted (os error1)`로2 FAIL입니다. fake listener나 성공 placeholder를 만들지 않았습니다. 정확히 같은 두 검사만 socket bind/connect가 허용되는 require_escalated 실행으로 검증해 compile0.21초·suite0.03초·2 PASS/exit0입니다. 변경은 합성 UUID 임시 home/project와 loopback socket뿐이며 자기 fixture를 정리합니다.
- HTTP 검사1: cached server/token 형식, path·누락/wrong token403, invalid JSON400, default Claude와 Gemini 상태·한 번 event/중복 event 없음, stop의 override clear/이미 받아 둔 연결503·재시작 old token403, 감독자 폐쇄와 AppState shutdown 시작 거절, 최종 tracked_count0을 확인했습니다.
- toggle 검사1: unchanged lazy home 미호출, 합성 설치 Gemini URL/CLI command·사용자 JSON 보존, 비소유 Pi 파일 유지·새 Claude project hook 미생성, 실제 새 서버200, unchanged cached server와 disable uninstall/stop·tracked_count0을 확인했습니다.
- 당시 `cargo clippy --lib --bins --tests -- -D warnings` exit0,13.65초입니다. 이후 IDE dispatcher 추가 영향의 최종 static 결과는 [IDE QA](2026-10-03-m8-native-ide-tools.md)를 따릅니다. 기존 Wry17개 dependency warning과 authored strict를 구분하며 suppression은 없습니다.
- authored3 exact Rustfmt edition2024 및 tracked diff whitespace 검사 exit0입니다. 최종 IDE 추가 뒤 authored5 exactfmt와 QA/상태 문서5개의 Prettier 포맷/검사, native source/manifest/lock·QA의 no-index whitespace를 확인했습니다. 기존 AppFile7건/ThemeEditor11건·runtime/server/reconcile 코어 성공은 재실행하지 않았습니다.

## 미완료 게이트

- [x] 실제 native loopback start/stop·supervised 연결·기존 인증/이벤트 경로
- [x] 실제 toggle/기존 설치 재조정과 합성 home/project2검사
- [ ] NativeApplication startup/exit·전체 Settings apply의 IDE→hooks→remote awaited 순서 및 실패/이벤트 조립
- [ ] 실제 설치된 Claude emitter probe·CLI 설치/실사용 hook·GUI/aux·재시작/동시 startup 전체 통합
- [ ] 요청/연결 폭주·aggregate/peak 메모리·원본 HTTP framing의 전체 보안 경계

원본 parser를 그대로 재현했으므로 Content-Length는 clamp되며 method·duplicate header·truncated body·query decoding의 엄격한 새 HTTP 정책을 구현한 것이 아닙니다. header와 body의 합산 상한 및 동시 연결 admission도 완료로 주장하지 않습니다. 재현 조건은 크거나 분할된/반복된 loopback 요청과 연결 폭주이고, 생략 이유는 기존 계약 재현의 최소2검사 경계를 넘어선 전체 보안/메모리 gate이기 때문입니다. 위험은 부분 수락·초과 메모리/동시 worker이며 native 서버의 최종 활성화/cutover 전 해당 M8 보안 게이트에서 검증해야 합니다. 보호 bundle·실제 앱·OS·제품TS·Git은 변경하지 않았습니다.
