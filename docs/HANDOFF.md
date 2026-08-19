# HANDOFF — 2026-08-20 세션 스냅샷 (T1 트랙 완결·T2 개시: T1-I·T1-H·T2-I 3배치 완주)

> 최종 갱신: 2026-08-20 / dev HEAD = main = origin 양측 = **`15cd460`** (**전 브랜치 동기 —
> 병합 대기분 없음**). 워킹트리 클린. **진행 중 작업 없음**(다음 배치 착수 직전 중단 — §6-1).
> 이 문서가 세션 인수인계 **단일 진입점**. 직전 스냅샷은 `git show 353d590:docs/HANDOFF.md`.
> **잔여 작업 총량의 정본은 `docs/PROCESS.md` 상단 "잔여 작업 총괄" 절**(중복 유지하지 않음).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유(ADR-0004), view 는 표시 전용. macOS(arm64) 우선.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J(완료) → 전문 QA(d) 통과 → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **전문 QA(d)** — **감사 T1 트랙 11묶음 전체 완결**(이번 세션 T1-I·T1-H 로 마감) + T2 개시(T2-I 완료). 잔여 = T2 나머지 9묶음·T2-E/J 캠페인·실기 QA·Phase 8 |
| 직전 작업 | T2-I 완결·prod 병합 직후, 다음 배치(추천 T2-D/F/G 통합 청소) 착수 직전 goal 해제·핸드오프 |

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션 완료 (dev 커밋 9건: f0e49a4..15cd460 — 전부 main 병합 완료)

전 배치 공통 패턴: **착수 전 메인 실물 재확인 → 계약(acknowledge) → 구현 Workflow(sonnet+
xhigh) → 4렌즈 검토(opus+xhigh, 배치 특성별 재구성)+major 이상 건별 적대적 검증(opus+high) →
confirmed 수정 Workflow → 메인 2차(스팟 실물+verify·vite 직접 재실행) → 커밋 → prod 병합**.
이번 세션 3배치 모두 적대적 검증 **refuted 0**(T1-H 는 confirmed 8 전건 — sync disconnect
되살림·blocking 풀 교착 등 실경합 적중. 검토 단계의 값어치 재재입증).

| 작업 | 커밋 | 계약 정본 | 핵심 |
|------|------|-----------|------|
| **T1-I 도메인 경계**(d-17) | `f0e49a4`+`1852820`+`9bad3df` | `2026-08-19-audit-t1i-domain-boundary-contract.md` | 결정 10-A·12-A 실행. `ProjectCapability` 확장점 8종(`domain/*/capability.rs`, open/close 수기 조립 이관·순서 바이트 재현), ide commands↔server 순환 절단(`ide/store.rs` 하강)·제2 진입점 service 경유(`layout/service.rs` 오케스트레이션 절), plugin↔vsix 단방향(`infra/archive.rs`), `infra/language.rs` `LanguageOverlay` 의존 반전, 조립부 배선 3종(SettingsToggleObservers·SystemUsageLabelProviders·PtySpawnEnvProvider — lib.rs 정적 등록), **`src-tauri/tests/domain_boundaries.rs` 아키텍처 테스트**(화이트리스트 기계 강제·stale 검출·import 우회 5형태 봉쇄). 검토 confirmed 2: C-1 스캔 우회(major)·D1 capabilities 명목 소비(minor 강등 — `detect_capabilities` 주입으로 단일 출처화) |
| **T1-H 락 IO**(d-18) | `bdf7c5c`+`100e6a4`+`e554c9f` | `2026-08-19-audit-t1h-lock-io-contract.md` | **begin_mutation 입도 재설계 기각(이월)** — 락-IO 결합 국소 해소만. git push/fetch 락 탈출·git_commit/init/undo spawn_blocking 이관·sync 3단계(`overlay_sync_bookkeeping`·`decide_download_apply` 재검증 대칭)·plugin/vsix stage/commit 분리·font OnceLock·tree_rows 되쓰기 제거+in-place. 검토 **confirmed 8·refuted 0**: sync disconnect 되살림(4렌즈 수렴)·stale 적용 역행·**git_pull blocking 풀 교착**(async 가드 동형화로 절단)·commit §1.4 오인용(실사 결함 정정) |
| **T2-I 로케일 외부화**(d-19) | `45428d8`+`194a7c3`+`15cd460` | `2026-08-19-t2i-locale-externalization-contract.md` | en/ko/ja 리터럴 → `src-tauri/resources/locales/{en,ko,ja}.json`(theme 선례). `locale/service.rs` **4,081→1,242줄**. 전수 파리티 기계 증명(구현 diff 0 + 검토 독립 재검증 — 이스케이프 표기까지 동일·**데이터 결함 0**). 미참조 32키 실측 제거(감사 39 정정)·보수 유지 77키 분해 기록. 검토 반영: `warm_builtin_catalogs` 부팅 eager 검증(파싱 실패 = loud crash)·should_panic/중복 키 테스트 |
| X-A 선행분 병합 | (기존 커밋) | — | 직전 세션 X-A 청소 3커밋+핸드오프를 세션 초두에 main 병합 |

### 3.2 기준선 (2026-08-20 세션 말 실측 — verify 로그 직접 확인)

- 프론트 테스트 **1375**(134파일) / Rust **1080**(lib 1054 + domain_boundaries 3 +
  session_restore 6 + taide-cli 17). `bun run verify`·`bunx vite build` 전부 exit 0.
- 커맨드 **176(+raw 3)**·이벤트 **23**·원격 ALLOWED 160 ⊎ DENIED 19 — 3배치 전부 **표면
  무변경**(bindings 는 doc 전사만).
- 신규 의존성 **0**. argon2·toml 여전히 미승인.
- 신규 기계 강제: 도메인 경계 아키텍처 테스트(T1-I — 위반=실패·stale 등재도 실패).

### 3.3 진행 중

**없음.** 3배치 전부 커밋·병합까지 완결. 다음 배치는 착수 직전(계약 미작성)에 중단 — §6-1.

### 3.4 미착수 — 잔여 (정본: `docs/PROCESS.md` 상단 "잔여 작업 총괄" + d-20 항목)

1. **T2-D/F/G 통합 청소 배치(다음 추천, d-20)** — 접근성 8·dead code 잔여·매직넘버 5.
   착수 직전 대표 실물 확인 완료(PROCESS d-20 에 기록): role='button' 6파일·entities/app
   3파일·OpenWithOverride 'preview' 실재 / T2-F 중 4건 기처리(app:ready=X-A·Project.
   capabilities=T1-I·resolve_terminal_path=X-A·로케일 키=T2-I) / DEFAULT_FONT_SIZE 는
   `shared/constants/terminal.ts:9` 1곳만 검출 — 감사 "중복 정의" 주장 재실사 필요.
   X-A 선례 형식(항목별 실사 후 유효분 수정) 권장.
2. **T2 나머지**: T2-A 중복 제거(17건)·T2-B 분해 잔여(settings-view 927·lib.rs·command-
   registry·git/service patch 등)·T2-C shared/lib 재구조화·T2-E/J AppError 369지점(별도 캠페인).
3. **이월 누적(각 계약 §5 정본)**: T1-I — AppState 공유 필드 스캔 사각지대(D8)·layout↔ide·
   window↔layout 순환·배선 4기구 통합(D6) / T1-H — git2 in-process 동기 IO 13건·repo 단위
   재진입 직렬화·원격 dispatch 동시 상한·pty_spawn·git_pull 네트워크 단계 락 탈출·
   begin_mutation 입도 재설계 / T2-I — 미판정 6키·F5 경량 summaries·D4 테이블화·R5#12
   역방향 검사 미결 / 직전 세션분(T1 3차 §5.1·X-A §6 신규 4건).
4. **제품 결정 3건**(사용자 몫): ide_publish_diagnostics/notify_at_mention 원격 게이트·
   agent_hooks install/uninstall 대칭화·키링 게이팅 비대칭(R3#7).
5. **e2e 파일럿·QA-W1 실기** — 사용자 준비 필요(§7). qa6 누적 +3절(T1-I 8·T1-H 9·T2-I 4항목).
6. **Phase 8 본착수** — 준비 완료 유지(secrets 5·release.yml). 전문 QA(d) 통과 후.

### 3.5 알려진 미검증·미해결 (KNOWN ISSUE)

- **전량 실기 미검증 지속** — 기계 검증만. 이번 세션 변경 중 특히 실기 필요: 프로젝트
  열기/닫기/재오픈(capability 순회)·IDE MCP 경로·git pull 중 저장 경합·sync 동시 조작·
  플러그인 설치 반응성·3언어 표시(qa6 신규 3절). **asset:// webview 라운드트립 최우선 유지**.
- T1-H 의 락 해제 구간 인터리빙은 검토(정적 추론)로만 확증 — 실동시 실행 검증은 실기 몫.
- 감사 미수정 잔여(T2 9묶음·이월)는 `architecture-audit.md` + 각 계약 §5 가 정본.

## 4. 의사결정 요약 (상세·기각 대안은 각 계약 정본)

| 영역 | 채택 | 주요 기각 |
|------|------|-----------|
| 도메인 경계 규칙(10-A) | 금지=함수 호출·Store 직접 참조 / 허용=types / infra→domain 전면 금지 / 화이트리스트+기계 강제(architecture.md §2 명문화) | 규칙을 코드에 맞춤(10-B)·분리안(10-C) |
| capability(12-A) | 동기·무오류 trait(실코드 도출)·조립부 정적 등록·부트 복원은 순회 미적용(동작 보존)·detect_capabilities 주입 단일 출처화 | async/Result 선언 강제·동적 레지스트리·부트 전체 순회 |
| T1-I 검토 기각 | — | D3 부트 이원화 해소·D4 DI 통일·D6 배선 4기구 통합·D7 ide 하강 심화·D8 즉시 해소(전부 재설계급 — 이월 기록) |
| T1-H 접근 | **입도 재설계 기각** — 락-IO 국소 해소만(105 호출지점 암묵 순서 보호). 분리 불명 지점은 보류(git_pull 네트워크·pty_spawn) | 락 쪼개기·도메인별 락·감사 문언 "축 분리"의 즉시 실행 |
| begin_mutation_blocking | 유계·짧은 대기 전용 규약(state.rs doc) — 장기 대기는 async 가드 후 spawn_blocking(git_pull 교착 교훈) | blocking 락으로 장기 park |
| sync 정합 규약 | 북키핑 last-write + 라이브 필드 소유 + **재획득 후 재검증**(gist id·last_synced 스냅샷 대조)·최초 생성만 가드 왕복 유지 | 무재검증 되쓰기·orphan gist 수용 |
| 로케일 외부화 | 평면 JSON+include_str!+OnceLock·MESSAGE_NAMESPACES 코드 유지(en 유도는 검증 순환)·파싱 실패 = 부팅 loud panic(theme .ok() 와 의도적 차이·근거 doc)·미판정 6키 보수 유지 | LocalePack 승격·조용한 폴백·네임스페이스 통째 제거 |
| prod 병합 절차 | **`git checkout main && git merge --ff-only dev && git push origin main && git checkout dev`** — `git branch -f` 가 이번 세션 권한 분류기에 차단됨(동등 절차로 전환·이후 세션도 이 절차) | branch -f(차단됨) |

## 5. 사용자 방향성 & 작업 규칙 (계승 + 이번 세션 갱신)

### 5.1 운영 방식 (역할 5단 — 계승·이번 세션 3배치 재검증)

- 오케스트레이팅·계약·2차 검토 = 메인(**직접 구현 금지**, 예외: 소규모 2차). 구현 =
  **sonnet+xhigh** / 렌즈 검토 = **opus+xhigh**(4렌즈, 배치 특성별 재구성 — T1-H 는 동시성
  렌즈 최우선·T2-I 는 이동충실성 렌즈가 구현 주장을 독립 재검증) / 적대적 = **opus+high**
  (major 이상 건별) / 정찰 생략(감사 정본+메인 실물 확인 대체 — 이번 세션 전 배치).
- 위임은 규모 무관 **Workflow 로만**. Rust 한 시점 한 에이전트(순차 다단 허용).
  **에이전트 보고 불신** — 메인 실물 재검증(스팟+verify 직접 재실행) 필수. 검토 원문 JSON 을
  스크래치패드에 보존해 fixer 에 경로 전달. **refuted 판정 수정 금지**(이번 세션 refuted 0).
- **효율보다 완벽** — 검토·검증 축소 금지(T1-H 검토가 기계 검증이 못 잡는 실경합 8건 전건
  적중). **보류가 잘못된 수정보다 낫다**(반쪽 수정 금지 — T1-H git_pull·pty_spawn 보류 선례).
- 확인 질문은 추천안 패키지("전부 추천안대로" 가능). 다음 착수 질문에 직전 산출물 prod 병합
  여부 포함(이번 세션은 전부 병합 완료 — 다음 세션 첫 질문은 배치 선택만). 새 결정 지점은
  멈춰 묻기.

### 5.2 답변·코드 규칙 (계승 — i18n 갱신)

- 한국어+존댓말·간결·이모지 금지. 검증 안 된 단언 금지. 보고만 하고 멈추지 말 것.
- arrow fn만·반환 타입 명시 금지·TS any/enum 금지(Rust enum 허용)·주석 금지(영어 JSDoc·러스트
  doc 만)·매직넘버 금지·useCallback/useMemo 금지·삼항 2중첩 금지·named export·1파일 1컴포넌트·
  FSD 위→아래·barrel 금지·서버상태 TanStack Query(queryOptions+QUERY_KEY).
- **i18n(T2-I 갱신): MESSAGE_NAMESPACES(locale/service.rs) + `resources/locales/{en,ko,ja}.
  json` 4곳 동기 + en⊆required** — 카탈로그는 JSON, 스키마만 코드. IPC 시간 f64·정수 u32.
  Rust 후 반드시 cargo fmt. 신규 커맨드 배선 3곳+파리티+T1-K ALLOWED/DENIED 등재. **[신규]
  도메인 간 참조는 `tests/domain_boundaries.rs` 화이트리스트 등재 없인 테스트 실패**(T1-I).
  시크릿 keyring 만.

### 5.3 금지 사항 (계승 + 이번 세션 갱신)

- 에이전트 셸 앱 실행 금지·main 직접 커밋·git add -A·force push·Co-Authored-By·.env 금지.
- 병렬 에이전트 공유 워킹트리 `git stash` 금지(전 Workflow 프롬프트 명시 유지 — 이번 세션
  사고 0). `mv` 복원 후 touch(cargo 캐시).
- HACK·검사기 끄기 금지. 승인 외 신규 패키지 금지(이번 세션 추가 0).
- 각 계약의 기각 대안 재론 금지·구현 완료분 재구현 금지.
- **[갱신] prod 병합은 §4 의 checkout+ff-only 절차**(branch -f 는 분류기 차단). 재시도 60초.
- goal hook("전부 추천대로 계속 진행")은 세션 종료 시 **해제됨** — 다음 세션은 표준 확인
  흐름(추천안 패키지 → 사용자 확인)으로 복귀.

## 6. 미해결 질문 / 사용자 확인 필요

1. **다음 배치**(d-20 대기) — 추천 **T2-D/F/G 통합 저위험 청소**(X-A 선례 형식·대표 실물
   확인 완료 — PROCESS d-20 기록이 출발점) vs T2-A 중복 제거 vs T2-B 분해(파일별) vs
   e2e·실기(사용자 준비 필요). **병합 대기분 없음**(전 브랜치 15cd460 동기 — 병합 질문 불요).
2. **제품 결정 3건**(급하지 않음 — §3.4-4).
3. **e2e 파일럿·QA-W1 실기** — 사용자 준비(§7) 후.
4. R5#12 역방향 검사(미참조 로케일 키 CI 검출) 도입 여부 — T2-I 계약 §4 미결 기록.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64), bun 1.3.14. cargo PATH 밖 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` |
| 실행 | dev = `bun run tauri dev`(사용자만). 빌드 = `bun run tauri build` |
| 검증 | `bun run verify`(typecheck→lint→format→bun test→cargo fmt/clippy/test) + `bunx vite build`. bindings 재생성 = cargo test |
| Phase 8 | secrets 5건 등록·release.yml 준비 완료(트리거 tags `v*`+수동만). Team ID `SN98P5V7J4` |
| e2e 준비(사용자) | ① `bun run tauri dev` ② 설정 REMOTE 비밀번호·활성화 ③ `export TAIDE_E2E_PASSWORD='<pw>'` ④ `bun run e2e` |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` |

## 8. 다음 세션 TODO (우선순위 순)

1. **다음 배치 확인**(§6-1) — 추천 T2-D/F/G 통합 청소. 착수 시 PROCESS d-20 의 대표 실물
   확인 기록이 출발점 — 계약(관련: role='button' 6파일·`src/entities/app/`·
   `src/entities/editor/open-with-registry.ts` 'preview'·`shared/constants/terminal.ts:9`
   DEFAULT_FONT_SIZE 재실사·감사 §5 T2-D/F/G 행) → 구현 Workflow(항목별 실사 재량·기처리
   4건 제외) → 4렌즈 → 적대적 → 수정 → 메인 2차 → 커밋 → 병합 확인.
2. **T2-A 중복 제거·T2-B 분해** — 이후 순차(T2-B 는 파일별 단독 배치 — editor-pane 선례).
3. **T2-E/J AppError 캠페인** — 별도 대형(감사 §6.3).
4. **e2e·QA-W1 실기** — 사용자 준비 후. qa6 이번 세션 +3절 포함 소화.
5. **이월 소화**(§3.4-3) — 적절한 배치에 편입(특히 D8 AppState 사각지대·git2 in-process 13건).
6. **Phase 8 본착수**(d 통과 후).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | **상단 "잔여 작업 총괄"(잔량 정본)** + 체크리스트(d-17~d-20 이번 세션) |
| `docs/acknowledge/2026-08-19-audit-t1i-domain-boundary-contract.md` | T1-I 계약(§4 구현·§5 검토 반영·기각 D 계열) |
| `docs/acknowledge/2026-08-19-audit-t1h-lock-io-contract.md` | T1-H 계약(§4 구현·§5 검토 반영·보류·이월) |
| `docs/acknowledge/2026-08-19-t2i-locale-externalization-contract.md` | T2-I 계약(§4 구현 — 제거 32키 목록·보수 유지 77키 분해·§5 검토 반영) |
| `docs/quality-assurance/2026-08-18-architecture-audit.md` | **감사 정본**(297발견 — T0·T1 전체 완결, T2 는 §6.3·§5 잔여) |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | 실기 QA 마스터(이번 세션 +3절: T1-I·T1-H·T2-I) |
| `docs/ipc-contract.md` | IPC 정본 — 커맨드 176+raw3·이벤트 23·원격 기본 거부·locale 카탈로그 경로(신규) |
| `docs/architecture.md` | 구조 정본 — §2 도메인 경계 판정 기준(T1-I 명문화)·§3 capability 실현 시그니처 |
| `src-tauri/tests/domain_boundaries.rs` | 도메인 경계 기계 강제 — 화이트리스트+사유 doc(경계 예외의 실질 정본) |
| 직전 세션 계약들(2026-08-18-\*·2026-08-19 나머지) | 기결 참조용 |

## 10. 복기 신뢰도

- **높음**: 이번 세션 커밋 9건이 dev·main 동기 고정. 계약 3건이 배치별 실시간 동기(§4/§5).
  각 배치 메인 2차 실물 재검증 수행. 기준선 수치 세션 말 verify 로그 직접 확인(Rust 1080·
  프론트 1375).
- **중간**: 검토 원문 전문(T1-I 17건·T1-H 29건·T2-I 15건)은 스크래치패드(세션 소멸) —
  confirmed·실질 수정·기각 요지는 각 계약 §5 에 반영, 나머지 minor 재론 시 실코드 확인.
- **낮음**: 없음.
