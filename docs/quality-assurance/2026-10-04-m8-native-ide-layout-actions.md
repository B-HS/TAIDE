# M8 production IDE layout callback

## 대상과 구현

대상은 `native/taide-native-app/src/ide-tools.rs`, `ide-tools-tests.rs`, `ide-server-tests.rs`입니다. 기존 IDE 도구의 필수 LayoutActions에 실제 공유 root action과 native Hub를 사용하는 constructor를 추가했습니다.

- 원본 Tauri `src/lib.rs::ide_layout_actions/open_ide_file_tab/close_ide_tab`, layout service와 root `open_tab_and_finish/close_tab_and_finish`를 대조했습니다. 별도 layout 모델/저장/preview/owner 정책을 만들지 않습니다.
- new의 open callback은 원본 shared open action이고 close callback은 IDE pending reconcile→root terminal kill→주입한 같은 Hub discard를 연결합니다. 원본 tool openFile/close_tab/closeAllDiffTabs/RPC wire 본문은 바꾸지 않았습니다.
- close callback은 필수 Hub를 capture할 수 있도록 함수 포인터에서 typed Arc closure로 바꿨습니다. 기존 두 테스트 fixture도 실제 기존 callback body를 그대로 Arc에 넣었으며 제품 default/no-op/echo는 추가하지 않았습니다.
- production App가 자신의 terminal Hub로 constructor를 호출하고 IDE server/Settings에 연결하는 다음 조립이 남습니다. constructor 제공은 server startup이나 화면 diff/save 처리기 완료가 아닙니다.

## 최소 검증

- [x] 첫 compile은 신규 fixture의 ClaudeDiff field 추측과 ToolError Debug unwrap으로 중단했습니다. 실제 모델의 request_id/path와 기존 code/message 오류 출력 방식으로 정정했으며 제품 모델/오류 타입은 바꾸지 않았습니다.
- [x] 정확한 신규 `ide_tools::tests::production_layout_actions는_파일_preview_diff_pending과_같은_hub의_pty_close를_연결한다`만 실행해 compile8.24초/suite0.01초·1 PASS입니다.
- [x] 검사1은 실제 root file/preview 설정 gate·tab close, 실제 PendingDiff의 TabClosed 응답/폐기, 실제 새 합성 /bin/cat의 root session/Hub/actor 회수·idle·감독 task0/IDE close 이벤트입니다. 사용자 child/app/home/Keychain/OS 설정은 건드리지 않았습니다.
- [x] native lib/bin/tests strict: exit0·13.48초입니다. 초기 strict는 Arc callback의 type_complexity로 실패했고 boundary의 named CloseTabAction으로 정정했습니다. 동작 성공1은 재사용하며 suppression은 없습니다. 기존 Wry dependency17 warnings와 authored strict를 구분합니다.
- [x] authored3 exact edition2024 rustfmt·tracked diff check exit0입니다. 신규 no-index 빈 출력/exit1은 정상 신규 diff입니다.

Cargo 환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, native manifest, --locked --offline --target-dir experiments/native-shell-spike/target입니다. 전체 suite와 이전 IDE server/도구 검사 성공은 반복하지 않았습니다. 해당 코드 상태의 fixture Arc 변환과 public contract는 lib test compile·최종 static으로 확인합니다. 이 slice는 root/Tauri/manifest/lock/MSRV/제품TS/보호bundle/TAIDE Git 불변입니다.

## 미완료

- [ ] 실제 App IDE LayoutActions/new·server startup와 Settings IDE→hooks→remote·event 전달/화면 diff/save·전체 owner/auxiliary/Exit를 기존 pending에서 연결합니다.
- [ ] 전체 N1~N8 0/8·keybinding RED/PTY remount·모든 view/cutover/Rust99%·IME/AX/perf/security/package/rollback은 남습니다. full M8 완료 전 commit/push하지 않습니다.
