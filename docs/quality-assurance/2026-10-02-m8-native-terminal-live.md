# M8 native terminal live sync·resize·Frame publication

## 대상·상태

`native/taide-native-terminal/src/{lib,session}.rs`, `tests/session.rs`와 합성 helper, `native/taide-native-app/src/{terminal_host,terminal_dispatch,host,application}.rs`, `tests/{terminal-host,terminal-dispatch}.rs`입니다. 실제 vte sync deadline을 root actor에 연결하고, 출력·sync flush·resize·최종 Frame을 같은 publication 순서로 보존했습니다. typed ResizeTerminal과 supervisor-owned blocking PTY resize·Core resize/effects를 연결합니다. 실제 renderer의 caller/geometry/palette·UI 입력은 아직 미완료이며 상위 N4-B/N4/M8를 완료 처리하지 않습니다.

## 소유·순서

- actual vte StdSyncHandler의 Instant를 Core에서 읽습니다. actor는 deadline이 있을 때만 sleep_until을 등록하고 accepted Frame/finish와 함께 select합니다. timer가 깨어도 같은 Core를 다시 확인해 취소/연장/종료된 sync를 중복 flush하지 않습니다. timer가 실제 flush한 Outcome도 원본의 같은 retained admission/Frame/Dispatcher/writer 경로를 거칩니다.
- SharedTerminal의 별도 publication mutex는 실제 read callback의 parse→Frame 제출, timer flush, resize와 finish를 직렬화합니다. Core 상태 mutex를 callback 전체에 유지하지 않으므로 borrowed snapshot과 callback panic의 상태 회수가 가능합니다. finish는 실제 PTY workers join 뒤 publication을 확보합니다. 공개 bare advance의 Frame을 임의로 늦게 제출하는 외부 caller까지 보장한다고 확대하지 않으며 실제 app read owner는 private ordered path만 사용합니다.
- resize는 Running/산술 admission·같은 size 여부를 먼저 확인합니다. 유효한 변경에서만 revision/Core resize/retained Outcome·실제 root store/master resize·bounded Frame submit을 진행합니다. 잘못된 요청·동일 size·종료 뒤 요청은 Core를 retire하지 않습니다. Core parser/retained 실패·실제 resize 실패·delivery 거절/unwind는 첫 실패를 보존하고 weak PTY stop에 연결합니다. wrapper/publication/guard의 추가 소유 비용은 aggregate memory gate에 남아 있습니다.
- Session resize는 TaskSupervisor의 actual blocking 작업에 Session/admission을 보유합니다. 요청 future 취소가 시작한 OS 작업을 버리지 않습니다. Frame Sender를 같은 Session이 보유하며 resize callback이 다른 queue나 새 parser를 만들지 않습니다.
- 모든 정상 consume 완료에 updated pulse를 추가했습니다. 일반 text처럼 stream/UI effect가 없어도 renderer가 repaint를 요청할 수 있습니다. 실제 Egui pulse/palette/geometry 제공은 다음 surface 단계이며 이번 합성 port 성공을 실제 GUI paint로 주장하지 않습니다. borrowed Core의 GridDimensions/Line/Column 경계를 공개합니다.

## 실제 검증

Cargo는 직렬·locked/offline·공유 target입니다. 신규 helper flag `TAIDE_NATIVE_FIXTURE_LIVE_SYNC=1`에서 ready→sync 입력→끝내지 않은 sync text/title→계속 stdin 대기→continue/기존 종료 순서를 사용합니다. 기본 helper 출력은 유지하며 보호 bundle·OS 설정·사용자 파일/clipboard/keychain은 변경하지 않았습니다.

1. [x] app 신규 actual live 검사만 실행: `cargo test --manifest-path native/taide-native-app/Cargo.toml … --test terminal-host live_sync_deadline -- --nocapture`: 1 PASS(compile 10.66초/suite 0.48초). 종료하지 않은 실제 child의 deadline title/text 반영·Running·deadline 해제, 같은 Core의 실제 root PTY resize 호출 성공·96×32 grid, 동일 size false/잘못된 size 오류에서 계속 Running, 최종 Dispatcher success/updated pulse·종료 후 resize 거절과 grid 보존·actual join/tracked 0을 확인했습니다. child 내부 stty size/SIGWINCH·실제 픽셀 geometry는 이 검사에서 직접 계측하지 않았습니다.
2. [x] native 신규 publication unit만 실행: `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --lib 출력_publication -- --nocapture`: 1 PASS(compile 9.31초/suite 0.00초). 첫 출력 callback을 채널로 실제 보류한 상태에서 publication lock 보유/Core state lock 해제·revision 1 snapshot을 확인하고 경쟁 resize 제출 순서 `[1, 2]`를 확인했습니다. resize OS port는 합성이며 실제 OS resize는 위 app 검사 근거입니다. 실패 시 callback release RAII와 3초 channel deadline을 유지합니다.
3. [x] native `cargo test … --test session callback_ -- --nocapture`: 2 PASS(compile 1.87초/suite 0.34초). 신규 deterministic deadline의 직전/만료/중복 flush·typed title/text·resize port 실패/retire/첫 실패 보존과, publication 변경의 영향을 받는 기존 actual callback panic/foreign completion/회수 1건을 확인했습니다. 출력의 synthetic delivery panic은 의도한 worker panic이며 검사 실패가 아닙니다. 기존 정상 Session/Hub/writer/Frame/consumer runtime은 반복하지 않았습니다.
4. [x] native `cargo clippy … --lib --test session --bin native-session-fixture -- -D warnings`: exit 0(1.37초). app `cargo clippy … --lib --test terminal-host --test terminal-dispatch --bin native-terminal-queue-fixture -- -D warnings`: exit 0(2.67초). dependency Wry의 기존 17 warnings는 유지됩니다. app check exit 0(2.14초)·exact authored rustfmt·git diff --check exit 0입니다.

## 다음 gate

실제 terminal placeholder→measured attach/resize·palette/query pixels·borrowed glyph/color/cursor renderer, per-view scroll/selection/search/link·IME/마우스·pending input/retry/종료 UI·project close Hub 정리·원본 full agent/remote/IDE/CLI, default/큰 history·visitor CPU·총 memory/peak/RSS·GUI/AX는 계속 미완료입니다. ordered callback은 nonblocking bounded queue 제출용이며 publication API로 재진입하거나 무한 대기하는 caller까지 보장하지 않습니다. sync 버퍼 안 text/agent의 관찰 시각은 실제 flush 경계이므로 기존 raw scanner의 입력 시각과 완전한 동등성을 이 기본 검사로 주장하지 않습니다. 전체 N1~N8 0/8·M8 전체 완료 뒤만 commit/push 조건을 유지합니다.
