# d-31 — T2-B TS 일괄 + d-26 이월 저대비 통합 배치 계약 (2026-08-24)

> 정본: `2026-08-21-batch-consolidation-decision.md` §2 d-31 행 / 감사
> `2026-08-18-architecture-audit.md` C7(:204-208)·§5 T2-B / d-26 이월
> `2026-08-20-palette-ux-contract.md` §4.3 잔존 결함. 통합 배치 1호 — 같은 성격(TS 구조
> 분해)·같은 위험 축을 한 계약·한 구현 회전·한 검토 회전으로 묶는다(검토 축소 아님).
> 사용자 승인: 2026-08-24 "d-31 만 승인"(완료 후 멈추고 재확인). 결정 4건 §1.1.

## 0. 착수 전 메인 실사 (2026-08-24)

| 대상 | 실측 | 감사 대비 |
|------|------|-----------|
| `shared/lib/theme-convert/mapping-tables.ts` | 638줄 — 4관심사 동거 확인: ① 변환 테이블(COLOR_MAPPING·family fallback·safe defaults·list 3종 default·isUsableListBackground·chain/derived) ② UI 토큰 어휘(COLOR_NAMESPACES·SYNTAX_TOKENS·SYNTAX_SCOPE_CANDIDATES) ③ LSP 시맨틱(SEMANTIC_TOKEN_TYPE_MAP·legend scope 2종·lookup) ④ ANSI/레인(VSCODE_DEFAULT_ANSI_PALETTE·GRAPH_LANE_ANSI_ORDER·TERMINAL_MIRRORED_TOKENS) | 612→638(d-26b 가드 확장분) |
| `widgets/editor-pane/lsp-session-registry.ts` | 986줄(테스트 761줄 별도 — 초판 987 은 §4 검토 L1-1 정정). 오배치 확인 — 크로스 위젯 공유 데이터 싱글톤이 editor-pane 소속. **import 위반은 0**: shared 2파일(document-symbol-session-waiters·workspace-edit-applier)의 registry 언급은 주석 문언뿐, 실 의존은 이미 역전(파라미터 주입) 완료. 실 소비처는 위젯 6(command-palette·editor-area·breadcrumbs-bar·use-editor-lsp-integration·use-lsp-session·outline-panel-container)+테스트 | 505→986(문서주석 증가) |
| `widgets/git-panel/git-panel.tsx` | 396줄 — merge/staged/unstaged 행 그룹의 ContextMenu+StatusRowItem+액션 구성 3중 복제 확인. props 33 | 무변동 |
| `widgets/command-palette/command-palette.tsx` | 420줄 — 5모드 JSX+로딩 effect 2+지역 컴포넌트 HighlightedText(SFC 위반) 동거 | 384→420(d-26 UX) |
| `widgets/explorer/explorer-container.tsx` | 349줄 — CRUD(draft/rename/delete)·클립보드·비교·브리지·컨텍스트 메뉴 맵 혼재 | 351→349 |
| d-26 이월 저대비 | 번들 `github-dark.json:49` `panel.matchHighlight: #ffd33d22`(알파 13%)·`github-light.json:49` `#ffdf5d66`(알파 40%) 실측. 원인 = `mapping-tables.ts:495` 체인이 `list.highlightForeground`(불투명 전경) 부재 시 `editor.findMatchHighlightBackground`(반투명 오버레이 전용)로 폴백. `features/search/search-match-row.tsx:41` SearchMatchRow 본문 `<mark>` 은 구식 배경-채움(초판 "ContextLine" 지목은 §4 검토 L1-4 정정 — ContextLine 에는 `<mark>` 가 없다)(`bg-panel-match-highlight text-app-background`) 잔존 | 이월 확정 |
| d-30 후속 소생 기회 | `command-palette.tsx` `'>'` 하드코딩 + `command-palette-query.ts` 미참조 export 5(buildCommandModeQuery·프리픽스 상수 4종) — d-30 §4 기록분. 팔레트 재구성 시 `buildCommandModeQuery()` 사용으로 전부 소생 | — |

## 1. 범위·원칙

### 1.1 사용자 결정 (2026-08-24, 4건)

1. **lsp-session-registry = 이동+소분할**(추천안): 상태 싱글톤 코어는 한 파일 유지,
   순수 함수 `buildInitializeParams`+전용 상수만 별파일 분리.
2. **matchHighlight = 파이프라인 가드 + CONTRAST_PAIRS 편입 병행**(이중 방어).
3. **git-panel = 행 그룹 통합만**: props 표면 33개 유지(소비처 무변경 — 최소 diff).
4. **command-palette = 모드 그룹 분리**: HighlightedText 별파일+모드 그룹 5분리+effect 훅화.

### 1.2 작업 항목

**a. mapping-tables 4분할** (`shared/lib/theme-convert/` 내부, d-30 선례 패턴)

- 경계(실사 §0 의 ①~④): 변환 테이블 본체는 `mapping-tables.ts` 잔류(diff 최소 기준 —
  소비처 대다수가 COLOR_MAPPING 계열 참조 시 무변경이 되는 쪽이 잔류), 나머지 3관심사
  신규 파일(명명 kebab-case, 구현이 소비처 전수 grep 으로 경계·명명 확정 — d-30 §3.1 방식).
- **동작·API 무변경**(f 의 matchHighlight 결함 수정만 예외). export 시그니처 유지,
  소비처는 import 경로만 변경. barrel 금지.

**b. lsp-session-registry 이동+소분할**

- `widgets/editor-pane/lsp-session-registry.ts` → **`entities/lsp/lsp-session-registry.ts`**
  (flush-registry 선례 위치·명명). 근거: registry 는 `@entities/lsp/lsp.ipc`·
  `lsp-session-flush-registry` 를 import 하므로 shared 불가(shared→entities 금지),
  데이터 레이어 싱글톤의 FSD 정위치는 entities. 소비처 위젯 6곳 widgets→entities 합법.
- 테스트 동행 이동: `entities/lsp/lsp-session-registry.test.ts`.
- 소분할: `buildInitializeParams`+`toWorkspaceFolderName`+`CODE_ACTION_KIND_VALUE_SET`+
  `FOLDING_RANGE_LIMIT` 를 `shared/lib/lsp/initialize-params.ts` 로(전 의존이 shared 뿐인
  순수부 — 실사로 확인됨). 코어(세션 맵 4종·acquire/release·재초기화 플로우)는 분할 금지
  (모듈 싱글톤 상태 공유 위험 — 사용자 결정 1).
- shared 2파일의 주석 문언 중 옛 경로 언급(`widgets/editor-pane/lsp-session-registry`)은
  새 경로로 정정(수정하는 코드에 붙은 낡은 주석 방치 금지).

**c. git-panel 행 그룹 통합**

- merge/staged/unstaged 3중 복제(ContextMenu+StatusRowItem+액션 배열 구성)를 그룹 종류
  파라미터화한 feature 컴포넌트 1개로 통합(`features/git/` — 데이터·콜백은 props 로만 받는
  순수 컴포넌트라 features 적합. 1파일 1컴포넌트). 그룹별 차이(액션 구성·kind 소스·
  onOpenChanges 그룹 인자·discard 유무)는 props 로 표현.
- 확인 다이얼로그 2종(discard·stage-all)은 분리 가능하면 분리(구현 판단 — 통합 후 본체가
  충분히 줄면 잔류 허용).
- **GitPanel props 표면 33개 무변경**(소비처 git-panel-container 무수정 — 사용자 결정 3).

**d. command-palette 모드 그룹 분리**

- `HighlightedText` 별파일 분리(SFC 준수). 위치는 소비처 기준 구현 판단 —
  단 f 의 search-match-row 전경 전환과 스타일 토큰 조합이 동일해지므로 공용화 검토
  (2회 이상 룰 충족 시 shared 승격, 데이터 형태가 달라 강제 통합이 부자연스러우면 각자 유지).
- 모드별 결과 그룹 5개(commands/files/symbol/line/workspaceSymbol)를 파일 분리, 심볼·
  워크스페이스 심볼 로딩 effect 2개를 훅 파일로. 본체는 상태·키맵·조립만.
- d-30 후속 소생: `'>'` 하드코딩 → `buildCommandModeQuery()` 로 교체(미참조 export 소생).
- 동작 무변경(키맵 이중 리스너·chord 게이트 주석의 계약 문언 포함 이동).

**e. explorer-container 관심사 훅 분리**

- CRUD(draft 생성·rename·delete)·클립보드(cut/copy/paste)·비교 로직을
  `widgets/explorer/` 내 훅 파일로 분리(비즈니스 로직이므로 widgets 잔류. 경계·명명은
  구현 실사 — 예: use-explorer-entry-crud·use-explorer-clipboard). 본체는 쿼리·조립·
  컨텍스트 메뉴 맵 유지. 동작 무변경.

**f. matchHighlight 저대비 근본 수정** (결함 수정 — 유일한 동작 변경 항목)

1. **파이프라인 가드**: `panel.matchHighlight` 를 `chain` → `derived` 전환.
   후보(`list.highlightForeground` → `editor.findMatchHighlightBackground`)가 **의미 있는
   알파(불투명 미만)** 이면 배제하고 다음 후보/안전 폴백으로 — 이 토큰은 전경 용법(팔레트·
   검색 매칭 글자색)이므로 반투명 오버레이 값은 어떤 배경 위에서도 흡수된다(d-26 §4.3).
   list 계열 derived 가드(d-26b) 선례 패턴.
2. **CONTRAST_PAIRS 편입**: `contrast.ts` 에 matchHighlight(전경) vs `panel.background` 쌍
   추가. **알파 합성 필수** — 대비 계산 전 전경을 배경 위에 합성(d-26 재스윕 방식).
   수리 전경 후보는 업스트림 불투명 전경 계열로 구성(구현이 후보 사슬 설계·기록).
3. **번들 데이터 정정**: `github-dark.json`·`github-light.json` 의 `panel.matchHighlight` 를
   업스트림 팔레트 충실 불투명 전경값으로 정정. **36종 재스윕**(panel.background 합성 대비)
   수치를 §3 에 기록 — github 2종 외 추가 FAIL 발견 시 데이터 정정 포함.
4. **Rust 데이터 린트 확장**: `domain/theme/service.rs` 번들 린트에 반투명
   `panel.matchHighlight` FAIL 규칙 추가(d-26b 린트 확장 선례).
5. **search-match-row 통일**: `SearchMatchRow` 본문 `<mark>`(:41 — 초판 "ContextLine" 은 §4 L1-4 정정)의 배경-채움을 d-26 전경 강조 방식
   (`bg-transparent text-panel-match-highlight font-semibold`)으로 전환.

### 1.3 범위 외

- T2-B 잔여 Rust(lib.rs·layout) = d-32. T2-C·이월 소형 = d-33. AppError = d-34.
- 팔레트 fuzzy 가중치 재설계·workspaceSymbol 하이라이트(d-26 §3.4 이월 유지 — d-33).
- 커맨드·기능 추가/제거, props 표면 변경(git-panel 33 유지), bindings 표면 변경.
- 각 계약의 기각 대안 재론 금지.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh) 영역 병렬 4 + Rust 순차 1:
  A=테마 축(a+f-1·2·5 TS) / B=lsp 이동(b) / C=팔레트(d) / D=git-panel+explorer(c+e) /
  E=**Rust 단독·A 완료 후 순차**(f-3 번들 JSON 정정+f-4 린트+재스윕 — Rust 한 시점 한
  에이전트). 각 에이전트에 **계약 §3 구현 기록 필수** 명시(feedback 2026-08-21 교훈).
- 검토(opus+xhigh) 렌즈 영역 배분: ① 동등성(분할 a~e 멀티셋 대조·소비처 전수·죽은 import 0)
  ② 테마·대비 정합(f 가드/합성 수학·번들 데이터·린트·재스윕 재계산) ③ LSP 수명·동시성
  (이동 후 싱글톤 단일성·flush 배선·이벤트 리스너 모듈 초기화 시점) ④ 컨벤션·FSD 경계
  (레이어 정합·SFC·2회 룰·주석 금지). 적대적(opus+high) major 건별 → confirmed 수정 →
  메인 2차(스팟+verify 직접) → 커밋(구조 분해 / 저대비 결함 수정 논리 단위 분리) → prod 병합.
- 검증: `bun run verify` + `bunx vite build` exit 0 + bindings 무변경 + Rust 테마 린트 통과.
  재스윕 수치 기록.

---

## 3. 구현 완료 기록 (구현 Workflow 가 §3.1~ 에 기록 — 검토 전 필수)

### 3-A. 테마 축 (a 4분할 + f-1 가드 + f-2 CONTRAST_PAIRS + f-5 search-match-row)

### 3.1 mapping-tables 4분할 — 파일 구성 (a)

착수 전 실사(§0)의 4관심사 경계를 그대로 따라 분리했다. 신규 파일명(kebab-case)은 소비처 grep 결과 각 관심사가 뚜렷이 갈리는 것을 확인한 뒤 용법을 드러내는 이름으로 확정했다(d-30 §3.1 방식).

| 파일 | 관심사 | 내용 | 줄수 |
|---|---|---:|---:|
| `mapping-tables.ts` (원파일명 잔류) | 변환 테이블 본체 | `SELF_REF_PREFIX`·`FAMILY_FALLBACK_SOURCE_KEYS`·`SAFE_DEFAULT_COLORS`·`VSCODE_LIST_*_DEFAULT` 3종·`chain`/`derived`·`COLOR_MAPPING`·`isUsableListBackground`·(f-1 신설)`isOpaqueForegroundCandidate` | 318 |
| `ui-token-vocabulary.ts` (신규) | UI 토큰 어휘 | `COLOR_NAMESPACES`·`SYNTAX_TOKENS`·`SYNTAX_SCOPE_CANDIDATES` | 173 |
| `semantic-token-map.ts` (신규) | LSP 시맨틱 | `SEMANTIC_TOKEN_TYPE_MAP`·`SEMANTIC_TOKEN_LEGEND_SCOPE_PREFIX`·`toSemanticTokenLegendScope`·`lookupSemanticTokenTypeMapping` | 123 |
| `ansi-palette.ts` (신규) | ANSI/레인 | `TERMINAL_MIRRORED_TOKENS`·`VSCODE_DEFAULT_ANSI_PALETTE`·`GRAPH_LANE_ANSI_ORDER` | 62 |

**잔류 파일명 판단(diff 최소 기준)**: 변환 테이블 본체(`COLOR_MAPPING` 등)를 원파일명 `mapping-tables.ts` 에 남긴 것은 실사 §0 이 확인한 대로 소비처 10곳 중 `resolve-colors.ts` 1곳만이 이 몸통을 참조하고 나머지 9곳은 전부 신규 3파일 쪽 export 만 참조하기 때문 — 잔류 쪽 소비처가 최소이므로 원파일명을 그쪽에 남겨 전체 diff 를 줄였다(신규 3파일은 애초에 새 경로이므로 "잔류로 인한 무변경" 이득이 발생하지 않는다).

**경계·순환 검증**: 각 파일의 실제 import 방향을 추적해 순환이 없음을 확인했다.
- `ui-token-vocabulary.ts` → 외부 의존 없음(자기완결).
- `semantic-token-map.ts` → `import type { SYNTAX_TOKENS } from ui-token-vocabulary`(전부 `typeof SYNTAX_TOKENS` 타입 위치에서만 쓰여 `import type` 으로 충분 — `verbatimModuleSyntax` 하에서 값 재노출 없이 타입만 전파).
- `ansi-palette.ts` → `import type { AnsiLookup, ThemeTypeArg } from types`(외부 값 의존 없음).
- `mapping-tables.ts`(잔류) → `import { GRAPH_LANE_ANSI_ORDER } from ansi-palette`(COLOR_MAPPING 의 `graph.lane*` 스프레드가 필요). `ui-token-vocabulary`/`semantic-token-map` 은 참조하지 않는다(COLOR_MAPPING 본문에 `SYNTAX_TOKENS`/`SEMANTIC_TOKEN_TYPE_MAP` 참조가 없음을 원본 확인).

의존 방향은 `mapping-tables → ansi-palette`, `semantic-token-map → ui-token-vocabulary` 두 갈래뿐이고 역방향 참조가 전혀 없어 순환이 성립하지 않는다.

### 3.2 소비처 갱신 (a)

착수 전 실사·grep 재확인 결과 실제 소비처는 정확히 10파일. **갱신 9 + 무변경 1**(초판 지시 목록과 갱신 결과 일치, 범위 밖 추가 발견 0):

- `resolve-colors.ts`: `import { COLOR_MAPPING, FAMILY_FALLBACK_SOURCE_KEYS, SAFE_DEFAULT_COLORS } from mapping-tables` + `import { VSCODE_DEFAULT_ANSI_PALETTE } from ansi-palette` 로 분리(2줄).
- `resolve-syntax.ts`: `SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS` → `ui-token-vocabulary` 로 경로만 교체.
- `resolve-terminal.ts`: `TERMINAL_MIRRORED_TOKENS` → `ansi-palette` 로 경로만 교체.
- `validate-completeness.ts`: `TERMINAL_MIRRORED_TOKENS` → `ansi-palette`, `COLOR_NAMESPACES, SYNTAX_TOKENS` → `ui-token-vocabulary` 로 분리(2줄).
- `build-shiki-theme.ts`/`build-shiki-theme.test.ts`: `SEMANTIC_TOKEN_TYPE_MAP, toSemanticTokenLegendScope` → `semantic-token-map`, `SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS` → `ui-token-vocabulary` 로 각각 분리(2줄).
- `highlighter-smoke.test.ts`: `SYNTAX_TOKENS` → `ui-token-vocabulary`.
- `semantic-tokens.ts`(lsp/adapters): `lookupSemanticTokenTypeMapping, toSemanticTokenLegendScope` → `semantic-token-map`, `SYNTAX_TOKENS` → `ui-token-vocabulary` 로 분리(2줄).
- `semantic-tokens.test.ts`(lsp/adapters): `toSemanticTokenLegendScope` → `semantic-token-map`.
- **무변경 1**: `shared/ui/command.tsx` — 최초 grep 이 매칭한 것은 실제 import 가 아니라 JSDoc 주석의 `mapping-tables.ts`의 `derived()`+`isUsableListBackground`` 문언 포인터뿐이며, 두 심볼 모두 잔류 `mapping-tables.ts` 에 그대로 남아 있어 포인터가 여전히 정확하다 — 수정 불필요.

모든 export 시그니처·함수 본문은 원본에서 한 글자도 바꾸지 않고 그대로 이동했다(f-1 의 `panel.matchHighlight` 항목만 결함 수정으로 예외). `entities/theme/theme-tokens.ts`/`widgets/theme-editor/theme-editor.tsx` 는 grep 에서 함께 걸렸으나 동명이지만 독립적으로 정의된 자체 `COLOR_NAMESPACES`/`SYNTAX_TOKENS`(테마 에디터 UI 전용)이지 이 파일의 소비처가 아님을 확인해 배제했다. barrel(index.ts)은 만들지 않았다.

### 3.3 f-1 — panel.matchHighlight 파이프라인 가드 (결함 수정)

`mapping-tables.ts` 의 `panel.matchHighlight` 엔트리를 `chain(['list.highlightForeground', 'editor.findMatchHighlightBackground'])` 에서 `derived()` 로 전환했다.

- **가드 로직**: 신설한 `MATCH_HIGHLIGHT_FOREGROUND_CANDIDATES` 배열을 순회해 `ctx.vscodeColors[candidate]` 값 중 `isOpaqueForegroundCandidate` 를 만족하는 첫 값을 채택한다. `isOpaqueForegroundCandidate` 는 8자리 hex(`#rrggbbaa`)에서 알파가 `ff` 미만이면 배제하고, 그 외(undefined·3/6자리 hex·알파 `ff`)는 통과시키는 술어 타입가드다. `isUsableListBackground`(배경 후보의 "행 배경과 구분되는가" 관심사)와는 독립된 별개 함수로 분리했다 — 배경 판별과 전경 불투명도 판별은 서로 다른 질문이기 때문.
- **폴백 경로 실사 확인**: 두 후보가 모두 배제되면 `derive` 는 `undefined` 를 반환한다. `resolve-colors.ts` 의 `resolveColorEntry` 는 `entry.derive(ctx) ?? resolveFamilyFallback(entry.category, ctx)` 이므로 `status` 카테고리의 `FAMILY_FALLBACK_SOURCE_KEYS.status = []`(빈 배열)을 거쳐 다시 `undefined` 가 되고, `resolveColors` 의 `ctx.resolved[key] = resolved ?? SAFE_DEFAULT_COLORS[type][entry.category]` 에서 `SAFE_DEFAULT_COLORS.dark.status = '#569CD6'` / `SAFE_DEFAULT_COLORS.light.status = '#0066BF'`(둘 다 6자리 불투명 hex)로 귀결됨을 코드 추적과 신규 테스트(§3.6)로 함께 확인했다 — "전부 배제되면 기존 category 폴백 경로에 맡긴다"는 요구를 그대로 만족한다.
- **JSDoc**: `list.hoverBackground`/`list.activeBackground` 의 d-26b 가드 JSDoc 과 같은 수준으로, github-dark(`#ffd33d22`, 13%)·github-light(`#ffdf5d66`, 40%) 실측치와 `docs/acknowledge/2026-08-20-palette-ux-contract.md` §4.3 인용을 포함해 작성했다.

### 3.4 f-2 — CONTRAST_PAIRS 편입 (contrast.ts)

- **알파 합성**: `contrast.ts` 로컬에 `compositeForegroundOverBackground(foregroundHex, backgroundHex)` 를 추가했다 — 8자리 hex 는 배경 위에 합성해 불투명 6자리 hex 로 변환하고, 그 외(3/6자리)는 그대로 반환하는 항등 연산이다. 모든 `contrastRatio` 호출부(초기 판정·배경 수리 후보 검사·전경 수리 후보 검사·`validateOutputColors`)를 `foregroundContrastRatio = (fg, bg) => contrastRatio(compositeForegroundOverBackground(fg, bg), bg)` 로 교체해 전경 인자에 **일괄 적용**했다. 6자리 전경에 한해 항등 — 8자리 전경(vitesse-dark `#dbd7caee`, 13.01→11.43)은 합성 후 측정으로 의도 변경이다(검토 반영, L2-4). `convert.test.ts` 최초 2개 테스트(133색 전량 safe-default, `outputColorErrors` 빈 배열)가 6자리 전경 기준으로 회귀 없이 통과함으로 확인했고, 8자리 전경의 의도된 변경은 `contrast.test.ts`(§4 참고)에 별도 케이스로 고정했다.
- **의도적 범위 판단(색상 유틸 미승격)**: `resolve-syntax.ts` 에 동일한 알파 합성 로직(`compositeAlphaOverBackground`)이 사설로 이미 존재한다. common.md §3.3 의 "2회 이상 사용 시 공통화" 규칙상 `shared/lib/color.ts` 로 승격하는 편이 원칙에 맞지만, 이번 배치 지시는 f-2 를 `contrast.ts` 로 한정하고 "요청 범위 밖 코드 수정 금지"를 명시했다 — `resolve-syntax.ts` 는 이번 배치에서 import 경로 교체만 허용된 소비처이지 로직 변경 대상이 아니다. 따라서 승격 대신 `contrast.ts` 안에 동형 로직을 로컬로 재작성했다(중복은 인지하고 있으며 openIssues 에 후속 승격 후보로 기록). **§4 해소**: 검토 L4-6 의견 수용으로 §4-F1 에서 `shared/lib/color.ts` 의 `compositeOverBackground` 로 승격 완료 — 이 openIssue 는 닫힘.
- **쌍 추가**: `CONTRAST_PAIRS` 에 `{ label: 'matchHighlight', foregroundKey: 'panel.matchHighlight', backgroundKey: 'panel.background' }` 를 추가.
- **수리(repair) 전경 후보 사슬 설계**: `CONTRAST_REPAIR_FOREGROUND_CANDIDATES['panel.matchHighlight'] = ['editor.foreground', 'foreground']`. `list.highlightForeground` 를 후보에서 제외한 이유는, 그 값이 애초에 f-1 가드를 통과할 만큼 불투명했다면 이미 1차 resolve 단계에서 `panel.matchHighlight` 자체로 채택되어 있을 것이므로 수리 단계에서 재시도해도 항상 동일한(이미 실패한) 값을 다시 검사하는 무의미한 후보이기 때문 — 대신 다른 category(`foreground`)에서 이미 "업스트림 불투명 전경"으로 검증되어 쓰이는 `editor.foreground`/`foreground`(COLOR_MAPPING 의 `app.foreground`/`editor.foreground`/`panel.sectionHeader` 수리 후보와 동일 계열)를 채택했다. `panel.background` 자체에 대한 `CONTRAST_REPAIR_BACKGROUND_CANDIDATES` 항목은 추가하지 않았다(지시가 "전경 후보 사슬"만 요구했고, panel.background 는 다른 페어에서도 수리 대상이 아니었던 기존 설계와 일관).

### 3.5 f-5 — search-match-row 전경 통일

`src/features/search/search-match-row.tsx:41` 의 `<mark>` 클래스를 `bg-panel-match-highlight text-app-background rounded-xs`(배경-채움)에서 `bg-transparent text-panel-match-highlight font-semibold`(d-26 팔레트와 동일한 전경 강조)로 교체했다. 이 파일의 다른 부분은 손대지 않았다.

**공용 하이라이트 컴포넌트 미신설 판단**: 팔레트의 `HighlightedText`(`command-palette.tsx`)는 `fuzzyMatch` 의 `indices`(문자 단위 인덱스 배열)를 `buildFuzzyHighlightSegments` 로 런(run) 분해해 렌더하는 반면, 이 파일의 `ContextLine`/매치 span 은 `match.matchStart`/`match.matchEnd`(연속 구간 하나)를 문자열 슬라이스로 나눠 렌더한다 — 입력 데이터 형태(희소 인덱스 배열 vs 연속 구간 두 정수) 자체가 달라 공용화가 부자연스럽다고 판단해 각자 유지했다(스타일 클래스 조합만 일치시켰다).

### 3.6 테스트

- `convert.test.ts` 에 f-1 검증 4건 추가: 두 후보가 모두 없을 때 safe-default 폴백, github-dark 실사례(`#ffd33d22`) 반투명 배제, github-light 실사례(`#ffdf5d66`) 반투명 배제, `list.highlightForeground` 가 불투명이면 우선 채택. 전부 `result.outputColorErrors` 가 빈 배열임을 함께 검증해 최종값이 대비까지 통과하는 가시값임을 확인했다.
- `contrast.test.ts` 신규 생성(이 디렉토리에 `contrast.ts` 전용 테스트가 없었으므로 이번에 처음 신설 — `convert.test.ts`/`jsonc.test.ts` 와 같은 파일당-1테스트 관행에 맞춤). `validateOutputColors`(반투명 시 저대비로 판정/불투명 시 정상 판정 2건), `repairContrastPairs`(matchHighlight 수리 1건, 기존 4쌍 합성-항등 확인 1건) 총 4건.

### 3.7 검증

- `bun run typecheck`(`tsc --noEmit`) — exit 0.
- `bunx prettier --check`(변경·신규 파일 17개 + `docs/features/editor.md`) — 전량 통과.
- `bunx eslint`(동일 파일 목록) — 오류·경고 0건.
- `bun test src/shared/lib/theme-convert/ src/shared/lib/shiki/ src/shared/lib/lsp/adapters/semantic-tokens.test.ts src/features/search/` — 72 pass / 0 fail / 204 expect(), 7 파일(신규 `contrast.test.ts` 4건 + `convert.test.ts` 신규 4건 포함). `src-tauri/resources` 번들 JSON 은 무수정(범위 외, Task E 몫) — `git status` 로 미노출 확인.

### 3.8 openIssues (범위 밖 발견, 수정 안 함)

1. `resolve-syntax.ts` 의 사설 `compositeAlphaOverBackground` 와 `contrast.ts` 의 신설 `compositeForegroundOverBackground` 가 동형 로직으로 중복된다. common.md "2회 이상" 룰상 `shared/lib/color.ts` 승격 후보이나, 이번 배치는 f-2 를 `contrast.ts` 로 한정하고 범위 밖 코드 수정을 금지했으므로 `resolve-syntax.ts` 는 건드리지 않았다 — 후속 배치에서 승격 검토 권고. **§4 해소**: L4-6 수용, §4-F1 에서 승격 완료.
2. `docs/acknowledge/2026-08-12-w7-textmate-contract.md:49`(`mapping-tables.ts:344~360`)와 `docs/acknowledge/2026-08-20-palette-ux-contract.md:199`(`mapping-tables.ts:610-616`)의 정밀 라인 포인터가 이번 분할로 낡았다. d-30 선례(§4)는 이런 stale 포인터를 구현 단계가 아니라 Phase E 검토 단계에서 정정했으므로, 이번에도 동일하게 검토 단계로 넘긴다(미수정).
3. `docs/quality-assurance/2026-08-18-architecture-audit.md` 의 C7 표(`mapping-tables.ts | 612 | 4개...` 행, :204)와 T2-B 요약 행(:347)은 시점 기록으로 유지했다 — d-30 완결 후에도 그 표의 `command-registry.ts` 행이 갱신되지 않은 선례와 동일하게 취급(감사 표는 발견 당시 스냅샷이지 실시간 동기화 대상이 아님).

### 3-B. lsp-session-registry 이동+소분할 (b)

#### b. lsp-session-registry 이동+소분할 — 구현 완료 기록

**이동**: `widgets/editor-pane/lsp-session-registry.ts`(986줄 — 초판 987 은 §4 L1-1 정정) → `entities/lsp/lsp-session-registry.ts`(832줄, `git mv`, 83% 유사도로 rename 인식·히스토리 보존). 테스트 `lsp-session-registry.test.ts`(761줄) 동행 이동(99% 유사도, `git mv`) — 동적 import 경로 1줄만 교체.

**파일 구성 표**

| 파일 | 관심사 | 줄수 |
|------|--------|------|
| `entities/lsp/lsp-session-registry.ts` | 세션 싱글톤 코어 — 4종 세션 맵(sessionsByKey·recordsBySessionId·waitersBySessionKey·languageAdapterListeners)·acquire/release·join 감지(createSession)·문서 refcount(acquire/releaseDocument)·재초기화 플로우(reinitializeSession)·이벤트 리스너 등록(모듈 최상단 부수효과) | 832 |
| `shared/lib/lsp/initialize-params.ts` (신규) | 순수 initialize 파라미터 빌더 — `buildInitializeParams`(export)·`toWorkspaceFolderName`(비export)·`CODE_ACTION_KIND_VALUE_SET`·`FOLDING_RANGE_LIMIT` | 154(초판 169 는 §4 L1-1 정정 — 832+154=986 원본 등치) |

**경계 판단 근거**: 실사(§0)로 확인된 4개 심볼(`buildInitializeParams`·`toWorkspaceFolderName`·`CODE_ACTION_KIND_VALUE_SET`·`FOLDING_RANGE_LIMIT`)은 전 의존이 `monaco`(setup)·`protocol.ts`·`command-relay.ts`(상수 2종)뿐인 순수 함수/상수로, `shared` 하위 계층 의존만으로 완결되어 shared 격리 대상. 코어(세션 맵 4종·acquire/release·재초기화)는 모듈 싱글톤 상태(`sessionsByKey` 등)를 공유하는 단일 파일 유지가 사용자 결정 1(추천안)로 확정. `toWorkspaceFolderName` 은 grep 확인 결과 `buildInitializeParams` 내부(옛 240번째 줄, 신규 파일 40번째 줄)에서만 참조되고 registry 코어 쪽에서는 미사용 → 비export 로 유지.

이동 위치는 `entities/lsp/lsp-session-flush-registry.ts` 선례를 그대로 따름(계약 명시). `entities` 는 eslint FSD 규칙상 `@widgets`/`@features`/`@app` import 가 금지되어 있고 registry 는 `@entities/lsp/lsp.ipc`·`@entities/lsp/lsp-session-flush-registry`(entities↔entities, 허용)만 참조하므로 신규 순환 의존 없음.

**소비처 갱신 전수 목록 (7개 파일 — 6 위젯 + 1 테스트)**

| 파일 | 갱신 내용 |
|------|-----------|
| `widgets/command-palette/command-palette.tsx:36` | import 경로만 교체(이 임무 시점). **§4 L3-4 정정**: 이후 임무 C 분해로 이 import 는 소멸 — 최종 트리에서는 `use-document-symbol-loader.ts:9`(waitForLspSessionForRoot)·`use-workspace-symbol-search.ts:4`(listSessionRecordsForProject)가 승계(소비처 계 8 = 소스 7+테스트 1). |
| `widgets/editor-area/editor-area.tsx:44` | import 경로만 교체(`subscribeLanguageAdapterRegistration`). |
| `widgets/editor-pane/breadcrumbs-bar.tsx:20` | import 경로만 교체(`waitForLspSessionForRoot`). |
| `widgets/editor-pane/use-editor-lsp-integration.ts:14` | import 경로만 교체(`peekLspSessionForRoot`·`waitForLspSessionForRoot`). |
| `widgets/editor-pane/use-lsp-session.ts:14-21` | `type SessionRecord` + 5개 value import 2줄 경로 교체. prettier 훅이 짧아진 두 import 문을 자동으로 한 줄로 병합(내용 불변, printWidth 150 이내). |
| `widgets/outline-panel/outline-panel-container.tsx:16` | import 경로만 교체(`waitForLspSessionForRoot`). |
| `entities/lsp/lsp-session-registry.test.ts:150` | `importRegistry` 동적 import 경로 1줄 교체. |

이동 후 `grep -rn "editor-pane/lsp-session-registry" src` = 0건 확인.

**낡은 주석 경로 정정 (2건)**

- `shared/lib/lsp/document-symbol-session-waiters.ts:34` — `@widgets/editor-pane/lsp-session-registry`/`shared→widgets` → `@entities/lsp/lsp-session-registry`/`shared→entities`. 논지("shared 는 이 모듈을 직접 import 할 수 없다")는 유지 — FSD 매트릭스상 shared→entities 도 금지이므로 여전히 참.
- `shared/lib/lsp/workspace-edit-applier.ts:199-200` — `widgets/editor-pane/lsp-session-registry.ts`/`shared 는 widgets 를 import 할 수 없다` → `entities/lsp/lsp-session-registry.ts`/`shared 는 entities 를 import 할 수 없다`.
- registry 파일 자체 JSDoc 자기 참조 1건 추가 정정: `createSession` 내 `semanticTokensRefreshDisposable` 주석의 `` `buildInitializeParams` above `` (같은 파일 내 "위쪽" 을 뜻하던 표현)가 함수 이동으로 더 이상 참이 아니게 되어 `` `buildInitializeParams` (`@shared/lib/lsp/initialize-params.ts`) `` 로 정정. 나머지(파일명만 언급하는 다수 참조, 이동한 `buildInitializeParams` 내부의 "above/below" 상대 위치 서술)는 계약이 정한 "경로 문자열만 정정" 범위 밖으로 판단해 원문 유지.

**확인 사항**

- 모듈 초기화 부수효과: `registerLspSessionAllFlush(flushAllLspSessionDisposals)`·`registerLspSessionProjectFlush(flushLspSessionsForProject)`·`events.lspSessionStatusChanged.listen(...)` 3줄은 파일 최하단에 원문 그대로 잔류. 소비처(예: `editor-area.tsx`)가 `@entities/lsp/lsp-session-registry` 를 정적 import 하는 순간 모듈이 평가되어 이전과 동일 시점(앱 부팅 시 `editor-area.tsx` 마운트)에 실행됨 — 파일 위치만 바뀌었을 뿐 import 그래프상 실행 시점 변화 없음.
- 순환 의존 신설 없음: 신규 `entities/lsp/lsp-session-registry.ts` 는 `@entities/lsp/lsp.ipc`·`@entities/lsp/lsp-session-flush-registry`(둘 다 entities↔entities, fsd.md §2 허용)만 참조. `shared/lib/lsp/initialize-params.ts` 는 `@shared/*` 만 참조(entities/widgets 미참조) — eslint FSD 규칙(`no-restricted-imports`) 통과 확인(eslint exit 0).
- 테스트: `bun test src/entities/lsp/` → 33 pass / 0 fail(3 파일: `lsp-session-registry.test.ts`·`lsp-session-flush-registry.test.ts`·`lsp.query.test.ts`).

**검증**: typecheck(`tsc --noEmit`) exit 0 / `prettier --check` 변경 11파일 전부 통과 / `eslint` 변경 11파일 전부 통과(무출력) / `bun test src/entities/lsp/` 33 pass.

**범위 외 발견(미수정, 보고만)**: `shared/lib/lsp/protocol.ts:208`·`shared/lib/lsp/adapters/semantic-tokens.ts:154` 의 JSDoc 이 각각 "`lsp-session-registry.ts` 의 `buildInitializeParams`"를 언급하는데, 이번 이동으로 `buildInitializeParams` 가 `initialize-params.ts` 로 옮겨져 두 참조 모두 낡았음. 계약이 명시한 정정 대상 2파일(document-symbol-session-waiters.ts·workspace-edit-applier.ts)에 포함되지 않아 범위 밖으로 판단, 수정하지 않고 openIssues 로 보고.

### 3-C. command-palette 모드 그룹 분리 (d)

### d. command-palette 모드 그룹 분리 — 구현 완료 기록 (임무 C)

#### 파일 구성

| 파일 | 관심사 | 줄수 | 비고 |
|------|--------|-----:|------|
| `widgets/command-palette/command-palette.tsx` | 본체 — 상태(useState 5)·쿼리(useQuery/useMutation)·핸들러·키맵 배선(`useGlobalKeymap`·`useKeydownCapture`)·조립 | 311 | 420→311. JSX는 모드별 그룹 컴포넌트 호출로, effect 2개는 훅 호출로 대체 |
| `widgets/command-palette/command-palette-files-group.tsx` | `files` 모드 결과 그룹 | 44 | 신규. widgets 잔류(사유는 "경계 판단" 참조) |
| `widgets/command-palette/use-document-symbol-loader.ts` | `symbol` 모드 문서 심볼 로딩 effect 훅 | 59 | 신규. `DocumentSymbolState` 타입 export |
| `widgets/command-palette/use-workspace-symbol-search.ts` | `workspaceSymbol` 모드 검색 effect 훅 | 51 | 신규. `WorkspaceSymbolState` 타입 export |
| `widgets/command-palette/command-palette-file-match.ts` | 파일 경로 표시 분할 유틸 | 25 | 무변경(기존 파일) |
| `features/command-palette/highlighted-text.tsx` | 퍼지매치 하이라이트 SFC | 16 | 신규 |
| `features/command-palette/command-palette-commands-group.tsx` | `commands` 모드 결과 그룹 | 44 | 신규 |
| `features/command-palette/command-palette-symbol-group.tsx` | `symbol` 모드 결과 그룹 | 30 | 신규 |
| `features/command-palette/command-palette-line-group.tsx` | `line` 모드 결과 그룹 | 23 | 신규 |
| `features/command-palette/command-palette-workspace-symbol-group.tsx` | `workspaceSymbol` 모드 결과 그룹 | 26 | 신규 |

합계 629줄(원본 420줄 대비 +209 — 1파일→9파일 분해에 따른 파일당 import문·타입 선언 오버헤드, 로직 자체는 무변경 이동).

#### 경계·명명 판단 근거

**HighlightedText**: 소비처(commands·files·symbol 3개 그룹)가 데이터·콜백만 받는 순수 컴포넌트이므로 결정 트리 4번 규칙 적용 — `features/command-palette/highlighted-text.tsx`. `features/search/search-match-row.tsx`와의 공용화는 계약 §1.2-d 단서대로 보류(데이터 형태 상이 — 하나는 `{text, indices}` 퍼지매치 인덱스, 다른 하나는 `{matchStart, matchEnd}` 오프셋 쌍이며, f-5(검토 대상 별도) 완료 후에도 스타일 토큰만 겹치고 입력 계약은 여전히 다름). `search-match-row.tsx`는 본 임무 범위 밖(Agent A 담당)이라 건드리지 않음.

**모드 그룹 5개 — 혼합 배치(4 features + 1 widgets)**: 전부 "데이터·콜백을 props로만 받는 도메인 UI"로 결정 트리 4번(features)에 해당하나, `CommandPaletteFilesGroup`만 위젯 전용 순수 유틸 `command-palette-file-match.ts`(`splitFileMatchForDisplay`)에 의존한다. 이 유틸은 현재 `widgets/command-palette/`에 있고 이동은 본 임무 체크리스트에 없어 범위 밖이므로, features 배치 시 `features→widgets` FSD 역참조가 발생한다(eslint `no-restricted-imports` 위반). 따라서 Files 그룹만 기본 위치인 `widgets/command-palette/`에 잔류시키고, 나머지 4개(Commands·Symbol·Line·WorkspaceSymbol)는 `features/command-palette/`에 배치했다. `features/search/search-results-list.tsx`(→`widgets/search-panel`·`widgets/search-editor`가 소비)와 `features/git/git-change-group.tsx`·`status-row-item.tsx`(→`widgets/git-panel`이 소비)가 동일 패턴의 선례다. `widgets→features` 참조(Files 그룹이 `HighlightedText`를, `command-palette.tsx`가 4개 features 그룹을 import)는 FSD 허용 방향이라 문제없음.

**use-document-symbol-loader / use-workspace-symbol-search**: 둘 다 `@entities/lsp/*` 값 import(LSP 세션·루트 해석)를 필요로 하는 비즈니스 로직이라 계약 지시대로 `widgets/command-palette/`에 배치(features는 `@widgets` import 금지이며, 애초에 LSP 세션 접근은 위젯 영역 로직).

**용어 정합성 확인**: `WorkspaceSymbolSearch`(어댑터가 이미 export하는 타입)를 재사용해 `search`/`cancel` 메서드 시그니처를 직접 선언하지 않음. `TreeRow`(`@shared/api/bindings`)·`OpenedFile`·`LspServerDetection`(둘 다 bindings)을 각각 손으로 타입을 다시 적지 않고 원본에서 가져와 사용.

**d-30 후속 소생(item 5)**: `setQuery('>')` → `setQuery(buildCommandModeQuery())`. `buildCommandModeQuery()`가 인자 없이 `'>'`를 반환함은 `shared/lib/command-palette-query.test.ts`의 기존 테스트(`검색어 없이 호출하면 ">" 만 반환한다`)로 이미 검증되어 있어 동작 동일성 확인됨(신규 테스트 불필요). `setQuery('#')`는 실사 결과 전용 빌더 함수는 없었으나 프리픽스 상수 `WORKSPACE_SYMBOL_MODE_PREFIX = '#'`가 이미 export 돼 있어 상수 참조로 교체(계약 "상수/빌더가 이미 export 돼 있으면 함께 소생" 충족). 두 교체 모두 리터럴 값이 완전히 동일하므로 런타임 동작 무변화.

**Line 그룹의 조건 재표현**: 원본 JSX `{mode === 'line' && lineTarget && activePath && (...)}` 중 `mode==='line'`은 본체에서 그대로 유지(`{mode === 'line' && <CommandPaletteLineGroup .../>}`)하고, `lineTarget && activePath`는 컴포넌트 내부 `if (!lineTarget || !activePath) return null`로 재표현했다. 세 조건 모두 참일 때만 렌더링되는 결과는 원본과 동일(단락 평가 `&&` 체인과 조기 반환은 논리적으로 동치이며, common.md §3.4의 early-return 선호 규칙에도 부합).

**JSDoc 이동**: `use-document-symbol-loader.ts`로 옮긴 root-aware/root-agnostic 대비 JSDoc은 원문 그대로 보존했다(파일 경로 문자열이 포함돼 있지 않아 정정 대상 없음). 본문 중 "Workspace Symbol effect below" 표현은 이동 후 더 이상 같은 파일 내 "아래"가 아니게 됐으나, 이는 파일 경로 문자열이 아닌 산문 표현이라 계약의 정정 범위("경로 문자열만 정정")를 벗어나 원문 그대로 유지했다 — 새로 고치거나 지우지 않는다는 지시를 우선 적용. `useKeydownCapture` 블록의 JSDoc(이중 리스너·chord 게이트 계약)은 본체에 그대로 유지(item 4 지시대로 무이동).

#### 소비처 갱신 전수 목록

grep 전수 조사(`@widgets/command-palette`, `@features/command-palette` 참조 전체) 결과:

- **갱신**: 없음. `command-palette.tsx`의 외부 소비처는 `src/app/app.tsx`(`import { CommandPalette } from '@widgets/command-palette/command-palette'`) 1곳뿐이며, export 시그니처(`CommandPalette` named export, 동일 파일 경로)가 무변경이라 **app.tsx는 무수정**.
- 신규 파일 6곳(features 4 + widgets 신규 2)의 유일한 소비처는 `command-palette.tsx`(및 `command-palette-files-group.tsx`가 `highlighted-text.tsx`를 소비) — 전부 본 작업에서 함께 작성.
- `command-palette-file-match.ts`·`command-palette-file-match.test.ts`는 무변경(기존 소비처인 `command-palette-files-group.tsx`가 새로 생겼을 뿐 유틸 자체는 그대로).
- `shared/lib/command-palette-query.ts`(`buildCommandModeQuery`·`WORKSPACE_SYMBOL_MODE_PREFIX`)는 소비처가 0→1로 늘었을 뿐 파일 자체는 무변경.
- 프로젝트 전역에서 `command-palette.tsx`를 문자열로 언급하는 타 레이어 파일(`shared/lib/keybinding-catalog.ts`·`shared/lib/command-catalog.ts`·`entities/lsp/lsp-session-registry.ts`·`shared/lib/lsp/document-symbol-session-waiters.ts`·`entities/lsp/lsp.constant.ts`·`entities/editor/reveal-registry.ts`)는 전부 JSDoc 산문 중 "이 로직을 쓰는 파일" 예시 나열이며 실제 import 관계가 아니어서 갱신 대상이 아님(단 `document-symbol-session-waiters.ts`·`lsp-session-registry.ts`의 "command-palette.tsx가 소비" 서술은 심볼 로딩 로직이 `use-document-symbol-loader.ts`/`use-workspace-symbol-search.ts`로 옮겨가며 엄밀히는 살짝 낡아졌음 — 본 임무 범위(command-palette.tsx만) 밖이라 openIssues로 보고).

#### 검증

- `bun run typecheck`(`tsc --noEmit`) → exit 0, strict + `noUnusedLocals`/`noUnusedParameters` 통과(죽은 import 0 확인의 기계적 증거).
- `bunx prettier --check` 변경/신규 9파일 → 전부 통과.
- `bunx eslint` 변경/신규 9파일 → 무경고(FSD 레이어 위반 0 — `no-restricted-imports` 통과로 features↔widgets 경계 판단이 실제로 유효함을 확인, `function` 키워드·`useCallback`/`useMemo`·`any`·`enum` 금지 규칙 위반 0).
- `bun test src/widgets/command-palette/ src/shared/lib/command-palette-query.test.ts` → 30 pass / 0 fail(`command-palette-file-match.test.ts` 7 + `command-palette-query.test.ts` 23, `buildCommandModeQuery()`의 `'>'` 동일성 케이스 포함).


### 3-D. git-panel 행 그룹 통합 + explorer 훅 분리 (c·e)

### 임무 D — git-panel 행 그룹 통합 + explorer-container 훅 분리 (계약 §1.2-c·e) 구현 완료 기록

#### D-1. 파일 구성 표

| 파일 | 관심사 | 줄수 (이전→이후) |
|---|---|---|
| `src/features/git/git-change-group.tsx` (신규) | merge/staged/unstaged 3그룹의 헤더+행 렌더링을 `variant` 판별 유니언으로 통합한 순수 컴포넌트 | — → 160 |
| `src/widgets/git-panel/git-panel.tsx` | 상단 바·CommitBox·stash 섹션·그래프 섹션·discard/stage-all 확인 다이얼로그 2종 + `isStagedRow`/`isUnstagedRow` 가드 | 396 → 290 |
| `src/widgets/explorer/explorer-path.ts` (신규) | `PATH_SEPARATOR`·`parentDirOf`·`joinPath` — CRUD/클립보드 두 훅과 컨테이너의 `targetDirFor` 가 공유하는 순수 경로 유틸 | — → 8 |
| `src/widgets/explorer/use-explorer-entry-crud.ts` (신규) | draft 생성(`startDraft`/`cancelDraft`/`commitDraft`)·rename(`startRename`/`cancelRename`/`commitRename`)·delete(`confirmDelete`+`deleteTarget`) 상태·핸들러 | — → 169 |
| `src/widgets/explorer/use-explorer-clipboard.ts` (신규) | cut/copy 상태(`clipboard`)·`pasteClipboard` | — → 67 |
| `src/widgets/explorer/explorer-container.tsx` | 쿼리·뮤테이션 조립·`targetDirFor`/`openFileTab`/`openSearchMatch`·openToTheSide/openInTerminal/findInFolder/compare(비교, 유지)·contextMenuHandlers 맵·collapseAllExpanded/refreshVisibleTree·JSX 조립 | 349 → 239 |

#### D-2. 경계·명명 판단 근거

**git-change-group (c항)**

- merge/staged/unstaged 세 블록은 `ContextMenu`+`StatusRowItem`+액션 배열 구성이라는 **동일한 렌더 트리**를 갖고 값(헤더 텍스트·행 kind 소스·액션 3종·컨텍스트 메뉴 항목 5~7개)만 달랐다. 이를 "값만 다른 3벌 JSX"가 아니라 **1벌 렌더 + variant 별 config 생성**으로 풀었다 — `GitChangeGroupConfig`(`title`/`actionLabel`/`actionIcon`/`onAction`/정규화된 `rows`/`buildActions`/`buildContextMenuEntries`/`onRowClick`)를 `props.variant` 3분기로 한 번만 만들고, 실제 `<ResourceGroupHeader>`+`{rows.map(...)}`+`<ContextMenu>` JSX 는 파일 전체에 **정확히 1곳**만 존재한다. 컨텍스트 메뉴 항목까지 `{key,type:'item'|'separator',label,destructive?,onSelect}[]` 디스크립터로 데이터화해 `<ContextMenuItem>`/`<ContextMenuSeparator>` 매핑도 공유 — 세 그룹 인스턴스에서 마크업 반복이 실제로 0이 됨을 diff 로 확인했다.
- 위치는 `features/git/`: 데이터·콜백을 전부 props(variant 유니언의 `on*` 콜백)로만 받고 자체 fetch/query 가 없는 순수 컴포넌트라 features 조건(비즈니스 로직 없는 근간 컴포넌트)에 부합. `features/git/*` 기존 8개 파일 중 5개(`branch-group`·`branch-switcher`·`commit-box`·`stash-list`·`conflict-*`·`create-tag-dialog`·`hunk-discard-dialog`)가 자체 `useTranslation()` 으로 `git.*` 라벨을 직접 렌더하는 관행을 grep 으로 확인했고(`resource-group-header`/`status-row-item` 2개만 라벨을 props 로 받는 범용 프리미티브), `git-change-group` 은 `git.*` 도메인 라벨을 다수(7개 키) 다루는 도메인 컴포넌트이므로 다수 관행(`useTranslation` 직접 호출)을 따랐다 — 라벨 7개를 전부 props 로 뚫는 대안은 다수 관행 위반 + prop 표면 폭증이라 기각.
- `rows` prop 은 `variant` 로 판별되는 유니언(`{variant:'merge';rows:StatusRow[]}` / `{variant:'staged';rows:(StatusRow&{staged:GitStatusChangeKind})[];onUnstage}` / `{variant:'unstaged';rows:(StatusRow&{unstaged:GitStatusChangeKind})[];onStage;onDiscardRequest}`)로 선언했다 — `isStagedRow`/`isUnstagedRow` 타입가드가 만드는 정확한 타입과 구조적으로 동일해 `as` 단언 없이 그대로 흘러들어간다(§5.4 `as` 최소화 원칙 준수). `GitStatusRow`(=`StatusRow` 로컬 별칭)를 쓰지 않고 `@shared/api/bindings` 의 `StatusRow` 를 직접 import 했다 — features 가 widgets(`@widgets/git-panel/git-panel`)를 참조하면 FSD 역방향(features→widgets)이 되므로 반드시 shared 원본에서 가져와야 한다.
- `isStagedRow`/`isUnstagedRow` 는 **git-panel.tsx 잔류**로 판단했다: `requestCommit`(스테이지 유무 분기)·`confirmStageAllAndCommit`(`unstagedRows.map` 스테이지)·`canGenerateCommitMessage` 판정에도 쓰여 GitPanel 자체 비즈니스 로직의 일부이고, 참조처가 git-panel.tsx 하나뿐이라 이동할 근거가 없다(참조 방향 판단).
- 확인 다이얼로그 2종(discard·stage-all)은 그룹 통합만으로 GitPanel 이 396→290줄로 충분히 줄었고, 각각 로컬 상태(`discardTargets`/`confirmStageAllOpen`)·핸들러(`confirmDiscard`/`confirmStageAllAndCommit`)와 강하게 결합된 10~17줄짜리 블록이라 분리해도 복잡도 절감 효과가 미미하다고 판단해 **잔류**시켰다(계약이 명시한 "충분히 줄면 잔류 허용" 조건 충족).

**explorer 훅 분리 (e항)**

- CRUD(`use-explorer-entry-crud.ts`)와 클립보드(`use-explorer-clipboard.ts`)를 별도 파일로 나눈 근거: 두 관심사가 참조하는 mutateAsync 집합이 다르고(CRUD 는 `createEntry`/`toggleNodeAsync`, 클립보드는 `copyEntryAsync`만 추가로 필요), 계약이 예시로 제시한 명명(`use-explorer-entry-crud`/`use-explorer-clipboard`)을 그대로 채택했다. 두 훅 모두 `widgets/explorer/` 잔류 — Rust/DB IO 없는 순수 UI 상태이지만 파일시스템 엔트리 CRUD·클립보드는 이 위젯 고유 비즈니스 로직이라 features 로 내릴 수 없다(계약 명시).
- 비교(compare) 로직(`compareSourcePath`+`compareWithSelected`, 14줄)은 CRUD·클립보드 어느 쪽과도 자연스럽게 묶이지 않아(별개의 상태·별개의 트리거) 계약의 기본값(본체 유지)을 따라 컨테이너에 남겼다.
- 두 훅이 공유하는 `parentDirOf`/`joinPath`(+`PATH_SEPARATOR`)는 컨테이너 파일에 남기면 훅→컨테이너 역참조(순환 import, 컨테이너가 두 훅을 import 하므로)가 발생해 별도 파일 `widgets/explorer/explorer-path.ts` 로 추출했다. `shared/lib/entry-name.ts`(자체 `PATH_SEPARATOR` 보유, `resolveEntryParentDir`/`validateEntryName` 소재)로 편입하는 대안도 검토했으나 요청 범위 밖 공유 파일을 건드리게 되어 최소 diff 원칙에 반해 기각 — 이 두 함수는 `widgets/explorer/` 3개 파일(컨테이너 1 + 신규 훅 2)에서 쓰인다 — **§4 L4-5 정정**: `parentDirOf` 는 실측상 `widgets/editor-pane/breadcrumb-path.ts`(export)·`app/providers/ipc-sync-provider.tsx`(사설)에도 바이트 동일 구현이 선재해 "3개 파일에서만"은 부정확(사본 수는 이번 배치로 순증 없음 — 이동만). shared 승격은 후속 후보로 §5 이월 기록.
- `t`(`useTranslation().t`)는 두 훅 모두 **주입받는 의존성**으로 설계했다 — `widgets/editor-pane/use-editor-blame.ts`·`use-editor-lsp-integration.ts`·`use-editor-git-gutter-and-conflicts.ts` 3개 기존 훅이 전부 `t: ReturnType<typeof useTranslation>['t']` 를 props 로 받는 동일 관행을 grep 으로 확인해 그대로 따랐다(훅 내부에서 `useTranslation()` 재호출하지 않음).
- mutateAsync 계열 의존성 타입은 손으로 적지 않고 `ReturnType<typeof useToggleTreeNode>['mutateAsync']` 형태로 유도했다(common.md §5.3). 해당 훅들은 `import type` 으로만 들여왔고 `typeof` 타입 질의로 참조했다 — `use-editor-file-persistence.ts` 의 `import type { useSetTabDirty } from '@entities/layout/layout.query'` 선례와 동일 패턴.

#### D-3. 소비처 전수 갱신 목록

| 소비처 | 갱신 여부 | 내용 |
|---|---|---|
| `src/widgets/git-panel/git-panel-container.tsx` | **무변경** (git diff 0줄) | `GitPanel` 컴포넌트 1개만 import — props 표면(33개) 무변경이라 호출부 수정 불필요함을 `git diff --stat` 로 확인 |
| `src/widgets/git-panel/git-panel.tsx` | 갱신 | `File`/`Minus`/`Plus`/`Undo2`(lucide) + `ContextMenu*` 5종 + `StatusRowItem`/`StatusRowAction` import 제거, `GitChangeGroup` import 추가, 3개 그룹 블록을 `<GitChangeGroup variant=.../>` 3호출로 치환 |
| `src/widgets/explorer/app-shell.tsx`(`ExplorerContainer` 유일 소비처) | **무변경** | `ExplorerContainerProps`(`{projectId}`) 및 export 시그니처 불변 — grep 으로 이 파일이 유일한 소비처임을 확인, import 경로·props 호출 모두 그대로 |
| `src/widgets/explorer/explorer-container.tsx` | 갱신 | CRUD/클립보드 관련 import 7개 제거(`FileTreeNodeKind`·`resolveEntryParentDir`·`validateEntryName`·`buildUniqueEntryName`·`FileTreeDraft`·`FileTreeRenameTarget` 등), 신규 훅 3개(`useExplorerEntryCrud`·`useExplorerClipboard`·`explorer-path`) import 추가, 로컬 `PATH_SEPARATOR`/`parentDirOf`/`joinPath` 정의 제거 후 `explorer-path.ts` 재수입, `contextMenuHandlers`/`ExplorerPanel` JSX/`EntryDeleteDialog` 호출부를 `crud.*`·훅 반환값 참조로 치환 |

#### D-4. 렌더 동등성 서술

`GitChangeGroup` 은 variant 별로 다음을 만들고 **동일한 단일 렌더 블록**(`<div><ResourceGroupHeader/>{rows.map(row=><ContextMenu key={row.path}><ContextMenuTrigger><StatusRowItem/></ContextMenuTrigger><ContextMenuContent>{entries.map(...)}</ContextMenuContent></ContextMenu>)}</div>`)에 흘려보낸다 — DOM 트리 형태 자체가 이전 코드의 3벌 JSX와 요소·중첩 수준에서 항등이다.

- **merge**: 헤더(action 없음, 이전과 동일) + 행마다 `kind='conflicted'` 고정, 액션 1개(open-file), 메뉴 5항목(openFile/openChanges(unstaged)/구분선/copyPath/reveal) — 이전 블록과 요소 수·순서 1:1 일치.
- **staged**: 헤더(action=unstageAll, 아이콘 Minus) + 행마다 `kind=row.staged`, 액션 2개(unstage/open-file), 메뉴 6항목(openFile/openChanges(staged)/unstage/구분선/copyPath/reveal) — 이전 블록과 1:1 일치.
- **unstaged**: 헤더(action=stageAll, 아이콘 Plus) + 행마다 `kind=row.unstaged`, 액션 3개(stage/discard/open-file), 메뉴 7항목(openFile/openChanges(unstaged)/stage/discard(`destructive`)/구분선/copyPath/reveal) — 이전 블록과 1:1 일치. `discard` 클릭이 `onDiscardRequest([row.path])`(=컨테이너의 `setDiscardTargets`) 호출로 확인 다이얼로그를 여는 흐름도 그대로 보존.
- `ContextMenuItem` 의 `variant={entry.destructive ? 'destructive' : undefined}` — `shared/ui/context-menu.tsx:85` 의 `variant = 'default'` 기본 파라미터는 `undefined` 명시 전달과 프롭 생략을 동일하게 취급하므로, destructive 가 아닌 6개 항목의 렌더 결과(class·`data-variant`)는 이전(속성 생략)과 바이트 단위로 동일.
- key 안정성: 행 `key={row.path}`(경로 불변 고유값, 이전과 동일), 액션 `key={action.id}`(`'open-file'`/`'unstage'`/`'stage'`/`'discard'` — 이전 리터럴 id 그대로 보존), 메뉴 항목 `key={entry.key}`(`'open-file'`/`'open-changes'`/`'unstage'`/`'stage'`/`'discard'`/`'sep-1'`/`'copy-path'`/`'reveal'` — 신규지만 각 variant 내 고유해 React 재조정에 영향 없음).

explorer 쪽은 함수 본문이 원문 그대로(문자 단위) 이동했고 상태·핸들러 이름도 동일해, 훅에서 반환되는 값을 `crud.`/구조분해로 참조하는 것 외에 로직 변경이 없다 — 렌더 트리는 `ExplorerPanel`/`EntryDeleteDialog` prop 값의 **출처**만 바뀌었을 뿐 값 자체와 계산 로직은 무변경이므로 항등.

#### D-5. 검증 결과

- `bun run typecheck` → exit 0 (변경 파일 6개 포함 전체 0 에러; 중간에 병행 작업 중이던 `contrast.ts` 의 일시적 미사용 변수 오류를 관측했으나 내 파일과 무관했고 이후 재실행에서 소거됨을 확인)
- `bunx prettier --check <6개 변경 파일>` → 통과 (수정 없음)
- `bunx eslint <6개 변경 파일>` → 출력 없음 (경고/에러 0)
- `bun test src/widgets/git-panel/ src/features/git/` → 28 pass / 0 fail (3 파일)
- `bun test`(전체) → 1435 pass / 0 fail (143 파일) — 전역 회귀 없음
- `git diff --stat src/widgets/git-panel/git-panel-container.tsx` → 무출력(0줄 변경) — GitPanel props 표면 33개 무변경 확인

### 3-E. Rust·번들 데이터 (f-3 정정 + f-4 린트 + 36종 재스윕)

### f-3·f-4 구현 완료 기록 — 임무 E(Rust·번들 데이터)

#### 0. 실사 — 업스트림 확정

github-dark.json·github-light.json은 `docs/utils/2026-08-12-w7-theme-original-sources.md`가 기록한 대로 primer/github-vscode-theme의 **classic**(dark.json/light.json, open-vsx vsix 배포) 변환본이다. `gh api`(인증된 GitHub 접근)로 해당 저장소의 `src/classic/theme.js`·`src/classic/colors.json`·`src/classic/primer.js`를 직접 대조했다.

- `src/classic/theme.js:151` `"editor.findMatchHighlightBackground": pick({ light: "#ffdf5d66", dark: "#ffd33d22" })` — 번들 JSON의 결함값과 정확히 일치(변환 파이프라인이 이 키를 폴백 후보로 삼았음을 재확인, task A의 f-1 분석과 합치).
- `list.highlightForeground`는 classic theme.js 전체(573줄)에 **미정의**(신형 main 브랜치 `src/theme.js`만 `"list.highlightForeground": color.accent.fg`를 갖되, 이는 @primer/primitives 기반 신형 팔레트로 classic과 색 체계가 달라 차용 불가 — classic 소비처는 자체 `src/classic/colors.json` 스케일만 사용).
- 따라서 계약 절차(list.highlightForeground 정의 시 그 값 / 미정의 시 업스트림 팔레트 최선 불투명 전경 후보)의 후자 분기를 적용했다.

#### 1. f-3 정정 표

원본 반투명값의 RGB부(`#ffd33d`/`#ffdf5d`)는 `src/classic/colors.json`의 `yellow` 스케일(라이트 원본 배열, 다크는 `primer.js`가 역순으로 재사용)에서 그대로 온 값이다 — dark는 `darkColors.yellow[4]`, light는 `lightColors.yellow[4]`(둘 다 원본 인덱스 4). 이 스케일 자체가 "업스트림 팔레트"이므로, 같은 스케일 안에서 `panel.background` 대비 실측 대비비가 가장 우수한 불투명 인덱스를 재선정했다(같은 배열에서 고른다는 점에서 "충실", WCAG로 실제 판독 가능한 인덱스를 고른다는 점에서 "불투명 전경 후보" 요건 충족).

| 테마 | 기존값(알파) | 신값 | panel.background | 신값 대비비 | 근거(sourceBasis) |
|---|---|---|---|---|---|
| github-dark | `#ffd33d22`(13%) | `#ffd33d` | `#1f2428` | **10.91:1** | classic은 `list.highlightForeground` 미정의. 원 반투명값의 RGB부가 이미 `darkColors.yellow[4]`(=classic `colors.json` yellow 스케일을 `primer.js`가 dark용으로 역순 재배열한 배열의 index 4) — 다크 배경에서는 이 인덱스 자체가 이미 최우수 대비(스케일 전체 재계산 결과 index0~9 중 index4가 이미 AA 상회, index 상향 시 더 밝아질 뿐 실익 없음)라 원 색상(같은 인덱스)을 그대로 불투명화. `statusIndicator.warning`(번들 JSON 기존값 `#ffea7f` = `editorWarning.foreground: primer.yellow[6]`)도 동일 스케일의 불투명 전경 채택 선례로 교차 확인. |
| github-light | `#ffdf5d66`(40%) | `#735c0f` | `#f6f8fa` | **6.04:1** | 원 반투명값의 RGB부(`lightColors.yellow[4]`)를 그대로 불투명화하면 옅은 노랑이 거의 흰색 배경에 묻혀 1.24:1(AA 대실패) — 동일 스케일(`colors.json` yellow, 라이트 원본 배열) 10개 인덱스를 전수 계산한 결과 index9(`#735c0f`, 스케일 최암값)만 유일하게 AA(4.5:1) 통과(index8 `#b08800`=3.10:1로 미달). 같은 배열 안에서 인덱스를 옮겨 실제 판독 가능한 값을 고른 것으로, 신규 색상 발명이 아니라 업스트림이 이미 정의한 10개 중 선택. |

검증: `git diff`로 두 파일 모두 해당 1줄씩만 변경됐음을 확인(그 외 623줄 무변경).

#### 2. 36종 재스윕(정정 후, panel.background 알파 합성 + WCAG 대비)

스크래치패드 1회용 bun 스크립트(`sweep-match-highlight.ts`, contrast.ts와 동일한 sRGB 상대휘도·합성 알고리즘 이식)로 36개 번들 테마 전량을 계산했다. **반투명(8자리 hex, 알파<ff) 후보는 정정 전 스윕에서도 github 2종뿐**이었고(별도로 원본 데이터를 임시 복원해 재확인 — §3), 정정 후에는 0종이다.

> **검토 반영(L2-1 confirmed, L2-2·L2-3 downgraded)**: 이 표는 원래 "AA(4.5)" 한 열만으로
> 판정해, 파이프라인이 실제로 강제하는 게이트(`MIN_CONTRAST_RATIO = 3`, `contrast.ts`)
> 위반 4종(ayu-light·everforest-light·rose-pine-dawn·solarized-light)이 "AA 미달 6종"
> 서술 속에 vscode-abyss·vscode-quiet-light(게이트는 통과, AA만 미달)와 뭉뚱그려져
> 은폐됐다. 아래 표는 두 기준을 별개 열로 분리하고, 게이트(3) 위반 4종 중 ayu-light·
> solarized-light 는 업스트림 팔레트 내 재선정으로 정정(§3-A 3.3, `docs/theme-system.md`
> §8.2.3), everforest-light·rose-pine-dawn 은 정정 후보가 없어 테스트 예외로 등재했다
> (`bundled-theme-contrast.test.ts` — §4 참고).

| id | matchHighlight | panel.background | 대비비 | 게이트(3) | AA(4.5) |
|---|---|---|---|---|---|
| ayu-dark | #e6b450 | #0d1017 | 9.98 | PASS | PASS |
| **ayu-light** | **#7e4b01(정정)** | #f8f9fa | **6.88** | **PASS(정정으로 신규 통과)** | **PASS(정정으로 신규 통과)** |
| catppuccin-mocha | #cba6f7 | #181825 | 8.64 | PASS | PASS |
| darcula | #569CD6 | #242424 | 5.26 | PASS | PASS |
| dracula | #8BE9FD | #21222C | 11.41 | PASS | PASS |
| everforest-dark | #a7c080 | #2d353b | 6.23 | PASS | PASS |
| everforest-light | #8da101 | #fdf6e3 | 2.69 | **FAIL(예외 등재 — 업스트림에 대체 후보 없음)** | FAIL(기존 잔존) |
| **github-dark** | **#ffd33d(정정)** | #1f2428 | **10.91** | **PASS(정정으로 신규 통과)** | **PASS(정정으로 신규 통과)** |
| **github-light** | **#735c0f(정정)** | #f6f8fa | **6.04** | **PASS(정정으로 신규 통과)** | **PASS(정정으로 신규 통과)** |
| gruvbox-dark | #689d6a | #282828 | 4.65 | PASS | PASS |
| intellij-islands-light | #0033B3 | #FFFFFF | 9.88 | PASS | PASS |
| kanagawa-wave | #7E9CD8 | #1F1F28 | 5.94 | PASS | PASS |
| monokai | #f8f8f2 | #1e1f1c | 15.54 | PASS | PASS |
| night-owl-light | #403f53 | #F0F0F0 | 8.97 | PASS | PASS |
| night-owl | #ffffff | #011627 | 18.33 | PASS | PASS |
| nord | #88c0d0 | #2e3440 | 6.24 | PASS | PASS |
| one-dark-pro | #ecebeb | #21252b | 12.94 | PASS | PASS |
| one-monokai | #C5C5C5 | #21252b | 8.92 | PASS | PASS |
| palenight | #ffffff | #292D3E | 13.64 | PASS | PASS |
| rose-pine-dawn | #d7827e | #faf4ed | 2.60 | **FAIL(예외 등재 — 업스트림에 대체 후보 없음)** | FAIL(기존 잔존) |
| rose-pine | #ebbcba | #191724 | 10.45 | PASS | PASS |
| **solarized-light** | **#584c27(정정)** | #EEE8D5 | **6.92** | **PASS(정정으로 신규 통과)** | **PASS(정정으로 신규 통과)** |
| tokyo-night | #668ac4 | #16161e | 5.14 | PASS | PASS |
| vitesse-dark | #4d9375 | #121212 | 5.13 | PASS | PASS |
| vitesse-light | #1c6b48 | #ffffff | 6.47 | PASS | PASS |
| vscode-abyss | #0063a5 | #060621 | 3.15 | PASS | FAIL(기존 잔존) |
| vscode-dark-modern | #569CD6 | #181818 | 6.02 | PASS | PASS |
| vscode-dark-plus | #569CD6 | #1E1E1E | 5.65 | PASS | PASS |
| vscode-kimbie-dark | #e3b583 | #362712 | 7.70 | PASS | PASS |
| vscode-light-modern | #0066BF | #F8F8F8 | 5.41 | PASS | PASS |
| vscode-light-plus | #0066BF | #FFFFFF | 5.75 | PASS | PASS |
| vscode-monokai-dimmed | #e58520 | #272727 | 5.48 | PASS | PASS |
| vscode-quiet-light | #9769dc | #F2F2F2 | 3.49 | PASS | FAIL(기존 잔존) |
| vscode-red | #ff4444 | #330000 | 5.40 | PASS | PASS |
| vscode-solarized-dark | #1ebcc5 | #00212B | 7.23 | PASS | PASS |
| vscode-tomorrow-night-blue | #bbdaff | #001c40 | 11.76 | PASS | PASS |

**각주 — builtin 2행(카탈로그 포함, 이 표의 36행에는 없음, L2-3 downgraded)**: `taide-light`
(`#df8e1d` on `#e6e9ef` = **2.15 FAIL** — 게이트(3)·AA(4.5) 모두 미달, `service.rs` 하드코딩
값, 선재 — 이번 배치가 만든 값 아님, Catppuccin Latte 팔레트 그대로), `taide-dark`
(`#f9e2af` on `#181825` = **13.81 PASS**). **이 표는 번들 36종 대상이고, 앱이 실제로
노출하는 카탈로그는 builtin 2 를 포함해 38종이다** — builtin 2 는 JSON 파일이 아니라
Rust 하드코딩(`builtin_dark()`/`builtin_light()`)이라 이 스윕 스크립트의 파일 순회
대상이 아니다. `taide-light` 는 카탈로그 전체 최저값(everforest-light 2.69·ayu-light
정정 전 2.16보다도 낮다)이지만 `DEFAULT_THEME_ID = "taide-dark"`(`follow_system_theme`
기본 `false`)라 아웃오브박스 노출값은 아니다. 정정 여부는 별도 결정 사안 — §5 이월 참고.

**게이트(3) 는 통과하지만 AA(4.5) 는 미달하는 2종(vscode-abyss 3.15·vscode-quiet-light
3.49)은 정정 대상이 아니다** — 계약 §1.2 f-3 이 강제하는 게이트는 3:1(`MIN_CONTRAST_RATIO`)
이고, 이 값 이상이면 파이프라인·테스트 어느 쪽도 위반으로 잡지 않는다. **게이트(3) FAIL
2종(everforest-light·rose-pine-dawn)은 업스트림 팔레트 안에 대체 가능한 같은 색상 계열
후보가 없어(everforest: `green` 유일, 더 어두운 변형 없음 — 더 밝은 `dimGreen` 뿐. rose-pine
dawn: `rose` 유일, 가장 가까운 `love` 는 이미 error 색으로 쓰이는 별개 accent) 정정하지
않고 `bundled-theme-contrast.test.ts` 의 명시 예외 목록에 사유와 함께 등재했다(§4 참고,
d-26b `LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS` 선례와 동일 패턴). 나머지 AA 미달 2종
(vscode-abyss·vscode-quiet-light)은 게이트를 통과하므로 이번 배치 지시(§1.2 f-3 "FAIL
발견 시 데이터 정정 포함" 의 FAIL = 게이트 위반)의 정정 대상이 아니다 — 전경-only 렌더
(f-5, search-match-row 전환)로 완전 비가시 실패 모드는 이미 해소된 상태이므로 수치만
기록한다.

#### 3. 재현 — 정정 전 FAIL / 정정 후 PASS 실측

`git stash`/커밋 없이 작업트리만 조작(d-26b §3.3 선례): `git show HEAD:<path>`로 원본(반투명) JSON을 스크래치패드에 백업한 뒤 `cp`로 두 파일을 원본으로 임시 복원 → 신규 테스트만 단독 실행:

```
thread '...번들_테마는_panel_매치_하이라이트가_불투명하다' panicked:
panel.matchHighlight defects in bundled themes:
'github-dark': panel.matchHighlight("#ffd33d22") is translucent — ...
'github-light': panel.matchHighlight("#ffdf5d66") is translucent — ...
test result: FAILED. 0 passed; 1 failed
```

계약이 지목한 정확히 그 2건(github-dark `#ffd33d22`·github-light `#ffdf5d66`)에서만 실패함을 확인했다. 이후 `Edit`으로 정정값을 재적용(정정 전후 `git diff`가 각 파일 1줄임을 재확인)하고 재실행해 PASS(28 passed)를 확인했다.

#### 4. f-4 — Rust 데이터 린트 확장

`src-tauri/src/domain/theme/service.rs`의 `mod tests`(기존 `번들_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다` 테스트 직후, :1305)에 `번들_테마는_panel_매치_하이라이트가_불투명하다()`를 추가했다. `normalize_hex_color`(d-26b가 추가한 기존 헬퍼 — 6자리 hex를 `ff` 알파로 패딩, 기존 알파는 보존)를 그대로 재사용해 `panel.matchHighlight`가 없으면 스킵, 있으면 정규화 후 `ends_with("ff")`가 아니면 위반으로 수집한다. `LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS` 선례와 달리 화이트리스트 상수는 만들지 않았다 — 정정 후 36종 전량이 예외 없이 통과하기 때문(향후 반투명 값이 재유입되면 즉시 FAIL해야 하므로 예외를 미리 만들어두지 않는 것이 원칙에 부합).

#### 5. 경계 판단 — 왜 이 파일들만인가

- TS(`shared/lib/theme-convert/*`, `search-match-row.tsx` 등)는 task A(f-1·f-2·f-5) 소관으로 이미 완료돼 있어 **전혀 건드리지 않았다**(`git status`로 세션 시작 전 대비 무변경 확인 — 다른 병렬 에이전트가 만든 diff만 존재).
- `src-tauri/src/domain/theme/service.rs` 외 Rust 파일(예: `types.rs`)은 매치하이라이트 데이터 형태·필드 정의를 바꾸지 않았으므로(값 자체만 정정) 손댈 필요가 없었다.
- 소비처 갱신: **없음** — 이번 변경은 데이터 값 정정(JSON 2줄)과 테스트 추가(Rust 1개 함수)뿐이라, 컴파일 경로·import를 갱신해야 할 소비처가 존재하지 않는다(구조 분할이 아닌 데이터 결함 수정이므로 d-30류 "소비처 import 경로 갱신" 항목이 발생하지 않음 — 이 계약 §1.2 f 자체가 "결함 수정만 예외"로 명시).

#### 6. 검증

- `cargo fmt --all --check` — 통과(diff 없음).
- `cargo test -p taide domain::theme::service` — 28 passed, 0 failed(신규 린트 포함). 재현 절차는 §3.
- `cargo clippy -p taide --lib -- -D warnings` — 경고 0건(범위 외 저비용 사전 점검).
- `git diff --stat -- src/shared/api/bindings.ts src/shared/generated` — 무변경(빈 diff), `git status`로 `src/` 아래 TS 무수정 확인(다른 병렬 에이전트 변경분 외 내 변경 없음).
- `git diff -- src-tauri/resources/themes/github-dark.json src-tauri/resources/themes/github-light.json` — 각 파일 정확히 1줄만 변경.
- typecheck/prettier: 해당 없음(Rust 전용 임무, TS 미수정).

## 5. 이월 (d-33 몫)

검토 반영 회전(L2-2·L2-3 downgraded)에서 major 에는 못 미치지만 코드 변경이 필요한 것으로
판정된 항목을 다음 통합 배치로 넘긴다. 이번 회전에서는 데이터·문서만 정정했고 아래 3건은
**착수하지 않았다**.

### 5.1 matchHighlight vs 본문 전경(app.foreground) 구별성 가드 (L2-2 downgraded)

- **문제**: `panel.matchHighlight` 는 지금까지 `panel.background` 대비로만 게이트를
  통과시켰다(d-26 §4.3·본 계약 §3-E 재스윕 표) — 그런데 f-5(search-match-row 전경 전환)
  이후 실제 렌더 맥락에서는 매치 강조 글자가 **본문 텍스트(`app.foreground` 상속)와 나란히**
  놓인다. 번들 3종(`monokai`·`night-owl-light`·`palenight`)은 `panel.matchHighlight` 가
  `app.foreground` 와 hex 완전 동일(ΔE=0)해서, 검색 패널·⌘P 팔레트 모두에서 매치 강조가
  `font-semibold` 굵기 하나에만 의존한다.
- **심각도가 이번 회전에서 major 에 못 미친 이유**: (a) 실패 모드가 "내용 손실"이 아니라
  "강조 손실"이다 — 글자 자체는 여전히 읽힌다. (b) 영향 범위가 3/36(8%)로 좁고, 나머지는
  ΔE 5.4(one-monokai)·7.2(kimbie-dark) 를 빼면 전부 13.8 이상으로 뚜렷이 구분된다.
  (c) 선재 — `git show HEAD`의 3종 데이터와 ⌘P `HighlightedText`(d-26 이후 이미 동일 클래스
  조합)가 이번 배치 이전부터 이 상태였다. f-5 는 두 표면(검색·팔레트)을 **일치**시켰을 뿐
  새 결함 계열을 만들지 않았다.
- **가드 설계 시 반드시 지킬 제약 — WCAG 대비비를 지표로 쓰지 말 것**: `github-dark` 의
  `panel.matchHighlight`(`#ffd33d`, 노랑) vs `app.foreground`(`#d1d5da`, 회색)는 WCAG
  대비비가 **1.03**(둘 다 밝아서 광도가 비슷)이지만 육안으로는 명확히 구분된다(CIE Lab
  **ΔE 77.7**). 대비비 기준으로 가드를 만들면 이런 정상 테마를 대량 오탐한다 — **ΔE(또는
  최소한 hue 차 병행)** 를 지표로 쓰고, 임계값은 36종 실측 분포(0 / 5.4 / 7.2 / 13.8 …,
  0과 5.4 사이 간극이 가장 크다)를 근거로 정한다.
- **후보 방향(택1, 둘 다 이번 배치 범위 밖)**:
  1. 파이프라인 가드 확장(권장) — `mapping-tables.ts` 의 `panel.matchHighlight` derived 술어에
     "`app.foreground`/`editor.foreground` 와 ΔE 로 지각 동일한 후보 배제"를 추가하고,
     배제되면 기존 경로대로 `SAFE_DEFAULT_COLORS.status` 로 폴백시킨다. `resolve-colors.ts`
     의 resolve 순서상 `app.foreground` 확정 시점을 먼저 실사해야 한다.
  2. `CONTRAST_REPAIR_FOREGROUND_CANDIDATES['panel.matchHighlight']` 를 상태색 우선으로
     재정렬 — 현재 사슬(`['editor.foreground', 'foreground']`)이 "대비는 확보하되 본문과
     동일색"이라는 국소최적에 갇힌다.
  - 어느 쪽이든 번들 3종은 **수리 경로가 발동하지 않는(대비 15.54/8.97/13.64로 게이트를
    이미 통과) 데이터 문제**이므로 별도로 `src-tauri/resources/themes/*.json` 값 정정 또는
    Rust 린트에 "matchHighlight != app.foreground" 규칙 추가가 필요하다.
  - 착수 시 d-26 §4.3·본 계약 §3-E 의 재스윕 표에 **vs 본문 전경 축**을 추가해야 한다(현재
    두 계약 모두 vs `panel.background` 축만 보유).

### 5.2 builtin_light `panel.matchHighlight` 저대비(2.15) 정정 여부 — 사용자 결정 필요 (L2-3 downgraded)

- **실측**: `taide-light`(builtin, `service.rs` 하드코딩)의 `panel.matchHighlight`
  `#df8e1d` vs `panel.background` `#e6e9ef` = **2.15** — 이번 계약 §3-E 표의 번들 36종
  전체 최저값(everforest-light 정정 전 2.16)보다도 낮은, **앱 카탈로그(38종) 전체 최저치**다.
  `taide-dark` 는 13.81 로 문제 없다. 두 값 모두 Catppuccin Latte/Mocha 팔레트 그대로이며
  선재값이다(`git diff HEAD -- service.rs` 상 이 두 값은 변경되지 않았다 — 이번 배치가
  만든 결함이 아니다).
- **f-4 린트가 이 값을 잡지 못하는 이유가 두 겹**: ① f-4 신규 린트
  (`번들_테마는_panel_매치_하이라이트가_불투명하다`)는 **불투명 여부만** 검사하는 게이트라
  `#df8e1d`(6자리, 이미 불투명)는 애초에 이 종류의 검사로는 걸리지 않는다 — 번들 6종의
  기존 AA 미달 잔존과 같은 사정. ② 설사 대비 검사로 바꿔도 `bundled_themes()` 는
  `BUNDLED_THEME_SOURCES`(JSON 파일 36종)만 순회하고, builtin 2종은 Rust 코드
  (`builtin_dark()`/`builtin_light()`) 로 별도 정의돼 있어 순회 대상에 없다(기존
  list 활성-배경 린트도 동일한 선재 갭을 가진다).
- **실사용 영향은 제한적**: `DEFAULT_THEME_ID = "taide-dark"`, `follow_system_theme` 기본값
  `false`(`settings/types.rs`) — 사용자가 명시로 `taide-light` 를 고르거나 시스템 팔로우를
  켠 상태에서 OS 가 light 일 때만 노출된다.
- **결정 필요 사항(다음 배치에서 §3.1 방식으로 한 번에 제시)**:
  1. `#df8e1d` 를 Catppuccin Latte 팔레트 안(같은 yellow/peach 계열의 더 어두운 인덱스)에서
     `panel.background` 대비 4.5:1 통과 값으로 재선정할지 — 번들 6종(everforest-light 등,
     이번 배치에서 정정하지 않고 잔존으로 기록한 것)과의 처분 일관성을 함께 고려해야 한다.
  2. builtin 2종을 f-4 류 린트의 순회 대상에 편입할지, 편입한다면 **불투명 게이트**가 아닌
     **대비 게이트**를 신설할지 — 대비 게이트를 신설하면 번들 6종(everforest-light·
     rose-pine-dawn·vscode-abyss·vscode-quiet-light 등)이 즉시 위반으로 걸리므로,
     d-26b `LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS` 방식의 명시 예외 목록(또는 비-assert
     경고)이 함께 필요하다.

### 5.3 이번 배치에서 신설된 폴백 예외 목록 (§1 참고, 위 5.2 와 병합 검토 대상)

`src-tauri/resources/themes/*.json` 36종 중 아래 2종은 업스트림 팔레트 안에 게이트(3) 통과
후보가 없어 정정하지 않고 `src/shared/lib/theme-convert/bundled-theme-contrast.test.ts` 의
`MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS` 에 사유와 함께 등재했다(§3-E 표 각주 참고).

| id | matchHighlight | panel.background | 대비비 | 사유 |
|---|---|---|---|---|
| everforest-light | #8da101 | #fdf6e3 | 2.69 | 업스트림 foreground 팔레트(`sainnhe/everforest-vscode`)에 `green` 유일 — 더 어두운 변형 없음, 더 밝은 `dimGreen`(#a4bb4a) 뿐 |
| rose-pine-dawn | #d7827e | #faf4ed | 2.60 | 업스트림 Rose Pine Dawn 팔레트에 `rose` 유일 — 더 어두운 변형 없음, 가장 가까운 `love`(#b4637a)는 이미 error 색으로 쓰이는 별개 accent |

두 항목 모두 5.2 의 builtin_light 정정 여부 결정과 같은 축(업스트림에 대체 후보가 없는
저대비 색을 어떻게 처분할 것인가)이므로, d-33 착수 시 함께 검토한다.

---

## 4. 검토 반영 (2026-08-24)

> 검토 wf_d97bca8c-55a(4렌즈 opus+xhigh): major 4·minor 19(중복 포함). 적대적 wf_f6fa503f-994
> (opus+high 건별): **L2-1 confirmed** / L2-0·L2-2·L2-3 downgraded(사실이나 major 아님 — 문서
> 종결·이월). L2-1 처분은 사용자 결정 = **A안+폴백**(업스트림 스케일 내 정정, 후보 없으면 명시
> 예외). 수정 wf_f95bc734-e73(sonnet+xhigh 3병렬 — 세션 재시작으로 종료 기록 유실됐으나 저널
> 실사로 3/3 완주 확인·이어받기 불요). 라이브 문서 4곳(architecture·ipc-contract·features/
> lsp·features/editor)의 경로 정정과 이 계약의 §0/§3 문언·수치 정정(§4.5)은 메인이 직접 반영.

### 4-F1. 테마·데이터 (L2-1 A안+폴백·L2-0/L2-3 종결·L2-4·L2-5·L4-6·이월 기록)

### 4. 검토 반영 (F1 — 테마·데이터 수정)

#### L2-1 (confirmed) — 번들 4종 matchHighlight 게이트(3) 위반, 사용자 결정 A안+폴백 적용

착수 전 업스트림 원본을 직접 대조(`ayu-theme/vscode-ayu` `ayu-light.json`, `sainnhe/everforest-vscode` `everforest-light.json`+`src/palette/light/foreground.ts`, `rose-pine/vscode` `rose-pine-dawn-color-theme.json`, `microsoft/vscode` `theme-solarized-light-color-theme.json`)한 결과, 4종 모두 현재값이 `list.highlightForeground`(업스트림이 직접 지정한 불투명 accent)에서 그대로 온 값임을 확인했다.

- **ayu-light**: `#f29718`(대비 2.16) → **`#7e4b01`(대비 6.88, AA 도 통과)**. 근거: 같은 업스트림 파일의 `button.foreground`(`scheme.common.accent.on` — accent 의 어두운 "on" 변형, hue 34.95°→35.52°로 동일 계열). `repairContrastPairs` 의 `editor.foreground` 폴백(무채색)은 사용하지 않음 — 테마 정체성 유지.
- **solarized-light**: `#B58900`(대비 2.62) → **`#584c27`(대비 6.92, AA 도 통과)**. 근거: 같은 업스트림 파일의 `activityBar.foreground`(hue 45.41°→45.31°, gold/olive 동일 계열의 어두운 변형).
- **everforest-light**(`#8da101`, 2.69)·**rose-pine-dawn**(`#d7827e`, 2.60): 업스트림 팔레트 전수 조사(HSV hue 스캔) 결과 같은 색상 계열의 더 어두운 대체값이 **존재하지 않음**(everforest 는 `green` 유일, 더 밝은 `dimGreen` 뿐; rose-pine dawn 은 `rose` 유일, 가장 가까운 `love` 는 이미 error 색으로 쓰이는 별개 accent) — 정정하지 않고 폴백(§2 신설 테스트의 명시 예외 목록에 등재)으로 처리.
- 정정 표·재스윕 표는 계약 §3-E 에 게이트(3)/AA(4.5) 열을 분리해 반영(아래 L2-2/L2-3 처리와 병합 정정).

#### L2-1 관련 — 재발 방지 테스트 신설

기존 TS 테스트 중 번들 리소스(`src-tauri/resources/themes/*.json`)를 읽는 선례가 없어(실사 확인) 신설했다. `src/shared/lib/theme-convert/bundled-theme-contrast.test.ts` — `node:fs`(`readdirSync`/`readFileSync`) + `import.meta.dir` 상대경로로 36종을 로드하고, `contrast.ts` 의 `validateOutputColors`(정본 그대로 재사용, 알고리즘 재구현 없음)로 0 에러를 단언한다. 타입은 `@shared/api/bindings` 의 `Theme_Serialize` 에서 유도(손으로 재정의하지 않음). 폴백 2건은 `MATCH_HIGHLIGHT_CONTRAST_EXEMPTIONS`(영어 JSDoc + 사유 문자열, d-26b `LIST_ACTIVE_BACKGROUND_LINT_EXEMPTIONS` 선례와 동일 패턴)로 등재하고, 별도 테스트로 "예외 등재분은 실제로 matchHighlight 사유로만 위반한다"를 고정했다. 실측: `bun test` 2 pass/0 fail/6 expect().

#### L2-0 (downgraded) — 문서 종결

사실관계(4종 중 github 2종은 safe-default 로, ayu-light/solarized-light 는 업스트림 원본값 그대로 재변환됨)는 검증에서 확정됐으나 major 미달로 downgraded. fixGuidance 대로 코드는 그대로 두고 `docs/theme-system.md` §8.2.2 인접에 **§8.2.3 "재변환 비재현 예외"** 를 신설했다 — 대상 4종(github 2종+이번에 정정한 ayu-light·solarized-light), 재변환 시 나오는 값과 이유(github: f-1 가드가 두 후보 모두 배제→safe-default `#569CD6`/`#0066BF`, ayu/solarized: `list.highlightForeground` 자체가 불투명해 가드 통과→원본 결함값 그대로 재현), 손수정 유지 근거 표(대비 실측 포함), "운영 지시"(비-zero diff 는 예상된 것 — 산출물 채택 대신 표의 값 재적용), d-31 계약 §3-A/§3-E 상호 참조를 전부 포함했다.

#### L2-7 (minor) — 파이프라인 정본 문서 갱신

`docs/theme-system.md` §8.2 "변환 파이프라인" 서술에 2개 단락을 추가했다: ① `panel.matchHighlight` 가 `chain()` 이 아니라 `derived()`+`isOpaqueForegroundCandidate` 가드로 계산되는 경로(후보·배제 조건·safe-default 귀결), ② `CONTRAST_PAIRS` 5쌍(app/editor/panel/tooltip/matchHighlight)·`MIN_CONTRAST_RATIO=3`·알파 합성(`compositeOverBackground`) 후 측정하는 규칙과 수리/임포트 거부 관계. §8.2.3(위 L2-0 처리와 동일 절)도 이 목적을 겸한다.

#### L2-4 (minor) — 합성 항등 주장 정정

`vitesse-dark` 의 `app.foreground`/`editor.foreground`/`panel.sectionHeader` 가 전부 8자리 hex(`#dbd7caee`, 알파 93%)임을 실측 확인. 프로젝트의 실제 `contrastRatio` 로 재계산한 결과 **13.01 → 11.43**(둘 다 게이트 통과, 판정 반전은 없음 — 문서에도 그대로 반영)로 정확히 일치했다.

- 계약 §3-A "3.4" 절의 "기존 4쌍의 계산 결과는 완전히 동일하다" 문장을 "6자리 전경에 한해 항등 — 8자리 전경(vitesse-dark `#dbd7caee`, 13.01→11.43)은 합성 후 측정으로 의도 변경"으로 정정.
- `contrast.test.ts` 에 8자리 전경을 가진 기존 pair(`app`) 케이스 1건 추가: `app.foreground='#00000030'`(반투명 검정) on `app.background='#ffffff'` — 알파를 무시하면 21.00(통과)이었을 값이 합성 후 1.56(저대비)으로 **판정이 뒤집히는** 사례로 의도를 고정(L2-4 원 evidence 의 반전 사례를 그대로 재현해 사용). 실측값(21.00→1.56)은 프로젝트 알고리즘으로 독립 재검증 완료.

#### L2-5 (minor) — isOpaqueForegroundCandidate 4자리 #rgba 보강

`mapping-tables.ts` 의 길이 기반 판정(`length !== 9` 이면 무조건 불투명 취급)이 4자리 축약형(`#rgba`, 전체 길이 5)을 놓치는 구멍을 확인 — 상류 `expandVscodeHex`(`jsonc.ts`, `merge.ts` 에서 적용) 정규화에 암묵 의존하고 있었다.

- `SHORT_HEX_ALPHA_LENGTH = 5`/`SHORT_ALPHA_CHANNEL_MAX = 15` 상수를 추가하고, `candidateHex.length === 5` 분기에서 `candidateHex.slice(-1)`(마지막 알파 니블)이 `f`(15)인지로 정확히 판정하도록 보강. 3/6/8자리 기존 로직은 무변경.
- JSDoc 에 "이 모듈의 호출자는 항상 `expandVscodeHex`(`jsonc.ts`, `merge.ts` 적용)로 정규화된 값을 넘긴다 — 4자리 분기는 정규화 여부와 무관하게 술어 자체가 정확하도록 하기 위함"을 명시.
- 테스트 1건 추가(`mapping-tables.test.ts`, 신설): `#fd03`(알파 3/15) → false, `#fd0f`(알파 f) → true.

#### L4-6 (minor, 검토 의견 수용) — 알파 합성 유틸 승격

`shared/lib/color.ts` 에 `compositeOverBackground(foregroundHex, backgroundHex)` 를 신설(8자리 아니면 항등 — 원 두 사본과 로직 100% 동일하게 이식)하고, `contrast.ts` 의 `compositeForegroundOverBackground`·`resolve-syntax.ts` 의 `compositeAlphaOverBackground` 사설 사본을 제거해 이 함수를 import 하도록 교체했다. 중복 상수 `HEX_ALPHA_LENGTH`(9)·`ALPHA_CHANNEL_MAX`(255) 는 `color.ts` 에서 export 하는 1곳으로 흡수하고, `contrast.ts`(전량 제거, 더 이상 미사용)·`resolve-syntax.ts`(`HEX_ALPHA_LENGTH` 만 여전히 필요해 import 로 교체)·`mapping-tables.ts`(둘 다 import 로 교체) 3곳의 사설 정의를 모두 제거했다. 동작 무변경 — `bun test`(전체) 1447 pass/0 fail 로 회귀 없음 확인(직전 배치 1435 pass 대비 +12 는 전부 이번 세션 신규 테스트).

#### L2-2 (downgraded) — 이월 기록 (§5.1)

matchHighlight vs `panel.background` 축만 게이트했을 뿐 vs 본문 전경(`app.foreground`) 구별성은 두 계약(d-26·d-31) 모두 미측정임을 확인(번들 3종 monokai/night-owl-light/palenight 이 `panel.matchHighlight === app.foreground`, ΔE=0). major 미달 사유(내용 손실이 아닌 강조 손실, 3/36 로 범위 좁음, 선재)로 이번 배치에서는 코드를 건드리지 않고, 계약 신설 §5.1 에 문제 정의·심각도 판단 근거·**WCAG 대비비 금지 근거(github-dark 대비비 1.03 인데 ΔE 77.7 로 오탐)**·후보 방향 2가지·번들 3종 데이터를 전부 이월 기록했다.

#### L2-3 (downgraded) — 이월 기록 (§5.2) + 표 각주 정정 (§3-E)

`taide-light`(builtin) 의 `panel.matchHighlight` `#df8e1d` vs `panel.background` `#e6e9ef` = **2.15**(프로젝트 알고리즘으로 재검증 완료, 카탈로그 38종 전체 최저)를 f-3 스윕(번들 36종)·f-4 린트(불투명 게이트만) 둘 다 놓치는 구조적 이유(JSON 파일이 아니라 Rust 하드코딩)를 계약 §3-E 표 아래 각주로 명시하고, "표는 번들 36종, 카탈로그는 builtin 2 포함 38종" 임을 함께 기록했다. `taide-dark` 는 13.81(PASS)로 문제 없음. 정정 여부(같은 Catppuccin Latte 팔레트 내 재선정 가능성)와 대비 린트 신설 정책(신설 시 번들 6종이 즉시 위반되므로 예외 목록 설계 필요)은 사용자 결정이 필요해 계약 신설 §5.2 에 결정 옵션 2가지로 정리해 이월했다.

#### §3-E 표 보강 (L2-1·L2-2·L2-3 공통 처리)

재스윕 표에 "게이트(3)" 열을 "AA(4.5)" 열과 분리 표기해, 기존에 "AA 미달 6종"으로 뭉뚱그려졌던 서술 속에서 파이프라인이 실제로 강제하는 게이트(3) 위반 4종(ayu-light·everforest-light·rose-pine-dawn·solarized-light)과 AA 만 미달하는 2종(vscode-abyss 3.15·vscode-quiet-light 3.49, 게이트는 통과)을 구분했다. ayu-light·solarized-light 행은 정정값·대비(6.88/6.92)로 갱신, everforest-light·rose-pine-dawn 행은 "FAIL(예외 등재)" 로 갱신. builtin 2행 각주(taide-light 2.15 FAIL·taide-dark 13.81 PASS)를 추가했다.

#### 이월 (d-33 몫) — 계약 "## 5. 이월" 신설

① matchHighlight vs 본문 전경 구별성 가드(L2-2 이월, ΔE 기반 설계·WCAG 금지 근거·번들 3종 데이터), ② builtin_light 2.15 정정 여부+대비 린트 신설 정책(L2-3 이월, 사용자 결정 필요 옵션 2가지), ③ 이번 배치에서 신설된 폴백 예외 목록(everforest-light·rose-pine-dawn, 사유 포함) — 3개 절 모두 신설해 기록했다.

### 4-F2. Rust (L2-6 normalize_hex_color 3/4자리 보강)

#### L2-6 — Rust 린트 `normalize_hex_color` 3/4자리 hex 오탐 보강 (처리 완료)

**대상**: `src-tauri/src/domain/theme/service.rs:990-1010`(`normalize_hex_color` 헬퍼), `mod tests` 내 신규 단위 테스트 2건(:1012-1020 부근).

**원 발견(minor, L2-6)**: 헬퍼가 길이 6일 때만 `ff` 알파를 패딩하고 그 외(3·4자리 포함)는 원문을 그대로 반환해, 3자리 불투명 `#fc0`(→ `"fc0"`, `ends_with("ff")` 실패)과 4자리 불투명 `#fc0f`(→ `"fc0f"`, 역시 실패)가 신규 `panel.matchHighlight` 불투명 린트에서 **오탐 FAIL**로 잡히는 잠재 결함. 현재 번들 36종은 전부 6/8자리라 실해는 없음(방어 보강 성격).

**처리**: `normalize_hex_color`에 `match trimmed.len()` 분기를 3→4→6→그 외(catch-all, 8자리 포함) 순으로 확장했다.
- 3자리(`abc`): 클로저 `expand_shorthand`로 각 니블을 두 번씩 복제(`a`→`aa` 등)해 6자리로 만든 뒤 `ff` 알파를 덧붙여 8자리로 정규화. 3자리 hex 표기 자체가 알파 채널을 갖지 않으므로 항상 불투명 취급이 맞다.
- 4자리(`abcf`): 동일한 `expand_shorthand`로 색 니블 3개와 알파 니블 1개를 **함께** 복제해 8자리로 정규화 — 알파 니블도 색 니블과 동일하게 복제 대상이므로 별도 분기 없이 재사용 가능했다(`f`→`ff`, `8`→`88` 등). 이 덕분에 4자리 불투명값(`#fc0f`→`"ffcc00ff"`)과 4자리 반투명값(`#fc08`→`"ffcc0088"`)이 각각 정확히 opaque/translucent 로 정규화된다.
- 6자리·그 외(8자리 포함)는 원 로직 그대로 유지 — 8자리는 이미 정규 형태이므로 `_ => trimmed` catch-all 이 그대로 통과시킨다.

**근거 수치**:
- 새 함수로 `normalize_hex_color("#fc0")` → `"ffcc00ff"`(불투명 정확 판정), `normalize_hex_color("#fc0f")` → `"ffcc00ff"`(구코드는 `"fc0f"`로 오탐 FAIL 유발하던 케이스 — 이제 정확히 opaque), `normalize_hex_color("#fc08")` → `"ffcc0088"`(알파 `88` 보존, translucent 로 정확히 남음)임을 단위 테스트로 직접 검증했다.
- 부수 효과로 `list.activeBackground` 동등성 린트(`번들_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다`)도 함께 정합화된다 — 3/4자리 hex 로 표기된 두 색이 실제로는 같은 색인데 예전엔 정규화 형태가 갈려 동등 비교가 항상 거짓이 되던 잠재 오탐이 함께 해소된다(발견이 지적한 "이중 방어선 판정 불일치" 우려도 이 보강으로 해소).

**테스트**: `mod tests` 관행(`함수명은_설명() { ... }`, 예: `list_themes는_내장_2종과_사용자_테마를_반환하고_파손파일은_제외한다`)을 그대로 따라 `normalize_hex_color는_3자리_축약_hex_를_불투명_8자리로_확장한다`·`normalize_hex_color는_4자리_축약_hex_의_알파_니블을_보존해_확장한다` 2건을 `normalize_hex_color` 정의 직후에 추가했다.

**검증**: `cargo fmt --all --check`(통과, diff 없음) → `cargo test -p taide domain::theme::service`(30 passed / 0 failed — 기존 28건 전량 유지 + 신규 2건, `번들_테마는_list_활성_배경이_패널_배경_및_hover_배경과_구분된다`·`번들_테마는_panel_매치_하이라이트가_불투명하다` 회귀 없음 확인) → `cargo clippy -p taide --lib -- -D warnings`(경고 0건). `git status`로 `src/` 및 `src-tauri/resources/`가 이번 작업으로 무수정임을 확인(표시된 변경분은 전부 병렬 진행 중이던 다른 임무의 기존 diff).

**범위 준수**: 지시된 발견(L2-6) 외 수정 없음. TS 파일·번들 JSON·`@ts-ignore`/`eslint-disable`/`#[allow]`·git add/commit/push·앱 실행·신규 패키지 전부 사용하지 않았다.

### 4-F3. 구조·컨벤션 (L1-3/L4-1·L1-2/L4-2·L4-7·L4-9·L4-8·L3-3·L3-0/L3-1/L4-3)

### §4. 검토 반영 — F3(구조·컨벤션 수정, minor 8건)

#### 1. [L1-3/L4-1] git-change-group.tsx 중첩 삼항 평탄화

`config` 를 만들던 2단 중첩 삼항(구현 당시 47~122행, 76줄)을 제거했다. variant 별로 `buildMergeGroupConfig`/`buildStagedGroupConfig`/`buildUnstagedGroupConfig` 3개 arrow 함수(각각 `Extract<GitChangeGroupProps, {variant:'...'}>` 를 인자로 받아 판별 유니언 narrowing 을 함수 경계로 그대로 전달)로 분리하고, 상위에 `buildGitChangeGroupConfig` 를 두어 `if (props.variant === 'merge') return ...` / `if (props.variant === 'staged') return ...` / `return buildUnstagedGroupConfig(...)` 형태의 early-return 분기로 골랐다 — common.md §3.4 "삼항 중첩(2단 이상) 금지 — early return 또는 lookup" 을 그대로 따른 형태이며, `props.variant ===` 비교는 `if` 문 2회로만 남고 `? ... : ... ? ... :` 문법은 완전히 제거됐다(grep 확인). 지시대로 단일 렌더 블록(`<div><ResourceGroupHeader/>{config.rows.map(...)}</div>`)은 문자 그대로 보존해 렌더 결과·요소·클래스·key 는 변경 전과 항등이다.

#### 2. [L1-2/L4-2] explorer-path.ts PATH_SEPARATOR export 제거

`export const PATH_SEPARATOR` → `const PATH_SEPARATOR` 로 환원. 실측(`grep -rn "PATH_SEPARATOR" src`) 결과 외부 소비처 0(3개 소비 파일 모두 `joinPath`/`parentDirOf` 만 import)을 재확인하고 파일 내부 전용으로 되돌렸다. `parentDirOf`/`joinPath` 는 3파일 실공유이므로 export 유지.

#### 3. [L4-7] explorer 훅 2개 import 순서 재배열

`use-explorer-entry-crud.ts`·`use-explorer-clipboard.ts` 모두 alias type import(`@features/explorer/file-tree-row`)가 react 보다 앞에 있던 것을 common.md §7 순서(react/프레임워크 → 외부 라이브러리 → `import type` 블록 → alias value)로 재배열했다. type 블록은 외부(`react-i18next`) → shared → entities → features 순으로, alias value 블록은 shared → widgets(자기 슬라이스) 순으로 묶어 선례(`use-editor-blame.ts`/`use-editor-lsp-integration.ts`/`use-editor-git-gutter-and-conflicts.ts`)와 동일 패턴(react/외부값 → 전체 type 블록 → alias value 블록)을 따랐다.

#### 4. [L4-9] NormalizedGitChangeRow 타입 유도

`{ path: string; origPath: string | null; absPath: string; kind: GitStatusChangeKind }` 손타입을 `Pick<ComponentProps<typeof StatusRowItem>, 'path' | 'origPath' | 'kind'> & Pick<StatusRow, 'absPath'>` 로 교체했다(finding 의 suggestedFix 그대로). `ComponentProps` 는 `react` 에서 함께 import. `StatusRowItemProps` 는 비export 타입이지만 `ComponentProps<typeof StatusRowItem>` 로 문제없이 유도된다(typecheck 통과로 확인).

#### 5. [L4-8] 팔레트 files 그룹 레이어 통일

- `src/widgets/command-palette/command-palette-file-match.ts`(+`.test.ts`)를 `git mv` 로 `src/shared/lib/command-palette-file-match.ts`(+`.test.ts`) 로 승격했다 — fsd.md §2.1 결정 트리 2번(도메인 무관 범용 util, 유일 의존이 `@shared/lib/relative-path`)에 해당하고, `shared/lib/command-palette-query.ts` 와 동일 명명 패턴을 따른다.
- `command-palette-files-group.tsx`(신규·미추적 파일이라 `mv` 사용)를 `src/features/command-palette/` 로 이동해 모드 그룹 5개(commands·files·symbol·line·workspaceSymbol)를 features 레이어 하나로 통일했다.
- 소비처 갱신: `command-palette-files-group.tsx` 자신의 `splitFileMatchForDisplay` import 를 `@shared/lib/command-palette-file-match` 로, `command-palette-file-match.test.ts` 의 대상 import 도 동일 경로로 갱신. `command-palette.tsx` 의 `CommandPaletteFilesGroup` import 를 `@widgets/...` → `@features/...` 로 바꾸고 features 블록 내 알파벳 순서(commands→files→line→symbol→workspaceSymbol)에 맞춰 재배치했다.
- 전수 확인: `grep -rn "widgets/command-palette/command-palette-file(-match|s-group)" src/` = 0건(이동 후 잔존 참조 없음). eslint(FSD `no-restricted-imports`) 무경고로 shared→(무의존)·features→shared·widgets→features 방향 모두 적합함을 재확인했다.
- **범위 판단**: `docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md`·`2026-08-20-palette-ux-contract.md` 의 옛 경로 언급은 계약 파일이라 지시대로 수정하지 않았다(시점 기록으로 유지 — d-30/d-31 선례와 동일 취급).

#### 6. [L3-3] mock export 누락 2건 + 검증 중 발견한 3번째 누락 1건 동반 수정

- `use-lsp-session.test.ts` 의 `@entities/lsp/lsp.ipc` mock 에 `reportLspReinitializeFailure: () => Promise.resolve()` 1줄 추가. 단독 실행 결과 **5 pass / 0 fail**(이전 `SyntaxError: Export named 'reportLspReinitializeFailure' not found` 소거 확인).
- `use-editor-lsp-integration.test.ts` 의 `@entities/project/project.ipc` mock 에 `listRecentProjects: () => Promise.resolve([])` 1줄 추가(지시분)한 뒤 단독 실행했더니 **다른 위치**에서 재실패했다: 같은 파일의 `@entities/lsp/lsp.ipc` mock(18~29행)에도 `reportLspReinitializeFailure` 가 누락돼 있어 `SyntaxError: Export named 'reportLspReinitializeFailure' not found in module '.../lsp.ipc.ts'` 로 막혔다. 원인은 `use-editor-lsp-integration.ts` → `use-lsp-session.ts`/`entities/lsp/lsp-session-registry.ts` 로 이어지는 동적 import 그래프가 `reportLspReinitializeFailure` 를 실제로 import(`lsp-session-registry.ts:28,789`)하기 때문 — `listRecentProjects` 하나만으로는 이 파일의 "단독 실행 시 pass" 완료 기준을 충족하지 못함을 실측으로 확인했다. **지시 범위를 벗어나지만** 같은 항목(L3-3)의 명시된 완료 기준("단독 실행해 pass 확인")을 충족하기 위해 불가피한 동반 수정으로 판단해, 동일한 mock 객체에 `reportLspReinitializeFailure: () => Promise.resolve()` 1줄을 추가로 넣었다. 이후 단독 실행 **4 pass / 0 fail** 확인.
- 두 파일 모두 재실행 결과를 위 수치로 검증했고, 전체 스위트(`bun test`)도 1443 pass/0 fail 로 회귀 없음을 재확인했다(수정 전과 파일 수 144 동일 — 이동만 있었을 뿐 파일 총량 불변).

#### 7. [L3-0/L3-1/L4-3] JSDoc 낡은 포인터 정정

**지시된 4곳(전부 실측 확인 후 정정):**
- `protocol.ts:208`, `semantic-tokens.ts:155` — `` `lsp-session-registry.ts`'s `buildInitializeParams` `` → `` `initialize-params.ts`'s `buildInitializeParams` ``.
- `initialize-params.ts:73,78`(현재 파일 기준 동일 라인) — "on `client` below"/"`createSession` below" → `` `createSession` (`@entities/lsp/lsp-session-registry.ts`) `` 참조로, registry.ts:334 의 기존 정정과 동일 형식(`심볼 (\`@경로\`)`)으로 맞췄다. 겸사겸사 문단 줄바꿈을 재정리했다(내용은 참조 경로 추가 외 무변경).
- `use-document-symbol-loader.ts:28` — "Workspace Symbol effect below" → "Workspace Symbol effect in `use-workspace-symbol-search.ts`".

**확장 기준(같은 기준 적용, 실측으로 "사실과 어긋남"을 확인한 것만 정정 — 3곳):**
- `document-symbol-session-waiters.ts:69` — "이 함수의 세 소비처" 목록 `breadcrumbs-bar.tsx`/`outline-panel-container.tsx`/`command-palette.tsx` 중 마지막 항목을 `use-document-symbol-loader.ts` 로 교체. `grep -rn "loadDocumentSymbolsForPath" src/widgets` 로 실제 3개 호출부가 `breadcrumbs-bar.tsx:171`·`outline-panel-container.tsx:45`·`use-document-symbol-loader.ts:48` 임을 확인 — `command-palette.tsx` 는 더 이상 이 함수를 직접 호출하지 않는다(사실과 어긋남 확정).
- `entities/lsp/lsp-session-registry.ts:482` — `listSessionRecordsForProject` 의 "used by (`command-palette.tsx`)" 를 `use-workspace-symbol-search.ts` 로 교체. `grep -rn "listSessionRecordsForProject" src/widgets/command-palette/` 결과 실제 import·호출이 `use-workspace-symbol-search.ts:4,36` 뿐임을 확인.
- `entities/lsp/lsp-session-registry.ts:507` — `waitForLspSessionForRoot` 의 "abandoning the wait" 3-소비처 목록 중 `command-palette.tsx` → `use-document-symbol-loader.ts`(위와 동일 근거 — 이 훅이 `waitForLspSessionForRoot` 를 직접 import).

**정정하지 않은 곳(판단 근거):**
- `initialize-params.ts:103-104` — "the palette's `@` mode (`command-palette.tsx`) ... depend on that hierarchy" — 이 문장은 "이 함수를 이 파일이 직접 호출한다"는 구체적 사실 주장이 아니라 "이 capability 를 이 기능(command-palette.tsx 가 대표하는 `@` 모드 전체)이 필요로 한다"는 기능 단위 서술이다. 실소비처가 `use-document-symbol-loader.ts` 로 바뀐 뒤에도 `command-palette.tsx` 는 여전히 그 모드의 상태·렌더링을 소유하므로 이 서술 자체는 거짓이 되지 않는다 — 지시문의 "파일명만 언급해 여전히 참인 다수 참조는 건드리지 말 것" 조건에 해당한다고 판단해 원문 유지했다(§3-C 의 동일 판단 선례와 일관).
- `docs/` 하위 4곳(`architecture.md`·`ipc-contract.md`·`features/lsp.md` 등, L1-0 발견)은 이번 F3 지시 범위(§7 목록)에 명시되지 않아 손대지 않았다 — 워킹트리에서 동시 진행 중인 다른 검토 반영 담당(문서 축)의 몫으로 판단했다.

#### 8. 검증 특칙 결과

- `bun test src/shared/lib/command-palette-file-match.test.ts`(신경로) → 7 pass / 0 fail.
- `bun test src/features/command-palette` 관련 → 해당 디렉토리에 `*.test.ts` 파일 자체가 없음(그룹 컴포넌트는 `command-palette.tsx` 를 통해서만 소비되고 자체 단위테스트가 없었음 — 이동 전후 동일, 신규 실패 아님).
- `bun test src/features/git/ src/widgets/git-panel/`(1·4 검증) → 28 pass / 0 fail(3파일).
- 전체 `bun run typecheck` → exit 0.
- `bunx prettier --check`(변경/이동 16파일) → 전부 통과.
- `bunx eslint`(동일 16파일) → 경고·오류 0.
- 전체 `bun test` → 1443 pass / 0 fail / 144 files(d-31 §3-D-5 기록치와 동일 — 회귀 없음).
- `docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md` 는 지시대로 미수정.
