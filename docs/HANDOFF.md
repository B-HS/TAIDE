# HANDOFF — 2026-08-08 세션 스냅샷

> 최종 갱신: 2026-08-08 / 대응 커밋: **`c8d33cd`** (`dev`, origin 푸시 완료, 워킹트리 깨끗함)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 수치는 이 문서 작성 시점에 **재실측**했다(§7).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J 전량 구현 → Phase 8 배포(서명·공증) |
| 직전 마일스톤 | **Phase 7.7~7.9 (실기 QA 1~3회차 반영) — 코드 완료** (이 세션, 커밋 21개) |
| 현재 상태 | **실기 재검증(QA 4회차) 대기** — 특히 IME 씹힘·CC 연결·테마 36종 |

## 3. 완료 / 진행 중 / 미착수

### 3.1 이 세션에서 완료 (전부 커밋·푸시됨, `13f31b2..c8d33cd`)

**Phase 7.7 — QA 1차 9그룹** (`70424a2`→`41d740b`)
| 내용 | 핵심 파일 |
|------|-----------|
| Problems 를 footer 아이콘+에디터 하단 패널로 재배치 | `status-bar.tsx`, `editor-area.tsx`, `app-shell.tsx` |
| 파일트리 프로젝트명 행·hover 툴바·인라인 생성(모달 제거) | `features/explorer/`, `widgets/explorer/` |
| 파일트리·탭 context menu 전량 + 미니맵 토글 | `file-tree-context-menu.tsx`, `tab-context-menu.tsx` |
| 오버레이 스크롤바 15표면 (state 0, ref 직접 갱신) | `shared/hooks/use-overlay-scrollbar.ts`, `shared/scroll/` |
| Claude Code IDE 연동 전체 (WS MCP 서버·lockfile·diff 탭·진단 push) | `src-tauri/src/domain/ide/`, `widgets/claude-diff-pane/` |
| footer CPU/RAM (TAIDE 프로세스만, 3s 폴링) | `domain/system/`, `entities/system/` |
| AI 상태 배지 + hooks 브리지 (`.claude/settings.local.json` 주입/제거) | `domain/agent/hooks.rs`, `features/project/agent-status-badge.tsx` |
| 설정 재구성·overflow 수정·opener 권한 재설계(`system_*` 커맨드 경유) | `infra/root_guard.rs`, `domain/system/commands.rs` |
| 검토: 발견 50→확정 44→수정 33 (opener 권한 재설계·openDiff pending 해소·레이아웃 뮤테이션 가드·diff 역조회 등) | `docs/PROCESS.md` 7.7-검토 절 |

**Phase 7.8 — QA 2차 11건** (`aa39bc4`→`a1e23af`)
| 내용 | 핵심 파일 |
|------|-----------|
| 탭 우클릭 메뉴 복구 — 7.5-C 부터 미동작(asChild 자식이 FC) + 전역 네이티브 메뉴 차단 | `sortable-tab.tsx`, `app-shell.tsx`, `docs/feedback/2026-08-07-radix-aschild-fc-trigger.md` |
| CC "Disconnected" 대응 — accept 재시도·stale lockfile 정리·mcp 서브프로토콜·부팅 env 레이스 대기·진단 로그·footer 3상태 | `domain/ide/server.rs`·`lockfile.rs`, `domain/terminal/commands.rs` |
| Monaco 위젯(find/peek/suggest/hover) 색 90키 매핑 | `shared/lib/monaco/theme.ts` |
| `⇧⌘F`(선택 시드)/`⇧⌘H`(replace) 실배선 + regex 토글 | `search-panel*.tsx`, `command-registry.ts` |
| 설정 15필드(에디터 11·터미널 3·UI 1) + 미니맵 토글 영속 + divider 0 | `settings-view.tsx`, `option-picker.tsx`, `code-editor.tsx` |
| untitled 탭 (더블클릭·+메뉴·⌘S save-as→File 변환, 휘발성) | `TabKind::Untitled`, `untitled-pane.tsx`, `untitled-registry.ts` |
| 디자인 토큰 정비 — 미매핑 43 해소·`dark:`→`data-appearance`·카드 분리 | `global.css`, `theme-provider.tsx` |
| 테마 라이브 프리뷰(쿼리 캐시 주입) + 번들 테마 10종 + 변환 스크립트 | `scripts/convert-vscode-theme.ts`, `src-tauri/resources/themes/` |
| 검토: 발견 29→확정 22→수정 8 (untitled 프로젝트 간 유실·검색 첫 호출 시딩·enablePreviewTabs 배선) + 번들 테마 폴백 재설계(fg=bg 3종 수정, 대비 검증 내장) | `cc0ea19`, `64fa63d` |

**Phase 7.9 — QA 3차 3건** (`de51533`→`c8d33cd`)
| 내용 | 핵심 파일 |
|------|-----------|
| **한글 씹힘 확정 수정** — 실기 로그로 원인 판명: xterm 이 음절 시작 insertText 를 간헐 미전송. `createInsertTextDeduper` 로 어댑터가 전송 직접 책임(50ms 중복 소비/자체 전송/늦은 중복 억제). 계측(링버퍼+팔레트 "IME 디버그 로그 복사") 유지 | `shared/lib/ime-input.ts`·`ime-debug.ts`, `terminal-view.tsx`, 정본 `docs/bug/2026-08-06-wkwebview-ime-composition.md` |
| AI 배지 지연 — ps 스캔 spawn_blocking 격리 + unix 폴링 500ms | `domain/agent/commands.rs`, `docs/acknowledge/2026-08-08-agent-badge-latency-decisions.md` |
| 번들 테마 26종 확충(총 36) — include 체인 병합·VSCode 공식 기본 ANSI 폴백·라이선스 전수 확인 | `scripts/convert-vscode-theme.ts`, `THIRD_PARTY_LICENSES.md` |

### 3.2 진행 중

**없음.** 모든 구현이 커밋·푸시됐고 워킹트리는 깨끗하다.

### 3.3 미착수 / 대기

- **실기 재검증 (최우선, §8)** — 특히: IME 씹힘 재검(재현 시 팔레트 "IME 디버그 로그 복사" 로 로그 확보 —
  이제 판정 경로 `:self-send`/`:already-sent`/`forward`/`drop` 이 표기됨), CC 연결(진단 로그 신설됨),
  테마 36종 시각, QA 2차 11건 전반
- `docs/backlog.md` — 3순위 + 7.7 보류 7건 + 7.8 보류 4건 (멀티 윈도우·hooks 세션 정밀화·Monaco syntax 19종 등)
- 추가 테마 후보 4종(검증 완료 대기): Winter is Coming·Andromeda·Cobalt2·SynthWave 84

### 3.4 알려진 미검증 (KNOWN ISSUE)

- **Windows 코드 컴파일 미검증** (`#[cfg(windows)]` — agent·pty). LSP 실환경 미검증.
- **CC 연동 실왕복 미확정** — 서버·lockfile·env 는 정적으로 정상이나 사용자 실기에서 "Disconnected" 1회 보고
  → accept 재시도·stale 정리·env 레이스 수정 후 재검증 대기. `FILE_SAVED` 응답 포맷(평문 vs JSON)도 실기 확인 필요.
- Working→Idle 배지 전이는 6s 디바운스가 지배적(의도값) — 더 줄이려면 실사용 텔레메트리 필요.
- one-monokai 는 원본에 foreground 키가 없어 일부 전경이 안전 기본값 — 시각 확인 필요.

## 4. 의사결정 요약

상세는 `docs/acknowledge/`(결정 1건 = 파일 1개, 이 세션 3건 추가) 및 `docs/adr/`.

### 4.1 채택 + 이유 (이 세션)

| 결정 | 이유 |
|------|------|
| **역할 분담 5단** (§5.1 표) | 사용자 확정 — 메인은 오케스트레이팅·계약·2차 검토만, 구현은 워크플로 |
| **CC 연동 전체 구현** (diff 편집 저장·진단 push 포함) | 사용자 확정. 프로토콜 비공식 인지 |
| **NEED DECISION = hooks 브리지** (휴리스틱은 idle/working 만) | 오탐이 배지 가치를 죽임 |
| **CPU/RAM = TAIDE 메인 프로세스만** | WKWebView 헬퍼 PPID=1 귀속 불가(실측) |
| **opener 권한 전면 제거 → `system_*` Rust 커맨드(루트 검증) 경유** | 1차 수정의 `path:**` 광역 개방을 2차 검토에서 회수 — 권한 최소화 |
| **hooks OFF 전이 시 주입분 자동 제거** (열린 프로젝트 한정) | 사용자가 권장안 위임 — OFF 상태에선 수동 제거 UI 가 사라지므로 |
| **번들 테마 ANSI 부재 시 VSCode 공식 기본 팔레트 폴백** | 발명이 아니라 VSCode 실동작의 정확한 이식 |
| **IME insertText 는 어댑터가 전송 책임** (deduper) | 실기 로그로 xterm 간헐 미전송 확정 — 전제 자체가 깨져 있었음 |
| **취약 작업(네트워크 다운로드)은 메인이 직접** | 테마 에이전트가 API 끊김으로 2회 사망 — 원본 확보 후 로컬 변환만 위임해 성공 |

### 4.2 기각·무효 판명 (**같은 삽질 반복 금지**)

| 기각안 | 이유 |
|--------|------|
| IME `getTargetRanges` 기반 수정 (`d59304c`) | **실기 로그에서 range 가 항상 빈 배열 — 가설 무효.** 코드는 무해한 폴백으로 잔존 |
| 휴리스틱으로 awaitingInput 추정 | API 대기와 입력 대기 구분 불가 — hooks 로만 |
| Cursor 자체 테마 번들 | 클로즈드소스 — 재배포 허가 없음. Material Theme·Panda 도 라이선스 미확정 제외 |
| JetBrains 원본 배포물(Darcula .icls 등) | 재배포 불가 — MIT 커뮤니티 포트만 채택 |
| opener capability `path:"**"`·`file://**` 스코프 | 권한 최소화 위반 — Rust 커맨드 경유로 재설계 |
| 테마 폴백의 색 계열 교차 (foreground↔background) | fg=bg 결함의 근본 원인 — 계열 내 폴백 + 대비 검증으로 재설계 |
| pty 출력 기반 배지 즉시 전이 | TUI idle 화면 재도장(스피너)이 "최근 출력"으로 오인되는 오탐 리스크 |
| 라이브 프리뷰 "매 렌더 clearPreview" 결함 주장 | **오탐** — React Compiler 실컴파일로 반환 객체 memo 확인. 실기 재현 시에만 재조사 |
| `ENABLE_IDE_INTEGRATION` env | CC 2.1.221 에 존재하지 않음(바이너리 0회) — `CLAUDE_CODE_SSE_PORT` 단독 |
| (기존 유지) TS7·Vec<u8> 채널·specta BigInt·shadcn resizable·xterm 교체·Tauri 버전업 IME·setBackgroundColor·Paraglide/Lingui·TabKind Preview·pptx LibreOffice | 이전 HANDOFF 참조 — 전부 유효 |

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (사용자 확정 — 이 세션 신규)

| 역할 | 담당 |
|------|------|
| 오케스트레이팅·상세플랜·계약 확정·배선 판단 | 메인 세션 (Fable) |
| 리서치·판단·최종 검증 | opus + medium |
| 구현 | sonnet + high |
| 버그 1차 검토·해결 | opus + high |
| 버그 2차 검토·해결 | Fable + medium |

- **메인은 직접 구현하지 않는다.** 예외: 사용자가 "fable 로 디버그"라 지정한 건(IME), 취약한 네트워크
  다운로드, 2차 검토에서의 소규모 직접 수정(주석 제거 등).
- 검증된 워크플로 패턴: **계약(스파인 단일 에이전트) → 구현 웨이브(파일 소유권 분리 병렬, 공유 파일은
  직렬 체인) → 검토(렌즈별 탐색 → 적대적 검증 → 수정) → 메인 2차(전체 verify 재실행 + 핵심 주장 실측)**.
- 스파인 파일(lib.rs·events.rs·locale/service.rs·settings·bindings·query-key.ts)은 **한 시점에 한 에이전트만**.
- 에이전트 보고는 검증 후 채택 — 이 세션 실사례: 쓰레기 구조화 보고("test"), 거짓 근거("기존 Rust 관행"),
  광역 권한 개방. 전부 메인 검증에서 잡았다.

### 5.2 답변·보고 스타일

- 한국어+존댓말, 간결, 미사여구·자축 금지, 이모지·아스키아트 금지.
- 검증 안 된 "완벽/잘 됨" 단언 금지. 보고만 하고 턴을 끝내지 말 것 — goal 이 있으면 이어서 진행.
- **버그는 추측 수정 금지** — 이 세션 교훈: IME 1차 수정(추측)은 실패, 계측→실기 로그→확정 수정이 성공했다.

### 5.3 코드 규칙 (ESLint 강제 + 파리티 테스트)

- arrow fn만, 반환 타입 명시 금지, any/enum 금지, 코드 주석 금지(JSDoc 영어만 — **Rust 도 동일**, doc 주석 금지),
  매직넘버 금지, useCallback/useMemo 금지, effect 동기 setState 금지, named export, 1파일 1컴포넌트,
  FSD 위→아래, barrel 금지, 색은 시맨틱 토큰만
- **i18n 키는 4곳 동기**: `domain/locale/service.rs` 의 MESSAGE_NAMESPACES 배열 + en/ko/ja (파리티 테스트)
- **테마 토큰은 5곳 동기**: theme/service.rs(네임스페이스·dark·light) + theme-tokens.ts + global.css + docs
- IPC 타입에 i64/u64/usize 금지 (u32/f64)
- Rust 수정 후 **cargo fmt 까지** 돌릴 것 (이 세션에서 메인이 빠뜨려 드리프트 1회)

### 5.4 금지 사항

- main 직접 커밋 금지 (dev 에서 작업, **자동 커밋·푸시 ON — dev 한정**, main 머지는 지시 시 직접 머지)
- git add -A 금지, force push 금지, Co-Authored-By 금지, .env 접근 금지
- 네이티브 UI 위젯 금지, HACK·검사기 끄기 금지(`#[allow]` 도 — 외부 트레잇 제약 등 불가피 시에만 + acknowledge 기록)
- 새 패키지 임의 설치 금지(보고 후 확인), **에이전트 셸에서 앱 실행 금지**(샌드박스·TCC 오진 2회 전례)
- Chrome 을 localhost:5173 에 붙인 채 두지 말 것 (vite 로그 오염)

## 6. 미해결 질문 / 사용자 확인 필요

1. **실기 재검증 결과** (§8 1순위) — IME·CC 연결·테마·배지 체감.
2. remote-control 범위 재합의 (기존 보류).
3. 추가 테마 4종(Winter is Coming 등) 확충 여부.
4. untitled 의 프로젝트 루트 밖 저장 허용 여부 (현재 root_guard 거부 — backlog).
5. 테마 크레딧 UI 노출 위치 (author/license/source 메타와 i18n 키는 준비됨 — backlog).

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS (arm64). Windows/Linux 미검증 |
| 패키지 매니저 | bun 1.3.x |
| **cargo 경로** | PATH 에 없음 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` |
| 리모트 | origin=github.com/B-HS/TAIDE (비공개). main=prod, dev=개발. 자동 커밋·푸시 ON(dev) |
| 실행 | `bun run tauri dev` (**사용자만** — 에이전트 셸 금지) |
| 검증 | `bun run verify` = typecheck→lint→format:check→bun test→cargo fmt/clippy/test |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` (themes/·locales/ 하위 포함) |

**현재 기준선 (2026-08-08 재실측)**
- 프론트 **332 tests**(32파일) / Rust **340 tests**(325 lib + 6 통합 + 9 CLI)
- IPC 커맨드 **115종**(specta 112 + raw 3), 이벤트 **20종**, Rust 도메인 **17개**(신설: system·ide)
- 번들 테마 **36종**(`src-tauri/resources/themes/`), lint 0 errors(허용 경고 4 — react-virtual 기존)
- 이 세션 커밋 21개 (`13f31b2..c8d33cd`)

**세션 소멸 주의**: 이 세션의 scratchpad(리서치 8+8건·플랜·다운로드 원본)는 세션 종료로 접근 불가.
필요 요지는 전부 docs(acknowledge·PROCESS·bug·feedback·backlog)에 반영돼 있다. 테마 재변환이 필요하면
원본은 `THIRD_PARTY_LICENSES.md` 의 출처 URL 에서 다시 받는다(다운로드는 메인이 직접 — §4.1 마지막 행).

## 8. 다음 세션 TODO (우선순위 순)

### 1순위 — 실기 재검증 (QA 4회차, 사용자 실행)

| 확인 대상 | 판정 방법 |
|-----------|-----------|
| 한글 빠른 입력 (CC 터미널 포함) | 씹힘·중복 없으면 완료. 재현 시 `⌘⇧P`→"IME 디버그 로그 복사" (판정 경로 표기됨) → 메인이 직접 재분석 |
| CC 연결 | 내장 터미널 `claude` → footer "연결됨". 실패 시 앱 로그의 ide 진단 로그(신설) 확보 |
| 배지 반영 속도 | 작업 시작→Working 이 1초 내. Working→Idle 은 6s 디바운스가 정상 |
| 테마 36종 | 설정→테마 목록에 내장/번들 구분 표시, 특히 Dark+·Darcula·One Monokai 가시성 |
| 7.8 잔여 확인 | 탭 우클릭 메뉴·untitled(프로젝트 전환 후 내용 유지)·⇧⌘F 시드·regex·설정 15필드·라이브 프리뷰 |

### 2순위 — 재검증에서 나온 버그 수정 (역할 분담대로)

### 3순위 — backlog 재검토

`docs/backlog.md` — 멀티 윈도우, hooks 세션 단위 정밀화, Monaco syntax 토큰 19종(번들 테마 에디터 구문색),
테마 크레딧 UI, untitled 루트 밖 저장, 추가 테마 4종, Monaco 스크롤바 색 통일 등.

### 4순위 — Phase 8 게이트

`docs/roadmap.md` Phase 8 (서명·공증·IDE MCP 정식·git 2차). **로컬 전수 확인 완료 후에만** (사용자 합의).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | Phase 0~7.9 시간순 체크리스트. **7.7 이후의 구현 순서 정본은 이 문서**(roadmap 은 7.6 까지) |
| `docs/roadmap.md` | Phase 0~8 로드맵 (7.6 까지 반영 — 7.7+ 는 PROCESS 참조) |
| `docs/backlog.md` | 보류·후속 후보 (7.7·7.8 보류분 포함, 항목별 보류 사유) |
| `docs/acknowledge/` | 결정 기록 — 이 세션: `2026-08-07-qa-batch-decisions`(운영 방식·CC·hooks·리소스 + §5 allow + §6 hooks OFF), `2026-08-07-a3-agent-activity-decisions`, `2026-08-08-agent-badge-latency-decisions` |
| `docs/bug/2026-08-06-wkwebview-ime-composition.md` | **IME 정본** — 조합 이벤트 부재→어댑터→빠른 입력 씹힘의 전체 이력·확정 원인·deduper 설계·계측 사용법 |
| `docs/feedback/2026-08-07-radix-aschild-fc-trigger.md` | Radix asChild 자식은 DOM 필수 — 재발 방지 |
| `docs/theme-system.md` | 테마 토큰·§8 번들 36종·변환 스크립트·ANSI 폴백 |
| `THIRD_PARTY_LICENSES.md` | 번들 테마 36종 출처·라이선스 (재변환 시 원본 URL) |
| `scripts/convert-vscode-theme.ts` | VSCode 테마 변환기 (include 체인·계열 폴백·대비 검증·ANSI 폴백) |
| `docs/PRD.md`·`architecture.md`·`tech-stack.md`·`ipc-contract.md`·`data-model.md`·`adr/`·`features/`·`research/`·`quality-assurance/` | 기존 정본 (이전 HANDOFF 와 동일 역할) |

## 10. 복기 신뢰도

- **높음**: 세션 전체가 커밋 21개로 고정돼 있고, 결정은 acknowledge/PROCESS/bug 문서에 즉시 기록하며 진행했다.
  §7 수치는 작성 시점 재실측.
- **중간**: 워크플로 에이전트가 구현한 세부 내부 로직(IDE 서버 내부·스크롤바 훅 세부·테마 변환기 세부) —
  게이트·검토 웨이브·메인 스팟체크로 확인했으나 실기 렌더는 QA 4회차 몫이다.
- **낮음**: 없음.
