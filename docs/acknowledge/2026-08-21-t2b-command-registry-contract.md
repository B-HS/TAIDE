# T2-B 2호 — command-registry 관심사 4분할 계약 (2026-08-21, d-30)

> 정본: 감사 `2026-08-18-architecture-audit.md` §5 T2-B 행 + :209 실측(301줄·동거 4개 —
> 상수·팔레트 파서·레지스트리·카탈로그). goal 하 T2-B 파일별 단독 배치 순번.
> 착수 전 메인 실사: 현 302줄(감사와 등치 — 기처리 아님)·export 19·소비처 다수(팔레트·키맵
> 계열). 형제 editor-pane-command-bridge·monaco-action-commands 는 이미 분리되어 있음.

## 1. 범위·원칙

- shared/lib 내 4파일 분리(관심사별·kebab-case — 정확한 경계·명명은 실사로 확정: 상수 /
  팔레트 모드 파서 / 커맨드 레지스트리(등록·조회) / 카탈로그(빌트인 커맨드 정의)). 기존
  파일명 유지 여부(레지스트리 본체가 command-registry.ts 잔류)는 소비처 diff 최소 기준.
- **동작·API 무변경 절대**: export 시그니처 유지(소비처는 import 경로만 변경). 타입은
  원본 유도 유지. barrel 금지 — 소비처가 분리 파일을 직접 import.
- 테스트(command-registry.test.ts)는 대상 분리에 맞춰 동행 분할 또는 유지(실사 판단).
- 범위 외: 커맨드 추가/제거·팔레트 동작 변경·T2-B 다른 파일.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, TS 전용) → 메인 2차 → Phase E(소형이므로 2렌즈:
  동등성 — export·소비처 전수 / 계약 — 컨벤션·경계) + major 적대적 → 수정 → 커밋 → 병합.
- 검증: bun run verify + bunx vite build exit 0 + bindings 무변경.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

### 3.1 파일 구성 (실사 확정 경계)

원본 302줄 4동거(상수·팔레트 모드 파서·레지스트리·카탈로그)를 `src/shared/lib` 내 4파일로
분리했다. 경계는 "누가 무엇을 실제로 참조하는가"를 소비처 전수 grep 으로 확인해 정했다 —
export 별 사용처가 상수(`KEYMAP_CATEGORY`, 소비처 10곳)와 팔레트 전용 타입(`PaletteMode` 등,
소비처는 `command-palette.tsx` 단일)으로 뚜렷이 갈렸다.

| 파일 | 관심사 | 내용 | 줄수 |
|------|--------|------|-----:|
| `keymap-category.ts` | 상수 | `KEYMAP_CATEGORY` (i18n 카테고리 키 상수 객체) 단독 | 21 |
| `command-palette-query.ts` | 팔레트 모드 파서 | `PaletteMode`·프리픽스 4종·`ParsedPaletteQuery`·`parsePaletteQuery`·`buildCommandModeQuery`·`PaletteLineTarget`·`parseLineModeTarget`·`FlatPaletteSymbol`·`flattenDocumentSymbols` | 57 |
| `command-registry.ts` (원파일명 잔류) | 커맨드 레지스트리(등록·조회) | `CommandContext`·`AppCommand` 타입 + `commandRegistry` Map·CRUD 5종·`isCommandRunnable`·`formatCategorizedLabel` | 51 |
| `command-catalog.ts` | 카탈로그(빌트인 커맨드 정의) | `DEFAULT_COMMANDS` + 내부 헬퍼(`notImplementedRun`·`alwaysDisabled`) | 174 |

**레지스트리 본체가 `command-registry.ts` 파일명에 잔류한 이유(계약 §1 diff 최소 기준)**:
`AppCommand`/`CommandContext` 타입을 레지스트리 파일에 둔 것은 —
1. 레지스트리(Map 저장·조회)와 `isCommandRunnable`이 이 두 타입을 직접 다루는 유일한 값
   로직이라 응집도가 가장 높고,
2. 소비처 대다수(엔티티 10개 `*.commands.ts`, 위젯 2곳)가 이미 `import type { AppCommand,
   CommandContext } from '@shared/lib/command-registry'` 를 **KEYMAP_CATEGORY 값 import 와
   별도 줄**로 쓰고 있어(타입 전용 import 를 별도 줄에 두는 컨벤션 때문에 기존에도 2줄),
   레지스트리 파일명·타입 export 를 유지하면 그 줄은 **한 글자도 바뀌지 않는다** — 실제로
   `keybinding-catalog.ts`·`monaco-action-commands.ts`·`keybinding-row.tsx`·
   `keybindings-editor.tsx`·`task.commands.test.ts`·`keybinding-catalog.test.ts` 6개 소비처는
   타입/`formatCategorizedLabel`/`listRegisteredCommands`/`isCommandRunnable` 만 참조해
   **무변경**으로 끝났다.
3. `command-catalog.ts`(카탈로그)는 `AppCommand` 타입을 레지스트리에서 import 하는 방향이
   자연스럽다 — 카탈로그는 레지스트리가 정의한 커맨드 "형태" 위에 값을 채우는 상위 개념이기
   때문(그 반대로 레지스트리가 카탈로그를 참조하면 순환이 된다).

### 3.2 소비처 갱신

전수 grep(변경 전 17개 파일)으로 확인 후 **13개 파일**을 갱신했다(4개는 이동한 export 를
참조하지 않아 무변경 확인만 하고 건드리지 않음 — `keybinding-row.tsx`·
`keybindings-editor.tsx`·`monaco-action-commands.ts`·`keybinding-catalog.test.ts`·
`task.commands.test.ts`, 총 5개).

- `KEYMAP_CATEGORY` import 경로만 교체(제자리 path swap, 순서 유지): `monaco-actions.ts`,
  `keybinding-catalog.ts`, `terminal.commands.ts`, `agent.commands.ts`, `sync.commands.ts`,
  `ai.commands.ts`, `task.commands.ts`, `git.commands.ts`, `git.commands.test.ts`(9개 — 마지막
  1개는 `KEYMAP_CATEGORY, isCommandRunnable` 합성 줄을 분리하며 교체).
- `DEFAULT_COMMANDS`/`registerCommands` 분리: `bootstrap-commands.ts`(1개, 두 줄로 분리).
- 타입 3종(`FlatPaletteSymbol`·`PaletteLineTarget`·`PaletteMode`) + 함수 3종
  (`flattenDocumentSymbols`·`parseLineModeTarget`·`parsePaletteQuery`)을
  `command-palette-query.ts` 로, 나머지(`AppCommand`·`CommandContext`·`formatCategorizedLabel`·
  `getRegisteredCommand`·`isCommandRunnable`·`listRegisteredCommands`)는 `command-registry.ts`
  유지: `command-palette.tsx`(1개, 4-import 블록으로 재구성).
- 테스트 3개 동행 분할(§3.3) 자체도 소비처 갱신에 포함: `command-registry.test.ts`(유지),
  `command-palette-query.test.ts`(신규), `command-catalog.test.ts`(신규).

**export 시그니처·타입 유도는 전부 무변경** — 함수 본문·타입 정의를 원본에서 그대로 옮겼을
뿐, 어떤 export 도 파라미터/반환 타입/이름이 바뀌지 않았다. barrel(index.ts) 은 만들지
않았다 — 소비처는 4개 파일을 각각 직접 import 한다.

### 3.3 테스트 분할 판단 근거

원본 `command-registry.test.ts`(286줄, `describe` 8블록)를 소스 분리 경계에 맞춰 **3개로
분할**했다 — 이 레포는 `*.ts`/`*.test.ts` 1:1 대응이 사실상 전수 관행(문서화된 룰은 아니지만
`shared/lib` 137개 파일 중 테스트 대상 전부가 동명 `.test.ts` 를 가짐)이라, 소스가 나뉘었는데
테스트 파일 하나가 3개 모듈을 계속 import 하면 그 관행과 어긋나고 향후 유지보수 시 "이
테스트가 어느 소스에 대응하는가"가 파일명만으로 드러나지 않게 된다.

- `command-registry.test.ts`: `registerCommand/getRegisteredCommand`,
  `registerCommands/unregisterCommand/clearCommandRegistry`, `isCommandRunnable` 3블록 +
  `dummyContext`/`buildCommand` 헬퍼 + `afterEach(clearCommandRegistry)` 유지.
- `command-palette-query.test.ts`(신규): `parsePaletteQuery`, `buildCommandModeQuery`,
  `parseLineModeTarget`, `flattenDocumentSymbols` 4블록 — 레지스트리 상태에 의존하지 않는
  순수 함수 테스트라 헬퍼·`afterEach` 불필요.
- `command-catalog.test.ts`(신규): `DEFAULT_COMMANDS` 1블록 — `isCommandRunnable`(레지스트리)
  을 값 검증에만 사용하고 등록 자체는 하지 않아 `afterEach` 불필요, `dummyContext` 만 재선언.

세 파일 모두 테스트 케이스 본문(assertion)은 원본에서 **한 글자도 바꾸지 않고** 그대로
옮겼다 — 이동만 있었고 커버리지 변화는 없다.

### 3.4 검증

`bun run verify` exit 0(TS: tsc/eslint/prettier/bun test 1435 pass 0 fail·143 files, Rust:
fmt/clippy/cargo test 전부 ok) + `bunx vite build` exit 0(청크 경고는 기존 대형 vendor 청크
관련으로 이번 변경과 무관) + `src-tauri`/`src/shared/generated` 등 bindings 산출물 무변경
(git status 에 미노출).
