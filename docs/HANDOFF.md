# HANDOFF — 2026-09-05 세션 스냅샷 (배치 3·4 완결 → v0.1.7 draft 완주 / 잔여 = draft 4건 공개·실기 8지표·e2e 14~25·수동 QA·결정 항목)

> 최종 갱신: 2026-09-05 / HEAD = `fb6208a`(chore(release) v0.1.7, dev=main, 이 docs 커밋이 그 위에 얹힘). 배치 3·4 완결 + **v0.1.7 draft 완주**(런 `33903401416`).
> 이 세션 — **사용성 배치 3**(퀵오픈 스테일 인덱스·파일 탭 열기 선검증 / 외부 URL 단일 오프너·OSC 8·
> WebView 네비게이션 가드 / explorer autoReveal)을 조사→계약→구현 wf(opus·xhigh)→리뷰 wf(sonnet·xhigh)
> →테스트 wf(fable·medium)→메인 2차 검증→커밋 5분할로 완결. 동시에 **사용성 배치 4**(OS 알림·Welcome·
> 성능 극한·프로젝트 목록·git 탭·터미널/탭 바 메뉴·테스트 하네스·CI) 조사 wf 2건 + 결정 7건 + 계약 작성
> 완료(구현 미착수). 작업 방식 역할표 갱신(리서치·구현 opus·xhigh / 리뷰 sonnet·xhigh / 테스트 fable·medium,
> 서브에이전트 금지). 직전 스냅샷은 `git show 3360aca:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 의 에이전트 친화 데스크톱 IDE. 상태는 Rust 소유
(ADR-0004)·view 표시 전용. macOS(arm64) 우선. 도메인 `taide.gumyo.net`·identifier `net.gumyo.taide`.

## 2. 현재 목표

- 최종: PRD FR-A~J(완료) → 전문 QA(qa6 실기 진행 중) → Phase 8 배포.
- 현재 마일스톤: **v0.1.7 draft 완주**(배치 3·4 수록). **다음 한 줄 = 사용자: draft v0.1.7 검토·공개(v0.1.3·5·6 누적) · 실기 8지표 측정(`quality-assurance/
  2026-09-04-perf-baseline.md` §2·§3, `TAIDE_PERF=1`) · e2e 14~25 실행(`TAIDE_E2E_NO_HMR=1 bun run tauri dev`
  + REMOTE 준비 후 `bun run e2e`) · 수동 QA(알림은 `tauri build` 산출물로·프로젝트 표시) · 결정 항목
  (계약 §3.4·§4.4 — JSDoc 범위 예외 명문화 여부·CI 액션 SHA 핀·`git.stashEmpty` 등) · draft 3건 공개.**
  메인 다음 착수는 그 결과에 따라(결함은 계약 §3/§4 에 추가).

## 3. 완료 / 진행 중 / 미착수

### 3.1 이 연속 세션의 릴리스 3건 (정본: `deployment.md` §9·각 release-notes)

| 태그 | 런 | 내용 |
|------|-----|------|
| v0.1.3 | `33231960782` (10m32s 콜드) | 전수조사 정비 — fix 41·성능·설정 10종·thin LTO+opt3 |
| v0.1.5 | `33248319213` (9m13s) | 파일트리 git 데코(색+M/A/D/R/U/! 뱃지·폴더 전파)·터미널 Shift+Enter→LF·SCM 목록 ↑↓ 로빙 포커스. 0.1.4 는 숫자 4 금지로 건너뜀 |
| v0.1.6 | `33286185781` (6m49s 최단) | Claude Code Ctrl+G→TAIDE(EDITOR/VISUAL PTY 주입·`cli.connectExternalEditor` 커맨드)·팔레트 전체선택 해제·빈 폴더 비버그 확정+회귀 테스트 4 |
| v0.1.7 | `33903401416` (8m34s, 새 크레이트로 부분 콜드) | 배치 3·4 전량 — 퀵오픈 선검증·외부 링크 OS 브라우저·autoReveal·OS 알림·Welcome·프로젝트 표시·git 섹션·터미널/탭 바 메뉴·성능·하네스·CI. dmg 15,078,659B |

### 3.2 사용성 배치 2 요약 (wf `wf_a068d4a0`, opus·max, 7 에이전트)

- **Ctrl+G**: PTY 스폰 시 `EDITOR`/`VISUAL`= `<taide CLI 경로> --wait`(인용 금지 — Claude Code
  는 공백 split 후 shell 없이 spawn, claude 2.1.251 바이너리 실측). 경로는 /usr/local/bin
  심링크 우선·번들 사이드카 폴백·실패 시 생략. 부모 env 미존중(B안)·셸 rc 최종 우선.
- **팔레트**: 원인 = Radix FocusScope 의 `select()` + WKWebView 전체선택 복원.
  `onOpenAutoFocus` 대체 + 재열림 `[open]` 이펙트(`shared/lib/text-input-caret.ts`).
- **빈 폴더**: 초기 진단("libgit2 가 빈 디렉토리 방출")을 wf 가 **반증** — libgit2 1.9.6 은
  recurse(false)에서도 내용 검사로 빈/ignored-only 디렉토리 제외(소스+실측, git CLI 일치).
  무수정, 계약을 테스트로 고정. 사용자 목격 증상은 재현 정보 대기.

### 3.3 사용성 배치 3 요약 (2026-09-04, 계약 `acknowledge/2026-09-04-usability-batch3-contract.md` §3 정본)

- **퀵오픈 "찾을 수 없는 파일"** (`ef2ae3a`): 원인 = `search_list_files` 프론트 캐시가 인앱 CRUD 에서
  무효화되지 않고(워처 300ms 에코만) `layout_open_tab` 이 존재 검증을 안 해 낡은 항목이 탭으로 열린 뒤
  `io::Error` 원문 노출. 수정 = 뮤테이션 4종 즉시 무효화 + `root_guard::ensure_existing_file` 선검증
  (`layout_open_tab`·IDE `openFile` 공유) + `error.file.notFound` 로케일 + `useOpenFileTab` 단일 진입점
  (10곳 치환, NotFound 시 인덱스 무효화) + 비-UTF8 경로 제외. 리뷰 A-1: `refetchType:'all'` 은 팔레트가
  항상 마운트된 disabled 옵저버 때문에 닫힌 상태에선 즉시 재-walk 를 만들지 못함(문서 정정, Rust 선검증이
  방어). 예외: app-shell 드래그앤드롭은 순차 await 필요로 미치환(문서화).
- **터미널 링크 창내 열림** (`14e129e`): 원인 = `window.open()` 선시도 순서 결함 + `linkHandler` 부재로
  OSC 8 링크가 xterm 기본 핸들러(confirm→무동작)로 샘 + WebView 네비게이션 가드 0(마크다운 앵커는 확정
  창내 열림). "데스크톱 non-null" 가설은 wry 0.55.1 소스로 반증·실측 미확증. 수정 = `openExternalUrl`
  단일 오프너(데스크톱=IPC 고정) + OSC 8 `linkHandler` + 앵커 위임 + Rust `navigation_guard`(main 창
  `create:false`+`from_config`, 보조 창 동일, `validate_external_url` 은 `infra/external_url.rs` 로 이동 —
  domain_boundaries 테스트) + opener JS 인터셉터 off.
- **explorer autoReveal** (`1f1ccbb`): reveal 인프라 재사용(`tree_reveal`·`selectPathRequest`), 활성 파일
  구독 훅 `useExplorerAutoReveal` + 순수 판정 `decideAutoReveal`(widgets/explorer — 리뷰로 shared 에서 이동)
  + 설정 `explorerAutoReveal`(기본 true). 게이트: 사이드바 가시·files 뷰·설정. 이미 보이는 행은 IPC 0.
- 테스트 (`0763994`): 단위 +19(bun 1867) · Rust +7(cargo 1265) · e2e 스펙 14~16 **미실행**.
- 작업 방식: 역할표 갱신 + `.claude/settings.json` 허용 목록(`d7424f7`, 승인 프롬프트 폭주 대응) +
  "파일 수정은 Edit/Write 도구로만" 규칙(배치 4 계약 §0).

### 3.4 사용성 배치 4 웨이브 1 요약 (2026-09-04, 계약 `acknowledge/2026-09-04-usability-batch4-contract.md` "## 3. 구현 기록 (웨이브 1)" 정본)

- **A OS 알림** (`8dfc769`): `tauri-plugin-notification` 2.4(MSRV 1.89), notification 도메인(`decide_delivery`
  순수 게이트 + `notification_notify`/`notification_open_system_settings`, 원격 거부), 설정 8종, 발화
  5카테고리. 권한 거부 감지는 플러그인이 항상 granted 라 불가 → 설정 섹션 버튼 상시 노출. 리뷰 major:
  터미널 탭 백그라운드 시 xterm 언마운트로 완료 감지 불가 → pty reader 가 OSC 133 C/D 스캔·실측 계측
  후 `terminal:command-finished` 이벤트(창 무관, 보조 창도 해소). dev 실행에선 'Terminal' 이름으로 뜸.
- **B Welcome** (`8866af9`): `view.welcome` 커맨드 + 탭 0 빈 pane Welcome(메인 창, 설정 `welcomeOnEmptyEditor`).
- **D 프로젝트 표시** (`3e6baa7`): `ProjectDisplay` 양쪽 파일 미러·`project_set_display`·큐레이션 아이콘 56·
  라벨 1~4자·lane 색 12, 다이얼로그. FR-A4(focus 아이콘)는 보류 각주.
- **E git 섹션** (`10b0443`): 빈 Stash 섹션 최상단이 모호함의 원인 → Stashes 강등·0건 미렌더, 접이식 sticky
  헤더 통일, 접힘 모듈 메모리, 로빙 헤더 포함.
- **F 터미널 메뉴** (`aef0971`): `layout_open_tab_in_split`(insert_new_leaf 공유) — 2회 mutation 조합의
  이중 스폰 구조적 차단, 분할 4방향 비활성 판정(pane rect), 복사/붙여넣기/종료.
- **G 탭 바 메뉴** (`c93a4d8`): 여백·'+' 영역 메뉴(순수 빌더·표시 규칙 테스트), 탭 '이름 바꾸기' 브리지.
- 테스트 (`0f8dcc9` + 단위): bun 2008·cargo 1288·e2e 17~20 **미실행**. 미결(사용자 실기): 계약 §3.4·§3.7.

### 3.5 사용성 배치 4 웨이브 2 요약 (2026-09-05, 계약 "## 4. 구현 기록 (웨이브 2)" 정본, wf `wf_6e5eb63f` 15 에이전트 + 리뷰 `wf_3a0d5f31`)

- **계측** (`d2d7089`): Rust `infra::perf`(슬롯 12·카운터 4·AtomicU64·`TAIDE_PERF` 게이트·`perf_snapshot/reset`
  원격 거부·`[profile.profiling]`) + FE `perf-mark`(8지표 마크·상한·`app.showPerfSnapshot`) + `bun run bench`.
- **Rust 성능** (`c4e2af7`): project_open 후절화(build/register 2단 — 리뷰로 attach 실패 시 되돌림·에러 보고
  추가), 워처 ScopedIdCache(557k → 1.5k 엔트리, ~105MB → ~0.2MB), git status 캐시(3축 무효화 + 2초 TTL,
  히트 8.5ms → 20µs), 테마 OnceLock(18배), flush fsync 비동기, read_children file_type, sniff 이어읽기.
- **FE 성능** (`a48ff08`): fuzzy 단일 주사(5.6배), FILE 캐시 회수, 충돌 인덱스, git/아웃라인 가상화(+로빙·
  sticky 패딩), 마커 2단 셀렉터, 스크롤바 관찰 축소. 팔레트 가상화는 cmdk 공존 불가로 상한 유지(계약 허용).
- **테스트·CI** (`fae9693`·`dbb1d80`·`f83af47`·`141c3f1`): happy-dom+RTL 하네스(UA macOS 고정·mock.restore·
  cleanup), 단위/컴포넌트 +180·Rust +85(bun 2287·cargo 1435), e2e 21~25 + `TAIDE_E2E_NO_HMR` 게이트, ci.yml
  (push dev/main + PR, 프론트 ubuntu·Rust macOS). lint 경고 9 → 11(가상화 useVirtualizer 2, 기존과 동종).
- 정본: `quality-assurance/2026-09-04-perf-baseline.md`(실기 8지표 체크박스·벤치·Rust 픽스처 실측),
  `2026-09-04-test-gap-map.md`, `memory/test-conventions.md`(하네스 규약·함정).

### 3.6 잔여

- **즉시(사용자)**: ① 실기 8지표 측정(perf-baseline §2·§3) ② e2e 14~25 실행 + 배치 3·4 표면 실기(알림은
  `tauri build` 산출물로 — dev 는 Terminal 이름)·수동 QA(`e2e-harness.md` §5.1) ③ 결정 항목(계약 §4.4-8-1/8-2:
  JSDoc 예외 명문화·CI SHA 핀, §3.4 미결)(⌘P 삭제 파일 토스트·OSC 8/마크다운 링크가 OS
  브라우저로·autoReveal·설정 스위치·IDE openFile 없는 경로 거절 표시) ② draft 3건 Publish ③ "빈 폴더" 재현 정보.
- **다음(메인)**: 실기·e2e 결과 회신 후 결함 처리(계약 §3/§4 파이프라인). 이월 백로그(monaco 지연·트리 응답
  재설계·`ps` fork·update_index·ide-sync 마커 전량 티어·resolver 3 등 `backlog.md` "성능 극한 이월" 절). 신규 의존성 승인분: `tauri-plugin-notification`,
  devDependency happy-dom·@testing-library/react·@testing-library/dom.
- qa6 실기·Phase 8 잔재·백로그(ignored 흐림·kitty·terminal 메뉴 이월분·target:null pane NotFound 등) 유지.
- 이월(사용자 결정 대기): 직전 스냅샷 §3.3 목록 유지(`git show 3360aca:docs/HANDOFF.md`).

## 4. 의사결정 요약 (상세는 각 정본)

- **채택(2026-09-04)**: 리서치·구현 opus·xhigh / 리뷰 sonnet·xhigh / 테스트 fable·medium, Workflow 전용
  (`agent-operations.md` §1·`feedback/2026-09-04-research-must-use-workflow-opus-xhigh.md`) / 배치 4 결정
  7건(알림 비활성 창+완료성 이벤트·플러그인 승인 / 성능 8지표 계측+2단계 / 탭 바 여백 메뉴+탭 메뉴 보강 /
  Welcome 커맨드+탭 0 자동 표시 / 하네스 happy-dom+RTL / CI push·PR 게이트 / 성능 범위) —
  `acknowledge/2026-09-04-usability-batch4-user-decisions.md`.
- **채택**: Shift+Enter→LF 매핑(kitty 프로토콜 구현은 backlog —
  `acknowledge/2026-08-29-terminal-shift-enter-decision.md`) / VISUAL 동시 주입 B안·부모 env
  미존중(`acknowledge/2026-08-30-usability-batch-decisions.md`) / EDITOR 값 셸 인용 금지(동
  문서 — 재론 방지) / 빈 폴더 비버그 확정(동 문서).
- **기각·보류**: 네이티브 터미널 임베드 불가(재확인 — terminal-reevaluation §0.1)·외부 터미널
  열기 커맨드 보류 / 기존 기각 목록 유지(직전 스냅샷 §4).

## 5. 사용자 방향성 & 작업 규칙 (직전 스냅샷에서 불변 — 요지만)

- 위임은 Workflow 로만(서브에이전트 금지 — 리서치 포함). 역할표(2026-09-04 갱신): 리서치·구현
  opus·xhigh / 리뷰 sonnet·xhigh / 테스트 fable·medium. Rust 한 시점 한 에이전트·보고 불신(메인
  실물 재검증)·기각 재론 금지 — 정본 `docs/agent-operations.md` §1.
- 커밋: 대화 내 명시 지시 우선·논리 단위 분할·선별 스테이징·dev→main ff→양 브랜치 푸시·
  릴리스는 `deployment.md` §3(태그 4 금지·3파일 동기·노트 필수). AI 트레일러·add -A·force 금지.
- 단축키 작업 수칙(신규 정본): `feedback/2026-08-29-keyboard-shortcut-care.md` — 앱
  keymap→xterm→PTY/TUI 3층 경로 점검, 터미널 포커스 키는 기본 통과.
- cargo PATH: `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"`

## 6. 미해결 질문 / 사용자 확인 필요

1. draft **v0.1.3·v0.1.5·v0.1.6 Publish** (세 건 누적 — 본문·산출물 검토 후).
2. "빈 폴더" 증상의 실체(§3.3 ③) — 확보 전까지 재추적 불가로 종결 보류.
3. qa6 계속 → Phase 8 잔재 착수 판단.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64)·bun 1.4.0·tauri 2.11.x·React 19(Compiler). cargo PATH §5 |
| 버전·식별 | **v0.1.6**(3파일 동기 — CI 가드)·identifier prod `net.gumyo.taide` / dev `.dev` |
| git | HEAD=`7880ba1`+완주 기록 커밋(main=dev·origin 동기). 태그 v0.1.2/3/5/6(0/1 폐기·0.1.4 결번) |
| 기준선(2026-08-30 메인 실측) | bun **1817**/0(189파일)·cargo workspace **1248**/0·verify+vite build 전체 exit 0·로케일 **963키×3**(en/ko/ja) |
| 실행·검증 | dev=`bun run tauri dev`(사용자만)·`bun run verify`·릴리스 산출은 루트 `target/release/bundle/` |
| CI | release.yml 4-job 병렬 + cache-warm.yml(main push 워밍 — v0.1.6 에서 build 6m29s 실증) |
| 신규 표면(실기 미확증) | 파일트리 git 데코·Shift+Enter LF·SCM ↑↓·팔레트 캐럿·Ctrl+G EDITOR 주입 — 전부 기계 검증만 완료, 실기는 §3.3 ② |

## 8. 다음 세션 TODO (우선순위)

1. 사용자 실기·e2e·수동 QA 결과 청취 → 결함은 계약 §3/§4 에 추가해 wf 파이프라인(구현 opus·리뷰 sonnet·
   테스트 fable). 승인 프롬프트 최소화: 워크플로 공통 규칙에 "Edit/Write 전용·한 줄 단순 셸만" 유지, 셸 전체
   허용(`Bash(*)`)은 사용자만 추가 가능(에이전트 자가 확대는 분류기 차단).
2. draft 3건 공개 여부 확인 · 백로그 성능 이월 12건 착수 여부.
3. "빈 폴더" 재현 정보 확보 시 재추적(유력 가설: 트리 숨김 목록만 든 폴더 — wf C 트랙 openQuestion).
4. qa6 계속 / Phase 8 잔재 판단 / 백로그(ignored 흐림·kitty protocol) 착수 여부.

## 9. 문서 지도 (직전 스냅샷 + 이 세션 신규)

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 체크리스트 — "사용성 배치 3"(완결)·"사용성 배치 4"(대기) 절 |
| `docs/acknowledge/2026-09-04-usability-batch3-contract.md` | 배치 3 계약 + §3 구현·리뷰·테스트 기록(이탈·미결 전건) |
| `docs/acknowledge/2026-09-04-usability-batch4-contract.md` · `-user-decisions.md` | 배치 4 계약 A~H · 결정 7건 · "## 3. 구현 기록 (웨이브 1)"(이탈·미결·리뷰 수정·테스트) |
| `docs/quality-assurance/2026-09-04-git-section-ux-hand-qa.md` · `2026-08-18-e2e-harness.md` §5·§5.1 | git 섹션 손 QA · e2e 13~25 표 + 수동 QA(알림·프로젝트 표시) |
| `docs/quality-assurance/2026-09-04-perf-baseline.md` · `2026-09-04-test-gap-map.md` · `docs/memory/test-conventions.md` | 성능 기준선(실기 체크박스·벤치·Rust 실측) · 테스트 공백 맵 · 하네스 규약 |
| `docs/debugging.md` §TAIDE_PERF · `docs/architecture.md`(perf·capability 2단·캐시 회수) | 계측 사용법·구조 |
| `docs/research/2026-09-04-batch4-topics1-5-research.md` · `-terminal-tabbar-context-menu-research.md` | 배치 4 조사 원문(파일:라인 인용) |
| `docs/bug/2026-09-04-quick-open-stale-index-not-found.md` · `-external-link-opens-in-app-window.md` | 배치 3 버그 정본 2건 |
| `docs/feedback/2026-09-04-research-must-use-workflow-opus-xhigh.md` · `docs/agent-operations.md` §1 | 역할표·서브에이전트 금지 |
| `docs/features/command-palette.md` §3.1 · `terminal.md` §6.1 · `explorer-sidebar.md` §2.2 · `architecture.md`(네비게이션 가드) · `ipc-contract.md` | 배치 3 반영분 |
| `docs/quality-assurance/2026-08-18-e2e-harness.md` §5 | e2e 스펙 13~16 행(14~16 미실행) |
| `docs/deployment.md` | 배포 정본 — §9 이력에 v0.1.5·v0.1.6 추가 |
| `docs/features/agent-integration.md` §2.3 | EDITOR/VISUAL 주입 정본 |
| `docs/features/terminal.md` §8 · `explorer-sidebar.md` §2.2 · `git.md` §2 · `command-palette.md` §5 | Shift+Enter·git 데코·SCM 키보드·팔레트 캐럿 반영분 |
| `docs/acknowledge/2026-08-29-terminal-shift-enter-decision.md` · `2026-08-30-usability-batch-decisions.md` | 이 세션 결정 정본 2건 |
| `docs/bug/2026-08-30-palette-select-all-on-open.md` | 팔레트 전체선택 원인·해결 정본 |
| `docs/feedback/2026-08-29-keyboard-shortcut-care.md` | 단축키 작업 수칙 |
| `docs/release-notes/v0.1.3·5·6.md` | 릴리스 본문 3건 |
| 그 외 | 직전 스냅샷 §9 유지(`git show 59015ee:docs/HANDOFF.md`) |

## 10. 복기 신뢰도

- **높음**: 릴리스 3건 런 번호·산출물 크기 실측, 기준 수치 전부 이 세션 메인 실측(bun/cargo
  직접 실행), 결정·반증은 wf 저널과 acknowledge 정본에 고정.
- **중간**: 신규 표면 5종은 기계 검증만 — 실기 미확증(§7 마지막 행). Ctrl+G 왕복은 설치본
  번들에서만 완전 재현 가능(dev 빌드는 사이드카 부재로 주입 생략 경로).
- **낮음**: "빈 폴더" 사용자 증상의 실체(재현 정보 없음 — 가설 단계).
