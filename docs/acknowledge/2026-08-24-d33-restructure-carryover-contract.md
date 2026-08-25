# d-33 — T2-C shared/lib 재구조화 + 이월 소형 일괄 계약 (2026-08-24)

> 정본: `2026-08-21-batch-consolidation-decision.md` §2 d-33 / 감사 T2-C(:348)·C7(:210) /
> 이월 원문: d-25 계약(FILE.RAW)·d-29 계약 §4.5 design-1(Switch)·d-26 계약 §3.4
> (workspaceSymbol 하이라이트)·X-A 계약 §1.3(T1 3차 소형)·d-31 계약 §5(테마 기술 이월).
> 운영: /goal — 커밋·푸시 없음, 결정은 추천안 자체 채택·기록.

## 0. 착수 전 메인 실사 (2026-08-24)

- **T1 3차 이월 소형분 실사 결과 — 전부 기처리**: (2) open-with 생명주기(useCloseTab
  prune+projectClosed prune, `open-with-registry.ts` JSDoc 명시) / (3) ws.rs writer 유한
  대기(`remote/ws.rs:156·242-245` bounded shutdown 구현) / (7) TREE_ROWS 센티널(Rust
  `rows_page(limit: Option<u32>)` 전환 완료·TS 상수 소멸 — 단 `domain/tree/service.rs:177`
  doc 이 존재하지 않는 프론트 상수 `TREE_ROWS_UNBOUNDED_LIMIT` 를 여전히 언급, **낡은 포인터
  1줄만 잔존**) / (8) PROJECT_SCOPED_KEYS 경로 predicate(`isQueryKeyUnderProjectRoot` 존재).
  → d-33 몫은 (7) 의 doc 정정 1줄뿐.
- **shared/lib 평면**: 하위 디렉토리 ai·lsp·monaco·pdf·remote·shiki·theme-convert 기존.
  평면에 monaco 계열 5(+test 1)·keymap 계열 8(+tests)·keybinding 2·bridge 계열 다수(`*-bridge`)
  잔존 — 감사 C7 "monaco/keymap/bridge 가 하위 디렉토리와 이원화" 그대로.
- **"shared UI 3분할 정리"**(감사 T2-C 행 문언)는 감사 본문에서 원 발견을 특정하지 못함 —
  구현이 감사 정본에서 원 항목을 실사·특정한 뒤 진행, 특정 불가 시 **보류·기록**(모호 문언
  임의 확대 금지).
- FILE.RAW: d-25 계약 §3 이 근본 수정 방향까지 명시(`ipc-sync-provider.tsx` `fsChanged`
  핸들러에 `FILE.RAW` invalidate 추가). 소비처는 `preview-pane.tsx`(이미지·PDF 등) 전용.
- Switch 26회 중복: d-29 원 리뷰 제안 = `features/settings/switch-field.tsx`(또는
  AgentHooksToggle 일반화) 추출.
- workspaceSymbol 하이라이트: LSP 결과에 fuzzy 인덱스 없음 — 표시 측에서 검색어로
  `fuzzyMatch` 재실행해 인덱스 확보하는 로컬 매칭 설계가 자연스러움(구현 확정·기록).
- d-31 테마 이월 기술분(§5): ① matchHighlight vs 본문 전경(`app.foreground`) 구별성 가드 —
  **ΔE(Lab) 기준**(WCAG 대비비 금지 — github-dark ΔE 77.7 인데 대비비 1.03 오탐 근거), 임계는
  36종 실측 분포(0/5.4/7.2/13.8…) 간극 근거로 설정 ② 번들 3종(monokai·night-owl-light·
  palenight) 동일색 데이터 업스트림 스케일 내 정정 ③ 재스윕에 vs-전경 축 추가 기록.
  **builtin_light 색·대비 린트 신설 정책은 사용자 결정 필요 — 이월 유지**(적대적 L2-3 판정).

## 1. 범위 (추천안 자체 채택)

- **a. T2-C 재구조화**: `shared/lib/bridge/`·`shared/lib/keymap/` 신설 이동(각 계열 전 파일
  +test 동행·kebab 이름 유지), monaco 평면 5파일 → `shared/lib/monaco/` 합류(monaco-internal
  .d.ts 포함 여부는 tsconfig 영향 실사), `theme/` 신설은 실사로 유효분 판정(theme-convert
  기존과의 경계 — 억지 신설 금지). 소비처 import 전수 갱신(경로만). barrel 금지. "shared UI
  3분할"은 §0 대로 특정 후 진행 or 보류.
- **b. FILE.RAW 무효화**: `fsChanged` 핸들러에 `QUERY_KEY.FILE.RAW` invalidate 추가(경로
  스코프는 기존 FILE.CONTENT 처리와 동일 패턴)·테스트(ipc-sync-provider.test 관행).
- **c. Switch 행 프리미티브**: 순수 컴포넌트 추출(`features/settings/switch-field.tsx` 기본,
  실사로 명명·경계 확정)·settings 섹션 12파일 중 해당 26회 교체. 렌더 동등성 유지.
- **d. workspaceSymbol 하이라이트**: 로컬 fuzzyMatch 인덱스로 `HighlightedText` 연결(팔레트
  d 그룹 파일 — d-31 산출 `command-palette-workspace-symbol-group.tsx`). LSP 결과 필터링은
  불변(표시 강조만).
- **e. 테마 이월 기술분**: §0 의 ①②③ — mapping-tables derived 에 ΔE 구별성 가드 추가(전경
  후보가 본문 전경과 지각 동일이면 배제→safe-default), 번들 3종 데이터 정정, d-31 계약 §5 에
  재스윕 표(vs-전경 축) 추가 기록. Rust 린트 확장(matchHighlight != app.foreground)은 데이터
  정정과 정합 확인 후 판단·기록.
- **f. 소형**: `domain/tree/service.rs:177` 낡은 doc 포인터 정정(1줄).
- 범위 외: builtin_light·대비 린트 정책(사용자 결정)·fuzzy 가중치 재설계·AppError(d-34)·
  Rust 하드닝(d-35).

## 2. 실행·검증

- 구현: T2-C(a — 기계적 대량)·소형 묶음(b+c+d — TS)·테마(e — TS+데이터) 병렬 3 + Rust 조각
  (e 린트+f — **d-32 Rust 완료 후 단독 순차**). 각자 §3 기록.
- 검토 3렌즈(opus+xhigh): ① 동등성·소비처 전수(a 이동·c 렌더 동등) ② 테마 정합(e — ΔE
  수학·데이터·재변환 재현성과의 관계는 d-31 §4 L2-0 결론 승계) ③ 캐시·FSD(b 무효화 범위
  적정·과대/과소, a 레이어 정합). major 적대적 → 수정 → 메인 2차(verify+vite). 커밋 없음.

---

## 3. 구현 완료 기록 — TS 축 (2026-08-24)

### 3-A. T2-C shared/lib 재구조화

### 임무 A — T2-C `shared/lib` 재구조화 (계약 §1-a) 실행 기록

### 1. 실사 확정 (분류)

`src/shared/lib` 평면 파일 전수(당시 130여 개)를 대상으로 세 계열을 확정했다.

| 계열 | 기준 | 파일 수 | 목적지 |
|---|---|---:|---|
| ① bridge | `*-bridge.ts`(+test), 단 `keybindings-bridge`는 keymap 소관으로 제외 | 13 base × 2 = **26** | `shared/lib/bridge/`(신설) |
| ② keymap | `keymap*.ts`(+test) 7 base(`keymap`·`-capture`·`-category`·`-chord-store`·`-context`·`-dispatch`·`-when`) + `keybinding-catalog`·`keybindings-bridge` 2 base | **16** (+test 포함) | `shared/lib/keymap/`(신설) |
| ③ monaco | `monaco-action-commands`·`monaco-actions`·`monaco-keybinding-runtime`·`monaco-keybinding`(+test) | **5** | 기존 `shared/lib/monaco/`(4파일 기존)에 합류 |
| ③′ monaco-internal.d.ts | 앰비언트 `.d.ts` 1개 | **1** | 실사 후 `shared/lib/keymap/`로 판정(아래 §2) |

합계 이동 파일 **48개**. `keymap` 계열 base-name 카운트는 계약 §0 의 "keymap 계열 8·keybinding 2"(대략치)와 실사치(7+2=9)가 다르다 — **실제 파일이 단일 출처**(ai-process §6.1)라는 원칙에 따라 실사치를 채택했고, 이 정정을 여기 기록한다.

### 2. 이동 판단 근거

- **`monaco-internal.d.ts` → `keymap/`(monaco/ 아님)**: tsconfig `include: ["src"]`가 디렉토리 전체를 포괄하므로 앰비언트 `.d.ts`는 어디로 옮겨도 타입체크에 영향이 없다(명시적 import/reference 없음, 전수 grep 확인). 유일한 실사용 소비 관계는 `keymap-when.ts`(딥임포트 대상 타입 제공)뿐이고, 파일 자체 JSDoc이 "`src/shared/lib/lsp/monaco-internal.d.ts`(LSP 소비 도메인)와 별개 파일로 둔 이유 = 다른 소비 도메인(keymap when 평가)이기 때문"이라고 **기존 선례를 이미 자체 설명**하고 있다. 그 선례(소비 도메인 기준 배치, `monaco`라는 문자열 기준이 아님)를 그대로 적용해 `keymap/`으로 이동했다. 이동 후 파일의 JSDoc 본문은 여전히 정확해 추가 수정이 필요 없었다.
- **`theme/` 신설 보류**: 평면에 `theme-draft`·`theme-variables`·`vsix-theme-import`·`xterm-theme`(4 base, 7파일)가 남아 있으나, 감사 C7 항목("`shared/lib` 루트 — monaco/keymap/bridge 가 하위 디렉토리와 이원화")이 지목한 결함은 **동일 이름의 평면 파일과 하위 디렉토리가 동시 존재하는 이원화**다. 이 4파일은 `theme-convert/`(변환 파이프라인: ansi-palette·contrast·convert·jsonc·mapping-tables·merge·resolve-*·semantic-token-map·types·ui-token-vocabulary·validate-completeness)와 이름이 하나도 겹치지 않고 관심사도 다르다(초안 상태·CSS 변수 도출·VSIX 추출·xterm 매핑 vs 변환 알고리즘). 즉 감사가 지목한 "이원화" 신호가 이 4파일에는 실재하지 않는다 — `theme/` 신설은 **억지 신설**에 해당해 보류하고, 이 파일들은 평면에 남겼다.
- **"shared UI 3분할 정리"(감사 T2-C 행 문언) 특정 불가 → 보류**: 감사 정본(`docs/quality-assurance/2026-08-18-architecture-audit.md`) 및 원 배치 상세(`...-batch-summaries.md`, `...-raw-R4-X1.md`)를 전수 검색했다. C7 표의 `shared/lib` 루트 행(1건)과 F6 총평의 "107파일 평면 배치"(F6#12, 발견 비용 문제)가 T2-C 해소 건수 "2" 중 1건에 대응하는 것은 명확하나, "shared UI 3분할"에 정확히 대응하는 원 발견을 특정하지 못했다. 근접 후보(F7#9+F2#7의 `file-group-header.tsx` 이중 결함 "한 번에 처리 가능", F7 배치의 shadcn 벤더 예외 판정에서 언급된 "자작 3종(icon-button·file-group-header·status-error-banner)")는 모두 "3분할"(재구조화·분리)이 아니라 "이미 존재하는 개별 이슈의 통합 처리"에 관한 서술이라 문언과 부합하지 않는다. **d-33 계약 §0의 기존 결론(특정 불가)과 동일하게 재확인**했으며, `shared/ui/`에는 아무 이동도 하지 않았다. 모호 문언 임의 확대 금지 원칙에 따라 그대로 보류.

### 3. 소비처 전수 갱신

계열별로 나눠 진행(전수 grep 확인 완료):

- bridge: 13개 base × 각 소비처(단, 이동 대상 자기 자신 상호참조 포함) 전수 갱신 — 외부 소비처 예: `editor-area.tsx`·`command-palette.tsx`·`app-shell.tsx`·`explorer-container.tsx`·`command-catalog.ts`·entities 6종 commands.ts 등.
- keymap: `keymap-category`가 `command-catalog.ts` + entities 6종(`terminal`·`agent`·`ai`·`task`·`sync`·`git`).commands.ts + `git.commands.test.ts`에서 소비, `keymap`(base)이 `features/settings/keybinding-row.tsx`·`features/welcome/welcome-screen.tsx`·`features/command-palette/command-palette-commands-group.tsx`·`app/providers/keybindings-runtime-provider.tsx`·`widgets/*` 다수에서 소비 — 전부 `@shared/lib/keymap/...`로 갱신.
- monaco: `monaco-actions`·`monaco-keybinding`이 `keybinding-catalog.ts`(keymap/)에서 교차 소비 — `@shared/lib/monaco/...`로 갱신. `monaco-action-commands`는 `app/bootstrap-commands.ts`에서 소비.
- **병행 임무 경계 준수**: `settings-keymap-section.tsx`(임무 C의 settings 섹션 파일)는 `keybindings-bridge` import 경로 1줄만 교체했다(diff 2줄, 내용 무변경). `keymap-category.ts`는 git mv 직전 `git status`로 재확인해 병행 수정이 없음을 검증한 뒤 이동했다.
- **관련 문서(살아있는 레퍼런스만) 갱신**: `docs/architecture.md`(2곳)·`docs/features/lsp.md`(1곳)·`docs/features/tasks.md`(1곳)·`docs/features/keymap.md`(2곳, 5개 경로)의 낡은 경로 포인터를 새 경로로 정정했다. `docs/acknowledge/*`·`docs/history/*`는 특정 시점의 합의·이력 기록이므로 의도적으로 미수정(과거 시점 기준 서술이 맞다).

### 4. 순환 의존 점검 (신규 순환 0)

디렉토리 간 참조 방향(파일 단위 grep 재확인):

- `bridge/` → (keymap/·monaco/ 어느 쪽도 참조하지 않음, `lsp/monaco-types`만 참조) — **최하위 기반**.
- `keymap/` → `bridge/`(`keybindings-bridge.ts` → `fire-and-forget-bridge`), `keymap/` → `monaco/`(`keybinding-catalog.ts` → `monaco-actions`·`monaco-keybinding`).
- `monaco/` → `bridge/`(`monaco-action-commands.ts` → `editor-pane-command-bridge`), `monaco/` → `keymap/`(`monaco-actions.ts`·`monaco-keybinding.ts`·`monaco-keybinding-runtime.ts` → `keymap.ts`/`keymap-category.ts`).

디렉토리 수준으로는 `keymap/` ↔ `monaco/`가 양방향이지만, **파일 단위로는 순환이 없다**: `keybinding-catalog.ts → monaco-actions.ts/monaco-keybinding.ts → keymap.ts/keymap-category.ts`로 끝나는 단방향 DAG이고, 역방향(예: `keymap-category.ts`가 `keybinding-catalog.ts`를 다시 참조)은 존재하지 않는다. 이 관계는 이동 전에도 동일하게 존재했던 기존 의존 구조를 그대로 옮긴 것이며, 이번 재구조화로 **신규 순환은 0건**이다.

### 5. 검증

- `bun run typecheck` — **통과**(에러 0).
- `bunx prettier --check <변경분 55개 ts/tsx>` — 1건(`bridge/active-editor-actions-bridge.test.ts`, import 경로가 길어져 150자 printWidth 초과)만 `--write`로 재포맷(내용 변경 없이 줄바꿈만), 이후 **전체 통과**.
- `bun test`(전체 스위트) — **1459 pass / 0 fail** (관련 경로만 먼저 323 pass 확인 후 전체 재실행으로 회귀 없음 재확인).
- `bunx eslint <변경분>` — **통과**(exit 0, 위반 없음).

### 6. 미해결/이월 사항

- "shared UI 3분할 정리"는 §2에서 서술한 대로 원 발견 특정 불가로 계속 보류 — 사용자가 원 문언의 소스를 알고 있다면 재확인 후 처리 필요.
- `theme/` 미신설 — 4개 평면 theme 파일(`theme-draft`·`theme-variables`·`vsix-theme-import`·`xterm-theme`)은 감사 근거 부재로 그대로 둠. 추후 실질적 발견(예: 새 감사)이 나오면 재검토.
- `docs/acknowledge/2026-08-16-monaco-contextkeyexpr-deep-import.md` 등 point-in-time 기록은 옛 경로(`shared/lib/keymap-when.ts` 등)를 그대로 담고 있음 — 의도적 미수정(과거 기록 보존 원칙).

**openIssues**:
- 'shared UI 3분할 정리'(감사 T2-C 행 문언)의 원 발견을 감사 정본·원 배치 상세 전수 검색으로도 특정하지 못함 — 계속 보류. 사용자가 원문 소스를 특정할 수 있으면 재확인 필요.
- theme/ 디렉토리는 신설하지 않음 — theme-draft·theme-variables·vsix-theme-import·xterm-theme 4파일이 평면에 남아 있으나 감사 근거(이름 이원화) 부재로 보류한 판단. 추후 별도 감사에서 실 발견이 나오면 재검토 필요.
- docs/acknowledge/2026-08-16-monaco-contextkeyexpr-deep-import.md 등 point-in-time 기록은 구 경로(shared/lib/keymap-when.ts 등)를 그대로 담고 있음 — 의도적 미수정.

### 3-B. 소형 이월 3건 (FILE.RAW·SwitchField·wsSymbol 하이라이트)

### d-33 임무 B — 소형 이월 3건 (계약 §1-b·c·d)

### b. FILE.RAW 무효화 (d-25 이월)

- **대상**: `src/app/providers/ipc-sync-provider.tsx`(`fsChanged` 핸들러) + `ipc-sync-provider.test.ts`.
- **판단 근거**: d-25 계약(`2026-08-20-boot-watcher-defer-contract.md` :191-202) 실사 결과 `QUERY_KEY.FILE.RAW` invalidate 호출부가 코드베이스 전체에서 `fileRawQueryOptions` 자신과 테스트뿐임을 재확인 — `useSaveFile`/`useRenameEntry`/`useCopyEntry`/`useDeleteEntry` 어디에도 `onSuccess` 로 FILE.RAW 를 무효화하는 경로가 없어(`entities/file/file.query.ts` 전량 읽음), `fsChanged` 워처 에코가 FILE.RAW 의 유일한 갱신 경로임을 확인했다(FILE.CONTENT 는 최소 `useSaveFile` 등 일부 경로가 있는 것과 대조적으로 더 강한 근거).
- **변경**: `fsChanged` 루프를 `for (const path of change.paths) { void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) }) }` 에서, 신규 순수 함수 `filePathQueryKeysToInvalidate(path) = [QUERY_KEY.FILE.CONTENT(path), QUERY_KEY.FILE.RAW(path)] as const` 를 순회하는 `for (const queryKey of filePathQueryKeysToInvalidate(path)) void queryClient.invalidateQueries({ queryKey })` 로 교체. 이 헬퍼는 이 파일의 기존 관행(`isQueryKeyUnderProjectRoot`·`isSelfEchoWithoutTreeImpact`·`isStaleLayoutRevision` 등 — 테스트 가능성을 위해 순수 로직을 top-level export 로 분리하는 패턴)을 그대로 따른 것이다.
- **게이팅**: `isSelfEchoWithoutTreeImpact` 게이트(트리 refresh 스킵 조건)는 FILE.CONTENT/FILE.RAW 무효화 루프보다 아래(트리 refresh 직전)에 있어 이번 변경으로 게이팅 범위에 편입되지 않는다 — 원래 FILE.CONTENT 처리와 완전히 동일한 위치·조건으로 FILE.RAW 를 추가했다(요청 사양대로 "경로 스코프는 기존 FILE.CONTENT 처리와 동일 패턴").
- **JSDoc 갱신**: 바로 아래 있던 "`FILE.CONTENT` invalidation 이 `fromApp` 게이팅에서 의도적으로 제외된 이유" 주석이 이번 수정으로 그대로 두면 낡은 서술이 되어(FILE.RAW 도 이제 같은 루프에서 무게이팅으로 처리되는데 문서가 FILE.CONTENT 만 언급) `comments.md` §1.2(수정하는 코드에 붙은 낡은 주석은 갱신)에 따라 FILE.RAW 문단을 추가해 갱신 — FILE.RAW 는 FILE.CONTENT 보다 더 강한 근거(어떤 `onSuccess` 무효화도 전혀 없음)를 명시.
- **테스트**: `ipc-sync-provider.test.ts` 에 `filePathQueryKeysToInvalidate` describe 블록 1개(경로 1개 → `['file','content',path]`+`['file','raw',path]` 2건 반환 검증) 추가 — 파일의 기존 테스트 관행(순수 함수를 `await import(...)` 로 직접 불러 단정)을 그대로 따름. `fsChanged` 핸들러 자체는 이 파일의 다른 IPC 핸들러들과 마찬가지로 컴포넌트 내부 인라인 콜백이라 렌더링 기반 테스트 인프라가 이 코드베이스에 전무함을 확인(`grep -rl useTauriEvent *.test.*` 무결과) — 새 인프라를 만드는 대신 무효화 대상 산출 로직만 순수 함수로 분리해 테스트 가능하게 만드는 것으로 관행을 지켰다.
- **소비처**: `src/widgets/preview-pane/preview-pane.tsx` 전문 확인 — `useQuery({ ...fileRawQueryOptions(path), enabled: needsRawBytes })` 로 §4 관행(`queryOptions` 팩토리 재사용) 그대로 소비 중이며, `invalidateQueries` 는 활성 쿼리를 자동 refetch 하므로(v5 표준) 이 파일은 **무수정**(계약 예상과 일치).

### c. Switch 행 프리미티브 (d-29 이월)

- **실사**: `grep -rn "<Switch" src/widgets/settings-view/*.tsx` → 정확히 26건, 4파일(appearance 1·interface 8·editor 16·terminal 1)에 분포. 12개 섹션 파일 중 Switch 를 실제로 쓰는 곳은 이 4곳뿐이며(나머지 8개 섹션엔 Switch 행이 없음), 계약 문언의 "섹션 12파일"은 settings-view 섹션 총 개수(TOC 12항)를 가리키는 표현으로 해석해 실제 교체 대상은 이 4파일로 확정.
- **패턴 실사**: 26건 전부가 정확히 두 변형으로 수렴함을 확인.
  1. 라벨만: `<label class="flex items-center justify-between gap-3 text-xs"><span class="text-app-foreground">{label}</span><Switch .../></label>`
  2. 라벨+설명: 위 `<span>` 이 `<span class="flex flex-col gap-0.5">` 로 감싸이고 그 안에 `text-app-foreground` 라벨 span + `text-app-sidebar-icon-default` 설명 span 이 들어감.
  모든 호출부가 `checked={settings.X ?? default} onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), X: checked })}` 동일 배선.
- **AgentHooksToggle 일반화 검토 후 기각(근거)**: `features/settings/agent-hooks-toggle.tsx` 를 확인한 결과 (1) hint 유무와 무관하게 항상 `<span class="flex flex-col gap-0.5">` 래퍼를 렌더해 "라벨만" 변형(래퍼 없음)과 렌더 트리가 다르고, (2) hint span 클래스가 `text-app-sidebar-icon-default flex flex-col gap-0.5` 로 26건의 설명 span(`text-app-sidebar-icon-default` 단독)과 클래스가 다르다. 계약이 요구하는 "렌더 트리 동등(클래스·구조·접근성 라벨) 유지"를 26건 그대로 만족시키려면 AgentHooksToggle 자체의 구조를 바꿔야 하는데, 이는 그 컴포넌트의 유일한 기존 소비처(interface-section 의 agentHooks 행, 요청 범위 밖)까지 불필요하게 건드리는 확장이라 판단해 기각. FSD 결정 트리(§2.1) 상으로도 "도메인 무관 범용 UI"는 `shared` 승격보다 먼저 `features` 신설이 자연스러워 기본안(`features/settings/switch-field.tsx`)을 그대로 채택.
- **구현**: `SwitchField({ label, checked, onCheckedChange, description? })` — `description` 유무로 위 두 렌더 분기를 그대로 재현(클래스·구조 100% 동일, `disabled` 등 26건에 없는 prop 은 추가하지 않음 — 조기 확장 금지). `label`/`description` 타입은 같은 디렉토리의 형제 프리미티브 `numeric-field.tsx` 관행을 따라 `string`(모든 실제 호출부가 `t()` 결과인 string).
- **교체**: 4파일 전량 — appearance 1·interface 8·editor 16·terminal 1 = 26건, `grep -c "<SwitchField"` 로 26 재확인, `<Switch\b` 잔존 0·`@shared/ui/switch` import 잔존 0 확인. i18n 키(`t('settings.*')`) 는 손대지 않고 그대로 전달(요청 사양).
- **관찰(범위 외, 기록만)**: `features/settings/ai-auto-tab-toggle.tsx` 도 동일한 "라벨+설명" 렌더 구조(클래스까지 일치)를 별도로 갖고 있어 SwitchField 와 잠재적으로 중복이지만, 이 컴포넌트는 26건에 속하지 않는 별도 소비처(도메인 로직 포함·disabled 분기 내장)라 이번 배치 범위 밖 — 후속 정리 후보로만 기록.

### d. workspaceSymbol 하이라이트 (d-26 이월)

- **대상**: `src/features/command-palette/command-palette-workspace-symbol-group.tsx` + 소비처 `src/widgets/command-palette/command-palette.tsx`.
- **원 계약 재확인**: d-26 계약(`2026-08-20-palette-ux-contract.md` §3.4) 은 `HighlightedText` 를 당시 `command-palette.tsx` 의 비export 지역 컴포넌트로 기록했으나, 실사 결과 이후(다른 배치에서) `features/command-palette/highlighted-text.tsx` 로 이미 승격·export 되어 `commands`/`symbol`/`files` 3개 그룹이 전부 이를 공용 소비 중임을 확인 — 이번 구현은 그 이미 정착된 패턴을 그대로 따름(신규 컴포넌트 생성 없음).
- **설계**: `workspaceSymbolSearch.search()`(LSP) 결과인 `NormalizedWorkspaceSymbol[]` 에는 `FuzzyMatch.indices` 가 없다. `command-palette.tsx` 가 이미 갖고 있는 `searchTerm`(로컬 `useState` 파생값, `workspaceSymbolResults` 자체가 `workspaceSymbolState.query === searchTerm` 로만 노출되어 현재 검색어와 항상 동기화 보장됨)을 새 prop 으로 `CommandPaletteWorkspaceSymbolGroup` 에 전달하고, 컴포넌트 내부에서 심볼별로 `fuzzyMatch(searchTerm, symbol.name)` 을 재실행해 인덱스를 확보 — `command-palette-files-group.tsx` 의 기존 관행(2단 렌더용 로컬 파생값을 `.map` 블록 바디에서 계산)과 동일 스타일.
- **매치 실패 폴백**: `fuzzyMatch` 가 `null` 을 반환하면(서버가 `containerName` 매칭 등 로컬 fuzzy 로 재현 불가한 기준으로 그 심볼을 반환한 경우) `HighlightedText` 를 거치지 않고 `symbol.name` 원문 그대로 렌더 — 원래(이번 변경 이전) 동작과 완전히 동일한 출력이라 회귀 없음.
- **불변 확인**: LSP 검색·필터링(`useWorkspaceSymbolSearch`, `workspaceSymbolsLoaded`/`workspaceSymbolResults` 산출)은 무수정 — 표시 측 강조 로직만 추가.
- **JSDoc**: 인덱스가 없는 이유·로컬 재매칭의 한계(서버 기준과 다를 수 있음)·실패 시 폴백 근거를 컴포넌트 위에 영어 JSDoc 으로 기록(이 코드베이스의 기존 관행 — `ipc-sync-provider.tsx` 등 비자명한 설계 판단에 상세 JSDoc 을 남기는 패턴을 따름).

### 검증

`bun run typecheck`(tsc --noEmit) 클린 → `bunx prettier --check` 변경분 9파일 전부 통과 → `bun test src/app/providers/ipc-sync-provider.test.ts`(23 pass) + `bun test src/shared/lib/fuzzy-match.test.ts`(19 pass) = 42 pass/0 fail → `bunx eslint` 변경분 9파일 클린. `widgets/command-palette`·`widgets/settings-view` 레이어는 d-29 §4.6 선례대로 기존부터 테스트가 없어 신규 테스트를 추가하지 않았다(무변경 확인). Rust/cargo·`bun run verify`·`vite build` 는 메인 몫으로 미실행. 병행 임무 A 의 shared/lib 재구조화 영향(예: `command-palette.tsx` 내 `@shared/lib/bridge/*`·`@shared/lib/keymap/*` 로의 기존 경로 변경)을 typecheck 시점에 반영해 확인했으며, 내가 새로 추가한 `@shared/lib/fuzzy-match` import 는 A 가 아직 이동하지 않은 경로로 typecheck 클린 통과.

**openIssues**:
- features/settings/ai-auto-tab-toggle.tsx 가 SwitchField 의 '라벨+설명' 변형과 렌더 구조·클래스가 동일해 잠재적 중복이나, 26건 대상 범위 밖이라 이번 배치는 미수정(후속 정리 후보로만 기록).
- entities/search 의 useReplaceSearch 가 FILE.CONTENT/FILE.RAW 어느 쪽도 onSuccess invalidation을 갖지 않는 근본 공백은 d-25 계약이 이미 '별도 배치·entities/search 소유'로 명시한 범위 밖 — 이번에도 손대지 않음(기존 워처 에코 경로로 커버됨, 문서화만 갱신).
- 임무 A(shared/lib 재구조화)가 병행 진행 중이라, 이 세션 종료 시점 이후 A 가 fuzzy-match.ts 등을 이동하면 이번 변경분(command-palette-workspace-symbol-group.tsx 의 @shared/lib/fuzzy-match import)의 typecheck 재확인이 필요할 수 있음 — 메인 2차 검증(verify+vite) 시점에 재확인 권장.

### 3-C. 테마 이월 기술분 (ΔE 구별성 가드·번들 3종 정정·재스윕)

#### §3-C 상세 (e — 임무 C, d-31 §5 승계 — 초판 "3-E" 번호는 검토 L3-6 정정: 코드·theme-system.md 의 §"임무 C" 참조는 이 절)

#### ① matchHighlight vs 본문 전경 구별성 가드

**ΔE 순수 함수 신설 위치**: `src/shared/lib/color.ts` — `deltaE76(hexA, hexB)`. sRGB(D65) → 선형화 →
XYZ → CIE L\*a\*b\* → 유클리드 거리(CIE76) 순수 함수. 선형화 임계값(0.04045)은 `contrast.ts`
의 WCAG 상대휘도용 임계값(0.03928)과 **의도적으로 다른 상수**로 분리했다(용도가 다름 — JSDoc
에 명시). 매트릭스·엡실론/카파 상수는 CIE 15:2004/Bruce Lindbloom 표준 유도값이라 매직넘버가
아니라 스펙 상수로 JSDoc 근거를 달았다.

**CIE76 충분성 검증**: 번들 36종 `panel.matchHighlight` vs `app.foreground` 전수에 적용한 결과
동일색 3종(0.0) → one-monokai(5.39) → vscode-kimbie-dark(7.16) → 나머지 31종(13.84+) 로 깨끗한
간극이 나왔다(적대적 검토 L2-2 의 0/5.4/7.2/13.8 실측과 소수점 단위로 일치, github-dark
`#ffd33d` vs `#d1d5da` 재계산 77.73 도 검토 수치 77.7 과 일치). CIE94/CIEDE2000 같은 지각
보정판이 필요할 만큼 애매한 경계값이 없어 **CIE76 으로 충분**하다고 판단·기록했다.

**임계값**: `mapping-tables.ts` 의 `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E = 2.3`. 위 간극(0 →
5.39) 안에 위치하며, 색차 문헌에서 널리 인용되는 CIE76 JND(just-noticeable-difference, 약
2.3) 값과 일치해 "끼워 맞춘 값"이 아니라 독립적으로 근거가 있는 상수임을 JSDoc 에 기록했다.
현재 카탈로그 기준으로는 (0, 5.39) 구간 안 어떤 값을 골라도 결과(배제 대상 3종)는 동일하다.

**가드 구현**: `isDistinctFromBodyForeground(candidateHex, bodyForegroundHex)` (mapping-tables.ts,
export — `isOpaqueForegroundCandidate` 와 동일한 노출 수준). `panel.matchHighlight` 의
`derived()` 를 `.filter(isOpaqueForegroundCandidate).find(candidate => isDistinctFromBodyForeground(candidate, ctx.resolved['app.foreground']))`
로 확장했다. **`ctx.resolved['app.foreground']` 접근 가능성 실사**: `COLOR_MAPPING` 배열에서
`app.foreground` 는 2번째 엔트리(`app.background` 바로 다음)이고 `panel.matchHighlight` 는
그보다 한참 뒤에 위치 — `resolveColors` 가 배열을 순서대로 순회하며 각 엔트리 처리 직후
`ctx.resolved[key]` 에 **항상 확정값**(safe-default 폴백 포함)을 채워 넣으므로, matchHighlight
derive 실행 시점에 `app.foreground` 는 이미 확정된 문자열이다(별도 `ctx.vscodeColors['foreground']
?? ctx.vscodeColors['editor.foreground']` 우회 계산 불필요 — 오히려 확정 resolved 값을 쓰는
쪽이 family-fallback 을 거친 실제 렌더값과 정확히 일치해 더 정확하다). WCAG 대비비가 아니라
ΔE 를 쓴 이유(github-dark ΔE 77.7 vs 대비비 1.03 오탐)를 두 함수 JSDoc 에 모두 기록했다.
배제 시 기존 경로(두 후보 소진 → `resolveColorEntry` 의 `status` 카테고리 폴백 → `SAFE_DEFAULT_COLORS`)
로 그대로 떨어진다 — 새 폴백 경로를 만들지 않았다.

**테스트**: `color.test.ts` 에 `deltaE76` 5건(항등 0·흑백 100·github-dark 저대비-고ΔE 사례·
36종 분포 대표값 4개·유효하지 않은 hex → null), `mapping-tables.test.ts` 에
`isDistinctFromBodyForeground` 3건(동일색 배제·one-monokai ΔE5.4 는 배제 안 함·WCAG 오탐
사례 배제 안 함), `convert.test.ts` 에 실제 파이프라인 신규 3건 + 기존 1건 무회귀 확인(초판 "통합 4건"은 검토 L3-4 정정 — monokai/night-owl-light 실사례
dark·light 각 1건 safe-default 폴백 확인 + one-monokai 실사례 1건 배제 안 됨 확인 + 기존
"list.highlightForeground 불투명이면 우선 사용" 케이스 무회귀 확인).

#### ② 번들 3종 데이터 정정

업스트림 원본을 재취득(`microsoft/vscode` main/theme-monokai, `whizkydee/vscode-palenight-theme`
master, `sdras/night-owl-vscode-theme` master — `docs/utils/2026-08-12-w7-theme-original-sources.md`
매니페스트의 등재 경로 그대로)해 세 테마 모두 `list.highlightForeground`(또는 그 상위 `foreground`)
가 **업스트림 자체에서** `app.foreground` 와 동일하게 정의돼 있음을 확인했다(우리 변환기의
오작동이 아니라 업스트림 설계). `editor.findMatchHighlightBackground` 는 palenight(`#7e57c233`)·
night-owl-light(`#93a1a16c`) 는 반투명이라 기존 f-1 불투명 가드가, monokai 는 upstream 이 이
토큰을 아예 정의하지 않아 후보 자체가 없어, 새 가드가 없어도 이미 폴백 경로로 향하고 있었다
(새 가드는 "동일색으로 잘못 채택되는" 1차 후보만 추가로 막는다).

각 업스트림의 **같은 테마 `tokenColors`**(반복 사용되는 강조색 — github-light `button.foreground`
선정과 같은 "새 값 발명 금지, 같은 팔레트 안에서 재선정" 원칙)에서 대비·ΔE 모두 우수한 값을
선정했다:

| 테마 | 정정 전 | 정정 후 | 근거(업스트림 위치) | 대비(vs panel.bg) | ΔE(vs app.fg) |
|---|---|---|---|---:|---:|
| monokai | `#f8f8f2`(=app.fg) | `#E6DB74` | `theme-monokai` tokenColors 의 string/regexp/link 등에 반복 사용되는 노랑 | 11.63 | 50.65 |
| palenight | `#ffffff`(=app.fg) | `#ffcb6b` | `vscode-palenight-theme` tokenColors 의 variable/type/attribute 등에 반복 사용되는 금색 | 9.10 | 56.85 |
| night-owl-light | `#403f53`(=app.fg) | `#aa0982` | `night-owl-vscode-theme` tokenColors 의 number 에 사용되는 마젠타 | 6.00 | 62.74 |

세 값 모두 `panel.background` 대비 **4.5:1 이상**(9~11.6대) 확보 + ΔE 임계(2.3) 대비 20배
이상 여유. `bundled-theme-contrast.test.ts`(d-31 산출)로 그린 확인, `prettier --check` 통과.
ΔE 축은 이번에 새로 추가된 관심사라 그 테스트는 손대지 않았다(WCAG 대비 게이트와 ΔE 구별성
게이트는 서로 다른 실패 모드 — everforest-light/rose-pine-dawn 의 기존 대비 예외와는 무관).

**Rust 도메인 린트 미확장**: `service.rs` 의 번들 린트(matchHighlight 불투명 검사)는 대비·ΔE
어느 쪽도 검사하지 않는 별개 게이트이고, 이번 3종 정정은 전부 불투명 6자리 hex 라 그 린트를
그대로 통과한다 — 계약 지시대로 Rust 확장은 하지 않았다(openIssues 참고).

#### ③ 재변환 재현성 관계 (제약 대응)

세 테마의 실제 원본(위 ②에서 재취득한 파일)을 레포의 실 CLI(`scripts/convert-vscode-theme.ts`)
로 직접 재변환해 확인했다:

- monokai(dark) → `panel.matchHighlight` = `#569CD6`(safe-default)
- palenight(dark) → `#569CD6`(safe-default)
- night-owl-light(light) → `#0066BF`(safe-default)

세 값 모두 손수정값과 **불일치** — 새 가드가 `list.highlightForeground`(app.foreground 와
동일) 를 배제하고, 두 번째 후보도 각기 다른 이유(반투명/후보 자체 없음)로 사라져 안전값으로
폴백하기 때문이다. `colors`/`syntax`/`terminal` 나머지 전량은 재변환 결과와 **diff 0**(오늘
시점 업스트림 재취득 기준 — 원본이 그 사이 갱신되지 않았음도 함께 확인)임을 직접 비교로
검증했다. 따라서 d-31 §4 L2-0 결론(재변환 diff-0 게이트의 예외로 문서에 등재)을 그대로 승계해
`docs/theme-system.md` §8.2.3 을 "손수정 4종" → **"손수정 7종"** 으로 갱신하고, 새 3종의
재변환 값·사유·정정 근거·대비 수치를 표에 추가했다(§8.2.3 전문 개정 — 아래 "파일 표" 참고).
정정 전(safe-default)도 이미 AA(4.5 이상)를 통과하므로, 이번 손수정의 목적은 대비 확보가
아니라 **테마 정체성 보존**(외지 파랑 대신 그 테마 고유 accent 유지)이라는 점을 앞 4종(대비
미달 해소가 목적)과 구분해 명시했다.

#### 36종 vs-app.foreground ΔE 재스윕 (정정 반영 후)

```
one-monokai                  ΔE=5.39   (배제선 밖 — 배치 미대상, 계약 명시)
vscode-kimbie-dark            ΔE=7.16   (배제선 밖 — 배치 미대상, 계약 명시)
night-owl                    ΔE=13.84
vscode-abyss                 ΔE=17.52
tokyo-night                  ΔE=18.57
nord                          ΔE=21.31
one-dark-pro                 ΔE=22.06
rose-pine                    ΔE=23.20
vscode-tomorrow-night-blue   ΔE=25.66
everforest-dark              ΔE=29.90
dracula                       ΔE=32.06
vscode-solarized-dark        ΔE=32.57
catppuccin-mocha              ΔE=34.65
solarized-light               ΔE=35.76
vitesse-light                 ΔE=35.90
gruvbox-dark                  ΔE=39.07
darcula                       ΔE=41.58
vscode-dark-modern            ΔE=41.58
vitesse-dark                  ΔE=41.62
vscode-dark-plus              ΔE=43.03
rose-pine-dawn                ΔE=50.47
monokai                       ΔE=50.65  (정정: #f8f8f2→#E6DB74, 이번 배치)
github-light                  ΔE=50.70
kanagawa-wave                 ΔE=53.88
palenight                     ΔE=56.85  (정정: #ffffff→#ffcb6b, 이번 배치)
vscode-light-modern           ΔE=57.35
ayu-light                     ΔE=62.72
night-owl-light               ΔE=62.74  (정정: #403f53→#aa0982, 이번 배치)
vscode-light-plus             ΔE=69.41
vscode-monokai-dimmed          ΔE=72.76
everforest-light              ΔE=76.48
ayu-dark                       ΔE=77.57
github-dark                    ΔE=77.73
vscode-quiet-light             ΔE=79.54
intellij-islands-light        ΔE=84.65
vscode-red                     ΔE=90.91
```

동일색(ΔE 0.0) 0건 — 정정 전 3건(monokai·night-owl-light·palenight) 전량 해소. `2.3` 미만
0건, 최소값 5.39(one-monokai, 계약 범위 밖·미대상). builtin 2종(참고, 범위 외 — 사용자 결정
이월 유지): `taide-dark` `#f9e2af` vs `#cdd6f4` ΔE=43.87 / `taide-light` `#df8e1d` vs
`#4c4f69` ΔE=88.89 — 둘 다 구별 문제 없음.

#### 파일 표

| 파일 | 변경 | 내용 |
|---|---|---|
| `src/shared/lib/color.ts` | 추가 | `Lab` 타입, CIE76 상수 13개(초판 12 는 검토 L3-4 정정), `hexToLab`(비공개), `deltaE76`(export) — 순수 함수, 부작용 없음 |
| `src/shared/lib/color.test.ts` | 추가 | `deltaE76` 5건 |
| `src/shared/lib/theme-convert/mapping-tables.ts` | 수정 | `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E` 상수, `isDistinctFromBodyForeground`(export), `panel.matchHighlight` derive 를 `.filter(isOpaqueForegroundCandidate).find(...)` 체인으로 확장 + JSDoc 갱신. import 에 `deltaE76` 추가(1줄) |
| `src/shared/lib/theme-convert/mapping-tables.test.ts` | 추가 | `isDistinctFromBodyForeground` 3건 |
| `src/shared/lib/theme-convert/convert.test.ts` | 추가 | 파이프라인 신규 3건(monokai/night-owl-light dark·light 폴백 + one-monokai 미배제) + 기존 1건 무회귀 확인 — 초판 "4건"은 검토 L3-4 정정 |
| `src-tauri/resources/themes/monokai.json` | 데이터 정정 | `panel.matchHighlight`: `#f8f8f2` → `#E6DB74` (예외 허용분 — C 임무 지시) |
| `src-tauri/resources/themes/palenight.json` | 데이터 정정 | `panel.matchHighlight`: `#ffffff` → `#ffcb6b` |
| `src-tauri/resources/themes/night-owl-light.json` | 데이터 정정 | `panel.matchHighlight`: `#403f53` → `#aa0982` |
| `docs/theme-system.md` | §8.2.3 개정 | "손수정 4종" → "손수정 7종" — 두 가드(불투명/구별성) 구분 서술, 신규 3종 표 행·재변환 값·diff-0 확인 결과 추가, d-33 계약 상호 참조 추가 |

#### 소비처 갱신

`deltaE76`/`isDistinctFromBodyForeground` 신규 export 는 이번 배치 안에서만 소비(순수 추가,
기존 export 시그니처·동작 변경 없음 — `isOpaqueForegroundCandidate` 시그니처·동작도 무변경).
`panel.matchHighlight` 의 값은 이 3개 테마에서만 바뀌었고 다른 33종·builtin 2종은 무변경.
`resolve-colors.ts`/`convert.ts` 등 소비 경로는 `derived()` 인터페이스(`(ctx) => string |
undefined`)를 그대로 따르므로 무수정.

#### 검증

- `bun run typecheck`(`tsc --noEmit`) — exit 0(프로젝트 전체). 착수 시점엔 T2-C 병렬 작업의
  중간 상태(bridge/keymap 이동 미완료)로 무관한 260건이 떠 있었으나, 종료 시점 재확인 결과
  0건 — 내가 만든 8개 파일 중 어디도 그 에러 목록에 걸린 적이 없었음을 grep 으로 매 단계
  재확인했다.
- `bunx prettier --check`(변경 8파일 — ts 5·test.ts 3 포함, json 3) — 전량 통과.
- `bunx eslint`(ts/tsx 5파일) — 무출력(오류·경고 0).
- `bun test src/shared/lib/color.test.ts src/shared/lib/theme-convert/` — 54 pass / 0 fail
  / 97 expect(). `bun test src/shared/lib/ src/features/search/` — 1085 pass / 0 fail(회귀
  없음, search-match-row 는 미수정이므로 렌더 자체는 동일).
- `bundled-theme-contrast.test.ts`(d-31 산출, WCAG 게이트) — 2 pass, 무회귀(everforest-light·
  rose-pine-dawn 예외 유지, 이번 정정 3종은 애초에 그 예외 목록에 없었음).
- CLI 재변환(`scripts/convert-vscode-theme.ts`, 오늘 재취득 원본) 로 3종 모두 safe-default
  폴백 재현·colors/syntax/terminal diff 0 직접 확인(위 ③).

#### openIssues

1. **Rust 대비/구별성 린트 미확장**(계약 명시 보류): `service.rs` 의 기존 번들 린트는 불투명
   여부만 검사한다. 이번 3종 정정값은 전부 불투명이라 통과하지만, 향후 새 번들 테마 추가 시
   TS 파이프라인을 거치지 않고 Rust 쪽에 직접 하드코딩되는 경로(`builtin_dark`/`builtin_light`
   같은)가 생기면 같은 동일색 결함이 Rust 단에서는 잡히지 않는다. Rust 쪽에 `matchHighlight
   != app.foreground` 검사를 추가할지는 순차 Rust 에이전트 판단 필요(이번 배치에서는 지시대로
   손대지 않음).
2. **임계값 재검증 필요 시점**: `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E = 2.3` 은 현재 36종
   분포(0 → 5.39 간극)에 근거한다. 향후 번들 테마가 늘어 이 간극에 새로운 값이 들어오면
   (예: ΔE 3~4대 테마) 재검증이 필요 — 이번 배치 범위는 아니라 기록만 남긴다.
3. **mapping-tables.ts 라인 포인터 누적 stale**: d-31 §3.8-2 가 이미 인지한 옛 정밀 라인 포인터
   (`2026-08-12-w7-textmate-contract.md:49`, `2026-08-20-palette-ux-contract.md:199`) 문제에
   이번 삽입(+29줄)이 더해져 괴리가 더 커졌다. d-31 선례대로 구현 단계가 아니라 검토(Phase E)
   단계에서 일괄 정정하는 것을 유지 권고 — 이번 배치에서 임의 수정하지 않았다.
4. **builtin_light 대비 정책**: d-31 §4 L2-3 적대적 판정으로 "이월 유지"가 확정된 사용자 결정
   사안(taide-light `panel.matchHighlight` 대비 2.15) — 이번 임무 범위(C) 밖이라 손대지 않았고
   ΔE=88.89 로 구별성 자체는 문제없음을 참고 기록만 남긴다(위 재스윕 표 하단).

**openIssues**:
- Rust 대비/구별성 린트 미확장(service.rs) — 계약 지시대로 이번 배치 보류, 필요 여부는 순차 Rust 에이전트 판단 필요(정정 3종은 불투명이라 기존 린트는 통과)
- MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E=2.3 은 현재 36종 분포(간극 0→5.39)에 근거 — 향후 번들 테마 추가로 그 간극에 새 값이 들어오면 재검증 필요
- mapping-tables.ts 삽입(+29줄)으로 기존 d-31 §3.8-2 가 이미 인지한 구버전 계약 문서의 정밀 라인 포인터 stale 정도가 더 커짐 — d-31 선례대로 Phase E 검토 단계 일괄 정정 유지 권고, 이번 배치에서 임의 수정 안 함
- builtin_light(taide-light) panel.matchHighlight 대비 2.15 이슈는 d-31 §4 L2-3 적대적 판정으로 사용자 결정 필요 이월 확정된 사안 — 이번 임무 범위 밖, ΔE=88.89 로 구별성 자체는 문제없음만 참고 기록

### 3-R. Rust 조각 (f doc 정정 + e 구별성 린트 채택)

### d-33 Rust 조각 2건 처리 기록 (2026-08-24)

### 1. [f] domain/tree/service.rs 낡은 doc 포인터 정정
`rows_page` 위 doc(구 177행)이 이미 소멸한 프론트 상수 `TREE_ROWS_UNBOUNDED_LIMIT`를 언급하고 있었다. grep으로 재확인한 결과 `src/`·`src-tauri/` 어디에도 해당 상수가 존재하지 않음을 확인했고(매치 0건), `src/entities/tree/tree.query.ts`가 실제로 `limit: null`(Option<u32>의 None)을 직접 보낸다는 사실도 확인했다. doc을 "used to be spelled as a u32::MAX sentinel instead of a real no-limit value. The frontend now sends this as limit: null directly (tree.query.ts's treeRowsQueryOptions) — the old sentinel constant no longer exists on either side"로 정정했다.

### 2. [e 린트 판단] 번들 테마 matchHighlight 구별성 Rust 린트 확장 — 판단: 채택(hex 동일성 검사로)
**판단 근거**: TS 파이프라인(`mapping-tables.ts`의 `isDistinctFromBodyForeground`)이 이미 derive 단계에서 CIE76 ΔE<2.3 정밀 검사를 수행한다. 계약 §3-C 재스윕 표 기준 36종 실측 분포를 보면 동일색 결함은 항상 ΔE=0.0(hex 완전 동일)이고 그다음으로 낮은 값은 5.39(one-monokai, 배치 미대상)라, hex 동일성 검사와 ΔE<2.3 검사가 현재 카탈로그 전량에서 판정이 일치한다. 따라서 Rust에 CIE76 전체를 이식하는 비용을 들이지 않고, TS = 정밀 1차 게이트(derive 단계 차단), Rust = "완전 동일색 데이터가 TS 파이프라인을 거치지 않고 재유입"하는 것을 막는 저비용 2차 게이트로 역할을 분담하는 것으로 결론지었다.

**구현**: `src-tauri/src/domain/theme/service.rs`의 `mod tests`에 `번들_테마는_panel_매치_하이라이트가_app_전경색과_동일하지_않다` 테스트 추가. `bundled_themes()`(36개 JSON 소스만 순회 — `builtin_dark()`/`builtin_light()`는 이 함수에 애초에 포함되지 않으므로 자동으로 범위 외, 지시대로 순회 대상에서 제외됨)를 순회하며 `panel.matchHighlight`와 `app.foreground`를 `normalize_hex_color`로 정규화 비교, 동일하면 위반으로 모아 한 번에 assert(기존 `list.activeBackground`/`panel.matchHighlight 불투명` 린트와 동일 관행). 역할 분담 근거를 영어 doc 주석(이 파일의 기존 `///` rationale 관행 준수)으로 명시.
> [d-36 각주 2026-08-25] 이 절의 테스트명은 d-36 에서 `카탈로그_테마는_...` 로 개명되고 순회 범위가 38종(번들 36+빌트인 2)으로 확장됨 — 정본은 `2026-08-25-d36-theme-catalog-audit-contract.md` §3.2.

**FAIL 재현(d-20 §3-E/d-31 선례 방식 — git stash/commit 미사용, 작업트리 파일만 일시 조작)**:
1. monokai.json·palenight.json·night-owl-light.json 3개(작업트리 = TS 임무 C 정정본)를 스크래치패드로 백업.
2. `git show HEAD:<path> > <path>`로 각 파일을 HEAD(정정 전 결함 데이터)로 일시 원복 — HEAD의 panel.matchHighlight가 각각 #f8f8f2/#ffffff/#403f53으로 app.foreground와 정확히 동일함을 사전 grep으로 확인.
3. 신규 테스트 단독 실행 → FAILED, 정확히 3개 테마 위반 보고:
```
'monokai': panel.matchHighlight("#f8f8f2") == app.foreground("#f8f8f2")
'palenight': panel.matchHighlight("#ffffff") == app.foreground("#ffffff")
'night-owl-light': panel.matchHighlight("#403f53") == app.foreground("#403f53")
```
4. 백업본으로 3개 JSON 복원(`git diff --stat` 재확인 — 원래와 동일하게 파일당 1줄 변경만 남음, 백업 왜곡 없음).
5. `domain::theme::service` 전체(31개) 재실행 → 전부 PASS(신규 테스트 포함).

**검증**: cargo fmt --all --check(클린) → cargo test -p taide domain::theme::service(31 pass) + cargo test -p taide domain::tree(13 pass) → cargo clippy -p taide --lib -- -D warnings(경고 0). #[allow] 사용 없음.

**openIssue 처리**: d-33 계약 §3-C openIssue 1번("Rust 대비/구별성 린트 미확장")을 hex 동일성 검사 채택으로 해소. CIE76 전체 이식은 미채택 — 향후 카탈로그가 늘어 hex는 다르지만 ΔE<2.3인 '거의 동일'색이 등장하면 이 Rust 린트는 못 잡고 TS derive 단계에서만 잡힌다는 한계를 doc에 명시.

**openIssues**:
- MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E=2.3(TS)과 이번 Rust hex-동일성 검사의 역할 분담은 현재 36종 카탈로그의 실측 간극(0→5.39)에 근거한 것 — 향후 카탈로그에 hex는 다르지만 ΔE<2.3인 '거의 동일'색이 추가되면 이 Rust 린트는 통과시키고 TS derive 단계에서만 잡힌다(의도된 한계, doc에 명시). TS 쪽 d-33 §3-C openIssue 2번(임계값 재검증 필요 시점)과 연동된 사항.
- builtin_light(taide-light) panel.matchHighlight 대비 이슈(d-31 §4 L2-3, 사용자 결정 이월)는 이번 Rust 조각 범위 밖 — bundled_themes()가 builtin_dark/builtin_light를 애초에 순회하지 않으므로 이번 신규 린트의 순회 대상에도 포함되지 않음(지시대로 미포함).

---

## 4. 검토 반영 (2026-08-25)

> 검토 wf_7300d902-b44(3렌즈): major 1(L2-1 수리 재유입)·minor 16(중복 제거 ~13). 적대적
> wf_baff4b05-4ce **downgraded**(발동 기하 좁음·선재 형제 경로가 실질·d-31 §5.1 잔여분 —
> 재론 아님. 처분 확정 = ② 상태색 우선 2패스·차단 대신 고지·①③은 실테마 2종 임포트 차단으로
> 기각). 수정 wf_ba3f8b26-f2c 8건 전건 반영(신규 테스트 4 — bun 1463). 문서·계약 몫
> (L1-1/L3-2·L2-3/L3-5·L3-4·L3-6·d-31 §5 번들 재변환 정책 이월 기록)은 메인이 직접 반영 완료.

d-33 수정 일괄 8건 처리 실측 기록.

1. [L2-1, major→구현] wlmujax5u.output 이 정본으로 확정한 fixGuidance 구현 스펙 3점(상태색 우선 8키 확장·2패스 술어·2패스 고지)을 문자 그대로 구현. matchHighlight 페어 한정 조건부 분기로 나머지 4쌍(app/editor/panel/tooltip) 수리 동작은 코드·테스트 양쪽에서 불변 확인(기존 contrast.test.ts 4건 전부 무수정 통과). fixGuidance 검증 절의 (i)(ii) 2케이스를 각각 2변형으로 나눠 총 4개 신규 test 로 구현하고 실측 대비/ΔE 값(#88C0D0: 대비 3.45/ΔE 21.3, everforest-light accent #8da101 후보0, rose-pine-dawn textLink.foreground #907aa9)을 사전 프로브(리포 내 임시 스크립트 실행 후 즉시 삭제, git status 로 잔존 0 확인)로 검증한 뒤 하드코딩. 순환 임포트 없음(mapping-tables.ts→color.ts/ansi-palette.ts/types.ts 뿐, contrast.ts 역참조 없음) 확인 후 mapping-tables.ts 의 isDistinctFromBodyForeground/isOpaqueForegroundCandidate 를 contrast.ts 에서 재사용.

2. [L1-3, minor→구현] fuzzyMatch 인자를 LSP 질의 문자열(trimmedQuery)과 동일하게 trim. 컴포넌트 테스트는 이 저장소에 DOM 렌더 하네스가 전무하다는 기존 관행(error-boundary.test.tsx JSDoc)과 일관되게 미추가 — 필요 시 별도 합의 후 진행 권고.

3. [L3-3, minor→구현] `//` 7줄 → JSDoc, 내용 무변경.

4. [L2-4, minor→구현] deltaE76 알파 무시 정책·isDistinctFromBodyForeground fail-open 근거 각각 JSDoc 문단으로 명시.

5. [L3-8, minor→구현] SwitchField 분기 중복 제거, label/Switch 1벌 통합. DOM 출력 동일성은 두 분기 모두 동일 label 래퍼+동일 Switch props 구조임을 소스 대조로 확인(자동 렌더 테스트 없음 — 프로젝트에 컴포넌트 렌더 하네스 부재는 위 2번과 동일 사유).

6. [L3-9, minor→구현] 자명한 `: boolean` 제거.

7. [L1-2/L3-1, minor→구현] 동일 파일의 stale JSDoc 경로 1건 정정(두 발견이 같은 지점을 가리켜 1회 수정으로 해소).

8. [L2-2, minor→구현] §8.2 불릿에 구별성 가드 1~2행+상호참조, §8.2.3 에 2패스 수리 동작 1문단. 번들 36종 중 safe-default 자체가 저대비인 재유입 구간(L2-1 원 버그의 발동 조건)은 여전히 무발동이라는 기존 서술을 유지하면서, everforest-light/rose-pine-dawn(원본 accent 자체가 저대비인 별도 구간)은 오늘 재변환 시 이 수리 경로를 실제로 거친다는 사실을 정확히 구분해 기술 — 두 구간을 혼동해 '번들 36종은 이 경로에 도달 안 함'으로 과잉 일반화하지 않도록 초안을 1차 자기검토로 수정.

범위 외(과제에 포함되지 않은 발견) — L1-1·L3-2(command-palette.md/editor.md stale keymap 경로)·L2-3·L3-4·L3-5·L3-6·L3-7·L3-10 은 오케스트레이터 지시(1~8번) 밖이라 손대지 않음.
