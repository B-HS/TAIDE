# M8 production agent host ports

## 대상과 구현

대상은 공유 runtime `agent_host.rs`, `agent_host_tests.rs`, `lib.rs`, Cargo.toml과 root/native lock의 runtime entry, 기존 Tauri `domain/agent/commands.rs`, native `remote-agents.rs`, `remote-agents-tests.rs`, `agent-hooks.rs`입니다.

- Unix의 batch ps/pid=,comm=,args=·stderr/실패 빈 결과·공유 parser, 미해결 PID만 supervised probe/cache하는 정책과 Windows의 전체 snapshot/descendant/name/cmdline 정책을 runtime으로 이동했습니다. 기존 Tauri는 같은 함수를 re-export합니다. 이전 source와 공유 source의 5개 함수 본문은 alias/공백 정규화 대조에서 모두 동일합니다.
- CLI metadata/canonicalize/dangling 상태는 실제 target Path를 받는 공유 함수로 옮겼고 기존 Tauri target/설치 정책은 불변입니다. 일반 파일도 metadata가 있으면 installed라는 원본 정책을 임의로 ownership 판정으로 바꾸지 않았습니다.
- native remote agent Ports::new/default는 실제 TerminalStore foreground·공유 probe·실제 CLI 상태/home·기존 native cached Claude emitter를 필수 lazy callback으로 제공합니다. emitter/target visibility만 crate 범위로 열었으며 기존 probe body는 불변입니다.
- Windows snapshot에는 표준 라이브러리 대체가 없어 기존 저장소의 sysinfo0.39를 runtime Windows 직접 edge로 재사용합니다. 실제 lock은 기존0.39.6/checksum이며 새 registry package/version/MSRV는 추가하지 않았습니다. 두 lock의 runtime dependency entry만 관련 추가이고 기존 dirty diff는 보존했습니다.

## 최소 검증

- [x] 공유 `cargo test -p taide-runtime --lib agent_host::tests ...`: compile13.90초/suite0.00초, 합성 CLI1 PASS·자체 PID1 실패입니다. sandbox ps 결과가 비어 own PID가 없었습니다. 정확한 실패 own PID 검사만 require_escalated로 실행해0.11초/suite0.01초 PASS이며 CLI 성공은 재사용했습니다. 사용자 process 목록을 조회하지 않았고 자체 테스트 runner만 조회했습니다.
- [x] CLI 검사는 미설치·일반 파일·정상 symlink·dangling symlink의 실제 metadata/canonical 상태, PID 검사는 빈 입력·자체 runner batch 정보·비agent cache/축출·supervised probe·shutdown/task0입니다.
- [x] native exact `remote_agents::tests::production_port`: compile13.69초/suite0.01초·1 PASS입니다. 실제 production Ports의 빈 foreground/probe와 agent_list·hooks-disabled 설치 거절/server 없음/task0을 확인했습니다. CLI/home/emitter는 lazy이며 제품 target·사용자 home/Claude CLI/hooks를 실행하지 않았습니다.
- [x] 공유 runtime lib/tests strict exit0·4.60초, native lib/bin/tests strict exit0·15.95초, 변경된 기존 Tauri caller lib check exit0·4.57초입니다. native Wry dependency17 warnings와 authored strict를 구분합니다. authored7 exactfmt/관련 신규 no-index check·tracked diff check를 완료했습니다.

Cargo는 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, --locked --offline --target-dir experiments/native-shell-spike/target입니다. native는 자신의 manifest를 사용하고 모든 Cargo는 직렬 실행했습니다. 종료 handles62525/86808/84972/9121/30107이며 live handle은 없습니다. 기존 성공·전체 suite는 반복하지 않았습니다. 제품TS/보호 bundle/사용자 앱/OS 설정/자격 증명/TAIDE Git은 불변입니다.

## 공식 근거와 미완료

[Rust symlink_metadata](https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html), [Command literal arguments](https://doc.rust-lang.org/std/process/struct.Command.html#method.args)를 확인했습니다. sysinfo 웹 조회 실패 뒤 pinned 공식 registry source sysinfo0.39.6의 System::new_all/processes를 확인했습니다. Windows 함수 본문 동일은 실제 Windows 실행이나 cross-compile 증거가 아닙니다.

- [ ] Windows OS probe와 실제 사용자 CLI/home/Claude emitter·agent polling/외부 open/sidecar·전체 App wiring/OS 실기는 기존 gate입니다.
- [ ] production events/assets/startup/Exit·HostBridge Settings/AppFile/화면·native editor LSP 복구·keybinding RED/PTY remount/N1~N8 0/8은 미완료입니다. 전체 M8 완료 전 commit/push하지 않습니다.
