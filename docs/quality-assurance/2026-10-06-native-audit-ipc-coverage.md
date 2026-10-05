# native 전환 감사: 백엔드 명령(IPC) 대 native 호출부 대응

작성일 2026-10-06. 영역 ipc-coverage. 읽기 전용 감사 결과이며, 이전 에이전트의 HANDOFF.md/PROCESS.md 수치는 사용하지 않고 실제 파일만으로 판정했습니다.

## 0. 요약

- 대상 command 는 specta 203종 + raw 3종(`pty_spawn`, `pty_attach`, `file_read_raw`) = 206종, event 30종입니다. 근거는 `src/shared/api/bindings.ts` 의 `__TAURI_INVOKE` 203건, `src-tauri/src/lib.rs:811-815` 의 raw 3종, `bindings.ts:1011-1042` 의 `events` 30종입니다. `docs/ipc-contract.md` 머리말의 186종/24종은 낡은 수치입니다.
- facade(taide-runtime action 또는 그 하위 crate 서비스)는 206종 중 agent_cli_install/agent_cli_uninstall 2종을 제외한 204종이 Tauri 없이 존재합니다(window_set_fullscreen·file_flush_complete 는 facade 가 필요 없는 항목으로 처리).
- native 앱이 실제로 호출하는 command 는 206종 중 91종(done)이고, 109종은 facade 만 있고 native UI 호출부가 없습니다(partial). 4종은 native 에 대응 개념이 불필요(n/a), 2종은 facade 자체가 src-tauri 에 묶여 있습니다(missing). unwired(native 호출 코드는 있으나 도달 불가)는 command 단위로는 0종입니다.
- 109종 중 41종은 git, 8종은 AI, 5종은 sync, 5종은 remote 자격증명, 4종은 검색, 7종은 에이전트, 14종은 프로젝트 관리(그룹·최근·닫기·재정렬)입니다. 이 도메인들은 native UI 가 없어 원격(WebSocket) dispatch 쪽에서만 facade 를 호출합니다.
- event 30종 중 native 가 수신해 UI 에 반영하는 것은 15종, 발행되지만 UI 소비자가 없는 것 12종, 대응 불필요 2종, 발행 자체가 없는 것 1종입니다.
- 가장 큰 구조 문제는 IDE 연동 서버(`ide-tools.rs`)입니다. 서버는 기동하지만 diff 리뷰·저장 응답·선택 영역·진단을 채우는 UI 쪽 호출부가 없어 Claude Code 가 openDiff 를 호출하면 600초, saveDocument 는 5초 뒤 실패합니다.

## 1. 범위

TS 측으로 읽은 경로
- `/Users/hyunseokbyun/development/TAIDE/src/shared/api/bindings.ts` (command 203종·event 30종 전수)
- `/Users/hyunseokbyun/development/TAIDE/src/{entities,features,widgets,shared,app}` 의 `commands.*`/`events.*` 호출 전수(테스트 제외)로 TS 사용 여부 확인
- `/Users/hyunseokbyun/development/TAIDE/src-tauri/src/lib.rs` 의 `collect_commands!`(509-713)·`collect_events!`(714-)·raw handler(811-815)
- `/Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/{agent,app,file,window,notification}/commands.rs` (facade 유무 확인)
- `/Users/hyunseokbyun/development/TAIDE/docs/ipc-contract.md` 머리말

native 측으로 읽은 경로
- `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/{host.rs,application-ports.rs,events.rs,event-relay.rs,bootstrap.rs,main.rs,ide-tools.rs,remote-*.rs,agent-hooks.rs,projects.rs}` 및 `application.rs` 의 조립부(270-470, 4440-4510)
- `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src/{commands.rs,controller.rs,snapshot.rs}`
- `/Users/hyunseokbyun/development/TAIDE/crates/taide-runtime/src/*_actions.rs`, `native_*_actions.rs`, `app_services.rs`, `platform_services.rs`
- `/Users/hyunseokbyun/development/TAIDE/crates/taide-remote/src/command-policy.rs`

판정 방식과 용어
- native UI 호출부는 `HostCommand` 처리(host.rs:560-1448), `taide_native_ui::commands::dispatch`(commands.rs:73-140), 그리고 application.rs 가 부르는 모듈들의 직접 호출(`taide_file::service::*`, `taide_layout::service::*`, `taide_lsp::native::*`)을 모두 포함합니다.
- 원격(WS) 열의 "WS"는 `remote-*.rs` 의 dispatch(`remote-dispatch.rs:50-66` 에서 조립)가 facade 를 부르는 경우입니다. 이는 브라우저 Wasm 클라이언트용이며 native UI 호출부로 인정하지 않았습니다. 원격 허용 목록과 dispatch 일치는 `remote-dispatch-tests.rs:399-401` 이 강제합니다.
- done 은 facade(또는 동등한 하위 서비스) 존재 + native UI 호출부 존재입니다. "done(직접)" 은 native 가 runtime facade 를 거치지 않고 같은 하위 서비스를 직접 부르는 경우입니다(§5 설계 문제 참고).
- 모든 열거는 grep 결과 기반입니다. 호출부 목록의 원본 grep 은 `(project_actions|layout_actions|file_actions|tree_actions|search_actions|plugin_actions|agent_actions|agent_hook_actions|lsp_actions|lsp_install_actions|git_actions|terminal_actions|task_actions|locale_actions|theme_actions|snippet_actions|settings_actions|system_actions|ide_actions|ai_actions|sync_actions|vsix_actions|remote_actions|notification_actions|app_actions|native_file_actions|native_lsp_actions)::` 를 `native/` 전체에서 돌린 것입니다.

## 2. command 대응표

열 약어: facade = taide-runtime 의 action 또는 하위 crate 서비스, WS = 원격 dispatch 에서 호출 여부, TS = TS view 가 호출하는지(U 사용, - 미사용).

### 2.1 app / window / perf / notification

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| app_get_info | - | `taide_app::service` | AppInfo 를 application.rs:~290 에서 직접 구성해 원격 포트로 전달 | O | done | TS UI 는 호출하지 않음 |
| app_file_read | U | `app_actions::app_file_read` | `HostCommand::ReadAppFile`(host.rs:592) → app-file.rs:142 | O | done | |
| app_file_write | U | `app_actions::app_file_write_admitted` | `HostCommand::WriteAppFile`(host.rs:561) → app-file-write.rs:81 | O | done | settings 는 reconcile 경유 |
| window_set_fullscreen | U | 불필요 | zen.rs:20 `ViewportCommand::Fullscreen` 직접 | 거부 | done | egui 가 대체 |
| perf_snapshot | U | `taide_app::service::perf_snapshot` | 없음 | 거부 | partial | 진단 패널 UI 없음 |
| perf_reset | U | `taide_infra::perf::global().reset()` (action 래퍼 없음) | 없음 | 거부 | partial | 위와 동일 |
| notification_notify | U | `notification_actions::notification_notify` | 발행 호출부 없음. `NativePlatform::send_notification`(bootstrap.rs:174) 만 존재 | 거부 | partial | 설정 토글만 있고 발화 지점 없음 |
| notification_open_system_settings | U | action 없음(`PlatformServices::open_url` 한 줄) | 없음 | 거부 | partial | |

### 2.2 project / session (25종)

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| project_list | U | `project_actions::project_list` | `ShellSnapshot::read`(snapshot.rs:21-33)가 `state.session` 직접 읽기 | O | done(직접) | |
| project_list_recent | U | `project_actions::project_list_recent` | 없음 | 거부 | partial | Welcome 의 최근 목록 UI 없음(shell.rs:853 welcome 에 목록 없음) |
| project_forget_recent | U | `project_actions::project_forget_recent` | 없음 | 거부 | partial | |
| project_get | U | `project_actions::project_get` | snapshot 직접 읽기 | O | done(직접) | |
| project_get_active | U | `project_actions::project_get_active` | snapshot 의 `shell.focused` | O | done(직접) | |
| project_open | U | `project_actions::project_open` | `HostCommand::OpenProject`(host.rs:1259) → projects.rs:49 | O | done | |
| project_close | U | `project_actions::project_close` | 없음(검색어: project_close, close_project, ProjectClosed) | O | partial | 슬롯 닫기와 별개로 프로젝트 닫기 동작 없음 |
| project_activate | U | `project_actions::project_activate` | `ShellMutation::ActivateProject`(commands.rs:76) | O | done | |
| project_reorder | U | `project_actions::project_reorder` | 없음 | O | partial | |
| project_set_display | U | `project_actions::project_set_display` | 없음 | O | partial | 프로젝트 아이콘/라벨/색 편집 없음 |
| project_group_list | U | `project_actions::project_group_list` | snapshot.groups | O | done(직접) | |
| project_group_create | U | `project_actions::project_group_create` | 없음 | O | partial | |
| project_group_rename | U | `project_actions::project_group_rename` | 없음 | O | partial | |
| project_group_set_color | U | `project_actions::project_group_set_color` | 없음 | O | partial | |
| project_group_set_collapsed | U | `project_actions::project_group_set_collapsed` | `ShellMutation::SetGroupCollapsed`(commands.rs:85) | O | done | |
| project_group_set_members | U | `project_actions::project_group_set_members` | 없음 | O | partial | |
| project_group_delete | U | `project_actions::project_group_delete` | 없음 | O | partial | |
| project_group_reorder | U | `project_actions::project_group_reorder` | 없음 | O | partial | |
| project_group_open | U | `project_actions::project_group_open` | 없음 | O | partial | |
| project_open_in_slot | U | `project_actions::project_open_in_slot` | 없음(OpenFolder 는 `OpenProject` 만 호출) | O | partial | 슬롯 분할로 프로젝트 열기 없음 |
| shell_slot_close | U | `project_actions::shell_slot_close` | `ShellMutation::CloseSlot`(commands.rs:82) | O | done | |
| session_get_shell_state | U | `project_actions::session_get_shell_state` | `taide_project::service::shell_state`(snapshot.rs:25) | O | done(직접) | |
| session_focus_shell_slot | U | `project_actions::session_focus_shell_slot` | `ShellMutation::FocusSlot`(commands.rs:79) | O | done | |
| session_set_shell_slot_sizes | U | `project_actions::session_set_shell_slot_sizes` | `ShellMutation::ResizeSlots`(commands.rs:88) | O | done | |
| session_set_window_chrome | U | `project_actions::session_set_window_chrome` | `ShellMutation::SetWindowChrome`(commands.rs:91, application.rs:4320) | O | done | |

### 2.3 layout (20종)

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| layout_get | U | `layout_actions::layout_get` | snapshot.layouts | O | done(직접) | |
| layout_open_tab | U | `layout_actions::layout_open_tab` | `HostCommand::OpenFileTab/NewTerminal/OpenSettings/OpenAppFile/OpenProblem`(host.rs:596,626,946,1297,1319) | O | done | |
| layout_close_tab | U | `layout_actions::close_tab_and_finish` | `HostCommand::CloseTab`(host.rs:1179) → tabs.rs:119 가 `finish_mutation` 으로 자체 구현 | O | done(직접) | |
| layout_activate_tab | U | `layout_actions::layout_activate_tab` | `ShellMutation::ActivateTab`(commands.rs:106) | O | done | |
| layout_move_tab | U | `layout_actions::layout_move_tab` | `ShellMutation::MoveTab`(commands.rs:115) | O | done | |
| layout_split | U | `layout_actions::layout_split` | `ShellMutation::SplitTab`(commands.rs:124) | O | done | |
| layout_open_tab_in_split | U | `layout_actions::layout_open_tab_in_split` | `HostCommand::OpenFileToSide`(host.rs:1334) | O | done | |
| layout_resize | U | `layout_actions::layout_resize` | `ShellMutation::ResizePane`(commands.rs:127) | O | done | |
| layout_focus_pane | U | `layout_actions::layout_focus_pane` | `ShellMutation::FocusPane`(commands.rs:112) | O | done | |
| layout_pin_tab | U | `layout_actions::layout_pin_tab` | `ShellMutation::PinTab`(commands.rs:118) | O | done | |
| layout_set_preview | U | `layout_actions::layout_set_preview` | `ShellMutation::KeepTab`(commands.rs:121) | O | done | 프리뷰 해제 방향만 |
| layout_reopen_closed | U | `layout_actions::layout_reopen_closed` | `ShellMutation::ReopenClosed`(commands.rs:109) | O | done | |
| layout_set_view_state | U | `layout_actions::layout_set_view_state` | 없음. native 는 모든 탭에 `view_state: None` 기록(terminal_tabs.rs:152, save.rs:145, application.rs:5460) | O | partial | 커서·스크롤·접기 상태가 layout 에 저장되지 않음. §5 결함 6 |
| layout_set_dirty | U | `layout_actions::layout_set_dirty` | `HostCommand::SetDirty`(host.rs:1435) | O | done | |
| layout_set_terminal_session | U | `layout_actions::layout_set_terminal_session` | terminal_tabs.rs:469 가 `taide_layout::service::set_terminal_session` 직접 | O | done(직접) | |
| layout_open_untitled | U | `layout_actions::layout_open_untitled` | `HostCommand::NewUntitled`(host.rs:1137) | O | done | |
| layout_convert_untitled | U | `layout_actions::layout_convert_untitled` | untitled.rs:199 가 `convert_untitled_to_file` 직접 | O | done(직접) | |
| layout_move_tab_to_window | U | `layout_actions::layout_move_tab_to_window`, `return_auxiliary_window_tabs` | 없음. `WindowRegistry`/`TabWindowTarget` 를 native 가 전혀 참조하지 않음 | 거부 | partial | 보조 창(editor-N) 개념 자체가 없음 |
| layout_apply_path_change | U | `layout_actions::layout_apply_path_change` | explorer_move.rs:78, workspace_rename.rs:270 가 `apply_tab_path_change` 직접 | O | done(직접) | |
| layout_set_shell_view | U | `layout_actions::layout_set_shell_view` | `ShellMutation::SetSidebarCollapsed`(commands.rs:94) | O | done | zen 토글 경로는 zen.rs |

### 2.4 file / tree (20종 + raw 1)

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| file_open | U | `file_actions::file_open` | `HostCommand::OpenDocument`(host.rs:1346) → `native_file_actions::open_document_file` | O | done | |
| file_save | U | `file_actions::file_save` | `HostCommand::Save/SaveTracked`(host.rs:1385) → file_sync.rs:142 `save_file_within_open_projects`, tabs.rs:184 | O | done | |
| file_create | U | `file_actions::file_create` | explorer.rs:1497-1529 `taide_file::service::create_entry` 직접 | O | done(직접) | |
| file_rename | U | `file_actions::file_rename` | explorer_move.rs:120,333, workspace_rename.rs:300 직접 | O | done(직접) | |
| file_delete | U | `file_actions::file_delete` | explorer_delete.rs:108, workspace_delete.rs:184 직접 | O | done(직접) | |
| file_copy | U | `file_actions::file_copy` | explorer_clipboard.rs:234 직접 | O | done(직접) | |
| file_mirror_dirty | U | `file_actions::file_mirror_dirty` | `HostCommand::MirrorDraft`(host.rs:1093), application.rs:4458(종료 시) | O | done | |
| file_list_mirrors | U | `file_actions::file_list_mirrors` | tabs.rs:173, file_sync.rs:61, missing_draft.rs:199 | O | done | |
| file_clear_mirror | U | `file_actions::file_clear_mirror*` | `HostCommand::CleanupFileMirror`(host.rs:1129) | O | done | |
| file_prune_mirrors | U | `file_actions::file_prune_mirrors` | 없음 | O | partial | 고아 미러 청소 없음 |
| file_mirror_untitled | U | `file_actions::file_mirror_untitled` | persistence.rs:259, application.rs:4477 | O | done(직접) | |
| file_list_untitled_mirrors | U | `file_actions::file_list_untitled_mirrors` | untitled.rs:250 | O | done(직접) | |
| file_clear_untitled_mirror | U | `file_actions::file_clear_untitled_mirror` | `HostCommand::CleanupUntitledMirror`(host.rs:1161), tabs.rs:110 | O | done(직접) | |
| file_prune_untitled_mirrors | U | `file_actions::file_prune_untitled_mirrors` | 없음 | O | partial | |
| file_flush_complete | U | `AppState::complete_flush`(상태 메서드) | 불필요 | 거부 | n/a | 웹뷰-백엔드 flush 핸드셰이크 대신 application.rs:4440-4500 이 종료 시 직접 flush(미러+layout) |
| tree_rows | U | `tree_actions::tree_rows` | `HostCommand::TreeRows`(host.rs:1390) | O | done | |
| tree_toggle | U | `tree_actions::tree_toggle` | `HostCommand::TreeToggle`(host.rs:1406) | O | done | |
| tree_collapse_all | U | `tree_actions::tree_collapse_all` | `HostCommand::TreeCollapse`(host.rs:1421) | O | done | |
| tree_reveal | U | `tree_actions::tree_reveal` | explorer.rs:1542,1599, explorer_clipboard.rs:194 | O | done | |
| tree_refresh | U | `tree_actions::tree_refresh` | `HostCommand::RefreshTree`(host.rs:1196) | O | done | |
| file_read_raw (raw) | U | `file_actions::file_read_raw` | preview.rs:143 가 `taide_file::service::read_raw` 직접 | O | done(직접) | |

### 2.5 search / plugin / vsix / task / font

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| search_run | U | `search_actions::search_run` | 없음. 호출은 remote-search.rs:70 만(검색어: SearchQuery, search_run, SearchStore, search_replace) | O | partial | 검색 패널 UI 없음 |
| search_cancel | U | `search_actions::search_cancel` | remote-search.rs:101 만 | O | partial | |
| search_replace | U | `search_actions::search_replace` | remote-search.rs:90 만 | O | partial | |
| search_list_files | U | `search_actions::search_list_files` | remote-search.rs:112 만 | O | partial | 퀵오픈 팔레트 없음 |
| plugin_list | U | `plugin_actions::plugin_list` | UI 호출 없음(remote-plugins.rs:32). 문서 열기만 `taide_plugin::service::ensure_loaded` 직접(host.rs:1347) | O | partial | |
| plugin_reload | U | `plugin_actions::plugin_reload` | remote-plugins.rs:33 만 | O | partial | |
| plugin_read_grammar | U | `plugin_actions::plugin_read_grammar` | remote-plugins.rs:35 만 | O | partial | native 하이라이트 엔진이 다르면 n/a 가능(다른 영역 확인 필요) |
| plugin_install | U | `plugin_actions::plugin_install` | 없음(검색어: plugin_install, PluginInstall, LoadedPlugin) | 거부 | partial | |
| plugin_uninstall | U | `plugin_actions::plugin_uninstall` | 없음 | 거부 | partial | |
| vsix_extract_themes | U | `vsix_actions::vsix_extract_themes` | 없음(검색어: vsix, extract_themes, import_plugin 은 native 전체 0건) | 거부 | partial | 테마 편집기의 VSIX 가져오기 없음 |
| vsix_import_plugin | U | `vsix_actions::vsix_import_plugin` | 없음 | 거부 | partial | |
| detect_tasks | U | `task_actions::detect_tasks` | remote-utilities.rs:140 만(검색어: detect_tasks, TaskDetect, run_task) | O | partial | 작업 실행기 UI 없음 |
| font_list | U | `taide_font::service::list_families` | `HostCommand::ReadSettingsResource`(host.rs:655-667) | O | done | |

### 2.6 agent (9종)

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| agent_list | U | `agent_actions::agent_list` | 없음(remote-agents.rs:115 만). `poll_agents`(agent_actions.rs:135)를 부르는 루프가 native 에 없음(src-tauri lib.rs:1087 만) | O | partial | |
| agent_release_marker | U | `agent_actions::agent_release_marker` | remote-agents.rs:128 만 | O | partial | |
| agent_cli_status | U | `agent_host::cli_install_status` | remote-agents.rs:61 만 | O | partial | |
| agent_cli_install | U | 없음. osascript 실행은 `src-tauri/src/domain/agent/commands.rs:185-207`(`run_cli_osascript`,`cli_install_target`)에 묶임. 하위 crate 에는 `taide_agent::service::build_cli_install_apple_script` 만 있음 | 없음(검색어: cli_install, cli_uninstall, install_cli, osascript) | 거부 | missing | |
| agent_cli_uninstall | U | 없음(commands.rs:209-241) | 없음 | 거부 | missing | |
| agent_pending_external_opens | U | `agent_actions::agent_pending_external_opens` | 없음. native 는 CLI 인자·single-instance 수신이 없어 `AgentExternalOpen` 이 발행되지 않음 | 거부 | partial | |
| agent_hooks_status | U | `agent_hook_actions::agent_hooks_status` | remote-agents.rs:138 만. 전역 토글만 `agent_hooks::apply_toggle`(agent-hooks.rs:77) | O | partial | 프로젝트·에이전트별 설치 UI 없음 |
| agent_hooks_install | U | `agent_hook_actions::agent_hooks_install` | remote-agents.rs:148 만 | O | partial | |
| agent_hooks_uninstall | U | `agent_hook_actions::agent_hooks_uninstall` | remote-agents.rs:164 만 | O | partial | |

### 2.7 lsp (11종)

native 는 `taide_lsp::native::*` 세션(lsp.rs:12-18, native_lsp_actions.rs:9)을 직접 사용하며 `lsp_actions` facade 는 remote-lsp.rs 만 사용합니다. 재초기화 처리는 세션 내부로 흡수되어 있습니다(session.rs:27-28, lsp.rs:1335-1366).

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| lsp_spawn | U | `lsp_actions::lsp_spawn` | lsp.rs:744 `spawn_session` | O | done(직접) | |
| lsp_send | U | `lsp_actions::lsp_send` | `SessionClient` 직접 | O | done(직접) | |
| lsp_stop | U | `lsp_actions::lsp_stop` | lsp.rs 세션 종료 경로 | O | done(직접) | |
| lsp_restart | U | `lsp_actions::lsp_restart` | lsp-recovery.rs | O | done(직접) | |
| lsp_confirm_reinitialize | U | `lsp_actions::lsp_confirm_reinitialize` | 세션 내부 처리(`REINITIALIZE_MAX_ATTEMPTS`) | O | done(직접) | 핸드셰이크 자체가 불필요 |
| lsp_report_reinitialize_failure | U | `lsp_actions::lsp_report_reinitialize_failure` | `Failure::ReinitializeExhausted`(lsp.rs:1335) | O | done(직접) | |
| lsp_sessions | U | `lsp_actions::lsp_sessions` | 세션 스냅샷 직접 | O | done(직접) | |
| lsp_detect_servers | U | `taide_lsp::service::detect_servers` | lsp.rs:636 | O | done | |
| lsp_resolve_root | U | `lsp_actions::lsp_resolve_root` | lsp.rs:649 `find_root` | O | done(직접) | |
| lsp_install | U | `lsp_actions::lsp_install`, `lsp_install_actions::run_download_install` | 없음(검색어: lsp_install, LspInstall, install_progress) | 거부 | partial | 서버 다운로드 설치 UI 없음 |
| lsp_install_cancel | U | `lsp_actions::lsp_install_cancel` | remote-lsp.rs:182 만 | O | partial | |

### 2.8 git (41종, 전부 partial)

facade 는 `git_actions` 에 41종 모두 존재하고(git_actions.rs:117-673), 호출부는 `remote-git.rs:99-318` 뿐입니다. native UI 쪽에서 `git_actions`/`GitStore` 를 쓰는 곳은 `event-relay.rs`(상태 캐시 무효화)와 `projects.rs`(`.git` 감시 발행)뿐입니다. 검색어: `git_actions`, `GitStatus`, `taide_git`, `stage_hunk`, `GutterHunk`, `BlameLine` — native UI 모듈 0건.

| command | TS | facade | 상태 |
|---|---|---|---|
| git_init | U | `git_actions::git_init` | partial |
| git_status | U | `git_actions::git_status` | partial |
| git_diff_file | U | `git_actions::git_diff_file` | partial |
| git_diff_staged_text | U | `git_actions::git_diff_staged_text` | partial |
| git_show_file | U | `git_actions::git_show_file` | partial |
| git_log | U | `git_actions::git_log` | partial |
| git_ahead_behind | - | `git_actions::git_ahead_behind` | partial |
| git_remotes | U | `git_actions::git_remotes` | partial |
| git_gutter | U | `git_actions::git_gutter` | partial |
| git_blame_range | U | `git_actions::git_blame_range` | partial |
| git_stage | U | `git_actions::git_stage` | partial |
| git_unstage | U | `git_actions::git_unstage` | partial |
| git_discard | U | `git_actions::git_discard` | partial |
| git_commit | U | `git_actions::git_commit` | partial |
| git_push | U | `git_actions::git_push` | partial |
| git_pull | U | `git_actions::git_pull` | partial |
| git_fetch | - | `git_actions::git_fetch` | partial |
| git_undo_last_commit | - | `git_actions::git_undo_last_commit` | partial |
| git_branches | U | `git_actions::git_branches` | partial |
| git_branch_create | U | `git_actions::git_branch_create` | partial |
| git_branch_checkout | U | `git_actions::git_branch_checkout` | partial |
| git_branch_delete | U | `git_actions::git_branch_delete` | partial |
| git_stash_list | U | `git_actions::git_stash_list` | partial |
| git_stash_push | U | `git_actions::git_stash_push` | partial |
| git_stash_apply | U | `git_actions::git_stash_apply` | partial |
| git_stash_drop | U | `git_actions::git_stash_drop` | partial |
| git_discard_hunk | U | `git_actions::git_discard_hunk` | partial |
| git_current_user | U | `git_actions::git_current_user` | partial |
| git_conflict_sides | U | `git_actions::git_conflict_sides` | partial |
| git_resolve_conflict | U | `git_actions::git_resolve_conflict` | partial |
| git_stage_hunk | U | `git_actions::git_stage_hunk` | partial |
| git_unstage_hunk | U | `git_actions::git_unstage_hunk` | partial |
| git_stage_lines | U | `git_actions::git_stage_lines` | partial |
| git_unstage_lines | U | `git_actions::git_unstage_lines` | partial |
| git_commit_files | U | `git_actions::git_commit_files` | partial |
| git_file_log | U | `git_actions::git_file_log` | partial |
| git_revert_commit | U | `git_actions::git_revert_commit` | partial |
| git_tags | U | `git_actions::git_tags` | partial |
| git_tag_create | U | `git_actions::git_tag_create` | partial |
| git_tag_delete | U | `git_actions::git_tag_delete` | partial |
| git_checkout_remote_branch | U | `git_actions::git_checkout_remote_branch` | partial |

(TS 열 "-" 인 git_ahead_behind, git_fetch, git_undo_last_commit 은 TS view 도 호출하지 않는 바인딩입니다. 전수 grep 으로 확인했습니다.)

### 2.9 terminal (10종 + raw 2)

native 터미널은 `terminal_actions::pty_spawn`(terminal_host.rs:729)으로 세션을 만들고, 입출력·크기·종료는 자체 `Hub`/`Writer`/frames 큐(terminal_host.rs, terminal_writer.rs, terminal_frames.rs)로 처리합니다.

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| pty_spawn (raw) | U | `terminal_actions::pty_spawn` | terminal_host.rs:729 | O | done | |
| pty_attach (raw) | U | `terminal_actions::pty_attach` | `HostCommand::AttachTerminal`(host.rs:853) → Tabs::attach | O | done(직접) | |
| pty_default_options | U | `terminal_actions::pty_default_options` | terminal_tabs.rs:421 | O | done | |
| pty_write | U | `terminal_actions::pty_write` | terminal_host.rs `Writer`(핸들 직접 쓰기) | O | done(직접) | |
| pty_resize | U | `terminal_actions::pty_resize` | `HostCommand::ResizeTerminal`(host.rs:844) | O | done(직접) | |
| pty_kill | U | `terminal_actions::pty_kill` | terminal_host.rs:524,558, `HostCommand::CloseTab`(host.rs:1185) | O | done(직접) | |
| pty_set_paused | U | `terminal_actions::pty_set_paused` | 없음 | O | n/a | 웹뷰 flow control 대체. native 는 유한 큐(terminal_frames Limits)로 역압 처리 |
| pty_detach | U | `terminal_actions::pty_detach` | 없음 | O | n/a | 웹뷰 구독자 개념이 없음 |
| terminal_sessions | U | `terminal_actions::terminal_sessions` | 없음 | O | n/a | 웹뷰 재로드 후 세션 재발견용. native 는 프로세스 내 Hub 가 보유 |
| shell_profiles | U | `terminal_actions::shell_profiles` | host.rs:673 | O | done | |
| resolve_terminal_path | - | `terminal_actions::resolve_terminal_path` | 없음 | O | partial | TS view 도 미사용 |
| terminal_resolve_link_candidates | U | `terminal_actions::terminal_resolve_link_candidates` | host.rs:737 `taide_terminal::service::resolve_link_candidates` | O | done(직접) | |

### 2.10 locale / theme / snippet / settings / system

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| locale_list | U | `locale_actions::locale_list` | settings-view.rs:102 | O | done | |
| locale_get | - | `locale_actions::locale_get` | `locale_get_for_language`(presentation-refresh.rs:75) 변형 | O | done | |
| locale_get_current | U | `locale_actions::locale_get_current` | application.rs:5494 | O | done | |
| theme_list | U | `theme_actions::theme_list` | settings-view.rs:101, theme-edit.rs:244 | O | done | |
| theme_get | U | `theme_actions::theme_get` | theme-edit.rs:323, status-editor.rs:489, ui-fonts.rs:315 | O | done | |
| theme_get_current | U | `theme_actions::theme_get_current` | `theme_get_for_settings`(presentation-refresh.rs:74) 변형 | O | done | |
| theme_save | U | `theme_actions::theme_save` | theme-edit.rs:261 | O | done | |
| theme_delete | U | `theme_actions::theme_delete` | theme-edit.rs:350 | O | done | |
| snippet_list | U | `snippet_actions::snippet_list` | snippet-edit.rs:113, snippet-catalog.rs:205 | O | done | |
| snippet_save | U | `snippet_actions::snippet_save` | snippet-edit.rs:117 | O | done | |
| snippet_delete | U | `snippet_actions::snippet_delete` | snippet-edit.rs:120 | O | done | |
| settings_get | U | `settings_actions::settings_get` | `state.settings.read()` 직접(application.rs 다수) | O | done(직접) | |
| settings_update | U | `settings_actions::settings_update` | host.rs:894,1484 | O | done | |
| settings_set_theme | U | `settings_actions::settings_set_theme` | host.rs:906 | O | done | |
| system_usage_get | U | `remote_utilities::Ports` 경유 | system-usage.rs:12, application.rs:324 | O | done | |
| system_usage_breakdown | U | 위와 동일 | system-usage.rs | O | done | |
| system_open_path | U | `system_actions::system_open_path` | `HostCommand::OpenPath`(host.rs:1028) | 거부 | done | macOS 전용(§5 결함 5) |
| system_reveal_path | U | `system_actions::system_reveal_path` | `HostCommand::RevealPath`(host.rs:1059) | 거부 | done | |
| system_open_in_browser | U | `system_actions::system_open_in_browser` | `HostCommand::OpenInBrowser`(host.rs:1076) | 거부 | done | |
| system_open_app_data_path | U | `system_actions::system_open_app_data_path` | `HostCommand::OpenSettingsFolder`(host.rs:678) | 거부 | done | |
| system_open_external_url | U | `system_actions::system_open_external_url` | `HostCommand::OpenTerminalUrl`(host.rs:775) | 거부 | done | 터미널 링크 경로만 |

### 2.11 ide (7종)

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| ide_get_status | U | `ide_actions::ide_get_status` | application.rs:5152 `services.ide.status()` 폴링 → status-ide.rs | O | done(직접) | |
| ide_set_selection | U | `ide_actions::ide_set_selection` | 없음(검색어: set_selection, ide_set_selection, IdeSelectionInput) | O | partial | `getCurrentSelection`(ide-tools.rs:327)이 항상 빈 값 |
| ide_clear_selection | U | `ide_actions::ide_clear_selection` | 없음 | O | partial | |
| ide_publish_diagnostics | U | `ide_actions::ide_publish_diagnostics` | 없음. native 는 `diagnostics::Store` 를 자체 보유하나 IdeStore 로 발행하지 않음 | O | partial | `getDiagnostics`(ide-tools.rs:199)가 "not published yet" 오류 |
| ide_resolve_diff | U | `ide_actions::ide_resolve_diff` | 없음. diff 리뷰 UI 와 `IdeDiffRequested` 소비자 없음(tabs.rs:129 는 탭 닫힘 시 TabClosed 응답만) | O | partial | `openDiff`(ide-tools.rs:144)가 최대 600초 대기 |
| ide_resolve_save | U | `ide_actions::ide_resolve_save` | 없음. `IdeSaveRequested` 소비자 없음 | O | partial | `saveDocument`(ide-tools.rs:239)가 5초 후 실패 |
| ide_notify_at_mention | U | `ide_actions::ide_notify_at_mention` | 없음 | O | partial | 선택 영역을 에이전트에 멘션하는 단축 동작 없음 |

### 2.12 ai / sync / remote (18종)

| command | TS | facade | native 호출부 | WS | 상태 | 비고 |
|---|---|---|---|---|---|---|
| ai_token_status | U | `ai_actions::ai_token_status` | remote-ai.rs:44 만 | O | partial | 검색어: ai_actions, inline_complete, AiInline, ai_token, commit_message |
| ai_set_token | U | `ai_actions::ai_set_token` | 없음 | 거부 | partial | 자격증명 입력 UI 없음 |
| ai_clear_token | U | `ai_actions::ai_clear_token` | 없음 | 거부 | partial | |
| ai_list_models | U | `ai_actions::ai_list_models` | remote-ai.rs:46 만 | O | partial | |
| ai_inline_complete | U | `ai_actions::ai_inline_complete` | remote-ai.rs:49 만 | O | partial | 에디터 자동 제안 없음 |
| ai_inline_edit | U | `ai_actions::ai_inline_edit` | remote-ai.rs:52 만 | O | partial | |
| ai_commit_message | U | `ai_actions::ai_commit_message` | remote-ai.rs:55 만 | O | partial | git UI 부재와 연동 |
| ai_request_cancel | U | `ai_actions::ai_request_cancel` | remote-ai.rs:58 만 | O | partial | |
| sync_status | U | `sync_actions::sync_status` | remote-sync.rs:44 만 | O | partial | 검색어: sync_actions, sync_connect, SyncStatus(gist 문자열은 register 와 겹쳐 제외) |
| sync_connect | U | `sync_actions::sync_connect` | 없음 | 거부 | partial | |
| sync_disconnect | U | `sync_actions::sync_disconnect` | 없음 | 거부 | partial | |
| sync_upload | U | `sync_actions::sync_upload` | remote-sync.rs:55 만 | O | partial | |
| sync_download | U | `sync_actions::sync_download` | remote-sync.rs:68 만 | O | partial | |
| remote_status | U | `remote_actions::remote_status` | remote-preferences.rs:238 만. 서버 기동은 설정 토글 reconcile(application-ports.rs:98-105)로만 | O | partial | 포트·접속 수 표시 없음 |
| remote_issue_link | U | `remote_actions::remote_issue_link` | 없음(검색어: issue_link, remote_actions) | 거부 | partial | 원격 거부 정책이므로 native UI 가 유일한 발급 경로 |
| remote_revoke_sessions | U | `remote_actions::remote_revoke_sessions` | remote-preferences.rs:240 만 | O | partial | |
| remote_set_password | U | `remote_actions::remote_set_password` | 없음 | 거부 | partial | |
| remote_clear_password | U | `remote_actions::remote_clear_password` | 없음 | 거부 | partial | |

### 2.13 집계

| 상태 | 건수 | 구성 |
|---|---|---|
| done | 91 | specta 88 + raw 3. 그중 "직접(facade 우회)" 이 약 30종 |
| partial | 109 | git 41, project 14, ide 6, ai 8, sync 5, remote 5, agent 7, search 4, plugin 5, vsix 2, lsp 2, layout 2, file 2, perf 2, notification 2, terminal 1, task 1 |
| unwired | 0 | command 단위로는 없음 |
| missing | 2 | agent_cli_install, agent_cli_uninstall |
| n/a | 4 | file_flush_complete, pty_set_paused, pty_detach, terminal_sessions |
| 합계 | 206 | |

원격 정책(`command-policy.rs`)의 REMOTE_DENIED 는 29항목이며, 그중 native 가 호출부를 가진 것은 window_set_fullscreen 과 system_* 5종뿐입니다. 나머지 거부 command 는 원격에서 쓸 수 없고 native UI 로만 실행할 수 있으므로, native UI 호출부가 없으면 기능이 어디에서도 도달 불가입니다: layout_move_tab_to_window, plugin_install/uninstall, lsp_install, vsix 2종, agent_cli_install/uninstall, agent_pending_external_opens, notification 2종, remote_issue_link/set_password/clear_password, project_list_recent/forget_recent, ai_set_token/clear_token, sync_connect/disconnect, perf 2종(합 약 25종). 이 목록이 Tauri 앱의 데스크톱 전용 기능을 native 가 잃는 범위입니다.

## 3. event 대응표 (30종)

native 는 이벤트 구독 대신 두 갈래로 반영합니다. (a) `NativeShellEvents`(controller.rs:22-27)가 모든 이벤트에서 `ShellSnapshot` 을 재읽기. (b) `PaintSink`(application.rs:52-60)가 `TreeChanges`(events.rs:36-111)와 `presentation_refresh::Changes`(presentation-refresh.rs:20-34)에 fs·settings·theme 이벤트를 기록. 원격 클라이언트에는 `event-relay.rs:70-205` 가 프레임으로 중계합니다.

| event | TS 소비 | 발행(native) | native 수신·반영 | 상태 | 비고 |
|---|---|---|---|---|---|
| project:opened | U | project_actions | snapshot 재읽기 | done | |
| project:closed | U | project_actions | snapshot 재읽기 | done | |
| project:activated | U | project_actions | snapshot 재읽기 | done | |
| project:list-changed | U | project_actions | snapshot 재읽기 | done | |
| project:groups-changed | U | project_actions | snapshot 재읽기 | done | |
| project:recent-cleared | U | 발행 호출부 없음(forget_recent 미호출) | 없음 | partial | |
| session:shell-slots-changed | U | project_actions | snapshot 재읽기 | done | |
| session:window-chrome-changed | U | project_actions | snapshot 재읽기 | done | |
| layout:changed | U | layout_actions | snapshot 재읽기 | done | |
| theme:changed | U | theme-edit.rs:263,339,354, settings 경로 | `presentation_refresh::Changes` | done | |
| settings:changed | U | settings_actions | `Changes`, presentation.rs:46, status-editor.rs:343 | done | |
| fs:changed | U | projects.rs:231 | `TreeChanges::record`(events.rs:36) → 탐색기·프리뷰·열린 파일 관찰 | done | |
| fs:rescan-required | U | projects.rs:222, explorer_*.rs | `TreeChanges::record` | done | |
| terminal:spawned | U | terminal_actions.rs:77 | 구독자 없음 | n/a | 웹뷰 미러링용. native 는 Hub 보유 |
| terminal:exited | U | terminal_host.rs:947 | UI 는 Hub 상태를 직접 읽음 | done(직접) | |
| terminal:cwd-changed | U | terminal_dispatch.rs:174 | 구독자 없음 | partial | 탭 제목 갱신 경로 별도 확인 필요 |
| terminal:command-finished | U | terminal_dispatch.rs:183 | 구독자 없음 | partial | 작업 완료 알림 없음 |
| git:status-changed | U | projects.rs:255 | `GitEvents::record` 가 캐시 무효화만 | partial | git UI 없음 |
| git:refs-changed | U | projects.rs:260 | 동일 | partial | |
| lsp:session-status-changed | U | lsp-process.rs:62 | 상태바는 자체 LSP 상태(lsp-status.rs) 사용 | done(직접) | |
| lsp:install-progress | U | 발행 없음(install 경로 미호출) | 없음 | partial | |
| agent:state-changed | U | `apply_hook_payload`(agent-hooks.rs:236) | 소비자 없음. 배지 UI 없음(검색어: AgentActivity, agentStatusBadge) | partial | 설정 토글만 존재(settings-controls.rs:60-95) |
| agent:external-open | U | 발행 없음(CLI·single-instance 수신 없음) | 없음 | missing | `taide <path>` 열기 불가 |
| ide:status-changed | U | ide-server.rs:159 | UI 가 `services.ide.status()` 폴링 | done(직접) | |
| ide:diff-requested | U | ide-tools.rs:179 | 소비자 없음 | partial | 결함 1 |
| ide:save-requested | U | ide-tools.rs:267 | 소비자 없음 | partial | 결함 1 |
| ide:close-tab-requested | U | ide-tools.rs:304,375 | 소비자 없음 | partial | |
| sync:state-changed | U | 발행 없음 | 없음 | partial | |
| remote:state-changed | U | remote-http.rs:122,206 | 소비자 없음 | partial | 상태 표시 UI 없음 |
| app:hot-exit-flush-requested | U | 발행 없음 | 불필요 | n/a | application.rs:4440-4500 이 종료 시 직접 flush |

집계: done 15, partial 12, n/a 2, missing 1.

## 4. 실제 앱 연결이 끊긴 지점

1. IDE 서버 응답 경로: `AppEvent::IdeDiffRequested/IdeSaveRequested` 를 application.rs 가 처리하지 않고(`PaintSink` 는 repaint 만), `IdeStore` 로의 selection·diagnostics 발행도 없음. 서버(ide-server.rs, ide-tools.rs)는 `application_ports.start`(application.rs:451-460)로 실제 기동됩니다.
2. 원격 접근 자격증명 경로: 서버는 설정 토글로 기동(application-ports.rs:98-105)하지만 `remote_issue_link`/`remote_set_password` 를 부르는 native UI 가 없어 브라우저 Wasm 클라이언트가 접속 링크를 받을 수 없습니다. 원격 정책이 두 command 를 거부하므로 우회 경로도 없습니다.
3. 에이전트 상태: hook 서버는 기동하고 `apply_hook_payload` 가 `AgentStateChanged` 를 발행하지만, `poll_agents` 루프(agent_actions.rs:135)는 native 에서 시작되지 않고 UI 소비자도 없습니다. 설정 토글 `agentStatusBadge`, `notifyAgent*` 는 영향이 없습니다.
4. 알림: `notification_notify` 를 부르는 곳이 없어 `notifyTaskCompleted`, `notifyGitRemote`, `notifyLspInstall`, `notifyError` 등 설정 토글이 전부 무효입니다.
5. 레이아웃 view state: `layout_set_view_state` 호출이 없고 모든 탭이 `view_state: None` 으로 만들어집니다.
6. 터미널 이벤트 `terminal:cwd-changed`, `terminal:command-finished` 는 발행되지만 구독자가 없습니다.
7. 보조 창: `WindowRegistry`(app_services.rs:31)는 존재하나 native 에서 창을 만드는 코드가 없고 `layout_move_tab_to_window` 호출부도 없습니다.
8. LSP 설치: `lsp_install_actions` 와 `LspInstallStore` 는 AppServices 에 조립되어 있으나 호출부가 없어 서버를 PATH 에서 찾지 못하면 설치할 방법이 없습니다.
9. git 이벤트 파이프라인: 파일 감시가 `GitStatusChanged/GitRefsChanged` 를 발행해 `GitStore` 를 무효화하지만 읽는 UI 가 없습니다. 원격 클라이언트만 소비합니다.

## 5. 잘못 구현됐거나 보강이 필요한 native 코드(결함·설계 문제)

1. 높음. IDE 연동이 기동 상태로 정지합니다. 근거: ide-tools.rs:144-197(openDiff 대기 600초), 239-283(saveDocument 5초), 327-338(getCurrentSelection), 199-222(getDiagnostics). 어떤 UI 도 `ide_resolve_diff/ide_resolve_save/ide_set_selection/ide_publish_diagnostics` 를 부르지 않습니다. 에이전트 쪽에서는 응답 지연과 "diagnostics not ready" 로 보이므로 기능이 켜진 것처럼 보이는 상태가 가장 위험합니다. 수정 방향: (a) 에디터 선택 변경과 `diagnostics::Store` 변경을 `ide_actions::ide_set_selection/ide_publish_diagnostics` 로 발행, (b) `IdeSaveRequested` 를 받아 `HostCommand::Save` 후 `ide_resolve_save`, (c) diff 리뷰 탭 UI 와 `ide_resolve_diff`.
2. 높음. 컷오버 차단 요인. `LaunchConfig::parse`(bootstrap.rs:26-47)가 `--data-dir <격리된 절대경로>` 없이는 종료하고, 키체인 서비스명이 `net.gumyo.taide.native-isolated`(bootstrap.rs:12)로 고정되어 기존 Tauri 앱의 설정·프로젝트·토큰을 읽지 못합니다. window title 도 `TAIDE Native`(main.rs:7)입니다. 기본 데이터 경로·기존 서비스명 승계와 CLI/single-instance 처리(`agent:external-open`)가 컷오버 전에 필요합니다. 이 항목은 다른 영역(M8 컷오버) 소관일 수 있으나 IPC 의 agent_pending_external_opens 와 직결되어 기록합니다.
3. 높음. 원격 접근이 켜져도 사용할 수 없습니다. 발급·비밀번호 command 호출부 부재(§4-2). 수정: 설정 화면에 링크 발급(`remote_issue_link`)·비밀번호 설정·세션 폐기·상태 표시를 추가하고 `remote:state-changed` 를 구독.
4. 중간. facade 우회로 인한 이중 구현. native 가 runtime action 을 거치지 않고 `taide_layout::service`/`taide_file::service`/`taide_lsp::native` 를 직접 호출하는 지점: tabs.rs:119, untitled.rs:199, explorer_move.rs:78, workspace_rename.rs:270, explorer.rs:1529, explorer_delete.rs:108, explorer_clipboard.rs:234, lsp.rs:744. 같은 논리 command(`layout_apply_path_change`, `file_rename`, `lsp_spawn` 등)가 원격에서는 facade 로, native UI 에서는 별도 구현으로 실행되어 락 순서·이벤트 발행·`finish_mutation` 호출이 갈릴 수 있습니다. 수정: facade 로 일원화하거나, 의도적 우회라면 동등성 테스트(`remote-*-tests.rs` 대 native 경로)를 추가.
5. 중간. `NativePlatform` 이 macOS 전용입니다(bootstrap.rs:174-208). 비 macOS 는 열기·표시·알림이 `Internal` 오류이며, 알림은 `osascript display notification` 이라 앱 아이콘·권한·클릭 동작이 없고 `notification_open_system_settings` 대응도 없습니다. Tauri 앱의 CLI 경로 상수(agent-hooks.rs:21-24)는 Windows 도 가정하므로 비대칭입니다.
6. 중간. 탭 view state(커서·스크롤·접기) 미저장. `layout_set_view_state` 호출이 없고 native 가 만드는 모든 `Tab` 에 `view_state: None`(terminal_tabs.rs:152,260,298, save.rs:145, missing_draft.rs:161, application.rs:5460)입니다. 에디터 스토어(editor_surface.rs:132,281)가 뷰 상태를 메모리에만 유지하는지 영속화하는지는 이 감사에서 확정하지 못했습니다(§7).
7. 중간. 설정 토글과 소비자 불일치. `settings-controls.rs:60-95` 의 `AgentStatusBadge`, `NotifyAgent*` 등 토글이 값만 저장하고 효과가 없습니다. 사용자에게는 동작하는 설정으로 보입니다.
8. 낮음. `docs/ipc-contract.md` 머리말이 낡았습니다(186/24 대 실제 203+3/30). 정본 선언(“이 문서의 목록이 정본”)과 어긋납니다. native 쪽 파리티 테스트는 원격 허용 목록과 dispatch 의 일치만 검증하고(`remote-dispatch-tests.rs:399-401`), “native UI 가 각 command 를 부르는가” 에 대한 검사는 없습니다.
9. 낮음. `file_prune_mirrors`·`file_prune_untitled_mirrors` 호출부 부재로 프로젝트 복원 시 고아 미러가 쌓일 수 있습니다(청소 시점이 TS 에서는 레이아웃 복원 직후였음). 정리 코드(`tabs.rs:173`, `untitled.rs:250`)가 대체하는지는 확정하지 못했습니다.
10. 낮음. 터미널 `terminal_sessions`/`pty_detach`/`pty_set_paused` 를 n/a 로 판정한 것은 native 가 프로세스 내에서 세션을 보유하는 전제입니다. 향후 창 분리·보조 창을 도입하면 `terminal_sessions` 대응이 필요합니다.

## 6. 권장 구현 순서(의존 관계 포함)

의존 관계 요약: 3(원격 자격증명 UI)은 설정 화면에, 5(git)는 탐색기 사이드바·diff 뷰에, 7(AI)은 git commit 입력과 에디터 ghost text 에 의존합니다. 모든 항목은 `HostCommand` 변형 추가 + `HostReply` 반영 + 이벤트 소비의 3단계로 같은 패턴을 재사용하므로 host.rs 확장이 선행 공통 작업입니다.

1. 컷오버 전제 정리(결함 2): 기본 데이터 경로, 서비스명 승계, CLI/single-instance 와 `agent:external-open` 발행. 의존: 없음. 규모 M.
2. IDE 연동 닫기(결함 1): selection·diagnostics 발행, `IdeSaveRequested` 처리, diff 리뷰 UI. 의존: 에디터 선택 모델, diagnostics Store. 규모 M.
3. 프로젝트 관리 command 14종: close, reorder, set_display, group CRUD, open_in_slot, 최근 목록·forget. 의존: shell.rs welcome 확장. `project_close` 는 종료 flush 와 같은 경로를 재사용해야 하므로 먼저 설계. 규모 L.
4. 원격 접근 설정 UI(결함 3): 링크·비밀번호·세션·상태. 의존: 설정 화면, `remote:state-changed` 구독. 규모 M.
5. 검색(4종)과 퀵오픈(search_list_files). 의존: host.rs 에 스트리밍 응답 채널(Channel 대체) 추가. 규모 L.
6. git 41종: 상태·diff·stage·commit·branch·stash·tag·conflict·blame·gutter. 의존: 탐색기 상태 아이콘, 에디터 gutter, diff 뷰어(2번 diff UI 재사용). 규모 XL.
7. AI 8종: 토큰 입력, 모델 목록, inline complete/edit, commit message 생성, 취소. 의존: 6(commit message), 에디터 ghost text. 규모 L.
8. 에이전트(7종)와 알림·이벤트: `poll_agents` 루프 시작, 상태 배지, hooks 프로젝트별 설치, `notification_notify` 발화 지점(`terminal:command-finished`, 에이전트 완료). 의존: 터미널 이벤트 구독. 규모 M.
9. LSP 설치(2종)와 진행 이벤트, 플러그인 관리(5종), VSIX(2종), sync(5종), 작업 감지(1종), 진단 패널(perf 2종). 서로 독립. 규모 각 S~M.
10. `layout_set_view_state` 영속화와 `file_prune_*` 호출(결함 6, 9), `layout_move_tab_to_window` 의 보조 창. 보조 창은 egui 멀티 뷰포트 설계가 필요해 마지막. 규모 M~L.
11. facade 일원화 또는 동등성 테스트(결함 4)와 `docs/ipc-contract.md` 갱신(결함 8). 규모 S~M.
12. agent_cli_install/uninstall 의 facade 화: `run_cli_osascript`, `cli_install_target` 을 taide-runtime 으로 이동한 뒤 native 설정 UI 에 연결. 규모 S.

## 7. 확인하지 못한 것(불확실성)

- 이 감사는 빌드·테스트를 실행하지 않았습니다. “done” 은 정적으로 호출 체인을 따라간 결과이며 런타임 동작 검증이 아닙니다.
- done(직접) 약 30종의 native 직접 경로가 해당 facade 와 동일한 인가(root_guard)·이벤트 발행·락 순서를 지키는지는 코드 비교까지 하지 않았습니다. 예: preview.rs:143 의 `read_raw` 가 `file_read_raw` 와 같은 `resolve_owning_project_or_cli_opened` 인가를 거치는지는 확인하지 못했습니다.
- `plugin_read_grammar` 는 native 에디터가 TextMate 문법 대신 다른 하이라이트 엔진을 쓰면 n/a 일 수 있습니다. 에디터 영역 감사에서 확정해야 합니다.
- `terminal:cwd-changed` 로 탭 제목을 갱신하는 별도 경로가 있는지(Hub 메타데이터 폴링 등)는 확정하지 못했습니다.
- 에디터 뷰 상태(커서·스크롤·접기)를 layout 대신 다른 저장소로 영속화하는지 확정하지 못했습니다(결함 6).
- REMOTE_ALLOWED 의 정확한 개수는 직접 세지 않았고 206 - 29(거부) = 177 로 산술했습니다.
- application.rs 는 약 5,500줄이며 이 영역에서는 조립부와 종료부만 읽었습니다. 다른 곳에서 `services.ide` 에 발행하거나 `notification_actions` 를 부르는 코드가 있다면 grep(`(…)_actions::` 패턴, `AppEvent::` 전수)에는 잡혔을 것이므로 가능성은 낮지만, 매크로나 별칭 import 로 가려진 호출은 놓칠 수 있습니다.
- taide-remote-web(브라우저 Wasm)이 원격 경로로 위 109종 중 어느 것을 UI 에서 쓰는지는 이 영역의 범위 밖이라 보지 않았습니다.
