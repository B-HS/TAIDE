# 기능 — 태스크 러너 · Run Selected Text (Wave E)

> 계약: `docs/acknowledge/2026-08-15-wave-e-terminal-tasks-contract.md`.
> IPC 정본: `docs/ipc-contract.md` §3 "task". 실행 전달 경로는 `terminal.md` §9 재사용.

## 1. 태스크 감지 (Rust, `domain/task/service.rs`)

`detect_tasks(projectId)` query 가 프로젝트 루트를 훑어 `Task[]`(`{ label, command, source, cwd }`)
를 반환한다. 부수효과 없음 — 실행은 하지 않는다.

- **npm 계열** (`source: 'npm'`): `package.json` 의 `scripts` 키를 `serde_json` 으로 파싱.
  패키지 매니저는 락파일로 판별한다 — `bun.lockb`/`bun.lock` → bun, `pnpm-lock.yaml` → pnpm,
  `yarn.lock` → yarn, 그 외 기본 npm. `command` 는 `{pm} run {스크립트명}` (스크립트명은
  `posix_quote` 로 인용 — 셸 특수문자·백슬래시 대응).
- **Makefile** (`source: 'make'`): `GNUmakefile` → `makefile` → `Makefile` 순으로 첫 파일만 채택
  (make 본연의 탐색 순서). 타겟은 정규식 `^([a-zA-Z0-9_][^:=]*):` 로 스캔하는 best-effort 방식이며
  (정식 Makefile 파서 아님), `:=`·`::=`(GNU 즉시 대입·POSIX 2012 즉시 대입) 로 시작하는 변수
  대입 행은 타겟으로 오인하지 않도록 가드한다. `target::`(GNU 독립 이중 콜론 규칙)은 정상적으로
  타겟으로 감지된다. `command` 는 `make {타겟명}` (타겟명도 `posix_quote`).
- **Cargo** (`source: 'cargo'`): `Cargo.toml` 존재 시 고정 명령 5종
  (`cargo build`/`test`/`run`/`check`/`clippy`) 를 반환한다. `[[bin]]`·workspace members·alias 등
  `Cargo.toml` 정밀 파싱은 **의도적으로 안 함** — `toml` 크레이트를 새 의존성으로 들이지 않기로
  한 계약 결정(§1) 때문이며, 정밀 파싱은 backlog.

세 소스는 순서(npm→make→cargo)대로 이어붙여 반환한다. 각 태스크의 `cwd` 는 항상 프로젝트 루트.

## 2. 실행 — 팔레트 "Run Task"

- `command-registry` 의 `task.runTask` 커맨드(카테고리: TERMINAL) → `task-runner-bridge` 로
  `TaskRunnerDialog` 를 연다. `activeProjectId` 가 없으면 커맨드 비활성.
- 다이얼로그가 열릴 때만 `detect_tasks` 쿼리를 활성화(`enabled: open && !!activeProjectId`) —
  팔레트를 열지 않는 한 매 프로젝트 활성화마다 파일시스템을 훑지 않는다.
- 목록은 `fuzzyFilter` 로 라벨 기준 필터링, 소스별 라벨(`task.sourceNpm`/`Make`/`Cargo`) 배지 표시.
- 태스크 선택 시 `editor-pane-command-bridge` 로 `{ type: 'run-in-terminal', text: task.command,
  cwd: task.cwd }` 를 보낸다 — 아래 §3 의 공통 전달 경로.
- 전용 태스크 패널·problem matcher(출력→Problems 연동)는 이번 범위 밖(경량 quick-pick, 계약 §4).

## 3. 실행 전달 — `run-in-terminal` (widgets/editor-area, 공통 경로)

Run Task 와 Run Selected Text 가 공유하는 단일 전달 로직(`editor-area.tsx::runInTerminal`):

1. 포커스된 pane 에 터미널 탭이 있으면 그 탭을 활성화하고 재사용한다.
2. 없으면 새 터미널 탭을 연다(`cwd` 전달).
3. 텍스트에 개행을 붙여 `terminal-write-bridge`(`shared/lib/terminal-write-bridge.ts`) 로 쓴다.
   `pty_write` 를 직접 부르지 않는 이유: 방금 연 탭은 `TerminalSession` 이 비동기로 크기 측정 후
   spawn 하므로, pty 준비 전에 온 쓰기 요청을 탭별 큐에 쌓아 두었다가 세션이
   `registerTerminalWriteHandler` 로 핸들러를 등록하는 순간 순서대로 흘려보낸다(레이스 방지).

## 4. Run Selected Text in Terminal (프론트 전용, `editor-area.tsx`)

- 커맨드: `terminal.runSelectedText` — 팔레트 + 에디터 컨텍스트 메뉴 진입.
- 포커스된 파일 에디터의 선택 텍스트를 가져오되, **선택이 없으면 현재 줄**(VS Code 동일 정책,
  `resolveSelectedTextOrCurrentLine`)로 폴백한다. 포커스된 에디터가 없으면 no-op.
- 얻은 텍스트를 §3 의 공통 `runInTerminal(text, cwd: null)` 로 전달한다(cwd 는 지정하지 않음 —
  기존 활성 터미널 재사용이 기본이라 새 탭을 여는 경우에만 프로젝트 기본 cwd 로 열림).

## 5. 범위 밖 (계약 §4)

| 항목 | 처리 |
|------|------|
| Cargo.toml `[[bin]]`/workspace members/alias 정밀 파싱 | backlog — `toml` 크레이트 미도입 |
| problem matcher(태스크 출력 → Problems 패널) | P1 보류 |
| 전용 태스크 패널(팔레트 아닌 상시 UI) | 보류 — quick-pick 로 충분 판단 |
