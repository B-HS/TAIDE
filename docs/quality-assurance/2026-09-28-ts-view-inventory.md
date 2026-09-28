# TS view 전수 inventory 진행 기록

## 범위와 판정

현행 앱은 `src/app/app.tsx`에서 메인 창과 보조 창을 분기합니다. 테스트 파일을 제외한 `src/**/*.tsx`는 212개입니다(`rg --files src | rg '\.tsx$' | rg -v '\.test\.tsx$' | wc -l`). 이 수에는 provider와 공용 UI도 포함되므로 화면 수가 아닙니다. 아래 표는 실제 진입 컴포넌트와 확인한 화면 축의 첫 연결입니다. 테스트 파일의 존재나 전체 `bun test` 통과는 키보드·시각·접근성·실제 OS 창 동작을 입증하지 않습니다. 직접 앱 실기는 아직 수행하지 않았습니다.

212개 경로 전체는 `src-tauri/tests/fixtures/rust-native/ts-view-components-v1.json`에 정렬해 고정했습니다. `src-tauri/tests/rust_native_phase0_ts_view_census.rs`는 현재 `src/`의 비테스트 `.tsx`를 다시 열거해 추가·삭제·이름 변경을 감지합니다. 대상 테스트 1건, 대상 Clippy, Rust fmt, JSON·이 문서 Prettier, diff 검사가 exit 0입니다. 이 경로 census는 전수 작업의 모집단일 뿐 화면·상태·상호작용·접근성의 동등성 판정이 아닙니다. 이 문서에 직접 적힌 비테스트 `.tsx` 경로는 현재 46개이며 나머지 166개는 하위 항목 단위의 의미·증거 연결이 필요합니다.

[대화상자·메뉴 inventory](2026-09-28-ts-overlay-inventory.md)에 별도 26개 경로의 열림·선택 경계와 직접 테스트 유무를 기록했습니다. 두 문서의 중복을 제외하면 경로가 직접 연결된 항목은 70개, 남은 경로는 142개입니다. 경로가 적혔다는 사실만으로 하위 상태나 실기 증거가 충분하다는 뜻은 아닙니다.

[설정 화면 inventory](2026-09-28-ts-settings-inventory.md)는 `settings-view` 17개·`features/settings` 22개를 section/입력 경계에 연결합니다. 세 문서의 중복을 제외한 직접 연결 경로는 107개, 아직 역할 연결이 필요한 경로는 105개입니다. 설정 필드별 저장·오류·실기 검증은 미완료입니다.

[앱 provider inventory](2026-09-29-ts-provider-inventory.md)는 추가 11개 경로의 전역 이벤트·설정·창 상태 동기화 책임을 연결합니다. 네 문서의 중복 제외 직접 연결 경로는 118개, 남은 경로는 94개입니다. 각 provider의 실제 GUI·접근성 동작은 별도 검증이 필요합니다.

[하위 feature inventory A](2026-09-29-ts-feature-inventory-a.md)는 command palette·editor·explorer·Git의 추가 21개 경로를 연결합니다. 다섯 문서의 중복 제외 직접 연결 경로는 139개, 남은 경로는 73개입니다. 실제 화면 상태·접근성 판정은 남아 있습니다.

[하위 feature inventory B](2026-09-29-ts-feature-inventory-b.md)는 outline·plugin·preview·problems·search의 추가 22개 경로를 연결합니다. 여섯 문서의 중복 제외 직접 연결 경로는 161개, 남은 경로는 51개입니다. 실제 형식별 preview·키보드·접근성 검사는 미완료입니다.

[하위 feature inventory C](2026-09-29-ts-feature-inventory-c.md)는 project·shell slot·snippet·split·tab·theme·welcome·window의 추가 22개 경로를 연결합니다. 일곱 문서의 중복 제외 직접 연결 경로는 183개, 남은 경로는 29개입니다. 실제 다중 창·drag/drop·접근성 검사는 미완료입니다.

[앱 진입점·shared·widget inventory](2026-09-29-ts-shared-widget-inventory.md)는 마지막 29개 경로를 연결합니다. 여덟 문서의 중복 제외 직접 연결 경로는 212/212개이며, 비시각 context·hook·테스트 helper를 화면과 구분했습니다. 실제 화면 상태·시각·접근성 검사는 미완료입니다.

## 창·화면 진입점

| 화면/경계 | 현행 컴포넌트와 상태·동작 | 확인한 자동 근거 | 남은 실기·접근성 확인 |
| --- | --- | --- | --- |
| 메인 창/빈 세션 | `src/widgets/app-shell/app-shell.tsx`: 프로젝트·shell 상태 로딩, 프로젝트 0개일 때 welcome, 프로젝트 drag/drop, zen·rail·status bar, 파일/폴더 OS drop | `src/widgets/app-shell/app-shell.test.tsx`, `src/widgets/app-shell/use-project-drag.test.tsx`, `src/features/welcome/welcome-screen.test.tsx` | cold start, 폴더·파일 drop, 빈 화면, 메뉴/키보드, 창 크기별 시각 |
| 프로젝트 rail/그룹 | `src/widgets/app-sidebar/app-sidebar.tsx`: 최근 프로젝트, 그룹 생성·정렬, 프로젝트 활성화·slot 분할, 경로 입력, 설정 진입 | `src/widgets/app-sidebar/app-sidebar.test.tsx`, `src/widgets/app-sidebar/sortable-project-group-header.test.tsx`, `src/features/project/project-group-dialog.test.tsx` | drag/drop 목표·순서, 그룹 축소, 메뉴 focus·스크린리더 |
| 메인 shell slot | `src/widgets/app-shell/shell-slot-tree-view.tsx`, `src/widgets/app-shell/project-shell.tsx`: slot split/close, 프로젝트별 explorer 접기·폭 조절, zen, problems | `src/widgets/app-shell/shell-slot-tree-view.test.tsx`, `src/shared/lib/shell-slot.test.ts` | 다중 slot 포커스·폭·저장/복원, separator 키보드 |
| 보조 창 | `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx`: 고정 project/windowSlot, 별도 explorer 상태, 마지막 tab·layout 오류 시 창 닫기, sidebar/status bar 없음 | `src/widgets/auxiliary-window-shell/auxiliary-window-shell.test.tsx`, `src/shared/lib/window-context.test.ts` | 실제 창 분리·반환·닫기, 메인 창과 설정/포커스 분리 |
| 공통 window chrome | `src/widgets/window-chrome/title-bar-content.tsx`, `src/widgets/window-chrome/status-bar-content.tsx`, `src/widgets/auxiliary-window-shell/auxiliary-title-bar-content.tsx`: title/status, 시스템 사용량, 창 동작 | `src/widgets/window-chrome/status-bar-content.test.tsx` | native title/menu, 상태 공지·VoiceOver, 플랫폼별 창 chrome |
| 공통 palette/task dialog | `src/app/main-window-dialogs.tsx`, `src/widgets/command-palette/command-palette.tsx`, `src/widgets/task-runner/task-runner-dialog.tsx`: 메인은 포커스 slot, 보조 창은 고정 프로젝트; 검색 모드·실행·닫기 | `src/widgets/command-palette/command-palette.test.tsx`, `src/widgets/task-runner/task-runner-dialog.test.tsx` | 메인/보조 창 shortcut·focus trap·IME·Escape·스크린리더 |

## 프로젝트 영역과 탭 종류

| 화면/경계 | 현행 컴포넌트와 상태·동작 | 확인한 자동 근거 | 남은 실기·접근성 확인 |
| --- | --- | --- | --- |
| explorer 4개 view | `src/widgets/explorer/explorer-panel.tsx`: files/search/git/outline tablist, 프로젝트별 검색 요청, root 누락 복구; `src/widgets/explorer/explorer-container.tsx`가 파일 조작을 조립 | `src/widgets/explorer/explorer-container.test.tsx`, `src/widgets/search-panel/search-panel-container.test.tsx`, `src/widgets/git-panel/git-panel.test.tsx`, `src/widgets/outline-panel/outline-panel-container.test.tsx` | 각 view 전환·focus, root 누락·오류, tablist 키보드·aria |
| 파일 트리 | `src/features/explorer/file-tree.tsx`, `src/features/explorer/file-tree-row.tsx`: tree/treeitem, 선택·확장·rename·draft·context menu·새 파일/폴더 | `src/features/explorer/file-tree.test.tsx`, `src/features/explorer/explorer-shortcuts.test.ts`, `src/widgets/explorer/use-explorer-entry-crud.test.tsx` | 큰 트리 스크롤, rename 오류, tree 키보드·VoiceOver |
| 검색·교체 | `src/widgets/search-panel/search-panel-container.tsx`, `src/features/search/search-panel.tsx`: query·case/word/regex/gitignore·exclude·scope·결과·replace | `src/widgets/search-panel/search-panel-container.test.tsx`, `src/entities/search/search-query.test.ts`, `src/features/search/search-result-rows.test.ts` | 긴 결과·취소·교체 확인, 결과 키보드/공유 상태 |
| Git | `src/widgets/git-panel/git-panel.tsx`, `src/widgets/git-panel/commit-detail-panel.tsx`, `src/features/git/diff-view.tsx`: 변경 그룹·선택·stage/unstage·discard 확인·commit·history·diff | `src/widgets/git-panel/git-panel.test.tsx`, `src/widgets/git-panel/commit-detail-panel.test.ts`, `src/features/git/git-section-header.test.tsx` | 실제 저장소 충돌·대량 변경, context menu·선택 키보드·dialog focus |
| outline/problems | `src/widgets/outline-panel/outline-panel-container.tsx`, `src/widgets/problems-panel/problems-panel-container.tsx`: 활성 문서 symbol, severity filter·문제 이동·닫기 | `src/widgets/outline-panel/outline-panel-container.test.tsx`, `src/features/problems/problem-list-rows.test.ts` | 실제 LSP 진단·문서 전환, tree/목록 키보드·발화 |
| pane/tab layout | `src/widgets/editor-area/editor-area.tsx`, `src/widgets/editor-area/pane-node-view.tsx`, `src/widgets/editor-area/pane-tab-bar.tsx`: split·resize·DnD·pin·dirty-close·tab 이동/분리·키맵 | `src/widgets/editor-area/pane-node-view-focus.test.tsx`, `src/widgets/editor-area/use-request-close-tab.test.tsx`, `src/features/tab/tab-bar-context-menu.test.tsx` | 실제 다중 창 DnD, tablist 키보드·focus·dirty dialog |
| 파일·untitled·diff/editor | `src/widgets/editor-pane/editor-pane.tsx`, `src/widgets/editor-pane/untitled-pane.tsx`, `src/widgets/diff-pane/diff-pane.tsx`, `src/widgets/claude-diff-pane/claude-diff-pane.tsx`: 저장·복원·누락 원본·Markdown preview·충돌 dialog | `src/widgets/editor-pane/editor-pane-missing-source.test.tsx`, `src/widgets/editor-pane/use-editor-file-persistence.test.tsx`, `src/features/editor/code-editor.test.tsx` | 실제 편집/IME·undo·selection·diff/충돌·보조 기술 |
| terminal | `src/widgets/terminal-pane/terminal-session.tsx`, `src/widgets/terminal-pane/terminal-pane.tsx`, `src/features/terminal/terminal-view.tsx`: spawn/attach·출력·입력·선택·메뉴·split | `src/widgets/terminal-pane/terminal-session.test.tsx`, `src/widgets/terminal-pane/terminal-flow-control.test.ts`, `src/features/terminal/terminal-view.test.ts` | 실제 PTY/CJK/복원·직접 Exit, terminal focus·메뉴 |
| preview | `src/widgets/preview-pane/preview-pane.tsx`: image/video/audio/HTML/PDF/sheet/PPTX/HWP·오류 시 외부 열기 | `src/shared/lib/preview-kind.test.ts`, `src/features/preview/pdf-preview.test.tsx`; 나머지 형식별 직접 검사는 미확인 | 실제 형식별 파일·오류·스크롤/seek·접근성·리소스 해제 |
| 기타 tab | `src/widgets/app-file-pane/app-file-pane.tsx`, `src/widgets/search-editor/search-editor-pane.tsx`, `src/widgets/commit-file-diff/commit-file-diff.tsx`: 앱 파일·검색 결과·commit diff tab | `src/entities/app-file/app-file-model-path.test.ts`, `src/widgets/search-editor/search-editor-context-lines.test.ts`, `src/widgets/git-panel/commit-detail-panel.test.ts` | 각 tab 열기·재진입·닫기·포커스·데이터 보존 |

## 설정·보조 화면

| 화면/경계 | 현행 컴포넌트와 상태·동작 | 확인한 자동 근거 | 남은 실기·접근성 확인 |
| --- | --- | --- | --- |
| 설정 13개 section | `src/widgets/settings-view/settings-view.tsx`: appearance/language/interface/notifications/editor/snippets/terminal/keymap/LSP/AI/plugins/sync/remote 목차·스크롤, 설정 JSON tab | `src/entities/settings/settings.query.test.ts`, `src/widgets/settings-view/agent-hooks-project-list.test.tsx`; section별 렌더 직접 검사는 미확인 | section별 값 변경·재시작·오류, 목차 키보드·라벨 |
| theme·snippet 편집 | `src/widgets/theme-editor/theme-editor.tsx`, `src/widgets/snippet-editor/snippet-editor.tsx`: 별도 화면, draft·저장·삭제·미저장 확인 | `src/shared/lib/theme-draft.test.ts`, `src/shared/lib/snippet-draft.test.ts`; 두 화면 직접 렌더 검사는 미확인 | 실시간 preview·색 대비·입력/삭제 dialog·키보드 |
| keymap·plugin | `src/widgets/keybindings-editor/keybindings-editor.tsx`, `src/widgets/plugin-manager/plugin-manager.tsx`: 키 검색·capture·conflict; install/uninstall·VSIX import·reload | `src/widgets/keybindings-editor/keybindings-editor.test.ts`, `src/entities/plugin/plugin.query.test.ts`; manager 직접 렌더 검사는 미확인 | 충돌/캡처·VSIX 오류, dialog focus·키보드 |
| 알림·사용량 | `src/widgets/app-toaster/app-toaster.tsx`, `src/widgets/system-usage-modal/system-usage-modal.tsx`: toast와 상세 사용량 dialog | `src/shared/constants/toast.test.ts`, `src/widgets/window-chrome/status-bar-content.test.tsx`; 사용량 modal 직접 검사는 미확인 | toast 발화/시간, modal focus/스크린리더 |

## 공통 축과 남은 전수 작업

- 테마/로케일은 `src/app/providers/theme-provider.tsx`, `src/app/providers/locale-provider.tsx`가 메인·보조 창 모두에 적용합니다. 로드 오류 배너·재시도와 `documentElement` 적용을 확인했지만, 각 화면의 테마별 캡처·CJK/문자열 길이·접근성은 미검증입니다.
- 키 입력은 `src/shared/hooks/use-global-keymap.ts`, `src/widgets/editor-area/editor-area.tsx`, palette와 각 tree/리스트가 소유합니다. 실제 shortcut 충돌·IME·VoiceOver는 미검증입니다.
- 이 표는 진입점 단위이고 연결 문서 여덟 개가 212개 `.tsx` 경로의 역할·자동 근거·실기 공백을 개별 기록합니다. 메뉴·dialog별 모든 상태/오류/빈 상태, 각 항목의 시각·접근성 캡처와 실제 창 결과는 아직 연결되지 않았습니다. 따라서 Phase 0 TS view inventory와 M7-C4b는 미완료입니다.
