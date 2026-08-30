# HANDOFF — 2026-08-30 세션 스냅샷 (v0.1.3·v0.1.5·v0.1.6 세 릴리스 완주 / 잔여 = draft 3건 공개·qa6·Phase 8 잔재)

> 최종 갱신: 2026-08-30 / HEAD = `7880ba1`(main=dev·origin 동기, 이 완주 기록 커밋이 그 위에
> 얹힘). 2026-08-29~30 연속 세션 — 전수조사 후속 d-50~53 완결 → **v0.1.3 릴리스** → 사용성
> 배치 1(터미널 Shift+Enter·파일트리 git 데코·SCM 키보드) → **v0.1.5 릴리스** → 사용성 배치
> 2(wf opus·max: Ctrl+G TAIDE 연결·팔레트 캐럿·빈 폴더 조사) → docs 정합 일괄 → **v0.1.6
> 릴리스**. 이 문서가 세션 인수인계 단일 진입점. 직전 스냅샷은 `git show 59015ee:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 의 에이전트 친화 데스크톱 IDE. 상태는 Rust 소유
(ADR-0004)·view 표시 전용. macOS(arm64) 우선. 도메인 `taide.gumyo.net`·identifier `net.gumyo.taide`.

## 2. 현재 목표

- 최종: PRD FR-A~J(완료) → 전문 QA(qa6 실기 진행 중) → Phase 8 배포.
- 현재 마일스톤: **v0.1.6 draft 완주**. **다음 한 줄 = 사용자: draft v0.1.3·v0.1.5·v0.1.6
  검토 후 Publish(전부 미공개 누적 상태) → 신규 기능 실기 확인 → qa6 계속.**

## 3. 완료 / 진행 중 / 미착수

### 3.1 이 연속 세션의 릴리스 3건 (정본: `deployment.md` §9·각 release-notes)

| 태그 | 런 | 내용 |
|------|-----|------|
| v0.1.3 | `33231960782` (10m32s 콜드) | 전수조사 정비 — fix 41·성능·설정 10종·thin LTO+opt3 |
| v0.1.5 | `33248319213` (9m13s) | 파일트리 git 데코(색+M/A/D/R/U/! 뱃지·폴더 전파)·터미널 Shift+Enter→LF·SCM 목록 ↑↓ 로빙 포커스. 0.1.4 는 숫자 4 금지로 건너뜀 |
| v0.1.6 | `33286185781` (6m49s 최단) | Claude Code Ctrl+G→TAIDE(EDITOR/VISUAL PTY 주입·`cli.connectExternalEditor` 커맨드)·팔레트 전체선택 해제·빈 폴더 비버그 확정+회귀 테스트 4 |

### 3.2 사용성 배치 2 요약 (wf `wf_a068d4a0`, opus·max, 7 에이전트)

- **Ctrl+G**: PTY 스폰 시 `EDITOR`/`VISUAL`= `<taide CLI 경로> --wait`(인용 금지 — Claude Code
  는 공백 split 후 shell 없이 spawn, claude 2.1.251 바이너리 실측). 경로는 /usr/local/bin
  심링크 우선·번들 사이드카 폴백·실패 시 생략. 부모 env 미존중(B안)·셸 rc 최종 우선.
- **팔레트**: 원인 = Radix FocusScope 의 `select()` + WKWebView 전체선택 복원.
  `onOpenAutoFocus` 대체 + 재열림 `[open]` 이펙트(`shared/lib/text-input-caret.ts`).
- **빈 폴더**: 초기 진단("libgit2 가 빈 디렉토리 방출")을 wf 가 **반증** — libgit2 1.9.6 은
  recurse(false)에서도 내용 검사로 빈/ignored-only 디렉토리 제외(소스+실측, git CLI 일치).
  무수정, 계약을 테스트로 고정. 사용자 목격 증상은 재현 정보 대기.

### 3.3 잔여

- **즉시**: ① draft 3건 Publish(사용자) ② 신규 기능 실기 — 파일트리 데코·Shift+Enter·SCM
  ↑↓·팔레트 캐럿·Ctrl+G 왕복(설치본에서 `cli.connectExternalEditor` 실행 후 새 터미널)
  ③ "빈 폴더" 로 보였던 폴더의 실제 내용 확인(`ls -a`) → 재추적 여부.
- qa6 실기 계속(`quality-assurance/2026-08-11-qa6-checklist.md`) — d-53 설정 10종·d-50/51
  수정 실기 포인트 포함. Phase 8 잔재(업데이터·Windows/Linux·OSC133 브리지)·도메인 프록시.
- 백로그 신규: 파일트리 ignored 흐림(ignore 판정 IPC)·kitty keyboard protocol(`backlog.md`).
- 이월(사용자 결정 대기): 직전 스냅샷 §3.3 목록 유지(`git show 59015ee:docs/HANDOFF.md`).

## 4. 의사결정 요약 (상세는 각 정본)

- **채택**: Shift+Enter→LF 매핑(kitty 프로토콜 구현은 backlog —
  `acknowledge/2026-08-29-terminal-shift-enter-decision.md`) / VISUAL 동시 주입 B안·부모 env
  미존중(`acknowledge/2026-08-30-usability-batch-decisions.md`) / EDITOR 값 셸 인용 금지(동
  문서 — 재론 방지) / 빈 폴더 비버그 확정(동 문서).
- **기각·보류**: 네이티브 터미널 임베드 불가(재확인 — terminal-reevaluation §0.1)·외부 터미널
  열기 커맨드 보류 / 기존 기각 목록 유지(직전 스냅샷 §4).

## 5. 사용자 방향성 & 작업 규칙 (직전 스냅샷에서 불변 — 요지만)

- 위임은 Workflow(사용자 지시 시 opus·max)·Rust 한 시점 한 에이전트·보고 불신(메인 실물
  재검증)·기각 재론 금지 — 정본 `docs/agent-operations.md`.
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

1. draft 3건 공개 여부 확인 + 신규 표면 실기 결과 청취(결함은 d-54~ 계약 파이프라인).
2. "빈 폴더" 재현 정보 확보 시 재추적(유력 가설: 트리 숨김 목록만 든 폴더 — wf C 트랙 openQuestion).
3. qa6 계속 / Phase 8 잔재 판단 / 백로그(ignored 흐림·kitty protocol) 착수 여부.

## 9. 문서 지도 (직전 스냅샷 + 이 세션 신규)

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 체크리스트(이 세션 절 5개 + 진행 중 큐. d-31~35 완결 절은 history 로 아카이브) |
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
