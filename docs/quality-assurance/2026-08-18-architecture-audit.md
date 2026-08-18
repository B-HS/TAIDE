# TAIDE 아키텍처·추상화 전수 감사 보고서 (2026-08-18)

> 기준 커밋 5f15a84 · 표준 16배치(프론트 F1~F7·Rust R1~R8·계약 X1, opus+xhigh 읽기 전용) + 2단 종합.
> 발견 297건(고유 289·critical 19·major 146·minor 133) + 압축 제외 126 = 관측 총 423.
> 메인 실물 재검증: 1차 4건(watcher .git·project_close·ime-debug·auto-save) + 2차 보안/데이터 5건
> (agent_cli_install·lsp_install 원격 허용·forbid_directory 0·search_replace 가드부재·pty_kill 호출0) 전건 확정.
> 진행: 감사 wf_10697c92-2cc(16배치) → 1차 종합 → 2차 종합 wf_1141da0d-61d(6배치 병합).

---

# Part 1 — 1차 종합 (10배치 기준 골격 · §2.1·§3 C1~C12·§4·§5 유효)

# TAIDE 아키텍처 감사 종합 보고서 (초안)

기준 커밋 `5f15a84` · 종합 시점 2026-08-18 · 읽기 전용(수정·실행 없음)

---

## 0. 수신 범위와 한계 (먼저 확인 필요)

**16배치를 종합하라는 지시였으나 실제로 전달된 원시 발견은 10배치(F1~F7, R1~R3)입니다.** 추가로 R3 는 8번째 발견(`퍼센트 디코더 이중화`) 중간에서 잘려 있습니다.

| 항목 | 상태 |
|---|---|
| 수신 배치 | F1, F2, F3, F4, F5, F6, F7, R1, R2, R3 (10개) |
| 미수신 배치 | 6개 (Rust 도메인 후반 R4~R6 및 계약/문서 정합 X 계열로 추정) |
| 불완전 배치 | R3 (findings 8번째 이후 절단) |

따라서 아래 통계·클러스터·티어는 **수신된 10배치 기준의 잠정 정본**입니다. 특히 다음 영역은 근거가 비어 있어 결론을 내리지 않았습니다: Rust domain 의 git·file·terminal·lsp·ai·sync·plugin·vsix·search·settings·theme·snippet·ide·agent 도메인 내부, 그리고 bindings↔dispatch↔docs 계약 정합 축. **R1·R2·R3 이 이 도메인들을 "참조만" 하며 남긴 관통 지적(예: theme_delete 경로 취약점, 전역 락 안 git 네트워크 IO)이 미수신 배치와 중복될 가능성이 높으므로, 6배치 수신 후 소유권 재조정이 필요합니다.**

---

## 1. 전체 통계

### 1.1 배치별 · severity별

| 배치 | 범위 | critical | major | minor | 소계 | 압축 제외(계수만) |
|---|---|---:|---:|---:|---:|---:|
| F1 | app + entities (99파일 3.5K LOC) | 0 | 7 | 13 | 20 | 9 |
| F2 | features (94파일 6.9K LOC) | 0 | 5 | 9 | 14 | 7 |
| F3 | widgets 에디터 계열 (21파일 4.7K LOC) | 2 | 8 | 8 | 18 | 10 |
| F4 | widgets 셸·패널 (33파일 4.5K LOC) | 1 | 11 | 10 | 22 | 6 |
| F5 | widgets 설정·확장 (15파일 2.6K LOC) | 1 | 7 | 11 | 19 | 10 |
| F6 | shared/lib (137파일) | 1 | 10 | 9 | 20 | 10 |
| F7 | shared/lsp·ui·hooks·constants (63파일) | 1 | 8 | 9 | 18 | 7 |
| R1 | Rust 셸 코어 + app/window/layout (약 5.0K LOC) | 2 | 11 | 7 | 20 | 8 |
| R2 | Rust infra (14파일 2.7K LOC) | 1 | 10 | 8 | 19 | 8 |
| R3 | Rust remote (9파일 3.9K LOC, 절단) | 0 | 7 | 1 | 8+ | 10 |
| **합계** | **약 490파일** | **9** | **84** | **85** | **178** | **85** |

압축 제외분까지 포함하면 관측 총량은 **263건**입니다.

### 1.2 렌즈별 분포 (주 렌즈 기준, 178건)

| 렌즈 | 건수 | 비중 | 집중 배치 |
|---|---:|---:|---|
| L-CODE (중복·매직값·네이밍) | 34 | 19% | 전 배치 |
| L-BRIDGE (싱글톤·구독 수명·창 격리) | 27 | 15% | F1·F3·F6·F7·R3 |
| L-FSD / L-RUST-LAYER (레이어 배치) | 26 | 15% | F2·F3·F4·F5·F6·R1·R2 |
| L-QUERY (서버상태 계약) | 18 | 10% | F1·F3·F4·F5 |
| L-TYPE (파생·단언) | 17 | 10% | F1·F5·F6·F7 |
| L-RUST-STATE (락·소유권) | 13 | 7% | R1·R2·R3 |
| L-A11Y | 9 | 5% | F2·F3·F4·F5·F7 |
| L-I18N | 9 | 5% | F1·F3·F4·F5·F6·F7·R1·R2 |
| L-SFC (단일 책임·비대) | 9 | 5% | F3·F4·F5 |
| L-CONTRACT / 보안 | 9 | 5% | R3·R2 |
| L-EFFECT | 7 | 4% | F2·F3·F4·F5·F7 |

### 1.3 기계 검증 축 (전 배치 재확인 결과 위반 0)

FSD 역방향 import(eslint `no-restricted-imports` 강제) · barrel `index.ts` · `any` · `enum` · `function` 키워드 · `useCallback`/`useMemo` · `@ts-ignore`/`eslint-disable` · Rust 프로덕션 경로 `unwrap`/`expect`/`panic`.

**시사점: 린터가 잡는 축은 전부 깨끗합니다. 이번 감사에서 나온 178건은 예외 없이 "규약이 문서에만 있고 기계가 강제하지 않는 축"에 몰려 있습니다.** 이것이 이 보고서의 단일 최상위 결론이며, 티어 설계의 기준입니다.

---

## 2. 실코드 재확인 결과 (하중 주장 검증)

의심스럽거나 파급이 큰 주장 15건을 실제 소스로 재확인했습니다. **전건 채택**, 다만 1건은 severity 하향입니다.

| # | 주장 | 판정 | 재확인 근거 |
|---|---|---|---|
| R2-1 | `.git` 이 무시목록에 있어 git 워처 100% 무력 | **채택(강화)** | `constants.rs:1-2` IGNORED_DIR_NAMES 첫 원소가 `.git`, `watcher.rs:102-105` 가 경로 **전 컴포넌트**를 검사. 테스트 `무시_디렉토리_하위_경로는_필터링된다` 가 `/repo/.git/HEAD` 필터링을 **정상 동작으로 고정**하고 있어 회귀 테스트가 결함을 보증 중. 추가 확인: 프로젝트 루트가 `dist`/`build`/`target` 등을 조상 경로에 가지면 일반 파일 워처도 전면 무력화 — 원 배치보다 영향이 넓음 |
| R1-1 | `project_close` 가 dirty 레이아웃 유실 | **채택** | `project/commands.rs` 가 `layouts.write().remove()` 만 하고 `dirty_layouts` 미정리, `lib.rs:300-312` flusher 의 `filter_map` 이 무경고 폐기. `save_layout` 호출부는 flusher 1곳뿐 |
| R1-2 | `theme_delete` 경로 검증 부재 | **채택** | `theme/service.rs` — `save_theme` 은 `contains(['/','\\','.'])` 검증, `delete_theme` 은 builtin 체크 후 곧바로 `themes_dir().join(format!("{theme_id}.json"))` remove_file. 같은 파일 안에서 검증이 비대칭 |
| F6-1 | ime-debug 가 터미널 입력 원문 상시 수집 | **채택** | `ime-debug.ts` 에 dev 게이트·마스킹 없음(300건×40자). `terminal-view.tsx:252` 가 `term.onData`(사용자 키 입력) 원문 전달. `command-registry.ts:261` 이 전량 클립보드 복사. `clearImeDebug` 프로덕션 호출부 0 |
| F7-1 | `isWithinRoot` 가 문자열 prefix 비교 | **채택** | `workspace-edit-applier.ts` — `path === root \|\| path.startsWith(\`${root}/\`)`, 정규화 없음. Windows 백슬래시에서는 전량 거부되는 두 번째 결함도 동반 |
| F3-1 | editor-instance-registry 키가 monaco 수명과 어긋남 | **채택(강화)** | 등록은 `editor-pane.tsx:496` 한 곳(`handleEditorMount`), `code-editor.tsx` 는 마운트 시 1회 생성 + 경로 변경은 `setModel` 로만 처리. `pane-node-view.tsx:118` 만 `key` 없음(형제 7종은 전부 `key={activeTab.id}`). 추가 확인: `if (isPending) return <div/>` 분기가 있어 **미캐시 전환 시 새 tabId 로 unregister 되어 옛 tabId 에 dispose 된 에디터가 남고, 캐시 전환 시엔 새 tabId 가 아예 등록되지 않음** — 두 분기 모두 깨짐. 소비자 4곳(⌘S 저장·Ln/Col·breadcrumbs) |
| F3-2 | auto-save 타이머가 경로 전환을 넘음 | **채택(조건 한정)** | `handleChange` 가 클로저 `path` 로 arm, 취소는 `handleChange`/`handleSave`/언마운트뿐이고 `[path]` 의존 취소 없음. `draftRef` 는 경로 전환 시 null 리셋되나 한 글자 입력으로 재충전. 재현 조건은 좁으나(자동저장 on + 지연 내 전환 + 새 파일 편집) 결과가 **A 경로에 B 내용 기록** |
| F4-1 | IDE 프로토콜 전체를 상태바 위젯이 소유 | **채택** | `status-bar-content.tsx:147·169·196·200` 이 3개 IDE 이벤트 + `useIdeStatusSync` 의 유일 마운트 지점, `app-shell.tsx:229` `{!(zen && hideStatusBar) && <StatusBarContent/>}` |
| F4-2 | `useAgentStateSync` 가 사이드바에만 | **채택** | 소비처 `app-sidebar.tsx:61` 단 1곳, `app-shell.tsx:194` `{!zen && <AppSidebar/>}` |
| F5-1 | monaco 키바인딩 오버라이드가 다이얼로그 소유 | **채택 · severity 하향** | `applyMonacoKeybindingOverrides` 호출부는 `keybindings-editor.tsx:166` 단 1곳, `app.tsx` 보조 창 분기는 `KeybindingsEditor` 미마운트(주석이 3개 다이얼로그 미마운트를 명시하나 이 부수효과는 미언급) |
| F1-1 | wait-marker 해제가 단일 경로 | **채택** | `registerWaitMarker` 는 `agent-external-open-provider.tsx:52`(메인 창 전용), `takeWaitMarkers` 는 `layout.query.ts:55`(useCloseTab) 각각 1곳 |
| F1-3 | 모든 settings 저장이 THEME/LOCALE 무효화 | **채택** | `settings.query.ts` `useUpdateSettings.onSuccess` 가 patch 무관 무조건 2스코프 invalidate, `search-history.ts` 의 `useAddRecentSearch` 가 같은 훅 사용 |
| F1-4 | projectClosed 캐시 정리가 3스코프뿐 | **채택** | `ipc-sync-provider.tsx` 가 PROJECT.DETAIL·LAYOUT.DETAIL·TREE.ROWS 만 removeQueries |
| C1 | 순수 표시 컴포넌트의 widgets 잔류 / query 를 쥔 features | **채택** | widgets 측 8파일 전부 query·mutation·ipc 0건. features 측 query 보유는 정확히 4파일(`agent-cli-status-row`, `agent-hooks-project-list`, `agent-hooks-project-row`, `use-zen-mode`) |
| R3-1 | 원격 sink 가 항상 `Ok(())` | **채택** | `ws.rs` — `let _ = sink_out.send(...)` 후 무조건 `Ok(())`. pty·lsp 도메인의 `retain(... .is_ok())` 프루닝이 원격에서만 무력 |

### 2.1 각 배치가 이미 기각한 시드 — 재론하지 않음 (승계 확인)

| 기각 항목 | 근거 | 종합 판정 |
|---|---|---|
| `code-editor.tsx` / `ai-inline-edit.ts` 의 entities `.ipc` 직접 호출 | Wave G 계약이 "기존 선례와 동일한 FSD 합법 방향"으로 기결 | 승계 기각 |
| 미러 `setQueryData` (editor-pane) | Wave B 하드닝 계약 §3.2 승인 | 승계 기각 |
| `extract_zip` 하드닝 이원화 (plugin vs LSP) | LSP 아카이브는 sha256 필수 게이트, tar 는 mode 마스킹 확인 | 승계 기각 |
| `protocol.ts` 301줄 수동 타입 | LSP 와이어 스펙에 파생 원본 없음(의존성 부재), `as const`+union 일관 사용 | 승계 기각 |
| shadcn 벤더 14파일의 `function`·다중 export | architecture.md 가 `ui(shadcn vendored)` 명문화, upstream 형상 유지가 재생성 가능성 보장 | 승계 기각 |
| 보조 창 닫기 미차단 · 핫엑싯 핸드셰이크 · AppFile 경로 은닉 · 레이아웃 v2 마이그레이션 | acknowledge 기결 | 승계 기각 |
| `agent.query.ts` 등의 `setQueryData`(서버 확정 상태 반환) | acknowledge 근거 코드에 기재 | 승계 기각 |

---

## 3. 중복·동일 근원 클러스터 (교차 배치)

**178건 중 61건(34%)이 12개 클러스터로 수렴합니다.** 클러스터 단위 수정이 개별 수정보다 압도적으로 효율적인 구간입니다.

### C1. 레이어 배치 오류 — features ↔ widgets 경계 (6건 / 4배치)

| 방향 | 항목 | 출처 |
|---|---|---|
| features → widgets 승격 필요 | `agent-hooks-project-row`, `agent-hooks-project-list`, `agent-cli-status-row` (query/mutation 소유) | F2#1 |
| " | `use-zen-mode.ts` (조회+변이+IPC+전역 키맵 소유, 소비처 1곳) | F2#2 |
| widgets → features 강등 필요 | `tab-context-menu`, `sortable-tab` | F3#8 |
| " | `file-tree`, `problems-panel`, `search-panel`, `outline-panel` | F4#12 |
| " | `plugin-list-body`, `vsix-import-grammars-section`, `snippet-entry-editor`, `snippet-file-list` | F5#6 |
| features → shared 강등 | `vsix-theme-import`, `selection-line-range` (React 무관, 소비처 widgets 전용) | F2#6 |

**동일 근원**: eslint 는 import **방향**만 검사하고 레이어의 **성격**(query 보유 여부)은 검사하지 않는다. 그 결과 한 슬라이스가 두 레이어로 갈라져 있다(explorer·problems·search·outline·settings·plugin·snippet 7개 슬라이스). 총 12파일 이동 + 4파일 props 리프팅.

### C2. 중복 구현 — "2회 이상 룰" 위반 (14건 / 8배치, 최대 클러스터)

| 중복 대상 | 사본 수 | 출처 |
|---|---:|---|
| 이벤트 브리지 `Set<Listener>` 보일러플레이트 | 12 | F6#2 |
| `NOOP_DISPOSABLE` (LSP 어댑터) | 16 | F7#7 |
| `fileNameOf` (shared 원본 존재) | 6 (+인라인 3) | **F3#13 · F4#8 · F6#14 — 3배치 중복 보고** |
| 확장자 추출 `lastIndexOf('.')` | 4 | F6#13 |
| `constant_time_eq` (Rust) | 3 | R2#8 |
| layout 커맨드 read-clone-mutate-writeback | 18 | R1#11 |
| `CodeEditor` 설정 프롭 배선 25개 + 기본값 상수 4종 | 3 | F3#5 |
| 편집기/터미널 기본값 상수 (shared·Rust·위젯) | 5~6 | **F3#5 · F5#5 중복** |
| LSP 문서 심볼 로딩 절차 30줄 | 2 | F4#7 |
| LSP 서버 선택 술어 (주석이 lockstep 강제) | 3 | F3#7 |
| capability 판정 술어 | 27 + 25 (이중 관리) | F7#3 |
| ANSI 토큰 목록 16종 | 3 | F6#10 |
| Location→monaco 변환 (JSDoc 포함 복제) | 2 | F7#10 |
| `findLeafPane` · `toggleInSet` · 프로젝트 열기 흐름 · 설정 탭 열기 | 각 2 | F4#8 · F4#14 |

**동일 근원 2갈래**: (a) shared 에 원본이 있는데 상위가 재정의 — 발견 비용 문제(107파일 평면 배치, F6#12), (b) 추상화 자체가 없어 각자 손으로 — 브리지 팩토리·어댑터 계약 타입·layout 변이 헬퍼 부재.

### C3. 모듈 스코프 싱글톤의 소유권·정리 계약 부재 (14건 / 5배치)

| 레지스트리/브리지 | 결함 | 출처 |
|---|---|---|
| `agent-wait-marker-registry` | 해제 경로 1개(useCloseTab), 창 이동·프로젝트 종료 시 영구 누수 | F1#1 |
| `editor-instance-registry` | 키(tabId)와 등록 트리거(monaco 수명) 불일치 | F3#1 |
| `reveal-registry` | pending 만료·정리 없음, 나중 열기 시 커서 탈취 | F1#15 |
| `open-with-registry` | 정리 경로 없는 무한 증가 Map, `'preview'` 는 도달 불가 | **F1#16 · F4#18 중복 보고** |
| `claude-diff-registry` | 미해결 탭 종료 시 잔존 | F1#16 · F3(압축) |
| `terminal-write-bridge` | 큐 상한·TTL·탭 종료 정리 없음 | F6#18 |
| LSP `codeLens/refresh` | 전역 브로드캐스트 (semanticTokens 는 세션 스코프 — 같은 문제의 정답이 옆에 있음) | F7#4 |
| LSP diagnostics owner | `serverId` 기준이라 동일 서버 두 세션이 상호 덮어씀 | F7#5 |
| `server-request-handler-registry` | 해제 시 핸들러 동일성 미검사(client.ts 는 검사 — 규약 갈림) | F7#12 |
| `useMonacoMarkers` | 모듈 로드 시 전역 구독, 해제 불가 | F7#17 |
| shiki 하이라이터 | 전역 5개 + monaco 공개 API 몽키패치, 재초기화 경합 | F6#19 |
| 브리지 12종 | "구독자 0" 정책 미정의(무시 10 / 보류큐 1 / false 반환 1) | F6#3 |
| `remote` 창 라벨 | 고정 `'remote'` 라 동시 접속 시 LSP 채널 소유권 충돌 | F6#5 |
| 원격 채널 sink | 실패를 `Ok(())` 로 위장 → 도메인 프루닝 불변식 파괴 | R3#1 |

**동일 근원**: 멀티 윈도우(창별 JS realm) 도입 이후에도 레지스트리의 **소유권 범위**(창/프로젝트/탭)와 **수명 종료 시점**이 계약으로 선언되지 않았다. 창 간 탭 이동(`useMoveTabToWindow`)이 이 공백을 실결함으로 전환시켰다.

### C4. 앱 수명 부수효과가 조건부 렌더 위젯에 얹힘 (4건 / 2배치) — C3 의 특수형

| 부수효과 | 소유 위젯 | 사라지는 조건 |
|---|---|---|
| IDE(Claude Code) 프로토콜 3종 + 상태 동기화 + 진단 push | `StatusBarContent` | Zen + 상태바 숨김 |
| 에이전트 상태 push 동기화 | `AppSidebar` | Zen |
| monaco 키바인딩 오버라이드 적용 | `KeybindingsEditor` (다이얼로그) | 보조 창 전체 |
| 키맵 에디터 열기 브리지 구독 | 동상 | 보조 창 전체 |

**동일 근원**: "표시"와 "구독"의 소유자가 분리되지 않았다. 정답 형태는 이미 존재한다(`app/providers/*` 의 상시 마운트 프로바이더 계층).

### C5. 서버 상태 경계 위반 (12건 / 4배치)

- **effect 안 직접 fetch**: ThemeProvider→`listPlugins` (F1#2), git 읽기 4종 (F1#17), git tags→useState (F4#6)
- **캐시를 클라이언트 스토어로 전용**: LSP 설치 진행률 (F1#10), 테마 미리보기를 `THEME.CURRENT` 에 덮어씀 (F1#11)
- **무효화 책임 이탈**: settings 과잉 무효화 (F1#3), 프로젝트 종료 부분 정리 3/15 (F1#4), 위젯이 `GIT.PROJECT`·`FILE.CONTENT` 대신 무효화 (F3#4), remote 무효화 (F5#10)
- **entities 우회**: openTab raw IPC + setQueryData 수기 복제 3곳 (F4#4), AI 커밋 메시지·검색 치환 mutation 수기 구현 (F4#5), 쿼리 캐시 저수준 구독 (F3#18)

### C6. 타입 파생 미사용 · `as` 단언 (14건 / 6배치)

**단일 최대 근원 — Rust `Settings` 열거형 필드가 `Option<String>`:** `editorRenderWhitespace`/`editorCursorStyle`/`editorCursorBlinking`/`terminalCursorStyle`/`aiProvider` 가 bindings 상 `string | null` 이라, 프런트가 총 **9지점에서 `as` 로 좁힘** — F3#12(editor-pane·untitled-pane·app-file-pane 3파일 9곳), F4#10(terminal-session), F5#3(settings-view), F6#17(inline-completion). **4배치가 각각 독립 보고했으나 근본 원인은 Rust 한 곳.**

기타: `emptySettingsPatch` 54필드 수기 (F1#6), specta 우회 raw invoke 3종 (F1#7), `SearchMatchRowData` (F1#8), 에이전트 로스터 하드코딩 (F2#5), `SyntaxStyle` 동명 섀도잉 (F6#16), ANSI 토큰 3중 (F6#10), capability 술어 이중 (F7#3), `SymbolKind` 서수 이중 (F7#11), `ids.rs` 검증 없는 newtype (R1#13), 비전수 배열 (F5#11).

### C7. 비대 파일 / 단일 책임 붕괴 (11건 / 5배치)

| 파일 | LOC | 소유 관심사 |
|---|---:|---|
| `layout/service.rs` | 2,116 (실코드 976) | — |
| `editor-pane.tsx` | 1,050 | 8개 (미러·저장·git gutter·충돌·blame·프리뷰·IDE selection·LSP) |
| `settings-view.tsx` | 927 | 12섹션 + 31훅 + 서브뷰 스위치 |
| `lib.rs` | 778 | 부트스트랩 + 도메인 로직 5종 |
| `mapping-tables.ts` | 612 | 4개 (변환 테이블·토큰 어휘·LSP 시맨틱·그래프 레인) |
| `lsp-session-registry.ts` | 505 | 프로세스 전역 싱글톤 (레이어도 오배치) |
| `git-panel.tsx` | 396 | 33 props + 행 그룹 3중 복제 |
| `command-palette.tsx` | 384 | 5개 모드 |
| `explorer-container.tsx` | 351 | CRUD·클립보드·비교·검색·터미널 |
| `command-registry.ts` | 301 | 4개 (상수·팔레트 파서·레지스트리·카탈로그) |
| `shared/lib` 루트 | 107파일 평면 | monaco/keymap/bridge 가 하위 디렉토리와 이원화 |

**주의: F3#3 은 `editor-pane.tsx` 비대가 F3 의 critical 2건의 구조적 원인이라고 지목합니다.** 즉 C7 은 단순 가독성 부채가 아니라 C3·타이머 결함의 상류입니다.

### C8. 접근성 — `role='button'` 계열 (8건 / 5배치)

| 결함 등급 | 대상 |
|---|---|
| 키보드 활성화 **전무** | `status-row-item`(F2#4), `file-history-panel`(F3#9), `commit-detail-panel`·`commit-graph`(F4#17) |
| Enter 만 처리(Space 누락) | `problem-row`·`search-match-row`·`outline-symbol-row`·`file-group-header`(F2#7) |
| 구조 위반 | `FileGroupHeader` 가 `div[role=button]` 안에 Checkbox 중첩(F7#9) |
| ARIA 미충족 | color-picker 슬라이더 2종(F2#8), overlay-scrollbar `role=scrollbar` 필수 속성 부재(F7#14) |
| label 부재 | theme-editor·keybindings-editor·new-snippet-file-dialog 입력 4곳(F5#13) |

`shared/ui/file-group-header.tsx` 는 F2#7 과 F7#9 에서 서로 다른 결함으로 각각 보고됨 — 한 번에 처리 가능.

### C9. i18n 양방향 하드코딩 (9건 / 8배치)

- **Rust → 사용자 노출 문자열이 로케일 체계를 우회**: `AppError` 5변종 자유문자열이 `toast.error(error.message)` 로 직송, 생성 지점 369건 중 **한국어 101 / 영어 268 혼재**(R1#9), infra 12건 한국어(R2#10), remote WS 한국어(F6#12). **R1#9 와 R2#10 은 동일 근원.**
- **프런트 하드코딩**: blame 오버레이 영어 + 상대시간 포매터 2벌(F6#9), workspaceEdit 실패 사유 영어(F7#8), 탭 제목 `(diff)` — **F3#15 와 F4#19 가 동일 문자열 중복 보고**, 탐색기 비교 탭 제목(F4#19), `common.retry` 키 노출(F1#13), 대화상자 필터명·`/bin/zsh`(F5#14).

### C10. 경로 안전 · 보안 경계 (10건 / 4배치)

| 항목 | 상태 | 출처 |
|---|---|---|
| `isWithinRoot` 문자열 prefix (`..` 우회 + Windows 전량 거부) | **critical** | F7#1 |
| 무루트 applyEdit fallback 창(initialize 왕복 전체) | major | F7#2 |
| `theme_delete` 경로 컴포넌트 미검증(원격 노출) | **critical** | R1#2 |
| `AppPaths` 가 검증 없는 join 헬퍼만 제공 → 검사 4갈래 분산 | critical 동반 | R1#2 |
| `ids.rs` newtype 이 투명 래퍼 | major | R1#13 |
| `resolve_owning_project` 가 HashMap 순회 순서 의존 | major | R2#4 |
| `shell_integration` 이 셸 source 대상 스크립트를 공용 `/tmp` 기본 umask 로 기록 | major | R2#11 |
| nonce 쿠키만 `Secure` 누락(세션 쿠키는 조건부 부여) | major | R3#4 |
| `/__taide/file` 응답에 CSP·nosniff 부재(SVG 동일 오리진 실행 표면) | major | R3#5 |
| 키링 5계정 중 원격 거부는 1계정뿐(AI 토큰 3 + GitHub PAT 원격 변경 가능) | major | R3#7 |
| ime-debug 터미널 원문 상시 보관 + 클립보드 명령 | **critical** | F6#1 |

**동일 근원(경로 축)**: 검증된 경로 타입/유틸이 없고 안전 책임이 호출부 관습에 분산 — Rust 4갈래(`ensure_safe_component`·`contains(['/','\\','.'])`·`sanitize_snippet_file_name`·무검증), TS 는 POSIX 구분자 하드코딩(F7#16).

### C11. Rust 상태 소유권 · 락 (9건 / 3배치)

- `AppState` 데이터 필드 전부 `pub RwLock` — 불변식 강제 지점 없음, 직접 락 접근 133건(R1#12) → **C11 의 상류**. 이것이 R1#1(project_close 유실)·R1#4(발행 순서 두 구현 상반)의 직접 원인.
- 전역 `begin_mutation` 단일 락 96지점, git push/pull 이 락 안에서 동기 네트워크 IO(R1#3) — architecture.md §2.1 정면 위반.
- pty 일시정지 중 kill 시 스레드 2개 영구 누수(R2#3), pty 배치 락 보유 중 IPC 전송(R2#14), 원격 파일 전체 메모리 적재(R3#6), `load_layout` 백업 없는 침묵 초기화(R1#10).

### C12. 계약이 문서에만 있고 기계가 강제하지 않음 (5건 / 3배치)

| 계약 | 검증 장치 상태 |
|---|---|
| 이벤트 25종 등록 + 원격 팬아웃 23종(T0 #14 로 AgentExternalOpen 제외) | 파리티 테스트 **없음**(커맨드 축에는 `RAW_CHANNEL_COMMANDS` 선례 존재) — R1#8 |
| 원격 dispatch 181 match arm | 테스트가 arm 을 읽지 않음, arm 누락이 무증상 통과 — R3#2 |
| 원격 명시 거부 16종 | 테스트가 헬퍼 직접 호출 = **동어반복**, 라우팅 미검증 — R3#3 |
| LSP capability 판정 | client 레지스트리 27개 + 어댑터 inline 25곳 **이중 관리** — F7#3 |
| LSP 서버 선택 술어 | 주석으로만 "lockstep 필수" 강제, 사본 3개 — F3#7 |

---

## 4. severity 재조정

### 4.1 하향 1건

| 항목 | 원 등급 | 조정 | 근거 |
|---|---|---|---|
| F5#1 monaco 키바인딩 오버라이드 보조 창 미적용 | critical | **major** | 사실관계는 전건 확인. 그러나 영향은 "보조 창에서 사용자 재바인딩이 기본값으로 동작" — 데이터 손실·보안 손상·기능 무력화 어디에도 해당하지 않는 **기능 저하**. 같은 배치의 F5#7(KEYMAP 버튼 무반응)과 동일 계열이며 그쪽은 major. 등급 내 일관성을 위해 major 로 통일하고 C4 클러스터로 함께 처리 |

### 4.2 상향 검토 후 유지 3건 (근거 기록)

| 항목 | 원 등급 | 판정 | 근거 |
|---|---|---|---|
| F1#1 agent wait-marker 누수 | major | **유지 + T0 승격** | 외부 CLI 프로세스가 앱 종료까지 블록 = 앱 밖으로 새는 결함. critical 상향을 검토했으나 Rust `cleanup_all_wait_markers` 가 앱 종료 시 회수하므로 영구 손실은 없음. 등급은 major, **처리 순위는 T0** |
| F3#2 auto-save 경로 이월 | critical | **유지** | 재현 조건이 좁다(자동저장 on + 지연 내 탭 전환 + 새 파일 편집 + `savingRef` false)는 점을 근거로 하향을 검토했으나, 결과가 **사용자 파일 A 에 파일 B 내용을 기록**하는 비가역 데이터 파괴. 조건 협소는 발생 빈도를 낮출 뿐 등급을 낮추지 않음 |
| R2#1 watcher `.git` 필터 | critical | **유지 + 영향 확대** | 재확인 중 두 번째 영향 확인 — 프로젝트 루트가 `dist`/`build`/`target`/`.cache` 등을 **조상 경로**에 가지면 일반 파일 워처도 전면 무력화. 또한 기존 테스트가 이 동작을 정상으로 고정하고 있어 **수정 시 테스트도 함께 뒤집어야 함** |

### 4.3 과대 판정 검토 후 유지 (기각한 하향 제안)

- **F6#2 브리지 보일러플레이트 12중복(major)**: 순수 중복이라 minor 하향을 검토했으나, F6#3(구독자 0 정책 미정의)의 **구조적 원인**이며 두 건이 한 번에 해소된다. major 유지, 단일 작업으로 묶음.
- **R2#6 http client 매 호출 생성(major)**: 성능 이슈지만 인라인 컴플리션(타이핑마다 호출) 경로라 체감 직결. 유지.
- **R3#4 nonce 쿠키 Secure 누락(major)**: 1회용·단수명이나 인증 1단계 자격증명이고, **같은 파일의 세션 쿠키는 이미 조건부로 부여**하는 비대칭. 유지.

### 4.4 소유권 조정 필요 (미수신 배치와 중복 예상)

| 항목 | 보고 배치 | 예상 정본 소유 |
|---|---|---|
| `theme_delete` 경로 취약점 | R1#2 | theme 도메인 배치 |
| 전역 락 안 git 네트워크 IO | R1#3 | git 도메인 배치 |
| `pty_kill` 일시정지 누수 | R2#3 | terminal 도메인 배치 |
| LSP 다운로드 http 우회 | R2#5 | lsp 도메인 배치 |
| 키링 게이팅 비대칭 | R3#7 | ai/sync 도메인 배치 |

---

## 5. 수정 우선순위 티어

### T0 — 즉시 (11항목 · 약 20파일 · 위험도 저~중)

데이터 손실 / 보안 / 핵심 기능 무력화 중 **근본 수정이 국소적인 것**만 선별했습니다. 구조 리팩터링은 전부 T1 이후로 미룹니다.

| # | 항목 | 출처 | 파일 | 위험도 | 비고 |
|---:|---|---|---:|---|---|
| 1 | watcher `.git` 필터 — 감시 루트 기준 상대경로로 좁힘 | R2#1 | 1~2 | 중 | **기존 테스트를 함께 뒤집어야 함**. 수정 후 git 워처가 처음 살아나므로 이벤트 폭주 여부 확인 필요 |
| 2 | `project_close` 시 해당 프로젝트 레이아웃 동기 flush | R1#1 | 2 | 저 | flusher `filter_map` 미스를 `warn` 으로 노출 |
| 3 | `editor-instance-registry` 등록을 tabId 의존 effect 로 이동 | F3#1 | 2 | 중 | 또는 `EditorPane` 에 `key={tabId}` — 후자는 hot-exit 미러 전제 변경이라 계약 재확인 필요 |
| 4 | auto-save 타이머에 path 캡처 + 발화 시 일치 검사 | F3#2 | 1 | 저 | `previewTimeoutRef` 동반 처리 |
| 5 | `isWithinRoot` 경로 정규화 + 구분자 중립화 | F7#1 | 1~2 | 저 | Windows 전량 거부도 동시 해소 |
| 6 | 세션 applyEdit 핸들러를 `initialize` **이전** 등록, 전역 fallback 제거 | F7#2 | 2 | 저 | #5 와 한 작업 |
| 7 | `delete_theme`/`load_theme` 에 `ensure_safe_component` 적용 | R1#2 | 1 | 저 | theme 배치와 소유권 조정 |
| 8 | ime-debug 수집을 진단 플래그로 게이트(기본 비수집) | F6#1 | 2 | 저 | 유지 시 원문 대신 길이·코드포인트 분류로 축약 |
| 9 | IDE 프로토콜 3종 + 상태 sync + 진단 push 를 상시 프로바이더로 승격 | F4#1 | 3 | 중 | C4 의 대표 항목 |
| 10 | `pty_kill` 이 `set_paused(false)` 선행 (스레드 2개 누수) | R2#3 | 2 | 저 | `PtySession` 에 Drop 추가 검토 |
| 11 | wait-marker 해제를 탭 생명주기(또는 Rust AgentStore)로 이동 | F1#1 | 2~3 | 중 | 창 간 이동 존재하므로 realm 로컬 Map 은 부적합 |

**부수 권고**: T0 의 #1·#3·#4·#9·#11 은 전부 "잘못된 소유자에 매달린 수명" 문제입니다. 개별 수정과 동시에 C3/C4 의 소유권 계약을 `docs/architecture.md` 에 한 절로 명문화해야 T1 에서 같은 결함이 재발하지 않습니다.

**추가 T0 후보(판단 필요)**: `lsp_proc` Content-Length 상한(R2#2 — `rustc -O` 실측 패닉 확인, 다만 악성 언어 서버 전제라 신뢰 경계 판단에 따라 T1 가능), 원격 sink `Ok(())`(R3#1 — 원격 기능 사용 여부에 따라).

### T1 — 다음 정비 (8묶음 · 약 60~80파일 · 위험도 중)

계약 공백을 메우고 클러스터를 통합합니다. 각 묶음은 독립 커밋 단위로 설계했습니다.

| 묶음 | 내용 | 해소 건수 | 파일 | 위험도 |
|---|---|---:|---:|---|
| **T1-A 전역 부수효과 소유권** | C4 전량 — 에이전트 sync·monaco 키바인딩·키맵 브리지를 프로바이더 계층으로 승격 | 3 | 5~7 | 중 |
| **T1-B Settings 타입 좁히기** | Rust 열거형 필드를 specta union 으로 노출 → 프런트 `as` 9지점 + 비전수 배열 2건 제거 | 6 | 1 Rust + 6 TS | 중 (bindings 재생성) |
| **T1-C 서버 상태 계약 정리** | C5 전량 — effect fetch 제거, 무효화 책임 entities 회수, 프로젝트 스코프 키를 `PROJECT_SCOPED_KEYS` 로 파생, 캐시 전용 쿼리 2건 store 이관 | 12 | 12~15 | 중 |
| **T1-D 레지스트리 정리 계약** | C3 잔여 — 브리지 팩토리 2종 도입(fire-and-forget / external store)으로 12중복 + 구독자 0 정책 동시 해소, reveal·open-with·claude-diff·terminal-write 정리 훅, codeLens/diagnostics 세션 스코프화 | 10 | 18~20 | 중 |
| **T1-E 계약 검증 테스트** | C12 전량 — 이벤트 파리티, dispatch arm 파리티, 거부 16종 라우팅 테이블화, capability 술어 단일화, LSP 서버 선택 술어 통합 | 5 | 6~8 | 저 |
| **T1-F 레이어 배치 정정** | C1 전량 — 12파일 이동 + 4파일 props 리프팅. 이동 후 `architecture.md` §5 에 배치 기준 명문화 | 6 | 16 | 저 (import 경로만) |
| **T1-G 인프라 하드닝** | `/tmp` 스크립트 권한, nonce Secure, `/__taide/file` CSP·nosniff, `write_atomic` temp 정리, `resolve_owning_project` 결정성, 원격 파일 스트리밍, http 프로필 분리 + 싱글톤 | 9 | 8~10 | 중 |
| **T1-H 전역 락 IO 분리** | `begin_mutation` 안 git push/pull/fetch 를 `spawn_blocking` 밖으로, 락 단위 축 분리 검토 | 2 | 3~5 | **높음** (동시성 회귀 위험 — 별도 검증 필요) |

**T1-H 는 위험도가 높아 단독 진행을 권고합니다.** 96개 호출 지점이 단일 락을 공유하는 상태에서 락을 쪼개면 기존에 우연히 성립하던 순서 보장이 깨질 수 있습니다.

### T2 — 백로그 (약 120파일 · 위험도 저 · diff 큼)

| 묶음 | 내용 | 해소 건수 |
|---|---|---:|
| **T2-A 중복 제거 잔여** | C2 잔여 — `fileNameOf` 6+3, 확장자 추출 4, `NOOP_DISPOSABLE` 16, `constant_time_eq` 3, CodeEditor 프롭 배선 3, LSP 심볼 로딩 2, Location 변환 2, ANSI 토큰 3, 기본값 상수 6, layout 커맨드 18 | 14 |
| **T2-B 비대 파일 분해** | C7 — `editor-pane`(관심사별 훅), `settings-view`(섹션 위젯), `lib.rs`(도메인 이관), `command-registry`(4분할), `mapping-tables`(변환기 밖으로), `lsp-session-registry`(레이어 이동), `git-panel`/`command-palette`/`explorer-container` | 11 |
| **T2-C shared/lib 재구조화** | 107파일 평면 → `bridge/`·`keymap/`·`theme/` 신설 + monaco 계열 5파일 하위 합류, shared UI 3분할 정리 | 2 |
| **T2-D 접근성** | C8 — Enter/Space 공용 헬퍼 1회 추출로 4지점, 키보드 전무 4종, ARIA 3종, label 4곳 | 8 |
| **T2-E i18n 통일** | C9 — `AppError` 코드 taxonomy 화 + locale 카탈로그 이관(369지점, **대규모 · 별도 캠페인 권장**), 프런트 하드코딩 7건 | 9 |
| **T2-F dead code · 네이밍** | `entities/app` 슬라이스 3파일, `getLocale`, `app:ready` 이벤트, `WindowStore::new`, `SYSTEM_FONT_FAMILY_VALUE`, `graph-lanes.color`, `OpenWithOverride.'preview'`, `overwrite` 오도 네이밍, `usize::MAX` 센티널, `NO_MATCH_SCORE` 재사용 | 10 |
| **T2-G 매직넘버·상수 정본화** | DNS 63/253, 폰트 범위 8~32 vs 6~48 드리프트, 터미널 highlightLimit, explorer 행 메트릭, 레인 색 12 | 5 |
| **T2-H 문서 정합** | `architecture.md` infra 목록(12개 중 4개만 기재), features/widgets 예외 조항, JSDoc 언어 정책(`HANDOFF.md` 는 영어라고 적혀 있으나 실제 한국어 혼용), `PaneTreeRef` 인덱스 규약 | 4 |

**T2-E 의 `AppError` 코드화는 369지점 · Rust+TS 양측 · 로케일 카탈로그 확장이 얽혀 있어 T2 안에서도 별도 캠페인으로 분리해야 합니다.**

### 티어 요약

| 티어 | 항목/묶음 | 예상 파일 | 위험도 | 해소 건수(누적) |
|---|---:|---:|---|---:|
| T0 | 11 | 약 20 | 저~중 | 11 |
| T1 | 8묶음 | 60~80 | 중 (1묶음 높음) | 64 (75) |
| T2 | 8묶음 | 120+ | 저 | 63 (138) |
| 미배정 | — | — | — | 40 (개별 minor·압축 제외분) |

---

## 6. 종합 판단

1. **린터가 강제하는 축은 완전무결합니다.** FSD 방향·`any`·`enum`·`function`·수동 메모이제이션·검사기 억제 위반이 490파일에서 0건입니다. 이는 예외적으로 높은 수준입니다.

2. **178건 전부가 "문서에는 있으나 기계가 검사하지 않는 축"에서 나왔습니다.** 레이어의 성격(방향 아님), 싱글톤의 소유권 범위, 무효화 책임의 소재, 계약 목록의 파리티, 경로 안전 검증 지점. 따라서 개별 수정만으로는 재발을 막지 못하며, **T1-E(계약 검증 테스트)와 T1-F 의 문서 명문화가 T2 전체보다 우선순위가 높습니다.**

3. **멀티 윈도우 도입이 미결제 부채를 청구서로 바꿨습니다.** critical 9건 중 4건(F3#1, F4#1, F1#1, F5#1→major)과 C3·C4 클러스터 18건이 전부 "창별 realm 격리" 또는 "조건부 렌더 위젯의 부수효과 소유"에서 파생됩니다. Zen 모드와 보조 창이 없었다면 전부 잠복 상태였을 결함입니다.

4. **`editor-pane.tsx`(1,050 LOC)는 가독성 부채가 아니라 결함 생성기입니다.** 이 배치의 critical 2건이 모두 이 파일의 관심사 응집에서 나왔고, 두 결함 다 "타이머/등록이 `path`·`tabId` 를 의존성으로 갖지 않아서" 발생합니다. 관심사별 훅 분해(T2-B)를 T1 로 올리는 것을 검토할 가치가 있습니다.

5. **보안 축은 설계는 견고하되 검증 지점이 분산되어 있습니다.** root_guard 의 경계 로직·remote 의 Host/Origin 이중 게이트·nonce 오라클 차단·constant_time 비교·shell_quote 는 모두 정확합니다. 실공백은 "경계를 어디서 몇 번 검사하는가"가 4갈래로 갈린 데서 나왔습니다(C10).

---

## 7. 사용자 확인 요청

1. **미수신 6배치를 기다릴 것인가, 이 10배치 기준으로 T0 를 착수할 것인가** — 추천: T0 착수(미수신 배치와 중복 예상 항목 5건은 소유권 표시해 두었고, T0 11항목 중 그에 걸리는 것은 #7 하나뿐)
2. **T0 #1(watcher) 수정 시 기존 테스트를 뒤집는 것을 승인하는가** — 현재 테스트가 결함을 정상 동작으로 고정 중
3. **T0 #3(editor-instance-registry) 을 effect 이동으로 갈 것인가 `key={tabId}` 로 갈 것인가** — 후자는 hot-exit 미러 전제 변경이라 계약 재확인 필요. 추천: effect 이동(계약 무변경)
4. **F5#1 severity 하향(critical→major)을 승인하는가**
5. **T1-H(전역 락 IO 분리)를 T1 에 둘 것인가 별도 캠페인으로 뺄 것인가** — 추천: 별도(동시성 회귀 위험)
6. **T2-E(`AppError` 369지점 코드화)를 백로그에 둘 것인가 별도 캠페인으로 승격할 것인가** — 추천: 별도 캠페인(한/영 혼재가 사용자 노출 문구라 UX 영향 있음)
---

# Part 2 — 2차 종합 (전 16배치 병합 · 최종 통계·티어·확인문항 정본)

# 2차 종합: 병합 결과

기준 커밋 `5f15a84` · 2차 종합 시점 2026-08-18 · 읽기 전용
1차 보고서(`audit-synthesis.md`)의 §2.1 · §3(C1~C12 골격) · §4.1~4.3 · §5 T1-A/C/F 는 **그대로 유효**하며 아래에서 절 번호로만 참조합니다.

---

## 0. 수신 완료 — 1차 §0 대체

6배치(R4·R5·R6·R7·R8·X1) 전량 수신. 1차가 "근거가 비어 있어 결론을 내리지 않았다"고 유보한 영역(git·file·project·tree / locale·settings·theme·sync / ai·agent·ide / lsp·plugin·vsix·search / terminal·task·system·snippet·font / 계약 정합)이 모두 채워졌습니다. **1차의 유보는 해제되며, 아래 결론이 정본입니다.**

여전히 남은 결손 1건: R3(Rust remote)이 8번째 발견 중간에서 절단된 상태. 지시된 총량 305건과 아래 실집계 297건의 차이 8건이 이 절단분으로 추정됩니다.

---

## 1. 통계 갱신 — 1차 §1 대체

### 1.1 배치별 · severity별 (16배치)

| 구간 | 배치 | critical | major | minor | 소계 | 압축 제외 |
|---|---|---:|---:|---:|---:|---:|
| 1차(§1.1) | F1~F7, R1~R3 | 9 | 84 | 85 | 178 | 85 |
| R4 | git·file·project·tree (6,230 LOC) | 2 | 12 | 8 | 22 | 8 |
| R5 | locale·settings·theme·sync (12,100 LOC) | 1 | 10 | 5 | 16 | 5 |
| R6 | ai·agent·ide (7,580 LOC) | 1 | 11 | 9 | 21 | 6 |
| R7 | lsp·plugin·vsix·search (19파일) | 3 | 10 | 11 | 24 | 9 |
| R8 | terminal·task·system·snippet·font (2,253 LOC) | 2 | 9 | 7 | 18 | 6 |
| X1 | 계약 정합 (4중 대조) | 0 | 10 | 8 | 18 | 7 |
| **2차 소계** | 6배치 | **9** | **62** | **48** | **119** | **41** |
| **최종 합계** | **16배치** | **18** | **146** | **133** | **297** | **126** |

관측 총량 **423건**(297 + 압축 제외 126).

**교차 배치 중복 8쌍 확인** — 실제 고유 결함은 약 **289건**입니다: R3#1≡R8#3(원격 sink `Ok(())`), R1#3≡R4#3(전역 락 git IO), R2#3⊂R8#5(pty pause), R1#1↔R4#7(같은 `project_close` 의 다른 누락), R1#8≡X1#8(이벤트 파리티 부재), F1#11≡R5#10(테마 프리뷰 캐시 주입), F5≡R5#11(`settings-view.tsx` 927줄), R2#6↔R6#7(`create_outbound_http_client` 매 호출).

### 1.2 렌즈 분포 (297건, 주 렌즈)

| 렌즈 | 1차 | 2차 | 합계 | 비중 |
|---|---:|---:|---:|---:|
| L-CODE | 34 | 19 | 53 | 18% |
| L-FSD / L-RUST-LAYER | 26 | 15 | 41 | 14% |
| L-RUST-STATE | 13 | 23 | 36 | 12% |
| L-BRIDGE | 27 | 6 | 33 | 11% |
| L-CONTRACT / 보안 | 9 | 18 | 27 | 9% |
| L-TYPE | 17 | 7 | 24 | 8% |
| L-QUERY | 18 | 3 | 21 | 7% |
| **L-RUST-BOUND (신규 축)** | 0 | 15 | **15** | 5% |
| L-I18N | 9 | 1 | 10 | 3% |
| L-SFC | 9 | 1 | 10 | 3% |
| L-A11Y | 9 | 0 | 9 | 3% |
| L-EFFECT | 7 | 0 | 7 | 2% |
| L-RUST-ERR | 0 | 7 | 7 | 2% |
| PTY/프로세스 수명 | 0 | 4 | 4 | 1% |

**분포의 무게중심이 이동했습니다.** 1차는 렌더러(L-BRIDGE·L-QUERY·L-A11Y)가 상위였으나, 6배치 합산 후 상위 5개 중 3개가 Rust 축(L-RUST-LAYER·L-RUST-STATE·L-RUST-BOUND)입니다. 특히 **L-RUST-BOUND(도메인 경계) 15건은 1차에 존재하지 않던 축**이며, 이것이 신규 클러스터 C13 의 근거입니다.

### 1.3 기계 검증 축 — 1차 §1.3 결론 유지·확대

6배치 전량이 동일하게 재확인했습니다(FSD 역방향 import 0 · barrel 0 · `any`/`enum` 0 · `useCallback`/`useMemo` 0 · `function` 키워드는 vendored shadcn 한정 · Rust 프로덕션 경로 `unwrap`/`expect`/`panic!` 0).

**단, 1차의 "Rust 프로덕션 경로 패닉 0" 은 반례 1건이 나왔습니다** — `lsp/manifest.rs:11-13` 의 `servers()` 가 `expect("bundled lsp manifest must be valid")` 로 IPC 핸들러 경로(`lsp_spawn`·`lsp_resolve_root`·`lsp_install`)에서 호출됩니다(R7#13). 컴파일 타임 상수를 파싱하므로 실패 확률은 0 에 가깝지만, "프로덕션 경로 패닉 0" 이라는 단정은 **"IPC 핸들러 경로에 `expect` 1건 잔존"** 으로 정정합니다.

---

## 2. 실코드 재확인 — 신규 13건 (1차 §2 이어서)

6배치의 critical 9건 전량 + 파급이 큰 major 4건을 실제 소스로 재확인했습니다. **13건 중 채택 12 · 조정 1 · 기각 0.**

| # | 주장 | 판정 | 재확인 근거 |
|---|---|---|---|
| R4#2 | git `StatusRow.path` 가 repo 상대인데 file 도메인은 절대 전제 | **채택(강화)** | `git/service.rs:1560` `path: path.to_string()` = git2 repo 상대. 소비부 `git-panel-container.tsx:170` `openFileTab = (path) => openTab({ kind:{kind:'file',path} ...})` — **루트 join 없음**. `root_guard.rs:56-69` `canonicalize_lenient` 재귀 확인: 프로덕션(CWD=`/`)에서 `src/main.rs` → parent `src` → parent `""` → `file_name()` None → `InvalidArgument("유효하지 않은 경로입니다: ")` 정확히 재현. **강화**: dev(CWD=`src-tauri`)에서는 `src-tauri/src/main.rs` 가 실재하고 **프로젝트 루트 안이므로 `root_guard` 를 통과** — 거부되지 않고 다른 파일이 열려 그대로 저장 시 덮어씀. 같은 값이 `onCopyPath`·`onRevealInExplorer`(git-panel.tsx:228-229) 로도 흐름 |
| R8#1 | pty 세션 회수 경로 부재 | **채택** | `pty_kill|ptyKill` 전수 grep: 정의(`terminal/commands.rs:216`) · 등록(`lib.rs:202`) · dispatch arm(`dispatch.rs:919`) · 래퍼(`terminal.ipc.ts:28`) · 문서 3곳뿐, **호출부 0**. `project_close`(`project/commands.rs:143-171`) 실측 확인 — `layouts`/`watchers`/`git_watchers` 만 remove, `TerminalStore` 참조 0. `docs/features/terminal.md:155` 는 반대로 규정 |
| R7#2 | 검색 오프셋이 바이트인데 프런트는 UTF-16/1-기반 | **채택(강화)** | `search/service.rs:49` `haystack[search_from..].find(&needle)` → 바이트 인덱스. `:235` `column: match_start` 그대로 탑재. 소비: `search-match-row.tsx:21-23` `preview.slice(matchStart, matchEnd)`(UTF-16), `reveal-registry.ts:15` `{lineNumber, column}` → monaco. **강화**: `reveal-registry.ts:21` 시그니처가 `column = 1` 기본값이라 1-기반이 명시적 계약 — 즉 **ASCII 줄에서도 상시 1칸 어긋남**이 인코딩 문제와 별개로 확정 |
| R5#1 | `SettingsPatch` 병합이 "해제"를 표현 불가 | **채택** | `settings/service.rs:314·319-321·324` 전부 `.or_else(|| settings.x.clone())`. 유일 예외 `merge_ai_omlx_base_url`(`:268-274`)만 `Some("")` → `None` 처리. `font-picker.tsx:71` `onSelect={() => handleSelect(null)}` 확인 — 시스템 기본 선택이 백엔드에 도달 불가 |
| R6#1 | agent CLI 설치가 원격에서 실행 | **채택(보안)** | `dispatch.rs:636-637` `"agent_cli_install" => respond(agent::agent_cli_install().await)` 가 `:624` `"plugin_install"|"plugin_uninstall" => Err(deny_remote_plugin_install(name))` **바로 아래 줄**에 위치. 실행 경로 확인: `commands.rs:364-376` `run_cli_osascript` → `service.rs:139-149` `build_cli_install_apple_script` = `do shell script "mkdir -p /usr/local/bin && ln -sf ..." with administrator privileges`. **원격 세션이 데스크톱에 관리자 암호 프롬프트를 띄우고 `/usr/local/bin` 에 심링크를 만든다** |
| R7#1 | LSP 자동 재시작이 프로토콜 세션 미복구 | **채택(강화)** | `lsp/commands.rs:333-336` `spawn_process` 성공 → 즉시 `set_status(..., Running, None)`. `initialize` 는 `lsp-session-registry.ts:273` 세션 생성 시 1회뿐. `lspSessionStatusChanged` 구독자 grep → `bindings.ts:389` 외 0. **강화**: `QUERY_KEY.LSP.SESSIONS` 전수 grep 결과 **`lsp.query.ts:12` 의 정의 1곳뿐 — 무효화 지점 0**. 즉 상태바의 LSP 표시(`status-bar-content.tsx:131-135`)는 이벤트로도 무효화로도 갱신되지 않고, 죽은 세션을 `running` 으로 계속 보고 |
| R4#1 | tree 커맨드 4종의 스토어 전체 복제 | **채택** | `tree/commands.rs:61-65` `let mut trees = tree_store.0.read().clone();` → `rows_page`(읽기 전용) → `*tree_store.0.write() = trees;`. **`tree_rows` 만 `begin_mutation` 없음**(`:77`·`:94`·`:111` 은 보유) 확인 — 조회 경로가 배타 없이 맵 전체를 되쓴다 |
| R7#3 | `search_replace` 만 뮤테이션 가드 없음 | **채택** | `search/commands.rs:96-110` 본문에 `begin_mutation` 없음. 대비 확인: 같은 파일 `search_run:75`(스코프 블록 안), `search_cancel:114`, `file/commands.rs:27` `file_save` 는 첫 줄 |
| R8#2 | `pty_write` 가 전역 락 보유 중 블로킹 write | **채택** | `terminal/commands.rs:200-204` — `let sessions = store.0.lock();` 가드가 함수 끝까지 살아 있는 채 `entry.pty.write(...)`. 같은 락을 `pty_resize:208`·`pty_kill:218`·`pty_set_paused:228`·`pty_attach`·`terminal_sessions`·`foreground_pids` 가 공유 확인 |
| X1#3 | `agent:external-open` 원격 팬아웃 + drain 원격 허용 | **채택(보안)** | `lib.rs:664` fanout 목록에 `AgentExternalOpen` 포함(`HotExitFlushRequested` 는 `:291` 등록만 되고 fanout 제외 — QA6 결정과 일치). `dispatch.rs:78`·`:638` `agent_pending_external_opens` 가 허용목록·실핸들러 양쪽에 존재, deny arm 없음 |
| X1#7 | asset 프로토콜 스코프가 회수되지 않음 | **채택(보안)** | `allow_directory` 전수 grep → `project/commands.rs:18` 단 1곳. **`forbid_directory` 는 `src-tauri` 전체에 0건**. `project_close:143-171` 실측 — 스코프 회수 없음 |
| R7#11 | `lsp_install` 만 원격 허용 | **채택** | `dispatch.rs:666` `"lsp_install" => respond(lsp::lsp_install(...).await)` vs `:624`(plugin) · `:1000`(vsix_extract_themes) · `:1001`(vsix_import_plugin) 전부 deny arm. 같은 성격의 4개 중 1개만 통과 |
| X1#2 | 이벤트 6종 구독자 0 → "LSP 크래시가 사용자에게 전혀 안 보임" | **채택 · 일부 조정** | 이벤트 구독자 0 은 채택(grep 확인). 그러나 **"전혀 보이지 않는다"는 과장** — `status-bar-content.tsx:38-40·131-135` 가 `lspSessionsQueryOptions` 로 `crashed` 를 렌더한다. 다만 위 R7#1 재확인대로 그 쿼리는 무효화 지점이 0 이므로 **"이벤트 기반 즉시 반영이 없고 refetch 타이밍에만 지연 표시"** 로 표현을 정정 |

### 2.1 부수 확인 (요청 외 재확인 중 발견)

- **R4#10(`file_save` 퍼미션 소실) 영향 확대**: `persist::write_atomic`(`persist.rs:11-24`)은 `File::create` → `rename` 으로 모드 미보존이 확정. **`search_replace` 도 같은 `write_atomic`(`search/service.rs:384`)을 쓰므로 일괄 치환도 실행 비트를 소실**시킵니다. 원 배치는 `file_save` 만 지목했으나 두 경로입니다.
- **R5#5(기본값 드리프트) 확정**: Rust `default_true()`(`settings/types.rs:230·233`) vs `settings-view.tsx:461·488` `?? false` vs `app-sidebar.tsx:40` `?? true` — 정본 1개에 사본 2개가 서로 다른 값. 실코드 일치 확인.
- **X1#4 확정**: `layoutSetViewState` 전수 grep 결과 `bindings.ts:27` **정의 1줄만** — 호출부 0. `viewState`/`view_state` 프런트 히트는 전부 `AiInlineEditPreviewState` 오탐. 문서(`data-model.md:53·101`, `ipc-contract.md:70`)가 정본으로 기재한 커서·스크롤 영속은 실재하지 않음.

---

## 3. 소유권 확정 — 1차 §4.4 해제

1차가 예고한 5건을 6배치와 실제 대조했습니다.

| 항목 | 1차 보고 | 도메인 배치 보고 여부 | **정본 소유 확정** |
|---|---|---|---|
| `theme_delete` 경로 검증 부재 | R1#2 | **R5(theme 소유 배치)가 미보고** | **R1#2 유지.** 실코드 재확인: `theme/service.rs:923` `save_theme` 은 `contains(['/','\\','.'])` 검증, `:932-942` `delete_theme` 은 builtin 체크 후 곧바로 `themes_dir().join(format!("{theme_id}.json"))` remove_file — 비대칭 확정. `load_theme:944` 도 동일. **도메인 담당 배치가 놓친 유일 항목이므로 관통 배치의 발견이 정본** |
| 전역 락 안 git 네트워크 IO | R1#3 | R4#3 (더 정밀) | **R4#3 으로 이관.** R4 가 `spawn_blocking` 미적용 27/41 통계와 `file/commands.rs:86-89` 의 자인 주석("hot-exit 플러시가 `git_push` 뒤에서 대기해 미저장 미러를 잃는다")을 추가. R1#3 은 흡수 |
| `pty_kill` 일시정지 중 kill 누수 | R2#3 | R8#4·R8#5 (+R8#1) | **R8 계열로 이관 · 재구성.** R8#1(회수 경로 자체가 없음)이 상류이므로 R2#3 은 그 하위 사례로 강등. 1차 T0 #10 은 **단독으로는 무효**(호출자가 없는 함수를 고치는 것) — 신규 T0 #21 과 묶어야만 효과 발생 |
| LSP 다운로드 http 우회 | R2#5 | **R7(lsp 소유 배치)가 미보고** | **R2#5 유지.** 다만 R6#7(ai 커맨드 5종이 호출마다 `create_outbound_http_client`)이 같은 `infra/http.rs` 축이며, R2#6 과도 동일 근원 — **세 건을 T1-G 에서 한 작업으로 처리** |
| 키링 게이팅 비대칭 | R3#7 | **R5·R6 모두 미보고** | **R3#7 유지.** R5#15(같은 keyring 항목 이중 읽기)만 인접 보고 |

**소유권 결론: 5건 중 2건 이관, 3건은 관통 배치 원보고가 정본입니다.** 도메인 담당 배치가 자기 도메인의 보안 결함 3건(theme 경로·LSP http·키링 게이팅)을 놓쳤다는 사실 자체가 §8 의 판단 근거가 됩니다.

---

## 4. 클러스터 병합 — 1차 §3 증분

119건 중 **83건(70%)이 클러스터로 수렴**합니다(1차 34% 대비 급증). 6배치가 Rust 전 도메인을 균일하게 훑으면서 같은 결함의 반복이 드러난 결과입니다.

### 4.1 기존 클러스터 증분 (C1~C12)

**C1 (레이어 배치 features↔widgets)** — 증분 없음. 6배치는 TS 레이어를 다루지 않음. 1차 §3-C1 유효.

**C2 (중복 구현 · "2회 이상" 룰)** — +13건, **최대 클러스터 지위 강화(27건)**

| 신규 중복 대상 | 사본 수 | 출처 |
|---|---:|---|
| `constant_time_eq` | **4** (1차 3 + agent 재구현) | R2#8 + **R6#9** |
| ollama↔omlx provider 전체 파이프라인(`complete`/`complete_chat`/`send_chat_request`/`extract_chat_text`/`build_chat_body`) | 2 | R6#10 |
| 경로 봉쇄 원시함수(`ensure_within_root`·`canonicalize_lenient`) | 2 | R7#5 |
| Settings 필드 목록 | **6** (Settings·Default·SettingsPatch·apply_patch·settings_to_sync_patch·emptySettingsPatch) | R5#3 |
| 테마 토큰 약 200개 | 2 (Rust·TS) | R5#9 |
| 로케일 메시지 키 | 4 (NAMESPACES + en/ko/ja) | R5#8 |
| provider 3분기 match | 3 (list_models·complete·instruct) | R6#13 |
| 훅 JSON 조작 API | 2 (url 계열/command 계열, 이미 동작 상이) | R6#15 |
| 폰트 크기 유효 범위 | 3 (Rust 미보정 / 8–32 / 6–48) | R5#4 |
| Rust 기본값 ↔ TS `??` 폴백 | 2 (+소비처끼리 불일치) | R5#5 |
| AI 응답 타입 동형 3종 | 3 | R6#18 |
| `agent_hooks_install` 설정 게이트 | 2 arm | R6#17 |
| codex 만 절단 보정 미적용 | — | R6#6 |

**동일 근원 3갈래로 확장**: 1차의 (a)발견 비용·(b)추상화 부재에 더해 **(c) 생성(ADR-0011)이 타입에만 적용되고 상수·기본값·키 목록에는 적용되지 않음** — R5 배치가 이를 명시적으로 지목했고 R5#3·#4·#5·#8·#9 5건이 전부 여기서 나옵니다.

**C3 (싱글톤 소유권·정리 계약)** — +5건, Rust 측 대칭 확인

| 추가 | 결함 | 출처 |
|---|---|---|
| `IdeStore.current_selection` | 창·세션 구분 없는 전역 단일 슬롯 — 원격이 데스크톱 선택을 덮어씀 | R6#12 |
| `AiRequestStore` | 호출자 제공 `requestId` 만으로 전역 공유 | R6#20 |
| LSP 세션 `channels` 맵 | **구조적으로 항상 1개** — 문서가 약속한 다중 창 구독이 도달 불가 코드 | R7#6 |
| 렌더러 LSP 레지스트리 키 | `projectId::serverId` 만 — root 누락으로 백엔드 멀티루트 기구가 통째로 미실행 | R7#7 |
| 검색 세션 키 | `useId()` = realm 로컬 → 두 번째 창이 메인 창 검색을 취소 | R7#8 |

**중요**: R7#8 은 1차 C3 의 진단("멀티 윈도우가 미결제 부채를 청구서로 바꿨다")을 정면으로 뒷받침합니다 — `lsp_stop` 은 Wave I 에서 `owner` 를 도입해 같은 문제를 이미 해결했는데 검색만 남았습니다. **정답이 옆에 있는 패턴이 C3 에서 세 번째로 반복**(1차 codeLens↔semanticTokens, LSP owner↔검색 세션, plugin↔LSP spawn_blocking).

**C5 (서버 상태 경계)** — +3건 (12→15)
- `tree.query.ts` 가짜 페이지네이션 — `offset:0, limit:5000` 고정 + `total` 미사용, 동시에 mutation 3종은 `full_page` 를 `setQueryData` 주입 → **같은 캐시 키가 "전량"과 "5000행 절단" 사이를 오감**(R4#12)
- `fsChanged` N+1 — 디렉토리마다 `refreshTreeDir`(전역 가드 + 전체 트리 직렬화) 호출 후 **반환값을 버리고** `TREE.ROWS` 재무효화(R4#13)
- 테마 프리뷰 `setQueryData`(R5#10) — **1차 F1#11 과 동일 결함**, 2배치 독립 보고

**C6 (타입 파생 미사용)** — +5건 (14→19). 1차의 "단일 최대 근원 = Rust `Settings` 열거형 필드 `Option<String>`" 진단이 **R5 에 의해 확대 확정**: 같은 `Settings` 타입이 필드 목록 6중 미러(R5#3) · 기본값 이중 정의(R5#5) · 3상태 표현 불가(R5#1, critical)의 근원이기도 합니다. **`Settings` 하나가 4배치·5클러스터에 걸친 부채의 진원지**입니다. 추가: specta 가 7타입을 `_Serialize`/`_Deserialize` 로 분할하며 `ResolvedTheme` 은 5필드 필수성이 다름(X1#18), `Project` 영속 스키마 version 규약 부재(R4#8).

**C7 (비대 파일)** — +4건 (11→15)

| 파일 | LOC | 비고 |
|---|---:|---|
| `locale/service.rs` | 4,081 (리터럴 3,760 / 로직 153) | 쌍둥이 `theme` 은 이미 `include_str!` 로 외부화 — 미해소 불일치(R5#8) |
| `git/service.rs` | 2,935 (프로덕션 1,726) | hunk/patch 재합성 블록 688~998 이 명확한 분리 단위 |
| `agent/commands.rs` | 717 | IPC + fs IO + `ps` 서브프로세스 + osascript 혼재, `hooks.rs` 가 역참조(R6#8) |
| `system/commands.rs` | 388 (커맨드 본문 60줄) | URL 검증 로직 + sysinfo 어댑터 흡수(R8#8) |

**C9 (i18n)** — +1건. Rust 사용자 노출 문자열 한/영 혼재가 **세 번째 배치에서 재보고**(R8#15, terminal). 1차 R1#9(369지점 한101/영268) · R2#10 과 동일 근원 — T2-E 별도 캠페인 근거 강화.

**C10 (경로 안전 · 보안 경계)** — +7건 (10→17), **가장 크게 증가한 클러스터**
- 경로 봉쇄 원시함수가 **5갈래**로 확장(1차 4갈래 + `plugin::service` 자체 구현, 에러 변종도 `InvalidArgument` vs `Forbidden` 로 갈림 — R7#5)
- `to_repo_relative` 가 POSIX 에서도 백슬래시 무조건 치환 + `CurDir` 미필터(R4#20)
- git 도메인 반환 경로 규약 부재(R4#2, critical)
- 원격 권한 게이팅 비대칭 4건 → **C16 으로 분리**

**C11 (Rust 상태 소유권 · 락)** — +10건 (9→19), **두 축으로 분해**

*축 A — 락을 쥔 채 블로킹/네트워크 IO*
| 항목 | 점유 시간 | 출처 |
|---|---|---|
| git push/pull/fetch/commit | 네트워크 왕복 전체 | R4#3 (R1#3 흡수) |
| sync upload/download | GitHub 왕복, 타임아웃 60초 | R5#7 |
| plugin/vsix 압축 해제 | 최대 128MB 해제 + 재귀 복사 | R7#10 |
| `pty_write` | 자식이 stdin 미독 시 무기한 | R8#2 (critical) |
| `font_list` | 시스템 폰트 전수 스캔, 캐시 없음 | R8#11 |

*축 B — 써야 할 곳에 가드가 없음*
| 항목 | 출처 |
|---|---|
| `search_replace` (프로젝트 전역 파일 덮어쓰기) | R7#3 (critical) |
| `ide_resolve_diff` (파일 쓰기 + `file_save` 본문 복제) | R6#2 |
| `tree_rows` (스토어 전체 되쓰기) | R4#1 (critical) |
| `git_status` (query 인데 `update_index(true)` 로 인덱스 기록) | R4#11 |

**동일 근원**: `begin_mutation` 이 "앱 전역 단일 뮤텍스"라는 **단 하나의 입도**만 제공하므로, 개발자는 매번 "전부 막을 것인가 / 아무것도 안 막을 것인가" 를 택해야 합니다. 두 축은 같은 설계 공백의 양쪽 끝입니다. `infra/http.rs` 의 doc 주석과 `file/commands.rs:86-89` 주석이 이미 이 결합을 자인하고 있으나 완화책(타임아웃·락 면제)만 있습니다.

**C12 (계약이 문서에만 있고 기계가 강제하지 않음)** — +5건 (5→10), **X1 이 정본 소유**
- 이벤트 4목록(events.rs 25 / collect_events! 25 / fanout 23 / bindings 25) 파리티 테스트 **0건** — R1#8 의 정본이 X1#8 로 확정
- 커맨드 파리티 테스트가 **생성 산출물 `bindings.ts` 를 기준선으로 사용** → `collect_commands!` ↔ bindings 엣지가 무강제. "낡은 bindings = 낡은 dispatch" 로 통과(X1#9)
- Settings 필드 집합 비교 테스트 없음(R5#3) / 테마 토큰 200개 비교 테스트 없음(R5#9)
- sync 버전 게이트가 구조적으로 발동 불가 — `SETTINGS_SCHEMA_VERSION` 이 정책상 1 로 동결(X1#6)

### 4.2 신규 클러스터 C13~C16

#### C13. Rust 도메인 간 직접 호출 — `architecture.md:77` 금지 조항의 시스템적 붕괴 (12건 / 5배치)

| 호출 방향 | 엣지 | 출처 |
|---|---|---|
| project → git·layout·ide·agent | 4 (사실상 조립부 겸업) | R4#5 |
| settings → agent·ide·remote | 3 (**같은 함수가 바로 아래에서 `SettingsChanged` 를 발행**하는데도 명령형 호출) | R5#6 |
| sync → ai·locale·settings·theme | 4 (집계 도메인, 일부 불가피) | R5(총평)·R5#14 |
| system → terminal·lsp·agent | 3 (`Store` 직접 참조) | R8#9 |
| terminal → ide | 1 (+ 모든 spawn 이 최대 2초 폴링 지연) | R8#10 |
| ide → layout·plugin **commands** | 2 (**커맨드 표면의 제2 진입점** — 원격 dispatch 의 거부 정책 미통과) | R6#3 |
| ide → file::service | 1 (+가드 누락) | R6#2 |
| git·file → plugin::commands | 2 | R4#6 |
| plugin ↔ vsix | **양방향 순환** | R7#4 |
| ide commands ↔ server | **양방향 순환** | R6#11 |
| **infra/language → domain::plugin** | **계층 역방향** | R4#6 |

**총 14개 도메인에 약 30엣지**(R5 실측). `architecture.md:77` 은 "도메인 간 직접 호출은 금지하고, 필요하면 이벤트 버스 또는 상위 조립부를 거친다"고 규정하지만 **이 규칙이 지켜지는 도메인 쌍을 찾기가 더 어려운 상태**입니다.

**동일 근원**: `architecture.md §3` 이 선언한 `ProjectCapability::attach/detach` 확장점이 **스텁으로만 존재**합니다(R4#9 — `Project.capabilities` 소비처 0). 원래 이 자리가 도메인들이 프로젝트 수명주기에 스스로 붙는 지점인데, 그것이 비어 있으니 `project/commands.rs` 가 대신 4개 도메인을 손으로 부르고 있습니다. **C13 과 C14 는 같은 공백의 두 증상**입니다.

#### C14. 프로젝트·세션 종료 시 자원 회수 누락 (9건 / 5배치)

`project_close`(`project/commands.rs:143-171`)가 정리하는 것: `layouts` · `watchers` · `git_watchers` **3개뿐**.

| 회수되지 않는 자원 | 결과 | 출처 |
|---|---|---|
| `dirty_layouts` | 미저장 레이아웃이 무경고 폐기 | R1#1 |
| `GitStore` (projectId→repo_root) | 같은 폴더 재오픈 시 옛 캐시 부활 | R4#7 |
| `TreeStore` (전체 트리 캐시) | 옛 디렉토리 목록 부활 + 앱 수명 동안 메모리 잔존 | R4#7 |
| `LspStore` | (ipc-contract 에 이미 기록, 프런트가 우회 처리) | 기지 |
| pty 세션 (프로세스+fd+3스레드+2MB) | **회수 경로 자체 부재** | R8#1 (critical) |
| asset 프로토콜 스코프 | **닫힌 프로젝트 트리를 webview 가 계속 읽음** | X1#7 (보안) |
| `AgentWaitMarker` | 창 이동·프로젝트 종료 시 영구 누수 | F1#1 (1차) |

인접 결함: `LspProcHandle::kill()` 이 AtomicBool 만 세우고 실제 시그널은 50ms 폴링 대기 → **앱 종료 시 고아 언어 서버**(R7#9, 같은 자리의 터미널은 동기 kill 이라 비대칭). `LspInstallStore` 해제 보장 없음(R7#17). `restart_count` 정상 가동 후 미리셋 → 누적 3회로 자동 재시작 영구 중단(R7#14). `shell_integration` 임시 디렉터리 정리 경로 없음(R8#16).

**동일 근원**: `architecture.md §6.3` 이 "Drop 구현 + 명시적 shutdown 경로 이중화"를 전 자원 공통 규칙으로 규정하지만, **실제로 두 겹을 갖춘 자원이 없습니다** — pty 는 명시 경로 한 겹인데 그 한 겹도 호출자가 없고(R8#1·R8#4), LSP 는 비동기 한 겹, asset scope 는 등록만 있고 해제 API 자체를 안 씀.

#### C15. 계약 배선이 한쪽만 존재 — 죽은 이벤트·죽은 커맨드·죽은 필드 (11건 / X1 소유)

| 유형 | 대상 | 상태 |
|---|---|---|
| 양방향 사망 | `terminal:cwd-changed` | emit 0 + 구독 0 (문서는 살아있는 흐름으로 단언) |
| 발행만 | `lsp:session-status-changed` · `terminal:exited` · `remote:state-changed` · `project:focus-kind-changed` · `app:ready` | 구독 0 (25종 중 6종) |
| 커맨드 사망 | `layout_set_view_state` | 호출 0 → **커서·스크롤 영속 기능이 실재하지 않음** |
| 커맨드 사망 | `git_ahead_behind`·`git_fetch`·`git_undo_last_commit`·`ide_start/stop`·`ide_notify_at_mention`·`remote_start/stop`·`window_open_auxiliary` | 호출 0 (10종) |
| 필드 사망 | `FsChange.from_app` | 상수 `false` + 읽는 곳 0 → **자기 쓰기/외부 변경 구분 수단이 있는 척만 함** |
| 필드 사망 | `LayoutChanged.revision` | Rust 는 증가·발행, 프런트는 무시 (낙관적 동시성 카운터가 무의미) |
| 과잉 발행 | `project:focus-kind-changed` | 레이아웃 변이 18종마다 diff 없이 무조건 발행, 소비자 0, 원격 팬아웃 포함 |
| 게이트 사망 | sync `schemaVersion` | 상수가 1 로 동결이라 미래 버전 페이로드도 통과 |

**동일 근원**: 계약이 **양단(발행/구독, 정의/호출)이 아니라 단면으로만 검증**됩니다. C12 가 "파리티 테스트가 없다"는 장치의 부재라면, C15 는 그 부재가 이미 만들어낸 **결과 재고**입니다. 두 클러스터는 반드시 함께 처리해야 합니다(테스트만 넣으면 11건이 즉시 실패, 배선만 고치면 재발).

#### C16. 원격(remote) 표면의 권한 게이팅 비대칭 (6건 / 3배치)

| 커맨드/이벤트 | 원격 상태 | 같은 성격의 이웃 | 판정 |
|---|---|---|---|
| `agent_cli_install` / `agent_cli_uninstall` | **허용** (관리자 암호 프롬프트 + `/usr/local/bin` 심링크) | `plugin_install` = 명시 거부 | R6#1 critical |
| `agent_hooks_install`(User 스코프) | **허용** (`~/.codex/hooks.json` 등 root-guard 밖에 command 훅 주입) | 동상 | R6#1 |
| `lsp_install` | **허용** (수백 MB 다운로드 + `go`/`gem`/`cs`/`ghcup` 프로세스 spawn) | `vsix_import_plugin` = 거부 | R7#11 |
| `agent_pending_external_opens` + `AgentExternalOpen` 팬아웃 | **허용 + 팬아웃** (원격이 데스크톱의 CLI 열기 요청을 가로챔, waitMarker 는 원격 realm 에 등록되어 CLI 영구 블로킹) | `HotExitFlushRequested` 는 QA6 에서 팬아웃 제외 | X1#3 |
| `ide_set_selection` / `ide_clear_selection` | **허용** (전역 단일 슬롯 덮어쓰기) | — | R6#12 |
| 키링 5계정 | 원격 거부는 1계정뿐 (AI 토큰 3 + GitHub PAT 원격 변경 가능) | — | R3#7 |

**동일 근원**: 거부 사유는 `dispatch.rs:309-357` 에 **분류로 명문화되어 있습니다**("데스크톱 로컬 파일시스템 밖 읽기·쓰기 금지", "원격에서 볼 수도 쓸 수도 없는 창을 데스크톱에 띄우지 않는다"). 결함은 정책의 부재가 아니라 **정책이 arm 단위 수기 열거로만 적용**되는 데 있습니다 — 새 커맨드는 기본 허용이고, 거부는 누군가 기억해야 합니다. **C16 의 근본 수정은 "기본 거부 + 명시 허용" 으로 뒤집는 것**이며, 이는 개별 arm 6건 수정과 별개의 T1 작업입니다.

---

## 5. severity 재조정 — 1차 §4 증분

1차 §4.1(F5#1 하향) · §4.2(3건 유지) · §4.3(하향 제안 기각) **전부 유지**. 신규 조정 4건:

| 항목 | 원 등급 | 조정 | 근거 |
|---|---|---|---|
| R7#11 `lsp_install` 원격 허용 | major | **critical 상향** | R6#1(critical)과 **완전히 동일한 실패 유형**(원격이 데스크톱에서 임의 다운로드 + 프로세스 실행)인데 등급이 갈렸습니다. `run_toolchain_install` 이 `tokio::process::Command::new(binary)` 로 실제 프로세스를 spawn 하므로 영향은 오히려 더 큽니다. 등급 내 일관성을 위해 상향 |
| X1#3 `agent:external-open` 원격 가로채기 | major | **critical 상향** | 결과가 (a) 데스크톱 사용자가 요청한 파일이 열리지 않고 (b) waitMarker 가 원격 realm 에 등록되어 **외부 CLI 프로세스가 영구 블로킹**됩니다. 1차가 F1#1(wait-marker 누수)을 "앱 밖으로 새는 결함"이라며 T0 로 승격한 것과 같은 축이며, 여기에 원격 신뢰 경계까지 겹칩니다 |
| R4#1 tree 스토어 lost update | critical | **major 하향** | 사실관계는 전건 확인. 그러나 손실되는 것은 **트리 확장 상태(UI 상태)** 이며, 사용자 파일·설정·레이아웃 같은 영속 데이터가 아닙니다. 1차 F3#2(파일 A 에 B 내용 기록)나 R7#3(편집 소실)과 같은 등급에 두는 것은 과대입니다. 성능 축(매 호출 전체 딥카피)은 별개로 유효하므로 major 유지. **처리 순위는 T0 유지**(수정이 3줄) |
| X1#2 이벤트 6종 구독자 0 | major | **유지 · 주장 범위 축소** | §2 표 마지막 행 참조. "LSP 크래시가 사용자에게 전혀 보이지 않는다" → "이벤트 기반 즉시 반영이 없고, 무효화 지점이 0인 쿼리의 refetch 타이밍에만 지연 표시된다"로 정정. 등급은 유지(6종 중 `terminal:exited` 미소비로 죽은 pty 가 alive 로 취급되는 부분은 그대로 성립) |

**조정 후 critical 총계: 18 → 19** (R7#11·X1#3 상향 2 − R4#1 하향 1).

---

## 6. 티어 개정

### 6.1 T0 — 즉시 (11 → 24항목)

1차 T0 #1~#11 은 **전부 유지**하되 2건에 선후 제약이 추가됩니다.

**1차 T0 항목 변경사항**
- **#2(`project_close` 레이아웃 flush)** → 신규 **T1-J** 에 흡수 권고. 같은 함수에서 회수해야 할 자원이 7종으로 늘었으므로(C14), 하나씩 고치면 세 번 건드립니다. 단 dirty layout 유실은 데이터 손실이므로 **T1-J 를 T0 직후 착수**하거나 #2 만 선행.
- **#10(`pty_kill` 이 `set_paused(false)` 선행)** → **단독으로는 무효**. `pty_kill` 호출자가 0 이므로(신규 #21) 반드시 #21 과 한 작업으로 묶습니다.

**신규 T0 #12~#24 (13항목)**

*T0-a — 보안 · 신뢰 경계 (5항목, 전부 국소)*

| # | 항목 | 출처 | 파일 | 수정 규모 | 위험도 |
|---:|---|---|---:|---|---|
| 12 | `agent_cli_install`/`agent_cli_uninstall` 을 원격 deny arm 으로 이동 (`deny_remote_plugin_install` 계열 헬퍼 신설, `IMPLEMENTED_JSON_COMMANDS` 는 유지해 파리티 보존) | R6#1 | 1 | arm 2 + 헬퍼 1 | 저 |
| 13 | `agent_hooks_install` 을 Project 스코프만 원격 허용, User 스코프 거부 분기 | R6#1 | 1 | 분기 1 | 저 |
| 14 | `AgentExternalOpen` 을 `fanout_remote_events!` 에서 제외 + `agent_pending_external_opens` deny arm | X1#3 | 2 | 2줄 | 저 |
| 15 | `project_close` 에 `forbid_directory` 추가 (asset 스코프 회수) | X1#7 | 1 | 1줄 | 저 |
| 16 | `lsp_install` 원격 처리 확정 — 거부 또는 명시 허용 문서화 | R7#11 | 1 | arm 1 | 저 (**결정 필요**) |

*T0-b — 데이터 손실·손상 (4항목)*

| # | 항목 | 출처 | 파일 | 위험도 | 비고 |
|---:|---|---|---:|---|---|
| 17 | `search_replace` 에 뮤테이션 가드 적용 | R7#3 | 1 | 저 | 대상 집합 확정 후 파일 단위 재획득 권장(C11 축 A 회피) |
| 18 | git `StatusRow` 경로 규약 확정 — 소비부에서 repo root join 또는 절대 경로 동봉 | R4#2 | 1~3 | 중 | **결정 필요**. 최소안은 `git-panel-container.tsx` 1곳 |
| 19 | `file_save` · `search_replace` 가 원본 모드 보존 (`write_atomic_with_mode`) | R4#10 | 2 | 저 | 실행 스크립트 저장 회귀 테스트 동반 |
| 20 | `pty_write` 락 범위를 핸들 조회까지로 축소 | R8#2 | 1 | 저 | 앱 전체 프리즈 경로 제거 |

*T0-c — 기능 무력화 (4항목)*

| # | 항목 | 출처 | 파일 | 위험도 | 비고 |
|---:|---|---|---:|---|---|
| 21 | pty 세션 회수 경로 신설 (탭 닫기 / 프로젝트 닫기) — **1차 T0 #10 을 여기에 흡수** | R8#1(+R2#3·R8#4·R8#5) | 3~4 | 중 | **결정 필요**(소유자 Rust vs 프런트). `PtySession` Drop + pause 해제 동반 |
| 22 | `SettingsPatch` 3상태(미지정/설정/해제) 표현 | R5#1 | 1 Rust + 1 TS | 중 | **결정 필요**(`Option<Option<T>>` vs 빈 문자열 규약 일반화). 후자면 Option<String> 6필드로 국소 |
| 23 | 검색 오프셋을 UTF-16 코드유닛으로 변환 + `column` 1-기반 보정 | R7#2 | 1 | 저 | 한글 포함 회귀 테스트 필수 |
| 24 | LSP 자동 재시작 **완화** — 재핸드셰이크 없이 `Running` 보고하지 않고 `Crashed` 유지 | R7#1 | 1 | 저 | 근본 수정(세션 세대 이벤트 + 렌더러 재수립)은 T1-D |

**T0 총계: 24항목 · 약 35파일 · 결정 필요 5건.**

**T0 부수 권고 갱신**: 1차는 "T0 의 5항목이 잘못된 소유자에 매달린 수명 문제"라고 했습니다. 2차에서는 여기에 **"권한 게이팅이 수기 열거"**(T0-a 5항목 전부)가 추가됩니다. 개별 arm 5건을 고치는 것과 별개로 **`dispatch.rs` 를 기본 거부로 뒤집는 작업(T1-K)** 을 함께 계획하지 않으면 다음 커맨드에서 동일 결함이 재발합니다.

### 6.2 T1 — 다음 정비 (8묶음 → 11묶음)

**기존 묶음 증분**

| 묶음 | 1차 내용 | 2차 추가 | 해소 건수 |
|---|---|---|---:|
| T1-A 전역 부수효과 소유권 | 그대로 | — | 3 |
| T1-B Settings 타입 좁히기 | 열거형 → specta union | **+R5#3**(필드 6중 미러 → 파리티 테스트), **+R5#5**(기본값 이중 정의 제거, `skip_serializing_if` 부재로 optional 생성되는 근원 정리), **+R5#4**(폰트 범위 3벌 통일 + `sanitize` 클램프 추가), **+X1#18**(specta 분할 7타입의 필수성 차이 문서화·정합) | 6 → **10** |
| T1-C 서버 상태 계약 | 그대로 | **+R4#12**(트리 페이지네이션 계약 확정), **+R4#13**(fsChanged N+1 제거), **+R5#10**(테마 프리뷰를 캐시 밖으로 — F1#11 과 동일) | 12 → **14** |
| T1-D 레지스트리 정리 계약 | 브리지 팩토리 2종 등 | **+R7#7**(LSP 레지스트리 키에 root 포함 → 백엔드 멀티루트 기구 활성화), **+R7#8**(검색 세션 키를 owner 스코프로 — `lsp_stop` 선례 적용), **+R6#12**(IdeStore 선택영역을 창/세션 키로), **+R6#20**(AiRequestStore 스코프), **+R8#3/R3#1**(원격 sink 실패 전파), **+R7#1 근본**(세션 세대 이벤트 + 재핸드셰이크), **+R7#6**(도달 불가 다중 구독 코드 — 실현 또는 제거 확정) | 10 → **17** |
| T1-E 계약 검증 테스트 | 5건 | **+X1#8**(이벤트 4목록 파리티 — R1#8 정본), **+X1#9**(파리티 기준선을 `collect_commands!` 로 이동), **+R5#3·R5#9**(Settings 필드·테마 토큰 집합 비교), **+X1#6**(sync 버전 게이트 실효화) | 5 → **10** |
| T1-F 레이어 배치 정정 | TS 12파일 이동 | — | 6 |
| T1-G 인프라 하드닝 | 9건 | **+R6#7·R2#6·R2#5**(http Client 싱글톤 + 프로필 분리 — 3건 동일 근원), **+R7#13**(매니페스트 캐시 + `expect` 제거), **+R6#4**(`ai_inline_complete` 바이트 상한), **+R6#5**(훅 서버 검사-후-바인드 경합), **+R7#9**(LSP kill 동기화), **+R7#17**(설치 스토어 해제 보장) | 9 → **15** |
| T1-H 전역 락 IO 분리 | 2건 | **+R4#3**(정본 이관), **+R5#7**, **+R7#10**, **+R8#11**, **+R8#13**, **+R4#11**(`git_status` 부수효과 제거) | 2 → **10** · 위험도 **높음 유지** |

**신규 묶음 3종**

| 묶음 | 내용 | 해소 | 파일 | 위험도 |
|---|---|---:|---:|---|
| **T1-I 도메인 경계 재조립** | C13 전량 — `ProjectOpened`/`ProjectClosed` 를 내부 구독 지점으로 삼아 각 도메인이 스스로 attach/detach, `ide/server.rs` 가 `commands` 대신 `service` 호출(제2 커맨드 진입점 제거), plugin↔vsix·ide commands↔server 순환 절단, `infra/language` 의 `domain::plugin` 역참조 제거(경량 `LanguageOverlay` 타입), system 의 라벨 조회를 조립부 경유로 | 12 | 15~20 | **중~높음** |
| **T1-J 프로젝트 종료 자원 회수** | C14 전량 — `project_close` 가 dirty_layouts·GitStore·TreeStore·LspStore·pty 세션·asset scope·waitMarker 를 일괄 회수. T1-I 의 `ProjectClosed` 훅 위에 얹으면 스토어가 늘어도 `project` 도메인 무수정. **1차 T0 #2 · T0 #11 · 신규 T0 #15·#21 과 직결** | 9 | 6~8 | 중 |
| **T1-K 원격 게이팅 기본 거부 전환** | C16 근본 — `dispatch.rs` 를 "명시 허용 목록 + 기본 거부"로 뒤집고, 거부 사유 분류(`dispatch.rs:309-357`)를 타입으로 승격. 신규 커맨드가 무단으로 원격 표면에 오르지 않게 함. **T0-a 5항목 수정 후 착수** | 6 | 2~3 | 중 |

**T1 총계: 11묶음 · 약 100~120파일 · 해소 108건.**

### 6.3 T2 — 백로그 증분

| 묶음 | 2차 추가 |
|---|---|
| T2-A 중복 제거 | +`constant_time_eq` 4번째 사본(R6#9), ollama↔omlx provider 전체(R6#10), 경로 봉쇄 원시함수(R7#5), provider match 3벌(R6#13), 훅 JSON API 2벌(R6#15), 게이트 arm 복제(R6#17), AI 응답 동형 타입 3종(R6#18), codex 절단 보정(R6#6) |
| T2-B 비대 파일 분해 | +`locale/service.rs` 4,081, `git/service.rs` patch 서브모듈 분리, `agent/commands.rs` 717(IO → infra), `system/commands.rs` 388(sysinfo → infra, URL 검증 → service) |
| T2-E i18n 통일 | +R8#15(terminal Rust 에러) — 동일 근원 3배치 확정, **별도 캠페인 권고 강화** |
| T2-F dead code | +git types 상수 2개(R4#16), `resolve_terminal_path` 호출자 0(R8#14), `Project.capabilities` 스텁(R4#9), 미참조 로케일 키 39개(R5#12) |
| T2-G 매직넘버 | +폰트 범위 8–32 vs 6–48 vs 무보정(R5#4), `DEFAULT_FONT_SIZE` 중복 정의 |
| **T2-I (신규) 로케일 데이터 외부화** | `locale/service.rs` 3,760줄 리터럴을 `resources/locales/*.json` + `include_str!` 로 — **theme 도메인이 이미 쓰는 형태**(R5#8·R5#12) |
| **T2-J (신규) 오류 분류 taxonomy** | `map_git_err`/`run_git`(R4#15) · provider HTTP(R6#21) · 설치 취소(R7#16)가 전부 `AppError::Internal` 문자열로 평탄화 → 프런트 분기 불가. **T2-E 의 AppError 코드화와 같은 작업**이므로 병합 |

**T2-H(문서 정합)는 §7 로 이관**합니다.

### 6.4 티어 요약 (개정)

| 티어 | 항목/묶음 | 예상 파일 | 위험도 | 해소 건수(누적) |
|---|---:|---:|---|---:|
| T0 | 24 | 약 35 | 저~중 (결정 필요 5) | 24 |
| T1 | 11묶음 | 100~120 | 중 (T1-H 높음, T1-I 중~높음) | 108 (132) |
| T2 | 10묶음 | 150+ | 저 | 107 (239) |
| §7 계약·문서 | 3분류 18건 | 12~15 | 저 | 18 (257) |
| 미배정 | — | — | — | 40 (개별 minor·압축 제외분) |

---

## 7. X1(계약 정합) 별도 티어링 — 지시 5

X1 18건은 **수정 성격이 코드 / 테스트 / 문서로 갈라지므로** 위 T0~T2 와 분리해 독립 트랙으로 둡니다. 같은 커밋에 섞으면 "문서 오탈자 수정"과 "죽은 배선 제거"가 한 diff 에 들어갑니다.

### X-A. 코드 수정 — 배선 완성 또는 제거 (8건)

각 항목은 **"살릴 것인가 지울 것인가"** 를 먼저 정해야 하며, 그 결정 없이는 수정 불가입니다.

| # | 항목 | 선택지 | 권고 |
|---|---|---|---|
| X1#3 | `agent:external-open` 원격 팬아웃 | 팬아웃 제외 / 유지 | **T0 #14 로 이미 승격** — 여기서 제외 |
| X1#7 | asset 스코프 미회수 | `forbid_directory` 추가 | **T0 #15 로 승격** — 여기서 제외 |
| X1#1 | `terminal:cwd-changed` 양방향 사망 | 셸 통합에서 발행 + 구독 신설 / 이벤트·타입 삭제 | **살리기** — `resolve_terminal_path`(R8#14, 호출자 0)와 한 세트이며 cmd+click 파일 링크 기능의 전제 |
| X1#4 | `layout_set_view_state` 호출 0 | monaco viewState 저장 배선 / 커맨드·필드 삭제 | **결정 필요** — 문서가 정본 기능으로 기재 중이므로 어느 쪽이든 문서 동반 수정 |
| X1#10 | `FsChange.from_app` 상수 false | 실제 echo 플래그 구현 / 필드 삭제 | **살리기 권고** — 구현하면 R4#13(fsChanged N+1)의 근본 완화가 함께 됨 |
| X1#11 | `LayoutChanged.revision` 미소비 | 프런트 게이트 구현 / 필드 삭제 | **살리기** — 멀티 윈도우에서 stale 갱신 방지가 원래 목적 |
| X1#12 | `project:focus-kind-changed` 무조건 발행 | diff 후 발행 + 소비자 신설 / 삭제 | **삭제 권고** — 소비자·타입 참조 모두 0, 이벤트 트래픽만 2배 |
| X1#13 | 커맨드 10종 호출 0 | 개별 판정 | `ide_start/stop`·`remote_start/stop` 은 `settings_update` 부수효과와 **표면 중복** → 제거. `window_open_auxiliary` 는 `layout_move_tab_to_window` 와 중복 → 제거. `git_ahead_behind`·`git_fetch`·`git_undo_last_commit` 은 UI 미구현 → 유지 결정 시 문서에 "예약" 표기 |
| X1#2 | 이벤트 6종 구독자 0 | 개별 판정 | `lsp:session-status-changed` 는 **T0 #24 · T1-D 에서 소비 신설**, `terminal:exited` 는 세션 정리 트리거로 소비 신설(T1-J 와 결합), 나머지는 위 항목들에 흡수 |

### X-B. 테스트 신설 — 강제 장치 (3건, T1-E 로 이관 완료)

| # | 항목 | 작업 |
|---|---|---|
| X1#8 | 이벤트 4목록 파리티 테스트 0건 | `events.rs` 25 / `collect_events!` 25 / `fanout_remote_events!` 23 / `bindings.ts` 25 의 집합 비교 테스트. **fanout 23(HotExitFlushRequested·AgentExternalOpen 제외)는 의도된 결정이므로 예외 목록을 테스트에 명시** — 현재는 주석으로만 남아 있음 |
| X1#9 | 커맨드 파리티가 생성물 기준 | 기준선을 `include_str!(bindings.ts)` 에서 `collect_commands!` 로 이동. 현재 세 출처가 모두 179 로 일치하나 강제 장치가 없음 |
| X1#6 | sync 버전 게이트 무력 | `SETTINGS_SCHEMA_VERSION` 동결 정책을 유지하려면 게이트를 **필드 화이트리스트 기반 미지 필드 감지**로 바꾸거나, 게이트가 무력하다는 사실을 doc 주석과 정합시킴. 현재 테스트(`sync/service.rs:340·350`)는 상수 자기 자신과만 비교해 무력화를 못 잡음 |

### X-C. 문서 수정 — 정본 갱신 (7건, 코드 무변경)

| # | 문서 | 오류 |
|---|---|---|
| X1#5 | `ipc-contract.md:385` | `data-model.md §6` 참조가 hot-exit 미러 절을 가리킴(sync 무관). **sync gist 페이로드 스키마 절 자체가 `data-model.md` 에 부재** — 사용자 테마·로케일 팩이 업로드·디스크 기록된다는 사실 미기재 |
| X1#14 | `ipc-contract.md:7` vs `:755` | 같은 문서 안에 커맨드 수 178/181 과 179 공존, 코드는 179(+raw 3) |
| X1#15 | `data-model.md:22` | 디스크 레이아웃에 `locales/` · `lsp/<serverId>/<version>/` 누락 — **다운로드된 실행 파일이 영속 정본에 없음** |
| X1#16 | `data-model.md:142` | `disk_modified_ms` 를 "view 가 전달"로 서술, 실제는 Rust 가 직접 stat(Wave B 이후). `ipc-contract.md:509-514` 와 상반 |
| X1#17 | `data-model.md:72` | `ProjectRef.root` 를 `PathBuf` 로 기재(실제 `String`), `Project` 구조체 자체가 §3 에 부재 |
| X1#18 | `data-model.md:320-325` | specta `_Serialize`/`_Deserialize` 분할을 Snippet 2종만 기록하고 "두 절반은 동일"로 단정. 실제 7종이며 `ResolvedTheme` 은 5필드 필수성이 다름 |
| — | `architecture.md:77` | **§9 결정 11 참조** — 도메인 간 직접 호출 금지 조항이 30엣지에서 위반 중. 코드를 고칠지(T1-I) 규칙을 개정할지 결정 후 반영 |

**추가**: `docs/features/terminal.md:155`(탭 닫기 → `pty_kill`) · `architecture.md §6.3`(Drop 이중화) · `ipc-contract.md:693-696`(다중 창 LSP 구독)은 **코드가 그렇게 되어 있지 않은 상태를 사실로 기술**합니다. 이들은 코드 수정(T0 #21 · T1-D · T1-J) 완료 후 자동 정합되므로 별도 문서 작업 불필요합니다.

---

## 8. 종합 판단 — 1차 §6 증분

1차 §6 의 5개 결론 중 **1·3·4·5 는 유지**, **2 는 확대**, 그리고 **신규 3개**를 추가합니다.

**(유지) 1차 §6-1** 린터가 강제하는 축은 여전히 무결. 490 → 약 950파일로 범위가 두 배 늘었는데도 FSD 방향·`any`·`enum`·`function`·수동 메모이제이션 위반 0건. 유일한 정정은 §1.3 의 `expect` 1건.

**(확대) 1차 §6-2** — "문서에는 있으나 기계가 검사하지 않는 축"이 297건 전부의 출처라는 결론이 강화됩니다. 2차는 그 축을 **네 개로 특정**했습니다: ① 도메인 경계(`architecture.md:77`, 30엣지 위반) ② 자원 수명(`architecture.md §6.3`, 이중화된 자원 0개) ③ 락 입도(`§2.1` "잠금 안에서 IO 금지", 양방향 위반) ④ 계약 파리티(이벤트 4목록·Settings 필드 6곳·테마 토큰 200개, 테스트 0). **네 축 모두 문서에 정확히 규정되어 있고, 네 축 모두 강제 장치가 없습니다.**

**(신규) 6. 규칙과 코드가 어긋났을 때 코드가 이겼습니다.** `architecture.md:77` 은 도메인 간 직접 호출을 금지하지만 30엣지가 위반 중이고, `§3` 의 `ProjectCapability::attach/detach` 확장점은 스텁이며(R4#9), `§6.3` 의 Drop 이중화는 어떤 자원도 만족하지 않습니다. 이는 개별 위반이 아니라 **문서가 설계 의도를, 코드가 실제를 각각 기술하며 갈라진 상태**입니다. C13·C14 를 고치기 전에 §9 결정 11(규칙에 코드를 맞출 것인가, 코드에 규칙을 맞출 것인가)이 선행되어야 합니다.

**(신규) 7. 원격(remote) 표면이 감사되지 않은 채 확장되었습니다.** C16 6건 중 3건은 **바로 옆 줄에 같은 성격의 거부 arm 이 있는데도** 통과합니다(`agent_cli_install` ↔ `plugin_install`, `lsp_install` ↔ `vsix_import_plugin`). 거부 정책 자체는 `dispatch.rs:309-357` 에 정확히 문서화되어 있습니다. 결함은 판단력이 아니라 **"기본 허용 + 수기 거부 열거"라는 구조**에 있으며, 커맨드가 179종으로 늘어난 지금 이 구조는 유지 불가능합니다.

**(신규) 8. 도메인 담당 배치가 자기 도메인의 보안 결함을 놓쳤습니다.** §3 소유권 대조에서 확인된 사실입니다 — `theme_delete` 경로 취약점(R5 가 theme 전체를 정독하고도 미보고), LSP 다운로드 http 우회(R7 이 lsp 전체를 정독하고도 미보고), 키링 게이팅 비대칭(R5·R6 모두 미보고). 세 건 모두 **관통 배치(R1·R2·R3)가 "참조만" 하며 발견**했습니다. 도메인 내부 정독은 "이 도메인 안에서 일관적인가"를 보고, 관통 시선은 "다른 도메인은 어떻게 하는가"를 봅니다. **보안 결함은 후자에서 나옵니다** — 세 건 다 "형제 함수/이웃 도메인은 검증하는데 여기만 안 한다"는 비대칭이었습니다. 향후 감사 설계에 반영할 점입니다.

---

## 9. 사용자 확인 요청 — 1차 §7 대체

1차 6문항 중 **1번(6배치 대기 여부)은 해소**되어 제거합니다. 나머지 5문항 유지 + 신규 6문항.

### 해소된 것
- ~~1. 미수신 6배치를 기다릴 것인가~~ → 전량 수신 완료. **T0 는 이제 24항목이며, 그중 13항목이 6배치에서 나왔습니다. 1차 기준으로 착수했다면 보안 5건과 데이터 손상 4건을 놓쳤을 것입니다.**

### 유지 (1차 §7)

1. **T0 #1(watcher `.git` 필터) 수정 시 기존 테스트를 뒤집는 것을 승인하는가** — 현재 테스트가 결함을 정상 동작으로 고정 중
2. **T0 #3(`editor-instance-registry`)을 effect 이동으로 갈 것인가 `key={tabId}` 로 갈 것인가** — 추천: effect 이동(계약 무변경)
3. **F5#1 severity 하향(critical→major)을 승인하는가**
4. **T1-H(전역 락 IO 분리)를 T1 에 둘 것인가 별도 캠페인으로 뺄 것인가** — **추천 강화: 별도 캠페인.** 1차의 2건에서 **10건으로 확대**되었고(C11 양축), `begin_mutation` 의 입도 자체를 바꾸는 작업이라 96개 호출 지점의 암묵적 순서 보장이 걸립니다
5. **T2-E(`AppError` 369지점 코드화)를 백로그에 둘 것인가 별도 캠페인으로 승격할 것인가** — **추천 강화: 별도 캠페인.** R8#15 로 동일 근원이 3배치 확정되었고, T2-J(오류 분류 taxonomy)와 같은 작업이므로 병합하면 규모가 더 커집니다

### 신규 결정 지점

6. **원격 설치 커맨드 정책** — `lsp_install` 을 (A) 거부(`plugin_install`·`vsix_*` 와 일관, **추천**) / (B) 허용 유지 + 사유를 `dispatch.rs` 거부 분류 doc 에 명시. `agent_cli_install`·`agent_hooks_install(User)`는 관리자 권한 프롬프트·root-guard 밖 쓰기이므로 **거부에 이견 없음으로 전제**했습니다 — 다르면 알려주십시오
7. **설정 "해제" 규약** — (A) `Option<Option<T>>` + `double_option` deserializer(정확, 53필드 타입 변경) / (B) **`ai_omlx_base_url` 이 이미 쓰는 "빈 문자열=해제" 를 Option<String> 6필드로 일반화**(국소, 기존 선례 계승, **추천**) / (C) 별도 `unset: Vec<String>` 필드 추가
8. **git 반환 경로 규약** — (A) `StatusRow`/`CommitFile` 에 절대 경로 동봉(계약 변경, 정확, **추천**) / (B) 소비부(`git-panel-container.tsx`)에서 repo root join(1파일, 최소) / (C) `layout_open_tab` 이 File 탭 경로의 절대성을 경계에서 거절(방어선 추가, A·B 와 병행 가능)
9. **pty 세션 소유자** — (A) **Rust: `project_close` 가 해당 projectId 세션 일괄 kill + 탭 닫기 시 `pty_kill`**(T1-J 와 한 작업, **추천**) / (B) 프런트: 탭 닫기 확인 다이얼로그 후 `killPty` 호출(`docs/features/terminal.md:155` 의 기존 규정에 부합). 어느 쪽이든 `PtySession` Drop 추가는 필수
10. **`architecture.md:77`(도메인 간 직접 호출 금지) 정본 결정** — (A) **코드를 규칙에 맞춘다**(T1-I, 30엣지 정리, 위험도 중~높음, `ProjectCapability` 확장점 실현 포함) / (B) 규칙을 코드에 맞춘다(문서 개정 + 허용 엣지 명시) / (C) **분리안(추천)**: 순환 2건(plugin↔vsix, ide commands↔server)과 계층 역방향 1건(infra→domain), 커맨드 제2 진입점 1건(ide/server→layout::commands)만 T1-I 로 고치고, 나머지 명령형 엣지는 규칙에 예외 조항을 명문화
11. **X-A 8건의 살리기/지우기 판정** — §7 표의 권고(살리기 3: `terminal:cwd-changed`·`from_app`·`revision` / 지우기 2: `focus-kind-changed`·중복 커맨드 5종 / 미정 1: `layout_set_view_state`)를 일괄 승인하시겠습니까, 아니면 `layout_set_view_state`(커서·스크롤 영속)만 별도로 판단하시겠습니까
12. **`Project.capabilities`(R4#9)** — (A) `architecture.md §3` 대로 attach/detach 확장점 실현(T1-I 의 핵심 기구가 됨, **추천**) / (B) 필드·타입 제거 + 문서 반영. **결정 10 과 연동됩니다** — (A) 를 고르면 결정 10 의 (A) 가 자연스럽고, (B) 를 고르면 (C) 분리안이 맞습니다