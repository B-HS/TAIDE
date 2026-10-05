# Native 전환 감사: 설정 · 테마 편집기 · 키바인딩 · 스니펫 · 플러그인 · 동기화 · 원격 · 시스템 사용량 · 토스트

감사일 2026-10-06. 읽기 전용 감사이며 빌드·테스트·앱 실행은 하지 않았습니다. 모든 판정은 실제 파일과 호출 체인 Grep 결과에 근거합니다. 이전 에이전트의 HANDOFF.md, PROCESS.md 수치는 사용하지 않았습니다.

## 1. 결론 요약

1. 설정 화면의 native 구현은 TS 13개 section(조건부 Performance 포함 14개) 중 8개(Appearance, Language, Interface, Notifications, Editor, Snippets, Terminal, Keymap)만 있습니다. `Section::BASIC`(`/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src/settings-controls.rs:30`)가 8개를 고정하며, 이전 문서의 "6~7개"가 아니라 8개가 사실입니다.
2. 있는 8개 section과 테마 편집기, 키바인딩 편집기, 스니펫 편집기, 시스템 사용량 모달, 토스트 엔진은 TS와 매우 정밀하게 대응하고 실제 앱 실행 경로(`application.rs` `tab_content`)에서 도달합니다.
3. 없는 6개 section(LSP, AI, Plugins, Sync, Remote, Performance)은 백엔드(`taide-runtime`의 `ai_actions`, `sync_actions`, `plugin_actions`, `remote_actions`, `lsp_actions::lsp_install`)가 대부분 이미 있으나 native UI 호출부가 0개입니다. 해당 명령은 원격 클라이언트에 대해 `command-policy.rs`가 의도적으로 거부하므로, 데스크톱 native UI 외에는 도달 경로가 없습니다. 즉 AI 토큰, GitHub 동기화 연결, 원격 접속 비밀번호·링크 발급, 플러그인 설치, LSP 설치, CLI 설치는 지금 사용자가 할 수 없습니다.
4. 설정 값은 저장되지만 소비처가 없는 경우가 많습니다. 알림 9종 스위치는 OS 알림 발송 호출부가 native에 없고, 에디터 표시 옵션 20여 종(줄바꿈, 미니맵, 커서, 룰러 등)은 native 에디터 표면이 읽지 않습니다.
5. 키바인딩 카탈로그는 TS 레지스트리를 정적 JSON(212개)으로 복사했으며 157개가 `monaco.*` id입니다. native 에디터에는 이를 실행하는 코드가 없고, 기본 키맵 41개 중 shell이 처리하는 것은 31개뿐입니다.

## 2. 범위

읽은 TS: `/Users/hyunseokbyun/development/TAIDE/src/widgets/{settings-view,theme-editor,keybindings-editor,snippet-editor,plugin-manager,system-usage-modal,app-toaster}` 전체 비테스트 파일, `/Users/hyunseokbyun/development/TAIDE/src/features/{settings,theme,plugin,snippet}` 주요 파일, `src/entities/notification/notify.ts`, `src/shared/lib/{numeric-field-commit.ts,vsix-theme-import.ts}`, `src/entities/theme/theme-tokens.ts`, `src/shared/lib/keymap/{keymap.ts,keymap-context.ts}`, inventory `2026-09-28-ts-settings-inventory.md`.

읽은 native: `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src/` 의 `settings-view.rs`, `settings-controls.rs`, `settings-code-controls.rs`, `settings-code-view.rs`, `settings-resources.rs`, `settings-owner.rs`, `theme-editor.rs`, `theme-edit.rs`, `theme-draft.rs`, `theme-color-picker.rs`, `theme-live-preview.rs`, `theme-editor-tokens.rs`, `keybinding-editor.rs`, `keybinding-catalog.rs`, `keymap.rs`, `snippet-editor.rs`(메시지 키 대조), `toast.rs`; `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/` 의 `settings-view.rs`, `settings-integrations.rs`, `system-usage.rs`, `system-usage-view.rs`, `keybinding-editor.rs`, `shell_keymap.rs`, `host.rs`, `application.rs`(settings, toast, keybindings, system_usage 호출부), `remote-preferences.rs`, `remote-plugins.rs`, `remote-sync.rs`, `bootstrap.rs`; 백엔드 `/Users/hyunseokbyun/development/TAIDE/crates/taide-runtime/src/{ai_actions,sync_actions,plugin_actions,vsix_actions,remote_actions,notification_actions,lsp_actions,agent_host}.rs`, `crates/taide-remote/src/command-policy.rs`, `crates/taide-model/src/settings.rs`.

## 3. 기능 대응표

native 근거 약어: `UI` = `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src`, `APP` = `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src`. 연결 체인(공통): `application.rs:4759 tab_content` → `Views::show(settings-view.rs:433)` → `Output` → `application.rs:4781-4809`(변경, 테마, 스니펫, 폴더, 키바인딩, settings.json) → `host.rs` `HostCommand::{UpdateSettings,ThemeEdit,SnippetEdit,OpenSettingsFolder}`.

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 설정 화면 루트, 스크롤, 로딩 placeholder, 제목 | `settings-view.tsx:155,176-185` | done | `UI/settings-view.rs:433-609,714-963` | 없음 | - |
| 2 | settings.json 열기 버튼 | `settings-view.tsx:131,181` | done | `UI/settings-view.rs:775-791`, `APP/application.rs:4803-4809` | 없음 | - |
| 3 | TOC 클릭 스크롤과 활성 표시 | `settings-view.tsx:78-92,133-140,189-196` | partial | `UI/settings-view.rs:808-834`, `settings-controls.rs:30` | TOC 항목이 8개뿐. LSP, AI, Plugins, Sync, Remote, Performance 항목 없음 | S(항목 추가 자체), 각 section 행 참조 |
| 4 | 테마/스니펫 편집기가 설정 화면 전체를 대체 | `settings-view.tsx:157-174` | done | `UI/settings-view.rs:531-579,591-607` | 없음 | - |
| 5 | 탭 소유권(보조창 포함 프로젝트별 Owner) | `settings-view.tsx:98-106` | done | `UI/settings-owner.rs:11-30`, `settings-view.rs:108-120` | 없음 | - |
| 6 | Appearance: 테마 선택 3그룹, 복제, 체크 | `settings-appearance-section.tsx`, `theme-picker.tsx` | done | `UI/settings-view.rs:965-1171` | 없음 | - |
| 7 | Appearance: 시스템 테마 추종 스위치 | `settings-appearance-section.tsx:50` | done | `UI/settings-view.rs:1173-1180` | 없음 | - |
| 8 | Appearance: 사용자 테마 헤더(폴더 열기, 새로 만들기) | 위 파일 56-76 | done | `UI/settings-view.rs:1181-1234` | 없음 | - |
| 9 | Appearance: 사용자 테마 목록(편집, 복제, 빈 상태) | `custom-theme-list.tsx` | done | `UI/settings-view.rs:1235-1310` | 없음 | - |
| 10 | Language: 언어 picker(시스템+locales, 키보드) | `language-picker.tsx` | done | `UI/settings-view.rs:1313-1500` | 없음 | - |
| 11 | Language: locales 폴더 열기 | `settings-language-section.tsx:38` | done | `UI/settings-view.rs:1502-1511`, `APP/host.rs:678-695` | 없음 | - |
| 12 | Interface: toast 위치 9칸 | `toast-position-picker.tsx` | done | `UI/settings-view.rs:1514-1582`, `settings-controls.rs:290-327` | 없음 | - |
| 13 | Interface: 분할 두께 숫자 필드(0-8) | `settings-interface-section.tsx:45-51` | done | `UI/settings-view.rs:1583-1591,1853-1901`, `settings-controls.rs:339-490` | 없음. blur 커밋과 클램프가 TS `resolveNumericFieldCommit`과 동일 | - |
| 14 | Interface: 스위치 9종(시스템 사용량, 미니맵, 상태 배지, diff 자동열기, 미리보기 탭, 탐색기 자동 reveal, 환영 화면, zen 전체화면, zen 상태바) UI | `settings-interface-section.tsx:52-117` | done | `UI/settings-view.rs:1592-1597`, `settings-controls.rs:56-195` | 없음(UI 기준). 소비처는 80번 | - |
| 15 | 시스템 사용량 표시 스위치가 상태바에 반영 | `settings-interface-section.tsx:52` | done | `APP/application.rs:3445-3455,5188-5197` | 없음 | - |
| 16 | Interface: agent CLI(`taide`) 설치/해제 행(macOS) | `agent-cli-status-row.tsx` | missing | 검색어: `agentCli`, `cliInstall`, `cliStatus`, `agent_cli_install`, `settings.cliInstall` 모두 native 비테스트 코드 0건. `crates/taide-runtime/src/agent_host.rs:100`은 `cli_install_status`(조회)만 있고 설치/해제는 `src-tauri/src/domain/agent/commands.rs:185,209`에만 있음 | UI, 설치/해제 구현(crates 이관) 전부 | M |
| 17 | Interface: agent hooks 토글, 에이전트 5종 목록, 프로젝트별 설치/해제, 동의 다이얼로그, CLI 누락 경고 | `settings-interface-section.tsx:68-76`, `agent-hooks-project-list.tsx`, `agent-hooks-project-row.tsx` | unwired | 설정값 변경 시 hooks 설치는 `APP/settings-integrations.rs:38-40`, `application-ports.rs:61-66`로 연결됨. 그러나 토글/목록/동의 UI는 없음. `agent_hook_actions::{agent_hooks_status,install,uninstall}`는 `APP/remote-agents.rs:138-164`(원격 디스패치)에서만 호출 | 토글 UI, 프로젝트별 행, 동의 다이얼로그, 사용자 수준 에이전트 4종 행, CLI 경고 | M |
| 18 | Interface: IDE 통합 스위치(설명 포함) | `settings-interface-section.tsx:77-82` | unwired | 변경 반영 경로 `APP/settings-integrations.rs:30-37`(`ide_server::apply_toggle`) 있음. `ideIntegration` 검색 0건, 스위치 enum `Switch`(20개)에 없음 | 스위치 1개와 locale 설명 | S |
| 19 | Interface: 검색 설정 그룹(searchOnType, debounce, 설명) | `settings-interface-section.tsx:118-136` | done | `UI/settings-view.rs:1598-1629` | 없음(UI). 소비처는 80번 | - |
| 20 | Notifications: 스위치 9종과 설명 UI | `settings-notification-section.tsx:67-114` | done | `UI/settings-view.rs:904-916`, `settings-controls.rs:153-170` | 없음(UI) | - |
| 21 | Notifications: 시스템 알림 설정 열기 버튼 | 위 파일 115-119 | missing | 검색어: `notificationsOpenSystemSettings`, `openNotificationSystemSettings`, `notification_open_system_settings` 모두 native 0건 | 버튼, 호스트 명령, 플랫폼 구현 | S |
| 22 | Notifications: 시험 알림 보내기와 suppressed 사유 toast | 위 파일 48-63,120-123 | missing | 검색어: `notificationsSendTest`, `sendNativeNotification`, `notification_notify` native UI 호출 0건 | 버튼, 사유별 warning toast | S |
| 23 | OS 알림 발송(agent 완료, 입력 대기, task 완료, git, 검색 치환, LSP 설치, 오류) | `entities/notification/notify.ts`, `app/providers/native-notification-provider.tsx:156-199`, `git.query.ts:222-231`, `search-panel-container.tsx:134` | unwired | `crates/taide-runtime/src/notification_actions.rs:12`와 `APP/bootstrap.rs:151`(osascript)은 있으나 `notification_notify`/`send_notification` 호출부가 native-app 비테스트 코드에 0건. 스위치 9종이 제어할 소비자가 없음 | 이벤트별 발송 호출부 전부, 창 포커스 게이트 연결 | L |
| 24 | Editor: 글꼴 크기, 글꼴 family picker(모노스페이스 필터, 검색, 시스템 기본, 미리보기) | `settings-editor-section.tsx:64-80`, `font-picker.tsx` | done | `UI/settings-code-view.rs:1237-1331`, `settings-resources.rs`, `APP/host.rs:654-670` | 없음 | - |
| 25 | Editor: 저장 시 동작(format, organizeImports, fixAll, trim, finalNewline) + editorConfig + codeLens 스위치 | `settings-editor-section.tsx:81-121` | done | `UI/settings-code-controls.rs:74-130`, 소비 `APP/application.rs:1399-1402,1480-1481` | 없음 | - |
| 26 | Editor: autoSave 지연+힌트, 탭 크기, 공백 삽입, 줄 번호 | 위 파일 122-153 | done | `UI/settings-code-view.rs:225`, 소비 `APP/application.rs:1436-1437,2482,5291-5292`, `editor_surface.rs:36,416` | 없음 | - |
| 27 | Editor: 표시·입력 옵션 UI(wordWrap, detectIndentation, renderWhitespace, bracket 2종, rulers+정규화, ligatures, cursor style/blinking/smooth caret, scrollBeyondLastLine, smoothScrolling, stickyScroll, semanticHighlighting, formatOnType/Paste, suggestPreview, emmet, diff 2종) | 위 파일 132-268 | done | `UI/settings-code-controls.rs:81-130,333-371,399-506`(룰러 파싱과 trim 규칙이 JS와 동일) | 없음(UI). 에디터 반영은 28번 | - |
| 28 | 위 표시·입력 옵션의 에디터 반영 | Monaco options | partial | 소비처 검색(`word_wrap`, `minimap`, `cursor_blinking`, `render_whitespace`, `ligatures`, `sticky_scroll`, `rulers`, `bracket_pair`, `semantic`, `code_lens`, `emmet`)이 `taide-native-*`, `taide-runtime` 등에서 0건. `editor_surface.rs:113`은 `layout_no_wrap` 사용 | wordWrap, minimap, renderWhitespace, cursor, rulers, ligatures, bracket, sticky, smooth, semantic, codeLens, formatOnType/Paste, suggestPreview, emmet, diff 옵션 전부. 에디터 영역 감사에서 처리할 본체 | XL |
| 29 | Snippets section(관리 버튼, 폴더 열기) | `settings-snippets-section.tsx` | done | `UI/settings-view.rs:1652-1702` | 없음 | - |
| 30 | Terminal section(글꼴 크기/family, shell, shell profile, scrollback+힌트, cursor, blink) | `settings-terminal-section.tsx` | done | `UI/settings-code-view.rs:120-170,415-526`, 소비 `APP/terminal_settings.rs`, `host.rs:671-674` | 없음 | - |
| 31 | Keymap section(편집기 열기) | `settings-keymap-section.tsx` | done | `UI/settings-view.rs:1704-1722`, `APP/application.rs:4164,4800-4802` | 없음 | - |
| 32 | LSP section: 서버 목록, 설치 여부, 경로, 버전, experimental 배지, 힌트 | `settings-lsp-section.tsx`, `lsp-server-status-list.tsx` | missing | 검색어: `lspStatus`, `lspInstall`, `lsp_servers`, `LspServerDetection`, `settings.lspDescription`. `detect_servers`는 `APP/lsp.rs:636`(시작용)과 `remote-lsp.rs:174`(원격)에서만 | UI 전부. 상태바 요약(`APP/lsp-status.rs`)만 별개로 있음 | M |
| 33 | LSP: 설치, 취소, 진행률(바이트), 실패 메시지, 설치 이벤트 동기화 | `lsp-server-status-list.tsx:37-102`, `settings-view.tsx:129` | unwired | `crates/taide-runtime/src/lsp_actions.rs:291,297`(`lsp_install`, `lsp_install_cancel`), `APP/remote-lsp.rs:182`(취소만 원격 허용). `lsp_install`은 `command-policy.rs`가 원격 거부. native UI 호출부 0 | 버튼, 호스트 명령, 진행 이벤트 구독 | M |
| 34 | LSP: 설치 명령 복사, 툴체인/SDK/체크섬 힌트 | `lsp-server-status-list.tsx:104-170` | missing | 위 검색어와 `lspCopyCommand`, `lspToolchainMissing` 0건 | 힌트 분기, 클립보드 복사 | S |
| 35 | AI: provider 토큰 행(ollamaCloud, codex 경고) 저장/해제/상태 | `settings-ai-section.tsx:67-82`, `ai-provider-token-row.tsx` | unwired | `ai_actions::{ai_token_status,ai_set_token,ai_clear_token}`(`crates/taide-runtime/src/ai_actions.rs:30-42`). native는 `remote-ai.rs:44`에서 `ai_token_status`만 원격에 노출, set/clear는 원격 거부. UI 0 | UI, 호스트 명령, 시크릿 비노출 입력 | M |
| 36 | AI: oMLX base URL, API key | `ai-omlx-row.tsx` | unwired | `settings.ai_omlx_base_url` 필드는 `taide-model/src/settings.rs:286`에 있음. UI 0 | UI | S |
| 37 | AI: provider 선택, 모델 목록 로딩/오류/빈 상태, 모델 선택 | `settings-ai-section.tsx:91-113` | unwired | `ai_actions::ai_list_models`는 원격 디스패치만(`remote-ai.rs:46`). UI 0 | 비동기 모델 조회, 상태 분기, 선택 | M |
| 38 | AI: auto-tab 토글(provider 미설정 시 비활성) | `ai-auto-tab-toggle.tsx` | unwired | `settings.ai_auto_tab_enabled`(`settings.rs:263`) 소비처 native 0. UI 0 | 토글, 에디터 inline 완성 연결(별도 영역) | S |
| 39 | AI: 프롬프트 파일 열기 3종(app file 탭) | `settings-ai-section.tsx:119-130` | unwired | `AppFileTarget::Prompt`는 모델에 있음(`crates/taide-model/src/app.rs:83`). native는 `APP/app-file-views.rs:317 settings_command`만 있고 prompt 명령 없음 | UI 3행, prompt 탭 열기 명령 | S |
| 40 | Plugins: 플러그인 목록(이름, 버전, 활성, 오류 코드 7종) | `plugin-manager.tsx`, `plugin-list-body.tsx` | unwired | `plugin_actions::plugin_list`는 `APP/remote-plugins.rs:32`(원격)만. UI 0 | 목록 UI, 오류 메시지 매핑 | M |
| 41 | Plugins: .zip 설치(파일 선택, 성공/실패 toast) | `plugin-install-button.tsx` | unwired | `plugin_actions::plugin_install`(`plugin_actions.rs:24`)는 원격 거부. `rfd`는 `APP/Cargo.toml:65`에 있어 파일 선택 가능. UI 0 | 버튼, 호스트 명령 | M |
| 42 | Plugins: VSIX 가져오기 다이얼로그(테마 후보 선택, 중복 덮어쓰기 확인, 대비 실패 표시, 문법 가져오기 3상태) | `vsix-import-dialog.tsx`, `vsix-import-grammars-section.tsx`, `shared/lib/vsix-theme-import.ts`, `shared/lib/theme-convert/*`(약 4천 줄) | missing | 검색어: `vsix`(native 전체 0건), `pluginImportVsix`, `theme-convert`, `buildVsixThemeCandidates`. `crates/taide-vsix`는 압축 해제/추출만(`service.rs:101,452`). VS Code 테마 변환(`convert.ts`, 매핑 표, 대비 검사)은 Rust 이식 없음 | 변환 엔진 Rust 이식, 후보 선택 UI, 문법 가져오기 UI | XL |
| 43 | Plugins: 새로고침, 플러그인 폴더 열기 | `plugin-manager.tsx:60-67` | unwired | `plugin_reload` 원격 디스패치만. `AppDataPathKind::Plugins` 경로는 system_actions가 처리 가능하나 버튼 없음 | 버튼 2개 | S |
| 44 | Plugins: 제거 확인 다이얼로그 | `plugin-uninstall-dialog.tsx` | unwired | `plugin_actions::plugin_uninstall`(`plugin_actions.rs:55`) 원격 거부. UI 0 | 다이얼로그, 호스트 명령 | S |
| 45 | Sync: GitHub PAT 연결/해제 | `sync-section.tsx:45-66,97-100` | unwired | `sync_actions::{sync_connect,sync_disconnect}`(`sync_actions.rs:126,168`)는 `remote-sync.rs:14` COMMANDS에 없음(원격 거부). native UI 호출부 0 | 입력 UI, 호스트 명령 | M |
| 46 | Sync: 상태(gist id, 마지막 동기화, remoteNewer 배지), 업로드, 다운로드 | `sync-section.tsx:68-96` | unwired | `sync_status/upload/download`는 `remote-sync.rs`로 원격에만 | UI, 호스트 명령, 성공/실패 toast | M |
| 47 | Sync: 충돌 다이얼로그(로컬 유지/원격 당김) | `sync-conflict-dialog.tsx` | unwired | `prepare_sync_download`/`apply_sync_download`(`sync_actions.rs:254,280`)가 충돌 분기를 위해 존재. UI 0 | 다이얼로그, 2단계 흐름 | S |
| 48 | Sync: 설정 변경 후 동기화 데이터 반영 | `settings-sync-section.tsx` | unwired | 위와 동일 | 위와 동일 | - |
| 49 | Remote: 서버 활성 토글(설명) | `remote-section.tsx:53-59` | unwired | 토글 값 변경 시 서버 시작/종료는 `APP/settings-integrations.rs:41-66`로 연결되어 있으나 스위치 UI 없음(settings.json 직접 편집만 가능) | 스위치 | S |
| 50 | Remote: 실행 상태, 포트, 클라이언트 수, 보안 경고 | `remote-section.tsx:60-73` | unwired | `remote_actions::remote_status`는 원격 디스패치만(`remote-preferences.rs:238`). UI 0 | 상태 행, 경고 | S |
| 51 | Remote: 비밀번호 설정/해제, 최소 길이, password-only 로그인 토글 | `remote-password-row.tsx`, `remote-section.tsx:74-89` | unwired | `remote_actions::{remote_set_password,remote_clear_password}`(`remote_actions.rs:46,58`) 있음. 원격은 `SelfAccessExpansion`으로 거부. 호출부 0 | UI, 호스트 명령 | M |
| 52 | Remote: 허용 호스트 목록(추가/삭제, RFC 1035 검증, 중복) | `remote-allowed-hosts-row.tsx` | missing | 검색어: `allowed_hosts`, `allowedHosts`, `remote.allowedHosts*`는 native UI 0건. 검증 규칙은 TS 쪽에만 있고 Rust 설정 서비스에 동일 규칙이 있다고 TS 주석이 명시 | UI, 입력 검증 | S |
| 53 | Remote: 접속 링크 발급과 클립보드 복사, 세션 폐기 | `settings-remote-section.tsx:45-56`, `remote-section.tsx:91-100` | unwired | `remote_issue_link`(`remote_actions.rs:21`)는 원격 거부, native 호출부 0. `remote_revoke_sessions`는 원격 디스패치만. 결과적으로 원격 서버를 켜도 로그인 링크를 얻을 방법이 없음 | 버튼 2개, 호스트 명령, 복사 toast | M |
| 54 | Performance section(개발 게이트): Rust perf snapshot 읽기/초기화 | `settings-performance-section.tsx` | missing | 검색어: `perf_snapshot`, `perf_reset`, `performanceRead`, `PerfSnapshot`. native 0건(`crates/taide-infra/src/perf.rs`, `taide-app`에는 있음). 프런트엔드(JS)/Monaco 부분은 n/a | native 쪽 읽기/초기화 UI | S(개발용 우선순위 낮음) |
| 55 | 테마 편집기: 헤더(뒤로, 이름 입력, 변경 개수, 저장, 삭제) | `theme-editor.tsx:171-199` | done | `UI/theme-editor.rs:297-366`, 저장/삭제 `UI/theme-edit.rs:203-356` | 없음 | - |
| 56 | 테마 편집기: 토큰 검색, namespace별 section | 위 파일 203-234 | done | `UI/theme-editor.rs:396-450`, 토큰 목록 `theme-editor-tokens.rs` | 없음(토큰 목록이 TS와 동일한 namespace 구조) | - |
| 57 | 테마 편집기: syntax 행(bold, italic, 색, 리셋) | `syntax-token-row.tsx` | done | `UI/theme-editor.rs:451-473,739,832-839` | 없음 | - |
| 58 | 테마 편집기: terminal 색 행 | `theme-editor.tsx:251-264` | done | `UI/theme-editor.rs:474-496` | 없음 | - |
| 59 | 테마 편집기: 컬러 피커(SV, Hue, hex 입력, transparent, 키보드, 드래그 커밋) | `color-picker.tsx` | done | `UI/theme-color-picker.rs:164-534` | 없음 | - |
| 60 | 테마 편집기: 라이브 프리뷰 패널과 앱 전체 실시간 미리보기 | `theme-live-preview.tsx`, `useThemePreview` | done | `UI/theme-live-preview.rs`, `UI/settings-view.rs:547-558`, `APP/application.rs:3865 sync_theme_preview` | 그룹 접근성 이름(`themeEditor.previewSyntaxTitle`, `previewAnsiTitle`) 미사용(사소) | S |
| 61 | 테마 편집기: 미저장 폐기 확인, 삭제 확인, 삭제 시 활성 테마를 같은 타입 builtin으로 대체 | `theme-editor.tsx:117-156,275-303` | done | `UI/theme-editor.rs:589-606`, `UI/theme-edit.rs:306-356` | 없음 | - |
| 62 | 키바인딩 편집기: 열기/닫기, 포커스 복원, Escape 처리 | `keybindings-editor.tsx:75-78,162-167` | done | `UI/keybinding-editor.rs:531-582`, `APP/application.rs:4164,4259` | 없음 | - |
| 63 | 키바인딩 편집기: 텍스트 퍼지 검색, 키 입력 검색 모드 | 위 파일 85-92,151-184 | done | `UI/keybinding-editor.rs:584-621,862-990`, `keybinding-search.rs` | 없음 | - |
| 64 | 키바인딩 편집기: 충돌/미할당 필터와 개수, context inspector, 컬럼 헤더 | 위 파일 93-97,203-213,248-286 | done | `UI/keybinding-editor.rs:993-1110,542-559` | inspector의 context key가 `editorTextFocus`, `terminalFocus` 두 개(TS `DEFAULT_KEYMAP_CONTEXT_GETTERS`와 동일) | - |
| 65 | 키바인딩 편집기: 행 캡처(단일, chord 2단계), 충돌 경고 toast, 초기화, 해제, 충돌 해결 | `keybinding-row.tsx`, `keybindings-editor.tsx:104-149` | done | `UI/keybinding-editor.rs:623-641,1213-1548`, `keybinding-capture.rs`, `APP/application.rs:4271-4275` | 없음 | - |
| 66 | 키바인딩 명령 카탈로그(`listRegisteredCommands` 대응) | `command-registry.ts` | partial | 정적 `UI/keybinding-commands.json`(212개)을 `include_str!`(`keybinding-catalog.rs:6`). 그중 157개가 `monaco.*`. 동적 레지스트리가 아니므로 native에서 새로 생기는 명령이 자동 반영되지 않음 | native 명령 레지스트리, monaco.* 제거 또는 native 에디터 액션으로 대체 | M |
| 67 | 편집한 키가 실제 동작에 반영 | `keymap.ts`, 명령 실행기 | partial | `APP/shell_keymap.rs:6 ACTIONS`(31개)만 `action()`으로 dispatch(`application.rs:3935`). 기본 키맵 41개 중 `command-palette`, `quick-open`, `find`, `search`, `search-replace`, `workspace-symbol`, `explorer`, `git`, `terminal-jump-*`(터미널 표면 전용) 등은 shell에서 처리되지 않음. `monaco.*` override는 `keymap.rs:455`에서 chord prefix 등록만 하고 실행 코드 없음(`cursorUndo`, `actions.find` 검색 0건) | 미지원 명령의 실행기. 다른 영역(팔레트, 검색, 에디터) 의존 | XL |
| 68 | 스니펫 편집기: 파일 목록, 새 파일 다이얼로그(전역/언어) | `snippet-editor.tsx`, `new-snippet-file-dialog.tsx` | done | `UI/snippet-editor.rs`, 메시지 키 `snippetEditor.*` 36개가 TS 사용 키와 일치(오류 키는 `toast.rs:446-481`) | 없음 | - |
| 69 | 스니펫 편집기: 항목 편집(name, prefix, body, description, scope) | `snippet-entry-editor.tsx` | done | `UI/snippet-editor.rs`, `snippet-draft.rs` | 없음 | - |
| 70 | 스니펫 편집기: 저장 검증(미완성, 중복 이름)과 성공/실패 toast | `snippet-editor.tsx:110-138` | done | `UI/snippet-editor-state.rs`, `toast.rs:446-481`, `APP/application.rs:3873-3875` | 없음 | - |
| 71 | 스니펫 편집기: 파일 삭제, 항목 삭제 확인 | 위 파일 140-154,226-273 | done | `UI/snippet-editor.rs`, `host.rs:616` | 없음 | - |
| 72 | 스니펫 편집기: 미저장 변경 폐기 확인(파일 전환, 닫기) | 위 파일 70-101 | done | `UI/snippet-editor.rs`(`common.unsavedChanges*`, `common.discardChanges` 사용) | 없음 | - |
| 73 | 시스템 사용량: 상태바 요약과 3초 폴링 | `entities/system` | done | `APP/system-usage.rs:17-165`, `system-usage-view.rs:158-225`, `application.rs:3445-3455,5188` | 없음 | - |
| 74 | 시스템 사용량: 상세 모달(5종 그룹, CPU %, MB, 빈 상태, 열려 있을 때만 조회) | `system-usage-modal.tsx` | done | `APP/system-usage-view.rs:45-53,295-400`, `system-usage.rs:134-146`, `application.rs:3853-3863` | 없음 | - |
| 75 | 토스트: 외형, 4초 수명, 최대 3개, hover 확장, 스와이프, 닫기 버튼, 라이트/다크 색 | `app-toaster.tsx`(sonner) | done | `UI/toast.rs:1-140,591-721`, `toast-motion.rs`, `toast-swipe.rs`, `APP/application.rs:3361-3368,4365` | 없음 | - |
| 76 | 토스트: 위치 9종 설정 반영 | `app-toaster.tsx:11-17` | done | `UI/toast.rs:363`, `APP/application.rs:3358-3367` | 없음 | - |
| 77 | 토스트 발행 API(success/error/warning/info 일반) | sonner `toast.*` 전역 호출 | partial | 공개 API는 `warning(title)`, `snippet`, `settings_failed`, `ipc_error`, `app_file_failed` 5개(`toast.rs:442-510`). 임의 성공/오류 문구를 띄우는 진입점이 없음 | 일반 `success`, `error`, `warning`(+description) API | S |
| 78 | 오류 표시 경로(TS는 `toast.error`) | 전역 | partial | `APP/application.rs`에서 `self.status = Some(error.to_string())`가 100곳, `HostReply::Failed`도 상태 문자열(`application.rs:1353`). toast는 설정/스니펫/app file/ipc 일부만 | 오류를 toast로 통일하는 라우팅 | M |
| 79 | 카탈로그 비동기 로딩(테마, locale, 글꼴, shell profile) | `font.query`, `locale.query`, `theme.query` | done | `UI/settings-view.rs:98-121`, `settings-resources.rs:200-266`, `APP/host.rs:640-676` | 없음 | - |
| 80 | Interface 설정값의 소비처(탐색기 자동 reveal, 환영 화면, agent 상태 배지, IDE diff 자동 열기, searchOnType, debounce, minimap) | 각 TS 소비 모듈 | partial | `explorer_auto_reveal`, `welcome_on_empty_editor`, `agent_status_badge_enabled`, `ide_auto_open_diff`, `search_on_type`, `search_on_type_debounce_ms`, `editor_minimap` 소비처 검색 결과 `taide-native-*`, `crates` 모두 0건. 소비되는 것은 `enable_preview_tabs`, `zen_*`, `resizer_thickness`, `show_system_usage` | 해당 기능 자체(다른 영역 감사 대상)와 설정 연결 | L |
| n/a-1 | `lazy`/`Suspense` 코드 분할(테마/스니펫 편집기, 플러그인 관리자) | `settings-view.tsx:45-46`, `settings-plugins-section.tsx:15` | n/a | 번들 분할은 웹 기술 전용 | - | - |
| n/a-2 | `settingsNotificationsDevHint`(DEV 빌드 안내) | `settings-notification-section.tsx:125` | n/a | `import.meta.env.DEV` 전용 안내 | - | - |
| n/a-3 | Monaco 키코드 바인딩 가능 여부(`isKeyBindable`, `resolveMonacoKeyCode`)와 Radix Dialog 포커스 트랩 | `keybindings-editor.tsx:102`, `dialog` | n/a | 대응 로직은 `keybinding-capture.rs:145-162 editor_key`, egui modal이 담당. Monaco 자체는 native에 없음 | - | - |

집계: 총 83행. done 44, partial 7, unwired 21, missing 8, n/a 3.

## 4. 잘못 구현되었거나 보강이 필요한 native 코드

1. 설정 화면이 8개 section을 하드코딩합니다. `Section` enum과 `BASIC`(`UI/settings-controls.rs:18-39`), `Views::show`의 `match section`(`UI/settings-view.rs:891-938`)이 8개만 다룹니다. 이름 `BASIC`이 미완성 상태를 시사하며, 나머지 6개 section은 비동기 데이터를 가진 위젯이라 `Catalog`(`settings-view.rs:92-95`)와 `settings_resources::Kind`(`settings-resources.rs:15-18`, 글꼴/shell 2종)에 종류를 추가하는 구조 확장이 필요합니다. 근거: `settings-controls.rs:30`.
2. 알림 스위치가 아무것도 제어하지 않습니다. `notification_notify`(`crates/taide-runtime/src/notification_actions.rs:12`)와 `APP/bootstrap.rs:151`의 macOS 전용 osascript 구현은 있으나 호출부가 0이라, 사용자가 켜고 끄는 9종 설정이 무의미합니다. 비 macOS는 `Err("native notification platform is not connected")`(`bootstrap.rs:152-155`)로 고정 실패합니다.
3. 에디터 설정 UI와 에디터 표면의 불일치. Editor section은 TS와 필드 33개가 모두 일치하지만, native 에디터는 글꼴 크기, 줄 번호, 탭 크기, 공백 삽입, 저장 시 동작만 읽습니다(`APP/application.rs:5167,5291-5292`, `UI/editor_surface.rs:36,416`). 나머지는 값이 저장되기만 하는 장식 컨트롤입니다. 줄바꿈은 `layout_no_wrap`(`editor_surface.rs:113`)로 구조적으로 불가능합니다.
4. 키바인딩 카탈로그가 Monaco 의존입니다. `keybinding-commands.json` 212개 중 157개가 `monaco.*`이고, `keybinding-catalog.rs:259-273`이 `is_monaco`로 분기하지만 이를 실행하는 native 코드가 없어 사용자가 재바인딩해도 동작하지 않습니다. 또한 `shell_keymap::ACTIONS`(`APP/shell_keymap.rs:6`)는 31개라 기본 키맵 41개 중 10개가 편집기에 표시되지만 동작하지 않습니다.
5. 원격 접근이 사실상 사용 불가합니다. 서버 토글은 `settings-integrations.rs`로 연결되어 있으나 비밀번호 설정, 링크 발급 UI가 없고(`remote_issue_link`, `remote_set_password` 호출부 0), 이 명령들은 원격 클라이언트에 `SelfAccessExpansion`으로 거부됩니다. 서버를 켜도 로그인 수단이 없습니다.
6. agent CLI 설치/해제가 crates로 이관되지 않았습니다. 구현이 `src-tauri/src/domain/agent/commands.rs:185,209`에만 있고 `crates/taide-runtime`에는 `cli_install_status`뿐입니다(`agent_host.rs:100`). TypeScript 제거와 Tauri 제거를 함께 진행하려면 먼저 이관해야 합니다.
7. VS Code 테마 변환 엔진이 Rust에 없습니다. `src/shared/lib/theme-convert/` 약 4천 줄(변환, 매핑 표, 대비 검사, 상태 구분성 검사)이 TS에만 있고 `crates/taide-theme`, `crates/taide-vsix`에는 추출만 있습니다. VSIX 가져오기 UI 이전에 선행 이식이 필요합니다.
8. 오류 표시가 TS와 다릅니다. TS는 모든 실패를 `toast.error(describeIpcError(...))`로 보여주는데 native는 `self.status`(100곳)에 문자열로 남깁니다(`APP/application.rs:1353` 등). `Toasts`의 공개 API가 5개로 닫혀 있어(`UI/toast.rs:442-510`) 새 section의 성공/실패 피드백(예: `settings.syncUploadSuccess`)을 낼 진입점이 없습니다.
9. 접근성 이름 누락(사소). 테마 라이브 프리뷰 그룹의 `previewSyntaxTitle`, `previewAnsiTitle`이 사용되지 않습니다(`UI/theme-live-preview.rs`에 키 없음).
10. 브라우저(Wasm) 설정 화면도 동일하게 8 section입니다. `taide-remote-web/src/browser-editor.rs:48`이 `taide_native_ui::settings_view::Views`를 그대로 공유하므로, 6개 section 구현 시 원격 거부 명령(AI 토큰, Sync 연결, 플러그인 설치, LSP 설치, perf 등)의 브라우저 노출 방식(숨김 또는 거부 표시)을 함께 정해야 합니다.

## 5. 실제 앱 연결이 끊긴 지점

1. `crates/taide-runtime/src/ai_actions.rs` `ai_set_token`, `ai_clear_token`: native 호출부 0.
2. `sync_actions.rs` `sync_connect`, `sync_disconnect`: 원격 COMMANDS에도 native UI에도 없음. `sync_status/upload/download`는 `APP/remote-sync.rs`로 원격 클라이언트에만 연결.
3. `plugin_actions.rs` `plugin_install`, `plugin_uninstall`: 호출부 0. `plugin_list/reload/read_grammar`는 `APP/remote-plugins.rs`로 원격에만 연결.
4. `vsix_actions.rs` `vsix_extract_themes`, `vsix_import_plugin`: `vsix` 검색 native 0건.
5. `remote_actions.rs` `remote_issue_link`, `remote_set_password`, `remote_clear_password`: 호출부 0. `remote_status`, `remote_revoke_sessions`는 원격에만.
6. `lsp_actions.rs` `lsp_install`: 호출부 0(`application.rs:4509`는 종료 시 drain만). `lsp_install_cancel`은 원격에만.
7. `notification_actions.rs` `notification_notify`와 `notification_open_system_settings`: 호출부 0. `PlatformServices::send_notification`은 구현만 있음.
8. `agent_hook_actions`의 설치/해제는 `APP/remote-agents.rs`에만. 로컬은 설정값 reconcile(`settings-integrations.rs:38`)로만 호출되고 UI 스위치가 없음.
9. `Settings.ide_integration_enabled`, `agent_hooks_enabled`, `remote_access_enabled`는 reconcile 경로가 살아 있으나 조작 UI가 없어 settings.json 직접 편집으로만 바꿀 수 있음.
10. `AppFileTarget::Prompt`는 모델에만 있고 native 탭 열기 명령은 `settings_command`뿐(`APP/app-file-views.rs:317`).

## 6. 권장 구현 순서와 의존 관계

1. 공통 기반(선행): `Toasts` 일반 API(`success`, `error`, `warning`, description)와 오류 toast 라우팅(S-M). 이후 모든 section이 사용합니다.
2. `Section` 확장 구조: `Section`/`Catalog`/`Resources`에 비동기 종류 추가, TOC 항목 확장, 브라우저 노출 정책(숨김/비활성) 결정(M). 3~8의 선행입니다.
3. Remote section(M): 토글, 상태, 비밀번호, password-only, 허용 호스트, 링크 발급/복사, 세션 폐기. 현재 가장 기능이 막혀 있고 백엔드가 모두 있습니다. 클립보드 복사 경로 필요.
4. Sync section(M): 연결/해제, 업로드/다운로드, 충돌 다이얼로그. 백엔드 `sync_actions`와 `create_client` 포트 재사용(`remote-sync.rs`).
5. AI section(M-L): 토큰 행, oMLX, provider/model, auto-tab, prompt 탭 열기. prompt 탭 명령과 `AppFileTarget::Prompt` 뷰(`app-file-views.rs`) 필요. auto-tab 소비는 에디터 영역.
6. LSP section(M): 목록, 설치/취소/진행률 이벤트 구독(`event-relay.rs` 확인 필요), 명령 복사.
7. Interface 보강(M): IDE 통합 스위치(S), agent hooks 토글과 프로젝트별 행/동의 다이얼로그(M), agent CLI 설치/해제(crates 이관 후 UI, M).
8. Notifications 보강: 시스템 알림 설정 열기, 시험 알림 버튼(S) 후 이벤트 발송 연결(L, 에이전트, git, 검색, LSP, task 이벤트 소비처와 함께).
9. Plugins section(L): 목록, zip 설치(`rfd` 사용), 제거, 새로고침, 폴더 열기. 이후 VSIX 가져오기(XL)는 `theme-convert` Rust 이식(별도 작업)이 선행되어야 합니다.
10. 키바인딩 정합(XL, 다른 영역 의존): 카탈로그를 native 명령 레지스트리로 교체하고 `monaco.*` 정리, 미지원 키맵 10개를 팔레트, 검색, 탐색기, git 영역 구현과 함께 연결.
11. Performance section(S, 개발용): 마지막.
12. 에디터 표시 옵션(XL)은 에디터 영역 감사 결과와 합쳐서 진행.

## 7. 확인하지 못한 것(불확실성)

1. 앱을 실행하거나 빌드/테스트를 돌리지 않았습니다. "done"은 코드 대조와 호출 체인 Grep 기준이며 실제 화면 동작은 미검증입니다.
2. 설정 값 소비처 검색은 `rg`로 필드명을 찾은 결과입니다(`taide-native-*`, `taide-runtime` 등 crates, 별칭 필드 이름은 일부 놓칠 수 있음). 28번, 80번 판정은 이 한계를 가집니다. 에디터 영역 감사와 교차 확인이 필요합니다.
3. 키바인딩 카탈로그 212개가 TS `listRegisteredCommands()` 실제 목록과 id 단위로 일치하는지는 전수 비교하지 않았습니다. 기본 키맵은 TS 41개와 native `keymap-defaults.json` 41개로 개수만 일치함을 확인했습니다.
4. 테마 토큰 목록(`COLOR_NAMESPACES`, `SYNTAX_TOKENS`, `TERMINAL_TOKENS`)은 구조와 앞부분만 대조했고 전체 개수 일치는 전수 비교하지 않았습니다.
5. 스니펫 편집기와 테마 편집기의 세부 픽셀, 키보드 순서, IME 동작은 읽지 않았습니다(메시지 키와 핵심 흐름 위주).
6. `enable_preview_tabs`가 `terminal_tabs.rs` 외 `taide-runtime`에서 소비되는지는 부분 확인만 했습니다.
7. native 알림 수신 기능을 다른 경로(예: 다른 crate의 이벤트 구독)로 구현했을 가능성은 `send_notification`, `notification_notify`, `NotificationCategory` 전체 검색으로 배제했으나, 문자열 동적 호출은 확인하지 못했습니다.
8. missing 판정에 사용한 검색어: 16번(`agentCli`, `cliInstall`, `cliStatus`, `agent_cli_install`, `settings.cliInstall`), 21-22번(`notificationsOpenSystemSettings`, `openNotificationSystemSettings`, `notification_open_system_settings`, `notificationsSendTest`, `sendNativeNotification`, `notification_notify`), 32번(`lspStatus`, `lspInstall`, `lsp_servers`, `LspServerDetection`, `settings.lspDescription`), 34번(`lspCopyCommand`, `lspToolchainMissing`), 42번(`vsix`, `pluginImportVsix`, `theme-convert`, `buildVsixThemeCandidates`), 52번(`allowed_hosts`, `allowedHosts`, `remote.allowedHosts`), 54번(`perf_snapshot`, `perf_reset`, `performanceRead`, `PerfSnapshot`). 모든 검색은 `/Users/hyunseokbyun/development/TAIDE/native` 전체(vendor 포함, target 제외)를 대상으로 했고, 일부는 `crates` 전체도 포함했습니다.
