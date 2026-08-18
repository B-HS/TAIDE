# 아키텍처 감사 T0(즉시 수정) 24항목 — 수정 요지

> 계약: `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md`. 감사 보고서 정본:
> `docs/quality-assurance/2026-08-18-architecture-audit.md` §6.1(T0 24항목)·§9(세부 결정).
> 이 문서는 24항목 각각의 증상·원인·수정을 한곳에 모은 **요지**다 — IPC 표면 변경의 정본은
> `docs/ipc-contract.md`(§3 "원격 dispatch 정책"·"T0 감사 데이터·기능 수정"), 영속 스키마 영향의
> 정본은 `docs/data-model.md` §13, 기능별 상세는 `features/git.md`·`features/terminal.md`·
> `features/lsp.md` 각 해당 절이다.

## 개요

감사(16배치 297발견, `2026-08-18-architecture-audit.md`)가 T0(즉시 수정 — 보안·데이터손실·기능무력화)
로 티어링한 24항목을 Phase R1(보안 5클러스터)·R2(데이터·기능 10항목)·F1~F4(프론트 접합 9항목) 3트랙
병렬 구현 후, Phase D(이 문서를 쓴 세션)가 전체를 통합·문서화·검증했다. 아래는 항목 번호(보고서 §6.1
기준) 순으로 증상 한 줄 + 원인 한 줄 + 수정 한 줄만 요약한다 — 상세 근거(파일:라인)는 계약 §2 를 본다.

## §2.1 보안·신뢰 경계 (Rust dispatch, Phase R1)

| # | 증상 | 원인 | 수정 |
|---|------|------|------|
| 12 | 원격 세션이 `agent_cli_install`/`agent_cli_uninstall` 을 그대로 호출할 수 있었다 | macOS 에서 이 핸들러가 관리자 권한 프롬프트(`osascript`)를 데스크톱 자신의 화면에 띄우거나 `/usr/local/bin` 에 직접 심링크를 건다 — 원격은 그 창을 보거나 답할 방법이 없다 | `dispatch.rs` 에 `deny_remote_agent_cli` 공용 헬퍼 + match arm 명시 거부(파리티는 `IMPLEMENTED_JSON_COMMANDS` 유지로 보존) |
| 13 | 원격 세션이 `agent_hooks_install` 로 `~/.codex/hooks.json`/`~/.gemini/settings.json` 에 커맨드 훅을 심을 수 있었다 | User 스코프 훅은 프로젝트 루트 가드 밖(홈 디렉터리)에 TAIDE CLI 가 모든 훅 이벤트마다 실행하는 셸 커맨드를 심는 것과 같다 — `settings_update` 가 스트립하는 `shellOverride` 와 동일한 백도어 성격 | `agentName` → `hook_scope_for_agent` 스코프 판정 후 `HookInstallScope::User` 만 거부(`Project` 는 그대로 허용) |
| 14 | 원격 세션이 데스크톱 CLI(`taide open --wait`)의 파일-열기 요청을 가로채 waitMarker 를 자기 realm 에 등록, 외부 CLI 프로세스를 앱 종료까지 블록할 수 있었다 | `AgentExternalOpen` 이벤트가 `fanout_remote_events!` 로 원격에도 나갔고, `agent_pending_external_opens` 큐는 세션 구분 없는 단일 큐라 먼저 드레인한 쪽이 통째로 비웠다 | `lib.rs` 의 `fanout_remote_events!` 에서 `AgentExternalOpen` 제거 + `agent_pending_external_opens` match arm 명시 거부 |
| 15 | 프로젝트를 닫아도 webview 의 `asset://` 스코프가 그 루트를 계속 읽을 수 있었다 | `project_close` 가 `asset_protocol_scope().forbid_directory` 를 호출한 적이 없었다(0건) | ~~`project_close` 에 `forbid_directory(root)` 추가~~ — **어드버서리얼 검증에서 되돌림**: Tauri 2.11.5 `fs::Scope` 는 forbidden 패턴을 제거하는 API 가 없어(add-only) 같은 프로젝트를 닫았다 재열면 재열기의 `allow_directory` 가 무력화되고 영구히 asset 접근이 막힌다. 상세는 `docs/bug/2026-08-18-audit-t0-adversarial-fixes.md` §1 |
| 16 | 원격 세션이 `lsp_install` 로 수백MB 언어서버 아카이브를 데스크톱에 내려받고 인스톨러를 spawn 할 수 있었다 | `plugin_install`/`vsix_import_plugin` 과 동일 계열의 임의 다운로드·실행 표면인데 거부 목록에서만 빠져 있었다 | `deny_remote_lsp_install` + match arm 거부(사용자 결정 6) |

## §2.2 데이터 손실·손상 (Phase R2)

| # | 증상 | 원인 | 수정 |
|---|------|------|------|
| 17 | `search_replace` 가 프로젝트 전역 파일을 무가드로 덮어썼다 | 뮤테이션 가드(`begin_mutation`) 자체를 잡지 않았다 | 대상 파일 목록은 `spawn_blocking` 으로 한 번 계산(락 밖), 실제 치환은 **파일마다** `begin_mutation` 재획득(전역 락 장기 점유 회피 — 권장안 채택) |
| 19 | `file_save`/`search_replace` 가 실행 스크립트(`chmod +x`)를 재작성하면 실행 비트가 사라졌다 | `write_atomic` 의 create-temp→rename 이 새 임시 파일에 프로세스 umask 기본 모드만 적용했다 | `write_atomic_preserving_mode` 신설(기존 파일 mode 조회 후 그대로 적용, 신규 파일/non-unix 는 기존 동작 폴백) — `save_file`·`replace_in_file` 둘 다 교체, 회귀 테스트 2건 |
| 20 | 자식 프로세스가 stdin 을 읽지 않으면 `pty_write` 가 블로킹되며 `TerminalStore` 전체 락을 붙든 채 앱 전체 터미널 기능이 정지했다 | `pty_write` 가 store 락을 쥔 채 블로킹 write 를 수행 | `PtySession.writer` 를 `Arc<Mutex<Writer>>` 로 바꿔 `writer_handle()` 로 clone 가능하게 하고, `pty_write` 는 store 락 밖에서 그 handle 로만 쓴다(같은 writer, 락 범위만 축소) |

## §2.3 기능 무력화 (Phase R2 + Phase D 접합부)

| # | 증상 | 원인 | 수정 |
|---|------|------|------|
| 21 | 프로젝트를 닫거나 터미널 탭을 닫아도 pty 세션이 앱 종료까지 계속 살아있었다 | `TerminalStore` 에 프로젝트/세션 단위 일괄 kill 이 없었고, `pty_kill` 호출부 자체가 0건이었다 | `TerminalStore::kill_project`(project_close 가 호출)·`kill_session`(탭 닫기가 호출) 신설 + `PtySession::drop` 이 kill·일시정지 해제를 함께 수행. **접합부 수정(Phase D)**: 계약 §3 이 "탭 닫기 시 pty_kill" 배선까지 명시했으나 R1/R2/F 어느 트랙도 실제 호출부를 연결하지 않아 재검 시점까지 `pty_kill`/`killPty` 호출부가 0건으로 남아 있었다 — `layout/commands.rs::close_tab_and_finish` 에 `TabKind::Terminal` 분기로 직접 배선해 닫았다(아래 "Phase D 접합부 수정" 절 상세) |
| 22 | 폰트 패밀리 "System Default" 선택, AI Provider 전환 시 Model 리셋 등 "필드를 해제"하는 조작이 백엔드에 반영되지 않았다 | `SettingsPatch` 의 `Option<String>` 필드가 "생략=건드리지 않음"과 "명시적 해제"를 구분하지 못했다(`aiOmlxBaseUrl` 만 빈 문자열 관례로 예외 처리돼 있었다) | `merge_clearable_string` 헬퍼로 그 관례를 `shellOverride`·`editorFontFamily`·`terminalFontFamily`·`uiFontFamily`·`aiProvider`·`aiModel` 6필드에 일반화(사용자 결정 7 — `Option<Option<T>>` 은 배제). 프론트 피커도 클리어 시 `null` 대신 `''` 를 patch 에 실어 보내도록 동반 수정 |
| 23 | 검색 결과의 컬럼·하이라이트 좌표가 한글 등 비 ASCII 문자 뒤에서 어긋났다(ASCII 에서도 이론상 1칸 오차 여지) | Rust 의 UTF-8 바이트 오프셋을 그대로 `column`/`matchStart`/`matchEnd` 에 실었는데, monaco `Position`·JS 문자열 slice 는 UTF-16 코드유닛 단위다 | `byte_offset_to_utf16_units` 신설, `search_file` 이 매치 지점·프리뷰 오프셋 전부 이 함수를 거치도록 변경. ASCII·한글 회귀 테스트 2건 |
| 24 | 언어서버가 크래시 후 백그라운드에서 자동 재시작에 성공하면 상태가 `Running` 으로 보고됐지만, 실제로는 요청에 응답하지 못했다 | 재기동된 프로세스는 `initialize` 핸드셰이크를 다시 거친 적이 없는데, 이 백그라운드 재시작 경로는 프론트의 어떤 액션도 기다리지 않아 재핸드셰이크를 걸어줄 주체가 없다 | `handle_process_exit` 의 재시작 성공 분기가 `Running` 대신 `Crashed`(+ 안내 메시지)를 보고하도록 정정 — 사용자가 수동 `lsp_restart`(재핸드셰이크 포함)로 복구. 근본 수정(세션 세대 이벤트)은 T1-D 로 명시 이월 |

## §2.4 1차 T0 잔여 (Phase R2 + Phase D 접합부)

| # | 증상 | 원인 | 수정 |
|---|------|------|------|
| 1 | 프로젝트 루트의 조상 경로에 `target`/`dist`/`.git` 등이 하나라도 있으면(흔한 레이아웃, git watcher 자신의 루트=`.git` 포함) 워처가 모든 이벤트를 침묵으로 삼켰다 | `is_ignored_path` 가 절대경로 전체의 컴포넌트를 검사해, 감시 루트 **밖** 조상 경로의 이름까지 걸렸다 | 감시 루트 기준 상대경로만 검사(`strip_prefix(root)`)하도록 정정, 루트 밖 케이스는 기존 보수적 동작으로 폴백. 회귀 테스트 4건(기존 테스트 뒤집기 — 사용자 결정 1) |
| 2 | 2초 flusher 가 아직 못 따라잡은 상태로 프로젝트를 닫으면 미저장 레이아웃 변경이 경고 없이 사라졌다 | `flush_dirty_layouts` 의 `filter_map` 이 `state.layouts` 에 없는 항목을 조용히 스킵했고, `project_close` 는 flush 를 기다리지 않고 바로 레이아웃을 제거했다 | `project_close` 가 제거 전 동기 flush, `filter_map` 미스는 `log::warn!` 로 노출 |
| 7 | `save_theme` 은 경로 구분자를 거부하는데 `delete_theme`/`load_theme` 은 `theme_id` 를 검증 없이 경로에 그대로 조인했다 | `../../../.ssh/id_rsa` 같은 `theme_id` 로 저장소 밖 임의 파일 삭제/읽기가 가능했다 | 두 함수 모두 `root_guard::ensure_safe_component` 적용(`file_mirror_untitled` 등이 이미 쓰는 검증 재사용). 경로 탈출 회귀 테스트 2건 |
| 3 | 같은 pane 에서 파일 A→B 로 탭을 전환해도 breadcrumbs/상태바가 계속 A 의 monaco 인스턴스를 반환했다 | `EditorPane` 은 `key` 없이 재사용되고 monaco 인스턴스도 재마운트 없이 `setModel` 로만 버퍼를 바꾸는데, 레지스트리 등록은 최초 마운트 시 1회(`handleEditorMount`)뿐이었다 | 등록을 `useEffect(..., [tabId, editor])` 로 이동 — 탭 전환마다 재등록(결정 2 — `key={tabId}` 는 hot-exit 미러 전제 변경이라 배제) |
| 4 | pane 을 재사용한 채 파일 A→B 로 전환하면, A 에서 걸어둔 auto-save/preview 타이머가 발화해 B 의 내용을 A 의 경로에 써버렸다 | 타이머 클로저가 스테일 `path`/`draftRef` 를 참조 — `EditorPane` 에 `key` 가 없어 인스턴스가 재사용되기 때문(#3 과 같은 근본 원인) | `pathRef` 로 "현재 이 인스턴스가 어떤 경로를 보고 있는가"를 추적, 타이머 발화 시 스케줄 당시 경로와 비교해 다르면 스킵 |
| 5·6 | 문자열 접두 검사(`isWithinRoot`)가 `..` 우회에 취약했고, Windows 경로 구분자를 혼용해 정상 경로까지 거부했다. 세션 `applyEdit` 핸들러가 initialize 이후 등록되고, 무루트 전역 fallback 이 있어 프로젝트 간 편집 침범 여지가 있었다 | monaco `Uri#fsPath` 는 `..` 를 resolve 하지 않는 순수 문자열 변환이고, 구분자 하드코딩(`/`)이 백슬래시 경로와 어긋났다. 핸들러 등록이 initialize 완료 후라 그 사이 요청이 무루트 fallback 으로 새 나갈 여지가 있었다 | `normalizeFsPath` 로 `.`/`..` lexical 정규화 + 구분자 통일 후 비교. 핸들러를 `spawnLspSession` 호출 **이전**(client 생성 직후)으로 이동 + 무루트 전역 fallback(`registerWorkspaceApplyEditHandler`) 제거 |
| 8 | 터미널 입력 원문이 항상(진단 플래그 없이) 최대 300건 메모리에 쌓이고 클립보드 복사 커맨드로 그대로 노출됐다 | `recordImeDebug` 에 수집 여부 게이트가 없었다 | `setImeDebugEnabled`/`isImeDebugEnabled` 신설, 기본 `false` — 꺼져 있으면 기록도 안 하고(`recordImeDebug` early return) 복사 커맨드도 `isEnabled` 로 숨는다 |
| 9 | Zen 모드로 상태바를 숨기면 IDE 프로토콜(diff/save/close-tab 요청·상태 sync·진단 push) 전체가 조용히 멈췄다 | 이 로직이 `StatusBarContent` 안에 있었는데, 그 컴포넌트가 Zen 에서 언마운트된다 | `IdeSyncProvider` 신설(main 창 앱 루트에 상시 마운트) — `StatusBarContent` 는 표시용 조회만 남기고 이벤트 핸들러·`useIdeStatusSync` 는 새 프로바이더로 이관 |
| 11 | 탭이 보조 창으로 이동한 뒤 그 창에서 닫히면, `taide --wait` 로 대기 중인 외부 CLI 프로세스가 영원히 풀리지 않았다 | `pathToWaitMarkers` 가 모듈 스코프 `Map` — 창마다 별개 JS 렐름이라, main 창(`AgentExternalOpenProvider`)이 등록한 마커가 보조 창의 `useCloseTab` 에서는 보이지 않았다 | `localStorage`(모든 창이 같은 origin 을 공유) 로 백엔드 교체 — `registerWaitMarker`/`takeWaitMarkers` 시그니처는 그대로 |

## Phase D 접합부 수정 — "탭 닫기 시 pty_kill" 배선 누락 (#21)

계약 §3 은 사용자 결정 9(`Rust: project_close 일괄 kill + 탭 닫기 시 pty_kill`)를 R1/R2 트랙에
배정했으나, R2 완료 보고(§2.2·§2.3 커버리지 확인)와 실제 코드를 대조한 결과 **`project_close` 쪽
(`TerminalStore::kill_project`)만 구현돼 있었고, "탭 닫기 시 pty_kill" 절반은 어느 트랙에서도 실제
호출부가 연결되지 않았다** — `rg 'pty_kill|killPty' src src-tauri` 기준 프론트 `killPty`(이미 존재하던
IPC 래퍼)도, Rust 쪽 신규 `close_tab_and_finish` 배선도 0건이었다.

수정: `src-tauri/src/domain/terminal/commands.rs` 에 `TerminalStore::kill_session(session_id)`
(`kill_project` 의 단일 세션 버전) 신설, `src-tauri/src/domain/layout/commands.rs::close_tab_and_finish`
— `layout_close_tab` 커맨드와 IDE 도구의 탭 닫기가 공유하는 단일 경로 — 가 닫힌 탭의 `TabKind` 가
`Terminal` 이면 그 `sessionId` 로 이 메서드를 호출하도록 배선했다. 새 IPC 커맨드는 없다(기존
`layout_close_tab` 의 내부 부수효과 확장). 상세는 `features/terminal.md` §10, IPC 표면 설명은
`ipc-contract.md`(§3 "T0 감사 데이터·기능 수정") 참조.

## 검증

- `bun run typecheck`/`lint`/`format:check`/`test`(1189/1189) 전부 그린.
- `cargo fmt --all --check`/`cargo clippy --workspace --all-targets -- -D warnings`/
  `cargo test --workspace`(935 lib + 6 session_restore + 17 cli = 958/958) 전부 그린.
- `bunx vite build` exit 0.
- 실기 확인(watcher 이벤트 폭주 해소·pty 탭닫기 회수·검색 한글 오프셋·설정 해제 반영·IDE 배지
  Zen 무관 표시·원격 거부 5종)은 `docs/quality-assurance/2026-08-11-qa6-checklist.md` 의 "감사 T0
  24항목 재검" 절로 이월 — 자동 테스트로 커버되지 않는 항목만 체크박스로 추적한다.

## 대상 파일

Rust: `src-tauri/src/domain/remote/dispatch.rs`, `src-tauri/src/lib.rs`,
`src-tauri/src/domain/project/commands.rs`, `src-tauri/src/domain/terminal/commands.rs`,
`src-tauri/src/domain/layout/commands.rs`(Phase D), `src-tauri/src/infra/pty.rs`,
`src-tauri/src/infra/persist.rs`, `src-tauri/src/infra/watcher.rs`,
`src-tauri/src/domain/search/commands.rs`, `src-tauri/src/domain/search/service.rs`,
`src-tauri/src/domain/search/types.rs`, `src-tauri/src/domain/settings/service.rs`,
`src-tauri/src/domain/theme/service.rs`, `src-tauri/src/domain/git/service.rs`,
`src-tauri/src/domain/git/types.rs`, `src-tauri/src/domain/lsp/commands.rs`.

프론트: `src/widgets/editor-pane/editor-pane.tsx`, `src/widgets/editor-pane/lsp-session-registry.ts`,
`src/shared/lib/lsp/workspace-edit-applier.ts`, `src/shared/lib/lsp/workspace-edit-apply-handler.ts`,
`src/app/bootstrap-lsp.ts`, `src/app/app.tsx`, `src/app/providers/ide-sync-provider.tsx`(신설),
`src/widgets/window-chrome/status-bar-content.tsx`, `src/shared/lib/ime-debug.ts`,
`src/entities/agent/agent-wait-marker-registry.ts`, `src/widgets/git-panel/git-panel.tsx`,
`src/widgets/settings-view/settings-view.tsx`, `src/shared/lib/command-registry.ts`,
`src/shared/api/bindings.ts`(자동 생성).
