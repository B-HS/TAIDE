# HANDOFF — 2026-08-19 세션 스냅샷 (감사 T1 3차~T1-K·editor-pane·X-A 완결, Phase 8 사전준비)

> 최종 갱신: 2026-08-19 / dev HEAD: **`6671c31`** / prod(origin/main): **`19d77e6`**(T1-K 까지 반영,
> **X-A 청소 배치 3커밋(`f1ca1fb`·`8328023`·`6671c31`)은 dev 선행 — 병합 확인 대기**).
> 워킹트리 클린. **진행 중 작업 없음**. 다음 세션은 §6 확인 → §8 TODO 착수.
> 이 문서가 세션 인수인계 **단일 진입점**. 직전 스냅샷(손 QA·T0·T1 1/2차)은 `git show 85ea29b:docs/HANDOFF.md`.
> **잔여 작업 총량의 정본은 `docs/PROCESS.md` 상단 "잔여 작업 총괄" 절**(사용자 요청으로 신설 — 이 문서와 중복 유지하지 않는다).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유(ADR-0004), view 는 표시 전용. macOS(arm64) 우선.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J(완료) → 전문 QA(d) 통과 → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **전문 QA(d)** — 감사 정비: T0·T1 9묶음·editor-pane·T1-K·X-A **완결**. 잔여 = T1-I·T1-H(고위험 2묶음)·T2·실기 QA |
| 직전 작업 | X-A 청소 배치(d-15) 완결(`6671c31`) + 결정 패키지(§6) 제시 직후 세션 종료 |

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션 완료 (dev 커밋 17건: 3f3c767..6671c31)

각 배치 패턴(전 배치 공통): **착수 전 메인 실물 재확인 → 계약(acknowledge) → 구현 Workflow(sonnet+xhigh)
→ 4렌즈 검토(opus+xhigh)+건별 적대적 검증(opus+high) → confirmed 수정 → 메인 2차(스팟 실물+verify·vite
직접 재실행) → 커밋(dev)**. 이번 세션 4배치 모두 검토가 실결함을 적중(마지막 배치는 confirmed 16/16).

| 작업 | 커밋 | 계약 정본 | 핵심 |
|------|------|-----------|------|
| **감사 T1 3차**(d-12) | `933a052`+`46d5504` | `2026-08-19-audit-t1-batch3-contract.md` | T1-C 서버상태 13·T1-D 레지스트리 17(owner/root 스코프·브리지 팩토리 2종·12종 마이그레이션·LSP 세대 이벤트+`lsp_confirm_reinitialize`)·T1-A 프로바이더 3. 검토 critical: **원격 owner 위장** → `enforce_remote_owner_label`(dispatch 진입 재귀 강제 치환) |
| **Phase 8 사전준비**(e-0·e-1) | `55c9d89`·`b0cc457`·`2ccfff0`·`ae67d83` | `2026-08-19-phase8-signing-secrets.md` | secrets 5건(raw-viewer 선례 — MACOS_CERTIFICATE_P12/PASSWORD·APPLE_ID/APP_SPECIFIC_PASSWORD/TEAM_ID) **사용자 등록 완료**. `.github/workflows/release.yml` 이식(tags v*+수동만·서명 아이덴티티 자동 추출·graceful skip). 실값: Team ID `SN98P5V7J4`·인증서 키체인 실재 확인 |
| **editor-pane 묶음**(d-13) | `500dbce`+`2f40d06` | `2026-08-19-editor-pane-batch-contract.md` | 1087→369줄 훅 6개 분해(동작 무변경 — 검토가 라인 전수 대조 확증) + blame/conflict 쿼리화·무효화 회수 완결(`useSaveFile(projectId?)`)·저수준 구독 2곳 표준화·root-aware 5곳. 검토 수정: Compare 고착·waiter 세션키 큐·3중복 shared 승격 |
| **T1-K 기본 거부**(d-14) | `4b76dda`+`19d77e6` | `2026-08-19-audit-t1k-default-deny-contract.md` | dispatch 를 명시 허용(160)+거부(19, `RemoteDenialPolicy` 8분류)+기본 거부로 반전. 완전 분할·arm↔테이블 파리티 테스트(인위 누락 실측). **정책 무변경 전수 대조 2회 독립**(구현·검토). 신규 커맨드는 등재 없인 원격 기본 거부(기계 강제) — C16 근본 해소 |
| **잔여 총괄 절**(d-0g) | `c5e96d2` | PROCESS 상단 | 감사 트랙별 처리 현황·이월·QA·배포 잔량 단일 뷰(사용자 요청) |
| **X-A 청소**(d-15) | `8328023`+`6671c31` | `2026-08-19-xa-wiring-cleanup-contract.md` | 살리기 4(**viewState 저장/복원**·**from_app 에코**·revision 게이트·**터미널 cwd+파일 링크 실배선**)·지우기(focus-kind·app:ready·중복 커맨드 5종 → 커맨드 176+raw3·이벤트 23)·§5.1 잔여 5(ws writer 유한 대기·`lsp_report_reinitialize_failure`·tree_rows Option·terminal:exited 소비·open-with prune). 검토 **confirmed 16/16** → viewState useLayoutEffect 재설계·from_app temp sibling 실효화·`guard_terminal_path` 보안 가드·OSC7 종결자 수정 |

### 3.2 기준선 (2026-08-19 세션 말 실측)

- 프론트 테스트 **1375**(134파일) / Rust **1030 lib** + 6 통합 + 17 taide-cli. `bun run verify` 전체 +
  `bunx vite build` 그린(마지막 배치 커밋 직전 메인 직접 재실행).
- 커맨드 **176종**(+raw 3=179)·이벤트 **23종**·원격 ALLOWED 160 ⊎ DENIED 19(완전 분할 기계 강제).
- 신규 의존성 **0**(이번 세션 승인 요청 없음). argon2·toml 여전히 미승인.

### 3.3 진행 중

**없음.** 모든 배치가 검토·수정·커밋까지 완결.

### 3.4 미착수 — 잔여 (정본: `docs/PROCESS.md` 상단 "잔여 작업 총괄")

1. **T1-I 도메인 경계(C13, 12건, 위험 중~높음)** — 직전 결정 패키지의 추천안. 착수 시 감사 §9
   **미결 결정 10·12 확정 필요**(추천: 12-A `Project.capabilities` attach/detach 실현 + 10-A 코드를
   architecture.md:77 규칙에 맞춤 — 연동) + 위험 재고지(30엣지·plugin↔vsix 등 순환 절단).
2. **T1-H 락 IO(10건, 위험 최고)** — 96 호출지점 암묵 순서. 별도 신중 착수.
3. **T2 백로그 10묶음·T2-E AppError 별도 캠페인·미배정 ~40건.**
4. **소규모 이월**: T1 3차 §5.1 잔여(throwaway race 보류·ide 브로드캐스트/agent_hooks 대칭화/키링
   게이팅 — 제품 결정 3건 묶음) + X-A §6 신규 4건(viewState closed_tabs 복원 공백·useReplaceSearch
   invalidation 부재·REMOTE 폴링/push 중복·경로 predicate 한계).
5. **e2e 파일럿(d-6)·QA-W1 멀티윈도우 실기** — 사용자 준비 필요(§7). qa6 실기 누적(이번 세션 +3절:
   T1 3차·editor-pane 7항목·X-A 7항목. **asset:// webview 최우선** 유지).
6. **Phase 8 본착수** — 준비 완료(secrets·release.yml). 실행은 전문 QA(d) 통과 후 태그(v0.1.0)
   푸시 또는 수동.

### 3.5 알려진 미검증·미해결 (KNOWN ISSUE)

- **전량 실기 미검증 지속** — 기계 검증(verify·vite build)만. 이번 세션 신설 배선(viewState 복원·
  from_app 무점멸·터미널 링크·cwd 해석·Restart·원격 상태 push)은 특히 실기 확증 필요(qa6 절 참조).
- **asset:// 실 webview 라운드트립 미검증**(T1 2차 이월) — qa6 최우선 실기.
- 감사 미수정 잔여(T1-I·T1-H·T2)는 `architecture-audit.md` 가 정본.

## 4. 의사결정 요약 (상세·기각 대안은 각 계약 정본)

| 영역 | 채택 | 주요 기각 |
|------|------|-----------|
| T1 3차 세부 | R7#6 다중 창 LSP 구독 **제거 확정**(창별 독립 세션 정본화)·store 이관은 external-store 브리지·F6#5 범위 외 | 다중 구독 실현·zustand 도입 |
| owner 신뢰 모델 | dispatch 진입 재귀 강제 치환(`enforce_remote_owner_label`) — 클라이언트 자기신고 불신 | 도메인별 개별 검사만 |
| Phase 8 secrets | **raw-viewer 5-secret 방식**(Apple ID 공증·아이덴티티 자동 추출) — 값 재사용 | App Store Connect API 키 방식(기록만)·7-secret 안 |
| editor-pane | 분해(동작 무변경) 후 수정 2단 — 검토 가능성 우선 | 분해+수정 동시 |
| T1-K | **정책 무변경 구조 전환**(현행 허용/거부 그대로 이전) — ide 2종·키링·agent_hooks 대칭화는 후속 제품 결정 분리 | 전환과 정책 변경 동시 |
| 세대 가드 의미론 | race guard 로 정직화(위조 방어 아님 — 원격=인증 동급 신뢰). confirm/report 는 상태 전제로 방어, owner 검사 비대칭 도입 금지 | owner 스코프 추가(한쪽만) |
| OSC7 cwd | 순수 경로 시퀀스(자체 파서 전용)·file:// 무시·cwd 선제 검증 없이 **최종 경로 단일 root guard** | 표준 spec 구현·cwd 선제 가드(절대경로 오탈락) |
| from_app 스킵 | kind==='modified' 한정(트리 구조 변경은 항상 refresh)·FILE.CONTENT invalidation 유지(useReplaceSearch 회귀 방지 — F1 계약 이탈 판단 승인) | fromApp 전량 스킵(계약 원문) |

## 5. 사용자 방향성 & 작업 규칙 (직전 세션 계승 + 이번 세션 추가)

### 5.1 운영 방식 (역할 5단 — 계승)

- 오케스트레이팅·계약·2차 검토 = 메인(**직접 구현 금지**, 예외: 소규모 2차 — 이번 세션 useSaveFile
  확장·arm 파리티 테스트·접합부 2건 등). 구현 = **sonnet+xhigh** / 렌즈 검토 = **opus+xhigh**(4렌즈,
  **배치 특성별 재구성** — 분해 배치엔 분해충실성 렌즈 등) / 적대적 검증 = **opus+high**(major 이상
  건별) / 정찰 불요 시 생략(감사 정본+메인 실물 확인으로 대체 — 이번 세션 전 배치).
- 위임은 규모 무관 **Workflow 로만**. Rust 한 시점 한 에이전트. **에이전트 보고 불신** — 메인 실물
  재검증(스팟+verify 직접 재실행) 필수. 검토 원문 JSON 경로를 fixer 에 전달(전문 재읽기 패턴).
- **확인 질문은 추천안 패키지**("전부 추천안대로" 응답 가능). 다음 착수 질문에 **직전 산출물 prod
  병합 여부 항상 포함**. 새 결정 지점 발견 시 멈춰 묻기(§1.0 표 패턴).
- **효율보다 완벽** — 검토·검증 축소 금지(이번 세션 4배치 검토가 전부 실결함 적중, 마지막은 16/16).
  refuted 판정은 수정 금지. 계약 이탈 판단은 근거 실증+기록 시 승인 가능.

### 5.2 답변·코드 규칙 (계승 — 변경 없음)

- 한국어+존댓말·간결·이모지 금지. 검증 안 된 단언 금지. 보고만 하고 멈추지 말 것.
- arrow fn만·반환 타입 명시 금지·TS any/enum 금지(Rust enum 관행 허용)·주석 금지(영어 JSDoc·러스트
  doc 만)·매직넘버 금지·useCallback/useMemo 금지·삼항 2중첩 금지·named export·1파일 1컴포넌트·FSD
  위→아래·barrel 금지·서버상태 TanStack Query(queryOptions+QUERY_KEY).
- i18n locale/service.rs 4곳 동기+en⊆required. IPC 시간 f64·정수 u32. Rust 후 **반드시 cargo fmt**.
  신규 커맨드 배선 3곳+파리티+**T1-K ALLOWED/DENIED 등재**(신규 — 미등재 시 원격 기본 거부·테스트
  실패). 시크릿은 keyring 에만.

### 5.3 금지 사항 (계승 + 이번 세션 추가)

- 에이전트 셸 앱 실행 금지·main 직접 커밋·git add -A·force push·Co-Authored-By·.env 금지.
- **[신규] 병렬 에이전트 공유 워킹트리에서 `git stash` 금지** — 이번 세션 2회 사고(전량 복구됨).
  이후 모든 구현 Workflow 프롬프트에 명시할 것.
- **[신규] `mv` 로 파일 복원 시 mtime 보존이 cargo 캐시를 속임** — 복원 후 `touch` 필요.
- HACK·검사기 끄기 금지. 승인 외 신규 패키지 금지(누적 목록 직전 스냅샷과 동일 — 이번 세션 추가 0).
- 각 계약의 기각 대안 재론 금지. 구현 완료분 재구현 금지.
- git: main=prod, dev=개발, dev 자동 커밋·푸시 ON. **prod 병합은 `git branch -f main dev` 와
  `git push origin main` 분리 실행**. 재시도 대기 60초 고정. 무관 변경 커밋 분리.

## 6. 미해결 질문 / 사용자 확인 필요

1. **X-A 청소 배치 prod 병합** — dev 선행 3커밋(`f1ca1fb`·`8328023`·`6671c31`)을 main 병합할지
   (직전 결정 패키지 제시 직후 세션 종료 — 미응답).
2. **다음 배치** — 추천 T1-I(도메인 경계) vs T1-H(락 IO) vs e2e·실기 vs T2. T1-I 착수 시 감사 §9
   결정 10(architecture.md:77 정본 — 추천 A: 코드를 규칙에)·12(Project.capabilities — 추천 A: 실현)
   확정 + 위험 재고지 필요.
3. **제품 결정 3건 묶음**(후속 — 급하지 않음): ide_publish_diagnostics/notify_at_mention 원격 게이트·
   agent_hooks install/uninstall 대칭화·키링 게이팅 비대칭(R3#7).
4. **e2e 파일럿·QA-W1 실기** — 사용자 준비(§7) 후.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64), bun 1.3.14. cargo PATH 밖 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` |
| 실행 | dev = `bun run tauri dev`(사용자만). 빌드 = `bun run tauri build`(래퍼가 taide-cli 사이드카+bundle conf 자동 적용) |
| 검증 | `bun run verify`(typecheck→lint→format→bun test→cargo fmt/clippy/test) + `bunx vite build`. bindings 재생성 = cargo test |
| Phase 8 | GitHub secrets 5건 등록 완료·release.yml 준비 완료(트리거 tags `v*`+수동만 — dev/main 푸시로 발화 안 함). Apple Team ID `SN98P5V7J4`, Developer ID 인증서 키체인 실재 |
| e2e 준비(사용자) | ① `bun run tauri dev` ② 설정 REMOTE 비밀번호·활성화 ③ `export TAIDE_E2E_PASSWORD='<pw>'` ④ `bun run e2e` |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` |

## 8. 다음 세션 TODO (우선순위 순)

1. **prod 병합 확인**(§6-1) — X-A 청소 3커밋. 분리 실행.
2. **다음 배치 확인**(§6-2) — 추천 T1-I. 착수 시 결정 10-A·12-A 추천안 패키지 + 위험 재고지 →
   계약(관련: `src-tauri/src/domain/*/commands.rs` 도메인 간 30엣지·`architecture.md:77`·
   `Project.capabilities` 스텁(R4#9)) → Workflow. 감사 §4.2-C13 이 정본.
3. **T1-H 락 IO** — T1-I 후. `src-tauri/src/state.rs` begin_mutation·96 호출지점. 위험 최고 재고지.
4. **e2e 파일럿·QA-W1 실기** — 사용자 준비 후. qa6 이번 세션 +3절 포함 소화.
5. **소규모 이월·제품 결정 3건**(§3.4-4·§6-3) — 적절한 배치에 편입.
6. **T2 백로그·T2-E 캠페인·Phase 8 본착수**(d 통과 후).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | **상단 "잔여 작업 총괄"(잔량 정본)** + 시간순 체크리스트(d-12~d-15 이번 세션) |
| `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md` | T1 3차 계약(§3.5 구현·§5 검토·§5.1 이월) |
| `docs/acknowledge/2026-08-19-phase8-signing-secrets.md` | Phase 8 secrets 5건 정본(raw-viewer 선례) |
| `docs/acknowledge/2026-08-19-editor-pane-batch-contract.md` | editor-pane 계약(§4 구현·§5 검토) |
| `docs/acknowledge/2026-08-19-audit-t1k-default-deny-contract.md` | T1-K 계약(§4 구현·검토 기록) |
| `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` | X-A 청소 계약(§4 구현·§5 정정·§6 검토 종합·신규 이월 4) |
| `docs/quality-assurance/2026-08-18-architecture-audit.md` | **감사 정본**(297발견·C1~C16·T0~T2·§9 미결 결정 10·12) |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | 실기 QA 마스터(이번 세션 +3절: T1 3차·editor-pane·X-A) |
| `docs/ipc-contract.md` | IPC 정본 — 커맨드 176+raw3·이벤트 23·**원격 기본 거부 구조(ALLOWED 160⊎DENIED 19·RemoteDenialPolicy 8분류)** |
| `docs/data-model.md`·`architecture.md` | 도메인 타입·구조 정본(이번 세션 다수 정정 동기됨) |
| `.github/workflows/release.yml` | Phase 8 릴리스 워크플로(준비 완료·미실행) |
| 직전 세션 계약 5건(2026-08-18-*) | 손 QA·T0·T1 1/2차·pro-qa-design — 기결 참조용 |

## 10. 복기 신뢰도

- **높음**: 이번 세션 커밋 17건이 dev 고정. 계약 5건이 배치별 실시간 동기(§4 구현·§5/§6 검토 기록).
  각 배치 메인 2차 실물 재검증 수행. 기준선 수치 세션 말 실측(프론트 1375·Rust 1030).
- **중간**: 검토 원문 45~47건짜리 minor 전량은 스크래치패드(세션 소멸) — 실질 수정분·기각 사유
  요지는 각 계약 §5/§6 에 반영, 나머지 minor 는 재론 시 실코드 확인.
- **낮음**: 없음.
