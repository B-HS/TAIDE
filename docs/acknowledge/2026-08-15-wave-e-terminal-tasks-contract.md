# Wave E 구현 계약 — 터미널·태스크 (2026-08-15)

> 정찰 wf_a5ff7069-171(opus+high 3축: 터미널/pty·태스크·OSC133, 축2 OSC133 웹 리서치는 2회 실패하나
> 축0 xterm 6.0 API 실측 + 기존 `docs/research/xterm-pty.md §8`(셸 rc 주입 설계 완비)로 재료 확보).
> 캠페인 계약: 2026-08-14-remaining-features-pro-qa-plan.md(완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-15, 전부 추천안)

| # | 결정 | 선택 |
|---|------|------|
| ① OSC133 | 주입·범위 | **자동 주입+opt-out** — VS Code 방식(zsh=ZDOTDIR·bash=--init-file·fish, macOS 우선) + TAIDE_SHELL_INTEGRATION opt-out. 순수 OSC133 A/B/C/D(관대 파싱). 명령 데코(gutter)·⌘↑/↓ 명령 간 이동·종료코드 배지·overviewRuler 스크롤바. VS Code OSC633 확장은 기각. PowerShell 후속 |
| ② 태스크 러너 | 감지·실행 | **Rust 감지 + pty 실행** — package.json scripts(serde_json)·Makefile(정규식)·Cargo(고정 명령 세트). 실행=기존 pty+layout_open_tab 재사용. problem matcher 는 P1 제외 |
| ③ Run Selected Text | 동작 | **추천** — 포커스 터미널 탭 타겟(없으면 신규)·자동 개행(즉시 실행)·선택 없으면 현재 줄(VS Code)·팔레트+에디터 컨텍스트 메뉴 |

**의존성 판단**: `toml` 크레이트 신규 추가 **안 함**(최소 의존성) — Cargo 태스크는 고정 명령 세트로 감지, Cargo.toml alias/[[bin]] 정밀 파싱은 backlog.

## 2. 확정 사실 (정찰·메인 확인)

- 터미널: portable-pty 직접 구현(infra/pty.rs) + xterm 6.0.0. spawn 스레드 3개(reader·flusher·wait, OUTPUT_BATCH_MS=4·READ_BUFFER 64KB)·flow control(PauseGate)·ring buffer attach 복원·5애드온(fit/search/unicode11/web-links/webgl, allowProposedApi:true). 출력=RAW 채널(pty_spawn·pty_attach), 입력=pty_write(String).
- OSC 파싱 0(Rust·프론트). TerminalCwdChanged 이벤트는 선언만·emit/소비 0(스캐폴드). build_command 는 env 만 설정(rc 주입 없음).
- xterm 6.0 실측 API: parser.registerOscHandler(ident, cb)·registerMarker(cursorYOffset)·registerDecoration(options)·IDecorationOptions.overviewRulerOptions·IMarker(scrollback 밀리면 자동 dispose). `xterm-pty.md §8` 에 OSC133 시퀀스(A/B/C/D;exit)·셸별 주입 스니펫·ZDOTDIR/--init-file 주입법·블록 모델 코드 완비.
- Run Selected Text primitive 전부 존재: focusedEditor 선택(editor-area.tsx:89-94)·터미널 탭 find/open(131-150)·pty_write. command-registry·bridge 패턴.
- 태스크 primitive: file_open 으로 루트 파일 읽기, layout_open_tab 으로 터미널 탭, pty_write 로 명령 전송. 신규 커맨드 배선 3곳(lib.rs·dispatch.rs allowlist+match·bindings 자동).
- 키맵: 터미널 키맵 toggle-terminal(ctrl+`)·new-terminal(ctrl+shift+`)뿐. 명령 간 이동 액션 없음(KeymapActionId+APP_KEYMAP+useGlobalKeymap 3곳 배선).

## 3. 확정 설계

### 3.1 OSC 133 Shell Integration (Rust rc 주입 + 프론트 xterm)

- **Rust(shell-integration)**: infra/pty.rs 또는 신규 shell-integration 모듈에서 spawn 시 셸별 rc 주입 —
  zsh=ZDOTDIR 임시 디렉터리(.zshrc 에 통합 스크립트 source + 원래 ZDOTDIR 복원·사용자 rc 재source, 초기화 후 정리)·
  bash=--init-file(스크립트 끝 ~/.bashrc source)·fish=이벤트 함수. 스니펫은 xterm-pty.md §8 그대로.
  TAIDE_SHELL_INTEGRATION=1 환경변수(중복 로드·opt-out 감지). 미지원 셸·opt-out 시 주입 스킵(현행 동작).
  powerlevel10k instant prompt 충돌 주의(risk). 통합 스크립트는 resources 또는 인라인.
- **프론트(터미널 위젯)**: term.parser.registerOscHandler(133) → 블록 모델(A=startMarker·C=outputStart·
  D=endMarker+exitCode). registerDecoration(gutter 데코 + overviewRulerOptions 스크롤바, 종료코드 색상
  테마 토큰 연동). ⌘↑/⌘↓ = blocks 배열로 이전/다음 명령 marker.line 으로 scrollToLine(키맵 신규 2액션
  3곳 배선). **dead marker 정리**(scrollback 밀림 dispose 감지). 알 수 없는 OSC133 파라미터 관대 무시.
- cwd(OSC 7)는 이번 제외 — TerminalCwdChanged 활성화는 backlog(OSC133 과 별개 시퀀스).

### 3.2 태스크 러너 (Rust 감지 + pty 실행)

- **Rust(신규 도메인 또는 project/terminal 확장)**: `detect_tasks(projectId)` 커맨드 — 프로젝트 루트에서
  package.json(serde_json 으로 scripts 키 파싱 → `npm run <name>`, 패키지 매니저는 lockfile 감지 bun/pnpm/
  yarn/npm)·Makefile(정규식 `^([a-zA-Z0-9_][^:=]*):` 타겟, .PHONY 참고)·Cargo.toml 존재 시 고정 명령
  (cargo build/test/run/check/clippy). Task 타입(label·command·source(npm/make/cargo)·cwd). 배선 3곳+파리티.
- **실행**: 신규 커맨드 없이 프론트가 detect_tasks 결과의 command 를 layout_open_tab(terminal) + pty_write
  로 실행(기존 재사용). 또는 전용 실행 헬퍼. 터미널 세션 재사용 정책(전용 태스크 세션 vs 활성).
- **UI**: 팔레트 Tasks 진입(신규 커맨드 "Run Task" → detect_tasks → quick-pick, 또는 팔레트 모드). 전용 패널은
  이번 제외(경량). Cargo.toml alias·[[bin]]·workspace members 정밀 파싱은 backlog(toml 크레이트 회피).

### 3.3 Run Selected Text (프론트 — 신규 Rust 0)

- command-registry 신규 커맨드(terminal.runSelectedText) — focusedEditor 선택 텍스트(없으면 현재 줄
  getLineContent) → 포커스된 터미널 탭 확보(없으면 layout_open_tab) → 터미널 write 브리지(신규 bridge 또는
  활성 sessionId 조회)로 writePty(텍스트 + 개행). 팔레트 + 에디터 컨텍스트 메뉴 진입. 키맵 선택(선택).

### 3.4 실행 구조

- **Phase A 백엔드(sonnet+xhigh, Rust 단독)**: OSC133 셸 rc 주입(pty spawn 경로·통합 스크립트·
  TAIDE_SHELL_INTEGRATION) + detect_tasks 커맨드(package.json/Makefile/Cargo 감지·Task 타입) + locale +
  배선 3곳 + 파리티 + bindings. cargo fmt/clippy/test 그린. exports 로 Task 타입·detect_tasks·주입 동작 전달.
- **Phase B 프론트 병렬 2(sonnet+xhigh, 파일 소유 분리)**: B1=OSC133 xterm 블록 모델·데코·⌘↑↓ 이동·키맵
  (터미널 위젯·keymap) / B2=태스크 러너 UI(팔레트 Run Task·quick-pick·실행) + Run Selected Text(command-
  registry·에디터 컨텍스트·터미널 write 브리지). keymap 은 B1 소유(명령 간 이동), B2 는 Run Selected 키맵만.
- **Phase C 검토**: 4렌즈(계약·정확성·보안(rc 주입 안전성·사용자 rc 미파괴·태스크 명령 injection·터미널
  write 원격)·설계, opus+xhigh) → 적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차 → 커밋.

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| VS Code OSC633 확장 | 기각 — 순수 OSC133(비VS Code 전용) |
| OSC133 수동 주입만 | 기각 — 자동 주입+opt-out 채택 |
| toml 크레이트 도입(Cargo 정밀 파싱) | 보류 — 고정 명령 세트로 P0 충족, alias/bin 정밀은 backlog |
| problem matcher(출력→Problems) | 보류(P1) — 감지·실행만 |
| cwd(OSC7) TerminalCwdChanged 활성화 | 보류 — OSC133 과 별개, backlog |
| 전용 태스크 패널 | 보류 — 팔레트 quick-pick(경량) |
| PowerShell rc 주입 | 후속 — macOS 셸 3종(zsh/bash/fish) 우선 |
| fish 4.0 미만 버전 감지 + 이벤트-함수 폴백 주입 | **보류(Phase C 결함 수정에서 명시 인지, 2026-08-15)** — fish 는 4.0+(fish-shell#10352) 부터 OSC133 을 네이티브 방출해 주입을 생략(`FishNative`)하는데, 이 처리는 4.0 미만(여전히 사용 중)엔 버전 감지·폴백이 없어 OSC133 을 아예 못 받는 gap 을 남긴다. `xterm-pty.md` §8 의 이벤트-함수 스니펫으로 메울 수 있으나 이번 구현 범위에서는 명시적으로 채택하지 않음 — 사용자 승인 시 후속 채택 |
| bash 3.2(macOS 기본) 의 output-start(`C`) 마커 미제공 | **보류(Phase C 결함 수정에서 명시 인지, 2026-08-15)** — `PS0` 가 bash 4.4+ 전용이라 macOS 기본 배포 bash(3.2.57)에서는 `C` 이벤트가 발생하지 않는다(`A`/`D` 는 정상 — 블록 경계·종료코드 배지는 영향 없음). DEBUG trap 기반 대안은 재진입 가드 검증 없이는 채택하지 않음 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드 반영).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: 셸 rc 주입이 사용자 rc 를 파괴하지 않는지(ZDOTDIR 복원·재source),
  opt-out·중복 로드 감지, dead marker 정리, 태스크 명령 조립의 injection(스크립트명 셸 이스케이프),
  터미널 write 브리지의 원격 세션 노출, Run Selected 현재 줄 폴백.
- 문서: features(터미널·태스크)·ipc-contract(detect_tasks)·xterm-pty.md(구현 반영)·qa6-checklist Wave E.
  (Wave D 문서 부채와 함께 문서화 워크플로 후보). 갭 §6 항목 종결.

## 6. Phase C 결함 수정 반영 (2026-08-15)

4렌즈+적대적 검증에서 발견된 확정/불확실 결함 전건과 minor 판정을 근본 수정했다.

- **zsh 셸 통합이 `.zshrc` 만 재-source 하고 `.zshenv`/`.zprofile` 를 유실**(major, 3건 중복 보고):
  `infra/shell_integration.rs` 가 임시 `ZDOTDIR` 에 `.zshenv`/`.zprofile` 패스스루 파일을 추가로
  심도록 수정(VS Code 실소스 확인 후 채택) — 상세는 `xterm-pty.md` §8 "구현 반영".
- **`posix_quote` 가 fish 의 홑따옴표 내 백슬래시 이스케이프를 고려하지 않아 인용 탈출 가능**
  (major→적대적 검증에서 minor 로 하향 — 태스크 실행 자체가 이미 저장소 코드 실행이라 실질 노출은
  제한적): 백슬래시를 항상 이중 이스케이프하도록 수정, 주석의 "세 셸 모두 로컬 검증" 과장 표현도
  정정.
- **Makefile `::=`(POSIX 즉시 대입) 가 `:=` 가드를 통과해 가짜 타겟으로 오검출**(minor):
  가드를 `rest.starts_with('=') || rest.starts_with(":=")` 로 확장(`target::` 이중 콜론 규칙은
  계속 정상 감지).
  `detect_make_tasks` 의 정규식도 `vsix/service.rs` 선례를 따라 `OnceLock` 캐싱으로 통일.
- **OSC133 블록 트래커의 `currentBlockIndex` 가 dead marker 정리 후 어긋나 완료 데코가 누락될 수
  있음**(minor): `pruneDisposedBlocks` 가 배열뿐 아니라 상태 전체를 받아 식별자 기준으로
  `currentBlockIndex` 를 재계산하도록 수정.
- **개행 없는 `133;A` 반복으로 블록/마커가 무한 누적**(minor): `MAX_TRACKED_COMMAND_BLOCKS`(500)
  상한을 넘으면 가장 오래된 블록을 강제 dispose 해 기존 dead-marker 정리 경로를 재사용.
- **키맵 전용 명령 간 이동 액션 2종이 키바인딩 설정 화면에서 카테고리 없이 노출**(minor):
  `KEYMAP_ONLY_CATEGORY` 에 `KEYMAP_CATEGORY.TERMINAL` 등록.
- **fish 4.0 미만 미지원**·**macOS 기본 bash 3.2 의 `PS0` 미지원**(minor 2건): 코드 주석·이 문서
  §4·`xterm-pty.md` §8 에 의도적 보류로 명시(위 §4 표). 자동 폴백은 채택하지 않음.
- **`build_command` 이 파일시스템 부작용(임시 rc 파일 쓰기)을 시그니처로 드러내지 않음**(minor):
  Rust doc comment 로 부작용을 명시(구조 변경 없음 — 최소 변경 원칙).
- **`outputStartMarker`/`endMarker` 가 등록만 되고 소비되지 않아 과설계로 보임**(minor, 기각): 두
  마커 모두 `disposeBlockMarkers` 로 정확히 정리되어 누수는 없고, OSC133 스펙 자체의 C/D 경계를
  그대로 보존한 것이라 향후 "명령 출력 선택/점프" 기능의 자연스러운 확장 지점이다. 코드 삭제 대신
  타입 필드에 JSDoc 으로 의도(현재 write-only)를 명시.
- **`terminal-write-bridge` 가 `entities/editor/editor-instance-registry` 와 다른 레이어(`shared/lib`)
  에 있음**(minor, 기각): `shared/lib` 에 이미 `-bridge` 접미사 파일 12개가 일관되게 모여 있는
  기존 관행과 일치하며, 유사물 1개(editor-instance-registry)와의 대칭을 위해 이 관행을 깨는 이동은
  기능적 이득이 없다고 판단.
- 이번 문서 갱신(features/terminal.md·features/tasks.md 신설·ipc-contract.md·xterm-pty.md §8·
  qa6-checklist Wave E)으로 §5 문서 완료조건과 §6 갭을 종결한다.
