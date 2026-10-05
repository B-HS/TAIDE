# M8 원격 전체 dispatcher 조립

## 대상과 결과

대상은 `native/taide-native-app/src/remote-dispatch.rs`, `remote-dispatch-tests.rs`, `lib.rs`입니다. 기존 177명령 domain adapter를 하나의 필수 포트 constructor로 조립했습니다. outer with_policy는 한 번만 적용하고 마지막 JSON/raw remaining은 Unclassified 원본 오류로 거절합니다. App/HTTP/OS 전체 생산용 조립 완료는 아닙니다.

## 구현 계약

- layout 포트는 terminal 포트의 같은 Hub에서 생성하므로 다른 Hub를 주입할 수 없습니다. sync reconcile도 preferences의 같은 callback에서 생성합니다.
- mandatory preferences/agents/Git/LSP/terminal/utilities/Gist factory와 ProjectLifecyclePort factory를 전달하며 제품 기본값/no-op/echo는 없습니다.
- 실제 AppServices·NativeProjects·runtime·파일·Settings·검색 channel·PTY를 합성 integration으로 연결했습니다. 프로젝트 factory/reconcile은 Arc identity를 검사합니다. 테스트의 합성 OS/font/Gist 포트를 제품 구현으로 세지 않습니다.
- 실제 /bin/cat만 새 합성 프로젝트에 실행하고 같은 Core/Hub의 raw 채널·입력·layout close·child idle·감독 task0을 확인했습니다. 사용자 앱/보호 bundle/자격 증명/OS 설정은 조작하지 않았습니다.

## 최소 검증

- [x] 초기 compile은 존재하지 않는 kill_all_sessions와 거부 목록 tuple fixture 오류로 중단했습니다. 실제 kill_all API와 tuple 이름 추출로 정정했습니다.
- [x] `cargo test … --lib remote_dispatch::tests -- --nocapture`: compile9.06초/suite0.09초, catalog·실제 PTY 2 PASS, Settings fixture 1 FAIL입니다.
- [x] 실패 fixture는 remoteAccessEnabled를 원격 patch 보호값으로 잘못 가정했습니다. 실제 원본 patch 보호4필드와 sync filter5필드는 다릅니다. remotePasswordOnlyLogin fixture로 정정하고 제품 정책은 바꾸지 않았습니다. 실패한 `실제_project_file_raw_search_channel과_설정_sync는_동일_services와_reconcile을_사용한다`만 exact 재실행해 compile3.86초/suite0.06초 PASS입니다. 이전 성공2건은 재사용했습니다.
- [x] 고유3건은 catalog177·중복0·29deny·unknown/JSONraw모드, 실제 project/file/raw/search/plugin/font/agent/Settings/sync·동일 services/reconcile·이벤트, 실제 PTY raw/write/close/idle/task0입니다.
- [x] native lib/bin/tests strict `-D warnings`: exit0·13.10초입니다. 기존 Wry dependency17 warnings와 authored strict를 구분하며 suppression은 없습니다. authored3 exact rustfmt·tracked diff check exit0입니다.

Cargo 환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, native manifest, --locked --offline --target-dir experiments/native-shell-spike/target입니다. 전체 suite/GUI/WS와 사용자 OS/GitHub는 실행하지 않았습니다. 이번 slice의 root/Tauri/manifest/lock/MSRV/제품 TS/TAIDE Git 변경은 없습니다.

## 미완료

- [ ] constructor를 생산용 OS/Gist/capabilities/AppInfo/Settings IDE→hooks→remote·assets/state/events·socket_action과 NativeApplication startup/Exit에 연결합니다.
- [ ] 기존 pending Gist HTTP 공유를 후속26으로 세분화합니다. Tauri 구현을 공유 sync crate로 이동하고 runtime trait adapter·Tauri re-export·기존 HTTP/payload 검사로 동작을 보존합니다. 새 registry package/version은 추가하지 않습니다.
- [ ] 전체 N1~N8 0/8·keybinding Tab RED·PTY remount·모든 view/cutover/Rust99%·IME/AX/성능/보안/패키징/rollback은 미완료입니다. 전체 M8 완료 전 commit/push하지 않습니다.
