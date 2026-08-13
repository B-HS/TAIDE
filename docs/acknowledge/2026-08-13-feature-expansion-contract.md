# 기능 확장 1차 구현 계약 (2026-08-13)

> 사용자 요구 4건(blame 위치·에디터 액션 노출·taide CLI PATH·갭 분석) + 갭 P0 퀵윈.
> 정찰 wf_58bd73f9-299(opus 5: blame/actions/cli/inventory/gap) — monaco 소스 인용·taide-cli 실체·
> LSP capabilities 최소 선언은 메인이 실물 재확인. 정찰 원문: 세션 스크래치패드
> `recon2-{blame,actions,cli,inventory,gap}.md`(+`monaco-0.56-actions.tsv`), 갭 분석 정본은
> `docs/research/2026-08-13-vscode-cursor-gap.md` 로 영구 저장됨.

## 1. 사용자 결정 (2026-08-13 확정 — 전부 추천안 채택)

| # | 결정 | 선택 |
|---|------|------|
| 1 | blame 표시 | 포커스 줄 **위** 작은 글자(사용자 지시). 구현은 IViewZone(메인 확정 — §3.1) |
| 2 | 에디터 액션 실행 소유권 | **B안: 표시·검색·재바인딩은 우리 모달, 실행은 monaco**(addKeybindingRules override, chord·precondition 유지) |
| 3 | 액션 i18n | **하이브리드** — tier1 42건만 en/ko/ja 번역, 나머지는 monaco 영어 label 폴백 |
| 4 | taide CLI | `/usr/local/bin/taide` 심링크(osascript 관리자) + externalBin 사이드카 + **열기 배선 포함**(콜드스타트 argv·AgentExternalOpen 구독·폴더/파일 분기) + dev 빌드 거부 + macOS 전용 + 팔레트·설정 양쪽 + Shell Command 카테고리. 빈 인자 `taide` 는 미포함 |
| 5 | 갭 착수 범위 | 이번 웨이브 = 1~3 + **P0 퀵윈 5건**(sticky scroll·커서위치 상태바·documentHighlight·selectionRange·LSP capabilities 확충). 나머지 P0/P1 은 QA 후 별도 편성 |

## 2. 확정 사실 (정찰 + 메인 재검증)

- **blame 충돌 구조**: 현행 blame 은 포커스 줄 끝 `after` 주입 — auto-tab ghost text 도 같은 줄
  `after` 주입(ghostTextView.js)이라 이어 붙는다. 줄 위로 옮기면 충돌이 구조적으로 소멸.
  IViewZone 의 시프트는 monaco 자신의 `keepCursorStable` 관용구(존이 커서 위면
  setScrollTop±h)로 커서 기준 제거 가능(실물 확인). `afterLineNumber: 0` = 첫 줄 앞 지원.
  CodeLens 는 provider 가 전역이라 diff 페인에도 렌즈가 뜨는 문제로 기각.
- **monaco 액션 3계층**: EditorAction 156(getSupportedActions 대상) / Action2 47 / EditorCommand 136.
  실행은 반드시 `editor.trigger(source, id)` — standalone 이 `setActiveCodeEditor` 후 커맨드 폴백
  (실물 확인)이라 Action2 계열(goTo/peek/quickFix)도 동작. `getAction().run()` 은 B계층 누락.
  실질 노출 상한 **150건**(A 131 + B 19). 전수표: `monaco-0.56-actions.tsv`.
- **재바인딩**: `monaco.editor.addKeybindingRules` 공개 API(실물 확인) — `weight1: 1000` 으로
  기본을 이김, `command: '-<id>'` 로 기본 제거, IDisposable 반환. chord 는 KeyMod.chord 숫자로 표현.
- **triggerSuggest 가 "없어 보이는" 원인**: macOS 가 Ctrl+Space 를 입력 소스 전환으로 선점(OS 레벨,
  VS Code 도 동일 이슈). mac 2nd 바인딩 Cmd+I·Opt+Esc 는 동작 예상(실기 확인 대상).
- **잠재 결함(수정 대상)**: 전역 capture keydown 이 매칭 시 preventDefault 만 하고
  stopPropagation 을 안 함 — monaco 와 키가 겹치면 이중 실행 소지.
- **taide-cli**: 워크스페이스 크레이트(`crates/taide-cli`, bin 명 taide-cli — GUI 덮어쓰기 사고
  이력으로 개명됨). 기능 = 파일 열기+`--wait` 마커 + `hook` 서브커맨드. **어느 배포 산출물에도
  미동봉**(tauri.conf.json 에 externalBin/resources 없음 — 메인 확인). 설치/제거 커맨드 없음,
  hooks 사용자 레벨 설치가 CLI 존재를 선행 조건으로 거부 중.
- **열기 경로 3끊김**: ① `AgentExternalOpen` 이벤트 프론트 구독 0건(`releaseWaitMarker` 호출부도
  0건) ② single-instance 는 두 번째 인스턴스만 argv 전달 — 콜드 스타트 시 argv 처리 코드 부재
  ③ `project_open` 은 디렉토리 전용 — 파일/폴더 분기 필요.
- **VS Code 방식**(원본 소스 확인): 심링크 + osascript `with administrator privileges`, 멱등,
  취소는 조용히, uninstall 은 unlink→EACCES 시만 osascript. VS Code 는 소유 검증을 안 하지만
  TAIDE 는 강화(read_link 가 taide-cli 로 끝날 때만 삭제).
- **LSP capabilities 최소 선언**(lsp/service.rs:305~, 메인 확인): documentSymbol·inlayHint·
  signatureHelp 를 미선언 상태로 요청 중 — 엄격한 서버에서 기능 저하 소지(갭 P0-0).
- **갭 분석**: P0 10·P1 10 — `docs/research/2026-08-13-vscode-cursor-gap.md`. backlog 정정 1건
  반영(임의 두 파일 비교 = 구현 완료).

## 3. 확정 설계

### 3.1 blame view zone (B-g, 프론트 전용)

- 신규 `src/shared/lib/monaco/blame-zone.ts`: `createBlameZoneController(editor) => { show(line, text), hide(), dispose() }`.
  `changeViewZones` 로 `afterLineNumber: line-1`(첫 줄은 0), `afterColumn` 대값, `suppressMouseDown`.
  높이 = CodeLens 식(`BLAME_FONT_SIZE_RATIO 0.9`·`BLAME_MIN_LINE_HEIGHT_FACTOR 1.3`, EditorOption 실측).
  존 추가/제거 시 `setScrollTop ± height`(존은 항상 커서 위). `onDidChangeModel` 에서 존 id 리셋
  (view zone 은 뷰 소유 — setModel 시 좀비화, 실물 확인), `onDidChangeConfiguration`(fontSize/lineHeight) 재계산.
- `editor-pane.tsx`: 조회 effect(디바운스·seq)는 유지, 데코레이션 effect 만 컨트롤러 호출로 교체.
- CSS: `.taide-blame-text` 제거 → `.taide-blame-zone`(기존 `--taide-editor-blame-*` 토큰 재사용,
  폰트 크기·줄높이는 컨트롤러가 인라인 주입). 테마 토큰 변경 없음(5곳 동기 비대상).
- suggest/hover 위젯이 존을 덮는 것은 허용(레이아웃 무파손).

### 3.2 에디터 액션 노출 (B-h, 프론트 전용 — B안·하이브리드)

- 신규 `src/shared/lib/monaco-actions.ts`: 정적 카탈로그 150건
  `{ actionId, categoryKey, defaultLabel, defaultBindingLabel(mac 표기 문자열 — chord 포함 표시용), keybinding?(재바인딩용 숫자 조합 불요 — 기본은 monaco 가 이미 가짐) }`.
  소스는 `monaco-0.56-actions.tsv`(정찰 산출물). 제외: quickCommand(F1, 팔레트 이중화)·
  linkedEditing(provider 부재 오탐)·dev 전용 4종·위젯 내부 transient.
- 커맨드 승격: id `monaco.<actionId>`, run = 브리지 `{ type: 'run-monaco-action', actionId }` →
  `editor-area` 에서 활성 에디터 `editor.trigger('taide.command', actionId, undefined)`.
  등록은 bootstrap-commands.
- 카탈로그 행: `buildKeybindingRows` 에 monaco 행 소스 추가 — 기본 바인딩은 **표시 전용
  문자열**(defaultBindingLabel, chord 그대로 표기), `source: 'monaco'`.
- **재바인딩(B안)**: 사용자가 monaco 행에 새 키 지정 시 override 저장(actionId `monaco.<id>`) +
  `addKeybindingRules([{ command: '-<id>' }, { keybinding: 변환값, command: '<id>' }])` 적용.
  부팅 시 저장된 override 전량 재적용(app 부트스트랩). `{key,mods}` → KeyMod/KeyCode 변환 유틸
  신설(`src/shared/lib/monaco-keybinding.ts` + 테스트). chord 로의 재바인딩은 미지원(캡처가 단일
  키) — 기본 chord 표시는 유지, 해제만 가능.
- **전역 capture 는 monaco 행을 실행하지 않는다**(이중 실행 방지). 부수 수정: capture 매칭 시
  `stopPropagation` 추가(기존 잠재 결함).
- 회색 처리: `CommandContext.activeEditorActionIds: Set<string> | null` — A계층만 판정, B계층은
  상시 활성(실패 시 무해).
- i18n: 라벨 해석 = `t('keymap.monaco.<actionId>', { defaultValue: defaultLabel })` 헬퍼
  (`formatCategorizedLabel` 시그니처 확장). tier1 42건(§4 목록)만 스파인이 en/ko/ja 등록.
  카테고리 8종 신설: `keymap.category.editorSuggest/editorNavigation/editorSelection/editorLines/editorFolding/editorFormat/editorRefactor/editorDisplay`.

### 3.3 taide CLI (B-rust + C-front)

- **Rust(B-rust)**: `domain/agent/service.rs` 순수 로직(경로 조립·심링크 소유 판정·AppleScript
  문자열 이스케이프·osascript 인자 생성 — 전부 테스트) + `commands.rs`
  `agent_cli_install/agent_cli_uninstall`(멱등: 이미 우리 target 이면 즉시 성공. uninstall 은
  unlink→PermissionDenied 시만 osascript, NotFound=성공, **read_link 가 taide-cli 아니면 거부**).
  취소(exit 1·"-128")는 에러 아님. dev 빌드(current_exe 가 .app 밖) 는 InvalidArgument 거부.
  비 macOS 도 거부. `agent_cli_status` dangling 판정 보강(canonicalize 실패 = 재설치 필요 상태).
  **콜드 스타트 argv**: lib.rs setup 에서 `std::env::args` 를 `parse_cli_payload` 로 처리해
  AgentStore 의 **pending 큐**에 적재. 신규 `agent_pending_external_opens`(drain) 커맨드 —
  single-instance 콜백도 프론트 미구독 시 유실되지 않게 pending 큐 경유로 통일.
  lib.rs collect_commands +3, dispatch.rs 파리티 **137→140**, bindings 재생성.
  **사이드카**: tauri.conf.json `bundle.externalBin: ["binaries/taide-cli"]` +
  `scripts/build-cli-sidecar.ts`(cargo build -p taide-cli --release → `src-tauri/binaries/taide-cli-<triple>` 복사)
  + package.json 빌드 파이프라인(beforeBuildCommand) 편입. 심링크 target =
  `TAIDE.app/Contents/MacOS/taide-cli`(bundler 가 트리플 접미사 제거 — 정찰 확인).
  **LSP capabilities 확충**(동일 에이전트 — Rust 단일 소유): lsp/service.rs initialize 에
  구현 어댑터와 일치하는 선언 추가 — documentSymbol.hierarchicalDocumentSymbolSupport,
  hover.contentFormat, signatureHelp, inlayHint, references, definition, formatting,
  documentHighlight, selectionRange(신규 어댑터 대응). **미구현 기능은 선언 금지**.
- **프론트(C-front, B-h·B-k 완료 후)**: `entities/agent/agent.commands.ts` 신설(팔레트 2건 —
  "Shell Command: Install 'taide' command in PATH"/"Uninstall", macOS 가드) + bootstrap 등록 1줄,
  agent.ipc/query 확장(mutation onSuccess 로 CLI 상태 setQueryData), 설정 에이전트 섹션 설치/제거
  버튼(막다른 경고 해소), **AgentExternalOpen 구독 + 부팅 시 pending drain** — is_dir 분기
  (폴더 → project_open, 파일 → 해당 프로젝트 탭 열기·프로젝트 밖이면 정책: 폴더 없이 파일만
  열 수단이 없으면 토스트 안내), wait marker 는 열린 탭이 닫힐 때 `releaseWaitMarker` 호출
  (모듈 레벨 path→marker 맵).

### 3.4 P0 퀵윈 (스파인 + B-k 프론트)

- **sticky scroll**: Settings `editor_sticky_scroll_enabled: bool`(기본 true — VS Code 기본과
  일치) 스파인 배관(types/service/sync/service.rs/patch) → code-editor.tsx 옵션 + 설정 외관/에디터
  섹션 토글.
- **커서 위치 상태바**: 상태바에 `Ln N, Col M`(`editor.cursorPosition` 키, {{line}}/{{column}}
  보간) — 활성 에디터 커서 이벤트 구독, 클릭 시 gotoLine 액션 트리거(선택).
- **documentHighlight·selectionRange 어댑터**: `src/shared/lib/lsp/adapters/` 2파일 신설 +
  lsp-session-registry 등록 + client.ts FEATURE_CAPABILITY_CHECKS 확장(서버 capability 게이트).

### 3.5 실행 구조

Phase A 스파인(sonnet+high 단일): locale 4곳(카테고리 8 + tier1 42 + CLI ~12 + 퀵윈 ~3) +
Settings sticky 필드 전체 배관(**sync/service.rs 의 SettingsPatch 리터럴 포함** — 직전 웨이브
교훈) + bindings 재생성 + emptySettingsPatch. **컴파일 그린 상태로 종료**(enum 함정 없음 확인).
Phase B 병렬 4(sonnet+high): B-rust / B-g / B-h / B-k — 파일 소유 분리(Rust 는 B-rust 단독,
설정뷰는 B-k, 커맨드·키바인딩 계열은 B-h, 에디터페인·blame 은 B-g).
Phase C 직렬 1: C-front(CLI 프론트 — bootstrap·settings-view 접촉이라 B-h·B-k 뒤).
Phase D 검토: 렌즈 3(opus+high) → 적대적 검증(opus+medium) → 수정(sonnet+high).
Phase E 메인 2차: verify 전체 + vite build + 커밋(dev 자동).

## 4. tier1 번역 42건 (키 = `keymap.monaco.<actionId>`)

editor.action.triggerSuggest · triggerParameterHints · showHover · revealDefinition ·
peekDefinition · goToReferences · quickOutline · gotoLine · marker.next · marker.prev ·
marker.nextInFiles · marker.prevInFiles · rename · quickFix · refactor · formatDocument ·
formatSelection · organizeImports · smartSelect.expand · smartSelect.shrink · insertCursorAbove ·
insertCursorBelow · insertCursorAtEndOfEachLineSelected · addSelectionToNextFindMatch ·
selectHighlights · duplicateSelection · copyLinesUpAction · copyLinesDownAction ·
moveLinesUpAction · moveLinesDownAction · deleteLines · joinLines · indentLines · outdentLines ·
commentLine · blockComment · transformToUppercase · transformToLowercase (이상 editor.action.*)
+ editor.fold · editor.unfold · editor.foldAll · editor.unfoldAll

## 5. 기각된 대안

| 기각안 | 사유 |
|--------|------|
| blame CodeLens | provider 전역 → diff 페인 오염, 커서 추종 재-provide 비용, 'no commands' 경로 |
| blame ContentWidget 오버레이 | 위 줄 코드를 가림 — 불투명 배경 필요 = 36테마 재변환 유발 |
| blame 이전 줄 끝 after | 윗줄 주석으로 오독·긴 줄에서 화면 밖·워드랩 어긋남 |
| 액션 A안(우리 단일 소유) | chord 기본값(폴딩 12종) 표현 불가·when 평가 수제 — 사용자 기각 |
| 액션 전량(339)·A+B(203) 노출 | 커서 프리미티브·위젯 내부 액션은 소음 — 150 전수 채택 |
| getAction().run() 실행 | Action2 19건(goTo/peek/quickFix) 누락 — trigger() 채택 |
| 액션 i18n 전량 번역 | locale 600줄·monaco 버전업마다 동기 — 하이브리드 채택 |
| CLI ~/.local/bin | macOS 기본 PATH 밖 + 기존 상수·hooks 경로와 불일치 |
| CLI 심링크만(열기 배선 없이) | taide . 이 앱만 띄우고 안 열림 — "설치 성공" 착시 |
| CLI resources 셸 wrapper | 네이티브 CLI 가 이미 존재 — 이중화 |
| 빈 인자 taide 지원 | 파서 변경 별도 축 — 사용자 미채택 |
| P0 대량 착수(3-way 머지 등) | QA6 실기 미완 상태 — 퀵윈만 편입, 대형은 별도 Phase |

## 6. 완료 조건

- `bun run verify` 전체 + vite build. 파리티: dispatch 140·locale 4곳·en⊆required.
- 실기 확인(사용자): blame 위치·시프트 체감, 에디터 액션 실행·재바인딩·회색 처리,
  taide 설치→`taide .`/`taide 파일` 열기·`--wait`, sticky scroll·커서위치·하이라이트.
  qa6-checklist 에 항목 추가.
