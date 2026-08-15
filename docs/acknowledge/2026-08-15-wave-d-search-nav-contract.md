# Wave D 구현 계약 — 탐색·검색 (2026-08-15)

> 정찰 wf_33479e2a-2e3(opus+high 3축: 팔레트·심볼 / 검색 도메인 / Breadcrumbs·LSP, 축0은 resume 성공).
> 캠페인 계약: 2026-08-14-remaining-features-pro-qa-plan.md(완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-15, 전부 추천안)

| # | 결정 | 선택 |
|---|------|------|
| 팔레트 모드 | 프리픽스·monaco 관계 | **VS Code 규약+병존** — @=문서 심볼·#=Workspace Symbol·:=줄 이동(:줄 또는 :줄:열). ⌘T=# 진입. monaco 내장 ⇧⌘O·⌃G 는 에디터 오버레이로 병존(이중 바인딩 회피 — 같은 키에 안 얹음) |
| ⑤ Search Editor | 배치 | **신규 TabKind::SearchEditor** — 영속·편집가능 에디터 표면(blame/history 오버레이와 다름). 직렬화·hot-exit·remote 파리티 비용 감수. 컨텍스트 줄 백엔드 확장·결과 내 편집 실파일 반영 |
| ⑥ gitignore | 구현 | **`ignore` 크레이트 도입**(ripgrep 표준·신규 의존성 승인). respect_gitignore: bool(기본 on) + 기존 IGNORED_DIR_NAMES 병행 |
| capability/히스토리 | 패키지 | **추천** — workspace.symbol + hierarchicalDocumentSymbolSupport 선언, 검색 히스토리 Settings recent_searches: Vec<String>(상한 20), Breadcrumbs 드롭다운=형제 심볼+경로 세그먼트 |

**신규 의존성 승인**: `ignore` 크레이트(gitignore 인식 walker) — 승인 목록에 추가.

## 2. 확정 사실 (정찰·메인 확인)

- 팔레트 parsePaletteQuery(command-registry.ts:61-64)는 `>`(commands)/기본(files) 2모드. PaletteMode union 확장점 단일 함수.
- ① @심볼·② :줄은 순수 프론트 — requestDocumentSymbols(document-symbol 어댑터, outline-panel 재사용)·requestReveal(reveal-registry) 재사용, 백엔드 0.
- ③ Workspace Symbol 완전 미구현 — protocol ServerCapabilities.workspaceSymbolProvider 부재·client.ts FEATURE_CAPABILITY_CHECKS 게이트 부재·buildInitializeParams workspace.symbol 미선언·어댑터 없음. 전송은 기존 lspSend(신규 Tauri 커맨드 불필요). SymbolInformation 타입 존재.
- ④ Breadcrumbs 전무(monaco 0.56 standalone breadcrumb API 부재 — 자작 확정). documentSymbol·커서 위치(status-bar) 재사용.
- ⑤ 검색: SearchQuery 6필드(text·case·word·regex·include/exclude glob), gitignore·context·history 필드 없음. SearchMatch.preview 단일 줄(컨텍스트 미지원). search_run 스트리밍 Channel·취소(AtomicBool)·begin_mutation. gitignore 파싱 0(하드코딩 IGNORED_DIR_NAMES 만). TabKind 에 Search 없음(File/Terminal/Settings/Diff/ClaudeDiff/Welcome/Untitled). excludeGlob 은 백엔드 지원하나 프론트 항상 null.
- LSP capability 는 프론트 buildInitializeParams 단독 저작(Rust lsp 도메인 무접촉 — 갭 문서 fact#3 정정). documentSymbol:{} 에 hierarchicalDocumentSymbolSupport 없음.
- ⌘T(mod+t) 미할당(reopen-closed-tab 은 ⇧⌘T). ⇧⌘O·⌃G 는 monaco 자체 바인딩.

## 3. 확정 설계

### 3.1 팔레트 모드 (프론트 — command-registry·command-palette)

- parsePaletteQuery 확장: PaletteMode union 에 `symbol`(@)·`line`(:)·`workspaceSymbol`(#) 추가. prefix 분기.
- ① @: 활성 파일 uri → requestDocumentSymbols(waitForLspSession·layout·lspServers 컨텍스트 — outline-panel 패턴) → 심볼 나열, 선택 시 requestReveal(path, line, col). DocumentSymbol/SymbolInformation 두 형태 모두 정규화(hierarchical capability 선언해도 서버별 편차 방어).
- ② :: `:123` 또는 `:123:45` 파싱(숫자) → 활성 파일 requestReveal. 활성 파일 없으면 무시(빈 상태 UI).
- ③ #: ⌘T 진입(KeymapActionId union+APP_KEYMAP+CommandPalette useGlobalKeymap 3곳 배선). workspace/symbol 어댑터(§3.2) 호출 → 결과 나열, 선택 시 openTab+requestReveal(cross-file).
- 각 모드 로딩/빈/에러 UI 명시(command-palette.tsx 가 현재 files/commands 만 상정). monaco ⇧⌘O·⌃G 병존(리바인딩 안 함).

### 3.2 Workspace Symbol LSP (프론트 — protocol·client·lsp-session-registry·어댑터)

- protocol.ts: ServerCapabilities.workspaceSymbolProvider(boolean | { resolveProvider? }) + WorkspaceSymbol 타입(SymbolInformation 재사용 가능하면 그대로).
- client.ts FEATURE_CAPABILITY_CHECKS: `workspace/symbol` 게이트 추가(미지원 서버 조용한 실패 방지).
- buildInitializeParams: workspace.symbol:{ symbolKind.valueSet } + textDocument.documentSymbol.hierarchicalDocumentSymbolSupport:true(+symbolKind.valueSet). (③④ 정확도 선행)
- adapters/workspace-symbol.ts 신설: client.request('workspace/symbol', { query }) → SymbolInformation[] 정규화. 다중 세션(멀티 서버) 결과 병합·디바운스. monaco provider 아님(팔레트 직접 소비 — outline 패턴).

### 3.3 Breadcrumbs (프론트 — editor-pane 헤더 신규)

- 에디터 pane 헤더(markdown 토글 바 선례 위치)에 경로 세그먼트(relative-path 재사용) + 심볼 계층(requestDocumentSymbols → 커서 위치 enclosing symbol 계산). 커서 구독은 별도 getPosition(code-editor 커서 콜백은 line-only 유지). 세그먼트 클릭 드롭다운: 경로=폴더 형제, 심볼=형제 심볼 → requestReveal. sticky scroll 과 시각 관계 정리.

### 3.4 Search Editor (신규 TabKind — Rust layout+search, 프론트)

- **Rust layout**: TabKind::SearchEditor 변형 추가(직렬화·세션 복원·hot-exit 대상 판단·remote 파리티). 검색 쿼리/결과 상태를 탭에 담는 스키마. lib.rs·dispatch·bindings·pane-node-view 라우팅·FocusKind.
- **Rust search**: SearchQuery 에 context_lines: u32(기본 상수)·respect_gitignore: bool(기본 true) 추가. SearchMatch 에 before/after 줄 배열. service 를 `ignore` 크레이트 walker 로 전환(respect_gitignore=true 시 .gitignore 인식, IGNORED_DIR_NAMES 병행). 신규 커맨드 발생 시 배선 3곳.
- **프론트**: SearchEditor 탭 위젯 — 결과 나열(컨텍스트 줄 포함)·클릭 시 requestReveal·결과 내 편집(실제 파일 열어 반영 또는 in-place → 파일 반영). 기존 search-panel 과 공존(패널=빠른 검색, 에디터=영속).

### 3.5 검색 히스토리 + gitignore 토글 UI

- Settings recent_searches: Vec<String>(상한 20, remote_allowed_hosts 선례 — types/service/sync/emptySettingsPatch/bindings). 검색 실행 시 prepend·중복 제거·상한. sync 동기화 대상(비밀 아님).
- 검색 패널·Search Editor 에 gitignore 존중 토글(respect_gitignore) + excludeGlob 입력(현재 항상 null — UI 배선) + 히스토리 드롭다운.
- locale 4곳 신규 키(팔레트 모드 라벨·workspace symbol·breadcrumbs·search editor·gitignore 토글·히스토리) + en⊆required.

### 3.6 실행 구조

- **Phase A 백엔드(sonnet+xhigh, Rust 단독)**: TabKind::SearchEditor + SearchQuery 확장(context_lines·respect_gitignore)·SearchMatch 컨텍스트 줄 + `ignore` 크레이트 walker 전환 + Settings recent_searches + locale + 배선 3곳 + Cargo `ignore` 추가 + 파리티 + bindings. cargo fmt/clippy/test 그린. exports 로 타입·커맨드·locale 전달.
- **Phase B 프론트 병렬 3(sonnet+xhigh, 파일 소유 분리)**: B1=팔레트 @/:/#  모드+workspace-symbol 어댑터+capability 확충(command-registry·command-palette·lsp protocol/client/session-registry·keymap) / B2=Breadcrumbs(editor-pane 헤더) / B3=Search Editor 탭 위젯+검색 히스토리·gitignore 토글 UI(search-panel·신규 search-editor 위젯·entities/search). protocol/client/session-registry 는 B1 단독 소유(capability), B2·B3 은 사용만.
- **Phase C 검토**: 4렌즈(계약·정확성·보안(ignore walker 경로·search editor 편집 파일 반영·workspace symbol 원격)·설계, opus+xhigh) → 적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차 → 커밋.

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| monaco ⇧⌘O·⌃G 를 팔레트 @/: 로 재바인딩 | 기각 — 이중 바인딩·monaco 억제 복잡. 병존 채택 |
| Search Editor 경량 배치(bridge+패널) | 기각 — 영속·편집 표면이라 TabKind 채택 |
| gitignore 수동 파서 | 기각 — 문법 복잡도·정확도. ignore 크레이트 채택 |
| 검색 히스토리 프론트 메모리(비영속) | 기각 — Settings 영속 채택 |
| workspaceSymbol/resolve 1차 구현 | 보류 — lazy location 서버 대비, 필요 시 후속 |
| 파일 히스토리 rename 추적 | (Wave C 보류 유지) |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드·TabKind 반영). `ignore` 크레이트 Cargo.lock 반영.
- 4렌즈+적대적 검증+메인 2차 통과. 초점: ignore walker 의 기존 IGNORED_DIR_NAMES 병행 정확성·검색 결과 집합 변화, Search Editor 결과 편집의 실파일 반영·hot-exit 상호작용, workspace/symbol 게이트·다중 세션 병합, TabKind 세션 복원·remote 파리티, 팔레트 빈 상태 UI.
- 문서: features(검색·에디터)·ipc-contract(SearchQuery·TabKind)·data-model(recent_searches)·tech-stack(ignore 크레이트)·qa6-checklist Wave D 항목. 갭 §3·§4 항목 종결.
