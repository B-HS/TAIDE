# 감사 T1-K — 원격 게이팅 기본 거부 전환 계약 (2026-08-19)

> 정본: 감사 `2026-08-18-architecture-audit.md` §4.2-C16·§6.2 T1-K("dispatch.rs 를 명시 허용
> 목록+기본 거부로 뒤집고 거부 사유 분류를 타입으로 승격. T0-a 5항목 수정 후 착수")·§8-7.
> 사용자 승인(2026-08-19): prod 병합(main=2f40d06) + T1-K 착수. 선행 조건 충족: T0-a 5건(#12~
> #16)·hand-qa #12 system 4종·Phase E owner 위장 차단(enforce_remote_owner_label) 완료.
> 착수 전 메인 실물 확인: dispatch() 진입 = remote_denied_response(REMOTE_DENIED_COMMANDS 테이블)
> → enforce_remote_owner_label → match arm ~135+(respond 형) — **신규 커맨드의 arm 추가 = 자동
> 원격 허용**이 현 구조. deny_* 헬퍼 5종의 사유 doc 은 자유문(타입 아님). IMPLEMENTED_JSON_
> COMMANDS 180종 + RAW_CHANNEL_COMMANDS 3종.

## 1. 범위

### 1.1 구조 전환 (정책 무변경 — 현행 허용/거부를 그대로 이전)

- **`REMOTE_ALLOWED_COMMANDS` 명시 허용 테이블 신설**: 현행에서 실제로 원격 실행되는 전 커맨드
  (180 − 명시 거부)를 등재. 부분 스트립 계열(settings 등)은 허용으로 분류하되 arm 내 스트립
  로직 불변.
- **dispatch 진입 3단 게이트**: 거부 테이블 → **허용 테이블 미등재 시 기본 거부**(신규 응답 —
  미분류 커맨드는 명시 등재 전까지 원격 불가) → owner 강제 치환 → match. `dispatch_raw` 도
  RAW_CHANNEL_COMMANDS 기준 동일 원칙 확인.
- **거부 사유 분류 타입 승격**: deny_* 헬퍼 5종+개별 arm 의 자유문 사유를 Rust enum
  (예: `RemoteDenialPolicy` — 로컬 파일시스템 밖 쓰기 / 원격 도달 불가 창 / 설치·프로세스 실행 /
  데스크톱 CLI 가로채기 / 미분류 기본 거부)으로 승격, `REMOTE_DENIED_COMMANDS` 엔트리를
  (name, policy) 로 재구성하고 응답 메시지를 policy 에서 파생(중복 메시지 문자열 통합).
- **완전 분할 파리티 테스트(핵심 강제 장치)**: `collect_commands!` 기준 전 커맨드가
  ALLOWED ⊎ DENIED 정확히 한쪽에 등재(교집합 0·합집합 = 전체)를 테스트로 고정 — **신규 커맨드를
  어느 목록에도 안 넣으면 테스트 실패**(현행 "누락 = 무증상 자동 허용"의 반전). 기존 라우팅
  테스트(REMOTE_DENIED_COMMANDS 경유)·바인딩 파리티 유지.
- match 의 unknown fallback 도 기본 거부 응답으로 정합(방어선 이중화 — 파리티가 1차).

### 1.2 문서

- `docs/ipc-contract.md` §원격 dispatch 정책 — 기본 거부 구조·분류 타입·"신규 커맨드는 명시
  등재 필수" 규칙 갱신. `docs/architecture.md` 해당 절 정합.

### 1.3 범위 외

- **정책 변경 일절 없음**: `ide_publish_diagnostics`/`ide_notify_at_mention` 원격 거부 전환
  여부(§5.1-6 제품 결정)·키링 게이팅 비대칭(R3#7 — AI 토큰 3종·GitHub PAT 의 원격 변경 허용
  재검토)은 이 배치에서 다루지 않고 현행 그대로 ALLOWED 이전. 후속 결정 시 목록 이동 1줄이 되는
  것이 이 구조 전환의 목적.
- F6#5 동시 원격 다중 접속(기결 범위 외).

## 2. 실행 구조

- **Phase R(Rust 단독 1 에이전트, sonnet+xhigh)**: §1.1 전량 + ipc-contract·architecture 문서
  갱신. cargo fmt/clippy/test·전 파리티 그린. 허용 목록 작성은 **현행 match arm 실사**로(문서
  아닌 코드가 정본) — 이전 후 라우팅 동작 무변경을 커맨드 전수 대조로 자가 검증.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 보안: 분할 완전성·기본 거부 실효·스트립
  계열 무변경 / 계약: 정책 무변경 전수 대조·파리티 테스트가 신규 누락을 실제로 잡는지(테스트의
  테스트) / 정확성: 진입 순서(거부→허용→owner)·에러 응답 형식 호환 / 설계: enum 분류 적정)
  → 적대적(opus+high) → 수정 → 메인 2차 → 커밋(dev).

## 3. 완료 조건

- `bun run verify` + vite build 그린. 완전 분할 파리티 신설 테스트 그린 + 인위 누락 커맨드가
  실제로 실패하는지 확인(테스트의 테스트). 원격 라우팅 동작 전수 무변경(허용 180−8 유지·거부
  8종 유지·응답 메시지 의미 보존).
- 실기 이월(qa6): 원격 미러 주요 흐름(파일 열기·저장·검색·터미널·LSP) 회귀 무변화.

---

## 4. 구현·검토 완료 기록 (2026-08-19)

> 구현 wf_301ba751-fe1(Rust 단독 sonnet+xhigh) → 검토 wf_837798fb-989(4렌즈 opus+xhigh, 발견
> 28: major 3·minor 25) → 적대적(opus+high) **confirmed 2(전부 minor 하향 권고)·refuted 1** →
> 메인 직접 수정(소규모 2차 예외). 메인 2차: 스팟 실물 확인 + verify·vite build exit 0.

- **구현**: REMOTE_ALLOWED_COMMANDS 163 신설(match arm 실사)·3단 게이트(거부→미등재 기본 거부→
  owner 강제)·deny 헬퍼 15개 → RemoteDenialPolicy 8분류(SelfAccessExpansion·UnreachableDesktop
  Window·LocalFilesystemEscape·InstallOrProcessExecution·DesktopCliInterception·DesktopExit
  Control·SharedSingletonStateRace·Unclassified)·완전 분할 파리티 테스트(허용⊎거부=전체 183).
  **정책 무변경 전수 대조**: 구현·검토가 각각 독립 수행 — 구/신 허용·거부 집합 symmetric
  difference 공집합. dispatch_raw 도 테이블 공유(전환 전엔 거부 조회조차 없던 경로의 강화).
  bindings 무변경. 문서: ipc-contract 원격 정책 재작성·커맨드 수 180/183 정정(**X1#14 해소**)·
  architecture 정합.
- **검토 총평**: 우회 경로·정책 회귀 0(보안 렌즈 — ws→dispatch 유일 진입·HTTP 측 커맨드 경로
  없음·대소문자 변형 fail-closed 전수 추적). refuted 1: window_set_fullscreen 과병합 주장(AppHandle
  라벨 패턴으로 arm 작성 가능이 반증 근거).
- **[confirmed→메인 수정] arm↔ALLOWED 파리티 미강제** — 분할 테스트는 이름 우주만 검증, 허용
  이름의 실제 arm 존재는 수동 감사 의존. → `허용_테이블의_모든_커맨드는_실제_match_arm_을_가진다`
  신설(자기 소스 include_str!+8칸 들여쓰기 정규식 — lib.rs 선례, 결합 가정 doc 명문화). 인위
  누락(app_get_info) 실측: FAILED 확인 후 복원 그린(mv 의 mtime 보존이 cargo 캐시를 속이는 것까지
  확인·touch 로 해소).
- **minor 반영(메인)**: LocalFilesystemEscape·InstallOrProcessExecution doc 에 pty_spawn 능력
  상한 단서(표면 축소이지 기밀성 경계 아님 — 기존 strip doc 근거 이관) / ipc-contract
  DesktopCliInterception 사유를 enum doc(백도어 설치)과 통일 / **agent_hooks_uninstall User
  스코프 원격 허용을 문서에 명시**(전환 전 정책 동일 — install/uninstall 대칭화 여부는 ide
  브로드캐스트·키링 게이팅과 함께 후속 제품 결정으로 이월, PROCESS 잔여 총괄 9·10번 등재).
- **미반영 기각**: 나머지 minor(163 평면 배열 그룹핑 등 개선 여지)는 실결함 아님 — 검토 원문
  보존(스크래치패드 소멸 전 요지는 이 기록이 정본).
