# M6 command 등록 owner 전수 대조

상태: 203개 Specta command와 raw 3개의 등록 owner는 확인했습니다. 각 body의 application 정책과 Tauri adapter 적합성 전수 판정은 미완료입니다.

## 대상과 방법

`src-tauri/src/lib.rs`의 `collect_commands!` 블록을 읽고 등록 경로별로 집계했습니다. `src-tauri/tests/fixtures/rust-native/ipc-contract-manifest.json`의 spectaCommands는 203개이며 rawChannelCommands는 pty_spawn·pty_attach·file_read_raw입니다. 이름 prefix만으로 owner를 추정하지 않았습니다. 예를 들어 session_* command 4개는 project owner, pty_*는 terminal owner이며 layout_move_tab_to_window는 composition root입니다.

## 등록 소유자

| owner | Specta 개수 | source |
| --- | ---: | --- |
| agent | 9 | src-tauri/src/domain/agent/commands.rs |
| ai | 8 | src-tauri/src/domain/ai/commands.rs |
| app | 5 | src-tauri/src/domain/app/commands.rs |
| composition root | 1 | src-tauri/src/lib.rs: layout_move_tab_to_window |
| file | 15 | src-tauri/src/domain/file/commands.rs |
| font | 1 | src-tauri/src/domain/font/commands.rs |
| git | 41 | src-tauri/src/domain/git/commands.rs |
| ide | 7 | src-tauri/src/domain/ide/commands.rs |
| layout | 19 | src-tauri/src/domain/layout/commands.rs |
| locale | 3 | src-tauri/src/domain/locale/commands.rs |
| lsp | 11 | src-tauri/src/domain/lsp/commands.rs |
| notification | 2 | src-tauri/src/domain/notification/commands.rs |
| plugin | 5 | src-tauri/src/domain/plugin/commands.rs |
| project | 25 | src-tauri/src/domain/project/commands.rs |
| remote | 5 | src-tauri/src/domain/remote/commands.rs |
| search | 4 | src-tauri/src/domain/search/commands.rs |
| settings | 3 | src-tauri/src/domain/settings/commands.rs |
| snippet | 3 | src-tauri/src/domain/snippet/commands.rs |
| sync | 5 | src-tauri/src/domain/sync/commands.rs |
| system | 7 | src-tauri/src/domain/system/commands.rs |
| task | 1 | src-tauri/src/domain/task/commands.rs |
| terminal | 10 | src-tauri/src/domain/terminal/commands.rs |
| theme | 5 | src-tauri/src/domain/theme/commands.rs |
| tree | 5 | src-tauri/src/domain/tree/commands.rs |
| vsix | 2 | src-tauri/src/domain/vsix/commands.rs |
| window | 1 | src-tauri/src/domain/window/commands.rs |
| 합계 | 203 | 25개 domain 및 composition root |

raw 등록은 terminal/commands.rs의 pty_spawn·pty_attach, file/commands.rs의 file_read_raw입니다. Specta 203개와 raw 3개는 별도 집합입니다. 전체 public command 등록 수는 206개입니다.

## 다음 판정에 영향을 주는 확인 결과

file action 15개는 Specta 14개와 raw file_read_raw 1개이며 file_flush_complete만 창 확인·exit adapter에 남습니다. settings 3개 및 비IPC 공통 apply는 이번 변경에서 runtime으로 옮겼습니다. 이 18개 command의 분리만으로 나머지 command body나 M6 전체를 완료 처리하지 않습니다.

search의 4개 command에는 프로젝트 조회·SearchStore 취소 identity·blocking 실행·파일별 mutation guard·self-write·UTF-8 목록 조립이 남습니다. search_replace는 blocking closure에서 AppHandle로 AppState를 다시 조회합니다. runtime에 이미 공유 AppState와 SearchStore가 있으므로 실제 앱을 실행하지 않고도 이 조립을 분리할 수 있습니다. Channel 전송과 perf span·진단 로그는 Tauri adapter에 남겨야 합니다.

tree의 5개 command에는 cache miss의 프로젝트 재확인·entry 재검사, prefetch 뒤 mutation guard와 TreeStore write lock, 인플레이스 수정 조립이 남습니다. 서비스와 TreeStore 분리만으로 이 action 경계가 이전된 것은 아닙니다. 네 개 기존 정책 unit도 commands.rs에 남아 있으므로 후속 이전에서는 함께 보존해야 합니다.

SettingsApplyPort·PluginRuntimePort·ProjectRestoreWatchers의 실제 AppHandle callback을 읽었습니다. settings callback 자체는 adapter에 남고 UI 비의존 apply는 runtime이 소비합니다. plugin 언어/설치 확정과 watcher build/register의 다른 소비처는 별도 body 판정 대상입니다. LSP 설치·infra LSP·PTY 작업 수명은 기존 잔여 경계 문서대로 미완료입니다.

이 문서는 등록 누락을 드러내는 조사 기록이지 runtime 분리 완료 검사나 전체 회귀 결과가 아닙니다. 앱 실행·설치·사용자 파일·시크릿에는 접근하지 않았습니다. 현재 활성 목표는 남아 있는 M 전체 완료이며 M7·M8의 선행 gate는 그대로 유지합니다.
