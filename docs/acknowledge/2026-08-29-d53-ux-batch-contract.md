# d-53 계약 — UX 저비용 5건 + on-save 정리 + EditorConfig (2026-08-29)

> 범위 결정: `2026-08-29-audit-followup-user-decisions.md` §4 (사용자 선택: 5건+on-save+EditorConfig).
> 발견 근거: `docs/quality-assurance/2026-08-29-full-audit.md` §5. d-50/51 완료 트리 위에서 실행.
> 산출물은 커밋 지시 전까지 워킹트리. 신규 크레이트·npm 의존성 도입 금지(기존 트리 내 해결).

## §0 원칙

- 기본값은 **VS Code 파리티(보수)** — 전부 기본 off/빈 값, 설정 토글로 노출.
- 설정 표면 관례 준수: settings 필드+bindings+설정 UI 섹션+로케일 ko/en/ja+로케일 등재 테스트.
- EditorConfig 는 코어 키만: indent_style·indent_size·tab_width·insert_final_newline·
  trim_trailing_whitespace (charset·end_of_line 은 §5 이월 기록). 글롭은 기존 의존 트리
  (ignore→globset) 또는 자체 최소 매처로 — 선택 근거를 §3 에 기록.

## §1 스테이지 (직렬 U1→U2→U3)

| 스테이지 | 항목 |
|---|---|
| U1 에디터 옵션 5건 | diff hideUnchangedRegions+showMoves(diff-view 연동), guides.bracketPairs, smoothScrolling+cursorSmoothCaretAnimation, suggest.preview, editorRulers(숫자 목록 설정) |
| U2 on-save 정리 | trim trailing whitespace on save·insert final newline on save — 수동 ⌘S·자동 저장·format-on-save 파이프라인 공통 적용(포맷 후 적용 순서), 커서 위치 보존 |
| U3 EditorConfig | .editorconfig 체인 해석(Rust)+파일 열기 시 모델 indent 오버라이드(detectIndentation 대체)+U2 항목의 파일별 오버라이드 우선 적용. 변경 감지는 다음 열기 반영까지(한계 §5 기록) |

## §2 공통 규칙

d-50 계약 §2 준용(재현 테스트 우선·bindings/dispatch/로케일 파리티·최소 diff·커밋 금지).
Rust 접촉 스테이지는 직렬(한 시점 한 에이전트) — U1→U2→U3 순서 고정.

## §3 구현 기록 (스테이지별 append)

### U1 — 에디터 옵션 5건 (2026-08-29)

설정 7종 순증(전부 §0 대로 VS Code 파리티 기본 off/빈 값). Rust 필드+기본값 → `SettingsPatch` →
`apply_patch` → `sanitize` → sync 업로드 → `bindings.ts` 재생성 → `emptySettingsPatch()` →
설정 UI Editor 섹션 → 로케일 ko/en/ja + `MESSAGE_NAMESPACES` 등재까지 기존 설정 신설 관례를 그대로
탔다. 커맨드·이벤트·dispatch 표면은 전부 불변이다.

**(1) diff `hideUnchangedRegions` + `showMoves`** — `editorDiffHideUnchangedRegions` ·
`editorDiffShowMoves`. `DiffView` 가 두 값을 필수 prop 으로 받고, 생성 시가 아니라
`renderSideBySide` 와 같은 형태의 `updateOptions` effect 로 반영한다(설정을 바꾸면 열려 있는 diff
에 즉시 걸린다). **diff 표면 전량 연동**: `DiffPane`·`CommitFileDiff` 는 각자
`settingsQueryOptions()` 를 읽고, `features` 레이어라 쿼리를 읽을 수 없는
`ConflictCompareDialog` 는 `EditorPane` 이 `resolveDiffViewSettingsProps(settings)` 결과를
`diffViewSettings` prop 으로 내려준다(FSD 역방향 참조 회피). 매핑 함수는 `code-editor-settings.ts`
의 형제로 `shared/lib/diff-view-settings.ts` 신설 — 3곳에서 쓰이므로 "2회 이상" 룰 충족.

**(2) 브래킷 페어 가이드선** — `editorBracketPairGuides` → monaco `guides: { bracketPairs }`.
기존 `editorBracketPairColorization`(`bracketPairColorization.enabled`)은 손대지 않았다(별개 옵션).

**(3) smoothScrolling + cursorSmoothCaretAnimation** — `editorSmoothScrolling` ·
`editorCursorSmoothCaretAnimation`, 각각 별도 토글. 후자는 monaco 가 `'off'|'explicit'|'on'` 3값
이지만 토글로 좁혀 `'on'`/`'off'` 로만 매핑했다(§5 기록).

**(4) suggest.preview** — `editorSuggestPreview` → monaco `suggest: { preview }`.

**(5) editorRulers** — `Vec<u32>`/`number[]`. **입력 형태 조사 결과**: 기존 설정 표면의 목록 입력은
`remoteAllowedHosts` 의 추가/삭제 행 하나뿐인데, 열 번호는 짧고 여러 개를 한 번에 고치는 값이라
기존 `TextField`(blur 커밋 — `shellOverride`·`aiOmlxBaseUrl` 선례) + **콤마 구분 문자열**을 택했다.
파싱/표기는 `shared/lib/editor-rulers.ts`, 정규화(1~1000 밖 버림 · 중복 제거 · 오름차순 · 16개
상한)는 백엔드 `sanitize_editor_rulers` 와 프론트 `parseEditorRulers` 의 양쪽 거울이다(허용 호스트
검증을 양쪽에 둔 선례와 동일). 정렬까지 하는 이유는 화면이 저장값을 그대로 되그리기 때문 —
정렬하지 않으면 무관한 설정 변경마다 사용자가 쓴 순서가 흔들린다.

**의존성**: 신규 크레이트·npm 0. monaco 0.56 이 이미 다섯 옵션을 전부 갖고 있고(`editor.api.d.ts`
확인), 입력 컴포넌트·목록 정규화도 기존 `TextField`/`SwitchField` 와 `sanitize()` 경로로 해결했다.

**파일**: `src-tauri/src/domain/settings/types.rs` · `settings/service.rs` ·
`domain/sync/service.rs` · `domain/locale/service.rs` ·
`src-tauri/resources/locales/{ko,en,ja}.json` · `src/shared/api/bindings.ts`(재생성) ·
`src/shared/lib/code-editor-settings.ts` · `src/shared/lib/diff-view-settings.ts`(신규) ·
`src/shared/lib/editor-rulers.ts`(신규) · `src/features/editor/code-editor.tsx` ·
`src/features/git/diff-view.tsx` · `src/features/git/conflict-compare-dialog.tsx` ·
`src/widgets/diff-pane/diff-pane.tsx` · `src/widgets/commit-file-diff/commit-file-diff.tsx` ·
`src/widgets/editor-pane/editor-pane.tsx` ·
`src/widgets/settings-view/settings-editor-section.tsx` ·
`src/entities/settings/settings.ipc.ts`.
문서: `docs/ipc-contract.md`(d-53 U1 절) · `docs/features/editor.md` §15 · `docs/features/git.md` §4 ·
`docs/data-model.md`.

**테스트**: Rust 5건 신설(기본값 파리티 · patch 반영 6종 · rulers 정렬/중복/범위 · rulers 개수 상한 ·
손편집 settings.json 로드 시 rulers 정규화) + sync 파리티 1건. TS 신설 `editor-rulers.test.ts`(9) ·
`diff-view-settings.test.ts`(3), 기존 `code-editor-settings.test.ts` 를 21 prop 으로 확장.
검증: `cargo fmt --all --check` OK · `cargo clippy --lib --all-targets` 무경고 ·
`cargo test --lib` 1189 pass · `bun run typecheck` OK · `bun test` 1762 pass ·
touched 파일 `prettier --check`/`eslint` clean.

### U2 — on-save 정리 2건 (2026-08-29)

설정 2종 순증(`trimTrailingWhitespaceOnSave`·`insertFinalNewlineOnSave`, 둘 다 §0 대로 VS Code
파리티 기본 off — `files.trimTrailingWhitespace`/`files.insertFinalNewline` 대응). U1 이 탄 설정
표면 관례를 그대로 따랐다: Rust 필드+기본값 → `SettingsPatch` → `apply_patch` → sync 업로드 →
`bindings.ts` 재생성 → `emptySettingsPatch()` → 설정 UI Editor 섹션(on-save 묶음인 formatOnSave·
organizeImportsOnSave·fixAllOnSave 바로 뒤) → 로케일 ko/en/ja + `MESSAGE_NAMESPACES` 등재. 커맨드·
이벤트·dispatch 표면은 전부 불변이고, 두 값은 순수 bool 이라 `sanitize` 를 늘리지 않았다.

**적용 지점 — 저장 파이프라인 한 곳**: `widgets/editor-pane/use-editor-file-persistence.ts` 의
`handleSave`. ⌘S·자동 저장·format-on-save 가 전부 이 함수를 지나므로 트리거별 분기가 없다. 순서는
**Code Actions on Save(명시 저장만) → format-on-save → 후행 공백 제거 → 마지막 줄바꿈 →
`file_save`** 다. 포맷 뒤인 이유는 계약대로 포맷 결과의 후행 공백까지 정리하기 위해서고, 공백 제거를
줄바꿈보다 앞에 둔 이유는 "공백만 있는 마지막 줄"을 먼저 비워야 그 뒤에 줄바꿈이 덧붙지 않기
때문이다(VS Code 의 참여자 순서와 동일한 결과).

**커서 보존 — 기존 수동 Trim 액션과 공통화**: 별도 trim 구현을 만들지 않고 monaco 내장 액션
`editor.action.trimTrailingWhitespace`·`editor.action.insertFinalNewLine` 을 `editor.getAction(...)`
으로 재사용했다(format-on-save 가 `editor.action.formatDocument` 를 쓰는 것과 같은 형태). 두 액션은
`executeCommands` 경로로 편집하고 그 커맨드(`TrimTrailingWhitespaceCommand`·
`InsertFinalNewLineCommand`)가 `builder.trackSelection` 으로 선택을 추적하므로 캐럿·멀티커서가
제자리에 남는다 — `pushEditOperations` 로 직접 구현했다면 이 보존 로직을 다시 쓰는 셈이었다.
액션 id 는 키맵 카탈로그 `shared/lib/monaco/monaco-actions.ts` 를 단일 출처로 상수화
(`TRIM_TRAILING_WHITESPACE_ACTION_ID`·`INSERT_FINAL_NEW_LINE_ACTION_ID`)해 수동 실행 행과 저장
경로가 같은 문자열을 공유한다. 실행부는 `shared/lib/monaco/on-save-cleanup.ts` 의
`runOnSaveCleanup` 하나다.

**자동 저장의 커서 줄 예외**: 자동 저장일 때만 trim 액션에 `{ reason: 'auto-save' }` 를 넘겨 커서가
있는 줄의 후행 공백을 남긴다(monaco 액션이 이 인자를 읽어 커서 위치를 `minEditColumn` 으로 쓴다 —
VS Code save participant 의 `isAutoSaved` 와 같은 규약). 타이머가 타이핑 도중 방금 친 들여쓰기를
지우는 것을 막기 위해서다. 명시적 ⌘S 는 인자 없이 돌아 커서 줄까지 정리한다.

**dirty·미러·정착(d-51 F1) 상호작용**: 정리 편집은 모델을 거치므로 `onDidChangeContent` →
`onChange` → `handleChange` 로 draft 가 갱신된 뒤 `readDraft()` 가 최종 내용을 읽는다. 이 재진입은
이미 서 있는 `savingRef` 에 걸려 자동 저장을 다시 무장시키지 않고(포맷 단계와 동일한 기존 가드),
저장 성공 시 `settleDraftToDiskContent` 가 미러 타이머·`saveEpochRef`·`syncedContent` 를 함께
정리하므로 정착 경로도 그대로다. 정리 편집으로 미러 debounce 가 한 번 재무장될 수 있지만, 그 타이머는
정착에서 `clearTimeout` 되고 남는 경합은 포맷-온-세이브가 이미 갖고 있던 것과 동일하다(에폭 검사가
처리).

**의존성**: 신규 크레이트·npm 0. monaco 0.56 이 두 액션을 모두 내장하고 있고
(`editor.action.trimTrailingWhitespace` 는 이미 키맵 행으로 노출 중), 설정 표면도 기존
`SwitchField` 로 해결했다.

**파일**: `src-tauri/src/domain/settings/types.rs` · `settings/service.rs` · `domain/sync/service.rs` ·
`domain/locale/service.rs` · `src-tauri/resources/locales/{ko,en,ja}.json` ·
`src/shared/api/bindings.ts`(재생성) · `src/shared/lib/monaco/monaco-actions.ts`(액션 id 상수화) ·
`src/shared/lib/monaco/on-save-cleanup.ts`(신규) ·
`src/widgets/editor-pane/use-editor-file-persistence.ts` · `src/widgets/editor-pane/editor-pane.tsx` ·
`src/widgets/settings-view/settings-editor-section.tsx` · `src/entities/settings/settings.ipc.ts`.
문서: `docs/ipc-contract.md`(d-53 U2 절) · `docs/features/editor.md` §16(+§3 포인터) ·
`docs/data-model.md`.

**테스트**: TS 신설 `on-save-cleanup.test.ts`(10 — 기본값 무동작 · 순서(trim→final newline) ·
켠 것만 실행 · ⌘S 는 인자 없음 · 자동 저장은 `auto-save` 사유 · 줄바꿈 액션엔 사유 미전달 ·
editor null · 액션 미등록 건너뛰기 · 액션 실패를 삼키고 다음 단계 진행). Rust 신설 4건(기본값
파리티 · patch 반영 · patch 생략 시 기존값 보존 · sync 업로드 파리티).
검증: `cargo fmt --all --check` OK · `cargo clippy --lib --all-targets` 무경고 ·
`cargo test --lib` 1193 pass · `bun run typecheck` OK · `bun test` 1772 pass ·
touched 파일 `prettier --check` clean, `eslint` 는 기존 경고 2건(`use-editor-file-persistence.ts` 의
`exhaustive-deps` — 이번 변경과 무관한 기존 effect)만.

### U3 — EditorConfig (2026-08-29)

설정 1종 순증(`editorConfigEnabled`, §0 대로 **VS Code 파리티 기본 off** — VS Code 도 EditorConfig 를
코어에 갖지 않고 확장으로 제공한다). U1·U2 가 탄 설정 표면 관례를 그대로 따랐다: Rust 필드+기본값 →
`SettingsPatch` → `apply_patch` → sync 업로드 → `bindings.ts` 재생성 → `emptySettingsPatch()` →
설정 UI Editor 섹션(on-save 묶음 바로 뒤) → 로케일 ko/en/ja + `MESSAGE_NAMESPACES` 등재. 순수 bool
이라 `sanitize` 를 늘리지 않았다. **커맨드·이벤트·dispatch·원격 정책 표면은 전부 불변** — 신규 커맨드
0이라 `IMPLEMENTED_JSON_COMMANDS`·`REMOTE_ALLOWED_COMMANDS`·`REMOTE_DENIED_COMMANDS`·dispatch `match`
어느 것도 건드리지 않았고, 완전분할 테스트도 그대로 통과한다.

**(1) 체인 해석(Rust) — `src-tauri/src/domain/file/editorconfig.rs` 신설**. 파일 디렉토리에서 위로
올라가며 `.editorconfig` 를 읽고 `root = true` 에서 멈춘다. 적용은 바깥 파일부터(가까운 파일이 이김),
한 파일 안에서는 위에서 아래로(뒤에 오는 매칭 섹션이 이김), `unset` 은 프로퍼티를 지운다. 코어 5키만
남기고, `indent_size = tab` 은 `tab_width` 로 해석한다. 상한은 조상 64단·파일당 64KB·indent 1~64열·
섹션 패턴 1024자(백트래킹 매처의 최악 비용을 묶는다). **프로젝트 루트 위로도 올라간다** — EditorConfig
의 정의가 그렇고, 읽는 것은 `.editorconfig` 라는 이름의 파일뿐이며 경로는 이미 루트 가드가 정규화한
값이다.

**글롭 선택 근거(§0 요구)**: `globset` 승격이 아니라 **자체 최소 매처**를 썼다. 두 안 모두 신규
크레이트 0이지만(globset 은 `ignore` 의 전이 의존이라 이미 트리에 있다), EditorConfig 의 패턴 언어가
globset 의 것이 아니다 — ① `{n1..n2}` 정수 범위에 globset 대응이 없고, ② 콤마 없는 중괄호(`{word}`)는
**리터럴**이라 globset 의 alternate 와 의미가 다르며, ③ `a**z` 처럼 경로 성분 안의 `**` 를 globset 은
아예 거부한다(`InvalidRecursive`). 즉 globset 을 쓰려면 "EditorConfig 패턴 → globset 문법" 번역층을
따로 짜야 하는데, 그 번역층의 테스트 표면은 매처와 같고 위 3건은 번역으로도 표현되지 않는다. 그래서
`Cargo.toml` 무변동 + 의미를 정확히 통제하는 쪽을 택했다. 지원 문법: `*`(구분자 제외)·`**`·`?`·
`[abc]`/`[!abc]`/`[a-z]`·`{a,b}`·`{n1..n2}`·`\` 이스케이프. 구분자 없는 패턴은 `**/` 를 앞에 붙이고,
있는 패턴은 설정 파일 디렉토리 기준으로 고정한다(선행 `/` 1개 제거). `**/` 는 **디렉토리 0개도**
매칭한다 — 이것이 `*.c` 가 설정 파일과 같은 층의 `main.c` 도 잡게 하는 지점이다.

**(2) IPC 표면 — `OpenedFile.editorConfig` 순증(별도 커맨드 아님)**. 신규 커맨드/쿼리를 두지 않고 열기
응답에 실은 이유는 **같은 왕복이어야 첫 렌더부터 올바른 들여쓰기로 모델이 붙기** 때문이다. 쿼리를 하나
더 두면 모델이 추정 들여쓰기로 붙었다가 한 틱 뒤 교정되는 깜빡임이 생기고, 캐시·무효화 표면도 하나
늘어난다. `editor_config_enabled` 가 꺼져 있으면 `open_file` 이 체인을 아예 걷지 않아 기본 상태에서
비용이 0이고, `refused` 티어도 전 필드 `null` 이다(편집 버퍼가 없다).

**(3) 들여쓰기 적용은 "모델"이다 (핵심 설계 판단)**. `tabSize`/`insertSpaces`/`detectIndentation` 은
monaco 의 **전역** 옵션(`IGlobalEditorOptions`)이다 — `editor.updateOptions` 는
`updateConfigurationService`(`standaloneCodeEditor.js`)를 거쳐 앱 하나짜리
`StandaloneConfigurationService` 를 바꾸고, 그 변경은 `ModelService._updateModelOptions` 가 **열려 있는
모든 모델**에 다시 뿌린다(설치본 0.56 소스 확인). 파일별 값을 그 경로로 보내면 스플릿한 두 페인이 서로의
들여쓰기를 덮어쓴다. 그래서 전역 경로에는 **기존대로 설정값만** 흐르게 두고(피크 프리뷰 등 CodeEditor
밖에서 만들어지는 모델의 기존 동작 보존), editorconfig 오버라이드는 `CodeEditor` 의 새 effect 가
`ITextModel.updateOptions` 로 그 페인의 모델에만 얹는다. 오버라이드가 없으면 **모델을 아예 건드리지
않으므로** `.editorconfig` 가 없는 파일의 동작은 이전과 완전히 동일하다.
- effect 의존성에 `tabSize`·`insertSpaces`·`detectIndentation`·`bracketPairColorization`·`largeFile`
  이 함께 들어간다. `_setModelOptionsForModel` 의 조기 반환("same indent opts")이 나머지 옵션 변경을
  전부 걸러 주고, 실제로 모델까지 닿는 전역 변경은 이 다섯이 만드는 것뿐이다(뒤의 둘은 bracket
  colorization 이 모델 생성 옵션이기 때문). 같은 커밋에서 옵션 effect(먼저 선언) → 이 effect 순으로
  돌아 오버라이드가 즉시 재확정된다. 자기치유 리스너(`onDidChangeOptions` 구독)를 쓰지 않은 이유는
  키맵이 노출하는 수동 `indentUsingSpaces`/`indentUsingTabs`/`detectIndentation` 을 되돌려 사용자와
  싸우게 되기 때문이다.
- 한쪽 축만 지정된 설정은 그 축만 뒤집는다(`updateOptions` 는 넘기지 않은 필드를 유지한다) — 즉
  `indent_style` 만 있는 파일은 탭 폭을 추정/설정값 그대로 둔다.

**(4) on-save 오버라이드(U2 파이프라인)**: `resolveOnSaveCleanupFlags` 가
`trim_trailing_whitespace`/`insert_final_newline` 을 전역 설정보다 우선 적용한다(`??` 폴백이라
editorconfig 의 명시적 `false` 가 전역 `true` 를 이긴다). 적용 지점은 U2 가 만든
`use-editor-file-persistence.ts` 의 `handleSave` 한 곳 그대로 — 순서·커서 보존·자동 저장 예외는 U2 의
규약을 건드리지 않았다.

**의존성**: 신규 크레이트·npm 0 (`Cargo.toml`·`package.json` 무변동).

**파일**: `src-tauri/src/domain/file/editorconfig.rs`(신규) · `domain/file/types.rs` ·
`domain/file/mod.rs` · `domain/file/service.rs` · `domain/file/commands.rs` ·
`domain/settings/types.rs` · `domain/settings/service.rs` · `domain/sync/service.rs` ·
`domain/locale/service.rs` · `src-tauri/resources/locales/{ko,en,ja}.json` ·
`src/shared/api/bindings.ts`(재생성) · `src/shared/lib/editorconfig.ts`(신규) ·
`src/features/editor/code-editor.tsx` · `src/widgets/editor-pane/editor-pane.tsx` ·
`src/widgets/editor-pane/use-editor-file-persistence.ts` ·
`src/widgets/settings-view/settings-editor-section.tsx` · `src/entities/settings/settings.ipc.ts` ·
`src/entities/layout/tab-path-change.test.ts`(`OpenedFile` 픽스처 1필드).
문서: `docs/ipc-contract.md`(d-53 U3 절) · `docs/features/editor.md` §17 · `docs/data-model.md`.

**테스트**: Rust 신설 19건 — `editorconfig.rs` 15(파싱: 섹션·코어키 필터·대소문자/콜론·섹션 안 root
무시 / 글롭: `*` 구분자 경계·`**`·`**/` 0디렉토리·선행 슬래시 고정·`?`·문자클래스·중괄호 교대와
리터럴·숫자 범위·이스케이프 / 체인: 가까운 설정+후행 섹션 우선·`root=true` 정지·`unset`·
`indent_size = tab`·범위 밖 값 무시·무매칭), `file/service.rs` 2(설정 off 면 체인 미탐색 / on 이면 열기
응답에 실림), `settings/service.rs` 2(기본값 off · patch 반영과 생략 시 보존), `sync/service.rs` 1
(업로드 파리티). TS 신설 `editorconfig.test.ts` 12(indent prop 유도 3축 · 모델 페이로드 null/부분/전체 ·
on-save 오버라이드 4). 기존 `tab_path_change` 픽스처에 필드 1개 추가.
검증: `cargo fmt --all --check` OK · `cargo clippy --lib --all-targets` 무경고 ·
`cargo test --lib` 1214 pass · `bun run typecheck` OK · `bun test` 1784 pass ·
touched 파일 `prettier --check` clean, `eslint` 는 기존 경고 2건
(`use-editor-file-persistence.ts` 의 `exhaustive-deps` — 이번 변경과 무관한 기존 effect)만.

### R — 검토 반영 (2026-08-29)

d-53 전체(U1~U3)에 대한 검토 발견 9건(minor 5 · info 4)의 처리. **재검증은 전부 설치본
monaco 0.56 소스와 실제 워킹트리 코드**로 했고, 확증분만 고쳤다. 항목별 판정은 §4 표, 고치지 않기로
한 것의 사유·한계는 §5 R 절.

**(1) 글롭 매처 메모이제이션(minor-4 — `editorconfig.rs`)**. `MAX_SECTION_PATTERN_CHARS` 의 doc
comment 는 "매처가 백트래킹하므로 패턴 길이를 묶으면 그 비용이 묶인다"고 적었지만 사실이 아니었다 —
`glob_matches` 에 메모이제이션이 없어 와일드카드가 겹치면 상태 하나를 도달 경로 수만큼 다시 판다.
실측(같은 알고리즘을 JS 로 옮겨 호출 수를 셈): `*a` ×6/입력 32자 = 254만 호출, ×8 = 3458만, ×10 =
2.58억, ×12 = 11.6억 — 스타 2개마다 약 8배. 1024자 상한은 스타 500개를 담으므로 사실상 끝나지 않고,
이 walk 는 `file_open`(`spawn_blocking`) 안에서 돌아 그 파일 열기가 응답하지 않고 블로킹 풀 스레드를
점유한다. 신뢰할 수 없는 저장소를 여는 IDE 라는 맥락에서 재현 조건이 "`.editorconfig` 파일 하나"뿐이라
와일드카드 개수 상한(경우의 수 자체는 여전히 조합적이라 실질 보장이 못 된다)이 아니라 **메모이제이션**을
택했다: `HashMap<(노드 주소, 남은 입력 길이), bool>`. 키가 성립하는 근거는 **파싱 트리에서 한 노드는
정확히 한 자리에만 있고, 그 노드 이후의 연속(continuation)은 그 자리로 완전히 결정**되기 때문이다 —
자기 리스트의 나머지 + 그 리스트가 속한 교대(alternation) 뒤의 나머지 + … 로 위까지 확정된다. 그래서
`Alternation` 이 분기를 앞에 스플라이스한 상태까지 포함해 (헤드 노드 주소, 남은 입력 길이)가 상태를
유일하게 지목한다. 상태 수가 노드 수 × 입력 길이로 묶이므로 다항이다. doc comment 의 근거 문장도 실제
보장(메모 테이블이 비용을 묶고, 상한은 그 테이블의 크기와 메모리를 묶는다)으로 고쳤다.

**(2) on-save 정리 액션의 동기 예외(minor-3 — `on-save-cleanup.ts`)**.
`getAction(id)?.run(args).catch(() => undefined)` 는 `run()` 이 **Promise 를 돌려준 뒤의** 거부만
잡는다. monaco 0.56 확인: `InternalEditorAction.run`(`common/editorAction.js`)은 `this._run(args)` 를
그대로 돌려주고, 그 `_run` 은 `codeEditorWidget.js:241` 의
`this._instantiationService.invokeFunction(...)` 이며 `invokeFunction`(`instantiationService.js:85`)은
`return fn(accessor)` 인 **동기 함수**다. 즉 액션이 자기 `Promise.resolve` 에 닿기 전에 던지면
(`executeCommands` 실패·telemetry/context-key 접근 등) `run()` 에서 **동기 throw** 가 나오고 `.catch`
는 평가조차 되지 않는다. 그러면 `runOnSaveCleanup` 이 reject → `use-editor-file-persistence.ts` 의
`await` 가 throw → `handleSave` 가 `saveFile` 을 부르지 못한 채 끝나고 `savingRef` 가 선 채로 남는다.
각 단계를 `try`/`catch` 로 감싼 `runCleanupAction` 하나로 정리했다(계약 §3 U2 가 규정한 "실패를 삼키고
다음 단계 진행"이 이 경로에서도 성립하게). 기존 페이크 액션이 `run: async` 라 동기 throw 를 재현하지
못했으므로 페이크를 "프로미스를 돌려주는 일반 함수"로 바꾸고 `throwingSynchronously` 케이스를 1건
추가했다 — 이 테스트는 수정 전 코드에서 실제로 실패한다(별도 재현: 같은 형태의 체인이 sync throw 에
reject 되는 것 확인).

**(3) `editorRulers` 입력의 커밋 되돌려 쓰기(minor-5 — `text-field.tsx`)**. `TextField` 는
`key={value}` + `defaultValue` 라 **저장값이 바뀔 때만** 입력이 리마운트된다. `parseEditorRulers` 가
범위 밖·비정수를 조용히 버리므로 정규화 결과가 기존 저장값과 같으면(`[80]` 에 `80, 2000`, 빈 목록에
`abc`) `value` 도 `key` 도 그대로여서 사용자가 친 텍스트가 화면에 남는다 — d-51 F6 이 numeric-field
에서 없앤 "쓰여 있는 값 ≠ 적용된 값"과 같은 부류다. numeric-field 의 blur write-back 선례를 그대로
따라 `TextField` 에 **선택적 `normalize` prop** 을 두고(넘기지 않은 `shellOverride`·`aiOmlxBaseUrl`
동작은 무변경), rulers 행만 `normalizeEditorRulersText`(`formatEditorRulers ∘ parseEditorRulers`)를
넘긴다. 정규화 함수는 `editor-rulers.ts` 에 두어 순수 함수로 테스트한다(이 레포에는 DOM 렌더 하네스가
없다 — `error-boundary.test.tsx` 가 적어 둔 제약).

**(4) `NO_EDITOR_RULERS` 를 `readonly` 로(info-8)**. 모든 CodeEditor 호스트가 공유하는 단일 빈 배열
인데 타입이 가변이었다. `readonly number[]` 로 좁히고 `CodeEditorProps.rulers` 도 맞췄으며, monaco 의
`rulers` 는 가변 배열을 요구하므로 `updateOptions` 경계에서만 `[...rulers]` 로 복사한다(effect 안이라
의존성 비교에는 영향이 없다). 참조 안정성 목적은 그대로다.

**문서 정정(minor-2 · info-6 · info-7 · info-9)**: `editor.md` §15 의 monaco 객체 옵션 병합 설명
(실제로는 `EditorOptionsUtil.applyUpdate` 가 하위 필드 단위 재귀 병합 — 기본값으로 채워지는 것은 "한
번도 설정한 적 없는" 필드뿐), §16 의 적용 범위 밖 목록에 untitled Save As 추가, §17 의 "다음에 그
파일을 열 때부터" 를 **해제 방향은 재시작 전까지 반영되지 않는다**로 정정(+ 다른 페인이 전역 옵션을
뒤집는 경로 1줄), `ipc-contract.md` d-53 U3 의 반영 시점 항목 동반 정정, `data-model.md` d-53 트리
주석의 괄호 짝, `settings-ui.md` §5 에 `TextField` 되돌려 쓰기 관례 추가.

**파일**: `src-tauri/src/domain/file/editorconfig.rs` · `src/shared/lib/monaco/on-save-cleanup.ts` ·
`src/shared/lib/editor-rulers.ts` · `src/features/settings/text-field.tsx` ·
`src/widgets/settings-view/settings-editor-section.tsx` · `src/features/editor/code-editor.tsx`
(prop 타입 `readonly` + 오버라이드 effect 의 doc comment 에 인스턴스 간 한계 명시).
문서: `docs/features/editor.md` §15·§16·§17 · `docs/features/settings-ui.md` §5 ·
`docs/ipc-contract.md`(d-53 U3) · `docs/data-model.md`.

**테스트**: Rust 신설 1건(`와일드카드가_겹쳐도_지수적으로_퍼지지_않는다` — `*a` ×24 를 64자 입력에
매칭, 실패 케이스와 성공 케이스 양쪽. 메모이제이션이 없으면 끝나지 않는다). TS 신설 4건
(`on-save-cleanup.test.ts` 동기 throw 1 · `editor-rulers.test.ts` `normalizeEditorRulersText` 3).
검증: `cargo fmt --all --check` OK · `cargo clippy --lib --all-targets` 무경고 · `cargo test --lib`
1215 pass · `bun run typecheck` OK · `bun test` 1788 pass · touched 파일 `prettier --check` clean,
`eslint` 는 기존 경고 2건(`use-editor-file-persistence.ts` 의 `exhaustive-deps`)만.

## §4 검토·수정 기록

### d-53 검토 발견 9건 판정 (2026-08-29)

| # | 심각도 | 발견 | 판정 | 근거 |
|---|---|---|---|---|
| 1 | minor | `.editorconfig` 오버라이드가 다른 페인의 전역 옵션 변경에 지워진다 | **확증 · 미수정(한계 기록)** | 경로 실증(monaco `standaloneCodeEditor.js:217` → `updateConfigurationService` → `modelService.js:180-209` 의 조기반환이 `bracketPairColorizationOptions` 비교까지 포함). 고치려면 `largeFile` 을 전역 bracket 옵션에서 떼거나 자기치유 구독이 필요한데, 전자는 대형 파일의 색칠 억제(성능 보호)를 모델 축으로 옮기는 설계 변경이고 후자는 계약 §3 U3 가 명시적으로 기각한 선택(수동 들여쓰기 명령과 싸운다) — §5 R 에 한계 기록 + 코드 doc comment·`editor.md` §17 에 명시 |
| 2 | minor | 오버라이드를 거두는 경로가 없어 규칙 삭제·설정 off 가 반영되지 않는다 | **확증(문서만 정정)** | 실파일 모델은 탭을 닫아도 dispose 되지 않음을 워킹트리로 확인(`disposeModel` 호출처는 untitled 2곳 + rename/delete 경로뿐). 되돌리기 구현은 "prop 이 null 이 된 전이"에 전역값·`detectIndentation` 을 다시 거는 동작 신설이라 수동 들여쓰기 명령·페인 간 파일 전환과 상호작용이 생긴다 — 계약 §1 U3 범위(적용) 밖으로 보고 §5 R 이월, 문서의 "다음에 열 때부터" 주장만 사실에 맞게 정정 |
| 3 | minor | on-save 정리 액션의 동기 예외가 저장을 취소한다 | **확증 · 수정** | monaco 0.56 소스로 동기 전파 경로 확인 + 같은 형태의 체인이 sync throw 에 reject 되는 것 실측. `runCleanupAction` 의 `try`/`catch` + 동기 throw 테스트 1건 |
| 4 | minor | 섹션 글롭 매처의 백트래킹이 묶이지 않는다(악성 `.editorconfig` DoS) | **확증 · 수정** | 동일 알고리즘의 호출 수 실측으로 지수 증가 확인(스타 2개마다 ×8). `glob_matches` 메모이제이션 + doc comment 근거 문장 정정 + 회귀 테스트 |
| 5 | minor | `editorRulers` 정규화 결과가 저장값과 같으면 잘못된 텍스트가 남는다 | **확증 · 수정** | `text-field.tsx` 의 `key={value}`+`defaultValue` 구조로 재현 성립. `normalize` prop + `normalizeEditorRulersText` |
| 6 | info | untitled Save As 가 on-save 정리를 타지 않는다(기록 누락) | **확증(문서만)** | `untitled-pane.tsx` 의 `handleSaveAs` 가 `useSaveFile` 직접 호출인 것 확인. 동작 변경 없이 `editor.md` §16·§5 R 에 기록 |
| 7 | info | `editor.md` §15 의 객체 옵션 병합 설명이 부정확 | **확증(문서만)** | `editorConfiguration.js:274-283` → `editorOptions.js:66-85` 의 재귀 병합 확인. 문장 정정 |
| 8 | info | `NO_EDITOR_RULERS` 가 가변 배열로 공유된다 | **확증 · 수정** | 실오염 경로는 없으나 타입만으로 막을 수 있어 `readonly` 로 좁힘 |
| 9 | info | `data-model.md` d-53 항목의 괄호 짝 어긋남 | **확증 · 수정** | HEAD 와 대조해 U1 줄이 닫아 버린 것 확인, U3 끝으로 이동 |

기각 0건 — 9건 모두 사실로 확증됐고, 그중 2건(#1·#2)은 수정이 계약 범위 밖의 동작·설계 변경이라
문서·한계 기록으로 처리했다.

## §5 이월·한계 기록

### U1 (2026-08-29)

- **`cursorSmoothCaretAnimation` 의 `'explicit'` 미노출**: monaco 는 3값(`'off'`/`'explicit'`(명시적
  커서 이동에만 애니메이션)/`'on'`)인데 설정은 on/off 토글이다. 중간값 하나를 위해 3지 선택 UI 를
  두지 않는 판단 — 필요해지면 `OptionPicker` 로 승격하면 되고, 그때 기존 `bool` 값의 마이그레이션이
  필요하다(false→`'off'`, true→`'on'`).
- **diff 옵션은 전역 설정 1축**: 표면별(SCM/커밋/충돌 비교) 개별 토글은 두지 않았다. 툴바의
  "Collapse Unchanged Regions" 버튼(`git.md` §4, 2차 항목)은 여전히 미구현이며, 구현하면 설정과
  **별개의 일시 토글**이 되어야 한다(설정값을 덮어쓰지 않도록).
- **`editorRulers` 는 열 번호만**: monaco `rulers` 는 `(number | IRulerOption)[]` 이라 열마다 색을
  지정할 수 있지만, 색까지 노출하지 않았다(VS Code 도 기본 UI 에서는 숫자 배열). 상한 16개·최대 열
  1000 은 이번에 정한 값으로 VS Code 에는 대응 상한이 없다 — 실사용에서 좁다는 보고가 오면 양쪽
  거울(`service.rs`·`editor-rulers.ts`)을 함께 올려야 한다.
- **`TextField` 는 blur 마다 커밋한다**: 값이 그대로여도 `settings_update` 가 한 번 나간다(기존
  `shellOverride`·`aiOmlxBaseUrl` 과 동일한 기존 동작 — 이번에 바꾸지 않았다). 숫자 필드처럼
  "저장값과 같으면 IPC 생략"을 `TextField` 전반에 넣는 것은 이 스테이지 범위 밖이라 미착수.

### U2 (2026-08-29)

- **외부 저장 경로에는 적용하지 않았다**: Claude Code 의 `ide:save-requested`
  (`app/providers/ide-sync-provider.tsx`)와 충돌 해소 쓰기
  (`use-editor-git-gutter-and-conflicts.ts`)는 모델만 들고 에디터 인스턴스(=커서)를 전제하지 않아
  이번 정리를 태우지 않았다. 계약 §1 U2 가 명시한 적용 지점(⌘S·자동 저장·format-on-save)에도
  포함되지 않는다. 태우려면 커서 없는 모델 레벨 정리(`model.pushEditOperations`)를 별도로 만들어야
  하고, 그때는 "커서 보존"이 성립하지 않는 경로라는 점을 문서화해야 한다.
- **`files.trimTrailingWhitespaceInRegexAndStrings` 미노출**: monaco 액션은 이 설정을
  `IConfigurationService` 에서 읽는데, standalone 에디터의 설정 서비스는 등록되지 않은 키에
  `undefined` 를 돌려주므로 **문자열·정규식 리터럴 안의 후행 공백은 보존**되고, 아직 토큰화되지 않은
  줄(대형 파일의 아래쪽)도 건너뛴다. VS Code 의 같은 설정 기본값은 `true`(=리터럴 안도 정리)라
  이 지점만 파리티가 어긋난다. 노출하려면 monaco standalone 설정 서비스에 키를 주입하는 경로를
  따로 만들어야 해서 이번 범위에서 제외했다.
- **`editor.action.insertFinalNewLine` 은 "공백만 있는 마지막 줄"에 아무 것도 하지 않는다**(monaco
  `InsertFinalNewLineCommand` 의 `lastLineIsEmptyOrWhitespace` 분기). trim 을 함께 켜면 그 줄이 먼저
  비워져 의도한 결과가 나오지만, 줄바꿈만 단독으로 켜면 파일이 `"...\n   "` 로 끝나는 경우 줄바꿈이
  추가되지 않는다 — VS Code 와 동일한 동작이라 그대로 두었다.
- **`TrimFinalNewLines` 는 도입하지 않았다**: VS Code 는 `files.trimFinalNewlines` 로 파일 끝의 여분
  빈 줄을 줄이는 세 번째 참여자를 갖지만, 계약 §1 U2 는 2건만 규정한다. monaco 에 대응 내장 액션이
  없어(직접 구현이 필요하다) 이번 범위 밖으로 남긴다.

### U3 (2026-08-29)

- **`.editorconfig` 변경의 실시간 반영 없음(계약 §1 U3-4 명시 범위 밖)**: 해석은 `file_open` 시점
  1회이고 결과는 `OpenedFile.editorConfig` 에 실린다. `.editorconfig` 를 편집해도 **이미 열린 탭에는
  반영되지 않고, 다음에 그 파일을 열 때부터** 적용된다. `editorConfigEnabled` 토글도 마찬가지다
  (설정 변경은 `FILE.CONTENT` 를 무효화하지 않는다). 실시간 반영을 하려면 워처가
  `.editorconfig` 변경을 감지해 영향받는 `FILE.CONTENT` 쿼리를 무효화하는 경로가 필요한데, 그
  무효화는 열린 탭의 draft 재동기화 경로(d-43 계열)를 건드리므로 별도 배치가 맞다.
- **`charset`·`end_of_line` 미지원**: 둘 다 지금은 얹을 자리가 없다. 파일 도메인은 UTF-8 로만
  디코딩하고(비 UTF-8 은 감사 §4-A-3 대로 손실 표식 + 읽기 전용), `file_save` 는 받은 `String` 을
  그대로 쓴다 — 즉 `charset` 은 인코딩 왕복 지원(백로그 §7)이 선행되어야 하고, `end_of_line` 은
  monaco 모델의 `defaultEOL`/저장 시 개행 변환 경로를 새로 만들어야 한다. 계약 §0 이 코어 5키로
  범위를 못 박았으므로 이번엔 파싱 자체를 하지 않는다(읽고 버리지 않는다 — 지원 여부가 코드에서
  명확히 드러나도록).
- **글롭 미지원 문법**: `{}` 안의 이스케이프·중첩은 지원하지만, `[` 문자 클래스 안의 `-` 범위 끝에
  이스케이프(`[a-\]]`)가 오는 경우처럼 극단적 조합은 리터럴로 낙착된다. EditorConfig 코어 테스트
  스위트 전체를 통과시키는 것은 목표로 하지 않았다 — 실사용 패턴(확장자·디렉토리·중괄호 목록)을
  덮는 것이 목표다. 불일치 보고가 오면 `editorconfig.rs` 의 매처와 그 단위 테스트가 단일 수정
  지점이다.
- **monaco `indentSize` 축은 쓰지 않았다**: 계약 §1 U3 가 적용 대상을 `tabSize`·`insertSpaces` 두
  축으로 명시했으므로 그대로 따랐다. monaco 모델은 `indentSize`(들여쓰기 폭)를 `tabSize`(탭 문자의
  표시 폭)와 별도로 가질 수 있어서, `indent_style = space` + `indent_size ≠ tab_width` 인 드문
  조합(`indent_size = 2, tab_width = 8`)에서는 파일 안의 실제 탭 문자가 8이 아니라 2폭으로 그려진다.
  들여쓰기 자체는 항상 의도대로 동작한다. 세 번째 축까지 맞추려면
  `resolveEditorConfigIndentProps` 가 `editorConfigIndentSize` 를 하나 더 내보내고 `tabSize` 를
  `tab_width` 우선으로 바꾸면 되는데, prop 1개와 매핑 규칙 1개가 더 늘어난다.
- **`indent_size`/`tab_width` 상한 1~64는 이번에 정한 값**이다. EditorConfig 스펙에는 상한이 없고,
  설정 UI 의 `editorTabSize`(1~8)와도 다른 축이다(그쪽은 사용자 입력 어포던스, 이쪽은 외부 파일
  방어). 실사용에서 좁다는 보고가 오면 `MAX_INDENT_COLUMNS` 한 곳만 올리면 된다.
- **모델 오버라이드 effect 의 의존성은 수동 목록이다**: `CodeEditor` 가 monaco **전역** 설정으로
  밀어 넣는 옵션 중 모델 생성 옵션(`tabSize`·`insertSpaces`·`detectIndentation`·`indentSize`·
  `trimAutoWhitespace`·`bracketPairColorization`)에 해당하는 것이 바뀌면 `ModelService` 가 모든
  모델의 들여쓰기를 전역값으로 되돌리므로, 그 prop 들이 effect 의존성에 함께 들어가 재확정을
  보장한다. 앞으로 **이 여섯 중 하나에 해당하는 전역 옵션을 새로 노출하면 그 prop 을 이 의존성
  목록에 추가**해야 한다(effect 의 doc comment 에도 명시). 자동으로 안전하게 만들려면
  `model.onDidChangeOptions` 자기치유 구독이 필요한데, 그러면 키맵의 수동
  `indentUsingSpaces`/`indentUsingTabs`/`detectIndentation` 을 즉시 되돌려 사용자와 싸우게 되므로
  택하지 않았다.
- **`.editorconfig` 를 프로젝트 루트 위에서도 읽는다**: EditorConfig 의 정의대로 `root = true` 또는
  파일시스템 루트까지 올라간다. 루트 가드가 막는 것은 사용자 입력 경로의 탈출이고, 이 walk 는 이미
  검증된 정규 경로에서 위로만 가며 `.editorconfig` 라는 이름의 파일만 읽는다(조상 64단·파일당
  64KB 상한). 프로젝트 루트에서 멈추는 정책으로 바꾸고 싶다면 `resolve_for_file` 에 루트를 넘기는
  한 줄이면 되지만, 서브디렉토리를 프로젝트로 연 경우 상위 저장소의 설정이 조용히 무시된다.
- **`untitled`·AppFile 탭에는 적용되지 않는다**: 디스크 경로가 없어(`untitled:`) 체인을 걸 기준
  디렉토리가 없고, AppFile 은 앱 소유 경로라 사용자의 `.editorconfig` 대상이 아니다. 두 호스트는
  `editorConfigTabSize`/`editorConfigInsertSpaces` 를 넘기지 않아(기본 `null`) 오버라이드 없이
  기존 경로를 그대로 탄다.

### R — 검토 반영 (2026-08-29)

- **인스턴스 간 전역 쓰기로 오버라이드가 풀리는 경로(검토 #1, 미수정)**: `CodeEditor` 가 전역으로
  밀어 넣는 모델 생성 옵션 중 `bracketPairColorization` 만은 값이 **페인마다 갈린다**
  (`bracketPairColorization && !largeFile`, `largeFile` 은 페인이 파일 tier 로 정하는 값). 옆 페인이
  large/readOnly tier 파일을 여닫으면 그 전역값이 뒤집혀 `ModelService._updateModelOptions` 가 모든
  모델의 들여쓰기를 다시 계산하고(조기반환이 `bracketPairColorizationOptions` 까지 비교한다),
  `.editorconfig` 오버라이드를 지닌 다른 페인의 모델이 전역/추정값으로 돌아간다. 그 페인의 prop 은
  하나도 바뀌지 않으므로 §3 U3 의 재확정 effect 도 돌지 않는다. **U3 의 "여섯 옵션을 새로 노출하면
  의존성에 추가하라" 규약은 이 인스턴스 간 경로를 덮지 못한다.** 두 해법 모두 이번 범위를 넘는다 —
  ① `largeFile` 을 전역 bracket 옵션에서 떼어 내려면 대형 파일의 색칠 억제를 모델 축
  (`ITextModelUpdateOptions.bracketColorizationOptions`)으로 옮겨야 하는데, 그러면 "오버라이드가 없으면
  모델을 아예 건드리지 않는다"는 U3 의 불변식이 깨진다. ② `model.onDidChangeOptions` 자기치유는 §3 U3
  가 이미 기각한 선택이다(수동 `indentUsingSpaces`/`indentUsingTabs`/`detectIndentation` 과 싸운다).
  좁히려면 "전역값과 정확히 같아졌을 때만 재확정" 같은 조건이 필요한데, 사용자가 전역값과 같은 값을
  수동으로 고른 경우와 구분되지 않는다. 실사용 영향은 스플릿 + 대형 파일 여닫기라는 조합에 한정된다.
- **오버라이드를 거두는 방향이 없다(검토 #2, 미수정 — 문서만 정정)**: `resolveEditorConfigModelIndent`
  가 두 축 모두 `null` 이면 모델을 건드리지 않는다(적용 방향에는 맞다). 실파일 모델은 탭을 닫아도
  dispose 되지 않으므로(`model-registry.ts` — `disposeModel` 호출처는 untitled 2곳과 rename/delete
  경로뿐), `.editorconfig` 에서 섹션을 지우거나 `editorConfigEnabled` 를 꺼도 **탭을 다시 열어도**
  이전 오버라이드가 남는다(앱 재시작 전까지). 되돌리기를 구현하려면 "직전에 오버라이드가 있었고 지금
  없다"는 전이에서 전역 `tabSize`/`insertSpaces`(또는 `detectIndentation` 이면 `model.detectIndentation`)
  를 다시 걸어야 하는데, 그 순간부터 이 effect 는 "적용"만이 아니라 "복원"도 하는 것이 되어 수동
  들여쓰기 명령·페인 간 파일 전환과의 상호작용을 새로 규정해야 한다. 계약 §1 U3 의 범위(적용)를 넘으므로
  이월하고, `editor.md` §17 · `ipc-contract.md` 의 "다음에 그 파일을 열 때부터" 문장만 사실에 맞게
  정정했다.
- **untitled 탭의 Save As 는 on-save 정리를 타지 않는다(검토 #6)**: `untitled-pane.tsx` 의
  `handleSaveAs` 는 `useSaveFile` 을 직접 부르고 `runOnSaveCleanup` 을 거치지 않는다. 동작은 바꾸지
  않았다 — untitled 은 format-on-save 도 원래 타지 않고 `.editorconfig` 대상도 아니라(§5 U3) 정리
  규약을 적용할 파일 맥락 자체가 없다. §1 U2 의 "수동 ⌘S" 를 문자 그대로 읽으면 포함되므로, 적용 범위
  밖 목록(`editor.md` §16)에 명시해 기록과 실제를 맞췄다.
- **format-on-save 단계의 같은 형태는 손대지 않았다(검토 #3 인접)**:
  `use-editor-file-persistence.ts` 의 `formatAction.run().catch(() => undefined)` 도 동기 throw 를
  놓치는 같은 모양이지만, d-53 이 만든 코드가 아니라 기존 format-on-save 경로다(최소 diff — 자기 항목
  외 무관 코드 수정 금지). 고칠 때는 이번 `runCleanupAction` 과 같은 `try`/`catch` 형태면 된다.
- **글롭 매처의 남은 비용(검토 #4 수정 후)**: 메모이제이션으로 상태 수는 (노드 수 × 입력 길이)로
  묶였지만, `Alternation` 은 여전히 방문할 때마다 분기+나머지를 새 `Vec` 으로 스플라이스한다. 최악은
  (교대 노드 수 × 입력 길이 × 패턴 길이)의 포인터 복사로, 상한(패턴 1024자) 안에서는 유한하지만
  상수는 작지 않다. 없애려면 매처를 명시적 continuation(또는 명령 리스트) 표현으로 바꿔야 해서, 이번
  수정 범위(지수 → 다항)에서는 두었다.
- **`TextField` 의 blur 마다 커밋하는 동작은 그대로다**: §5 U1 에 적은 대로 값이 같아도
  `settings_update` 가 한 번 나간다. 이번에 추가한 `normalize` 는 **화면 텍스트**를 정규화 결과로
  되돌려 쓸 뿐, "저장값과 같으면 IPC 생략"(numeric-field 의 두 번째 성질)은 넣지 않았다 — 그것은
  `TextField` 전 사용처의 동작 변경이라 여전히 이 범위 밖이다.
