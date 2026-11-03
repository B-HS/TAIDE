# 01. 제품과 전환 목표, 현재 진척, 남은 범위

> 작성일: 2026-11-04(2026-11-03 판을 실제 파일로 다시 확인해 갱신)
> 기준 상태
> - 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`: 브랜치 `to_rust_native`, HEAD `7be1ad14`.
> - git lane은 `f3a5b5d0`으로 병합 완료. lane worktree는 낡았으므로 읽지 않습니다.
>
> 이 폴더의 공통 기반 문서는 세 개입니다. 01(이 문서)은 무엇을 왜 만드는지와 진척·남은 범위, 02는 도구·버전·명령·알려진 실패, 03은 코드 구조와 새 기능 연결 방법입니다. 영역별 남은 작업의 세부는 6절 지도의 문서가 다룹니다.

## 0. 이 문서를 읽는 법

- 독자는 이 저장소를 처음 보는 AI 모델이라고 가정합니다. 모든 사실에 근거 파일을 붙였습니다. 줄 번호는 바뀔 수 있으므로 심볼 이름·절 번호를 함께 적었습니다. 줄 번호가 안 맞으면 `rg -n "<심볼>" <파일>`로 다시 찾습니다.
- 수치는 2026-11-04에 기능표 행을 직접 세어 얻었습니다(3절). 과거 문서의 수치를 현재 상태로 옮겨 쓰지 마십시오. 과거 문서의 "통과" 기록이 실제와 달랐던 사례가 있습니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절 "이전 에이전트의 QA 문서 PASS").
- `docs/HANDOFF.md`(2026-10-09 스냅샷, 첫머리 3줄에 스스로 "낡았습니다"라고 적음)와 `docs/architecture.md` 9절(최종 갱신 2026-10-09)은 일부가 낡았습니다. 예: architecture 9.3절 표는 "창: 단일 창(보조 창 미구현)"이라고 적지만 보조 창은 구현됐습니다(기능표 `shell-4` complete, `docs/quality-assurance/2026-10-27-native-auxiliary-window.md`). architecture 9.2절은 시작을 `eframe::run_native`로 적지만 지금 `native/taide-native-app/src/main.rs` `main`은 `eframe::create_native`와 직접 만든 winit 이벤트 루프를 씁니다(03 문서 7절). 현재 상태의 정본은 `docs/PROCESS.md` 맨 위 절과 기능표 행입니다.

## 1. 제품: TAIDE

### 1.1 한 줄 정의

TAIDE는 macOS(Apple Silicon)용 개인 IDE입니다. 프로젝트·탭·분할·터미널·Git·언어 서버의 상태를 Rust 코어가 소유하고 화면은 그것을 그리기만 합니다(`README.md` 7·15줄, `docs/PRD.md` 1절 "비전"). 지금 배포되는 앱은 Tauri 2 + React/TypeScript이고, 이 작업은 화면까지 Rust(egui/eframe 0.36.2)로 옮기는 전환입니다(`docs/HANDOFF.md` 1절, 문서 안에서는 마일스톤 이름 "M8"로도 부릅니다).

### 1.2 원본 앱의 구성

| 층 | 위치 | 내용 | 근거 |
| --- | --- | --- | --- |
| 화면(원본) | `src/` | React 19.2.8 + TypeScript 5.9.3, FSD 구조(`app`, `widgets`, `features`, `entities`, `shared`) | `package.json` dependencies·devDependencies, `ls src` |
| 편집기(원본) | `node_modules/monaco-editor` | Monaco 0.56.0, 구문 강조는 Shiki 4.4.3(`@shikijs/monaco`·`@shikijs/langs`) | `package.json` |
| 터미널(원본) | `node_modules/@xterm/xterm` | xterm 6.0.0과 addon(fit 0.11.0·search 0.16.0·unicode11 0.9.0·web-links 0.12.0·webgl 0.19.0) | `package.json` |
| 셸(원본) | `src-tauri/` | Tauri 2 앱(`tauri = { version = "2", features = ["protocol-asset"] }`), 명령·이벤트 IPC | `src-tauri/Cargo.toml` 22줄 |
| 공용 코어 | `crates/taide-*` | 두 앱(Tauri 앱과 native 앱)이 함께 쓰는 Rust crate 30개(모델·런타임·Git·LSP·터미널 PTY·설정·테마·로케일 등) | 루트 `Cargo.toml` `[workspace] members`(src-tauri + crate 30개) |
| 새 화면(전환 대상) | `native/` | egui/eframe 0.36.2 native 앱과 그 crate 7개(각각 독립 workspace) | `native/taide-native-app/Cargo.toml` 56줄 `eframe = { version = "=0.36.2", ... }` |

### 1.3 원본 기능 영역과 native 대응 위치

기능표(`docs/quality-assurance/2026-10-09-native-function-matrix.md`)는 원본 기능을 8개 영역(`## shell`, `## editor`, `## terminal-agent`, `## explorer-search`, `## preview`, `## git`, `## settings`, `## commands-ui`)으로 나눕니다. 아래 TS 위치는 영역의 대략적 대응이며, 행 단위 원본 근거는 기능표의 "TS 근거" 열이 정본입니다. native 쪽 파일 이름은 `ls native/taide-native-app/src`와 `ls native/taide-native-ui/src`로 확인했습니다.

| 영역(기능표 절) | 원본 기능(README `Features` 절 기준) | 원본 TS 주요 위치 | native 주요 위치 |
| --- | --- | --- | --- |
| shell | 다중 프로젝트 rail, 슬롯·분할·탭, 보조 창, 창 크기·위치 복원, Welcome, Zen, 메뉴바, 파일 drop, 시스템 사용량, 단일 인스턴스·`taide` CLI 열기, 앱 수준 provider(알림·IPC·IDE 동기화) | `src/app/`(특히 `src/app/providers/*.tsx`), `src/widgets/{app-shell,app-sidebar,auxiliary-window-shell,window-chrome,welcome,editor-area,system-usage-modal}`, `src/features/{project,shell-slot,split,tab,welcome,window}` | 앱 `src/domains/shell.rs`, `auxiliary-windows.rs`, `window-state.rs`, `app-menu.rs`·`app-menu-bar.rs`, `welcome.rs`, `zen.rs`, `file-drop.rs`, `native-notifications.rs`, `provider-events.rs`, `single-instance.rs`, `external-open.rs`, `ide-sync.rs`; UI `shell.rs`, `project-rail.rs`, `tab-drag.rs`, `controller.rs` |
| editor | Monaco 편집(키·커서·찾기·접기·minimap·sticky scroll·괄호), LSP(완성·hover·정의·이름 변경·code action·inlay·code lens·포맷), 저장·hot exit·draft, 스니펫·Emmet, AI 완성·인라인 편집, Markdown 미리보기 분할 | `src/widgets/editor-pane`, `src/features/editor`, `src/entities/{editor,lsp,snippet,ai}`, `src/shared/lib/monaco` | `native/taide-native-editor`(egui 비의존 코어), `native/taide-native-syntax`(구문), UI `editor_surface.rs`·`editor-*.rs`, 앱 `domains/editor.rs`·`editor-*.rs`·`lsp*.rs` |
| terminal-agent | 실제 PTY 터미널, 명령 블록, 파일 링크, 검색, 에이전트(Claude Code 등) 감지·배지, IDE MCP 서버, hooks, CLI 설치 | `src/widgets/terminal-pane`, `src/features/terminal`, `src/entities/{terminal,agent,ide,task}` | `native/taide-native-terminal`, 앱 `terminal_*.rs`, `domains/terminal_agent.rs`, `agent-cli.rs`, `agent-hooks.rs`, `ide-server.rs`, `ide-tools.rs` |
| explorer-search | 파일 트리(CRUD·drag·클립보드·Git 장식·자동 reveal), 검색·치환 패널, 검색 편집기 탭, 아웃라인, 문제 패널, breadcrumb | `src/widgets/{explorer,search-panel,search-editor,outline-panel,problems-panel}`, `src/features/{explorer,search,outline,problems}` | 앱 `domains/explorer_search.rs`, `explorer*.rs`, `search-*.rs`, `symbol-*.rs`, `problems*.rs`, `breadcrumb*.rs` |
| preview | 이미지·SVG·비디오·오디오·PDF·HTML·XLS/XLSX/CSV·PPTX·HWP/HWPX·Markdown 미리보기, 다시 열기 | `src/widgets/preview-pane`, `src/features/preview` | 앱 `domains/preview.rs`, `preview*.rs` |
| git | SCM 패널(변경·stage·commit·push/pull·branch·stash·tag), diff·commit diff·Claude diff, 그래프, blame·gutter, 충돌 해결, 파일 이력 | `src/widgets/{git-panel,diff-pane,commit-file-diff,claude-diff-pane,file-history}`, `src/features/git`, `src/entities/git` | 앱 `domains/git.rs`; Git lane에서 추가된 `git-*.rs`(`git-cache.rs`, `git-scm.rs`, `git-diff.rs`, `git-claude-diff.rs`, `git-blame.rs`, `git-conflict.rs`, `git-graph.rs`, `git-history.rs` 등)와 UI `diff-view.rs`(커밋 `5195d415`) |
| settings | 설정 화면 전 section, 테마 편집기·VSIX 가져오기, 스니펫 편집기, 플러그인, Sync(gist), 원격 제어, 알림 설정, LSP·AI 설정 | `src/widgets/{settings-view,theme-editor,snippet-editor,plugin-manager}`, `src/features/{settings,theme,snippet,plugin}` | 앱 `domains/settings.rs`, `settings-*.rs`, `theme-*.rs`; UI `settings-*.rs`, `theme-*.rs`, `snippet-*.rs` |
| commands-ui | 명령 카탈로그·팔레트·키바인딩 편집기·태스크 러너, 공용 UI 위젯(dialog·menu·button·toast·scroll·error boundary 등), 전역 스타일·아이콘·문자열 국제화 | `src/widgets/{command-palette,keybindings-editor,task-runner,app-toaster}`, `src/features/command-palette`, `src/shared/ui` | 앱 `domains/commands.rs`, `command-*.rs`, `keybinding-*.rs`, `shell_keymap.rs`; UI `command-*.rs`, `keymap.rs`, `toast.rs`, `modal.rs`, `tooltips.rs`, `error-boundary.rs` |

## 2. 전환 목표와 완료 정의

### 2.1 최종 목표

1. 원본 TS 화면의 화면·상태·상호작용을 그대로 재현한 Rust-native 앱을 만듭니다(`docs/HANDOFF.md` 2절, `docs/roadmap-rust-native.md` 7절 "Phase 7" 마지막 항목: "기능 누락 0과 유사한 시각적 구성의 실기 확인 없이 기존 view를 제거하지 않습니다").
2. 전체 게이트(기능·실기·성능·보안·패키징·데이터 호환)를 통과한 뒤 native를 기본 앱으로 전환하고(Phase 8), 그 뒤에만 TS·Tauri·Monaco·xterm 코드를 제거합니다(Phase 9). roadmap 10절 "삭제 게이트"의 다섯 단계 순서는 뒤집지 않습니다.
3. 전체 완료 범위는 "TS 화면·상태·상호작용 대응, 편집기 표시·LSP·사이드바/검색·Git·셸·설정/에이전트/태스크·보조 창·구조 정리·실기/성능/보안/패키징/전환 게이트"입니다(`docs/acknowledge/2026-10-09-native-full-resume.md` 7줄).

### 2.2 행(요구사항) 하나의 완료 정의

기능표 서두(5줄)와 lane 운영 문서(`docs/memory/2026-10-28-native-lane-operations.md` 4절 "완료 기준은 그대로입니다")의 정의를 그대로 씁니다.

| 판정 | 뜻 |
| --- | --- |
| complete | 구현 + 실제 앱 경로 도달(`main.rs` → `NativeApplication` → 해당 경로) + 그 동작 묶음의 자동 검사 근거가 모두 있음. 픽셀·OS 입력·접근성·성능·출시 통과는 뜻하지 않음 |
| partial | 일부 하위 동작이나 검증이 남음. 남은 것을 기능표 "범위/잔여 동작" 열에 적음 |
| unwired | 모델·서비스·계약은 있지만 실제 앱 실행 경로에서 도달하지 못함 |
| missing | 사용자 구현이 없음 |
| not-applicable | 웹 기술·내부 지표 전용이거나 사용자 결정으로 범위 밖 |
| frozen | 사용자 동결 범위(브라우저 Wasm 클라이언트 `native/taide-remote-web`) |

"실제 앱 경로 도달"을 강조하는 이유: 2026-10-06 감사에서 이전 에이전트가 주장한 진척과 실제가 크게 달랐고, 원인이 unwired(모듈은 있으나 `main.rs`에서 닿지 않음)였습니다(`docs/memory/2026-10-07-native-transition-operations.md` 2절).

판정을 complete로 올릴 때 확인하는 질문(lane workflow 리뷰 기준, `docs/utils/2026-11-02-native-lane-workflow.js` 170줄 부근 "확인:" 문장):

1. 원본 TS 동작과 화면·상태·상호작용이 일치하는가. 오류·빈 상태·비활성·취소·수명(닫힘·종료·프로젝트 제거) 분기가 빠지지 않았는가. 원본에 없는 동작을 새로 만들지 않았는가.
2. 실제 앱 경로에 도달하는가(unwired가 아닌가).
3. 상태 수명·동시성·오래된 응답·panic 같은 정확성 결함이 없는가.
4. 검사가 실제로 그 동작을 단언하는가.

### 2.3 하지 않는 것

| 규칙 | 근거 |
| --- | --- |
| 새 기능·새 디자인을 추가하지 않습니다. TS에 없는 동작이 필요하면 사용자 결정으로 올립니다 | `docs/HANDOFF.md` 5절 1번, `docs/acknowledge/2026-10-09-native-find-regex-decisions.md` 7줄 |
| 원본 결함·내부 수치를 억지로 재현하지 않습니다. 기능을 정확하게, 버그 없이, 꼼수 없이 구현합니다 | `docs/acknowledge/2026-10-09-native-full-resume.md` 11줄, find-regex 결정 5줄 |
| 동작 기준은 TS 소스와 TS가 쓰던 Monaco·Shiki·xterm의 실제 소스입니다. 기준 자료를 구현 출력에 맞춰 만들지 않습니다 | `docs/memory/2026-10-07-native-transition-operations.md` 3절 |
| 동결된 `native/taide-remote-web`의 소스·`Cargo.toml`·`Cargo.lock`을 바꾸지 않습니다(컴파일만 유지) | `docs/acknowledge/2026-10-06-native-transition-decisions.md` 결정 표 15줄 |

원본과 의도적으로 다르게 둔 대표 사례(결함으로 판단해 재현하지 않은 것):

- 세션 중 테마 전환: TS는 테마 이름이 항상 `taide`라 두 번째 테마부터 토큰 색이 섞입니다. native는 새 테마로 다시 토큰화합니다(`2026-10-06-native-transition-decisions.md` 기각 대안 표 43줄).
- 찾기 "모두 바꾸기"의 일치 19,999개 이상 경로에서 원본이 `m` 플래그를 강제하거나 단어 단위를 무시하는 결함은 재현하지 않습니다(find-regex 결정 12줄, 4번).
- `/usr/local/bin` CLI 설치의 관리자 대화상자를 취소해도 원본은 성공 문구를 보이지만 native는 보이지 않습니다(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` "채택" 절).

### 2.4 보호 대상과 허락이 필요한 작업

자동 작업에서는 아래를 하지 않고 보류 목록에 둡니다(`docs/acknowledge/2026-10-26-native-autonomous-continuation.md` "적용" 절 12줄, `2026-10-28-native-lane-spec-decisions.md` "보류" 표, lane 운영 문서 4절).

1. 실제 사용자 데이터 경로·keychain 승계(`shell-15`, 컷오버 정책).
2. 실제 OS 알림 표시, 시스템 설정 앱 열기, `/usr/local/bin/taide` 설치·해제, Keychain 쓰기, 사용자 홈 hooks 파일·실제 프로젝트 `.claude/settings.local.json` 쓰기, 네트워크(LSP 다운로드·gist·모델 목록).
3. 실제 휴지통을 쓰는 검사와 추적 안 된 파일 discard 실검사(02 문서 12.3절에 정확한 검사 이름 3개).
4. 허용되지 않은 새 외부 의존성(예: `editor-57`의 TS/JS 내장 서비스용 임베디드 JS 엔진).
5. 배치 32·33(기본 앱 전환, TS·Tauri 제거).
6. 사용자 실기(픽셀, CJK IME, VoiceOver, 실제 PTY, 다중 창 focus)는 배치 31에서 사용자와 함께 합니다.
7. 작업자(하위 에이전트)는 GUI 실행, 합성 OS 입력(키·마우스), 클립보드, 저장소 밖 탐색, git 상태 변경을 하지 않습니다(lane workflow `RULES` 문자열).

## 3. 현재 진척(2026-11-04, 행을 직접 센 값)

### 3.1 왜 직접 세는가

기능표 상단의 합계 표(9~21줄)는 3줄의 "기준: 2026-10-14"이며 그 뒤로 갱신되지 않았습니다. lane workflow가 작업자에게 "상단 합계 표는 건드리지 않습니다"라고 지시하기 때문입니다(`docs/utils/2026-11-02-native-lane-workflow.js` `DONE` 문자열, 64줄). 그래서 상단 표의 complete 314는 현재 값이 아닙니다. 행을 직접 세야 합니다.

### 3.2 세는 방법

1. 행 수 확인. 각 영역 표의 행은 `| <영역>-<번호> |`로 시작합니다.

    ```
    grep -cE "^\| (shell|editor|terminal-agent|explorer-search|preview|git|settings|commands-ui)-[0-9]+[a-z]* +\|" /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md
    ```

    결과는 599입니다.

2. 판정 열로 영역별 집계. 표의 열은 `ID | 요구사항 | TS 근거 | 현재 판정 | 근거 묶음 | 범위/잔여 동작`입니다(기능표 92·198·292줄 등의 머리 행). 행이 `|`로 시작하므로 `awk -F'|'`에서 `$2`가 ID, `$5`가 판정입니다.

    ```
    awk -F'|' '/^\| (shell|editor|terminal-agent|explorer-search|preview|git|settings|commands-ui)-[0-9]+[a-z]* +\|/ { s=$5; gsub(/ /,"",s); id=$2; gsub(/ /,"",id); area=id; sub(/-[0-9]+[a-z]*$/,"",area); c[area" "s]++; t[s]++ } END { for (k in c) print k, c[k]; for (k in t) print "TOTAL", k, t[k] }' /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md
    ```

3. 함정: `git-61` 행의 요구사항 칸에 이스케이프된 파이프(`` `\|\|\|\|\|\|\|` ``)가 있어 열이 밀립니다. 이 행만 판정이 `\`로 집계됩니다. 다음 명령으로 그 행의 판정을 눈으로 확인해 해당 칸에 더합니다.

    ```
    grep "^| git-61 " /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md | rg -o "\| (complete|partial|unwired|missing|not-applicable|frozen) +\|"
    ```

    `| complete  |`가 나오며, awk 합계 complete에 1을 더합니다.

4. 기능 분모 = 전체 행 − not-applicable − frozen. 2026-10-10 계획 당시에는 not-applicable이 10이라 분모가 588이었고(`2026-10-10-native-remaining-batch-plan.md` 9줄), 이후 lane이 근거와 함께 재분류해 not-applicable이 늘었습니다. 분모는 매번 다시 계산합니다.

5. 백분율은 "행 비율"일 뿐 전체 전환율이 아닙니다. 행마다 규모가 다르고 같은 구현을 여러 행이 참조합니다(기능표 7줄 "행 수로 전체 전환율을 계산하지 않습니다", 계획 문서 47줄). 실기·성능·출시 게이트도 포함하지 않습니다.

### 3.3 메인 작업 트리(`to_rust_native` `7be1ad14`)의 현재 집계

| 영역 | 행 | complete | partial | unwired | missing | not-applicable | frozen |
| --- | --: | --: | --: | --: | --: | --: | --: |
| shell | 99 | 87 | 9 | 0 | 0 | 3 | 0 |
| editor | 87 | 75 | 8 | 0 | 1 | 3 | 0 |
| terminal-agent | 78 | 69 | 3 | 1 | 1 | 4 | 0 |
| explorer-search | 45 | 40 | 4 | 0 | 0 | 1 | 0 |
| preview | 40 | 34 | 3 | 0 | 0 | 2 | 1 |
| git | 65 | 60 | 4 | 0 | 0 | 1 | 0 |
| settings | 83 | 72 | 10 | 0 | 0 | 1 | 0 |
| commands-ui | 102 | 63 | 21 | 11 | 3 | 4 | 0 |
| 합계 | 599 | 500 | 62 | 12 | 5 | 19 | 1 |

- 이 표의 합계는 awk로 센 값과 일치합니다.
- git의 complete 60은 awk가 센 59에 `git-61`(complete) 1을 더한 값입니다.
- 분모 579, complete 500(86.4%), 미완료 79(partial 62 + unwired 12 + missing 5).
- git lane 병합 결과는 `docs/quality-assurance/2026-11-03-native-git-lane-merge.md`입니다.

### 3.5 진척 이력(같은 방법의 과거 기록)

| 시점 | complete | 분모 | 근거 |
| --- | --: | --: | --- |
| 2026-10-10 계획 고정 | 286(이후 배치 22로 288) | 588 | `2026-10-10-native-remaining-batch-plan.md` 9·45줄 |
| 2026-10-28 lane 전환 | 338 | 588 | 같은 문서 31줄 |
| 2026-11-03 wave 1 병합 뒤 | 405 | 580 | `docs/PROCESS.md` 31줄 |
| 2026-11-03 settings 병합 뒤 | 434 | 580 | PROCESS 32줄 |
| 2026-11-04 Git lane 병합 `f3a5b5d0` | 500 | 579 | 3.3절 |

## 4. 배치 23~33 계획과 실행 방식

### 4.1 배치 번호와 범위(고정)

2026-10-10에 남은 범위를 배치 23~33으로 고정했습니다. 세부 수정 때문에 새 배치 번호를 만들지 않습니다(`docs/quality-assurance/2026-10-10-native-remaining-batch-plan.md` "끝점과 범위" 7줄).

| 배치 | 범위 | 최초 배정 행 | 영역 |
| --- | --- | --: | --- |
| 23 | 편집기의 나머지 기능 | 33 | editor |
| 24 | 셸·창·프로젝트·탭 | 52 | shell |
| 25 | 탐색기·검색 | 23 | explorer-search |
| 26 | Git 화면과 동작 | 64 | git |
| 27 | 터미널·에이전트 | 32 | terminal-agent |
| 28 | 설정·통합 | 39 | settings |
| 29 | 명령·태스크·공통 UI 진입 | 51 | commands-ui |
| 30 | 프리뷰의 나머지 기능 | 6 | preview |
| 31 | 구조 정리와 전체 통합 게이트 | 별도 게이트 | 전체 |
| 32 | 기본 앱 전환과 베타 | 별도 게이트 | 사용자 허락 필요 |
| 33 | 기존 TS·Tauri 제거와 최종 검증 | 별도 게이트 | 사용자 허락 필요 |

- ID별 배정은 `docs/quality-assurance/2026-10-10-native-remaining-batch-plan.json`의 `assignments` 배열(302개, `"finalBatch": 33`)입니다. 302개 중 300개는 위 표의 "영역 → 배치" 규칙과 정확히 같고, 나머지 2개(`editor-41`, `editor-79`)는 배치 22로 이미 닫혔습니다. 확인 명령(결과 300):

    ```
    rg -U --count-matches "\"domain\": \"(shell\",\s+\"batch\": 24|editor\",\s+\"batch\": 23|explorer-search\",\s+\"batch\": 25|git\",\s+\"batch\": 26|terminal-agent\",\s+\"batch\": 27|settings\",\s+\"batch\": 28|commands-ui\",\s+\"batch\": 29|preview\",\s+\"batch\": 30)" /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-10-native-remaining-batch-plan.json
    ```

    따라서 남은 행의 배치는 영역 이름만 보면 정해집니다.
- 배치 31의 내용(계획 문서 22줄): 전체 기능표, crate·엔진·의존 경계, 자동 검사, 시각·IME·접근성·다중 창, 대형 성능·soak, 보안, 패키징, 데이터·rollback 호환을 실제 근거로 확인합니다. 여기에 lane 운영 문서 3절의 "편집기 장애 경계 설계"와 `docs/HANDOFF.md` 3.3절 "구조 정리" 행(native workspace 통합과 CI 편입, vendored fork를 `native/vendor`로 이동, 원격·IDE 서버 crate 추출, `application.rs`·`terminal_surface.rs` 분해, `include!` 테스트 복제 제거)이 들어갑니다.
- 배치 32(계획 문서 23줄): roadmap Phase 8의 기본 전환·이전 릴리스 fallback·실제 장기 세션/재시작/sleep-wake/다중 창·beta 회귀. 배치 33(24줄): 선행 게이트와 beta/rollback 확인 뒤에만 TS·Tauri·Monaco·xterm 경로 제거, 금지 참조·서명/공증/이전 데이터 검증.

### 4.2 실행 방식: 영역 lane 병렬(2026-10-28부터)

배치 번호와 ID 배정은 그대로 두고 실행만 영역 lane 병렬로 바꿨습니다(`docs/memory/2026-10-28-native-lane-operations.md`). 이유는 모든 기능이 `application.rs`·`host.rs`·UI `shell.rs`를 함께 고쳐 병렬 작업이 충돌했기 때문입니다(같은 문서 1절 표). lane 하나는 git worktree `/Users/hyunseokbyun/development/TAIDE-lanes/<lane>`(브랜치 `lane/<lane>`)와 전용 cargo target `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`을 씁니다(`docs/PROCESS.md` 28줄. lane 운영 문서 2.2절은 `<worktree>/target-lane`으로 적었지만 실제로 만든 것은 `targets/<lane>`입니다).

| wave | lane | 배치 | 상태(2026-11-04) | 근거 |
| --- | --- | --- | --- | --- |
| 0 | 없음(메인 직렬) | lane 경계 구조 이동, dev 프로필 debuginfo 축소, fmt 정리 | 완료 `d5559c46` `87bf1f51` | PROCESS 26줄 |
| 1 | shell · explorer-search · terminal-agent · editor · preview | 24 잔여, 25, 27, 23+30 | 병합 완료(preview `e87b053c`, shell `4fa4beff`, terminal-agent `446964b7`, editor `fb43d2ad`, explorer-search `6dba64f9`) | PROCESS 30줄, `git -C /Users/hyunseokbyun/development/TAIDE log --oneline` |
| 2 | settings · git | 28, 26 | settings `3864e1db`, git `f3a5b5d0` 모두 병합 완료 | PROCESS 32·33줄 |
| 3 | commands-ui | 29, 앞 wave에서 진입만 남은 행, shell-89 | 시작 전 | PROCESS 35줄 |
| 4 | 메인 | 31(편집기 장애 경계 설계 포함). 32·33은 사용자 허락 뒤 | 시작 전 | PROCESS 36줄 |

- 2026-11-02에 W1·W2를 합쳐 7개 lane을 동시에 돌렸습니다(빌드 한 벌 25GB, PROCESS 28줄). `git -C /Users/hyunseokbyun/development/TAIDE worktree list`에는 2026-11-04에도 7개 lane worktree가 모두 남아 있습니다. 모두 낡았으므로 사용 금지이고, 회수는 사용자 확인 후에 합니다. 병합이 끝난 worktree 회수(`git worktree remove`)와 target 정리는 메인이 합니다(lane 운영 문서 2.3절 4번).
- git-38 → explorer-search-7, git-7 → `notify_event`는 `f3a5b5d0`으로 완료됐습니다.

## 5. 남은 범위 지도

### 5.1 영역별 미완료 행(2026-11-04 현재, 79행)

아래 목록은 3.3절 기능표에서 complete·not-applicable·frozen이 아닌 행을 뽑은 것입니다(명령: 3.2절 awk의 조건을 `s!="complete" && s!="not-applicable" && s!="frozen"`으로 바꿔 ID를 출력). ID 옆 괄호가 판정입니다. 각 행의 남은 동작은 기능표 "범위/잔여 동작" 열에 있습니다. "사양"은 `docs/research/2026-10-28-lane-<영역>-spec.md`, "QA"는 `docs/quality-assurance/` 아래 파일입니다.

| 영역(배치) | 미완료 행 | 원본·설계 정본 | 최근 구현 기록 |
| --- | --- | --- | --- |
| shell(24), 9행 | shell-3 창 위치/크기 복원(partial), shell-5 보조 창 셸(partial), shell-12 전역 ErrorBoundary(partial: 편집기·루트 경계는 배치 31 설계), shell-15 기존 사용자 데이터 디렉터리(partial, 사용자 허락 필요), shell-59 explorer 4개 view 전환(partial), shell-78 탭 context menu(partial), shell-89 탭 내용 diff/commit diff/claudeDiff/searchEditor(partial, 남은 것: ClaudeDiff 수정 쪽 revert 화살표 `git-50`), shell-92 IDE 동기화(partial), shell-93 IPC 동기화(partial) | 사양 `lane-shell-spec.md` | QA `2026-11-02-native-lane-shell-providers.md`, `2026-11-02-native-lane-shell-windows-layers.md`, `2026-10-27-native-*.md` |
| editor(23), 9행 | editor-7 semantic token(missing), editor-15 minimap 토글(partial), editor-16 sticky scroll(partial), editor-32 formatOnType·formatOnPaste(partial), editor-34 Monaco 액션의 팔레트·키바인딩 노출(partial), editor-49 포맷(partial), editor-57 내장 언어 서비스 JSON·CSS·HTML·TS(partial, TS/JS는 임베디드 JS 엔진 허락 필요), editor-72 Markdown 미리보기 분할(partial), editor-76 diff 편집기(partial) | 사양 `lane-editor-preview-spec.md` | QA `2026-11-02-native-lane-editor-{css-services,html-services,json-services,display-commands}.md` |
| terminal-agent(27), 5행 | terminal-agent-19 Option-as-Meta(partial), -55 훅 설치·제거·상태 UI(unwired), -56 CLI 설치 상태·외부 에디터 연결(partial), -64 설정 화면 테스트 알림·시스템 설정 열기(missing), -68 openDiff → Claude diff 탭(partial, 남은 것은 git-50·git-52) | 사양 `lane-terminal-agent-spec.md` | QA `2026-11-02-native-lane-terminal-agent-*.md` |
| explorer-search(25), 4행 | explorer-search-16 삭제 확인(partial), -20 터미널에서 열기(partial), -21 폴더에서 찾기(partial), -33 교체(partial) | 사양 `lane-explorer-search-spec.md` | QA `2026-11-02-native-lane-explorer-search-*.md` |
| preview(30), 3행 | preview-5 외부 저장·이동·삭제 시 갱신(partial), preview-14 비디오(partial), preview-15 오디오(partial) | 사양 `lane-editor-preview-spec.md` | QA `2026-11-02-native-lane-preview-preview-markdown.md` |
| git(26), 4행 | git-40 Diff 렌더(partial: 문자 단위 강조·±·overview ruler), git-42 diff 설정 반영(partial), git-50 ClaudeDiffPane 렌더(partial: revert 화살표), git-52 탭 닫힘/창 이동 시 암묵 reject(partial) | 사양 `lane-git-spec.md` | `docs/quality-assurance/2026-11-02-native-lane-git-g1-base.md ~ g8-blame-conflicts.md` |
| settings(28), 10행 | settings-5 탭 소유권(partial), -22 시험 알림(partial), -23 OS 알림 발송 범주(partial), -28 표시·입력 옵션의 편집기 반영(partial), -30 Terminal section(partial), -42 VSIX 가져오기 다이얼로그(partial), -48 Sync 반영(partial), -67 편집한 키 반영(partial), -78 오류 표시 경로(partial), -80 Interface 설정값 소비처(partial) | 사양 `lane-settings-spec.md` | QA `2026-11-02-native-lane-settings-s{1..5}-*.md` |
| commands-ui(29), 35행 | missing 3: commands-ui-1 `window.reload`, -82 error-boundary(구역 격리, 배치 31 설계 필요), -83 status-error-banner. unwired 11: -14 `view.git`, -30·31 sync upload/download, -32 `cli.connectExternalEditor`, -33·34 CLI 설치·해제, -36 `git.revertHead`, -37 `git.createTagOnHead`, -38 `git.toggleBlame`, -39 `git.openFileHistory`, -62 explorer/git 디스패치. partial 21: -43 `monaco.*` 액션, -59 디스패치 대상, -61 find/search/search-replace, -67~-77·-79 공용 위젯(alert-dialog·button·card·checkbox·command·context-menu·dialog·dropdown-menu·popover·progress·scroll·switch), -84 file-group-header, -85 toast, -86·87 전역 스타일, -96 비국제화 문자열, -98 lucide 아이콘 | 사양 `lane-commands-ui-spec.md`(3절 묶음 계획 W1-A·W1-B·3~8, 4.1절 결정) | QA `2026-11-02-native-lane-terminal-agent-commands-{ui-entry,ui-evidence,run}.md`(W1 선행 묶음은 terminal-agent lane이 수행) |

### 5.2 영역과 무관한 남은 일

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 편집기 상태 이동 | `NativeApplication` struct의 필드 109개 중 영역 상태 묶음은 7개(`*_state`)뿐이고, 편집기 상태 대부분은 아직 루트 필드입니다(seams 문서는 약 60필드로 기록). 편집기 필드를 추가하는 작업은 루트 struct와 생성자 `new_with_overrides`를 고쳐야 해 충돌 지점으로 남습니다 | `docs/research/2026-10-28-native-lane-seams.md` 5.9절 2~4단계, `native/taide-native-app/src/application.rs` `pub struct NativeApplication` |
| 편집기·루트 장애 경계 | 편집기 입력을 render 밖 트랜잭션으로 나누는 설계가 필요합니다. shell-12와 commands-ui-82가 여기에 걸려 있습니다 | PROCESS 21줄(E2), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md` "단계 E 결정" 표 |
| 테스트 신뢰성 | 부하 시 흔들리는 실제 App 검사, 기존 실패 2건(`editor_fonts`, `agent_probe`) | 02 문서 12절 |
| 실기 | 픽셀, CJK IME, VoiceOver, 실제 PTY, 다중 창 focus, 실제 OS 알림 | lane 사양 결정 "보류" 표 |
| 미결 사용자 결정 | `docs/HANDOFF.md` 6절에 추천안만 제시된 항목이 있습니다. 예: 전역 ⌘N(원본에 없음)은 지금도 `application.rs` `show_window`의 `input.consume_key(egui::Modifiers::COMMAND, egui::Key::N)` 분기로 남아 있습니다. 현재 해결 여부는 항목마다 코드와 해당 QA로 확인합니다 | `docs/HANDOFF.md` 6절, `docs/acknowledge/2026-10-06-native-transition-decisions.md` "미결" 절 |
| 전환·제거 | 배치 32·33은 사용자 허락 뒤에만 시작합니다 | PROCESS 36줄 |

## 6. 문서 지도(어느 문서가 무엇을 다루는가)

### 6.1 이 폴더(`docs/handoff/native-completion/`)

| 문서 | 다루는 것 |
| --- | --- |
| `01-product-and-goal.md`(이 문서) | 제품, 목표·완료 정의, 진척 집계 방법과 값, 배치 계획, 남은 범위 지도 |
| `02-stack-and-environment.md` | 버전과 근거, 디렉터리, 빌드·검사·실행·캡처 명령, 디스크, 알려진 실패와 판별 |
| `03-architecture.md` | crate 책임·의존 방향, 앱 내부 구조(영역 모듈, host 호출, intent·surface, 이벤트, 프레임 순서), 새 기능 연결 예시, 공용 파일 영역 블록 규칙 |
| `04-rules-and-conventions.md` | 사용자 전역 규칙과 저장소 합의를 native 작업에 맞게 모은 것 |
| `05-process-and-gates.md` | lane 병렬 작업 방식, 묶음 절차, 병합 게이트, 기능표 갱신 |
| `06-decisions.md` | 사용자 결정 색인 |
| `10-remaining-shell.md` ~ `16-remaining-commands-ui.md` | 영역별 남은 행의 원본 근거·설계·작업 순서(10 shell, 11 editor·preview, 12 explorer-search, 13 terminal-agent, 14 git, 15 settings, 16 commands-ui) |
| `20-gates-31-33.md` | 배치 31~33 게이트 |
| `30-pending-permissions.md` | 사용자 허락이 필요해 보류한 항목 |

이 폴더의 문서도 기준 커밋 시점의 사본입니다. 아래 원본 문서와 실제 코드가 다르면 실제 코드와 원본 문서를 따릅니다.

### 6.2 저장소의 원본 문서

| 문서 | 다루는 것 | 상태 |
| --- | --- | --- |
| `docs/PROCESS.md` 처음 80줄 | 현재 진행 단계, lane 병합 기록, 게이트 결과, 보류 목록. 파일이 매우 커서 처음 80줄만 읽습니다 | 현재 정본 |
| `docs/memory/2026-10-28-native-lane-operations.md` | lane 병렬 운영 방식, wave 순서, 통합 게이트, 중단·재개 | 현재 정본 |
| `docs/memory/2026-10-07-native-transition-operations.md` | 5절 화면 캡처, 6절 반복된 함정(1·3·4절은 lane 문서로 대체) | 5·6절만 유효 |
| `docs/research/2026-10-28-native-lane-seams.md` | 영역 모듈 구조(`domains/<영역>.rs`), 영역 하위 enum, 공용 파일 영역 블록 규칙, 영역별 테스트 필터 | 현재 정본(03 문서의 근거, 2026-11-02 갱신) |
| `docs/research/2026-10-28-lane-<영역>-spec.md` 7개(shell, editor-preview, explorer-search, terminal-agent, git, settings, commands-ui) | 영역의 남은 ID마다 원본 TS 근거(경로·줄), native 자산, 설계, 묶음 계획, 결정 | 구현 시작 전 필독. 사양과 실제 코드가 다르면 실제 코드를 따르고 QA에 차이를 적습니다(lane workflow `BACKGROUND`) |
| `docs/acknowledge/2026-10-06-native-transition-decisions.md` | 구문 엔진(`ferriki-textmate`·`ferroni`), remote-web 동결, 보조 창 다중 viewport, muda 메뉴, 기각 대안 | 유효 |
| `docs/acknowledge/2026-10-09-native-full-resume.md`, `2026-10-09-native-find-regex-decisions.md` | 전체 완료 범위, 원본 결함 비재현 원칙, `regress` 채택과 성능 기준 | 유효 |
| `docs/acknowledge/2026-10-26-native-autonomous-continuation.md` | 자율 진행 지시, 허락 보류 목록, 단계 E 결정, 2026-10-28 운영 변경 | 유효 |
| `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` | 사양 4.1절 결정의 추천안 채택, 중복 범위 소유, `notify_event` 단일 진입점, 보류 표 | 유효 |
| `docs/quality-assurance/2026-10-09-native-function-matrix.md` | 599행 기능표(행별 TS 근거·판정·남은 동작) | 행은 정본, 상단 합계 표는 낡음 |
| `docs/quality-assurance/2026-10-10-native-remaining-batch-plan.md`·`.json` | 배치 23~33 범위와 ID 배정 | 배정 정본 |
| `docs/quality-assurance/2026-11-02-native-lane-*.md` | lane 묶음별 구현·검증 기록 | 해당 묶음 정본 |
| `docs/utils/2026-11-03-native-merge-gate.sh` | 병합 게이트 스크립트(31줄) | `34fb88d7`부터 휴지통 검사 3개 `--skip` 포함, 루트 workspace 4개는 별도(00-README 1절 2번) |
| `docs/utils/2026-11-02-native-lane-workflow.js` | lane workflow 스크립트(작업자 지시 `BACKGROUND`·`RULES`·`DONE`, 리뷰 형식) | 다음 lane 실행 때 재사용 |
| `docs/roadmap-rust-native.md` | Phase 0~9, 10절 삭제 게이트 | 유효 |
| `docs/architecture.md` 9절 | native 구조 요약 | 2026-10-09 기준이라 일부 낡음. 03 문서가 대체 |
| `docs/HANDOFF.md` | 2026-10-09 스냅샷 | 낡음(파일 첫머리에 스스로 명시). 3.3절 구조 정리 목록과 6절 미결 결정 목록만 참고 |
