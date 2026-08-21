# T2-B 1호 — settings-view 섹션 위젯 분해 계약 (2026-08-21, d-29)

> 정본: 감사 `2026-08-18-architecture-audit.md` §5 T2-B 행("settings-view(섹션 위젯)").
> goal("추천안대로 배치 계속진행") 하 HANDOFF §8-2 순번 — T2-B 는 파일별 단독 배치
> (editor-pane 관심사 분해 d-13 선례).
> 착수 전 메인 실사: `widgets/settings-view/settings-view.tsx` 926줄(감사 927 실측 유지 —
> d-28 이 상수 4개만 소거). 동거: 상수 블록(:75~) + 옵션 목록 3종 + hooks 19회
> (useState/useQuery/useMutation) + SettingsSection(features) 소비. 형제 파일 선례 실재
> (agent-cli-status-row 등 3파일 — 부분 분해 이미 시작된 형태).

## 1. 범위·원칙

- **섹션 단위 위젯 분해** — editor-pane 선례(관심사별 파일·1파일 1컴포넌트): 각 설정 섹션
  (에디터·터미널·자동 저장·AI·원격·키맵·테마·에이전트 등 — 실사로 섹션 경계 확정)을
  `widgets/settings-view/` 하위 파일로 분리. 섹션 전용 상수·옵션 목록은 해당 섹션 파일로
  동행(2곳 이상 공유분만 shared — 임의 승격 금지).
- **동작·마크업 무변경 절대 조건**: 렌더 트리 동등(스크롤 앵커·SETTINGS_SCROLL_OFFSET_PX
  내비게이션·검색/점프가 있으면 그 배선 보존), settings 쿼리/뮤테이션 배선 의미 동일. props
  는 상위(컨테이너)에서 주입 — 섹션 파일이 직접 쿼리를 갖는지 여부는 기존 형제 파일
  (agent-hooks-project-list 등) 선례 실사 후 일관 선택·근거 기록.
- 컨테이너(settings-view.tsx)는 조립·공유 상태만 남김(목표: 대폭 축소 — 수치 목표는 실사
  후 자연 결정, 억지 분할 금지). FSD·barrel 금지·named export.
- 범위 외: 설정 항목 추가/제거·UI 리디자인·T2-B 다른 파일·d-28 이월(불리언 기본값 9종은
  이 분해에서 자연 해소되면 겸사, 아니면 이월 유지).

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, TS 전용) → 메인 2차 → Phase E 4렌즈(구조 동등성 —
  렌더 트리·배선 diff / 실사충실성 — 섹션 경계·상수 배치 / 설계 — 분해 입도·선례 정합 /
  계약 — 표면·컨벤션) → 적대적(major 이상) → confirmed 수정 → 커밋 → prod 병합.
- 검증: `bun run verify` + `bunx vite build` exit 0 + bindings 무변경. 실기 이월(qa6):
  설정 화면 전 섹션 렌더·값 변경 반영·스크롤 내비게이션.

---

## 3. 구현 완료 기록 (2026-08-21, Phase E 검토 전)

> 구현 wf_8b86dab0-684(단독 sonnet+xhigh). 구현이 계약 §3 기록을 누락해 메인이 구현 보고
> (스크래치패드 d29-impl-report.json)에서 전사. 메인 2차: 컨테이너 201줄·섹션 12파일 실물
> 확인 + verify·vite build 직접 재실행 exit 0·bindings 무변경.

### 3.1 분해 결과 (926 → 201줄 + 섹션 12파일)

- **settings-appearance-section.tsx**: SettingsAppearanceSection(91줄) — 테마 선택/커스텀 테마 목록/시스템 테마 따르기. 자체 useQuery(themeListQueryOptions)+useSetThemeId 소유. props: id, settings, updateSettings, onOpenAppDataFolder(컨테이너 공유 콜백), onOpenThemeEditor(전체화면 스왑 트리거, 컨테이너 setThemeEditorState). ThemeEditorState 타입을 이 파일에서 export해 컨테이너가 재사용.
- **settings-language-section.tsx**: SettingsLanguageSection(45줄) — 로케일 선택. 자체 useQuery(localeListQueryOptions). props: id, settings, updateSettings, onOpenAppDataFolder.
- **settings-interface-section.tsx**: SettingsInterfaceSection(126줄) — 토스트 위치/리사이저 두께/시스템 사용량/미니맵/에이전트 배지/에이전트 훅/IDE 연동/미리보기 탭/젠모드 스위치 9종. 자체 useQuery(projectListQueryOptions)로 AgentHooksProjectList 배선(형제 파일 AgentCliStatusRow는 이미 자족형이라 그대로 소비). props: id, settings, updateSettings.
- **settings-editor-section.tsx**: SettingsEditorSection(255줄) — 에디터 폰트/포맷/자동저장/줄바꿈/탭/커서/화이트스페이스 등 22개 필드. 섹션 전용 상수(MIN/MAX_AUTO_SAVE_DELAY_MS·MIN/MAX_TAB_SIZE·EDITOR_CURSOR_STYLE/BLINKING/RENDER_WHITESPACE_OPTIONS) 동행. 자체 useQuery(fontListQueryOptions). props: id, settings, updateSettings.
- **settings-snippets-section.tsx**: SettingsSnippetsSection(30줄) — 스니펫 관리/폴더 열기 버튼 2개. 쿼리 없음. props: id, onManage(전체화면 스왑 트리거), onOpenAppDataFolder.
- **settings-terminal-section.tsx**: SettingsTerminalSection(97줄) — 터미널 폰트/셸/셸프로필/스크롤백/커서 6개 필드. 섹션 전용 상수(MIN/MAX/DEFAULT_SCROLLBACK·DEFAULT_TERMINAL_CURSOR_STYLE·TERMINAL_CURSOR_STYLE_OPTIONS) 동행. 자체 useQuery(fontListQueryOptions·shellProfilesQueryOptions). props: id, settings, updateSettings.
- **settings-keymap-section.tsx**: SettingsKeymapSection(21줄) — 키맵 에디터 열기 버튼 1개. 쿼리·설정 불필요. props: id만.
- **settings-lsp-section.tsx**: SettingsLspSection(47줄) — LSP 서버 상태 목록. 자체 useQuery(lspServersQueryOptions)+useLspInstallProgress+useLspInstallProgressSync(부작용 구독)+설치/취소 뮤테이션. props: id만.
- **settings-ai-section.tsx**: SettingsAiSection(142줄) — AI 프로바이더 토큰 3종/모델 선택/자동탭/프롬프트 템플릿 3개. 섹션 전용 상수(DEFAULT_AI_PROVIDER JSDoc 포함·AI_PROVIDER_OPTIONS·PROMPT_ROWS) 동행. 자체 useQuery(aiTokenStatusQueryOptions·aiModelsQueryOptions)+useSetAiToken/useClearAiToken+useOpenTab(projectId)+layoutQueryOptions(projectId)(프롬프트 파일 열기 전용, 컨테이너의 헤더용 동일 훅과는 독립 인스턴스 — 쿼리 캐시는 공유). props: id, projectId, settings, updateSettings.
- **settings-plugins-section.tsx**: SettingsPluginsSection(18줄) — PluginManager 위젯 임베드. 쿼리 없음. props: id만.
- **settings-sync-section.tsx**: SettingsSyncSection(81줄) — 동기화 연결/업로드/다운로드/충돌 다이얼로그. isSyncConflictOpen 로컬 state + SyncConflictDialog 렌더링을 이 파일로 완전 이관(Radix AlertDialog가 Portal 기반이라 선언 위치 이동이 렌더 DOM에 영향 없음을 shared/ui/alert-dialog.tsx 확인). 자체 useQuery(settingsQueryOptions — syncGistId 읽기 전용·syncStatusQueryOptions)+4개 뮤테이션. props: id만.
- **settings-remote-section.tsx**: SettingsRemoteSection(81줄) — 원격 접속 토글/링크 발급/세션 폐기/비밀번호/허용 호스트. issuedRemoteUrl 로컬 state. 자체 useQuery(remoteStatusQueryOptions)+4개 뮤테이션. props: id, settings, updateSettings, isUpdatingSettings(교차 관찰 — 아래 wiringDecision).
- **settings-view.tsx**: 컨테이너 926/927→201줄. 잔류: scrollContainerRef·activeSectionId·themeEditorState·isSnippetEditorOpen(전체화면 스왑 2종은 컨테이너 절대 소유 — ThemeEditor/SnippetEditor가 트리 전체를 대체하므로), SETTINGS_SCROLL_OFFSET_PX·SETTINGS_SECTION_ID·SETTINGS_TOC_ITEMS·handleTocSelect(스크롤 내비게이션), settings/themes(ThemeEditor 스왑용)/updateSettings·isUpdatingSettings(단일 공유 인스턴스)/openTab+layout(헤더 settings.json 버튼)/handleOpenAppDataFolder(3섹션 공유 콜백)/handleOpenSettingsFile. 12개 섹션을 원본과 동일 순서로 조립.

### 3.2 배선 결정 (교차 관찰 보존)

형제 파일 실사(agent-cli-status-row.tsx·agent-hooks-project-list.tsx·agent-hooks-project-row.tsx) 결과 3파일 모두 자신의 useQuery/useMutation을 직접 소유하고 부모로부터 데이터를 prop으로 받지 않는 패턴을 이미 확립 — 이를 기본 원칙으로 채택. 단, settings(엔티티 쿼리)와 useUpdateSettings(범용 뮤테이션) 2가지는 예외적으로 컨테이너 소유·prop 주입: (1) settings는 settingsQueryOptions()가 staleTime:Infinity라 캐시 공유는 안전하지만, 컨테이너의 blank-render 게이트(isSettingsPending || !settings)가 non-null을 이미 보증하므로 8개 섹션(Appearance/Language/Interface/Editor/Terminal/Ai/Remote 및 읽기전용인 Sync는 자체쿼리)에 그대로 넘겨 타입 내로잉 재작업을 피함. (2) useUpdateSettings는 원본 코드에서 컨테이너가 단일 인스턴스로 만들어 7개 섹션(Appearance/Language/Interface/Editor/Terminal/Ai/Remote)이 동일 mutate를 호출했고, 그 결과인 isUpdatingSettings(isPending)는 REMOTE 섹션의 allowedHostsSaving으로 '교차 관찰'된다(테마·에디터 등 무관한 필드를 바꿔도 Remote의 저장 중 표시가 깜빡이는 원본 그대로의 동작) — 섹션마다 독립 useUpdateSettings() 인스턴스를 쓰면 이 교차 관찰이 사라져 동작 무변경 절대조건을 어기므로, 컨테이너가 단일 인스턴스를 소유하고 mutate+isPending을 prop으로 주입해 원본 동작을 정확히 보존. 그 외 완전히 섹션 국소적인 엔티티 쿼리/뮤테이션(themes·locales·fonts·shellProfiles·projects·lspServers+progress·aiTokenStatus+aiModels+토큰뮤테이션·syncStatus+동기화4종·remoteStatus+원격4종)은 교차 섹션 관찰이 전혀 없어 형제 파일 선례대로 섹션이 직접 소유. SETTINGS_SECTION_ID·SETTINGS_TOC_ITEMS·SETTINGS_SCROLL_OFFSET_PX·스크롤 내비게이션은 계약 지시대로 컨테이너 절대 소유 — id는 컨테이너가 각 섹션에 prop으로 주입(섹션이 자기 id를 하드코딩하지 않음)해 TOC 매핑과 앵커 배선이 한 곳(컨테이너)에서만 정의되도록 유지. SyncConflictDialog는 원본에서 ScrollContainer 최하단(별도 섹션 밖)에 있었으나 Radix AlertDialog가 Portal로 렌더되어 React 트리상 선언 위치와 실제 DOM 위치가 무관함을 shared/ui/alert-dialog.tsx로 확인 후, isSyncConflictOpen 상태를 만드는 Sync 섹션 파일로 함께 이관(자족형 섹션 원칙).

### 3.3 상수 배치

섹션 전용 상수(EDITOR_CURSOR_STYLE_OPTIONS 등 Editor 3종·MIN/MAX_AUTO_SAVE_DELAY_MS·MIN/MAX_TAB_SIZE, Terminal의 TERMINAL_CURSOR_STYLE_OPTIONS·MIN/MAX/DEFAULT_SCROLLBACK·DEFAULT_TERMINAL_CURSOR_STYLE, Ai의 DEFAULT_AI_PROVIDER(JSDoc 포함)·AI_PROVIDER_OPTIONS·PROMPT_ROWS)는 사용처 섹션 파일로 동행. shared/constants(code-editor·code-font-size·layout·toast)에 이미 있던 것은 그대로 각 섹션이 import(Editor·Terminal이 DEFAULT_CODE_FONT_SIZE 등을 공유하지만 shared 층 소유라 이동 불필요). SETTINGS_SECTION_ID·SETTINGS_TOC_ITEMS·SETTINGS_SCROLL_OFFSET_PX는 12개 섹션+컨테이너가 공유하지만 '컨테이너 절대 소유' 지시에 따라 승격하지 않고 컨테이너에 유지, id만 prop으로 주입.

### 3.4 d-28 이월(불리언 기본값) 판정

d-28 이월(불리언 기본값 9종)은 이번 분해가 순수 이동(로직 무변경)이라 자연 해소되지 않았고 그대로 이월 — 각 섹션 파일에 `?? true`/`?? false` 인라인 리터럴 34곳 전부 원본 그대로 옮겼으며, 새 상수화·정리는 하지 않았다.

### 3.5 검증

bun run verify: typecheck(tsc --noEmit) 클린 → eslint(consistent-type-imports 위반 7건을 useUpdateSettings의 import type 분리로 수정 후 재검증 클린) → prettier --check 전체 통과 → bun test 1435 pass/0 fail(TS) → cargo fmt/clippy/test 전량 ok(Rust 1068+3+6+17 pass) — 전체 exit 0. bunx vite build exit 0(청크 크기 경고는 기존부터 있던 것). src/shared/api/bindings.ts git diff 없음(무변경 확인). 렌더 트리 동등성: SETTINGS_SECTION_ID 12개 값 리터럴을 원본과 동일하게 유지하고 컨테이너가 id를 prop으로 12개 섹션에 주입, 조립 순서(Appearance→Language→Interface→Editor→Snippets→Terminal→Keymap→Lsp→Ai→Plugins→Sync→Remote→spacer div)와 early-return 게이트 3종(설정 대기 blank div → 테마에디터 전체화면 스왑 → 스니펫에디터 전체화면 스왑 → 메인 트리) 순서를 원본 그대로 보존. 컴포넌트 소비처(widgets/editor-area/pane-node-view.tsx)는 SettingsView의 공개 API({projectId})가 무변경이라 수정 불필요함을 grep으로 확인.
