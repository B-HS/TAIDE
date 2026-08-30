# PROCESS 아카이브 — d-31~d-35 커밋·자율 진행 완료 절 (2026-08-30 이동)

> docs/PROCESS.md 가 1300줄을 넘어, 2026-08-24~25 에 완결된 아래 3개 절을 원문 그대로 이동한다.
> (ai-process §2.1 아카이브 규칙). 후속 큐 d-36~d-39 절은 진행 중 항목이 있어 PROCESS 에 유지.

## 완료: d-32~35 산출물 커밋 5분할 + main ff 병합 (2026-08-25 사용자 승인)

> 승인: 5분할(HANDOFF §6-1 원안 — ① refactor d-32 ② refactor+fix d-33 ③ feat d-34 ④ perf d-35
> ⑤ docs) + main ff-only 병합까지, 푸시 제외. 워킹트리는 4개 배치의 누적 상태라 복수 배치가
> 건드린 파일은 **지배 배치(그 파일의 본질 변경을 만든 배치)** 커밋에 배정하고 타 배치 동승분을
> 커밋 본문에 명기한다(파일 단위 스테이징 한계 — 중간 커밋의 단독 컴파일은 보장하지 않음,
> 최종 트리 = verify 그린 실측 상태).

- [x] a. 배치별 파일 배분표 산출 — 각 계약 filesChanged 실사 + diff 마커 스캔으로 215파일 전수 분류
- [x] b. 커밋 ① refactor(d-32 Rust 구조) `d695aba` — 11파일(lib.rs -632·project +368·layout 공통화)
- [x] c. 커밋 ② refactor+fix(d-33 재구조화·FILE.RAW·테마 이월) `67ab4e0` — 93파일(rename 48 전건 인식)
- [x] d. 커밋 ③ feat(d-34 AppError taxonomy) `5627be3` — 82파일(로케일 3·bindings 포함)
- [x] e. 커밋 ④ perf(d-35 Rust 하드닝) `20f3fd5` — 7파일
- [x] f. 커밋 ⑤ docs(계약 4건 신설+문서 현행화) — 이 커밋. git status 클린 확인
- [x] g. 커밋 후 검증 — `bun run verify` exit 0 + `bunx vite build` exit 0 재확인(커밋 ④ 시점 트리 실측)
- [x] h. main ff-only 병합(커밋 ⑤ 직후 실행) — HANDOFF/PROCESS 현행화 완료

## 완료: d-32~d-35 연속 자율 진행 (2026-08-24 지시 → 2026-08-25 완주 — **전량 미커밋 워킹트리 보존**)

> 지시: "나머지 작업들을 commit/push 하지 말고 쭉 이어서 마지막까지" — **커밋·푸시 금지**
> (워킹트리 누적), d-32→d-33→d-34→d-35 순차 완주. 각 배치의 계약→구현→검토→적대적→수정→
> 2차(verify) 파이프라인은 유지. 계약 결정 지점은 추천안 자체 채택 후 기록, bindings 표면·
> 행동 변경 등 사용자 결정 필수 항목만 이월. 자율 종점 = d-35 완료(이후 제품 결정 묶음은
> 사용자 개입 필요 — batch-consolidation-decision.md §3).

- [x] d-32 Rust 구조 일괄 — 구현(lib.rs 1453→850·layout 골격 13개 공통화 370→334)·검토 3렌즈(동시성 전부 통과·major L3-1 적대적 downgraded)·수정 전건·cargo 그린·bindings diff 0. 계약 §3·§4 완결
- [x] d-33 T2-C 재구조화 + 이월 소형 일괄 — TS 3축(48파일 재편·FILE.RAW·SwitchField 26·wsSymbol·ΔE 가드+3종 정정)+Rust 조각(구별성 린트·tree doc) 구현, 검토 3렌즈(major 1 → 적대적 downgraded·처분 ② 2패스 수리)·수정 8건 전건(bun 1463). 계약 §3·§4 완결 — 메인 2차 verify 는 d-34 와 묶어 실행
- [x] d-34 AppError 캠페인 — 설계(확정안 A)·구현 3단(R1 인프라+T2-J·R2 119+9건·T 소비처 78건)·검토 3렌즈(major 2 기계 확정 — 적대적 생략 d-30 선례)·수정 13건 전건·메인 2차 그린(bun 1481·Rust 1099·로케일 916키×3·bindings +16/-1). 계약 §3~§5 완결
- [x] d-35 Rust 하드닝 이월 일괄 — R1(git2 in-process 13건+pty_spawn 보류 해제 이관)·R2(repo 경로 키 push/fetch 직렬화·dispatch 상한 128·NoCache 기각·capability 이월 유지)·검토 3렌즈(동시성 major 3 → 적대적 전건 downgraded — 처분: doc 수용비용 명시+"원격 dispatch QoS" 캠페인 이월)·수정 doc 12항목·최종 verify 그린(bun 1481·Rust 1106·신규 테스트 7·vite 0·bindings 실토큰 d-34 순증만). 계약 §3~§5 완결

## 완료: d-31 통합 배치 — T2-B TS 일괄 + d-26 이월 저대비 (2026-08-24)

> 계약: `docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md`. 사용자 승인 "d-31 만" —
> 완료 후 멈추고 d-32 착수 여부 재확인. 결정 4건 = 계약 §1.1.

- [x] 0. 실사 — 5파일+이월 2건 유효 확정, 오배치 정정(import 위반 0·주석 문언뿐), github 2종 알파 실측 (계약 §0)
- [x] 1. 계약 확정 — 결정 4건(LSP 이동+소분할 / matchHighlight 병행 / git-panel 행 그룹만 / 팔레트 모드 분리)
- [x] 2. 구현 Workflow — wf_20fc8ee3-93d 5에이전트 전원 완료·§3 기록 계약 통합(3-A~3-E)
- [x] 3. 메인 실물 재검증(가드·JSON 1줄 diff·팔레트 배선·부수효과 잔류·선재 테스트 결함 실증) + verify exit 0(bun 1443·Rust 1095) + vite build exit 0 + bindings 무변경
- [x] 4. 검토 4렌즈 major 4·minor 19 → 적대적(L2-1 만 confirmed·3건 downgraded) → 수정 3병렬(A안+폴백: ayu/solarized 정정·everforest/rose-pine 예외 등재·재발 방지 테스트 신설·composite 승격·구조 minor 전건) — 수정 wf 는 세션 재시작으로 종료 기록 유실됐으나 저널 실사로 3/3 완주 확인
- [x] 5. 메인 2차(verify 1447/1097·vite·bindings 무변경·단독 실행 mock 2건 pass 실측) → 커밋 3분리(00fb1e6 refactor 구조·a65c46c fix 저대비·docs) → prod 병합 → HANDOFF/PROCESS 현행화 — **d-31 완결. 다음 = d-32 착수 여부 사용자 확인 대기**

