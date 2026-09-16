# d-64 — TS 진단 폴백 정직화 · LSP 무음 실패 관측성 · 파일트리 하드 제외 해제 (2026-09-16)

> 사용자 보고 2건(같은 세션): ① gumba 프로젝트에서 `node_modules/`·`.next/` 가 있는데도 에디터가
> 모든 import 에 빨간 밑줄(설치 안 된 것처럼) — "감지를 못하는 건가, 의도적으로 막아둔 건가?"
> ② 파일 트리에 숨김 항목이 "싹 다" 떠야 하는데 `node_modules/`·`.next/` 가 없다.
> 부수 보고 ③ 간헐 토스트 `File not found: /Users/gkn/taide/help` — §0.4.

## §0 근본 원인 (실측·소스 실물)

### 0.1 ①의 진단은 monaco 내장 ts worker 가 낸 것이다 (vtsls 세션 없음)

실행 중 릴리스 v0.2.1(`/Applications/TAIDE.app`, pid 47334, 2026-09-16 16:0x KST) 실측:

1. IDE 서버 `getDiagnostics`(`file:///Users/gkn/gumba/src/shared/utils/vendor-utils.ts`) 결과의 메시지가
   전부 `Cannot find module 'X'. Did you mean to set the 'moduleResolution' option to 'nodenext', or to add
   aliases to the 'paths' option?`(TS2792) 와 `Cannot find name 'process'. … @types/node`(TS2580),
   `source: null`. TS2792 는 compilerOptions 없이 기본(classic) 해석을 쓸 때만 나오는 문구 — gumba
   `tsconfig.json` 을 읽는 vtsls/tsserver 라면 `next`·`@/…` 는 그냥 해석된다. 즉 **내장 worker 산출물**.
2. `ps`: vtsls·node 프로세스 없음. TAIDE 자식은 터미널 zsh 1개뿐.
3. 앱 로그(`~/Library/Logs/net.gumyo.taide/TAIDE.log`, 40KB 회전): `lsp`/`vtsls` 줄 0. 소스 실물도 Rust LSP
   경로의 로그 문이 `commands.rs:1074`(툴체인 취소 warn) 하나뿐 — **감지 결과·spawn 성공/실패·프로세스
   종료가 어디에도 기록되지 않는다.** stderr 는 `Stdio::null()`(`infra/lsp_proc.rs:237`). 프론트도 spawn/
   ready 거부를 `.catch(() => undefined)` 로 삼킨다(`widgets/editor-pane/use-lsp-session.ts:150`,
   `entities/lsp/lsp-session-registry.ts:457`). 감지 쿼리는 `staleTime: Infinity`(부팅당 1회 스캔).
4. vtsls 자체는 정상: `~/.bun/bin/vtsls` → `@vtsls/language-server/bin/vtsls.js`(`#!/usr/bin/env node`).
   `fix_path_env` 가 실행하는 것과 동일한 프로브(`/bin/zsh -ilc 'echo -n "_SHELL_ENV_DELIMITER_"; env; …'`)를
   launchd 유사 빈 환경(`env -i HOME USER LOGNAME SHELL=/bin/zsh PATH=/usr/bin:/bin:/usr/sbin:/sbin`)에서
   돌리면 exit 0, 파싱된 PATH 에 `~/.bun/bin`·`~/.nvm/versions/node/v22.14.0/bin` 포함. 그 PATH 로
   `vtsls --stdio` `initialize` 핸드셰이크 응답 정상. launchd PATH 만으로는 `env: node: No such file or
   directory`(shebang 이 node 를 PATH 에서 찾는다).
5. 설치본 v0.2.1 바이너리에 `fix-path-env` 문자열(`_SHELL_ENV_DELIMITER_`) 존재(d-47 포함).
6. 코드 실물: `src/shared/lib/monaco/setup.ts` 는 `import * as monaco from 'monaco-editor'`(전체 번들 —
   내장 typescript 기여 포함) + worker 배선 + 언어 id 등록뿐. `monaco.languages.typescript.*Defaults`
   호출은 저장소 전체에 **0건**(`setDiagnosticsOptions`/`setCompilerOptions`/`setEagerModelSync`/
   `setModeConfiguration` 전부 부재). 따라서 내장 worker 는 미설정 기본값(tsconfig·paths·node_modules 없음,
   lib 만)으로 `typescript`/`javascript` 모델에 semantic 진단을 낸다. `.tsx` 는 TAIDE 가 `typescriptreact`
   로 따로 등록해 내장 worker 가 붙지 않는다(노이즈 없음) — `.ts`/`.js` 에서만 보이는 이유.
7. 문서 불일치: `docs/features/editor.md` §12(내장 baseline — `setEagerModelSync(true)`, Rust 가 tsconfig 를
   읽어 `setCompilerOptions` 주입)와 `docs/features/lsp.md` §4("LSP 세션이 뜨면 `setModeConfiguration` 으로
   내장 기능 off") 는 **미구현을 구현된 것처럼** 기술.

결론: "의도적 차단" 이 아니다. (a) 이 파일에 TS 언어 서버(vtsls) 세션이 붙어 있지 않고(감지 불가 · spawn
실패 · 크래시 중 어느 것인지는 **로그가 없어 미확정**), (b) 그 폴백인 내장 worker 는 구조적으로 node_modules ·
tsconfig 를 볼 수 없어 모든 import 를 오류로 표시한다. 화면의 밑줄은 (b) 의 산출물.

### 0.2 ② 트리 — 하드 제외 목록이 디렉토리를 통째로 숨긴다

`constants::IGNORED_DIR_NAMES`(`.git`·`node_modules`·`target`·`dist`·`build`·`.next`·`.turbo`·`.venv`·
`venv`·`__pycache__`·`.cache`·`.idea`·`.DS_Store`)를 `domain/tree/service.rs:75` `read_children` 가
디렉토리 항목에 적용해 목록에서 제거한다(파일이면 통과 — 그래서 `.DS_Store` 파일은 보인다). 2026-09-15 G5 는
미회신 → 추천안 "현행 유지" 적용, "사용자가 번복 가능" 으로 기록(`2026-09-15-usability-batch5-user-decisions.md`
§2.1·§3 #4). 사용자 지시(2026-09-16) "file tree 에 숨김파일도 싹다 떠야… node_modules/ .next/ 다 없어" 로
**번복 확정**. 같은 목록을 쓰는 워처 가지치기(`infra/watcher.rs`)·검색/퀵오픈 walk(`domain/search/service.rs`)는
이번 지시 범위 밖(성능 근거 유지).

### 0.3 IDE `getDiagnostics` 결과가 항목마다 2번씩 나온다 (info — 범위 외)

`app/providers/ide-sync-provider.tsx` 의 push 는 `getModelMarkers({})`(앱 전역) 전체를 **활성 프로젝트 키**로
보내고, Rust `IdeStore::diagnostics(uri)` 는 프로젝트 전체를 합친다 → 활성 프로젝트가 바뀐 적이 있으면 같은
마커가 프로젝트 수만큼 중복. 후속 후보(§4).

### 0.4 ③ 토스트 `File not found: /Users/gkn/taide/help`

이 AI 세션이 CLI 를 탐색하며 `taide help` 를 실행한 것이 원인. `crates/taide-cli`(`normalize_absolute`)는
모든 인자를 cwd 기준 파일 경로로 정규화해 실행 중 앱에 열기 요청을 보내고, 앱이 `error.file.notFound` 토스트를
낸다. Claude Code IDE 통합이 아니라 셸 호출이었다. 앱 결함 아님. 재발 방지 기록:
`docs/feedback/2026-09-16-taide-cli-probe-opens-file-in-app.md`.

## §1 수정 방향 (범위 = 아래 5묶음, 그 밖은 §4)

### R1 (Rust) LSP 관측성 — 다음 실행에서 (a) 의 원인이 로그로 확정되게

- `domain/lsp/commands.rs::lsp_detect_servers`: 서버별 `log::info!("lsp detect: {id} available={} path={:?}")`
  (또는 1줄 요약), 사용한 PATH 는 `log::debug!` 1회.
- `spawn_process`: 해석 실패 `log::warn!("lsp {id}: executable not found (bin={}, root={})")`, 성공
  `log::info!("lsp {id}: spawn {resolved} {args:?} cwd={root} pid={pid}")`(pid 는 `LspProcHandle` 이 노출).
- `infra/lsp_proc.rs::spawn`: stderr 를 `Stdio::piped()` 로 받아 bounded tail(4 KiB 링, 상수화)로 보관하는
  reader task 추가, `LspProcHandle` 에 tail 접근자. `handle_process_exit` 에서
  `log::warn!("lsp {id}: exited code={code:?} stderr_tail=…")` — tail 은 `infra::redact::mask_known_secrets`
  통과(d-57 설치 tail 과 동일 정책). `stopping` 경로(정상 종료)는 `info`.
- `lib.rs`: `fix_path_env` 성공 시 `log::info!("PATH 보정 완료: {n} 항목")`(실패 warn 은 유지).
- 프론트 `use-lsp-session.ts`·`lsp-session-registry.ts`: spawn/ready 거부를 무음 삼킴 대신
  `console.warn('[lsp] session failed', serverId, root, error)` 1회(→ `error-log-forwarding.ts` 가 파일 로그로
  포워딩). 기존 테스트가 무음을 가정하면 함께 갱신.
- 단위 테스트: stderr tail 캡처(기존 `lsp_proc.rs` 의 `sh` victim 패턴 재사용) 1건, tail 마스킹 1건.

### F1 (TS) 내장 ts worker 폴백 정직화

- `src/shared/lib/monaco/setup.ts`(또는 새 `src/shared/lib/monaco/builtin-typescript.ts` 를 setup 에서 호출):
  `typescriptDefaults`·`javascriptDefaults` 모두 `setDiagnosticsOptions({ noSemanticValidation: true,
  noSyntaxValidation: false, noSuggestionDiagnostics: true })`.
- 근거: 프로젝트 컨텍스트(tsconfig·node_modules) 없는 worker 의 semantic 진단은 사실상 전부 오탐(TS2792/2307/
  2580/2304). 구문 오류는 유지. 완성·호버 등 내장 기능은 폴백에서 계속 동작.
- **가정(사용자 확인 대상)**: 폴백 = 구문 검사만. 대안(`diagnosticCodesToIgnore` 로 코드만 골라 무시)은 남는
  semantic 진단도 컨텍스트 부재로 신뢰 불가라 채택하지 않음.

### F2 (TS) 문서화된 미구현 게이팅 구현 — LSP 세션 중 내장 provider 정지

- 새 모듈 `src/shared/lib/monaco/builtin-typescript-mode.ts`: `suspendBuiltinTypeScriptMode(languageId)` →
  `release`. `typescript`/`typescriptreact` → `typescriptDefaults`, `javascript`/`javascriptreact` →
  `javascriptDefaults`, 그 외 no-op. defaults 객체별 refcount: 0→1 에서 `setModeConfiguration`(모든 기능
  `false`), 1→0 에서 기본 `modeConfiguration` 복원(초기값을 최초 정지 시 `getModeConfiguration()` 으로
  스냅샷).
- 호출 지점: `entities/lsp/lsp-session-registry.ts::ensureLanguageRegistered` 의 `disposables` 배열에
  `{ dispose: suspendBuiltinTypeScriptMode(languageId) }` 추가 → `disposeSession` 이 언어별 disposables 를
  전부 dispose 하므로 자동 복원. `serverId` 무관(언어 기준).
- 단위 테스트: refcount·복원·비TS 언어 no-op(monaco defaults 는 목).

### T1 (Rust) 파일트리 — 전부 표시

- `domain/tree/service.rs::read_children` 의 `is_ignored_dir` 분기 제거. `has_children` 계산·정렬 그대로.
- 테스트 `무시_디렉토리는_자식_목록에서_제외된다` → `무시_디렉토리도_자식_목록에_포함된다`(node_modules 가 보이고
  `has_children` true).
- 워처(`infra/watcher.rs`)·검색/퀵오픈(`domain/search`) 의 `IGNORED_DIR_NAMES` 적용은 **변경하지 않는다**.
  트리가 보여주는 `node_modules/` 내부의 변경은 워처가 감시하지 않으므로 펼친 상태에서 실시간 갱신되지 않을 수
  있다(VS Code 의 `files.watcherExclude` 기본과 같은 절충) — 문서에 명시.

### D (문서) — 실물과 일치시키기

- `docs/features/editor.md` §12: 내장 baseline 실체(미설정 worker → F1 구문 전용 + F2 세션 중 정지)로 정정.
- `docs/features/lsp.md` §4 "에디터 연동 규칙" 첫 항목을 실구현(F2 모듈·호출 지점·복원 시점)으로 정정, §1/§2 에
  R1 로그 항목(감지·spawn·종료·stderr tail 마스킹) 추가.
- `docs/features/explorer-sidebar.md` §2.3: "워처·트리·검색이 동일 규칙 공유" → 트리는 무시 목록 미적용(전부
  표시), 워처·검색만 적용 + 위 절충 명시. `docs/ipc-contract.md` 의 트리 목록 관련 서술이 있으면 동기.
- 버그 기록 `docs/bug/2026-09-16-ts-builtin-worker-fallback-module-errors.md`(메인 작성), 이 계약 §3, PROCESS.

### 범위 외 (§4 로 이월)

- PATH 탐색 경로 보강(`~/.bun/bin`·nvm 등 하드코딩 + spawn 시 node 디렉토리 PATH prepend): 사용자 실기
  로그가 "감지 불가" 를 보일 때만. 지금은 근거 없음(0.1-4).
- 상태바 "언어 서버 미감지(vtsls)" 표시·설정 LSP 섹션 링크: 제안.
- IDE `getDiagnostics` 중복(0.3).

## §2 실행·검토 계획

- wf1(구현): fixer-rust(opus·xhigh — R1 Rust + T1 + Rust 측 문서: lsp.md·explorer-sidebar.md·ipc-contract.md)
  ∥ fixer-ts(opus·xhigh — F1 + F2 + R1 프론트 warn + editor.md §12) → verify(sonnet·high: `bun run typecheck`·
  `bun run lint`·`bun run format:check`·`bun test`·`cargo test -p taide`(lsp·tree·lsp_proc 모듈 + 전체)·
  `bunx vite build`) → lens 1개(sonnet·xhigh: 근본성·회귀·경계) major/minor/info.
- 메인: 발견 판정 → wf2(수용 반영 + 재검증) → 메인 2차 verify.
- 사용자 실기(dev 또는 release 재빌드 후): ① 설정 → LSP 서버 → vtsls 행(설치됨/미설치·경로) ② 상태바
  `LSP n/m` ③ `TAIDE.log` 의 `lsp detect` / `lsp vtsls: spawn` / `exited` 줄 ④ `vendor-utils.ts` 밑줄
  ⑤ 트리에 `node_modules/`·`.next/`·`.git/` 표시. ③ 이 "available=false" 면 §4 PATH 보강 착수.

## §3 기록 (구현·검토·검증 — 실시간 동기)

### 구현 — fixer-rust

wf1 fixer-rust 완료 (2026-09-16).
- R1(Rust): `infra/lsp_proc.rs` stderr `Stdio::piped()` + 최신 4 KiB 링(`LSP_STDERR_TAIL_BYTES`) + `LspProcHandle::stderr_tail()`; `on_exit` 를 `(code, tail)` 2인자로 바꿔 tail 을 실어 보낸다(핸들이 `SessionEntry::proc` 에 설치되기 전에 죽는 프로세스 대응). 종료 콜백 전 reader join 500ms 상한, `exited` 는 그 전에 set.
- R1(로그): `lsp_detect_servers` 서버별 info + PATH debug / `spawn_process` 해석실패 warn·성공 info(pid 포함) / `handle_process_exit` stopping info·크래시 warn(code·restarts·마스킹 tail) / `lib.rs` PATH 보정 성공 info 1줄. tail 은 `masked_stderr_tail`(= `mask_known_secrets` + " / " 합치기)로 d-57 정책 준수.
- T1: `domain/tree/service.rs::read_children` 의 `is_ignored_dir` 분기 제거(+ 미사용 `use crate::constants` 정리). 테스트 `무시_디렉토리는_..._제외된다` → `무시_디렉토리도_..._포함된다`(kind=Directory, has_children=true)로 반전하고, fixture 를 유지한 채 영향받는 7개 테스트의 카운트·정렬·offset 단언 18곳을 갱신.
- D: `lsp.md` §1 관측성 로그 · §2 stderr tail 마스킹 추가, §4 첫 항목을 F2 실구현(실제 모듈 실물 확인 후)으로 정정. `explorer-sidebar.md` §2.3 을 "트리는 무시 목록 미적용·전부 표시 / 워처·검색만 적용 / 절충 2가지"로 교체. `ipc-contract.md` :1802 의 트리 서술 1줄 정정(:447 search 서술은 지시대로 유지 — 단 트리≠퀵오픈 집합 불일치가 생겨 후속 판단 필요).
- 검증: `cargo test -p taide` 전체 1661+4+3+8 통과(0 실패), `cargo fmt --all --check` exit 0, `cargo clippy --workspace --all-targets -- -D warnings` exit 0. 신규 테스트 3건(tail 캡처·상한 절단·tail 마스킹). 커맨드 시그니처 무변경 → `bindings.ts` 영향 없음.

### 구현 — fixer-ts

F1/F2/R1(프론트)/D(editor.md §12) 구현 완료 — fixer-ts. 신규 `shared/lib/monaco/builtin-typescript.ts`(부팅 1회, ts·js defaults 에 `{noSemanticValidation:true, noSyntaxValidation:false, noSuggestionDiagnostics:true}`)·`builtin-typescript-mode.ts`(language service 객체별 refcount 정지/복원) + 각 테스트(3·8건). `ensureLanguageRegistered` 의 언어별 disposables 에 release 를 넣어 `disposeSession` 이 복원한다. `acquireLspSession` 의 `ready.catch` 와 `use-lsp-session.ts:150` 의 무음 삼킴을 `console.warn('[lsp] session failed'|'[lsp] attach failed', …)` 로 교체(레지스트리 쪽은 회귀 테스트 1건 추가).

**계약 §1 전제 2건 정정(monaco-editor 0.56.0 실물 확인).** ① `monaco.languages.typescript` 는 런타임 값이 없는 타입 전용 deprecated 스텁 → 최상위 **`monaco.typescript`** 사용(`esm/vs/index.js:90-91`, `monaco.d.ts:10231-10236`). 그대로 썼으면 부팅 크래시. `getModeConfiguration()` 도 없어 `modeConfiguration` 게터 사용. ② "tsMode 가 `onDidChange` 에서 provider 재등록" 은 **사실이 아님** — `tsMode.js:30-132` 의 `setupMode` 가 `registerProviders()` 를 1회만 호출하고 구독이 없어 `setModeConfiguration` 은 이미 등록된 provider 를 되돌리지 못한다. 그래서 정지 시 `setDiagnosticsOptions` 3종 off 를 함께 적용(=`DiagnosticsAdapter` 가 `onDidChange` 구독 → 마커 비우고 재계산, `languageFeatures.js:183`·`:153`)했고, 이것이 실제로 밑줄을 없애는 경로다. 남는 provider 한계는 editor.md §12 에 '알려진 한계' 로 명시(은폐 없음).

API 는 주입식(필수 인자)으로 바꿔 두 모듈이 `monaco/setup` 을 import 하지 않게 했다 — 신규 테스트가 전역 `mock.module` 없이 돈다. 범위 밖 1줄: `use-editor-lsp-integration.test.ts` 의 `project.ipc` 페이크에 누락된 `setProjectDisplay` 추가(계약 검증 명령이 이것 때문에 기존부터 9건 실패 — test-gap-map 의 `widgets/**` 부분 실행 깨짐).

검증: `bun run typecheck` exit 0 / eslint·prettier(변경 파일) 0 / `bun test` **2745 pass 0 fail** / 지정 3개 디렉토리 208 pass 0 fail / `bun test src/widgets` 376 pass 0 fail / `bunx vite build` exit 0. 실기(dev 재빌드 후 밑줄·완성 중복) 확인은 미완.

### 검증(wf1 verify)

| # | 명령 | 결과 |
|---|------|------|
| 1 | `git status --short` / `git diff --stat` | 변경 14개 파일(diffstat: `+466/-62`) + 신규 4개(`builtin-typescript.ts`·`.test.ts`, `builtin-typescript-mode.ts`·`.test.ts`) + docs 3개(acknowledge/bug/feedback). fixer-rust/fixer-ts 보고의 changedFiles 와 일치. |
| 2 | `bun run typecheck` | `tsc --noEmit` 출력 없음 — exit 0 |
| 2 | `bun run lint` | `eslint .` — 0 error, 11 warning(전부 기존 파일: outline-panel/problems-panel/search-results-list/commit-graph/git-panel 의 `useVirtualizer` incompatible-library, untitled-pane/use-editor-file-persistence 의 exhaustive-deps — 이번 변경 파일 무관, 사전 존재) — exit 0 |
| 2 | `bun run format:check` | `prettier --check .` — "All matched files use Prettier code style!" — exit 0 |
| 3 | `bun test`(전체) | **2745 pass / 0 fail**, 6139 expect(), 275 files, 8.61s |
| 4 | `cargo test -p taide`(전체: lib+bin+통합 3종) | lib **1661 passed**/0 failed + bin 0/0 + `capability_symmetry` 4/0 + `domain_boundaries` 3/0 + `session_restore` 8/0 — 도합 **1676 passed / 0 failed** |
| 4 | `cargo fmt --all --check` | 출력 없음 — exit 0 |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings` | "Finished `dev` profile" — 경고/에러 0 — exit 0 |
| 5 | `bunx vite build` | "✓ built in 5.07s" — exit 0 (500KB 초과 청크 경고는 rolldown 표준 안내이며 사전 존재하는 사전 번들 특성, 이번 변경과 무관) |

두 fixer 보고의 검증 수치(bun test 2745/0, cargo test 1661+4+3+8/0, fmt/clippy 0)를 통합 실행으로 재확인함 — 편차 없음. `git add -A` 등 스테이징·커밋은 수행하지 않음(권한 범위 밖).

### 렌즈 검토(wf1, sonnet·xhigh 1렌즈) → 메인 판정

- 발견 2건(major 0): **minor** `docs/ipc-contract.md:442-443` — `search_list_files` 의 "트리 = 퀵오픈 집합" 근거가 T1
  이후 깨졌는데 그 자리에 캐비어트가 없음 → **수용, 메인 직접 정정**(문서 정정은 메인 허용 범위) — "이었다 … d-64 T1
  이후 성립하지 않는다(explorer-sidebar.md §2.3 절충 2)" 캐비어트 추가. **info** `use-lsp-session.ts:150` 의
  `console.warn('[lsp] attach failed')` 는 전용 테스트 없음(`attachLspSession` 미export, 원래도 이 경로 테스트 없음) →
  **QA 부채로 기록**(§4), 코드 변경 없음.
- 적대적 검증 생략 근거: major 0(§1 표의 생략 조건 — 발견이 문서 1줄·테스트 부재라 실행 재현 불요, 메인 직접 확인).
- fixer 가 계약 §1 전제를 정정한 2건(`monaco.typescript` 네임스페이스 / `setModeConfiguration` 소급 미적용 → 정지 시
  `setDiagnosticsOptions` 동반) 은 메인이 신규 모듈·diff 실물을 통독해 **수용**. fixer-ts 의 범위 밖 1줄(`use-editor-lsp-
  integration.test.ts` 페이크에 `setProjectDisplay` 추가 — 기존부터 깨져 있던 `widgets/**` 부분 실행 14건 해소)도 수용,
  editor 커밋에 포함.
- 잔존 설계 절충(수용·문서화됨): ① refcount 가 language service 전역이라 A 프로젝트 세션이 B 프로젝트의 세션 없는 `.ts`
  구문 진단도 함께 끈다 ② 세션 중 내장 완성·호버 provider 는 등록된 채 남는다(monaco 0.56 한계) ③ 큰 `node_modules/`
  펼치면 `has_children` 용 `read_dir` 이 항목마다 1회 — 실기 체감 확인 대상.

### 메인 2차 검증 (2026-09-16, 렌즈 수용 반영 후)

| 명령 | 결과 |
|------|------|
| `bun run verify`(typecheck·lint·format:check·bun test·rust fmt·clippy·cargo test) | exit 0 — bun **2745 pass / 0 fail**(275 files), cargo lib **1661**·bin 0·통합 4+3+8·cli 17 전부 0 fail, eslint 0 error(기존 warning 11), clippy 0 |
| `bunx vite build` | built in 5.04s, exit 0 |
| `bun run typecheck:e2e` | exit 0 |

커밋 분할(5): editor(F1·F2·프론트 warn·editor.md) / lsp(R1 Rust·lsp.md) / tree(T1·explorer-sidebar·ipc-contract) / docs(계약·버그·
피드백·PROCESS) / release(v0.2.2 동기 3파일·Cargo.lock·릴리스 노트). 사용자 지시 "다 끝나면 draft 까지" → dev 푸시 · main ff ·
태그 `v0.2.2` · Release 런 · draft. 실기 확인(§2 ①~⑤)은 설치본으로 사용자 몫.

## §4 후속

- PATH 보강(§1 범위 외 1) — 실기 로그 조건부.
- 상태바 미감지 표시 — 제안.
- IDE getDiagnostics 프로젝트 병합 중복(0.3) — push 를 프로젝트 루트로 필터하거나 store 에서 dedupe.
- QA 부채: `attachLspSession` 실패 경로의 `console.warn('[lsp] attach failed')` 회귀 테스트(함수 미export 라 훅 통합
  테스트로 접근 필요). 레지스트리 쪽 warn 은 테스트 고정됨.
- 세션 중 내장 완성·호버 provider 소급 해제(monaco 업스트림 또는 `monaco-editor` 전체 import 를 부분 import 로 바꿔 내장
  TS 기여 자체를 빼는 방안) — 제품 판단 필요(폴백 완성·호버를 포기할지).
