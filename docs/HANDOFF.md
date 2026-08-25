# HANDOFF — 2026-08-25 세션 스냅샷 (통합 배치 d-31~d-35 전량 완주·커밋 5분할 완료)

> 최종 갱신: 2026-08-25(후속 세션) / d-32~35 산출물 215파일을 사용자 승인(5분할+main ff 병합,
> 푸시 제외)으로 커밋 완료: ① `d695aba` refactor(d-32) ② `67ab4e0` refactor(d-33) ③
> `5627be3` feat(d-34) ④ `20f3fd5` perf(d-35) ⑤ docs(이 커밋). 워킹트리는 4배치 누적
> 상태였으므로 복수 배치가 건드린 파일은 지배 배치 커밋에 배정하고 동승분을 각 커밋 본문에
> 명기했다(중간 커밋의 단독 컴파일 비보장 — 최종 트리 = verify 그린).
> 이 문서가 세션 인수인계 단일 진입점. 직전 스냅샷은 `git show ac1cd27:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 의 에이전트 친화 데스크톱 IDE. 상태는 Rust 소유
(ADR-0004)·view 표시 전용. macOS(arm64) 우선.

## 2. 현재 목표

- 최종: PRD FR-A~J(완료) → 전문 QA(d) → Phase 8 배포(서명·공증).
- 현재: **통합 배치 큐(d-31~d-35) 전량 소진 + 커밋·병합 완료** — 감사 297발견의 T0·T1 전
  트랙+T2 전 트랙(B·C·E/J)+T1-H 이월이 완결됨. 남은 것은 §6 의 사용자 결정 묶음·이월
  캠페인·실기/e2e·Phase 8.
- 직전 작업: d-32~35 커밋 5분할·main ff 병합(2026-08-25 승인). **다음 한 줄 = §6-2 사용자
  결정 묶음 제시(§8-1).**

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션(08-24~25) 완료 — 각 배치 계약의 §0~§5 가 실질 정본

전 배치 공통 파이프라인: 계약(실사·결정) → 구현 Workflow(sonnet+xhigh) → 검토 3~4렌즈
(opus+xhigh) → major 적대적(opus+high) → 수정 → 메인 2차(verify 직접). **모든 배치에서
검토가 실결함 적중.**

| 배치 | 산출(대표 실물) | 계약 정본 |
|------|----------------|-----------|
| **d-31**(커밋 3: `00fb1e6` refactor·`a65c46c` fix·`ac1cd27` docs — prod 병합) | mapping-tables 4분할(`theme-convert/{ui-token-vocabulary,semantic-token-map,ansi-palette}.ts`)·lsp-session-registry→`entities/lsp/`+`shared/lib/lsp/initialize-params.ts`·팔레트 모드 분리(`features/command-palette/` 5+훅 2)·`features/git/git-change-group.tsx`·explorer 훅 2·matchHighlight 가드+CONTRAST_PAIRS 알파합성+번들 4종 정정+예외 2종+`bundled-theme-contrast.test.ts`+Rust 반투명 린트·`shared/lib/color.ts` composite 승격 | `2026-08-24-d31-t2b-ts-batch-contract.md` |
| **d-32**(`d695aba` refactor) | lib.rs 1453→**851**(도메인 이관 12함수+테스트 — window/layout/project/agent)·layout 커맨드 골격 13개 공통화(`run_layout_mutation`+`LayoutLocate`, 370→**338**)·`tests/domain_boundaries.rs` 화이트리스트 4건(문서 정련 종결) | `2026-08-24-d32-rust-structure-contract.md` |
| **d-33**(`67ab4e0` refactor) | shared/lib 재구조화 48파일(`bridge/` 26·`keymap/` 17 신설·monaco 합류 — theme/ 신설·"shared UI 3분할"은 근거 부재 보류)·FILE.RAW 무효화(`ipc-sync-provider`)·`features/settings/switch-field.tsx`(26회)·wsSymbol 하이라이트·`deltaE76`+구별성 가드(ΔE 2.3)+번들 3종 정정+Rust hex 2차 게이트·수리 사슬 상태색 우선 2패스(`contrast.ts`) | `2026-08-24-d33-restructure-carryover-contract.md` |
| **d-34**(`5627be3` feat) | `error.rs` 38→**100**(`Localized(LocalizedError)`+`AppErrorKind`+빌더 — bindings 의도 순증 +16/-1)·한국어 생성 지점 119+9건 전수 이관·로케일 `error.*` 126키(**916키×3** 실번역)·`normalizeAppError`/`describeIpcError`/`useIpcErrorMessage`(`shared/hooks/`)·소비처 78건·terminal raw invoke 정규화 | `2026-08-24-d34-apperror-campaign-contract.md` |
| **d-35**(`20f3fd5` perf) | git2 in-process 13건+pty_spawn(보류 해제) 가드 쥔 채 spawn_blocking 이관·repo 경로 키 push/fetch 직렬화(`GitStore::push_fetch_lock`)·원격 dispatch 상한 128(`RemoteDispatchLimiter`)·NoCache **기각**·capability deferred-attach **이월 유지**·doc 수용 비용 12항목 | `2026-08-25-d35-rust-hardening-contract.md` |

### 3.2 최종 검증 (2026-08-25 메인 직접 실측)

`bun run verify` exit 0 — bun **1481**(시작 1435, +46 전부 신규 테스트)·Rust **1106**
(1080+3+6+17, +12)·eslint 0 error(경고 6은 선재 관행)·`bunx vite build` exit 0·로케일
**916키×3** 완전 파리티·bindings 실토큰 델타 = d-34 의도 순증(Localized·AppErrorKind·
LocalizedError)만. 진행 중 작업 없음.

### 3.3 미착수 (이월 — 정본 위치 병기)

- **"원격 dispatch QoS" 캠페인**(d-35 계약 §5.1 — 신규): C1 취소 계열 permit 면제·C2 세션
  재검증/CancellationToken·C3 run_git 수명 관리(Child 기반 kill·GIT_TERMINAL_PROMPT·lowSpeed —
  호출부 9개 반경)·C4 permit 실패 seq 응답. **선행 조건**: 철회 semantics 계약 확정 +
  handle_socket 통합 테스트(현행 테스트 7건은 프리미티브만 고정 — 배선 미고정 한계 명시됨).
  채택 불가 확정: spawn_blocking timeout 씌우기·GIT_TERMINAL_PROMPT 단독 추가.
- 사용자 결정 필요: builtin_light matchHighlight(2.15)+대비 린트 정책(d-31 §4 L2-3)·번들
  재변환 정책(d-31 §5 추가 이월 — 번들 JSON 이 현행 파이프라인 산출물과 다름)·제품 결정
  묶음(원격 게이트 3·AI 응답 타입 통합·codex/omlx — `2026-08-21-batch-consolidation-decision.md` §3).
- 기술 이월: LspInstallProgress 한국어(d-34 §미결5)·{{placeholder}}↔with_arg 파리티 자동화
  (`docs/quality-assurance/2026-08-25-locale-arg-parity-checklist.md`)·FILE.RAW 캐시 스캔 2배
  비용(predicate 일원화 — d-33 §4 L3-7)·parentDirOf/PATH_SEPARATOR 3중복 shared 승격(d-31
  §3-D 정정 기록)·화이트리스트 함수 단위 입도(d-32 §4)·fuzzy 가중치 재설계(d-26 이월 유지).
- e2e 파일럿·QA-W1 실기(사용자 준비)·Phase 8 배포.

## 4. 의사결정 요약 (상세·기각 사유는 각 계약 §)

- **사용자 결정**: d-31 착수 "d-31 만"(이후 /goal 로 d-32~35 연속 승인) / d-31 계약 4건(LSP
  이동+소분할·matchHighlight 병행·git-panel 행 그룹만·팔레트 모드 분리) / L2-1 처분 A안+폴백.
- **적대적 판정(재론 금지 — 각 계약 § 가 정본)**: d-31 L2-0 downgraded(번들 손수정값 유지·
  theme-system §8.2.3 예외 명문화 — 재변환 일치 강제 기각) / d-31 L2-2·L2-3 downgraded(이월)
  / d-32 L3-1 downgraded(화이트리스트 문서 정련 (a) 채택 — capability 경유 (b)는 가드 충돌로
  기각·d-35 재확인) / d-33 L2-1 downgraded(처분 ② 상태색 우선 2패스 — 하드 필터 ①·validate
  ΔE ③은 실테마 2종 임포트 차단으로 기각) / d-34 major 2 는 3렌즈 수렴+실행 재현으로 기계
  확정(적대적 생략 — d-30 선례) / d-35 C1·C2·C3 전건 downgraded(도달성·심각도 미달 — 코드
  이월·doc 수용 비용만, timeout 안·GIT_TERMINAL_PROMPT 단독 기각 사유 명시).
- **설계 확정**: d-34 확정안 A(Localized newtype variant — B~F 기각 표는 계약 §3.2)·미결
  8건 추천안 채택(§3.3). d-35 NoCache 기각(FSEvents rename cookie 부재 — notify 소스 실사)·
  capability 이월(트레잇 재설계 선행).
- **/goal 운영 규칙**: 계약 결정 지점은 추천안 자체 채택 후 기록, bindings 표면·행동 변경
  등 사용자 결정 필수 항목만 이월(단 d-34 taxonomy 의 additive 표면 순증은 큐 정본이 승인한
  본질로 판정·기록).

## 5. 사용자 방향성 & 작업 규칙

- 역할 5단: 구현 sonnet+xhigh / 렌즈 검토 opus+xhigh(배치 특성별 3~4렌즈) / 적대적 opus+high
  (major 건별 — 단 3렌즈 수렴+실행 재현이면 d-30 선례로 생략 가능) / 메인=오케스트레이팅·계약·
  2차(직접 구현 금지·문서 정정 등 소규모 예외). 위임은 Workflow 로만·**Rust 한 시점 한
  에이전트**(TS 병렬은 허용 — 이번 세션 실증). 에이전트 보고 불신 — 메인 실물 재검증+verify
  직접 재실행. 원문 JSON 은 태스크 출력 파일로 보존·fixer 에 경로 전달. refuted/downgraded
  처분 준수(재론 금지).
- **커밋·푸시**: d-32~35 는 /goal 로 미커밋 — **/goal 은 그 세션 한정이라 새 세션에 자동
  승계되지 않는다.** 새 세션 기본값 = 사용자 지시 전 커밋·푸시 금지(git.md 원칙 복귀).
- 효율보다 완벽 — 검토·검증 축소 금지(이번 세션도 전 배치에서 검토가 실결함 적중: d-33 수리
  재유입·d-34 {{status}} 누락·raw invoke 회귀·d-35 동시성 3건). 보류가 잘못된 수정보다 낫다
  (d-35 계약 §1 원칙 — 적대적 3건이 실제로 이 원칙으로 처분됨).
- 확인 질문은 추천안 패키지·한국어+존댓말·간결·이모지 금지·검증 안 된 단언 금지·보고만 하고
  멈추지 말 것.
- 코드: arrow fn만·반환타입 명시 금지·any/enum 금지·주석 금지(영어 JSDoc·Rust `///` doc 만 —
  Rust 비-doc `//` 도 금지 대상, d-35 L3-1 선례)·매직넘버 금지(상수+근거 doc)·named export·
  1파일 1컴포넌트·FSD 위→아래·barrel 금지·타입 원본 유도·서버상태 TanStack Query.
- i18n: 로케일 3파일+`MESSAGE_NAMESPACES` **4곳 동기**(파리티 테스트가 기계 강제)·3언어
  실번역·ko 는 완결 문장 하우스 스타일(명사형 종결 금지)·조사 공백 분리(`{{name}} 을(를)`)·
  `{{placeholder}}`↔`with_arg` 파리티는 수동 체크(자동화 이월). 신규 커맨드는 배선 3곳+파리티+
  T1-K 등재. cargo 후 `cargo fmt`. 시크릿 keyring 만.
- cargo PATH: `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"`

### 5.1 오케스트레이션 교훈 (계승 + 이번 세션 신규)

- 계승: ① 백그라운드 완료 판정은 태스크 통지 기준 ② 스테이징 확인·커밋 분리 ③ 구현 Workflow
  에 계약 §3 기록 필수 ④ API 사망·세션 재시작 시 부분 산출물 실사 후 이어받기.
- **[신규] 세션 재시작 이어받기 실증 2회**: 워크플로 종료 기록이 유실돼도 이전 세션 디렉토리의
  `subagents/workflows/wf_*/journal.jsonl` 에 에이전트별 result 가 남는다 — 저널 실사로 완주
  여부 판정 후 재기동 없이 결과 회수(d-31 수정 wf·이번 세션 전환). 이전 세션의 workflow
  scripts 경로도 그대로 유효.
- **[신규] Stop hook(/goal) 하 대기**: 백그라운드 워크플로 대기는 `TaskOutput(block=true)`
  반복으로 턴을 유지(멈춤 금지 조건 준수).
- **[신규] 병행 작업 중 검토·구현 간섭**: 병렬 TS 에이전트가 서로의 중간 상태 typecheck
  에러를 관측할 수 있음(무해 — 종료 시점 재확인으로 해소, d-33 실례). 검토를 병행시킬 때는
  파일 스코프를 명시 분리.

## 6. 미해결 질문 / 사용자 확인 필요

1. ~~d-32~35 커밋 여부·분할~~ **해소(2026-08-25)** — 5분할+main ff 병합 승인·실행(문서 상단
   해시). 배분 규칙: 지배 배치 배정+동승분 커밋 본문 명기(diff 마커 스캔으로 215파일 전수 분류).
2. 사용자 결정 이월(§3.3): builtin_light·번들 재변환 정책·제품 결정 묶음.
3. 실기 미확증: 테마 색 변화(팔레트·검색 매칭 강조 — github/ayu/solarized/monokai/palenight/
   night-owl-light)·오류 토스트 3언어 표시·qa6 d-35 신설 2항목(동일 repo 동시 push 망 단절).
   앱은 Rust·번들 리소스 변경이 커서 **재빌드·재시작 필요**.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64)·bun 1.3.14. cargo PATH 는 §5 |
| 실행·검증 | dev=`bun run tauri dev`(사용자만) / `bun run verify` + `bunx vite build` / bindings 재생성=cargo test |
| git 상태 | d-32~35 커밋 5분할 완료(문서 상단 해시)·main=dev ff 동기·미푸시. stash 금지(공유 워킹트리) |
| e2e 준비(사용자) | ① tauri dev ② REMOTE 비밀번호·활성화 ③ `export TAIDE_E2E_PASSWORD` ④ `bun run e2e` |
| Phase 8 | secrets 5·release.yml 준비 완료(태그 `v*`+수동). Team ID `SN98P5V7J4` |
| 사용자 실기 환경(암묵) | 활성 테마 darcula·복원 프로젝트 RESUME·로그 `~/Library/Logs/dev.taide.app/TAIDE.log`(UTC·40KB 회전)·wry "web content process terminated" 는 오출력 버그 |
| 장애 재시도 | 세션 재시작·API 사망 시 §5.1 저널 실사 이어받기 패턴 |

## 8. 다음 세션 TODO (우선순위)

1. **d-32~35 커밋 여부·분할 확인**(§6-1) — 승인 시 각 계약의 filesChanged 로 배치별 선별
   스테이징(`git add -A` 금지)·Conventional Commits·prod 병합(main ff-only)까지.
2. 사용자 결정 묶음 제시(§6-2) — builtin_light/번들 재변환은 d-31 계약 §4·§5, 제품 결정은
   batch-consolidation-decision.md §3 이 정본.
3. (승인 시) "원격 dispatch QoS" 캠페인 — d-35 §5.1 의 선행 조건(철회 semantics 계약·
   handle_socket 통합 테스트)부터.
4. 실기/e2e — qa6 미확증 절 전량(§6-3 포함)·e2e 파일럿(사용자 준비 후).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 잔여 총괄(통합 배치 큐 전량 완료 표기)+d-31~35 체크리스트 |
| `docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md` | d-31 정본(§0 실사~§5 이월 — 검토·적대적 판정 포함) |
| `docs/acknowledge/2026-08-24-d32-rust-structure-contract.md` | d-32 정본(이관 맵·화이트리스트 판정) |
| `docs/acknowledge/2026-08-24-d33-restructure-carryover-contract.md` | d-33 정본(48파일 분류·ΔE 설계·2패스 수리) |
| `docs/acknowledge/2026-08-24-d34-apperror-campaign-contract.md` | d-34 정본(taxonomy 설계·매핑·미결 8건) |
| `docs/acknowledge/2026-08-25-d35-rust-hardening-contract.md` | d-35 정본(이관·직렬화·상한·기각/이월 판단·§5.1 QoS 이월) |
| `docs/acknowledge/2026-08-21-batch-consolidation-decision.md` | 통합 배치 큐 원 결정(§3 자율 종점 이후 = 현재 잔여) |
| `docs/theme-system.md` | §8.2 가드 2종·§8.2.3 재변환 예외 7종·수리 2패스 |
| `docs/architecture.md` | §2 tier-2 경계 경로·§2.1 blocking 풀 공유·§3 capability |
| `docs/ipc-contract.md` | AppError 6변종 표면·정규화/표시 헬퍼 계약 |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | 실기 QA 마스터(+d-35 신설 2항목) |
| `docs/quality-assurance/2026-08-25-locale-arg-parity-checklist.md` | placeholder↔with_arg 파리티 수동 체크(자동화 이월 근거) |

## 10. 복기 신뢰도

- **높음**: 배치별 계약 §0~§5 가 실시간 동기(구현·검토·적대적·수정 기록 전부)·최종 verify
  수치 메인 직접 실측·d-31 커밋 git 고정.
- **중간**: 검토·설계 원문 전문은 태스크 출력 파일(`/private/tmp/claude-501/.../tasks/*.output`,
  세션 스크래치 — 소멸 가능) — 요지는 계약에 반영, 재론 시 실코드 확인. d-34 설계 전문
  (`scratchpad/d34-design.md`)도 동일(요지는 계약 §3).
- **낮음**: 없음.
