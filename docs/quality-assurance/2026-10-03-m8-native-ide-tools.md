# M8 native IDE tool dispatcher·pending 응답 경계

## 대상 파일

- `native/taide-native-app/src/{ide-tools,ide-tools-tests,lib}.rs`
- 원본: `src-tauri/src/domain/ide/server.rs`, `src-tauri/src/domain/ide/types.rs`, `src-tauri/src/lib.rs`의 IdeLayoutActions
- 재사용: `crates/taide-ide/src/{protocol,service,store}.rs`, `crates/taide-runtime/src/{ide_actions,layout_actions}.rs`

## 리포트

원본12개 tool과 JSON-RPC initialize/tools/list/tools/call/ping 응답을 Tauri-free native dispatcher로 옮겼습니다. 직접 AppServices를 사용하고 layout open/close는 필수 typed callback입니다. 파일 경로/상태 조회와 실제 runtime diff/save responder·취소 회수3개 검사가 통과했습니다. 실제 WebSocket 서버·lockfile/start/stop·생산용 layout callback·화면의 IdeDiffRequested/IdeSaveRequested 처리기는 아직 연결하지 않았으므로 IDE 서버/전체 Settings/M8 완료가 아닙니다.

## 상세

1. openFile은 열린 프로젝트 root guard·existing file을 먼저 검증하고 정확한 canonical path/title/preview를 필수 layout port에 보냅니다. source의 makeFrontmost는 성공 응답 종류를 고르는 분기이며 별도 OS frontmost 조작을 새로 넣지 않습니다. 언어는 실제 PluginStore ensure_loaded/overlay와 기존 MCP-only csharp/php/sql fallback을 사용합니다.
2. getCurrentSelection/getLatestSelection/getOpenEditors/getWorkspaceFolders/getDiagnostics/checkDocumentDirty는 동일 IDE store·전체 layout roots·project snapshot과 기존 protocol encoder를 사용합니다. File 아닌 AppFile의 content나 내부 app-owned path를 IDE에 새로 노출하지 않습니다. source의 dirty path exact match를 임의로 loose 비교로 바꾸지 않습니다.
3. openDiff는 root guard로 등록한 UUID request·PendingRequestOwner와 원본600초 timeout, IdeDiffRequested 이벤트 및 Saved/Rejected/TabClosed wire 응답을 재현합니다. saveDocument는 열린 File 탭 확인 뒤 동일5초 timeout·IdeSaveRequested·bool responder를 사용합니다. pending owner는 future 취소/Drop에서 요청을 회수하고 실제 native tool operation lease를 마지막 응답까지 유지합니다. pending policy에 새 즉시 성공/디스크 직접 저장을 넣지 않습니다.
4. close_tab/closeAllDiffTabs는 caller의 필수 close callback이 반환한 실제 Tab으로 IdeCloseTabRequested/request_id를 발행합니다. 원본은 callback 실패를 도구 성공 응답의 실패로 바꾸지 않습니다. source의 layout backend close와 native UI tabs.close의 dirty 확인 경로는 다르므로 생산용 callback을 임의의 no-op 또는 무조건 discard UI handler로 대체하지 않습니다. 테스트 callback은 실제 runtime layout mutation과 IdeStore.reconcile_closed_tab만 연결하며 native mirror/terminal/UI document cleanup의 완성 증거가 아닙니다.
5. executeCode unsupported와 unknown/missing 입력 오류, id 없는 notification 무응답을 기존 protocol 코드로 유지했습니다. initialize version은 caller 필수 인자라 실험 app의0.1.0을 자동으로 제품 버전이라고 보고하지 않습니다. source에서는 Tauri package version이었으며 최종 host가 실제 제품 버전을 전달해야 합니다. 새 dependency는 이 dispatcher 작업에 추가하지 않았습니다.

## 실제 최소 검증

공통 Cargo 환경은 agent hooks QA와 동일하며 모든 Cargo를 직렬 실행했습니다.

- `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib native_ide_tools`: compile6.34초·suite0.04초·3건 중 protocol/필수 포트 응답1 PASS, 실제 diff/save responder·취소 owner1 PASS입니다. layout/query1은 fixture `/var/...` 기대값과 root guard의 `/private/var/...` canonical 응답 차이로 FAIL했습니다. canonical 응답은 원본 계약이므로 제품 코드를 바꾸지 않았습니다.
- fixture를 생성 직후 directory.canonicalize해 실제 경로 표기를 맞추고 미사용 PaneId import도 제거했습니다. 실패한 `--lib native_ide_tools는_실제_layout`만1회 재실행해 compile3.72초·suite0.01초·1 PASS/exit0입니다. 단순 fixture 경로 표기 정정으로 앞선 두 성공을 반복하지 않았고 동일 제품 분기의 증거를 재사용합니다.
- layout/query 검사: root 밖/없는 파일을 port 전 거절, 실제 open/preview·csharp 응답/dirty·열린 editor/workspace, desktop selection을 remote owner가 바꾸지 않음·latest selection 보존, 진단 ready/error 응답, 실제 close mutation·빈 diff close count를 확인했습니다.
- protocol 검사: 기존12tool descriptor·실제 caller version·ping encode·missing/unsupported/unknown error code와 id, notification에 layout 이벤트 없음, state shutdown gate를 확인했습니다.
- pending 검사: root 밖 diff 즉시 Rejected, 실제 IdeDiffRequested→runtime ide_resolve_diff/guarded persistence→FILE_SAVED·실제 파일, 취소 시 pending diff/lease 회수, 실제 열린 File의 IdeSaveRequested→file_save→ide_resolve_save→saved 응답·실제 파일과 취소 save 회수를 확인했습니다. GUI가 아닌 synthetic 이벤트 소비자이며 timeout600초를 기다린 것은 아닙니다.
- 최종 `cargo clippy --lib --bins --tests -- -D warnings`는 최초 새 공개 fn port의 type_complexity로 exit101이었습니다. 같은 fn 계약을 OpenFileTabAction 별칭으로 명시해 suppression 없이 수정하고 관련 static 검사만 재실행해 exit0,11.85초입니다. 기존 Wry17개 dependency warning은 별도이며 authored 검사를 억제하지 않았습니다. 해당 타입 별칭은 동작을 바꾸지 않아 성공한 기능 검사는 재실행하지 않았습니다.
- authored5 exact Rustfmt edition2024·tracked diff whitespace 검사 exit0입니다. PROCESS/HANDOFF/재개 프롬프트와 이번 QA2개의 기존 Prettier 포맷/검사 exit0이며 authored native source5·manifest/lock2·QA2의 no-index whitespace도 빈 출력으로 확인했습니다. 신규/untracked no-index diff check의 빈 출력 exit1은 추가 차이가 있음을 뜻하며 whitespace 오류가 아닙니다.

## 다음 연결

- [x] Tauri-free 원본12tool 분기/프로토콜·필수 layout callbacks·pending owner/lease
- [x] 서로 다른3검사의 실제 성공과 실패 fixture 정정·성공 재사용
- [ ] 실제 authenticated WebSocket/MCP subprotocol·connection/notify·supervised request/timeout·lockfile·시작/중지/refresh/stale pending
- [ ] production layout lifecycle ports·실제 NativeApplication diff/save 이벤트 소비/응답·desktop selection/diagnostics 발행
- [ ] agent hooks·remote와 전체 Settings apply의 awaited 순서·native startup/Exit/실제 GUI/aux·전체 보안/메모리/성능

WebSocket dependency는 root/Tauri의 기존 tokio-tungstenite0.30 계약에 있으나 현재 native lock에는 아직 없습니다. 이번 dispatcher에는 새 dependency/가짜 server를 추가하지 않았습니다. 다음 실제 transport 구현 전 existing package/cache와 필요성을 확인하고 조립해야 합니다. timeout·취소된 connection의 하위 task Drop·diff close callback/mirror/terminal 정리·동시 사용자 편집·열린/new project 수명·aux/window·AX/픽셀은 실행하지 않았으며 각각 기존 M8 gate에서 native 서버/화면을 연결한 뒤 확인해야 합니다. 별개 keybinding Tab RED와 PTY remount A/B는 그대로 남아 있고 full suite green을 주장하지 않습니다.
