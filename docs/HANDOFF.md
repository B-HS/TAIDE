# HANDOFF — 2026-08-21 세션 스냅샷 (실기 사건 대응 + UX/구조 9배치 완주·통합 운영 전환)

> 최종 갱신: 2026-08-21 / **main=dev 동기(d-30 포함 전량 병합 완료)**. 워킹트리 클린.
> d-30 검토 = major 0·minor 7(문서 정정 — 반영 완료). **다음 = d-31 통합 배치, 사용자 지시
> ("d30 까지만 해두고")로 착수 보류 — 재개는 사용자 확인 후.**
> 이 문서가 세션 인수인계 단일 진입점. 직전 스냅샷은 `git show 282b4e1:docs/HANDOFF.md`.
> 잔여 총량 정본은 `docs/PROCESS.md` 상단 "잔여 작업 총괄"(2026-08-21 현행화).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 의 에이전트 친화 데스크톱 IDE. 상태는 Rust 소유
(ADR-0004)·view 표시 전용. macOS(arm64) 우선.

## 2. 현재 목표·마일스톤

- 최종: PRD FR-A~J(완료) → 전문 QA(d) → Phase 8 배포(서명·공증).
- 현재: 전문 QA(d) — 감사 T0·T1 전체 + T2 의 I·D/F/G·A·B(3파일) 완결. **잔여는 5묶음 통합
  배치(d-31~d-35)로 압축**(사용자 지시 — `acknowledge/2026-08-21-batch-consolidation-
  decision.md` 정본). 이후는 사용자 결정·실기·Phase 8 만 남음.

## 3. 이번 세션(08-20~21) 완료 — 9배치 + 실기 사건 대응

전 배치 공통: 계약(acknowledge) → 구현 Workflow(sonnet+xhigh) → 4렌즈 검토(opus+xhigh,
배치 특성별 재구성) → major 적대적(opus+high) → confirmed 수정 → 메인 2차(스팟+verify·vite
직접) → 커밋 → prod 병합. **모든 배치에서 검토가 confirmed>0 적중**(적대적 refuted 는 2건 —
d-25 "체감 그대로" 주장·d-24 1차의 correctness-1(이후 재조정으로 전복)).

| 배치 | 내용 | 계약 정본 |
|------|------|-----------|
| d-21 T2-D/F/G | 접근성 8(Enter/Space 헬퍼·ARIA·label)·dead code 9·매직넘버 정본화. 검토가 배치 도입 실회귀 2건 적중 | `2026-08-20-t2dfg-cleanup-contract.md` |
| **d-22 크래시**(실기 보고) | 새 파일 열기 빈 창 — 근원 eecb493(T0) 재등록 effect+monaco addAction 무가드 재충전 사슬 확정(재조정 wf 가 반증을 전복). 렌더 중 조정+프리뷰 fiber 고정. **사용자 실기 확증 완료** | `2026-08-20-blank-window-hotfix-contract.md`(§1 사슬·§7) |
| d-24 봉인+방어층 | registry 등록 CodeEditor 이관(unregister-before-dispose)·ErrorBoundary 6곳(부팅 크래시 폴백 가시성 onCaught·영역별 sizeClassName)·사이드바/git 헤더 1px(실기 보고) | `2026-08-20-crash-class-seal-contract.md` |
| d-26 팔레트 UX(실기 보고) | 선택 하이라이트 가시화(shared CommandItem ring-app-accent — cmdk 소비처 7곳)·파일명+상대경로 2단·매칭 강조(전경)·유니코드 인덱스 보존 | `2026-08-20-palette-ux-contract.md` |
| d-26b 테마 데이터(실기 보고 "색상코드 확인" 적중) | 번들 12/36 테마 list 색 결함 — 업스트림 충실 정정+**변환 파이프라인 가드 확장**(무가드 chain→derived)+Rust 데이터 린트(FAIL 13 실측→PASS) | `2026-08-20-theme-list-colors-contract.md` |
| d-27 Welcome(사용자 요청) | last_opened_at+project_list_recent(커맨드 177·DENIED 20)·최근 프로젝트(활성 전환)·파일/폴더 열기(루트 검증·창 인지 target)·실효 키맵 카드·화면 통일 | `2026-08-20-welcome-page-contract.md` |
| d-25 부팅(실기 보고 5초) | 워처 attach 후절화(창 표시 비차단) — 검토 반영으로 빌드 무가드/등록 마이크로 가드 분리·활성 우선 정렬·FILE/git 공백 보정·종료 취소. dev 모드 잔여 비용은 monaco eager(2~4초, release 소멸) | `2026-08-20-boot-watcher-defer-contract.md` |
| d-28 T2-A | 중복 제거 18(+1)항목 — 13 fixed(공통 모듈 6 신설)·4 deferred. 검토가 감사 "인라인 3" 축 잔존 적중 → 완결 | `2026-08-20-t2a-dedup-contract.md` |
| d-29 T2-B settings-view | 926→201줄·섹션 12분할. 검토가 **스왑 게이트 언마운트 전제 오류** 적중(상태·구독·뮤테이션 콜백 수명) → 스왑 생존 관심사 컨테이너 환원 | `2026-08-21-t2b-settings-view-contract.md` |
| d-30 T2-B command-registry | 302→관심사 4파일(51+57+174+21)·소비처 갱신 12+무변경 5·테스트 3분할. 검토 major 0·minor 7(문서 정정 반영) | `2026-08-21-t2b-command-registry-contract.md` |

### 3.1 기준선 (2026-08-21 실측)

- bun test **1435** / Rust **1094**(1068+3+6+17). verify·vite build 전 배치 exit 0.
- 커맨드 **177**(+raw3)·이벤트 23·ALLOWED 160 ⊎ DENIED **20**·로케일 **792키×3**·의존성 추가 0.
- ErrorBoundary 6곳 신설(루트+영역 5)·shared 공통 모듈 다수 신설(activation-key·path-root·
  file-extension·code-editor-settings·noop-disposable·infra/clock·command-palette-query·
  command-catalog·keymap-category 등).

### 3.2 실기 확증 상태

- **크래시(d-22) 실기 확증 완료**("실기는 괜찮아"). 나머지 실기 항목은 qa6 신설 절 참조:
  T2-D/F/G 접근성 8절·크래시 수정 7항목·d-25 워처 6항목 + Welcome·팔레트·테마.
- FILE.RAW 는 어떤 이벤트로도 무효화 안 되는 **선재 결함**(d-25 검토 발견 — d-33 몫).

## 4. 운영 방식 (계승 + 이번 세션 갱신)

- 역할 5단 유지: 구현 sonnet+xhigh / 렌즈 opus+xhigh(배치 특성별 재구성·소형은 2렌즈 축소
  가능 — d-30 선례) / 적대적 opus+high(major 건별) / 진단 fable+high / 메인=오케스트레이팅·
  계약·2차(직접 구현 금지·소규모 2차 예외). 위임은 Workflow 로만·Rust 한 시점 한 에이전트.
  에이전트 보고 불신 — 메인 실물 재검증+verify 직접 재실행. 검토·구현 원문 JSON 스크래치패드
  보존 후 fixer 에 경로 전달. refuted 판정 수정 금지(단, 별도 검증 라운드가 소스로 전복하면
  재개 가능 — d-22 재조정 선례·전복 근거 계약 기록 필수).
- **[신규] 통합 배치 운영**(사용자 지시): d-31 부터 5묶음 — 같은 성격·위험 축은 한 계약·한
  검토 회전. 검토 축소가 아니라 오버헤드만 축소. 정본 `2026-08-21-batch-consolidation-
  decision.md`.
- **[신규] 오케스트레이션 교훈 3건**: ① 백그라운드 완료 판정은 **태스크 통지 기준**(산출
  파일 존재로 판정 금지 — 빈 파일 오판이 이중 워크플로 사고 유발, d-24) ② 스테이징 확인과
  커밋을 분리(add -u 는 untracked 미포함 — d-28 비원자 커밋 쌍 원인) ③ 구현 Workflow 는
  계약 §3 기록 필수 명시(`docs/feedback/2026-08-21-implementation-workflow-skips-contract-
  record.md`).
- 네트워크 오류(ENOTFOUND·500)로 에이전트 사망 시: 워킹트리 부분 산출물 실사 → 이어받기
  프롬프트로 재기동(d-25 fixer·d-24 선례).
- 코드·i18n 규칙 계승: arrow fn·주석 금지(JSDoc/러스트 doc 영어)·TS any/enum 금지·매직넘버
  금지·named export·FSD·barrel 금지·로케일 4곳 동기(MESSAGE_NAMESPACES+3 JSON·3언어 실번역)·
  신규 커맨드 표면 절차(배선 3곳+파리티+T1-K 등재)·domain_boundaries 화이트리스트·Rust 후
  cargo fmt·시크릿 keyring 만.
- git: dev 자동 커밋·푸시 ON. prod 병합 = checkout main && merge --ff-only dev && push &&
  checkout dev. 재시도 60초. 사용자 dev 서버 실행 중 가정(TS=HMR·Rust=재빌드 재시작 고지).

## 5. 미해결 / 사용자 확인 필요

1. **[다음 세션 TODO 1순위] d-31 통합 배치 착수 여부 확인** — 사용자 "d30 까지만" 지시로
   보류 중. 재개 승인 시: 계약 작성(정본 `2026-08-21-batch-consolidation-decision.md` §2 의
   d-31 T2-B TS 일괄 — mapping-tables·lsp-session-registry·git-panel·explorer-container·
   command-palette 실사 + d-26 이월 알파 토큰/search-match-row) → 통합 파이프라인(구현→렌즈
   영역 배분 검토→적대적→수정→2차→커밋→병합) 순.
2. **제품 결정 묶음**(자율 종점 이후): 원격 게이트 3건·AI 응답 타입 통합(bindings 표면 승인)·
   codex 절단/ollama↔omlx(행동 변경·재설계 판단).
3. e2e 파일럿·QA-W1 실기 — 사용자 준비 필요(§7).

## 6. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64)·bun 1.3.14. cargo: `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` |
| 실행·검증 | dev=`bun run tauri dev`(사용자만) / `bun run verify` + `bunx vite build` / bindings 재생성=cargo test |
| e2e 준비(사용자) | ① tauri dev ② REMOTE 비밀번호·활성화 ③ `export TAIDE_E2E_PASSWORD` ④ `bun run e2e` |
| Phase 8 | secrets 5·release.yml 준비 완료(태그 `v*`+수동). Team ID `SN98P5V7J4` |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` |
| 사용자 실기 환경(암묵) | 활성 테마 **darcula**·복원 프로젝트 RESUME(15,320파일)·`bun run tauri dev` 상시 실행 가정. 앱 로그(`~/Library/Logs/dev.taide.app/TAIDE.log`)의 타임스탬프는 **UTC**(KST−9h)·40KB KeepOne 회전. 로그의 "web content process terminated" 는 tauri-runtime-wry 2.11.4 의 **생성 시 오출력 버그**(크래시 증거 아님 — d-22 광역 진단 확정) |
| 장애 재시도 | API 500/ENOTFOUND 로 에이전트 사망 빈발(이번 세션 3회) — 부분 산출물 실사 후 이어받기 프롬프트로 재기동이 정착 패턴 |

## 7. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 상단 "잔여 작업 총괄"(2026-08-21 현행화·기준선) + d-체크리스트 |
| `docs/acknowledge/2026-08-21-batch-consolidation-decision.md` | **통합 배치 정본**(5묶음 정의·자율 종점) |
| `docs/acknowledge/2026-08-20-*.md` · `2026-08-21-*.md` | 이번 세션 계약 10건(각 §구현·§검토 반영이 실질 정본) |
| `docs/quality-assurance/2026-08-18-architecture-audit.md` | 감사 정본(297발견) — 잔여는 T2-C·E/J 와 이월뿐 |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | 실기 QA 마스터(이번 세션 다수 절 신설) |
| `docs/feedback/2026-08-21-implementation-workflow-skips-contract-record.md` | 프로세스 교정(구현 §3 기록 필수) |
| `docs/ipc-contract.md` | IPC 정본 — 커맨드 177·DENIED 20·이벤트 합성 발신 조건(d-25) |

## 8. 복기 신뢰도

- **높음**: 배치별 계약 §구현/§검토 반영이 실시간 동기·전 배치 메인 2차 수행·기준선 수치
  세션 말 verify 로그 직접 확인. 커밋·병합 이력 git 고정.
- **중간**: 검토·구현 원문 전문은 스크래치패드(세션 소멸) — 요지는 계약에 반영, 재론 시
  실코드 확인.
- **낮음**: 없음.
