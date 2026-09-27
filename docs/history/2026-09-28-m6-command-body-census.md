# M6 command body 전수 대조

상태: 현재 등록된 Specta 203개와 raw 3개, 합계 206개의 실제 body를 읽고 아래 목록에 대응했습니다. 이 결과는 정적 경계 조사이며 M6 구현 완료나 동작 동등성 검증이 아닙니다.

## 대상과 판정 기준

조사 시점의 제품 원천은 `8e818b4`입니다. `src-tauri/src/lib.rs`의 `specta_builder`·raw 등록과 `src-tauri/tests/fixtures/rust-native/ipc-contract-manifest.json`을 대조했습니다. 각 domain의 `commands.rs`는 제품 body와 호출하는 private helper까지 읽었으며 cfg별 같은 이름은 command 하나로 집계했습니다. `layout_move_tab_to_window`는 domain 이름을 추정하지 않고 composition root의 실제 body를 읽었습니다.

- F: 분리된 runtime application action에 위임합니다. callback의 실제 AppHandle·Channel·OS 창 작업은 adapter에 남을 수 있습니다.
- S: 이미 분리된 service/store 또는 공통 검증에 얕게 위임합니다. 단순 State 해제·경로/입력 전달을 새 정책으로 오인하지 않습니다.
- A: 제품 프로세스·OS·toolkit adapter입니다. native 소비자도 자기 adapter가 필요하며 이 분류가 자원 드레인 검증을 면제하지는 않습니다.
- P: 현재 Tauri body/helper에 공유해야 할 application 조립이 남습니다. 일부 하위 서비스나 자원 owner가 이전됐더라도 command 전체가 이전된 것은 아닙니다.

아래 command는 정확히 한 번씩 기록합니다. F/S/A 판정은 command entry body에 대한 것이며 연결된 callback·감독·전체 종료의 최종 합격 판정은 별도입니다. 특히 framework가 다른 native 소비자에게 Tauri State를 그대로 넘길 수 있다는 뜻이 아닙니다.

## command 대응 목록

### agent

원천: `src-tauri/src/domain/agent/commands.rs`, Specta 9개.

- P: `agent_list`, `agent_release_marker`, `agent_hooks_status`, `agent_hooks_install`, `agent_hooks_uninstall`
- S: `agent_pending_external_opens`
- A: `agent_cli_status`, `agent_cli_install`, `agent_cli_uninstall`

project 확인→foreground PID port→probe cache→hook override 조립, marker 검증/파일 제거/forget, hook scope·설정 gate·파일 소유권 조립이 남습니다. CLI의 macOS/비macOS 분기와 실제 osascript·symlink 작업은 OS adapter입니다. hook emitter의 timeout이 이미 시작한 blocking probe 종료를 보장하지 않으며 poll/attach/server callback은 별도 수명 대조 대상입니다. 실제 ps·CLI·사용자 hook 파일은 실행하거나 읽지 않았습니다.

### ai

원천: `src-tauri/src/domain/ai/commands.rs`, Specta 8개.

- P: `ai_token_status`, `ai_list_models`, `ai_inline_complete`, `ai_inline_edit`, `ai_commit_message`
- S: `ai_set_token`, `ai_clear_token`, `ai_request_cancel`

settings의 oMLX snapshot, 세 요청의 byte 상한, edit/commit의 provider/model 해석→begin 순서, prompt 로드→select→identity finish→응답 조립이 남습니다. 해석 실패가 begin 전에 끝나야 하고, 취소된 이전 요청의 늦은 finish가 새 요청을 제거하면 안 됩니다. 실제 HTTP·키링·자격증명에는 접근하지 않았습니다.

### app

원천: `src-tauri/src/domain/app/commands.rs`, Specta 5개.

- F: `app_file_read`, `app_file_write`
- A: `app_get_info`, `perf_snapshot`, `perf_reset`

비IPC `apply_settings_file`도 runtime action에 위임합니다. 제품 패키지 버전과 process-wide perf registry는 프로세스 adapter입니다.

### file

원천: `src-tauri/src/domain/file/commands.rs`, Specta 15개와 raw 1개.

- F: `file_open`, `file_save`, `file_create`, `file_rename`, `file_delete`, `file_copy`, `file_mirror_dirty`, `file_list_mirrors`, `file_clear_mirror`, `file_prune_mirrors`, `file_mirror_untitled`, `file_list_untitled_mirrors`, `file_clear_untitled_mirror`, `file_prune_untitled_mirrors`, `file_read_raw`
- A: `file_flush_complete`

raw 바이트 정책은 runtime, Response 포장은 Tauri입니다. flush 확인은 toolkit이 주입한 호출자 window label로 core handshake를 확인하고 All 완료에서만 exit를 요청하는 adapter이며 실제 창 종료/직접 Exit gate는 별도입니다.

### font

원천: `src-tauri/src/domain/font/commands.rs`, Specta 1개.

- A: `font_list`

기존 process font cache의 첫 OS 스캔을 blocking 실행합니다. 실제 시스템 폰트 스캔은 이번 조사에서 수행하지 않았습니다.

### git

원천: `src-tauri/src/domain/git/commands.rs`, Specta 41개.

- P: `git_init`, `git_status`, `git_diff_file`, `git_diff_staged_text`, `git_show_file`, `git_log`, `git_ahead_behind`, `git_remotes`, `git_gutter`, `git_blame_range`, `git_stage`, `git_unstage`, `git_discard`, `git_commit`, `git_push`, `git_pull`, `git_fetch`, `git_undo_last_commit`, `git_branches`, `git_branch_create`, `git_branch_checkout`, `git_branch_delete`, `git_stash_list`, `git_stash_push`, `git_stash_apply`, `git_stash_drop`, `git_discard_hunk`, `git_current_user`, `git_conflict_sides`, `git_resolve_conflict`, `git_stage_hunk`, `git_unstage_hunk`, `git_stage_lines`, `git_unstage_lines`, `git_commit_files`, `git_file_log`, `git_revert_commit`, `git_tags`, `git_tag_create`, `git_tag_delete`, `git_checkout_remote_branch`

`resolve_repo_root`의 cached root→열린 project→discover→cache 조립과 상태 조회 token/캐시 commit이 남습니다. 실제 Tauri 이벤트 구독은 adapter지만 invalidation→event 순서는 공유 정책입니다. mutation guard를 취득하는 위치가 init/stage/pull 등에서 다르며 push/fetch만 repo lock을 쓰고 전역 mutation guard는 쓰지 않습니다. stash_drop은 원래 이벤트가 없고 branch_create의 status는 checkout일 때만, revert의 refs는 비충돌일 때만 발행합니다. 이 차이를 하나의 무조건 후처리로 통일하지 않습니다. 실제 Git body를 실행하지 않았습니다.

### ide

원천: `src-tauri/src/domain/ide/commands.rs`, Specta 7개.

- P: `ide_set_selection`, `ide_clear_selection`, `ide_resolve_diff`, `ide_resolve_save`
- S: `ide_get_status`, `ide_publish_diagnostics`, `ide_notify_at_mention`

remote owner no-op, selection 투영→notification→store/broadcast, pending take→Saved content 확인→guard→save port→responder 조립이 남습니다. diff 저장의 Forbidden만 경고 후 Saved resolution을 유지하고 다른 오류는 반환하는 계약을 보존해야 합니다. 비IPC toggle/start/stop·lockfile·pending reconcile와 실제 MCP server는 별도 callback/lifecycle 경계입니다.

### layout

원천: `src-tauri/src/domain/layout/commands.rs`, Specta 19개.

- F: `layout_get`, `layout_open_tab`, `layout_close_tab`, `layout_activate_tab`, `layout_move_tab`, `layout_split`, `layout_open_tab_in_split`, `layout_resize`, `layout_focus_pane`, `layout_pin_tab`, `layout_set_preview`, `layout_reopen_closed`, `layout_set_view_state`, `layout_set_dirty`, `layout_set_terminal_session`, `layout_open_untitled`, `layout_convert_untitled`, `layout_apply_path_change`, `layout_set_shell_view`

close의 service wrapper도 runtime `close_tab_and_finish`를 소비하고 state 기록 후 assembly observer를 호출합니다. 같은 이벤트/상태 순서로 모든 layout 경로를 통일하지 않습니다.

### locale

원천: `src-tauri/src/domain/locale/commands.rs`, Specta 3개.

- F: `locale_get_current`
- S: `locale_list`, `locale_get`

현재 선택 정책은 runtime, 목록/팩 읽기는 분리된 service입니다.

### lsp

원천: `src-tauri/src/domain/lsp/commands.rs`, Specta 11개.

- P: `lsp_spawn`, `lsp_send`, `lsp_stop`, `lsp_restart`, `lsp_confirm_reinitialize`, `lsp_report_reinitialize_failure`, `lsp_install`
- S: `lsp_sessions`, `lsp_resolve_root`, `lsp_install_cancel`
- A: `lsp_detect_servers`

owner/root 재사용·subscriber·workspace notification·store 등록/rollback, stop의 guard 밖 프로세스 종료, restart의 종료 후 guard 재취득/entry 재확인, epoch·generation/status 조립이 남습니다. 설치 download/toolchain body는 runtime으로 이전됐지만 command의 shutdown·중복 admission·strategy 선택 조립은 남습니다. `spawn_process` callback의 epoch 확인, exit recovery/backoff/healthy reset도 공유 application 경계입니다. Channel 포장과 PATH 측정/log는 adapter입니다. 일반 LSP/설치의 이전 owner 검증을 이 command 조립 전수 이전으로 해석하지 않습니다.

### notification

원천: `src-tauri/src/domain/notification/commands.rs`, Specta 2개.

- F: `notification_notify`
- A: `notification_open_system_settings`

실제 focus 조회와 macOS 고정 설정 URL·비macOS 오류는 adapter입니다. notification 반환이 OS 표시 성공을 입증하지는 않습니다.

### plugin

원천: `src-tauri/src/domain/plugin/commands.rs`, Specta 5개.

- P: `plugin_reload`, `plugin_read_grammar`, `plugin_install`, `plugin_uninstall`
- S: `plugin_list`

guard→reload/store 기록, ensure_loaded→grammar 조회, blocking stage→guard→authoritative commit→reload→installed ID 선택이 남습니다. VSIX의 commit callback과 같은 plugins_dir/PluginStore를 사용하며 stage 동안 mutation guard를 취득하지 않는 기존 순서를 보존해야 합니다. stage 뒤 요청 Drop/중복 설치/종료에서 임시 경로와 blocking worker의 실제 소유권은 별도 재현 대상입니다.

### project

원천: `src-tauri/src/domain/project/commands.rs`, Specta 25개.

- P: `project_forget_recent`, `project_get_active`, `project_open`, `project_close`, `project_activate`, `project_reorder`, `project_set_display`, `project_group_create`, `project_group_rename`, `project_group_set_color`, `project_group_set_collapsed`, `project_group_set_members`, `project_group_delete`, `project_group_reorder`, `project_group_open`, `project_open_in_slot`, `shell_slot_close`, `session_focus_shell_slot`, `session_set_shell_slot_sizes`, `session_set_window_chrome`
- S: `project_list`, `project_list_recent`, `project_get`, `project_group_list`, `session_get_shell_state`

기존 service에 단순 전달하는 조회 5개는 다른 domain의 얕은 위임과 같은 S 기준입니다. 순수 service 자체의 재이전이 아니라 나머지 공유 상태/영속/이벤트/복원 조립의 facade가 남습니다. open의 guard 안 상태 기록→guard 밖 capability build→guard 안 재확인/commit→실패 close rollback을 보존해야 합니다. close는 창 snapshot→project flush→guard→중복 close 재확인→service/state write→detach→이벤트입니다. group 변경은 guard 해제 후 event인 경우가 있고 activate/reorder/display는 guard 안이므로 일괄 통일하지 않습니다. group open의 첫 성공만 activation을 소비하고 shutdown 뒤 나머지를 skipped로 기록하는 helper도 이전 대상입니다. 비IPC restore_state·watcher 복원 계획/build/register와 capability callback은 별도 경계입니다.

### remote

원천: `src-tauri/src/domain/remote/commands.rs`, Specta 5개.

- P: `remote_issue_link`, `remote_set_password`, `remote_clear_password`
- S: `remote_status`, `remote_revoke_sessions`

running 확인→token→allowed-host snapshot→URL, password 검증→secret port write→configured cache→session revoke 순서가 남습니다. 실제 secret port 구현은 OS adapter이며 메모리 fixture만으로 검증해야 합니다. 비IPC start/stop/toggle/cache refresh와 HTTP/WS dispatch port는 별도 lifecycle 경계입니다.

### search

원천: `src-tauri/src/domain/search/commands.rs`, Specta 4개.

- F: `search_run`, `search_cancel`, `search_replace`, `search_list_files`

Channel·perf/log는 adapter, 취소 identity·project guard·파일별 guarded replace·결과 정책은 runtime입니다.

### settings

원천: `src-tauri/src/domain/settings/commands.rs`, Specta 3개.

- F: `settings_get`, `settings_update`, `settings_set_theme`

비IPC 공통 apply도 runtime입니다. 실제 IDE→agent→remote observer callback 순서를 유지해야 합니다.

### snippet

원천: `src-tauri/src/domain/snippet/commands.rs`, Specta 3개.

- S: `snippet_list`, `snippet_save`, `snippet_delete`

분리된 service의 경로/검증/파일 정책에 위임합니다.

### sync

원천: `src-tauri/src/domain/sync/commands.rs`, Specta 5개.

- P: `sync_status`, `sync_connect`, `sync_disconnect`, `sync_upload`, `sync_download`

settings/secret snapshot→HTTP→guard 재검증→저장/공통 apply→이벤트 조립과 `overlay_sync_bookkeeping`·`decide_download_apply`가 남습니다. 최초 gist 생성은 guard를 유지하고 기존 gist upload만 해제/재취득합니다. disconnect/다른 sync의 승리, conflict/retry가 malformed payload 파싱보다 먼저인 순서, SettingsApplyPort 비재진입과 theme/locale 적용 순서를 보존해야 합니다. 실제 GitHub 호출이나 token 읽기는 수행하지 않았습니다.

### system

원천: `src-tauri/src/domain/system/commands.rs`, Specta 7개.

- F: `system_open_path`, `system_reveal_path`, `system_open_in_browser`, `system_open_app_data_path`, `system_open_external_url`
- A: `system_usage_get`, `system_usage_breakdown`

OS process 측정은 분리된 SystemUsageStore/service를 사용합니다. label provider의 terminal→agent→LSP 순서와 domain snapshot 조립은 composition callback 잔여 대조에 포함하며 OS process 측정을 실행하지 않았습니다.

### task

원천: `src-tauri/src/domain/task/commands.rs`, Specta 1개.

- S: `detect_tasks`

공통 `root_guard::project_root` 뒤 분리된 task service를 blocking 실행하는 얕은 위임입니다.

### terminal

원천: `src-tauri/src/domain/terminal/commands.rs`, Specta 10개와 raw 2개.

- P: `pty_default_options`, `pty_write`, `pty_kill`, `pty_detach`, `pty_spawn`, `pty_attach`
- S: `pty_resize`, `pty_set_paused`, `terminal_sessions`, `shell_profiles`, `resolve_terminal_path`, `terminal_resolve_link_candidates`

spawn은 이미 감독된 blocking owner와 TerminalStore admission/drain을 소비하지만 env port→owned guard→project/shutdown→ID/output/metadata→spawn→store→event 조립은 Tauri에 남습니다. 입력 observer→writer 취득→blocking write/flush, attach/detach/kill guard, 기본 cwd/root/settings/크기 정책도 이전 대상입니다. output Channel 포장·perf는 adapter이며 CWD/command marker/observer callback의 상태와 EventSink 조립을 별도로 공유해야 합니다. 하위 PTY 세 thread·부분 시작 회수의 완료를 이 command 전체 facade 완료로 해석하지 않습니다.

### theme

원천: `src-tauri/src/domain/theme/commands.rs`, Specta 5개.

- F: `theme_get_current`
- S: `theme_list`, `theme_get`, `theme_save`, `theme_delete`

현재 선택 정책은 runtime, 파일/해석 정책은 분리된 service입니다.

### tree

원천: `src-tauri/src/domain/tree/commands.rs`, Specta 5개.

- F: `tree_rows`, `tree_toggle`, `tree_collapse_all`, `tree_reveal`, `tree_refresh`

공유 TreeStore/잠금/prefetch 정책은 runtime에 있으며 perf만 adapter입니다.

### vsix

원천: `src-tauri/src/domain/vsix/commands.rs`, Specta 2개.

- P: `vsix_import_plugin`
- S: `vsix_extract_themes`

VSIX service의 extract는 얕은 위임입니다. import의 stage→guard→PluginRuntimePort commit과 root의 commit→reload/store→ID 선택 조립이 남습니다.

### window

원천: `src-tauri/src/domain/window/commands.rs`, Specta 1개.

- A: `window_set_fullscreen`

실제 호출자 toolkit window에 fullscreen을 적용하는 adapter입니다. 비IPC auxiliary create/close/restore·flush timeout·quit은 command 하나의 적합성만으로 완료되지 않습니다.

### composition root

원천: `src-tauri/src/lib.rs`, Specta 1개.

- F: `layout_move_tab_to_window`

runtime action에 실제 OS 창 create/close callback을 주입합니다. auxiliary tab return도 runtime action을 소비하며 root가 transient 감독자를 연결합니다.

## 집계와 남은 M6 조건

| 분류 | 개수 | 의미                                  |
| ---- | ---: | ------------------------------------- |
| F    |   57 | runtime action 위임                   |
| S    |   35 | 분리된 service/store의 얕은 위임      |
| A    |   13 | 실제 프로세스/OS/toolkit adapter      |
| P    |  101 | application 조립이 남은 command entry |
| 합계 |  206 | Specta 203 + raw 3                    |

P 101개는 미구현 사용자 기능 개수가 아니라 공유 경계를 아직 분리하지 않은 entry 개수입니다. 단순 service/store 위임을 공유 정책 미완료 개수에 포함하지 않았으며 각 service를 다시 쓰라는 뜻은 아닙니다. 기존 테스트/행동을 보존해 기능별 단위로 이전해야 합니다.

composition root의 SettingsApplyPort·PluginRuntimePort·ProjectRestoreWatchers·SystemUsageLabelProviders·PtySpawnEnvProvider·PtySessionObservers·AgentForegroundPids·layout close observer·IDE layout/save·remote dispatch·menu sources 연결을 읽었습니다. 단순 toolkit callback인 경우와 아직 state/순서 정책이 들어 있는 경우를 위 domain 설명에 구분했습니다. `ProjectCapabilities` 구현·hook/IDE/remote server 내부·watcher/flush의 nested blocking worker·direct Exit 전체는 후속 실제 body/owner 검증이 필요하므로 HK 전체를 완료하지 않습니다.

새 spawn 검색에서 project capability build/watchers restore의 awaited Tauri blocking 호출과 root 주기 layout flush의 blocking 호출을 확인했습니다. 상위 async 작업이 감독돼도 이미 시작한 내부 blocking 작업의 실제 완료까지 감독됐다는 증거가 아닙니다. AI/select 요청 취소와 plugin/VSIX stage 소유권도 기존 install/PTY owner의 green 결과로 대체할 수 없습니다. 요청형 동기 조회와 장수/외부 프로세스 작업, cfg(test) fixture를 이름만 보고 동일하게 분류하지 않습니다.

이번 변경은 문서만 수정하며 제품 코드·IPC·bindings·dependency는 불변입니다. 새 Rust·frontend·실기 성공을 주장하지 않으며 같은 제품 상태의 이전 성공 근거만 재사용합니다. 실제 앱·사용자 파일/프로세스·시크릿·키링·push는 실행하지 않습니다. 남은 M 전체와 M6 완료 후 일반 push 조건은 유지합니다.

## 실행한 검증

inline `bun -e` 읽기 전용 검사는 collect_commands 등록의 순서를 manifest와 비교하고, raw 3개의 실제 owner를 더한 뒤 위 분류 목록의 이름/owner를 대조했습니다. command 중복·누락·owner 불일치·원천 함수 이름 부재를 실패 조건으로 둔 결과 exit 0입니다. 초기 project 단순 조회 5개의 분류를 다른 service 위임과 같은 S로 바로잡은 뒤 영향받은 분류 검사를 한 번 재실행했습니다. 함수 이름 검사는 정적 존재 확인이며 Rust 타입·실제 callback 완료·동작 parity 증거는 아닙니다.

```json
{
    "specta": 203,
    "raw": 3,
    "total": 206,
    "owners": 26,
    "counts": { "F": 57, "S": 35, "A": 13, "P": 101 },
    "missing": 0,
    "duplicates": 0,
    "ownerMismatch": 0,
    "missingBodies": 0
}
```

새 문서에는 `bun node_modules/prettier/bin/prettier.cjs --ignore-path /dev/null --write docs/history/2026-09-28-m6-command-body-census.md`를 적용했습니다. 기본 ignore가 docs를 제외하므로 ignore를 해제해 실제 파일을 선택했습니다. 같은 파일의 `--check`와 `git diff --check`는 exit 0입니다. 기존 긴 PROCESS나 과거 표 전체를 재포맷하지 않습니다.
