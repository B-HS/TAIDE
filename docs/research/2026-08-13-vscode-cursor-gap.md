# VS Code · Cursor 대비 TAIDE 갭 분석 (2단계)

조사일 2026-08-13 / dev 브랜치. TAIDE 상태는 1단계 인벤토리 + 이번에 직접 확인한 코드(file:line)를 근거로 한다. VS Code·Cursor 는 공식 문서를 fetch 해 대조했다(URL 은 맨 아래).

---

## 0. 이번 정찰에서 추가로 확인한 아키텍처 사실 (난이도 판정의 근거)

1. **Monaco 는 full ESM 로 임포트된다** — `src/shared/lib/monaco/setup.ts:1` (`import * as monaco from 'monaco-editor'`, 0.56.0). 따라서 standalone 기여 전량(멀티커서·컬럼선택·코드 접기·find/replace 정규식·`editor.action.gotoLine`·`editor.action.quickOutline`·peek 위젯 인프라)이 이미 번들에 들어 있다. **"없는" 기능 상당수는 미구현이 아니라 미배선/미노출이다.**
2. **Monaco 0.56 이 제공하는 provider 등록 API** (`node_modules/monaco-editor/monaco.d.ts` 직접 확인):
   `registerCodeActionProvider` · `registerCodeLensProvider` · `registerDocumentHighlightProvider` · `registerFoldingRangeProvider` · `registerSelectionRangeProvider` · `registerDocumentSemanticTokensProvider` / `registerDocumentRangeSemanticTokensProvider` · `registerDeclarationProvider` · `registerImplementationProvider` · `registerTypeDefinitionProvider` · `registerLinkedEditingRangeProvider` · `registerOnTypeFormattingEditProvider` · `registerColorProvider` · `registerLinkProvider` · `registerNewSymbolNameProvider`.
   → 이 13종은 전부 **기존 어댑터 패턴(`src/shared/lib/lsp/adapters/*.ts` 11종)에 파일 하나 추가**로 붙는다.
   **반대로 call hierarchy / type hierarchy 는 Monaco API 자체가 없다** → VS Code 급으로 하려면 커스텀 패널 자작(난이도 상).
3. **LSP `initialize` 의 클라이언트 capabilities 가 최소 선언이다** — `src-tauri/src/domain/lsp/service.rs:305-317`. 현재 선언: `general.positionEncoding`, `workspace.{workspaceFolders,configuration,didChangeConfiguration}`, `textDocument.{synchronization.didSave, completion.completionItem.snippetSupport, rename.prepareSupport, publishDiagnostics.relatedInformation}` — **끝**.
   - `codeAction` · `semanticTokens` · `documentHighlight` · `foldingRange` · `selectionRange` · `codeLens` · `documentSymbol.hierarchicalDocumentSymbolSupport` · `inlayHint` · `signatureHelp` · `hover.contentFormat` · `workspace.symbol` · `workspace.applyEdit` · `workspace.executeCommand` **미선언**.
   - 즉 documentSymbol·inlayHint·signatureHelp 어댑터는 **capability 를 선언하지 않은 채 요청**하고 있다. 엄격한 서버는 계층형 심볼 대신 플랫 심볼을 주거나 기능을 끈다(예: jdtls·gopls 계열). **이건 "갭"이자 잠재 버그다.**
   - 프론트 게이트 테이블은 `src/shared/lib/lsp/client.ts:39-55` (`FEATURE_CAPABILITY_CHECKS`) — 확장 지점이 이미 있다.
4. **에디터 생성 옵션** `src/features/editor/code-editor.tsx:106-113` — `stickyScroll` 미설정(Monaco 기본 비활성), `glyphMargin: false`. sticky scroll 은 **옵션 한 줄** 수준.
5. **팔레트 모드는 `files` / `commands` 2종뿐** — `src/shared/lib/command-registry.ts:49-52` (`parsePaletteQuery`). `@`(심볼)·`:`(줄) 확장 지점은 이 함수 하나.
6. **dirty 미러**가 이미 있다 — `src-tauri/src/domain/file/service.rs:187` `mirror_dirty`, 호출 `src/widgets/editor-pane/editor-pane.tsx:125`. VS Code 의 Hot Exit 에 해당하는 안전망이 이미 존재.

---

## 1. 갭 표 — 에디터 핵심

| 기능 (VS Code) | TAIDE 상태 | 근거 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|---|
| 멀티커서·컬럼선택·라인이동/복제 | **있음** (Monaco 내장) | `setup.ts:1` full ESM | - | - | - |
| 코드 접기(folding) | **있음** (대형 파일만 off) | `code-editor.tsx:152` | - | - | - |
| 미니맵·워드랩·들여쓰기·공백표시 등 옵션 | **있음** (설정 44필드) | `settings/types.rs:62-81` | - | - | - |
| 찾기/바꾸기(정규식·단어단위) | **있음** (Monaco find) | `command-registry.ts:133` | - | - | - |
| **Sticky Scroll** | **없음** (옵션 미설정) | `code-editor.tsx:106-113` | 중 | **하** (옵션 1줄 + 설정 필드) | **P0** |
| **Breadcrumbs (경로+심볼 계층)** | **없음** | grep 0건 | 중 | 중 (documentSymbol 어댑터 재사용 + 헤더 UI) | **P0** |
| **사용자 정의 스니펫** (tabstop/placeholder/변수/transform) | **없음** (LSP 서버가 주는 스니펫만 통과) | backlog 참조 | 중 | 중 (Monaco `CompletionItemInsertTextRule.InsertAsSnippet` + 스니펫 파일 스키마·Rust 소유) | **P0** (backlog 기재) — **구현 완료**(Wave F, 2026-08-15 — `entities/snippet`·`widgets/snippet-editor`·`docs/features/editor.md` §10). 항목 종결 |
| **Emmet** (HTML/CSS 약어 확장) | **없음** | grep 0건 | 중(웹 작업 시 상) | 중 (`emmet-monaco-es` 의존성 추가 필요 → 승인 사항) | P1 — **구현 완료**(Wave F, 2026-08-15 — `shared/lib/emmet-integration.ts`·`docs/features/editor.md` §11). 항목 종결 |
| Format Document / Selection | 있음 (LSP formatting 어댑터) | `adapters/formatting.ts` | - | - | - |
| **Format on Type / on Paste** | 없음 (`registerOnTypeFormattingEditProvider` 미사용) | monaco.d.ts 대조 | 하 | 하 | P2 — **구현 완료**(Wave F, 2026-08-15 — `adapters/formatting.ts`의 `registerOnTypeFormatting`/`registerRangeFormatting`·`docs/features/editor.md` §9). 항목 종결 |
| **Code Actions on Save** (organizeImports 등) | **없음** | codeAction 자체 부재 | 상 | 중 (codeAction 선행) | **P0**(codeAction 묶음) |
| Auto Save / 저장 시 포맷 | 있음 | `settings/types.rs:44-47` | - | - | - |
| Hot Exit(미저장 복원) | **있음(동등)** | `file/service.rs:187` mirror_dirty | - | - | - |
| **파일 비교: 클립보드와 비교 / 저장본과 비교** | 없음 (파일↔파일 비교만 있음) | `explorer-container.tsx:254` | 하 | 하 (DiffPane 재사용) | P2 |
| **Overtype 모드 / 파일 인코딩 선택** | 없음 | grep 0건 | 하 | 중(인코딩은 Rust 읽기 경로 변경) | P2 |
| 접근성: Accessible View·오디오 신호·Tab Focus Mode·고대비 테마 | **없음/미점검** | backlog "접근성" | 하(단일 사용자) | 상 | P2 (backlog 기재) |

---

## 2. 갭 표 — 코드 인텔리전스 (LSP)

TAIDE 어댑터 11종: completion·diagnostics·hover·definition·references·rename·formatting(2)·signatureHelp·inlayHints·documentSymbol.

| 기능 | TAIDE 상태 | Monaco API | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|---|
| **Code Action / Quick Fix (⌘.)** | **없음** — grep `codeAction` 0건 | `registerCodeActionProvider` 있음 | **최상** (진단은 보이는데 고칠 수단이 없음) | 중 (어댑터 1 + `workspace/applyEdit`·`executeCommand` 대응 + initialize 선언) | **P0-1** |
| **LSP initialize capabilities 확충** | **최소 선언** | - | 상 (기존 기능 정확도에도 영향) | 하~중 | **P0-0 (선행)** |
| **Semantic Tokens** | 없음 | `registerDocumentSemanticTokensProvider` | 상 (TextMate 만으로는 타입/변수 구분 불가) | 중 (토큰 legend → 테마 토큰 매핑 필요, W7 build-shiki-theme 와 연결) | **P0** — **구현 완료**(Wave F, 2026-08-15 — `adapters/semantic-tokens.ts`·`mapping-tables.ts`·`build-shiki-theme.ts`·`docs/features/editor.md` §8). 항목 종결 |
| **Document Highlight** (커서 심볼 동일 위치 강조) | 없음 | `registerDocumentHighlightProvider` | 상 (일상 체감 큼, 구현은 가장 쌈) | **하** | **P0** |
| **Selection Range** (⌃⇧⌘←/→ 확장 선택) | 없음 | `registerSelectionRangeProvider` | 중 | **하** | **P0** |
| **Folding Range (LSP 기반)** | 없음 (Monaco 들여쓰기 추정만) | `registerFoldingRangeProvider` | 중 | 하 | P1 |
| **Code Lens** (참조 수·실행 렌즈) | 없음 | `registerCodeLensProvider` | 중 | 중 | P1 |
| **Peek Definition / Peek References** | **미확인** — Monaco 내장 액션(`peekDefinition`·`referenceSearch`)이 provider 등록만으로 동작할 가능성 높으나 키맵/실기 확인 안 됨 | - | 상 | **하** (확인 후 키맵 등록만일 수 있음) | **P0(확인)** |
| **Go to Implementation / Type Definition / Declaration** | 없음 | 3종 register API 있음 | 중 | 하 (definition 어댑터 복제) | P1 |
| **Workspace Symbol (⌘T)** | 없음 | LSP `workspace/symbol` + 팔레트 모드 | 상 | 중 | **P0** |
| **Go to Symbol in File (`@`) / Go to Line (`:`)** | 팔레트 미지원. 단 Monaco 내장 `quickOutline`(⇧⌘O)·`gotoLine`(⌃G)은 에디터 포커스 시 동작할 가능성 | `command-registry.ts:49-52` | 상 | 하 (파서 확장 + outline 어댑터 재사용) | **P0** |
| **Call Hierarchy / Type Hierarchy** | 없음 | **Monaco API 부재** → 자작 필요 | 중 | **상** | P2 |
| **Refactor Preview** (변경 미리보기 후 부분 적용) | 없음 (rename 은 즉시 적용) | - | 중 | 상 (다중 파일 diff 미리보기 UI) | P1 |
| **Linked Editing** (태그 동시 편집) | 없음 | `registerLinkedEditingRangeProvider` | 하 | 하 | P2 |
| **Color Provider / Document Link** | 없음 | 2종 API 있음 | 하 | 하 | P2 |
| 에러/경고 순회(F8) | 없음(Problems 패널만) | `editor.action.marker.next` 내장 | 중 | **하** (키맵 등록) | P1 |

---

## 3. 갭 표 — 탐색 · 워크스페이스 · 레이아웃

| 기능 | TAIDE 상태 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|
| 파일 트리(가상화·lazy·watcher·CRUD·컨텍스트 메뉴) | 있음, VS Code 동급 이상 | - | - | - |
| **멀티 윈도우 / Move into New Window / 부동 에디터** | **없음** — `capabilities/main.json` 단일 `main` 전제 | 중 | **상** | P1 (backlog 기재) |
| **멀티 루트 워크스페이스(`.code-workspace`)** | 없음 (프로젝트 1 = 폴더 1, 다중 프로젝트는 사이드바로 분리) | 중 | 상 (프로젝트 모델 변경) | P2 — TAIDE 멀티프로젝트로 상당 부분 대체 |
| **Zen / 포커스 모드, Centered Layout** | 없음 | 중 | 중 (layout 도메인 표시 필드) | P1 (backlog 기재) |
| **Timeline 뷰 / 파일 단위 히스토리** | 없음 | **상** | 중 (`git_log -- <path>` + 패널) | **P0** |
| **로컬 히스토리(파일 자동 백업·복구)** | 없음 | 중 | 중 | P2 |
| **북마크** | 없음 | 하 | 중 | P2 (backlog 기재) |
| **커서 위치(줄:열) 상태바 표시** | 없음 | 중 (VS Code 관성) | **하** | **P0** (roadmap 7.5-D) |
| Secondary Side Bar / 패널 위치 이동 | 없음 | 하 | 중 | P2 |
| 탭 컨텍스트 메뉴 5항목(Move/Copy into New Window·File History·Find File References·Keep Open) | 부분 | 하 | 멀티윈도우 종속 | P1 |

---

## 4. 갭 표 — 검색

| 기능 | TAIDE 상태 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|
| 전역 검색(정규식·대소문자·단어·include/exclude·스트리밍·취소) | 있음, 동급 | - | - | - |
| 전역 바꾸기(원자적·바이너리 스킵) | 있음 | - | - | - |
| **Search Editor (결과를 에디터 탭으로, 컨텍스트 줄, 결과 내 직접 편집)** | 없음 | 중 | 중 (탭 타입 추가) | P1 |
| **검색 히스토리 / 최근 검색어** | **미확인** (grep 미실시) | 중 | 하 | P1 |
| **검색 결과에서 개별 항목 제외(dismiss)** | 미확인 | 하 | 하 | P2 |
| `.gitignore` 존중 토글 | 미확인 (`search/types.rs` 는 include/exclude glob 만 확인됨) | 중 | 하 | P1 |

---

## 5. 갭 표 — Git

| 기능 | TAIDE 상태 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|
| status·stage/unstage/discard·commit·push/pull/fetch·브랜치·stash·graph·split diff·gutter·inline blame·hunk discard | 있음 (VS Code + GitLens + Git Graph 조합에 근접) | - | - | - |
| **3-way 병합 충돌 해결 에디터** | **없음** (merge 그룹 표시만) | **상** (충돌 나면 TAIDE 밖으로 나가야 함 = dogfooding 차단) | 상 (3-way 뷰 자작, Monaco DiffEditor 로 부족) | **P0** (backlog 기재) |
| **줄/선택 단위 stage (stage selected lines)** | 부분 — hunk **discard** 만 있음, hunk **stage** 없음 | 상 | 중 (`git_discard_hunk` 대칭 커맨드 + patch apply --cached) | **P0** |
| **커밋 상세 뷰**(커밋 클릭 → 변경 파일 목록 → diff) | 없음 (그래프는 있으나 드릴다운 없음) | 상 | 중 | **P0** |
| **파일 전체 blame 뷰** | 없음 (커서 줄 인라인만) | 중 | 중 (`git_blame_range` 재사용) | P1 |
| **revert commit · rebase · cherry-pick · tag · worktree · submodule** | 없음 | 중 (revert/tag 는 중, rebase 는 하) | 중 (커맨드별 증분) | P1 (revert·tag) / P2 (rebase·worktree) |
| **원격 브랜치 checkout / 브랜치 rename·merge GUI** | 미확인 (`git_branches`·`git_checkout` 은 있음, 원격 추적 브랜치 처리 미확인) | 중 | 하 | P1 |
| **PR/이슈 통합(GitHub)** | 없음 | 하 | 상 | P2 — 범위 밖 권장 |
| AI 커밋 메시지 생성 | 없음 (VS Code·Cursor 는 있음) | 중 | **하** — provider 3종(`domain/ai/providers/`) + `git_diff` 재사용이면 얹기 쉬움 | P1 — **구현 완료**(Wave G, 2026-08-16 — `ai_commit_message`+신규 `git_diff_staged_text`(**주의**: 정찰 당시 "`git_diff` 재사용" 전제는 오류로 판명 — 통합 diff 커맨드가 당시 없어 신설했다, 계약 §2-2), `features/ai.md` §4/§7). 항목 종결 |

---

## 6. 갭 표 — 터미널 · 태스크 · 실행

| 기능 | TAIDE 상태 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|
| pty 소유·세션 유지·flow control·분할·프로필·검색·링크·폰트·ANSI 테마 | 있음, VS Code 동급 | - | - | - |
| **Shell Integration (OSC 133): 명령 데코레이션·명령 간 이동(⌘↑/↓)·종료 코드·스크롤바 표시** | **없음** (roadmap Phase 8 미착수) | **상** (warp 참조 제품의 핵심) | 중 (xterm OSC 파서 + 셸 rc 주입) | **P0** |
| 터미널 sticky scroll(현재 명령 고정) | 없음 | 중 | 중 (OSC133 선행) | P1 |
| **Run Selected Text in Terminal** | 없음 | 중 | **하** | P1 |
| **터미널을 에디터 영역 탭으로** | 있음 (탭 타입에 터미널 존재) | - | - | - |
| **태스크 시스템**: npm scripts/Cargo/Makefile 자동 감지, tasks.json, problem matcher(출력→Problems 패널), build/test 태스크, watch 태스크 | **없음** | **상** | 중~상 (감지+실행은 중, problem matcher 는 상) | **P0** (감지·실행만 P0, matcher 는 P1) — backlog 기재 |
| **디버거(DAP)** | 없음 | 상(언어에 따라) | **최상** | **비추천 — PRD §2 명시 비목표** |
| **테스트 러너/탐색기** | 없음 | 중 | 상 | P2 |
| Notebook(.ipynb) 편집 | 없음 (IDE MCP `executeCode` 도 not supported) | 하 | 상 | 비추천 — 범위 밖 |

---

## 7. 갭 표 — 설정 · 확장 · 동기화

| 기능 | TAIDE 상태 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|
| 설정 UI 12섹션·키맵 편집·폰트·i18n·테마 에디터 | 있음 (VS Code 보다 GUI 완성도 높음) | - | - | - |
| **settings.json 을 에디터 탭에서 직접 편집** | 없음 (Finder 열기까지) | 중 | 중 (root_guard + 탭 projectId 종속 해제) | P1 (backlog 기재) |
| **키맵 chord (⌘K ⌘S) · when 컨텍스트** | **있음 — Wave H 구현 완료**(⌘K 프리픽스·2단 무조건 삼킴·전역 chord 스토어·monaco 유예 창으로 기존 monaco chord 21건 보존·when 기반 컨텍스트 게이팅·컨텍스트 인스펙터. `docs/features/keymap.md` 참조) | - | - | - |
| shift+기호 키 캡처(`event.code` 기반) | **있음 — 이 행 자체가 stale 이었다**: `normalizeKeymapEventKey`(기능확장 2차 k, 커밋 23c3fb2)가 이미 기호를 `event.code` 기반으로 유도해 레이아웃 독립적이다. Wave H 에서 실물 재검증 + 회귀 테스트로 확정(`docs/acknowledge/2026-08-16-wave-h-keymap-contract.md` §2-1) | - | - | - |
| **Profiles(설정 프로파일 전환·워크스페이스 연결·템플릿)** | 없음 | 하 (단일 사용자) | 상 | P2 |
| Settings Sync | 있음 (Gist 기반 자체 구현, 실기 미검증) | - | - | - |
| **확장 마켓플레이스 / 플러그인 설치 UI** | 없음 (앱데이터 폴더 수동 배치) | 중 | 중 (레지스트리 정책 결정 선행) | P1 — MS Marketplace 연동은 금지 방침 유지 |
| VSIX grammar/languages 임포트 | 없음 (테마만) | 중 | 중 | P1 (backlog 기재) |

---

## 8. 갭 표 — Cursor 축 (AI)

TAIDE 의 AI 전략은 "터미널 에이전트 연동 우선, 자체 AI 는 추후"(PRD §2). 그래서 **대부분은 의도된 비목표**지만, 사용자가 체감하는 격차는 실재하므로 구분해 적는다.

| Cursor 기능 | TAIDE 상태 | 체감 | 난이도 | 우선순위 |
|---|---|---|---|---|
| **Tab (다중 라인·import 자동 추가·다음 편집 위치 점프·크로스파일 예측)** | **부분** — FIM 인라인 완성은 있음(`ai_inline_complete`, `code-editor.tsx:190-196`), 단 **단일 위치 제안**. jump-to-next-edit·크로스파일 예측 없음. 실기 미검증 | 중 | 상 (모델 품질 종속) | P2 |
| **Inline Edit (⌘K)** — 선택 영역 자연어 편집 → diff 수락/거절 | **없음** | **상** | **중** — provider 3종·프롬프트 파일화(`domain/ai/prompt.rs`)·인라인 완성 인프라가 이미 있어 재사용 가능 | **P1 (AI 축 최우선)** — **구현 완료**(Wave G, 2026-08-16 — 기본 키는 **⌘I**로 채택(⌘K 는 모나코 chord 21건이 이미 점유해 보류, 계약 §4), 모델 무변경 프리뷰(삭제 데코+ViewZone)·`taide.aiInlineEdit`·`features/ai.md` §3). 항목 종결 |
| Agent / Composer (에이전트 채팅 패널, 체크포인트, 큐잉, 도구 호출) | 없음 — 대신 **터미널 에이전트 + IDE MCP 서버 내장** | 상 | 최상 | 비추천 (PRD 비목표) — 단 Codex app-server 패널은 backlog 기재 |
| Rules (`.cursor/rules`, AGENTS.md, 중첩·글롭) | 없음 (파일 규약이라 에디터가 관여 안 해도 에이전트가 읽음) | 하 | 하 | P2 |
| Codebase indexing / semantic search / Instant Grep | 없음 (리터럴·정규식 검색만) | 중 | 상 (임베딩 인프라) | P2 |
| @-컨텍스트 심볼(파일·폴더·심볼·docs·git) | 없음 | 중 | 상 (채팅 UI 선행) | 비추천 |
| Background / Cloud Agent, Bugbot, Slack·Linear 통합, iPad 앱 | 없음 | 하 (개인용) | 최상 | 비추천 — 범위 밖 |
| AI 커밋 메시지 / 코드 리뷰 | 없음 | 중 | 하~중 | P1 (§5 참조) — **커밋 메시지 구현 완료**(Wave G, §5 참조. 코드 리뷰는 범위 밖으로 유지). 커밋 메시지 항목 종결 |

---

## 9. P0 요약 (일상 루프에서 매일 막히는 것, 10건)

| # | 항목 | 한 줄 근거 | 난이도 |
|---|---|---|---|
| 1 | **LSP initialize capabilities 확충** (`service.rs:305-317`) | 아래 항목 전부의 선행 조건이자, 현재 documentSymbol/inlayHint/signatureHelp 가 미선언 상태로 요청되는 잠재 결함 | 하~중 |
| 2 | **Code Action / Quick Fix (⌘.) + Code Actions on Save** | 진단은 표시되는데 고칠 수단이 앱 안에 없음 — 단일 최대 공백 | 중 |
| 3 | **팔레트 `@`(심볼) · `:`(줄) 모드 + Workspace Symbol(⌘T)** | `command-registry.ts:49-52` 확장. documentSymbol 어댑터 이미 있음 | 중 |
| 4 | **Peek Definition / Peek References 배선 확인·노출** | Monaco 내장 액션이라 이미 동작 중일 수 있음 — **먼저 실기 확인**(미확인) | 하 |
| 5 | **Document Highlight + Selection Range** | 각각 어댑터 1파일. 체감 대비 최저 비용 | 하 |
| 6 | **Semantic Tokens** | TextMate(W7)만으로는 타입/변수/파라미터 구분 불가. 테마 파생 경로(`build-shiki-theme.ts`)와 연결 | 중 |
| 7 | **Sticky Scroll + Breadcrumbs + 커서 위치(줄:열) 상태바** | sticky/커서위치는 옵션·이벤트 수준, breadcrumbs 만 UI 신설 | 하~중 |
| 8 | **Git: 3-way 충돌 해결 UI + 줄 단위 stage + 커밋 상세 뷰 + 파일 히스토리** | 충돌 시 TAIDE 밖으로 나가야 함 → 성공 기준 1(dogfooding) 직접 위협 | 중~상 |
| 9 | **터미널 Shell Integration (OSC 133)** — 명령 블록·명령 간 이동·종료 코드 | warp 를 참조 제품으로 삼은 PRD 대비 가장 큰 미달 | 중 |
| 10 | ~~태스크 러너 (npm/Cargo/Makefile 감지 실행) + 키맵 chord(⌘K ⌘S)~~ | 태스크 러너는 Wave E, chord 는 Wave H 에서 각각 구현 완료 — 이 행은 해소됨 | - |

보너스 P0 후보(11): **사용자 정의 스니펫** — backlog 기재, LSP completion 과 우선순위 조정 필요.

## 10. P1 요약 (10건)

1. **AI Inline Edit (⌘K)** — 기존 provider·프롬프트 인프라 재사용, Cursor 격차 중 가장 비용 대비 효과 큼
2. **Go to Implementation / Type Definition / Declaration** (definition 어댑터 복제 3건)
3. **Code Lens + LSP Folding Range**
4. **에러/경고 순회(F8·⇧F8)** — Monaco 내장 액션 키맵 등록
5. **Search Editor(검색 결과 탭·결과 내 편집) + 검색 히스토리 + `.gitignore` 존중 토글**
6. **git revert commit · tag · 원격 브랜치 checkout + 파일 전체 blame 뷰**
7. **AI 커밋 메시지 생성** (`git_diff` + 기존 provider)
8. **Zen/포커스 모드 + 멀티 윈도우**(backlog) — 멀티 윈도우는 탭 메뉴 4항목의 전제
9. **Emmet** (`emmet-monaco-es` 의존성 추가 승인 필요) + Format on Type/Paste
10. **설정 파일 에디터 탭 편집 + 플러그인 설치 UI + VSIX grammar 임포트**(전부 backlog 기재)

## 11. 비추천 (범위 밖 — 착수 권고하지 않음)

디버거(DAP, PRD 비목표) · Notebook 편집 · 확장 생태계 호환(VSIX 실행) · SSH 원격 개발 · 협업 편집(Live Share) · Cursor Cloud/Background Agent·Bugbot·Slack/Linear 통합 · Profiles · Call/Type Hierarchy(Monaco API 부재로 비용 급증) · PR/이슈 통합.

---

## 12. TAIDE 가 오히려 앞서는 것

- **상태 소유 모델**: 프로젝트·탭·스플릿·pty·LSP 세션이 Rust 소유라 view reload 로 아무것도 잃지 않는다. VS Code 는 렌더러 크래시 시 터미널 세션이 사라진다. 여기에 dirty 미러(`file/service.rs:187`)까지 있어 Hot Exit 동등 보호도 갖춤.
- **멀티 프로젝트 사이드바 + focus kind 아이콘**: VS Code 는 창 단위(멀티 윈도우) 또는 멀티루트로만 푼다. 프로젝트별 에이전트 실행 상태가 아이콘에 뜨는 건 VS Code·Cursor 둘 다 없다.
- **테마 시스템**: 앱 UI + Monaco 구문색 + 터미널 ANSI 16색이 **하나의 정의에서 파생**되고, 테마 에디터·VSIX 임포트가 내장. VS Code 는 에디터 테마와 터미널 색이 별개 설정이고 테마 편집 GUI 가 없다.
- **LSP 원클릭 설치**: 19종 매니페스트 + 진행률·취소가 앱 코어에 내장. VS Code 는 언어별 확장 설치 + 각 확장의 자체 다운로드에 의존.
- **AI 에이전트 연동이 코어**: Claude Code hooks 설치/제거 + IDE MCP 서버(툴 12종) + `taide --wait` CLI 가 **확장 없이** 앱에 들어 있다. VS Code 는 확장 설치가 전제.
- **미리보기 폭**: PDF·스프레드시트(xlsx/csv)·HWP/HWPX·pptx 개요·비디오/오디오까지 내장. VS Code 는 전부 확장.
- **remote-control**(웹 미러링, 디스패치 137종) — VS Code 는 vscode.dev/tunnel 이 대응하지만 자체 서버 내장은 다른 축.
- **터미널 flow control + ring buffer 복원**: 대량 출력 시 UI 정지 방지와 attach 복원이 코어 설계에 들어감.
- **설정 UI 의 GUI 완성도**: 키맵 모달·폰트 피커·언어팩 `extends`·toast 위치 9분할 등은 VS Code 가 JSON 편집을 요구하는 영역이다.

---

## 13. 미확인 (검증 필요 — "없음"과 구분할 것)

- **Peek Definition/References 실동작** — Monaco 내장 액션 + provider 등록 조합이라 이미 될 가능성이 높음. 실기 확인 전에는 "미확인".
- **Monaco 내장 `⌃G`(Go to Line)·`⇧⌘O`(Quick Outline)** 이 에디터 포커스에서 통하는지 — full ESM 임포트라 등록은 되어 있으나 TAIDE 키맵과의 충돌 여부 미확인.
- **검색 히스토리 · `.gitignore` 존중 토글** — `search/types.rs` 전문을 열지 않아 미확인.
- **원격 브랜치 checkout / 브랜치 merge** 지원 범위 — 커맨드 이름만 확인.
- **Sync 페이로드에 키맵·플러그인 포함 여부** (`sync/types.rs:25`).
- **Cursor 측**: 공식 문서에서 확인 못 한 것 — Agent 의 모드 분화(Ask/Plan)·todos·diff 리뷰 UI 세부는 fetch 한 overview 페이지에 없어 **미확인**. `.cursorignore` 문법과 `@` 심볼 전체 목록도 인덱싱 문서 페이지에 없어 **미확인**. Cursor 2026-08 최신은 Google Workspace 플러그인·iPad·Cursor Router 등 **에디터 편집 기능이 아닌 방향**으로 이동 중.
- TAIDE W1~W7 전량 실기 미검증(`docs/quality-assurance/2026-08-11-qa6-checklist.md`) — 특히 shiki CSP 런타임, 원격 실접속, LSP 다운로드 Gatekeeper.

## 14. backlog 중복 항목 (신규 생성 금지, 참조만)

멀티 윈도우 · Zen/포커스 모드 · 스니펫 · git 충돌 해결 UI · 북마크 · 접근성 · 앱데이터 파일 에디터 편집 · VSIX grammars 임포트 · Gemini IDE companion · Codex app-server 패널.
**backlog 정정 필요 2건**: "임의 두 파일 비교"는 이미 구현됨(`src/widgets/explorer/explorer-container.tsx:254`). "작업 러너 패널"·"chord/when 키맵"·"shift+기호 키 캡처"는 각각 Wave E·Wave H(chord/when)·Wave H(shift+기호 회귀 확정)에서 이미 구현/확정되어 이 목록(신규 생성 금지 대상)에서 제외한다.

---

## 출처

VS Code (code.visualstudio.com)
- https://code.visualstudio.com/docs/editing/codebasics
- https://code.visualstudio.com/docs/editing/intellisense
- https://code.visualstudio.com/docs/editing/refactoring
- https://code.visualstudio.com/docs/editing/editingevolved
- https://code.visualstudio.com/docs/editing/userdefinedsnippets
- https://code.visualstudio.com/docs/editing/workspaces/multi-root-workspaces
- https://code.visualstudio.com/docs/sourcecontrol/overview
- https://code.visualstudio.com/docs/terminal/basics
- https://code.visualstudio.com/docs/debugtest/tasks
- https://code.visualstudio.com/docs/getstarted/userinterface
- https://code.visualstudio.com/docs/getstarted/tips-and-tricks
- https://code.visualstudio.com/docs/configure/settings-sync
- https://code.visualstudio.com/docs/configure/profiles
- https://code.visualstudio.com/docs/configure/accessibility/accessibility

Cursor (cursor.com)
- https://cursor.com/docs
- https://cursor.com/docs/tab/overview
- https://cursor.com/docs/agent/overview
- https://cursor.com/docs/context/rules
- https://cursor.com/docs/context/codebase-indexing
- https://cursor.com/changelog

TAIDE 코드 근거 (절대경로)
- `/Users/hyunseokbyun/TAIDE/src/shared/lib/monaco/setup.ts:1`
- `/Users/hyunseokbyun/TAIDE/src/features/editor/code-editor.tsx:106-113,152,190-196`
- `/Users/hyunseokbyun/TAIDE/src/shared/lib/lsp/client.ts:39-55`
- `/Users/hyunseokbyun/TAIDE/src/shared/lib/lsp/adapters/` (11파일)
- `/Users/hyunseokbyun/TAIDE/src-tauri/src/domain/lsp/service.rs:288-322`
- `/Users/hyunseokbyun/TAIDE/src/shared/lib/command-registry.ts:49-52,133`
- `/Users/hyunseokbyun/TAIDE/src-tauri/src/domain/file/service.rs:187`
- `/Users/hyunseokbyun/TAIDE/src/widgets/editor-pane/editor-pane.tsx:125`
- `/Users/hyunseokbyun/TAIDE/src/widgets/explorer/explorer-container.tsx:254`
- `/Users/hyunseokbyun/TAIDE/node_modules/monaco-editor/monaco.d.ts` (0.56.0, provider API 목록)
- `/Users/hyunseokbyun/TAIDE/docs/backlog.md`, `/Users/hyunseokbyun/TAIDE/docs/PRD.md`