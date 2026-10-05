# M8 actual TerminalCore의 live-mode 입력

## 대상·계약

`native/taide-native-terminal/src/{lib,input}.rs`, `tests/input.rs`, `experiments/terminal-core-spike/src/lib.rs`입니다. 기존 spike의 `src/input.rs`를 native shared source로 이동하고 원본 `AlacrittyProbe::encode_input`의 mode 취득만 pure 함수의 `TermMode` 인수로 추출했습니다. spike의 기존 API는 얇은 wrapper로 유지합니다. 입력 정책은 두 곳에 복사하지 않습니다. 원래 경로의 source는 새 native 경로에서 복구할 수 있습니다.

actual Core의 `encode_input`이 실제 현재 mode를 읽어 같은 인코더에 전달합니다. 퇴역한 Core에는 `InputError::Retired`를 반환하고 preedit·확정 text도 전송하지 않습니다. 출력 parser/mode를 별도로 복제하지 않습니다. 기존 navigation/기능·modifier·Shift+Enter·paste의 CRLF/LF 정규화/bracketed decoration·focus reporting·UTF8/확정 text·caller byte quota 정책을 유지합니다. 아직 key encoding은 물리 키/OS/IME adapter가 아니고 PTY writer를 호출하지 않습니다.

메인이 workflow·서브에이전트 없이 수행했습니다. 외부 dependency·lock·root/MSRV·native app·제품 PTY/scanner·사용자 실기 bundle/OS 설정은 이 입력 이동에서 변경하지 않았습니다. 실제 clipboard/input기·shell/PTY·GUI를 실행하지 않았습니다.

## 최소 검증

Cargo는 직렬, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

1. [x] `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test input -- --nocapture`: 1 PASS, compile 0.71초/suite 0.00초. actual Core의 APP_CURSOR set/reset·수정 키·Shift+Enter·preedit 비전송·CJK/NFD/supplementary 확정 text·정확한 byte 경계·bracketed paste set/reset·CRLF/LF/CR 보존·focus set/reset·selection/page action·retire 뒤 모든 입력 거절을 한 연속 검사로 확인했습니다.
2. [x] `cargo clippy --manifest-path native/taide-native-terminal/Cargo.toml … --lib --test input -- -D warnings`: exit 0(0.42초).
3. [x] 공유 source 이동의 기존 소비자 `cargo check --manifest-path experiments/terminal-core-spike/Cargo.toml … --lib --test input-contract --test pty-input`: exit 0(0.56초). 기존 source·실제 PTY fixture 소비자가 compile됨을 확인했으며 같은 입력 정책의 기존 성공 runtime body/실제 PTY 검사는 반복하지 않았습니다. 새 actual Core 경계만 실행했습니다.
4. [x] native lib/input/new test·spike lib exact Rust fmt와 `git diff --check`: exit 0.

## 남은 실제 연결

- [ ] Native event/focus·물리/layout key·키패드/mouse·IME 조합·selection/search/link·paste 보안·입력 ack/queue와 PTY writer.
- [ ] 기존 PTY spawn port/TaskSupervisor/session store에 단일 Core를 연결하고 metadata/agent/effect/query consumer·borrowed renderer·native placeholder를 구현합니다. 기존 `TerminalSessionOutput::attach`는 preamble과 truncated raw replay를 전달하므로 새 Core의 완전한 state 복원으로 사용하지 않습니다. 새 view에 parser를 만드는 이중 소유도 하지 않습니다.
- [ ] 종료 시 실제 child/read/flusher/callback join·snapshot/live 무손실·다중 창·aggregate/peak/RSS·CPU/GUI·전체 N4/N1~N8/M8.

현재는 live-mode 입력 소비 경계만 완료입니다. 전체 M8 완료 뒤만 commit·push합니다.
