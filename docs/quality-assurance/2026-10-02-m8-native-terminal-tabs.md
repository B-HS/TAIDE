# M8 native terminal 탭·host·환경 기본 연결

## 대상·상태

`native/taide-native-app/src/{terminal_tabs,terminal_environment,terminal_host,host,application,lib}.rs`, `tests/{terminal-host,terminal-environment}.rs`, `crates/taide-runtime/src/{terminal_env,lib}.rs`입니다. 실제 NewTerminal 의도→bounded host→원본 layout mutation, 앱의 단일 Tabs/Hub→typed attach/close→root PTY/actor join과 환경 주입을 연결했습니다. 실제 terminal surface는 아직 placeholder이고 AttachTerminal의 renderer caller·reply state·measured geometry·palette는 다음 단계입니다. 전체 N4-B/N4/M8 완료가 아닙니다.

## 원본·수명

- 원본 `useOpenTerminalTab`처럼 NewTerminal은 빈 session ID의 독립 non-preview tab을 만듭니다. 이름은 실제 locale의 `terminal.title`입니다. start는 별도 typed AttachTerminal이며 caller가 측정 크기와 actual effect ports를 제공합니다. 탭/aux tree를 실제 layout 전체에서 찾고 원본 pty_default_options로 cwd·shell을 해석합니다.
- 한 Tabs 안의 attach는 직렬화합니다. persisted ID가 같은 Hub에 있으면 같은 Session/Core를 반환하고 env/spawn을 다시 실행하지 않습니다. project 소유와 실패를 확인합니다. settings의 실제 history 및 원본 512B/line 추정을 raw replay 요청에 전달하고 원본 Rust clamp를 유지합니다. history를 작게 고정하거나 raw replay를 native grid로 재파싱하지 않습니다.
- spawn 뒤 PendingSession RAII가 Hub entry를 소유합니다. 탭 부재/다른 kind·ID/다른 project/닫힌 project/shutdown·settle future 취소 시 entry 제거→기존 root PTY stop/remove를 수행하고 actor는 TaskSupervisor 아래에서 회수됩니다. 정상 settle은 owned mutation 아래 원본 set_terminal_session/finish_mutation을 사용합니다. 탭 전환은 제거하지 않습니다.
- NativeApplication이 단일 Arc<Tabs>를 소유하고 최초 bridge와 close 실패 후 재연결에도 같은 instance를 전달합니다. terminal close는 원본 탭 정책/PTY kill 이후 native actor의 실제 join까지 기다립니다. 기존 non-terminal host·clipboard API는 유지하며 없는 terminal host는 명시적 오류입니다.
- root terminal_env는 원본의 설정 snapshot·IDE status/readiness(2,000ms/50ms)→SSE port→기존 agent editor builder→protocol/version 순서를 유지합니다. CLI 소유 판정은 원본 helper의 실제 file-name 정책이며 새 서명/신뢰 검증을 추가했다고 주장하지 않습니다. 설치 대상·sidecar 해결은 감독된 blocking worker에서만 수행합니다. native provider는 격리 패키지의 실제 버전 0.1.0을 전달합니다. 제품 release version 통합은 N8이며 0.3.0인 척하지 않습니다. 기존 Tauri provider/API는 바꾸지 않았습니다.
- 앱 초기 격리 limit은 session 64, Frame queued+inflight 4MiB/64·visit 1,000,000, writer 1MiB/64·기존 Core default입니다. 제품 parity/tuning·전체 합산/peak/RSS를 이 기본값으로 완료 처리하지 않습니다. native IDE 서버 자체의 기동·CLI round-trip·agent discovery는 이 env port 성공과 별개의 N5 연결입니다.

## 실제 검증

Cargo는 직렬·`--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 성공한 이전 Hub/Core/writer/Frame/Dispatcher runtime은 반복하지 않았습니다. synthetic helper·memory layout/AppServices와 UUID tmp fixture만 사용하며 보호 `.app`·사용자 데이터·실제 IDE listener/CLI/clipboard/keychain은 호출하지 않았습니다.

1. [x] 새 탭/attach 2건만 실행: `cargo test --manifest-path native/taide-native-app/Cargo.toml … --test terminal-host native_attach -- --nocapture`: 2 PASS(compile 9.57초/suite 0.28초). 실제 NewTerminal 두 탭의 ID/title/non-preview·빈 ID, 실제 PTY 생성·같은 Core/Session 재사용·원본 탭 close 후 actual join, env-await 중 닫힌 탭의 실제 spawn event 뒤 native/root entry 회수·tracked 0입니다. test 함수의 대문자 snake_case warnings 2개는 rename으로 수정했고 같은 runtime은 반복하지 않았습니다. 이 시점 strict exit 0(2.88초)은 이후 host ports 변경 전 근거입니다.
2. [x] 실제 typed host 왕복 1건: `cargo test … --test terminal-host 실제_host_attach -- --nocapture`: 1 PASS(compile 11.25초/suite 0.37초). 원본 IdeStore에 synthetic running status만 주입했고 서버를 bind하지 않았습니다. SSE/editor/visual/protocol/version의 정확한 벡터·순서, 실제 native PTY attach/layout settle/close reply와 retained view의 완료·Hub/root entry 제거·actor join/tracked 0을 확인했습니다. clipboard port는 호출 시 실패하도록 주입했고 호출되지 않았습니다. UI gesture/프로세스 내부 env 출력·실제 agent 연결까지 검증했다고 확대하지 않습니다.
3. [x] synthetic CLI/path 환경 1건: `cargo test … --test terminal-environment -- --nocapture`: 1 PASS(compile 1.25초/suite 0.00초). 소유된 alias·타사 alias/개발 executable·dangling alias와 bundled sidecar fallback, 비활성 IDE/공백 editor 경로에서 editor/SSE를 생략하고 protocol/version만 유지합니다. synthetic 비실행 파일/링크를 UUID tmp 안에 만들고 해당 fixture만 회수했습니다.
4. [x] 최종 app `cargo clippy … --lib --test terminal-host --test terminal-environment -- -D warnings`: exit 0(2.48초). root `cargo clippy -p taide-runtime --lib … -- -D warnings`: exit 0(4.06초). dependency Wry의 기존 17 warnings는 유지됩니다. root env 최초 Rust 2021 let-chain compile/fmt 오류를 edition 변경 없이 short-circuit is_ok_and로 수정했고 app check exit 0(3.00초)입니다. 새 package·dependency/lock·MSRV 변경은 없습니다.
5. [x] exact authored native/root rustfmt·`git diff --check`: exit 0.

## 남은 gate

terminal surface의 borrowed glyph/color/cursor·가시 화면, actual palette/pixel query·geometry resize와 live sync deadline, 입력/IME/마우스/선택/검색/link·다중 view별 상태, 종료/오류 화면과 retry·pending input, project close의 Hub registry 정리·non-tab 실행/remote/agent·제품 limits/큰 history/전체 memory/perf·OS/AX는 별도 미완료입니다. settle 전 cwd 재지정·중간 blocking factory 취소·서버 readiness 실제 listener/CLI는 현재 runtime에서 직접 검증하지 않았으며 해당 연결 변경 때 검증합니다. N1~N8 0/8이며 전체 M8 완료 뒤만 commit/push합니다.
