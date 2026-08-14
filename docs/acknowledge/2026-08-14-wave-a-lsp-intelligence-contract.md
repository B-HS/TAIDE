# Wave A 구현 계약 — LSP 인텔리전스 (2026-08-14)

> 정찰 wf_0d6ea4d6-7b4(opus+high 4축: Rust 코어·프론트 어댑터·monaco 0.56 실물·LSP 3.17 스펙+서버 3종).
> 하중 주장 8건(dead code·요청 폐기·오프너 부재·BulkEditService 예외·organizeImports run 비대기·
> 카탈로그 기노출·capability 타입 공백·buildInitializeParams 정본)은 메인이 grep·monaco 소스로 재검증 완료.
> 캠페인 계약: `2026-08-14-remaining-features-pro-qa-plan.md` (완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-14, 전부 추천안 채택)

| # | 결정 | 선택 |
|---|------|------|
| 1 | Wave A 범위 | **풀 패키지** — 선행 기반(서버→클라 요청 라우팅·cross-file 오프너·initialize 정본화) + Code Action 전체 + on-save + Impl/TypeDef/Decl + CodeLens 축소안 + LSP Folding + Peek/F8 배선 |
| 2 | on-save 설정 | **boolean 2종** — `organize_imports_on_save`/`fix_all_on_save`(기본 false), 명시적 저장(⌘S)에만 적용·autoSave 제외, 상한 초과 시 스킵+토스트 |
| 3 | 기존 결함 | **전부 상환** — dead `initialize_params` 삭제·configuration 무응답·didSave 미전송·initializationOptions 미전달 복원·rename 미열린 파일 한계(적용기 통일)·lsp_send 전역 락 해제·CancellationToken 13종 소급 |
| 4 | WorkspaceEdit | **전체 지원** — documentChanges + resourceOperations(create/rename/delete) 선언·순서 준수 적용, 미열린 파일은 Rust file IPC 직접 기록 |

## 2. 확정 사실 (메인 재검증 완료 — 근거는 실물)

1. **Rust `initialize_params`(service.rs:288)는 dead code** — 호출부는 자기 테스트 3곳뿐. 실제 initialize 는
   프론트 `lsp-session-registry.ts:59 buildInitializeParams` 가 전송. 2026-08-13 확충(P0-0)은 런타임 무효였음.
   매니페스트 `initializationOptions`(rust-analyzer 등 2종)도 프론트 미전달 — `LspServerDetection` 에 필드 자체가 없음.
2. **client.ts:105 handleMessage 는 응답·알림만 분기** — 서버→클라 요청(workspace/applyEdit·configuration·
   client/registerCapability·codeLens/refresh)은 무응답 폐기. `isJsonRpcRequest` 는 존재하나 테스트에서만 사용.
   `workspace.configuration: true` 를 선언 중이므로 현재도 잠재 결함.
3. **`registerEditorOpener` 미등록(grep 0)** — monaco standalone `findModel` 이 타 파일 uri 를 거부해
   cross-file F12·Peek 클릭·F8(InFiles)이 현재도 무음 실패. Peek 내부 미리보기는 `StandaloneTextModelService`
   가 미생성 모델을 reject — 오프너만으로 해결 안 됨(모델 선생성 필요).
4. **StandaloneBulkEditService 는 텍스트 편집 외·미열린 모델에 throw**(standaloneServices.js:553·557),
   저장도 안 함 — rename 의 기존 한계이며 Code Action edit 도 동일.
5. **`organizeImports`/`fixAll` 의 run() 은 적용 완료를 기다리지 않음**(triggerCodeActionsForEditorSelection
   → manualTriggerAtCurrentPosition 모두 return 없음 — 소스 확인). monaco 액션으로 on-save 순서 제어 불가.
   `codeActionsOnSaveTimeout` 옵션은 d.ts 만 있고 런타임 미소비.
6. **커맨드·키맵 노출은 기완료** — quickFix(⌘.)·autoFix·fixAll·organizeImports·refactor·sourceAction·
   peek 5종·goTo 계열·marker 4종(F8)·folding 전 계열·codelens 액션이 monaco-actions 카탈로그에 존재.
   Action2 계열은 MONACO_ACTION2_IDS 로 상시 활성, EditorAction 계열(organizeImports 등)은
   `providedCodeActionKinds` 기반 precondition — **provider metadata 생략 시 영구 비활성**.
7. **capability 확장 지점**: `protocol.ts:156 ServerCapabilities`(codeAction·codeLens·foldingRange·
   implementation·typeDefinition·declaration·executeCommand 필드 부재) + `client.ts:39
   FEATURE_CAPABILITY_CHECKS`(미등록 메서드는 무검사 통과) + buildInitializeParams. 어댑터 등록은
   `LANGUAGE_ADAPTER_REGISTRARS` 배열 + `subscribeLanguageAdapterRegistration` 재계산 경로 기존재.
8. **Rust 는 무검증 pass-through**(`lsp_send` 가 문자열 그대로 프레이밍) — 신규 LSP 메서드에 Rust 변경 0.
   단 `lsp_send` 가 전역 mutation 뮤텍스를 잡아(commands.rs:404) 고빈도 요청이 저장·git 과 직렬화됨.
   세션 stdin 은 별도 mutex 로 이미 직렬화(lsp_proc.rs) — 전역 락 해제 대상.
9. **서버별 특이사항(스펙 정찰)**: codeActionLiteralSupport 미선언 시 rust-analyzer 는 무응답,
   resourceOperations 미선언 시 ra 파일 생성형 액션이 요청 단위 실패. resolveSupport ['edit'] 선언 시
   gopls 는 resolve 로 edit 지연 제공. tsls 는 only 미지정 시 organizeImports 계열 미계산.
   진단 원본(code·data)을 marker 역변환으로 만들면 gopls/tsls quickfix 대부분 유실 — 원본 보관 필수.
   edit 적용 성공 후에만 command 실행(스펙 명문). kind 매칭은 '.' 계층 접두사.

## 3. 확정 설계

### 3.1 선행 기반 (모든 축의 전제)

- **initialize SSOT = 프론트** `buildInitializeParams` 확장(정본). 추가 선언: textDocument.codeAction
  { codeActionLiteralSupport.codeActionKind.valueSet(전 kind), isPreferredSupport, disabledSupport,
  dataSupport, resolveSupport: { properties: ['edit'] } } · codeLens · foldingRange { lineFoldingOnly:
  true, rangeLimit } · implementation/typeDefinition/declaration { linkSupport: true } (definition 도
  linkSupport 정합) · synchronization.didSave 유지 + **didSave 실전송**(저장 성공 시 notification) ·
  workspace { applyEdit: true, executeCommand: {}, workspaceEdit: { documentChanges: true,
  resourceOperations: ['create','rename','delete'], failureHandling: 'textOnlyTransactional' },
  codeLens.refreshSupport: true }. **기준 3.17 고정**(3.18 초안·ra 실험 확장 snippetTextEdit/
  codeActionGroup 미선언 — 파서 미지원 선언은 시각 버그 위험).
- **Rust `initialize_params` + 전용 테스트 삭제**(dead code). `LspServerDetection` 에
  `initialization_options: Option<serde_json::Value>` 추가(매니페스트 값 전달) → 프론트가 initialize 에 포함.
- **서버→클라 요청 라우팅**: client.ts handleMessage 에 `isJsonRpcRequest` 분기 + 핸들러 맵.
  처리: workspace/applyEdit(적용기 실행 → `{ applied, failureReason? }` 실결과), workspace/configuration
  (**null 채움 배열 기본 응답** — 서버별 설정 매핑은 backlog), client/registerCapability(빈 결과),
  workspace/codeLens/refresh·inlayHint/refresh(빈 결과 + provider onDidChange 발화),
  window/workDoneProgress/create(빈 결과). 미지원 메서드는 **-32601 MethodNotFound 응답**.
  세션 미준비 구간(sessionId null)의 응답 유실 방지 확인.
- **cross-file 오프너**: `monaco.editor.registerEditorOpener` 1회 등록(부트스트랩, bridge 패턴 —
  problems 패널 선례 `requestReveal(path,line,col)` + `openTab(preview)` 재사용, untitled scheme 분기).
- **lsp_send 전역 mutation 락 해제**(세션 stdin mutex 로 충분 — 검토 렌즈 집중 검증 항목).

### 3.2 Code Action / Quick Fix + on-save

- 진단 **원본 사이드 맵**: diagnostics 어댑터가 publishDiagnostics 원본(Diagnostic[] — code·data·source
  보존)을 uri 키로 보관, codeAction 요청 시 range 교차분만 context.diagnostics 로 전달.
- codeAction 어댑터: `registerCodeActionProvider(lang, provider, { providedCodeActionKinds })` —
  kinds 는 서버 광고값, boolean-true 서버는 `['quickfix','refactor','source']` 폴백. resolve 지원
  (resolveProvider 시). 목록은 didChange 시 stale 폐기(ra version 검증 대응). CodeActionList 는
  dispose 포함. ⌘. 은 only 미지정, 메뉴·on-save 는 kind only 지정. kind 계층 접두사 매칭 유틸 신설.
- **WorkspaceEdit 자체 적용기** 신설(shared/lib/lsp): changes + documentChanges(TextDocumentEdit·
  CreateFile·RenameFile·DeleteFile) **배열 순서대로** 적용. 열린 모델 = pushEditOperations(열린 탭
  dirty/미러는 기존 onDidChangeContent 경로 — 언마운트 탭 정합은 검토 항목), 미열린 파일 = Rust file
  IPC 읽기→적용→기록. 파일 생성/이름변경/삭제는 기존 file 도메인 IPC 재사용. **rename 어댑터도 이
  적용기로 통일**(기존 한계 해소). 실패는 all-or-nothing 지향·실결과 반환.
- command 중계: 서버 `executeCommandProvider.commands` 를 세션 초기화 시 `monaco.editor.registerCommand`
  로 일괄 등록(세션 dispose 시 해제, 서버 간 중복 id 는 마지막 등록 우선 + 세션 라우팅) → 핸들러는
  `workspace/executeCommand` 중계. **edit 적용 성공 후에만 command 실행**. 클라이언트 커맨드
  `editor.action.showReferences(uri, position, locations)` VS Code 시그니처로 구현(peek 위젯 열기).
  화이트리스트 차단 금지(docs/features/lsp.md 규정) — 방어는 기존 원격 게이트 층(보안 렌즈 확인).
- **Code Actions on Save**: editor-pane handleSave 의 formatOnSave 블록 직전에 LSP 직접 호출 조립 —
  kind 별 별도 요청(`source.fixAll` 먼저 → `source.organizeImports`, only+triggerKind Automatic) →
  적용기 → format → save. 명시적 저장만(autoSave 경로 제외), 재무장 가드(autoSave/mirror 타이머)
  확인. 상한 `CODE_ACTIONS_ON_SAVE_TIMEOUT_MS` 상수 — 초과 시 스킵+토스트. untitled 저장 경로는 비대상.

### 3.3 이동 3종 · Peek · F8

- definition 어댑터를 **파라미터화 공통화**(4회 사용 — '2회 이상' 룰 충족): capability 술어·LSP 메서드·
  monaco 등록 함수만 주입하는 팩토리로 definition/implementation/typeDefinition/declaration 4종 생성.
  declaration 미지원 서버(gopls·tsls)는 capability 게이트로 자동 무해.
- Peek 미리보기: definition·references 계열 provider 가 위치 반환 전 **대상 파일 모델 선생성**
  (파일 읽기 IPC + 기존 model-registry 경로, `PEEK_MODEL_PRELOAD_LIMIT` 상수 상한).
- F8 4종은 기노출·동작 — cross-file 은 오프너로 해소. 신규 구현 없음, 검증·문서화 항목.

### 3.4 CodeLens (축소안) · LSP Folding

- codeLens 어댑터: provideCodeLenses + resolve(가시 렌즈만 — monaco 기본 동작) + `onDidChange`
  (didChange debounce 상수 + workspace/codeLens/refresh 수신 시 발화). 클라 커맨드는
  showReferences·rust-analyzer.gotoLocation 만(experimental.commands 에 그 2종 선언 —
  runSingle/debugSingle 은 실행 인프라 부재로 제외). 설정 `editor_code_lens_enabled`(기본 true) 토글.
- folding 어댑터: 0-based↔1-based +1 변환(전용 — 기존 Range 유틸 재사용 불가), kind 는
  `FoldingRangeKind.fromValue`, largeFile 기존 정책(꺼짐) 유지.
- **CancellationToken 소급**: 신규 어댑터 전부 + 기존 13종 소급(취소 시 요청 중단·stale 응답 폐기).

### 3.5 설정·i18n 스파인

- Settings 신규 3필드: `organize_imports_on_save`·`fix_all_on_save`(기본 false)·
  `editor_code_lens_enabled`(기본 true) — types/service/patch/**sync/service.rs 리터럴**/
  emptySettingsPatch/bindings 전체 배관. gist 동기화 대상(비밀 아님 — 화이트리스트 판단은 기존 규칙).
- locale 4곳(en/ko/ja + MESSAGE_NAMESPACES): 설정 라벨·설명 3쌍 + on-save 스킵 토스트 +
  적용 실패 토스트 등 (~10키), en⊆required 유지.

### 3.6 실행 구조 (역할 상향 반영)

- **Phase S 스파인(sonnet+xhigh 단독, Rust 단일 소유)**: §3.5 전체 + `initialize_params` 삭제 +
  `LspServerDetection.initialization_options` + lsp_send 락 해제 + cargo fmt + bindings 재생성.
  컴파일 그린(cargo test + tsc) 종료.
- **Phase B1 클라 코어(sonnet+xhigh 단독)**: protocol.ts(타입 확장) + client.ts(요청 라우팅·게이트 확장)
  + lsp-session-registry.ts(buildInitializeParams 정본 확장·initializationOptions·didSave 전송 배선).
- **Phase C 병렬 3(sonnet+xhigh)** — 파일 소유권 분리:
  C1 = 적용기+Code Action+on-save(workspace-edit·code-action 어댑터·diagnostics 사이드 맵·
  kind 유틸·command 중계·showReferences·editor-pane.tsx·rename.ts 통일) /
  C2 = 이동·Peek·오프너(definition 팩토리화+3종·오프너 bridge·모델 선생성·references.ts) /
  C3 = CodeLens+Folding+설정 UI(어댑터 2종·settings-view.tsx·CancellationToken 소급 — C1·C2 소유
  파일 제외한 기존 어댑터 9종).
- **Phase D 통합(sonnet+xhigh 단독)**: LANGUAGE_ADAPTER_REGISTRARS 등록·부트스트랩(오프너·커맨드)·
  문서(features/lsp.md·research 갱신)·전체 verify.
- **Phase E 검토**: 4렌즈(계약 정합/정확성/보안/설계·추상화 — opus+xhigh) → 적대적 검증(opus+high)
  → 수정(sonnet+xhigh) → **메인 2차**(verify 전체 + vite build + 하중 지점 실물 재검증) → 커밋(dev).

## 4. 기각·보류된 대안

| 안 | 처리 |
|----|------|
| on-save 를 monaco 액션(run) 재사용 | 기각 — run 이 적용 완료 비대기(소스 확인), 비결정 레이스 |
| IBulkEditService 내부 교체 | 기각 — 비공개 esm 내부 API 우회 |
| monaco 동봉 monaco-lsp-client 재사용 | 기각 — 전송 계층 전제 불일치·기존 12 어댑터와 이중화(참고 구현으로만) |
| initialize SSOT 를 Rust 로 승격 | 기각 — 실전송 주체가 프론트·커맨드 3곳 배선 증가. initializationOptions 는 detection 필드로 복원 |
| on-save kind 맵(VS Code 동형) | 기각 — boolean 2종 채택(사용자 결정 2) |
| CodeLens runSingle/debugSingle | 보류 — 실행·디버그 인프라 필요(태스크 러너 웨이브 이후 재검토) |
| executeCommand 화이트리스트 차단 | 기각 — docs/features/lsp.md 명시 금지(비광고 private 커맨드) |
| glyphMargin true 전환 | 보류 — 라이트벌브 gutter 폴백 한계는 문서화만(시각 영향, 재현 조건 협소) |
| 다중 루트 프론트 배선 복원 | 보류 — Wave A 밖, backlog(cross-root 인텔리전스와 함께) |
| workspace/configuration 서버별 매핑 | 보류 — null 기본 응답 채택, vtsls 매핑 테이블은 backlog |
| ra 실험 확장(snippetTextEdit 등) 선언 | 기각 — 파서 미지원 상태 선언은 '$0' 리터럴 삽입 버그 유발 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(커맨드 수 변화 0 — 신규 IPC 없음).
- 검토 4렌즈 + 적대적 검증 통과, 메인 2차 실물 재검증(초점: lsp_send 락 해제 안전성·요청 라우팅
  응답 정확성·적용기 순서·on-save 재무장 가드·diagnostic data 보존).
- 문서: features/lsp.md(어댑터 목록·요청 핸들러·on-save)·data-model(설정 3필드)·qa6-checklist 에
  Wave A 실기 항목 추가(→ 전문 QA 이월분).
- 검증 대상 실기(전문 QA 로 이월): ⌘. quickfix 목록·적용, organizeImports on save, F12/⌘F12
  cross-file 점프, Peek 미리보기, F8 파일 간 순회, CodeLens 표시·클릭, LSP folding, ra 파일 생성형 액션.
