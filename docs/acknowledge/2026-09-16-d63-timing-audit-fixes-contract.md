# d-63 — 같은 클래스 전수 조사 후속 수정 + 부팅 스플래시 계약 (2026-09-16)

> 조사 정본: `docs/research/2026-09-16-same-class-timing-bug-audit.md`(항목 1~13 번호 동일). 원형 버그: `docs/bug/2026-09-16-git-graph-pane-late-mount-constraints-crash.md`.
> 사용자 지시: "A, B 수정하고 C 는 정석적, 진짜로 보강하는 느낌으로 필요하다면 수정 아니면 패스. 또한 index.html 초기 구동 시 배경색을 내가 수정했는데 이거와 동시에 정중앙에 logo + TAIDE 문구가 나오게."
> 실행: wf `wf_f75f3381` — fixer 6개(opus·xhigh, 파일 소유권 분리) → 통합 검증 1회(sonnet·high: typecheck·lint·format·test·vite build).

## §0 근본 원인

각 항목의 근거·재현·검증자 반박은 조사 정본에 있다. 요약: 원형(등록 전 명령형 핸들 호출)과 정확히 같은 지점은 추가로 없었고, 인접 클래스에서 확인 3·경미 4묶음·미검증 6이 나왔다.

## §1 수정 방향·판정

| # | 대상 | 판정 | 방향 |
|---|---|---|---|
| 1 | `git-panel-container.tsx` AI 커밋 메시지가 프로젝트 전환 뒤 다른 저장소에 적용 | **수정(A)** | 전환 리셋에서 요청 ref/state 도 무효화 + projectId cleanup 에서 pending 생성 취소 |
| 2 | `terminal-view.tsx` 0px pane 에서 fit 이 (2,1) 을 PTY 로 전송 | **수정(A)** | "측정 가능한 컨테이너에서만 fit" 헬퍼(0 크기·애드온 하한 제안 건너뜀) — rAF·fontSize·fontFamily 3곳 공용 |
| 3 | `project-shell.tsx` 사이드바 리사이즈마다 setShellView IPC | **수정(A)** | `sidebarCollapsed` 와 같으면 생략 + `schedulePaneResizeCommit` 디바운스(형제 3곳과 동일) |
| 4 | clipboard writeText 무 catch 6곳 | **수정(B)** | `@shared/lib/copy-text-to-clipboard`(부재·거부 모두 false + 실패 토스트 `common.copyFailed`) 로 통일. 이미 방어된 3곳(terminal-pane·lsp-server-status-list·settings-remote)은 유지 |
| 5 | `pdf-preview.tsx:85` `void renderPage()` | **수정(B)** | try/catch — cancelled 면 무시, 아니면 `status='error'` |
| 6 | `command-palette.tsx` async `run` reject 무처리 | **수정(B)** | `runCommandSafely` 헬퍼(try/catch + Promise catch, 동기 throw 도 포착) — 두 dispatch 지점 공용 |
| 7 | IPC `void` 위생 5파일 | **수정(B)** | `.catch(() => undefined)` 관용구 통일, `Promise.all` 을 try 안으로. 실발생 없음(백엔드 항상 Ok) |
| 8 | `code-editor.tsx:323` 무조건 `editor.focus()` | **판정 위임(C)** | 터미널의 `autoFocus`(포커스된 pane 만) 정책이 이미 있으므로, 다중 pane 복원·split·language-only 변경에서 포커스 강탈이 코드로 확인되면 같은 정책으로 배선. 아니면 패스 |
| 9 | `pdf-preview.tsx` 외부 변경 후 재렌더 누락 | **수정(C)** | load effect 진입 시 `status='loading'` 으로 되돌려 ready 전이가 다시 일어나게 — 5 와 같은 파일, 정석 보강 |
| 10 | `pane-node-view.tsx:80` pane 제거 후 디바운스 발화 | **패스(C)** | 사용자 가시 효과 없음(무시되는 IPC 1회). 언마운트 취소는 모듈 설계(리마운트를 견디는 module-level 타이머, pane-resize-commit.ts JSDoc)와 충돌 — split 직후 리사이즈 영속을 잃을 위험이 더 큼 |
| 11 | `git-panel.tsx:449` 숨긴 슬롯에서 `resize(px)/0` | **패스(C)** | 확신 낮음·손상 경로 미입증(라이브러리가 `K()` 로 레이아웃 검증). 0 크기 가드는 다시 보일 때 재동기화 공백을 만든다 |
| 12 | 정적 Panel id 중복(슬롯 분할) | **수정(C)** | Separator `aria-controls` 가 panel id 를 참조해 두 번째 슬롯부터 접근성 연결이 틀리고 HTML 무효 → `useId()` 접두사. 테스트는 접미 매칭 |
| 13 | `AppToaster` 가 `TooltipProvider` 밖 | **수정(C)** | 한 줄 이동. 토스터는 ErrorBoundary 밖이라 미래의 툴팁 포함 토스트가 앱 루트를 죽이는 구조 차단 |
| S | 부팅 스플래시 | **신규** | `index.html` body 첫 자식 `#boot-splash`(fixed·중앙·검정 배경·`visibility: visible` 로 body 게이트 우회) + 인라인 icon.svg + "TAIDE". `html[data-theme-ready][data-locale-ready]` 신호로 페이드아웃. global.css 게이트 무변경 |

범위 외: `terminal-pane:160`·`lsp-server-status-list`·`settings-remote-section` 의 기존 clipboard 방어 3곳, `auxiliary-window-shell` 의 Panel id(별도 document).

## §2 검토 계획

- 통합 검증 1회(typecheck·lint·format:check·bun test 전체·vite build). 렌즈 검토는 생략 — 항목별 근거가 조사 정본에서 이미 반박 검증(major 10건)을 거쳤고, 메인이 diff 를 직접 대조한다(생략 근거: `agent-operations.md` §1 "다렌즈 수렴/기계 재현 + 메인 직접 확인").
- 사용자 실기: git 뷰 첫 오픈·프로젝트 전환(원형), AI 커밋 메시지 생성 중 프로젝트 전환, 세로 분할 터미널 창 축소, 스플래시 표시·페이드, Copy Path 실패 토스트(원격 미러).

## §3 기록

### 구현 (wf `wf_f75f3381`, fixer 6 병렬 — 메인이 diff 전수 대조, 2026-09-16)

| # | 실제 구현 | 회귀 테스트 | 계약과 다른 점 |
|---|---|---|---|
| 1 | `git-panel-container.tsx`: 전환 리셋 블록에 `setCommitMessageRequestId(null)`; ref 무효화 + 취소는 `useLayoutEffect(() => () => abandon(), [projectId])` cleanup(`useEffectEvent`)로 — 같은 커밋 안 동기 실행이라 passive 갭 없음. `finally` 에서 정상 완료 시 ref 도 null. 취소 API 가 백엔드 `tokio::select!` 를 실제로 깨움 확인(`domain/ai/commands.rs`) | `git-panel-container.test.tsx` 신규 2건(수정 전 "A 메시지가 B 입력창에" 재현 → 통과) | 렌더 중 ref 쓰기는 `react-hooks/refs` 가 막아 layout cleanup 으로 대체(타이밍 동등) |
| 2 | `terminal-view.tsx`: `fitIfMeasurable(term, fit, container)` 순수 헬퍼 — 0px 컨테이너·비유한·애드온 하한(cols≤2 **또는** rows≤1)·치수 동일이면 skip. rAF·fontSize·fontFamily 3곳 경유 | `terminal-view.test.ts` +7(red 4 → green) | 초기 `fit.fit()`(:325) 미가드 → 후속 워커에서 보강 |
| 3 | `project-shell.tsx`: `collapsed === sidebarCollapsed` 면 return + `schedulePaneResizeCommit(\`${projectId}:sidebar-collapsed\`)` | 없음(하네스 0px 라 리사이즈 제스처 불성립 — 부채) | — |
| 4 | `@shared/lib/copy-text-to-clipboard`(try/catch: 거부·부재 모두 false + `common.copyFailed` 토스트). 호출부 6곳 교체, `common.copyFailed` en/ko/ja + `MESSAGE_NAMESPACES` 등재 | 유틸 3건 | — |
| 5·9 | `pdf-preview.tsx`: `renderPage` try/catch(취소면 무시, 아니면 error) + `data` 교체 시 렌더 단계 state 보정(`loadedData` 비교 → `status='loading'`) | `pdf-preview.test.tsx` 신규 2건 | effect 안 `setStatus` 는 `react-hooks/set-state-in-effect` 가 막아 렌더 단계 보정으로(editor-pane 선례) |
| 6 | `command-registry.ts` `runCommandSafely`(try/catch + Promise catch → `toast.error(describeIpcError)`), 팔레트 2 dispatch 지점 교체 | `command-registry.test.ts` +3 | — |
| 7 | 5파일 `.catch(() => undefined)` 통일, `agent-external-open-provider` 의 `Promise.all` 을 try 안으로 | 없음(백엔드 항상 Ok — 부채) | — |
| 8 | **REAL 판정 → 수정**: `CodeEditor.autoFocus?`(기본 true) + `EditorPane.autoFocus` 필수 + `pane-node-view` 가 `node.id === focusedPaneId` 전달. path 불변·language 만 바뀐 재실행은 focus 안 함 | `code-editor.test.tsx` 신규 5건(red 2 → green) | `untitled-pane`·`app-file-pane` 은 기본값 true 로 기존 동작 유지 → 후속 워커에서 배선 |
| 12 | `useId()` 접두 — project-shell(explorer/editor)·editor-area(editor-panes/problems-panel)·git-panel(git-changes/git-graph). 테스트는 정규식 접미 매칭 | `git-panel.test.tsx` +1(두 패널 id 상이) | `GIT_GRAPH_PANEL_RESIZE_KEY` 슬롯 공유 → 후속 워커 |
| 13 | `AppToaster` 를 `TooltipProvider` 안으로 | 없음(도달 경로 없음) | — |
| S | `index.html`: `#boot-splash`(fixed·중앙·#000·`visibility: visible` 로 body 게이트 우회) + 인라인 icon.svg(그라데이션 id `boot-` 접두) + "TAIDE"(20px·600·0.2em, `text-indent` 로 광학 중앙). `html[data-theme-ready][data-locale-ready]` 에서 150ms 페이드, reduced-motion 시 즉시. 사용자의 `html { background: #000 }` 유지 | 없음(실기 확인 대상) | — |

### 통합 검증 (sonnet·high, 1회)

`bun run typecheck` exit 0 · `bun run lint` 0 error(기존 warning 11 — TanStack Virtual incompatible-library·exhaustive-deps, 미변경 파일 포함) · `bun run format:check` 통과 · **`bun test` 2733 pass / 0 fail(273 files)** · `bun run build` 성공. 알려진 것: `bun test src/widgets` 처럼 **부분 실행**하면 `use-editor-lsp-integration.test.ts` 의 부분 mock 이 새어 다른 컨테이너 테스트가 실패하나(`test-conventions.md` §3 클래스) 전체 실행에서는 발생하지 않음.

### 후속 워커 (wf `wf_e7f3f4b5`)

초기 `fit.fit()` 가드 · untitled/app-file pane autoFocus 배선 · 그래프 디바운스 키 인스턴스화 · editor-pane JSDoc 리터럴 정정 — **전부 반영**(2026-09-16).

- `terminal-view.tsx` 초기화 effect 의 `fit.fit()` → `fitIfMeasurable`. 눌린 pane 에서 터미널 탭을 열면 PTY 가 2×1 대신 xterm 기본 80×24 로 spawn 되고 ResizeObserver 가 실제 치수로 정정(동작 변화, 안전한 쪽). 초기화 effect 자체는 실 xterm 이 필요해 회귀 테스트 없음(부채).
- `untitled-pane`·`app-file-pane` 에 `autoFocus: boolean` 필수 prop, `pane-node-view` 가 `node.id === focusedPaneId` 전달(호출부는 각 1곳뿐임을 grep 으로 확인).
- `GIT_GRAPH_PANEL_RESIZE_KEY` 를 접미로 바꾸고 `${panelIdPrefix}:git-graph-panel` 인스턴스 키 사용.
- 검증: `bun run typecheck` 0 · `bun test` 2733 pass / 0 fail · eslint 0 error(기존 warning 2) · prettier 통과.

### 테스트 부채(요약)

항목 3(사이드바 리사이즈 IPC 1회 합침)·7(백엔드 Err 분기 부재)·13(토스트 안 툴팁)·S(스플래시 렌더)·터미널 초기 fit 경로·clipboard 실기 거부 토스트 — 전부 하네스(0px 레이아웃·IPC 없음) 밖이라 실기/e2e 로만 확인 가능.

## §4 후속

- 항목 10·11 은 패스 사유와 함께 QA 부채로 유지(실기에서 증상이 보이면 재개).
