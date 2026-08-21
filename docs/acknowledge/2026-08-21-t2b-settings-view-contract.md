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

- **settings-appearance-section.tsx**: SettingsAppearanceSection(92줄) — 테마 선택/커스텀 테마 목록/시스템 테마 따르기. 자체 useQuery(themeListQueryOptions)+useSetThemeId 소유. props: id, settings, updateSettings, onOpenAppDataFolder(컨테이너 공유 콜백), onOpenThemeEditor(전체화면 스왑 트리거, 컨테이너 setThemeEditorState). ThemeEditorState 타입을 이 파일에서 export해 컨테이너가 재사용(§4 Phase E 에서 컨테이너 소유로 환원됨 — contract-4).
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

형제 파일 실사(agent-cli-status-row.tsx·agent-hooks-project-list.tsx·agent-hooks-project-row.tsx) 결과 3파일 모두 자신의 useQuery/useMutation을 직접 소유하고 부모로부터 데이터를 prop으로 받지 않는 패턴을 이미 확립 — 이를 기본 원칙으로 채택. 단, settings(엔티티 쿼리)와 useUpdateSettings(범용 뮤테이션) 2가지는 예외적으로 컨테이너 소유·prop 주입: (1) settings는 settingsQueryOptions()가 staleTime:Infinity라 캐시 공유는 안전하지만, 컨테이너의 blank-render 게이트(isSettingsPending || !settings)가 non-null을 이미 보증하므로 7개 섹션(Appearance/Language/Interface/Editor/Terminal/Ai/Remote)에 그대로 넘겨 타입 내로잉 재작업을 피함. Sync는 이 구현 당시 예외적으로 자체 쿼리(`settings?.syncGistId`)를 썼으나 §4 Phase E 에서 나머지 7개와 통일됨(contract-5/design-2). (2) useUpdateSettings는 원본 코드에서 컨테이너가 단일 인스턴스로 만들어 7개 섹션(Appearance/Language/Interface/Editor/Terminal/Ai/Remote)이 동일 mutate를 호출했고, 그 결과인 isUpdatingSettings(isPending)는 REMOTE 섹션의 allowedHostsSaving으로 '교차 관찰'된다(테마·에디터 등 무관한 필드를 바꿔도 Remote의 저장 중 표시가 깜빡이는 원본 그대로의 동작) — 섹션마다 독립 useUpdateSettings() 인스턴스를 쓰면 이 교차 관찰이 사라져 동작 무변경 절대조건을 어기므로, 컨테이너가 단일 인스턴스를 소유하고 mutate+isPending을 prop으로 주입해 원본 동작을 정확히 보존. 그 외 완전히 섹션 국소적인 엔티티 쿼리/뮤테이션(themes·locales·fonts·shellProfiles·projects·lspServers+progress·aiTokenStatus+aiModels+토큰뮤테이션·syncStatus+동기화4종·remoteStatus+원격4종)은 교차 섹션 관찰이 전혀 없어 형제 파일 선례대로 섹션이 직접 소유 — 단 이 "교차 섹션 관찰"만 검토한 논증은 §4 Phase E 에서 드러난 "전체화면 스왑 게이트로 인한 마운트 수명 축소" 축을 놓쳤다(contract-1/equivalence-2/3, wiring-1). SETTINGS_SECTION_ID·SETTINGS_TOC_ITEMS·SETTINGS_SCROLL_OFFSET_PX·스크롤 내비게이션은 계약 지시대로 컨테이너 절대 소유 — id는 컨테이너가 각 섹션에 prop으로 주입(섹션이 자기 id를 하드코딩하지 않음)해 TOC 매핑과 앵커 배선이 한 곳(컨테이너)에서만 정의되도록 유지. SyncConflictDialog는 원본에서 ScrollContainer 최하단(별도 섹션 밖)에 있었으나 Radix AlertDialog가 Portal로 렌더되어 React 트리상 선언 위치와 실제 DOM 위치가 무관함을 shared/ui/alert-dialog.tsx로 확인 후, isSyncConflictOpen 상태를 만드는 Sync 섹션 파일로 함께 이관(자족형 섹션 원칙) — isSyncConflictOpen 이 §4 Phase E 에서 컨테이너 소유로 환원된 뒤에도 다이얼로그 JSX 자체는 Sync 섹션에 그대로 남아 있다(Portal 렌더라 선언 위치가 레이아웃에 영향이 없다는 이 문단의 논증은 그대로 유효).

### 3.3 상수 배치

섹션 전용 상수(EDITOR_CURSOR_STYLE_OPTIONS 등 Editor 3종·MIN/MAX_AUTO_SAVE_DELAY_MS·MIN/MAX_TAB_SIZE, Terminal의 TERMINAL_CURSOR_STYLE_OPTIONS·MIN/MAX/DEFAULT_SCROLLBACK·DEFAULT_TERMINAL_CURSOR_STYLE, Ai의 DEFAULT_AI_PROVIDER(JSDoc 포함)·AI_PROVIDER_OPTIONS·PROMPT_ROWS)는 사용처 섹션 파일로 동행. shared/constants(code-editor·code-font-size·layout·toast)에 이미 있던 것은 그대로 각 섹션이 import(Editor·Terminal이 DEFAULT_CODE_FONT_SIZE 등을 공유하지만 shared 층 소유라 이동 불필요). SETTINGS_SECTION_ID·SETTINGS_TOC_ITEMS·SETTINGS_SCROLL_OFFSET_PX는 12개 섹션+컨테이너가 공유하지만 '컨테이너 절대 소유' 지시에 따라 승격하지 않고 컨테이너에 유지, id만 prop으로 주입.

### 3.4 d-28 이월(불리언 기본값) 판정

d-28 이월(불리언 기본값 9종)은 이번 분해가 순수 이동(로직 무변경)이라 자연 해소되지 않았고 그대로 이월 — 각 섹션 파일에 `?? true`/`?? false` 인라인 리터럴 34곳 전부 원본 그대로 옮겼으며, 새 상수화·정리는 하지 않았다.

### 3.5 검증

bun run verify: typecheck(tsc --noEmit) 클린 → eslint(consistent-type-imports 위반 7건을 useUpdateSettings의 import type 분리로 수정 후 재검증 클린) → prettier --check 전체 통과 → bun test 1435 pass/0 fail(TS) → cargo fmt/clippy/test 전량 ok(Rust 1068+3+6+17 pass) — 전체 exit 0. bunx vite build exit 0(청크 크기 경고는 기존부터 있던 것). src/shared/api/bindings.ts git diff 없음(무변경 확인). 렌더 트리 동등성: SETTINGS_SECTION_ID 12개 값 리터럴을 원본과 동일하게 유지하고 컨테이너가 id를 prop으로 12개 섹션에 주입, 조립 순서(Appearance→Language→Interface→Editor→Snippets→Terminal→Keymap→Lsp→Ai→Plugins→Sync→Remote→spacer div)와 early-return 게이트 3종(설정 대기 blank div → 테마에디터 전체화면 스왑 → 스니펫에디터 전체화면 스왑 → 메인 트리) 순서를 원본 그대로 보존. 컴포넌트 소비처(widgets/editor-area/pane-node-view.tsx)는 SettingsView의 공개 API({projectId})가 무변경이라 수정 불필요함을 grep으로 확인.

**"요청 수 불변" 정정(wiring-4)**: 위 문단 및 §3.2 가 전제하는 "요청 수 불변"은 **초기 마운트**(동일 queryOptions 를 여러 섹션이 구독해도 같은 queryKey 라 fetch 는 1회)에 한해 성립한다. 전체화면 스왑(테마/스니펫 에디터) 왕복은 섹션을 통째로 언/재마운트하므로, `staleTime` 이 기본값(60s, `app/query-client.ts`)인 쿼리(themeList·projectList·aiTokenStatus·syncStatus)는 스왑 복귀 시 재요청되고 `remoteStatusQueryOptions` 의 `refetchInterval` 폴링은 스왑 동안 멈춘다. `staleTime: Infinity` 쿼리(settings·fontList·locale·shellProfiles·lspServers)는 영향 없다. 이 차이는 React Query 의 표준 캐시 수명 동작이고 사용자 관찰 영향이 미미(재요청은 화면이 비표시 상태였던 짧은 스왑 구간 이후 1회일 뿐 무한 반복이 아님)하여 **코드 변경 없이 알려진 차이로 기록**한다 — staleTime 을 조정하면 이 화면 밖의 다른 소비처(예: Editor·Terminal 이 공유하는 fontList)의 캐시 정책까지 바뀌므로 이번 Phase E 수정 범위를 벗어난다.

---

## 4. Phase E 검토 반영 (2026-08-21)

> 검토 원문: 4렌즈 + major 이상 적대적 검증 — 발견 19건(critical/major 6, minor 13), confirmed 6·refuted
> 0. confirmed 전건(equivalence-1/2/3, wiring-1/2, contract-1)과 실질 영향이 있는 minor 를
> 아래에 반영한다. 커밋 dad3a52(구현) 이후 diff 대상.

### 4.1 근본 원인

§3 는 "settings·useUpdateSettings 만 스왑을 넘어 컨테이너가 소유해야 한다"는 전제로 배선을
결정했다(§3.2). 이 전제는 **교차 섹션 관찰**(한 값을 여러 섹션이 봐야 하는가)만 검토했고,
**전체화면 스왑 게이트로 인한 마운트 수명 축소**(ThemeEditor/SnippetEditor early-return 이
섹션 서브트리 전체를 언마운트하는가)라는 별도 축을 놓쳤다. 원본(6604f15)은
`themeEditorState`/`isSnippetEditorOpen` 게이트가 **SettingsView 자신을 언마운트하지
않는 조건부 return** 이었으므로, 컨테이너에 있던 모든 state·구독·뮤테이션 옵저버가 스왑 중에도
살아 있었다. 분해 후 같은 게이트가 섹션 서브트리를 언마운트하는 위치로 남았는데, 그 값들 중
일부가 섹션으로 내려가면서 원본에 없던 "스왑 중 상태 소실·이벤트 구독 해제·뮤테이션 콜백
유실"이 생겼다. 수정 원칙은 **"스왑을 넘어 살아야 하는 관심사는 컨테이너 소유"**로
환원하는 것 — 이미 §3.2 가 `updateSettings`/`isUpdatingSettings` 에 적용한 것과 동일한
논리를 나머지 사례에도 일관 적용한다. 오버레이 렌더(게이트를 return 대신 CSS 은닉으로
바꿔 섹션 트리를 항상 마운트 유지)는 렌더 구조 자체를 바꾸는 변경이라 채택하지 않았다 — §1
의 "동작·마크업 무변경" 중 "마크업"(렌더 트리 동등)까지 건드리는 더 큰 개입이며, §3 가 이미
확정한 12파일 분해 경계를 흔든다.

### 4.2 confirmed 수정

- **useLspInstallProgressSync (equivalence-3=wiring-1=contract-1)**: `settings-lsp-section.tsx`
  에서 호출을 제거하고 `settings-view.tsx` 의 훅 구간(early return 이전)으로 되돌렸다. 값
  읽기(`useLspInstallProgress`)와 설치/취소 뮤테이션은 섹션에 그대로 둔다 — 이 훅은 반환값이
  없는 순수 구독이라 컨테이너 이전이 곧 원본의 "훅 수명" 계약 복원이다.
  `entities/lsp/lsp.query.ts` 의 JSDoc 도 소비처를 `settings-lsp-section.tsx`(값 읽기)와
  `settings-view.tsx`(구독)로 정확히 갱신했다(equivalence-4/contract-2 겸함).
- **issuedRemoteUrl + 원격 링크 발급 뮤테이션 (equivalence-1=wiring-2)**: `issuedRemoteUrl`
  state 와 `useIssueRemoteLink()` 훅 호출을 `settings-view.tsx` 로 되돌렸다. Remote 섹션은
  `issuedUrl`(값)·`onIssuedUrlChange`(setter)·`issueRemoteLink`(mutate)·`isIssuingRemoteLink`
  4개 prop 을 받아 기존 핸들러 로직(`handleIssueRemoteLink` 의 클립보드 복사 + toast)을
  그대로 유지한다. `updateSettings`/`isUpdatingSettings` 와 동일한 "컨테이너가 옵저버를 쥐고
  mutate+isPending 을 prop 주입" 형태다.
- **sync 4종 뮤테이션 + isSyncConflictOpen (equivalence-2 의 sync/충돌 다이얼로그 부분)**:
  `useConnectSync`/`useDisconnectSync`/`useUploadSync`/`useDownloadSync` 4개 훅 호출과
  `isSyncConflictOpen` state 를 `settings-view.tsx` 로 되돌렸다. Sync 섹션은 4개
  mutate+isPending 쌍과 `isSyncConflictOpen`/`onSyncConflictOpenChange` 를 prop 으로 받아
  기존 핸들러(토스트·충돌 다이얼로그 오픈)를 그대로 유지한다. `SyncConflictDialog` JSX 는
  Portal 렌더라는 §3.2 의 기존 논증대로 Sync 섹션 파일에 남겼다 — state 만 컨테이너 소유로
  바뀌었을 뿐 다이얼로그 자체의 마운트 위치는 스왑 복귀 시 여전히 정확한 open 값으로
  재렌더된다.

**TanStack Query v5 의미 실사(원칙 15) 및 형태 선택**: `node_modules/@tanstack/query-core`
소스(`mutation.js`/`mutationObserver.js`)를 직접 대조했다. `useMutation({ onSuccess,
onError, ... })` 처럼 **훅 레벨**로 준 콜백은 `Mutation.execute()` 가 `this.options.onSuccess`
를 관찰자 존재 여부와 무관하게 무조건 호출한다(`mutation.js` 136·176행). 반면
`mutate(vars, { onSuccess, onError })` 처럼 **콜사이트**로 준 콜백은
`mutationObserver.js` 의 `notify_fn` 이 `this.hasListeners()` 로 게이트한다(98행) — 관찰자를
가진 컴포넌트가 언마운트되면 실행되지 않는다. 이번 실패 사례(동기화 토스트·충돌
다이얼로그·원격 링크 클립보드 복사)는 전부 **콜사이트** 패턴이었고, 그 콜백이 `setState`(컨테이너로
옮긴 `isSyncConflictOpen`/`issuedRemoteUrl` 의 setter)를 호출해야 하므로 "훅 레벨로 옮겨
관찰자 무관 실행"(대안 b)만으로는 불충분하다 — `useMutation()` 자체가 여전히 섹션 안에서
호출되면 그 관찰자(옵저버)가 스왑 중 사라지는 문제는 남고, 옵저버가 사라져도 훅 레벨 콜백은
실행되지만 그 콜백이 참조하는 state setter 가 컨테이너 소유가 아니면 언마운트된 섹션의
setState 호출은 무의미하기 때문이다. 따라서 **"뮤테이션 훅 호출 자체를 컨테이너로 되돌리는"
형태(대안 a)를 채택**했다 — 이미 `updateSettings`/`isUpdatingSettings` 에 적용된 선례와
정확히 같은 형태이고, entities 레이어의 훅 시그니처를 바꾸지 않아도 되는 더 작은 diff다.

### 4.3 wiring-3 잔여(ai 토큰·lsp 설치 토스트) — 낮은 관찰성 근거로 기록, 미수정

`settings-ai-section.tsx` 의 `setAiToken(..., { onError: toast.error })` 와
`settings-lsp-section.tsx` 의 `installLspServer`/`cancelLspInstall` 콜사이트 `onError`
토스트도 이론상 동일 클래스(콜사이트 콜백, 스왑 중 유실 가능)이나, 컨테이너로 옮기지 않고
현재 상태로 남긴다. 근거:

- 두 경우 모두 **훅 레벨 콜백(entities 소유)이 이미 상태를 정확히 복원**한다 —
  `useSetAiToken`/`useClearAiToken` 의 onSuccess 는 `aiTokenStatus`/`aiModels` 쿼리를
  무효화하므로(훅 레벨, 언마운트 무관 실행) 스왑 복귀 시 `AiProviderTokenRow` 의 `configured`
  값은 항상 최신이다. `useInstallLspServer` 의 `onError` 도 훅 레벨로 `dropIfSettled` 를
  실행해 진행 스토어를 정리하고, 성공 시 `QUERY_KEY.LSP.SERVERS` 를 무효화한다.
  유실되는 것은 콜사이트의 **토스트 알림 하나뿐**이고, 화면 상태(설치됨/실패 표시)는
  fix 4.2 로 되살아난 `useLspInstallProgressSync` 구독 덕에 `lsp-server-status-list.tsx` 의
  `isFailed` 분기로 이미 보인다.
- sync/remote 사례와 달리 **로컬 state 를 되살리는 setter 가 없다** — 콜백이 하는 일이
  toast 하나뿐이라 훅째로 컨테이너에 올릴 필요가 없다(sync/remote 는 컨테이너 state 를
  갱신해야 해서 훅도 함께 옮겨야 했다).
- 발생 조건이 좁다 — AI 토큰 저장은 네트워크 검증 1회 왕복(대개 1초 내), LSP 설치/취소
  실패도 실패 응답이 오는 시점에 정확히 스왑을 트리거해야 하는 좁은 창이다.

이 항목을 기계 검증 없이 남기는 결정이므로, 재발 시(사용자가 실제로 겪으면) 이 문단을 근거로
컨테이너 이전(4.2 와 동일 형태)을 재검토한다.

### 4.4 그 외 minor 반영

- **Sync 섹션의 settings 자체 재조회 통일 (design-2=contract-5)**: `settings-sync-section.tsx`
  의 `useQuery(settingsQueryOptions())` 를 제거하고 나머지 7개 섹션과 동일하게
  `settings: Settings` prop 을 받는다(`settings-view.tsx` 가 `settings={settings}` 를
  주입). `settings?.syncGistId ?? null` 의 옵셔널 체이닝도 non-null `settings.syncGistId ??
  null` 로 되돌렸다 — 컨테이너의 blank-render 게이트가 이미 non-null 을 보증하므로 원래
  불필요했던 경로다.
- **ThemeEditorState 소유 역전 (contract-4)**: 타입 정의를 `settings-appearance-section.tsx`
  에서 `settings-view.tsx`(state `themeEditorState` 의 실제 소유자)로 옮기고,
  `Pick<ComponentProps<typeof ThemeEditor>, 'mode' | 'sourceThemeId'>` 로 유도해 손으로
  다시 적지 않는다(common.md §5.3) — `@widgets/theme-editor/theme-editor` 가 이 값의
  실제 계약 대상이므로 그 props 에서 파생하는 편이 독립 리터럴 유지보다 정확하다. 섹션은
  `import type { ThemeEditorState } from '@widgets/settings-view/settings-view'` 로
  가져온다(타입 전용 import 라 순환 참조가 런타임에 발생하지 않음 — `tsc`/`vite build`
  로 확인).
- **handleOpenAppDataFolder 모듈 스코프 승격 (design-5)**: 클로저 캡처가 없는 순수
  래퍼라서 `settings-view.tsx` 컴포넌트 밖 모듈 스코프로 올렸다. prop 배선(3개 섹션)은
  형태 변경 없이 유지.
- **useOpenTab 이중 배선 통합 (design-4)**: `entities/layout/layout.query.ts` 에
  `useOpenAppFileTab(projectId)`(내부에서 `useOpenTab` + `layoutQueryOptions` +
  `currentWindowFocusedPane` + 에러 토스트를 묶음)를 추가하고, `settings-view.tsx`(설정
  파일 열기)와 `settings-ai-section.tsx`(프롬프트 파일 열기)가 이를 호출하도록 통합했다.
  `command-palette.tsx` 의 3번째 사용처는 이번 T2-B 범위(settings-view) 밖이라 손대지
  않았다(제안된 흡수는 별건으로 남긴다).
- **docs/features/vsix-theme-import.md 갱신 (equivalence-4 부수)**: VSIX 테마 임포트의
  실배선 위치가 `settings-view.tsx` 가 아니라 `widgets/plugin-manager/plugin-manager.tsx`
  (PLUGINS 섹션이 임베드)임을 반영했다. 이 stale 참조는 T2-B 이전부터 있던 것으로 이번
  분해가 만든 문제는 아니다.

### 4.5 실사 후 미수정 · 근거만 기록

- **design-3 (TOC labelKey ↔섹션 title 키 중복)**: 12쌍 전부 현재 일치함을 재확인했다.
  고치려면 `id` 처럼 `title` 도 컨테이너가 prop 주입해야 하는데, 이는 12개 섹션 파일 전부의
  시그니처를 건드리는 규모라 이번 대응(수명 버그 수정 중심)의 비례를 벗어난다고 판단해
  미수정. 코드 변경 없이 이 문단으로 근거를 남긴다 — 두 키가 어긋나는 실제 회귀가 생기면
  그때 컨테이너 주입으로 통일한다.
- **design-1 (Switch 행 프리미티브 부재, 26회 중복)**: 재설계급 항목이라 이번 배치 범위 밖으로
  이월한다. 원 리뷰가 제안한 `features/settings/switch-field.tsx`(또는 `AgentHooksToggle`
  일반화) 추출은 후속 배치로 남긴다.

### 4.6 검증 (Phase E 반영 후)

`bun run tsc --noEmit` 클린 → `eslint`(대상 파일 전부) 클린 → `prettier --check` 통과 →
`bun test` 1435 pass/0 fail(변경 없음, 위젯 레이어에 테스트 없음은 §3.5 그대로) →
`bun run verify`(rust fmt/clippy/test 포함) exit 0 → `bunx vite build` exit 0(청크 크기
경고는 §3.5 와 동일하게 기존 것) → `src/shared/api/bindings.ts` git diff 없음(무변경
확인). 파일별 최종 줄수: `settings-view.tsx` 201→223, `settings-appearance-section.tsx`
92→91(타입 정의 이관으로 순감소), `settings-remote-section.tsx` 81→85,
`settings-sync-section.tsx` 81→98, `settings-lsp-section.tsx` 47→39,
`settings-ai-section.tsx` 142→130, `entities/layout/layout.query.ts` 115→132(새 헬퍼
`useOpenAppFileTab`), `entities/lsp/lsp.query.ts` JSDoc 갱신만(순수 로직 변화 없음).
지시 밖 파일(§1 범위인
settings-view 12파일 + entities 배선 헬퍼 2곳 + 문서 2곳 이외) 수정 없음.
