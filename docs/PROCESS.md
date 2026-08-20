# PROCESS — TAIDE 작업 상태

> 기준 문서: `~/.claude/convention/*.md`(전 컨벤션), `docs/acknowledge/`(결정), 이 문서(체크리스트).
> 구현 순서 정본은 `docs/roadmap.md`, 버전 정본은 `docs/tech-stack.md`, API 정본은 `docs/research/*.md`.

> 과거 기록(문서화·Phase 0~7.10 W1~W7)은 `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 아카이브됨 (2026-08-14).
> QA6 후속·기능 확장 1~3차(2026-08-12~14)는 `docs/history/2026-08-16-process-archive-qa6-feature-waves.md` 로 아카이브됨 (2026-08-16).

## 잔여 작업 총괄 (2026-08-19 실측 — 다음 착수 판단의 단일 뷰)

> 기능(PRD FR-A~J)은 캠페인 A~I 로 전량 구현 완료. 남은 것은 감사 잔여·QA 실기·Phase 8.
> 상세 정본: 감사 = `quality-assurance/2026-08-18-architecture-audit.md`, T1 3차 이월 =
> `acknowledge/2026-08-19-audit-t1-batch3-contract.md` §5.1, 실기 = qa6-checklist.

### 감사 297발견 처리 현황

| 트랙 | 상태 |
|------|------|
| T0 24항목 | **완료**(#15 는 T1-2차 재등록으로 해소) |
| T1 11묶음 중 8 | **완료**: E·J·B(1차)·G·asset·F(2차)·C·D·A(3차)·editor-pane 이월(d-13) |
| **T1-K 원격 기본거부** (6건) | **완료(d-14)** — C16 근본 해소(기본 거부 기계 강제) |
| **T1-H 락 IO 분리** (10건) | **완료(d-18)** — 락-IO 결합 국소 해소(입도 무변경). 계약 `2026-08-19-audit-t1h-lock-io-contract.md`. 이월: git2 in-process 동기 IO 13건·repo 단위 재진입 직렬화·원격 동시 상한·pty_spawn·git_pull 네트워크 락 탈출(계약 §4·§5) |
| **T1-I 도메인 경계 C13** (12건) | **완료(d-17)** — 결정 10-A·12-A 실행(capability 확장점·아키텍처 테스트 기계 강제). 이월: AppState 공유 필드 사각지대·layout↔ide·window↔layout 순환(계약 §5) |
| T2 백로그 10묶음 (~107건) | **T2-I 완료(d-19)** — 로케일 외부화(4081→1242·미참조 32키 제거). 잔여: 중복 제거·비대 파일 분해(settings-view 927·lib.rs 등)·shared/lib 재구조화·접근성 8·dead code·매직넘버 |
| T2-E AppError 369지점 | 미착수 — **별도 캠페인**(T2-J 병합, 한/영 혼재 UX) |
| X-A 배선 8건 | **완료(d-15)** — 살리기 4·지우기(focus-kind·app:ready·중복 5종)·X1#2 소비 전량. 신규 이월 4건(viewState closed_tabs 복원 공백·useReplaceSearch invalidation 부재·REMOTE 폴링/push 중복·경로 predicate 한계)은 d-15 계약 §6 |
| X-C 문서 잔여 | 일부 해소(커맨드 수·data-model roots 는 정정됨) — X1#5(sync gist 스키마 절)·#15(디스크 레이아웃)·#16·#17·#18 잔여 |
| 미배정 minor·압축 제외 | ~40건(감사 원문에만 존재 — 재론 시 실코드 확인) |

### T1 3차 §5.1 이월 잔여 (8건 중 6.5건)

1. ~~editor-pane 묶음~~ **완료(d-13)** 2. open-with-registry 생명주기(LRU 는 임시) 3. ws.rs
writer 무한 대기(무트래픽 프루닝 지연) 4. Rust 재핸드셰이크 실패-확인 커맨드 5. throwaway client
race(하네스 재현 불가 — 보류) 6. ide_publish_diagnostics/notify_at_mention 원격 게이트(제품 결정
— T1-K 범위 외 명시) 7. TREE_ROWS 센티널 → Option<u32> 8. PROJECT_SCOPED_KEYS 경로 키 미커버
(실피해 낮음) 9. agent_hooks_uninstall User 스코프 원격 허용의 install/uninstall 대칭화 여부
(T1-K 검토 노출 — 6번과 함께 제품 결정) 10. 키링 게이팅 비대칭 R3#7(AI 토큰 3·GitHub PAT 원격
변경 — 동일 결정 묶음)

### QA·배포 트랙

- **e2e 파일럿 실행**(d-6): 하네스·12스펙 준비 완료 — **사용자 준비 필요**(앱 기동·REMOTE 설정·
  TAIDE_E2E_PASSWORD·`bun run e2e`)
- **실기 QA 누적**(qa6-checklist): 캠페인 A~I 재검 + 손 QA 재검 + T0/T1 1~3차 재검 + editor-pane
  재검 7항목 + **asset:// webview 라운드트립(최우선)** + QA-W1 멀티윈도우 40항목(사람 실기 주축)
- QA-W2~W7 웨이브(pro-qa-design §5)·Wave I deferred 4건 — 미착수
- **Phase 8**: secrets 5건 등록 완료·release.yml 이식 완료(e-0·e-1) — 본착수(태그 릴리스 실행·
  공증 실검증)는 전문 QA(d) 통과 후

## 진행 중: 실기 QA → 잔여 기능(P0+P1) → 전문 QA → Phase 8 (2026-08-14, 새 세션)

> 계약: `docs/acknowledge/2026-08-14-remaining-features-pro-qa-plan.md` (범위·순서·역할 배정 정본)
> 품질 원칙(2026-08-14 추가 지시): **효율보다 완벽** — 역할 전면 상향(구현 sonnet+xhigh·렌즈
> opus+xhigh·적대적 opus+high·정찰 opus+high), 웨이브 검토 4렌즈(+설계·추상화) 상설. 계약 §3.1.

- [x] a. e2e 실행 경로 리서치 완료 (wf_736692ca-b95, opus+medium 3축) — 정본
      `docs/research/2026-08-14-e2e-path-research.md`. 결론: A=remote 미러+Playwright(webkit·node
      실행) 1차 축(커버리지 50~60%, 앱 무변경, password_only 로그인으로 자동 인증) /
      B=embedded WebDriver(tauri-plugin-wdio-webdriver, 공식 macOS 경로·debug 전용) 파일럿 후
      보조 / tauri-driver 단독·CrabNebula(유료)·puppeteer(WebKit 없음) 배제. bun 런타임 e2e
      통합은 전제하지 않음(Playwright 공식 not planned). 핵심 주장 메인 실물 재검증 완료.
      채택·의존성 승인은 전문 QA 설계 시점(파일럿과 함께)
- [x] b. ~~실기 QA 일괄~~ **취소** (2026-08-14 사용자 지시) — "이미 구현된 건 간단하게 구동해봤다"
      가정(스모크 수준 치명 결함 없음)으로 대체. qa6-checklist 미체크 항목 전량은 전문 QA(d)로
      이월 — e2e·심층 검토로 검증. **실기 미검증 상태 자체는 유지됨**(KNOWN ISSUE 존속)
- [x] c. 잔여 기능 구현 — 갭 P0 잔여 9건 + Remote 하드닝 9 + Hot Exit 미세 3 + P1 10건.
      **웨이브 A→I 확정**(계약 §2-2): A LSP 인텔리전스 → B 하드닝 → C Git → D 탐색·검색 →
      E 터미널·태스크 → F 에디터 표현 → G AI → H 키맵 엔진 → I 셸·워크스페이스.
      **전량 완료(9/9, 2026-08-16)** — prod(main)=b4e7318 반영(2026-08-18 문서 커밋 포함
      fast-forward). 상위 체크박스 미체크는 표기 누락이었음(2026-08-18 정정 — 하위 로그·계약·
      HANDOFF 는 완료로 정합)
    - [x] c-A. Wave A — LSP 인텔리전스. 정찰 완료(wf_0d6ea4d6-7b4, 529 재시도 4차 만에 성공,
          하중 주장 8건 메인 재검증) + 계약 확정(`2026-08-14-wave-a-lsp-intelligence-contract.md`,
          결정 4건 전부 추천안). **중대 발견**: 직전 세션 P0-0(LSP capabilities 확충)은 dead code
          적용이라 런타임 무효 / 서버→클라 요청 전면 폐기(configuration 무응답 기존 결함) /
          cross-file F12·Peek 이 원래부터 무음 실패(registerEditorOpener 부재).
          구현 완료(wf_7238f3c1-544, 6에이전트 — D 가 병렬 축 openIssues 6건 수정 포함).
          검토 완료(wf_ecdeda9d-40f, 28에이전트): 4렌즈 발견 42건 → critical/major 24건 적대적
          검증 → **확정 20·반증 4**. 확정 전건 + minor 12건 수정, 기각 5건(사유 기록 — L1-8
          코스메틱 하이라이트·L3-7~10 구조 제안은 후속 분리). 주요 확정 결함: peek orphan 모델이
          cross-file 편집을 디스크 미기록으로 삼킴(critical·적용기 분기 수정+저장 동기화) /
          프리로드 TTL dispose 가 입양된 탭 모델 파괴(critical·타이머 해제) / 백그라운드 탭 편집
          디스크 덮어쓰기(model-dirty-tracker 신설) / file_open 루트 가드 부재(Rust 수정) /
          applyEdit 핸들러 프로젝트 간 침범(세션 root 스코프) / ra CodeLens experimental.commands
          미선언 / targetSelectionRange 유실 / waitForLspSession 행(tier 게이트) / failureHandling
          선언 불일치(abort 로 정정) / 진단 사이드 맵 uri 정규화 / executeCommand 미대기.
          메인 2차: 수정 7건 실물 재확인 + verify 전체·vite build·bindings diff 검수 그린. 커밋
    - [x] c-B. Wave B — 하드닝 마감. 정찰 완료(wf_e628c338-e87, opus+high 2축)에서 "minor" 를
          넘는 **신규 중대 4건** 발견·메인 재검증: gist 인바운드 shell_override 미필터(RCE급)·
          게이트 필드 미필터·저장 왕복 타이핑 소실·cross-file 편집 hot-exit 미러 구멍(Wave A 회귀).
          계약 확정(`2026-08-15-wave-b-hardening-contract.md`, 결정 3건 전부 추천안 — 범위=신규 4+잔여
          10, Remote 보안 패키지, Hot Exit 패키지). 구현(wf_9ddbde2a-6d0 S→B1∥B2) 완료 →
          검토(wf_8e6238d4-bd2, 4렌즈+적대): 확정 3(stale nonce 오라클·저장 타이핑 미러 baseline
          stale·비밀번호 무피드백) + minor 6 수정, #2(잠금 축 선택)는 L2-0 수정으로 해소 확인(메인
          재판정). Host 허용목록 터널 회귀(L0-0) → 사용자 결정 UI+링크 완결(계약 §6) →
          후속 구현·검토(wf_37dd3b24-7a4): 포트 완화(loopback 엄격·등록 호스트 포트 무관)·링크
          호스트 반영·편집 위젯 + minor 6 수정(L0-0 @-userinfo 방어심층·L0-1 remote_issue_link
          원격 거부). 메인 2차: format 정리·스팟 검증·verify 전체(678/664+6+17)·vite build 그린. 커밋
    - [x] c-C. Wave C — Git 확장(3-way 머지·줄 stage·커밋 상세·파일 히스토리·revert/tag·원격
          checkout·파일 blame 뷰). 정찰(wf_563a88b2-20f) 축1 Rust·축3 API 완료(git 커맨드 27종·
          git2+CLI 하이브리드 확인), 축2 프론트 UI 는 StructuredOutput 초과 실패 → resume 재실행 중.
          정찰 3축 완료(축2 resume 성공) → 계약 확정(`2026-08-15-wave-c-git-contract.md`, 결정 4건).
          구현(wf_3d5d4450-95b, 백엔드 13커맨드+프론트 3): git_apply anchor 버그 실패테스트 재현·수정.
          검토(wf_aa0a4674-768, 4렌즈): 발견 18 → L1-0 에디터 unstage 좌표 불일치(confirmed, unstage
          제거)·L1-1 미추적 파일 stage 실패·minor 다수 수정. L0-0 revert 혼입은 검증자 standalone
          재현으로 반증(git2 SAFE checkout guard). **보안 렌즈 critical(git_resolve_conflict 경로
          트래버설 — to_repo_relative 가 상대경로 `..` 무검증)은 검증 실패로 fixer 누락 → 메인 직접
          수정**(Component::ParentDir/RootDir/Prefix 거부 + 테스트, 모든 git write 커맨드 공유 방어).
          메인 2차: verify 전체(729/690)·vite build 그린. 커밋
          - 후속 기록: 신규 파괴 커맨드 원격 dispatch 허용은 기존 27종 정책 유지(pty_spawn 동급·
            checkout safe)·문서화만. 계약 문언 갱신 필요분은 `2026-08-15-wave-c-review-fix-decisions.md`
    - [x] c-D. Wave D — 탐색·검색(팔레트 @/: 모드·Workspace Symbol ⌘T·Breadcrumbs·Search Editor·
          검색 히스토리·gitignore 토글). 정찰(wf_33479e2a-2e3) 축1 검색·축2 breadcrumbs/LSP 완료,
          정찰 3축 완료(축0 resume 성공) → 계약 확정(`2026-08-15-wave-d-search-nav-contract.md`,
          결정 4건 전부 추천: 팔레트 VS Code 규약(@문서/#Workspace Symbol ⌘T/:줄)+monaco 병존·
          Search Editor 신규 TabKind·gitignore=`ignore` 크레이트 도입(신규 의존성 승인)·capability
          확충+히스토리 Settings). 구현 A(백엔드 Rust 단독)→B(프론트 병렬 3) 완료.
          **신규 승인 의존성: `ignore` 크레이트**(ripgrep gitignore walker)
          검토(4렌즈) 확정 2(L1-0 검색 취소 스토어가 `ProjectId` 단일 키라 패널·Search Editor
          동시 검색이 서로 조용히 절단·L3-0 검색 실행 오케스트레이션 3중 중복 — 검증에서 major→
          minor 강등) + minor 9건(우발 판정 포함) → 전건 처리:
          - L1-0: `SearchStore` 를 `ProjectId` 대신 caller 세션 id(패널=`useId()`, Search Editor
            탭=`tabId`) 로 키 교체. `search_run`/`search_cancel` 에 `session_id` 파라미터 추가,
            같은 세션의 재검색만 이전 실행을 취소. 완료 시 자기 항목만 정리(`Arc::ptr_eq`)해 스토어
            무한 증식 방지. bindings 재생성(cargo test) 확인.
          - L3-0(강등 후 함께 처리): `entities/search/use-search-run.ts` 신설 — 스트리밍 검색
            오케스트레이션(collected→group→total→toast→isSearching) 을 단일 훅으로 통합, 패널·
            에디터·초기 자동검색 3곳 중복 제거. 세대(generation) ref 로 구식 실행의 콜백이 최신
            실행 상태를 덮어쓰는 경쟁(L1-2) 도 함께 해소.
          - minor: L1-1 workspace-symbol file 스킴 가드 추가(definition.ts 선례) · L1-3
            addRecentSearchTerm 동일 최상단 재검색 시 참조 보존(불필요 재동기화 방지) · L1-4
            인접 매치 컨텍스트 줄 중복 렌더 제거(`dedupeAdjacentContext`) · L2-0 gitignore 상위
            디렉토리 읽기(미적용) 관련 부정확한 doc comment 정정 · L2-1 `recent_searches` 상한을
            서버(`sanitize`)에도 강제(gist 다운로드 경로 방어, sync 포함 정책은 유지) · L0-0/L3-2
            `search.excludeGlobPlaceholder` locale 키 3개 언어 추가(B3 결정기록 제안 문구) ·
            L3-3 `toggleInSet`/`fileNameOf` 를 Wave D 가 건드린 파일 한정으로 `shared/lib` 승격.
            기각: L0-1 심볼릭링크 미추적 주장은 `DirEntry::metadata()` 가 symlink_metadata 와
            동일(심링크 비추적)함을 실측 확인 — Wave D 이전에도 동일 동작이라 회귀 아님. L0-3
            문서 5종 갱신은 계약 §3.6 Phase C 몫으로 명시 유예된 항목이라 이번 결함수정 범위 밖으로
            판단(별도 보류). L0-2 git 도메인 rustfmt 재포맷은 `cargo fmt --check` 로 현재 툴체인
            기준 필수임을 확인 — 되돌리면 fmt 게이트 실패.
          검증: `bun run verify` 전체 그린(tsc·eslint 0 error·prettier·bun test 809/809·
          `cargo fmt --check`·`cargo clippy -D warnings`·`cargo test --workspace` 733/733).
          메인 2차: 스팟 검증(세션 취소·file 스킴 가드·상한) 정합, search-editor-pane 초기검색
          useEffect 의 신규 exhaustive-deps 경고를 didAutoRunRef 가드+run deps 로 정리(eslint-disable
          우회 없이, 경고 6건 기존 관행 수준 복귀). Wave C fmt 누락(to_repo_relative)도 이번에 정리. 커밋
    - **문서 부채 정리 완료(2026-08-16, wf_de5dfea2-788)** — ipc-contract·data-model·qa6 를 Wave
      A~I 실코드 기준 일괄 갱신(문서 소유 분리 병렬 3에이전트). **ipc-contract**: 커버리지 72%→100%
      (bindings 178커맨드 전수·이벤트 25종 — 누락 49건 추가, 잔재 6건·시그니처 4건·이벤트 잔재 3건
      정정, 원격 dispatch 정책 통합 섹션 신설). **data-model**: 설정 리네임(ai_provider/ai_model)·
      TabKind 9종·레이아웃 v2 마이그레이션·스니펫/프롬프트/SearchQuery 영속 스키마(§9)·비영속 IPC
      타입(§10)·SettingsChanged(§11) 추가. **qa6**: Wave B/C/D 섹션 신설(누락돼 있던 것)+E~I 보강,
      체크박스 275건(무삭제 순증 149줄, Wave I 멀티윈도우 40항목+deferred 4). 메인 스팟 검증 완료.
      발견 부산물(코드 정본이라 문서만 기록): app:ready 죽은 이벤트 배선·features/agent-integration.md
      의 release_wait_marker 잔재(별도 문서 세션 필요)·ipc-contract snippet 절 _Serialize 분할 미반영.
    - **PROCESS.md 아카이브(2026-08-16)** — QA6 후속·기능 확장 1~3차(완료분)를
      `docs/history/2026-08-16-process-archive-qa6-feature-waves.md` 로 이관(589→471줄).
    - [x] c-E. Wave E — 터미널·태스크(OSC133 셸 통합·태스크 러너·Run Selected Text). 정찰
          (wf_a5ff7069-171) 축0 터미널·축1 태스크 완료, 축2 OSC133 웹 리서치 2회 실패 → 기존
          `docs/research/xterm-pty.md §8`(셸 rc 주입 설계 완비) + xterm 6.0 API 실측으로 대체.
          계약 확정(`2026-08-15-wave-e-terminal-tasks-contract.md`, 결정 3건 추천: OSC133 자동 주입
          +opt-out·순수 133/⌘↑↓·태스크 Rust 감지+pty 실행(toml 크레이트 회피, 고정 명령 세트)·
          Run Selected 포커스 터미널+현재 줄 폴백). 구현(wf_035e2cd5-46b) — fish 실설치 검증 후
          네이티브 OSC133 지원 확인해 주입 생략·posix_quote injection 방어·zsh/bash 실행으로 rc
          검증. 검토(wf_0510f5df-39e, 4렌즈): **major 확정 4건**(zsh 주입이 .zshenv/.zprofile 유실 →
          PATH/env(brew) 손실, 검토가 실제 zsh 로 재현 — 3중복 + posix_quote fish 백슬래시 탈출) +
          minor 다수 수정. fixer 가 VS Code 방식 .zshenv/.zprofile 패스스루 추가·백슬래시 이중
          이스케이프·Makefile ::= 가드·OSC133 블록 인덱스/무한누적 상한·키맵 카테고리 수정.
          fish 4.0 미만·bash 3.2 PS0 미지원은 의도적 보류(계약 §4·§6 명시). 메인 2차: zsh 패스스루·
          posix_quote 수정 실물 확인, verify 전체(856/751+6+17) 그린. 커밋
    - [x] c-F. Wave F — 에디터 표현(Semantic Tokens·사용자 스니펫·Format on Type/Paste·Emmet).
          착수 승인(2026-08-15, 3건 전부 추천안 — Wave E prod 병합 완료(`d525640` fast-forward)·
          다음=F·recent_searches 동기화 유지: `2026-08-15-wave-f-kickoff-decisions.md`).
          - [x] 정찰 완료 (wf_12eece29-284, opus+high 4축, 805k 토큰) — 하중 주장 메인 재검증 완료
                (monaco 소스 5곳 직독 + 웹 3건 fetch). **반증 1건**: gopls 는 initializationOptions 를
                `options.Set` 으로 파싱(설정 맵 불필요). 핵심 확정: semantic 워시아웃 경로·
                'semanticHighlighting.enabled' 필수·formatOnPaste 의 rangeFormatting 게이트·monaco 스니펫
                completion 부재(파서·변수는 완비)·emmet-monaco-es tokenizer 'standard' 필수
          - [x] 계약 확정 (`2026-08-15-wave-f-editor-presentation-contract.md`, 결정 4건 — ①풀 패키지+
                emmet-monaco-es 5.7.0 승인 ②Semantic **delta 포함**(추천안+확장) ③포매팅 둘 다+어댑터
                2종 ④스니펫 추천 패키지). **신규 승인 의존성: emmet-monaco-es(+전이 emmet)**
          - [x] 구현 완료 (wf_583cce34-8a3, 6에이전트: S Rust 스파인∥B0 LSP 코어 → C1 semantic
                재인코딩·delta∥C2 포매팅 어댑터 2종∥C3 스니펫·Emmet → D 통합·문서). 전체 verify
                체인 그린(프론트 928·Rust 765+6+17·vite build). D 가 선행 openIssues 3건 근본 수정
                (CodeEditorProps required 통일·AppDataPathKind::Snippets·snippetsManage locale).
                검토 이월 항목 10건 명시(ra 별칭 소스 미대조·확장자 화이트리스트·prefetch 추가 등)
          - [x] 검토 완료(wf_97ea3033-c77, 15에이전트): 4렌즈 발견 33건 → critical/major 10건 적대적
                검증 **전건 confirmed**(3건 minor 강등) → 4개 근본 클러스터 + minor 전건 수정.
                **critical: semantic 테마 rule bare scope 가 monaco 트라이 last-wins 덮어쓰기로 번들
                테마 11종 실색 변화** → `taideSemantic.<token>` 네임스페이스로 재설계. major: prefetch
                옵저버 부재 gcTime GC(→QueryObserver 구독)·Windows 드라이브 경로 탈출(':' 금지)·설정
                토글 미반영(캐시 구독→refresh). 상세: 계약 §6. 보류 1건(emmet jsx/tsx/heex 트리거 —
                문서화). 메인 2차: 수정 7건 실물 재검증 + verify 전체 exit 0(937/766+6+17) + vite
                build exit 0. 커밋
    - [x] c-G. Wave G — AI(Inline Edit ⌘I·AI 커밋 메시지). Wave F prod 병합 완료(56712b0 fast-forward).
          - [x] 정찰 완료(wf_8904a89d-424, opus+high 4축) — 하중 7건 메인 재검증 전건 확정. **갭 문서
                "git_diff 재사용·난이도 하" 전제 오류 판명**(통합 diff 커맨드 부재·provider chat 진입점
                부재·⌘K=monaco chord 21건 1단계·프롬프트 오버라이드 무경고 파손·ViewZone 선례 0)
          - [x] 계약 확정(`2026-08-16-wave-g-ai-contract.md`, 결정 4건 — F 병합·공통기반+**설정 리네임**
                (ai_provider/ai_model, serde alias)·Inline Edit 추천(⌘I·모델 무변경 프리뷰·일괄 응답)·
                커밋 메시지 추천(git_diff_staged_text git2 native·log 20건·Sparkles))
          - [x] 구현(S Rust 단독 → B1 Inline Edit∥B2 커밋 메시지 UI → D 통합) 완료. S: provider
                trait `instruct` 추가(complete 무변경)·프롬프트 2파일 신설+로더 공통화·
                `AiInlineStore`→`AiRequestStore`+`ai_inline_cancel`→`ai_request_cancel` 리네임·
                신규 커맨드 3종(`ai_inline_edit`/`ai_commit_message`/`git_diff_staged_text`)·설정
                리네임(`ai_auto_tab_provider/model`→`ai_provider/ai_model`, `ai_auto_tab_enabled`
                유지) — **설계 이탈**: 계약이 지시한 `#[serde(alias)]` 는 이 프로젝트의 specta
                고정 버전에서 `Settings`/`SettingsPatch` 를 `_Serialize`/`_Deserialize` 유니온으로
                쪼개 무관 필드까지 타입을 깨뜨림을 실물 검증으로 확인 → raw JSON 사전 마이그레이션
                (`migrate_legacy_ai_provider_keys`)으로 대체, 하위호환 기능은 동일 보장(근거:
                `data-model.md` §7). B1: ⌘I 모나코 액션+ContentWidget 입력+ViewZone 프리뷰(레포
                최초 도입)+모델 무변경 불변식+undo 1스텝. B2: SCM 패널 Sparkles 버튼+diff/log 조립+
                응답 후처리, 설정 리네임 프론트 소비처 갱신. 병렬 산출물 접합부 결함(팔레트 미배선·
                구 필드명/커맨드명 잔존 10건·중복 취소 래퍼)은 D 가 처리.
          - [x] Phase D 통합 완료 — `AI_COMMANDS` 를 `app/bootstrap-commands.ts` 에 배선(팔레트
                노출), `cancelAiInline`→`cancelAiRequest` 개명 정리 + `entities/ai/ai-inline-edit.ipc.ts`
                의 중복 취소 래퍼 제거(단일 소스는 `entities/ai/ai.ipc.ts`), B2 가 미배선 남긴
                locale 키 2종(`git.generateCommitMessageFailed`/`noStagedChangesForCommitMessage`)
                을 커밋 메시지 실패 토스트·Sparkles 버튼 비활성 툴팁에 근본 배선(死locale 키 제거).
                키맵 카탈로그 미노출은 `git.toggleBlame` 등 기존 커스텀 모나코 액션과 동일한
                구조적 한계로 확인(Wave G 신규 결함 아님, Wave H chord 엔진까지 보류). 프리뷰
                모델-무변경 불변식 코드 추적 검수 통과(수락 전 `executeEdits` 호출 없음). 문서:
                `features/ai.md` 신설·`features/git.md`(`git_diff_staged_text`)·`ipc-contract.md`
                (신규 `ai` 도메인 섹션+`git_diff_staged_text`)·`data-model.md` §7(설정 리네임
                메커니즘)·`quality-assurance/2026-08-11-qa6-checklist.md`(Wave G 실기 21항목)·
                `research/2026-08-13-vscode-cursor-gap.md` §5·§8(AI 커밋 메시지·Inline Edit 종결
                표기) 갱신. 검증: `bun run verify` 전체 그린(tsc·eslint 0 error·prettier·
                bun test 966/966·`cargo fmt --check`·`cargo clippy -D warnings`·
                `cargo test --workspace` 796+6+17/819) + `vite build` exit 0. 4렌즈·적대적 검증·
                메인 2차·커밋은 후속(Phase E)
          - [x] Phase E 검토 완료(wf_1f2bcdcf-0bb, 16에이전트) — 4렌즈 52건(major 11 → 중복 제거
                **5 클러스터 전건 confirmed**·minor 41) → 수정 50/기각 1/보류 1. 클러스터:
                instruct 256 토큰 캡 상속(→전용 4096+length 감지 에러)·팔레트 무동작(precondition
                이 run() 게이트 → keybindingContext 이전)·펜스 스트립 앵커 실패(→스캔 관대 파서
                +공용화)·truncated/skipped 모델 미전달(→diffText 본문 안내+**시크릿 파일 제외**)·
                제출 시 포커스 이탈(→상태별 재포커스). 보류 1: staged 0건 버튼 비활성(계약 명문
                유지 — 사용자 재확인 여지, features/ai.md §9). 상세: 계약 §6. 메인 2차: 수정 6건
                실물 재검증+반환 타입 1건 직접 수정+verify 전체 exit 0(972/817+6+17)+vite build 0.
                플레이키 1건(dirty 미러 — Wave G 무관·단독 5회 통과) 기록. 커밋
    - [x] c-H. Wave H — 키맵 엔진(chord ⌘K ⌘S·when 컨텍스트·shift+기호 캡처 — 전면 개정).
          - [x] 정찰 완료(wf_21fd174b-f0d, opus+high 4축) — 하중 전건 메인 재검증. **핵심 발견**:
                shift+기호는 이미 code 경로 동작(갭 §7 stale)·window capture 가 monaco 선행(⌘K
                점유 시 chord 21건 전멸)·**기존 회귀 4건**(APP_KEYMAP ⌘B/F/=/- 가 monaco chord 2단
                삼킴)·monaco chord 5000ms 타임아웃+status no-op·when dead field·terminal-pane
                오버라이드 미적용·ContextKeyExpr 번들 실존
          - [x] 계약 확정(`2026-08-16-wave-h-keymap-contract.md`, 결정 4건 — G prod 병합 완료
                (c3e60c4)·Sparkles unstaged 폴백 채택(G 보류 해소)·범위 재정의+결함 상환·chord/when
                추천 패키지+**마이그레이션 보수**(행동 변화 0))
          - [x] 구현(A Rust 소규모 → B 키맵 코어 단독 → C1 캡처 UI∥C2 소비처·카탈로그 → D 통합)
                완료. D 에서 접합부 정리 중 **핵심 회귀 재발견·수정**: N개 `useGlobalKeymap`
                리스너가 같은 물리 keydown 을 전부 처리하는 구조라(`stopPropagation` 은 형제
                리스너를 막지 못함), chord/monaco유예 진입 판정이 리스너별로 반복되며 첫 리스너의
                mutation 을 나머지가 "다음 keydown" 으로 오판 → 마이크로태스크로 유예/대기가
                진짜 두 번째 keydown 도착 전에 풀려 monaco chord 4건·⌘K⌘S 자체가 동작하지 않는
                상태였음(유닛 테스트로 결정적 재현). `getKeymapChordDispatchSnapshot(event)`(이벤트
                참조 동등성으로 메모이즈)로 근본 수정 — 회귀 테스트 고정(`keymap-chord-store.test.ts`).
                그 외: monaco 소스 행 chord 재바인딩이 표시만 되고 실제 미동작하던 인코딩 누락 수정
                (`buildMonacoChordKeybinding`)·`findKeymapConflict` chord 2단 오탐 수정·
                `handleChangeBinding` 미배선(새 chord 후보 누락) 수정·`keybindings.open` 팔레트
                중복 행 수정·locale 키 부정확(`noStagedChangesForCommitMessage`→
                `noChangesForCommitMessage`, staged+unstaged 통합 조건과 정합) 수정. terminal-pane
                의 `isFocused` 게이팅은 계약 문구("제거")와 달리 **의도적으로 유지**(스플릿 다중
                터미널 인스턴스 오발동 회귀 방지 — "기존 21 엔트리 행동 변화 0" 완료조건 우선,
                근거는 `features/keymap.md` §7). 문서 신설(`features/keymap.md`) + 기존 문서 정정
                (`ai.md`·`git.md`·`ipc-contract.md`·갭분석 §7·qa6 Wave H 실기 항목). `bun run
                verify`(typecheck·lint·format·test 1060/1060·rust fmt/clippy/test 819/819)+vite
                build 전부 통과(LSP PATH 테스트 1회 flaky 재확인 — 격리 재실행 통과, Wave H 무관)
                → 4렌즈 → 적대적 → 수정 → 메인 2차 → 커밋(Phase E — wf_8f13dc1e-79c).
          - [x] 4렌즈+적대적 검증 완료 → major 확정 4(중복 제거, 실제로는 5개 결함 클러스터를
                중복 보고) + minor 확정 다수. **수정 완료(sonnet+xhigh)**:
                (1) `taide.*` 커스텀 monaco 액션 카탈로그(§3.3)가 팔레트 영구 비활성+재바인딩
                무동작이던 근본 원인(`editor.addAction` 이 `id` 를 `${editorId}:${id}` 로만
                등록) 수정 — `editor-area.tsx` 의 활성 액션 id 정규화 + `monaco-action-commands.ts`
                의 `registerTaideCustomActionCommands()`(전역 커맨드 신규 등록). (2) command-palette
                의 독립 window 리스너가 chord `pending`/`monacoDeferral` 을 무시하고 우회하던 결함
                수정(스냅샷 가드 + `findRunnableCommandBinding` 의 chord 행 제외). (3) 컨텍스트
                인스펙터가 Radix 모달 포커스 트랩으로 항상 비어 있던 구조적 결함 수정(다이얼로그
                닫혀 있을 때만 폴링 → 열기 직전 스냅샷을 얼려 표시). (4) 표본 chord 를 계약 문언대로
                `⌘K ⌘S`(2단도 `mod` 필요)로 정정. (5) 터미널 포커스 시 ⌘K 가 터미널 입력을 삼키던
                신규 회귀를 `when: '!terminalFocus'` 로 해소. 그 외 minor: `keybindings.open`→
                `open-keybindings-editor` 리네임에 대한 오버라이드 레거시 별칭 마이그레이션·
                monaco 유예 창이 앱 내부 포커스 이동으로는 안 풀리던 문제(focusin 리스너)·비-⌘K
                monaco chord 프리픽스 2단이 앱 단일 키에 뺏기던 문제(`deriveMonacoChordPrefixes`
                동적 프리픽스)·키 자동반복/IME 조합 중 keydown 이 chord 대기·유예를 스스로 소비하던
                문제·상태바 인디케이터가 monaco 유예 창에서는 안 뜨던 문제·chord 2단에 Enter 를
                쓸 수 없던 캡처 UI 결함·Sparkles 폴백의 untracked 하위 디렉토리 누락(`recurse_untracked_dirs`)
                +시크릿 확장자 목록 보강(jks·ppk·der·crt·keystore·.netrc·.npmrc)+충돌 전용 상태
                버튼 비활성 불일치·chord 스토어 notify 낭비(idle 재알림)·이벤트 참조 누수. 반려
                (사유 기록): taide.* 신규 locale 키 미등록(기존에도 monaco 액션 158건 중 42건만
                번역되는 확립된 패턴, 7건 추가는 비일관성만 키움)·팔레트 중복 행 통합(어느 쪽을
                남길지는 제품 판단 필요, keymap.md §10 에 결정 보류로 기록)·에디터 밖 전역 ⌘K 삼킴
                범위 축소(계약 범위 밖 확장, 사용자 확인 필요)·`editorTextFocus` 게터 명명(계약
                §3.2 가 명시한 정의라 재정의는 계약 위반 — JSDoc 로 의미 차이만 명시). `bun run
                verify` 전체 + vite build 통과 확인.
          - [x] 메인 2차 완료 — 핵심 수정 7건 실물 재검증(registerTaideCustomActionCommands·팔레트
                스냅샷 가드·인스펙터 동결·⌘K ⌘S mods·when !terminalFocus·레거시 별칭·
                deriveMonacoChordPrefixes 전건 소스 확인) + verify 전체 exit 0(프론트 1098·Rust
                822+6+17) + vite build exit 0 메인 직접 실행. 계약 §7 검토 요약 보강. 커밋
    - [x] c-I. Wave I — 셸·워크스페이스(Zen/포커스·멀티 윈도우·설정 파일 탭 편집·플러그인 설치
          UI·VSIX grammar — 최대 구조 변경, 캠페인 최종).
          - [x] 정찰 완료(wf_c2974bc4-295, opus+high 4축) — 하중 전건 메인 재검증. **멀티 윈도우
                차단급 결함 4종 확정**(부창 닫기=앱 전체 종료·lsp_spawn 재사용 Channel 폐기·
                pty_attach 단일 구독자 탈취·capability 반생존) + load_layout 버전 폴백(마이그레이션
                없음 — 탭 전량 소실 위험)·설정 저장 미반영(sync 동일 기존 결함)·플러그인 신규 언어
                grammar 화면 미도달·vsix 원격 노출·zip 하드닝 부재
          - [x] 계약 확정(`2026-08-16-wave-i-shell-workspace-contract.md`, 결정 4건 — H prod 병합
                (9f16b3e)·**멀티 윈도우 완전 구현**(사용자: "MVP 가 아니라 제대로 완벽하게" — 창=탭
                분리 tabs.md §4.4 정합·차단급 4종 근본 수정: LSP 채널 다중화·pty 다중 구독자·창별
                close·capability 글로브)·Zen 추천(레이아웃 영속+마이그레이션 arm 신설)·설정탭+
                플러그인 추천(AppFile·SettingsChanged·VSIX→플러그인 착지·동적 언어 등록·원격 거부
                전환·zip 하드닝))
          - [x] 구현 완료(wf_45c10e53-806, 6에이전트: S1 인프라 → S2 도메인 → F1 창∥F2 Zen∥F3
                설정탭·플러그인 → D 통합) — 프론트 1128·Rust 872+6 그린, D 가 접합부 결함 4건 근본
                수정(SettingsView projectId prop·aux shell isError·팔레트 openSettingsFile·zen 토글
                UI). **F2 가 Wave H 엔진 실결함 발견·수정**(chord pending entryId→entryIds — 형제
                chord 공존 불가 결함). 하위 상세는 아래 페이즈별 기록 참조
          - [x] Phase E 검토 완료(wf_04872c23-eb7, 37에이전트) — 4렌즈 61건 → critical/major **31건
                confirmed·1 refuted** → 수정 24/보류 4/기각 4. **3 critical 클러스터**: 보조 창 layout
                커맨드 전부 NotFound(all_roots 탐색)·원격 app_file_write RCE 우회(strip 가드 신설)·LSP
                세션 재사용 창간 id 충돌+중복 initialize(owner 스코프 재사용으로 창별 독립 세션 —
                계약 §3.1 문언 정정). major: pty detach 누수·vsix 경로탈출·window-state 겹침·.tmp 유령·
                focused_pane dangling·Zen fullscreen 강제해제·target:null 보조 창 오생성 등. 보류 4
                (보조 창 flush 핸드셰이크·restore 정책·팔레트·chord 삼킴 — backlog). 상세 계약 §6.
                메인 2차: 핵심 6건 실물 재검증+verify 전체 exit 0(1127/881+6+17)+vite build 0. 커밋
                - [x] S1 Rust 인프라(sonnet+xhigh) 완료 — 차단급 결함 4종 근본 수정. **LSP 채널
                      다중화**(`domain::lsp::commands::SessionEntry.channels: Vec<Channel<String>>`,
                      `lsp_spawn` 재사용 경로가 새 `on_message` 를 폐기 대신 구독 추가, 브로드캐스트
                      +send 실패 자동 제거, `broadcast_message` 단위 테스트 3건). **pty 다중
                      구독자**(`TerminalStore` `subscribers: Vec<Channel<...>>`, `pty_attach` 마다
                      추가+ring buffer 개별 replay+브로드캐스트, `broadcast_output` 단위 테스트 3건,
                      PauseGate 는 세션 전역이라 다중 구독자와 무관함을 doc comment 로 명시).
                      **창 생명주기**: `domain::window` 신설(`window_open_auxiliary` 커맨드 —
                      Rust 가 `editor-<n>` 라벨 발급, `WindowStore` 로 라벨→(project, slot) 레지스트리)
                      + `handle_close_requested` 라벨 분기(보조 창은 close 를 막지 않고 그냥
                      닫히며 `plan_return_of_auxiliary_window_tabs` 훅 — 실제 탭 복귀는 S2 의
                      `ProjectLayout.auxiliary_windows` 스키마가 없어 **S2 로 인계**, main 은 기존
                      전역 flush 유지) + `AppState` hot-exit 을 `Pending(HashSet<라벨>)` 로 바꿔
                      **창별 confirm 집계**(`state.rs` 테스트 7건) + `file_flush_complete` 가
                      호출 창 라벨을 받아 확인 + `tauri-plugin-window-state` `map_label` 로 보조
                      창 정규화. **capability**: `capabilities/main.json` windows 를
                      `["main","editor-*"]` 로 글로브 확장 — tauri-utils/tauri 소스 직접 확인
                      (`glob::Pattern` 매칭, `ipc/authority.rs::resolve_access`) 으로 실동작
                      검증 완료(별도 capability 파일 불필요). 원격 dispatch: `window_open_auxiliary`
                      명시 거부(`deny_remote_window_open`). 배선(lib.rs 커맨드 등록·dispatch 파리티·
                      bindings 재생성) 완료. 검증: cargo fmt/clippy(-D warnings)/test(843) 그린,
                      bun typecheck/lint/format/test 그린. 기존 단일 창 시나리오 회귀 없음(state.rs
                      테스트로 고정). 미해결: `domain::file::service::tests::dirty_미러는...`
                      mtime 기반 사전 존재 flaky 테스트(내 변경 무관, base 커밋에서도 재현 확인,
                      S1 파일 무접촉) — 별도 트리아지 필요. S2(레이아웃 스키마+마이그레이션+Zen+
                      AppFile+플러그인)·F(프론트 3분할)·D(통합·문서) 완료. E(검토) 잔여.
                - [x] S2 Rust 도메인(sonnet+xhigh) 완료. **레이아웃 스키마 v2**: `ProjectLayout`
                      에 `auxiliary_windows: Vec<AuxWindowLayout{slot,root,focused_pane}>` +
                      `shell_view: ShellViewState{zen,sidebar_collapsed}` 추가,
                      `LAYOUT_SCHEMA_VERSION` 1→2 + `migrate_layout`(v1 무손실 마이그레이션 —
                      순수 가산 필드라 버전 스탬프만, 왕복 테스트로 고정) + `load_layout` 이
                      `version <= 현재` 는 마이그레이션·초과분만 default 폴백(기존 "버전 불일치=
                      전량 소실" 결함 상환). **다중 창 인식 pane 연산**: 내부 `PaneTreeRef`(Main/
                      Auxiliary(idx))로 `open_tab`·`close_tab`·`move_tab`·`split`·`resize`·
                      `activate_tab`·`pin_tab`·`focus_pane`·`reopen_closed`·
                      `convert_untitled_to_file`·`next_untitled_index` 전부를 tree-aware 로
                      재작성(단일 창 시나리오는 정확히 기존 순서 보존 — 같은 리프 재정렬 엣지
                      케이스 포함). **탭 창간 이동**: `layout_move_tab_to_window`(Main/Existing/
                      NewAuxiliary) — `move_tab`의 cross-tree 능력을 그대로 재사용, 빈 보조 창
                      정리(`cleanup_emptied_auxiliary_windows`, `WindowStore::label_for` 역조회
                      추가). `window::commands::open_auxiliary_window` 를 mutation-guard-free
                      코어로 리팩터(창 열기 호출부 3곳 — 커맨드·부팅 복원·탭 이동 — 이 각자
                      가드를 잡아 재진입 데드락 회피 + 동시 라벨 경합 방지). **보조 창 닫기 인계
                      완결**: `window::service::plan_return_of_auxiliary_window_tabs` 스텁 →
                      실제 구현(탭을 main 말미로 복귀, LayoutChanged 이벤트+dirty 마킹). **부팅
                      시 보조 창 복원**: `lib.rs::restore_auxiliary_windows` 신설(열려있는 전
                      프로젝트의 `auxiliary_windows` 를 순회해 창 재생성, 각자 mutation guard로
                      직렬화). **Zen**: `layout_set_shell_view`+`ShellViewPatch` +
                      `Settings.zen_fullscreen`(기본 false)·`zen_hide_status_bar`(기본 true,
                      types·Default·SettingsPatch·apply_patch·sync 왕복 5곳) +
                      `window_set_fullscreen`(호출 창 자동 주입, capability 불필요 — 커스텀
                      커맨드는 ACL 밖). **AppFile**: `domain::app` 신설(`AppFileTarget`·
                      `PromptTemplateId` 실제 enum) + `TabKind::AppFile` + `app_file_read`/
                      `app_file_write` + `settings::commands::apply_and_broadcast`(가드-프리
                      공유 코어 — settings_update·sync_download·app_file_write 3곳이 동일
                      parse→sanitize→적용→`SettingsChanged`(신설, 전 창+원격 fanout) 경로
                      공유, sync_download 의 "저장은 되는데 반영 안 됨" 기존 결함 동반 해소).
                      **플러그인**: `plugin_install`(디렉토리/zip)·`plugin_uninstall`(빌트인
                      보호 없음) + `vsix::service::extract_hardened_zip`(엔트리 상한 5000·
                      스트리밍 예산 128MB·0o644/0o755 마스킹, `lsp_install::extract_zip`
                      재사용 안 함) + `vsix_import_plugin`(실 VS Code vsix
                      languages+grammars→`taide-plugin.json` 합성 후 기존 `install_from_*`
                      경로 재사용) + `infra::language::language_id_for_path`(플러그인
                      오버레이+빌트인 46개 캐노니컬 테이블로 file/git/ide 3벌 중복 통합 —
                      ide 의 grammar-미지원 csharp/php/sql 3개는 의도적으로 드롭). **원격
                      정책**: `plugin_install`·`plugin_uninstall`·`vsix_import_plugin`·
                      `window_set_fullscreen`·`layout_move_tab_to_window` 명시 거부,
                      `vsix_extract_themes` 허용→거부 전환. **locale**: 신규 네임스페이스
                      `zen`·`prompts` + 기존 `app`·`tab`·`keymap`·`settings` 확장, 총 33키×
                      en/ko/ja(필요 키 집합 809개 3벌 완전 일치 자동 검증). 검증: cargo fmt/
                      clippy(경고 0)/test(S1 843 → 878, 신규 35건 — layout 멀티 윈도우·마이그
                      레이션 12·app::service AppFile 5·plugin install/uninstall 6·
                      infra::language 오버레이 5·기타) 전부 그린, bindings.ts 재생성(2회 — 파리티
                      테스트 포함) 통과, `bun run typecheck`/`lint`/`format:check`/`bun test`
                      (1098) 전부 통과 — `settings.ipc.ts`의 `emptySettingsPatch`(S2 명시 소유)
                      에 zenFullscreen/zenHideStatusBar 2필드만 추가, 그 외 프론트 소비처 깨짐
                      0건. F1/F3·D 완료. E(검토) 잔여.
                - [x] F2 프론트 Zen·표시 상태(sonnet+xhigh) 완료. **shell_view 소비**:
                      `entities/layout`에 `setShellView`/`useSetShellView` 추가, `AppShell`이
                      zen 시 사이드바(AppSidebar)·explorer 패널(force-collapse)·상태바(설정 시)를
                      숨기고, `EditorArea`→`PaneNodeView`에 `zen` prop 을 스레딩해 모든 리프의
                      탭바를 숨김(보조 창은 main-only shell_view 라 항상 `zen=false`). 사이드바
                      접힘은 `shell_view.sidebarCollapsed`로 영속(드래그·⌘B 토글 양쪽 경로 모두
                      persist, 폭은 기존 로컬 default 유지 — ADR-0004 예외). **⌘K Z chord**:
                      `open-keybindings-editor`(⌘K ⌘S)와 1단(⌘K)을 공유하는 두 번째 실제 chord
                      로 추가하면서 Wave H 디스패치 엔진의 실제 결함을 발견·수정 — pending 상태가
                      `entryId`(단수)만 기억해 같은 1단을 공유하는 형제 chord 중 배열상 먼저 오는
                      것만 영구 우선했다(§8 충돌 판정은 형제 chord 를 이미 허용했지만 디스패치는
                      아니었음). `entryIds`(복수)로 전환 + `findMatchingChordPrefixEntries` 신설,
                      기존 단일 chord 시나리오 회귀 0(전체 keymap 테스트 156→충분히 확장 후 그린) +
                      신규 다중 chord 유닛/통합 테스트 다수 추가(`keymap.test.ts`·
                      `keymap-dispatch.test.ts`·`keymap-chord-store.test.ts`). ESC 복귀는 capture
                      단계가 아닌 `window` bubble 단계 리스너로 별도 구현(`event.defaultPrevented`
                      가드) — Radix 다이얼로그·monaco 자체 Escape 처리와 우선순위 충돌 없음.
                      `zen_fullscreen` 설정 시 `windowSetFullscreen` 호출(`entities/window` 신설).
                      팔레트 커맨드 `view.toggleZenMode`(auxiliary 창에서 비활성) 추가. Zen 진입
                      힌트 오버레이(`features/window/zen-mode-hint.tsx`, 3초 자동 소멸,
                      `useState(true)` 초기값 + 마운트 자체가 세션 경계라 effect 내 동기 setState
                      없음 — `react-hooks/set-state-in-effect` 준수). 설정 UI 2필드 토글은 F3
                      소유(경계만 설계, 미구현). 검증: `bun run typecheck`/`lint`(0 error)/
                      `format:check`/`test`(1125 pass) 전부 그린. `docs/features/keymap.md` 에
                      Wave I 추가분(entryIds 복수화) 콜아웃 반영.
                - [x] F1 프론트 창 부트스트랩(sonnet+xhigh) 완료. **부트스트랩 분기**:
                      `shared/lib/window-context.ts`(URL 쿼리 파싱) + `app.tsx` 가 보조 창엔
                      `AuxiliaryWindowShell`(사이드바·상태바 없는 에디터 전용 크롬)만, main 창엔
                      기존 `AppShell`+`CommandPalette`+`KeybindingsEditor`+`TaskRunnerDialog`(셋
                      다 전역 활성 프로젝트 의존이라 보조 창엔 미마운트 — 설계 결정으로 문서화)를
                      렌더. **다중 윈도우 인식 pane 연산**: `shared/lib/pane-tree.ts`
                      (`resolveWindowPaneTree`·`isPaneTreeEmpty`·`collectAllPaneTabs`)로
                      `editor-area.tsx`·`pane-tab-bar.tsx`의 GC/DnD/탭 커맨드 전부를 main-only
                      전제에서 다중 창 인식으로 전환. **Move Tab UI**: 탭 컨텍스트 메뉴에 "Move
                      into New Window"·"Move back to Main Window"·"Move to Window N" 3액션 +
                      팔레트 커맨드 2종. 검증: `typecheck`/`lint`/`format:check`/`test`(1128 pass)
                      전부 그린. 열린 이슈(D 가 처리): 보조 창의 layout query 가 에러로 실패해도
                      자동 닫기 effect 가 감지 못함, `settings-view.tsx` 가 전역 활성 프로젝트를
                      써서 보조 창의 Settings 탭이 잘못된 프로젝트 기준으로 동작할 수 있음.
                - [x] F3 프론트 설정탭·플러그인 UI(sonnet+xhigh) 완료. **AppFile**:
                      `TabKind::AppFile` 라우팅(`pane-node-view.tsx`) + `widgets/app-file-pane`
                      (monaco JSON 에디터, dirty·저장·에러 표시) + `entities/app-file` +
                      설정 화면 "Open settings.json" 버튼·프롬프트 3종 편집 진입점 +
                      `ipc-sync-provider.tsx`의 `SettingsChanged` 구독(`SETTINGS.CURRENT`
                      직접 갱신). **플러그인 UI**: `widgets/plugin-manager`(목록+설치+제거+VSIX
                      통합 임포트 다이얼로그)로 구 `features/settings/plugin-list.tsx`·
                      `features/theme/vsix-theme-import-*.tsx` 완전 대체(삭제). **monaco 동적
                      언어**: `shared/lib/monaco/register-plugin-languages.ts` — 플러그인 목록이
                      바뀌는 4개 뮤테이션(install/uninstall/reload/vsix-import) + 부팅
                      부트스트랩 전부에서 호출해 monaco 언어 등록 + shiki 재생성을 공유.
                      검증: `typecheck`/`test`(1128 pass)/`lint`(0 error, 내 파일 기준) 전부 그린.
                      열린 이슈(D 가 처리): 팔레트로 "Open settings.json" 여는 경로가 없었음
                      (command-registry.ts/command-palette.tsx 는 F1/F2 동시 수정 중이라 미착수로
                      명시 위임).
                - [x] D 통합·문서·전체 verify(sonnet+xhigh, 단독) 완료. **접합부 수정**(선행
                      openIssues 전수 검토 후 근본 수정): ① `settings-view.tsx` 가
                      `activeProjectQueryOptions()`(전역 활성 프로젝트) 대신 `PaneNodeView` 가
                      이미 갖고 있던 `projectId` prop 을 받도록 전환(`SettingsView: FC<{projectId}>`)
                      — 보조 창으로 옮겨진 Settings 탭이 그 창의 고정 프로젝트가 아니라 main 창의
                      활성 프로젝트 기준으로 "settings.json 열기"/프롬프트 탭을 열던 결함의 근본
                      수정(F1 openIssue). ② `auxiliary-window-shell.tsx`의 자동 닫기 effect 가
                      `!layout`뿐 아니라 `isError`도 감지하도록 확장 — `useQuery` 가 실패 후에도
                      마지막 성공 데이터를 들고 있어(project_close 로 `layout_get` 이 `NotFound`
                      로 계속 실패해도) 그 창이 마지막 상태에 얼어붙어 영원히 안 닫히던 결함의 근본
                      수정(F1 openIssue). ③ 팔레트에 `app.openSettingsFile` 커맨드 신설
                      (`command-registry.ts`+`command-palette.tsx`) — F3 가 소유 경계(command
                      palette 는 F1/F2 동시 수정 파일이라 위임)로 미룬 접합부 배선(F3 openIssue).
                      ④ 설정 화면 INTERFACE 섹션에 `zenFullscreen`/`zenHideStatusBar` 토글 2개
                      추가 — F2 가 "F3 소유로 위임"했고 F3 도 구현하지 않아 **양쪽 다 놓친** 배선
                      갭(Rust `Settings` 필드·locale 키·`use-zen-mode.ts` 소비 로직은 이미 있었으나
                      설정 UI 에 노출하는 스위치 자체가 없어 사용자가 절대 켤 수 없었다). 로케일
                      키 자체는 en/ko/ja 693키 3벌 완전 일치(자동 대조 재확인, 갭 0)로 이미
                      맞아 있어 신규 로케일 텍스트 추가는 필요 없었다.
                      **동작 검수(코드 추적, 실행 금지 준수)**: 부창 닫기 격리(`handle_close_
                      requested`가 `editor-*` 라벨을 `prevent_close` 없이 그냥 통과시킴, main 은
                      기존 전역 flush 유지) / LSP `channels: Vec<Channel<String>>`·pty
                      `subscribers: Vec<Channel<...>>` 가 단일 원소일 때 기존 단일 창 동작과
                      동치임을 브로드캐스트 함수·테스트로 확인 / 레이아웃 v1→v2 마이그레이션이
                      실제 v1 셰입 JSON(신필드 제거 후 저장) 왕복 테스트로 무손실 고정돼 있음을
                      확인 / `SettingsChanged` 가 `settings_update`·`sync_download`·
                      `app_file_write`(Settings) 3경로 전부에서 `apply_and_broadcast` 하나를
                      공유하고 프론트가 `setQueryData` 로 즉시 반영함을 확인 / 플러그인 설치→
                      `registerPluginLanguages`→`reinitShiki` 도달 경로가 install/uninstall/
                      reload/vsix-import 4개 뮤테이션 전부에서 공유됨을 확인 / 원격 거부 6종
                      (`window_open_auxiliary`·`window_set_fullscreen`·
                      `layout_move_tab_to_window`·`plugin_install`·`plugin_uninstall`·
                      `vsix_import_plugin`)+전환 1종(`vsix_extract_themes`)이 `dispatch.rs` 에
                      전부 배선돼 있음을 확인. **문서**: `layout-shell.md` §7(멀티 윈도우 신설)·
                      `window-chrome.md` §5/§6(보조 창 크롬·Zen 신설)·`tabs.md`
                      §3.1/§4.4("추후"·"미지원" stale 문구 정정, TabKind 목록에 AppFile 추가)·
                      `plugins.md` §6(설치 UI·VSIX grammar·zip 하드닝·동적 언어·원격 정책 신설)·
                      `ipc-contract.md`(Wave I 신규 커맨드·이벤트·zip 하드닝 보안 절)·
                      `data-model.md` §8(레이아웃 스키마 v2·AppFile·Settings 신필드)·
                      `qa6-checklist.md`(Wave I 실기 섹션 신설 — 창 열기/이동/닫기/복귀·공유 자원·
                      Zen·AppFile·플러그인·회귀 7개 카테고리)·`research/2026-08-13-vscode-cursor-
                      gap.md` §3·§7(멀티 윈도우·Zen·settings.json 편집·플러그인 설치 UI·VSIX
                      grammar 항목 전부 "구현 완료" 표기, P1 목록·backlog 중복 목록 정리)·
                      `docs/backlog.md`(멀티 윈도우·Zen·앱데이터 파일 에디터 편집·VSIX grammar
                      임포트 항목 종결, Copy into New Window 만 잔여 항목으로 분리). **해소 불가로
                      보고만 한 항목**(소유 밖·설계 판단·계약 승인 범위): LSP 채널의 root 별
                      소유 추적 부재(S1 문서화된 단순화) / plugin 재설치 거부(재설치=업그레이드
                      시맨틱 미정의, 안전한 기본값) / `layout_move_tab_to_window` 3변형 전부 원격
                      거부(확장 해석이나 근거 동일) / 빈 보조 창은 `layout_move_tab_to_window`
                      경로에서만 자동 정리(플레인 탭 닫기는 프론트가 감지해 자기 창을 닫는 것으로
                      대칭) / `restore_auxiliary_windows` 가 활성 프로젝트 무관 전체 복원(계약
                      문언 그대로) / `domain::file::service` 의 mtime 기반 사전 존재 flaky 테스트
                      (S1·S2 가 이미 base 커밋에서도 재현 확인 — 이번 세션 `cargo test` 반복
                      실행에서도 재현 1회 관찰, 무관한 도메인이라 미수정) / 팔레트가 보조 창에
                      미마운트되는 설계(F1 결정 유지) / `Copy into New Window` 미구현(신규 backlog
                      분리) / plugin 디렉토리 설치용 폴더 피커 부재(Tauri dialog 제약) / VSIX
                      grammar 실패 사유 세분화 불가(`AppError` 코드 부족).
                      **전체 검증**: `bun run verify`(typecheck→lint→format:check→test 1128 pass→
                      cargo fmt→cargo clippy -D warnings→cargo test 895) exit 0 +
                      `bunx vite build` exit 0(청크 크기 경고만, 에러 0) 재확인 완료.
                      F(3분할)·D 전부 완료. **E(4렌즈 검토)만 잔여** — 캠페인 A~I 코드/문서
                      작업은 이번 세션으로 종결, 검토·커밋은 별도 단계.
- [ ] d. 전문 QA — 기능 전수 리스트업 → 체크리스트 신설 → 기능별 심층 검토(opus+xhigh,
      심층은 opus+max) + e2e + **아키텍처·추상화 전수 감사 축**(기존 코드 포함 — 계약 §3.1)
    - [x] d-0b. 착수 확인 2차(2026-08-18, 3건 전부 추천안): ① 손 QA 수정 산출물 prod 병합
          완료(main=09e0e0f, branch -f + push 분리) ② #12 형제 system_* 원격 거부 전환 완료
          (wf_52b753ae-2e2 — system_open_path 포함 4종, 보안 렌즈 전수 누락 0, dev 4229020)
          ③ 다음 착수 = QA-W0 e2e 하네스 구축
    - [ ] d-3. QA-W0 — e2e 하네스(경로 A) 부트스트랩. **구축·검토·수정 완료, 파일럿 실행만
          잔여(사용자 준비 필요)**. 구축(wf_cecf692f-248): e2e/ 27파일·파일럿 12스펙·
          @playwright/test@1.62.1+webkit(실측 294MB)·bun test 오염 0. 설계 정합 검토
          (opus+high)가 10건 발견(H1 게이트 스펙 복구 불능 구조·M2 trace 세션 쿠키 유출면 등)
          → 전건 처분·수정(wf_c5d77ae9-329): H1 필드별 시도+하드 스톱+수동 복구 절차,
          M1 오라클 실질화, M2 outputDir 이동+gitignore 이중 방어, M3 진단 첨부, M4 픽스처
          한정 커밋 허용 명문화, M5 활성 프로젝트 원복, L1 재로그인 1회, L2 pgrep 보강,
          L3 한계 문서화, L4 셀렉터 강화, L5 자식 프로세스 env 위생. 사용 문서
          `docs/quality-assurance/2026-08-18-e2e-harness.md`. **파일럿 실행 전제(사용자)**:
          bun run tauri dev 기동 + REMOTE 비밀번호(8자+) 설정 + password_only ON + 활성화 ON
          + TAIDE_E2E_PASSWORD export 후 bun run e2e
    - [x] d-0c. 착수 확인 3차(2026-08-18, 3건 전부 추천안): ① #12+하네스 prod 병합 완료
          (main=5f15a84) ② 파일럿 지금 실행(사용자 준비 진행) ③ 아키텍처 감사 지금 기동
    - [x] d-4. 아키텍처·추상화 전수 감사 — **완료(보고서 정본화, 수정은 별도 승인 대기)**.
          표준 16배치(wf_10697c92-2cc, opus+xhigh 읽기 전용) → 1차 종합(10배치) → 2차 종합
          (wf_1141da0d-61d, R4~R8·X1 병합). **정본: `docs/quality-assurance/2026-08-18-
          architecture-audit.md`**(+배치요약·원시발견 부록 2건). 발견 297건(고유 289·critical
          19·major 146·minor 133)+압축제외 126=관측 423. 기계 강제 축(FSD·any·enum·function·
          useCallback/useMemo)은 ~950파일 위반 0 — 발견 전부가 "문서에만 있고 기계 미강제"인
          4축(도메인 경계·자원 수명·락 입도·계약 파리티). 16클러스터(C1~C16)·티어 T0 24항목
          (보안 5·데이터 손상 4 포함)/T1 11묶음/T2 10묶음/X1 계약 별도 트랙. **메인 실물
          재검증 9건 전건 확정**(watcher .git 필터·project_close 미회수·ime-debug 원문 수집·
          auto-save 경로 이월·agent_cli_install/lsp_install 원격 허용·forbid_directory 0·
          search_replace 가드부재·pty_kill 호출0). 확인 문항 12건은 보고서 §9
    - [x] d-5. 감사 T0(즉시 수정) 24항목 — **완료**(2026-08-18 사용자 4문항 전부 추천안:
          T0 지금·세부 8건 추천안·대형 3건 T1 포함·e2e 파일럿 준비). **계약 정본:
          `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md`**. 실행: R1(Rust 보안 deny 5)
          → R2(Rust 데이터·기능, 순차) → F 병렬 4 → D 통합 → **E 검토 완료**(wf_ae1ed818-690,
          4렌즈+적대적: major 3 confirmed → 수정. **#15 forbid_directory 롤백** — Tauri 2.11.5
          asset scope add-only 라 재오픈 시 asset(video/audio 미리보기) 영구 차단 회귀. asset
          scope 회수(감사 X1#7)는 T1 이월(register_uri_scheme_protocol). #18 isConflicted 절대경로
          미이관(병합충돌 UI 회귀) 수정 + minor 6. 상세 계약 §6). 메인 재검증: 롤백·수정 실물
          확정 + verify·vite build exit 0(flaky 소멸로 안정, 프론트 1195·Rust 959). 대형 3건
          (T1-H 락 IO·T2-E AppError·C13 도메인경계)은 T1 편성 — T1-H 착수 시 동시성 위험 재고지
        - [x] R1 — dispatch.rs 원격 보안 deny arm 5클러스터(#12~#16) 완료(별도 세션).
        - [x] R2 — Rust 데이터·기능 단독 완료. §2.2(#17 search_replace 파일단위 mutation
              guard 재획득·#19 write_atomic_preserving_mode 로 file_save/search_replace 원본
              모드 보존·#20 pty_write 락을 writer_handle 조회까지로 축소), §2.3(#21 project_close
              가 TerminalStore::kill_project 로 세션 일괄 kill + PtySession Drop 이 pause 해제
              선행 후 kill·#22 SettingsPatch 빈문자열=해제 를 merge_clearable_string 으로
              shellOverride/editorFontFamily/terminalFontFamily/uiFontFamily/aiProvider/aiModel
              6필드로 일반화·#23 검색 column 을 UTF-16 코드유닛+1-based 로 보정(preview
              matchStart/End 도 UTF-16 화)·#24 LSP 자동재시작 성공 시 Running 대신 Crashed 유지),
              §2.4(#1 watcher 무시필터를 감시 루트 기준 상대경로로 좁힘(git 워처 자체가 `.git`
              루트라 전량 무력이었던 결함도 함께 해소)·#2 project_close 동기 레이아웃 flush +
              flusher filter_map 미스 warn·#7 delete_theme/load_theme 에 ensure_safe_component),
              #18 Rust 측(git StatusRow/CommitFile 에 absPath/origAbsPath 동봉). cargo
              fmt/clippy -D warnings/test --workspace(935) 전부 그린, bindings.ts 재생성 확인,
              TS 소비 파형 1건(commit-detail-panel.test.ts 픽스처)만 최소 수정. dispatch.rs·
              agent·lib.rs fanout·ipc-contract 는 R1 소유라 무수정(project_close 의
              TerminalStore 는 `app.state::<TerminalStore>()` 로 우회 획득해 dispatch.rs 의
              `project_close(app, state, projectId)` 3-arg 호출부를 건드리지 않음). 소비부
              배선(#18 프론트·#22 프론트)은 Phase F4 이월
        - [x] F1 — editor-pane 완료(#3·#4). auto-save/preview 타이머에 `pathRef` 발화시점 경로
              일치 가드 추가(스테일 클로저가 B 탭 내용을 A 탭 경로에 쓰던 결함), 에디터 인스턴스
              레지스트리 등록을 `handleEditorMount` 1회성에서 `[tabId, editor]` 의존 effect 로
              이동(`key` 없는 pane 재사용 시 breadcrumbs/상태바가 낡은 탭의 인스턴스를 반환하던
              결함). `editor-instance-registry.test.ts` 신규(6건, 이 모듈에 테스트가 전무했음).
              `pane-node-view.tsx`/`editor-instance-registry.ts` 는 계약대로 무수정
        - [x] F2 — `isWithinRoot`(`workspace-edit-applier.ts`) 경로 정규화(#5, `.`/`..` lexical
              resolve + 구분자 통일 — monaco `Uri#fsPath` 가 `..` 를 resolve 하지 않는 점 +
              Windows 백슬래시 하드코딩 접두 버그 동시 해소) + 세션 `applyEdit` 핸들러를
              `spawnLspSession` 호출 이전(client 생성 직후)으로 등록 이동 + 무루트 전역 fallback
              (`registerWorkspaceApplyEditHandler`) 제거(#6, `bootstrap-lsp.ts` 호출부도 제거)
        - [x] F3 — `IdeSyncProvider` 신설(#9) — Claude Code IDE 프로토콜(diff/save/close-tab
              요청·상태 sync·진단 push)을 `StatusBarContent`(Zen 에서 언마운트돼 프로토콜 전체가
              멈추던 원인)에서 main 창 앱 루트(`app.tsx`, 보조창 트리는 제외)로 이관.
              `agent-wait-marker-registry.ts` 를 모듈스코프 `Map` 에서 `localStorage` 백엔드로
              교체(#11) — 창마다 별개 JS 렐름이라 보조 창으로 이동한 탭을 그 창에서 닫으면
              main 창이 등록한 마커가 안 보이던 결함
        - [x] F4 — git 경로 소비부 절대경로(#18 프론트, `git-panel.tsx` 의 파일 열기/경로 복사/
              탐색기 표시 3곳을 `row.path`→`row.absPath` 로 전환, git 동작 자체(stage/unstage/
              diff)는 상대경로 유지) + ime-debug 진단 플래그 게이트(#8, `setImeDebugEnabled`/
              `isImeDebugEnabled` 기본 false — `recordImeDebug` early return + 복사 커맨드
              `isEnabled` 게이트) + settings patch 프론트 반영(#22, 폰트 패밀리 피커·Shell
              Override·AI Provider 전환의 "해제" 조작이 `null` 대신 `''` 를 patch 에 싣도록 정정
              — 새 3상태 규약에서 `null` 은 "건드리지 않음"이라 이전 코드는 해제가 조용히
              무시됐다)
        - [x] Phase D 통합 — 접합부 검토(R1/R2/F1~F4 전체 diff 정독) + 문서 갱신 + 전체 검증.
              **접합부 결함 1건 직접 수정**: 계약 §3 이 "탭 닫기 시 pty_kill" 배선을 명시했으나
              R2 는 `project_close`(`TerminalStore::kill_project`)만 구현했고, 어느 F 트랙도
              탭 닫기 쪽을 배선하지 않아 재검 시점까지 `pty_kill`/`killPty` 호출부가 0건으로
              남아 있었다(#21 절반 누락) — `TerminalStore::kill_session` 신설 +
              `layout::commands::close_tab_and_finish` 에 `TabKind::Terminal` 분기로 직접 배선.
              그 외 접합부(SettingsPatch absPath/클리어 규약 소비 일치, IdeSyncProvider 마운트,
              deny arm 이름·bindings 정합)는 전부 이미 정합 확인. 문서:
              `docs/bug/2026-08-18-audit-t0-fixes.md` 신규(24항목 요지), `ipc-contract.md`
              (§3 "T0 감사 데이터·기능 수정" 신설 절), `data-model.md` §13 신설,
              `features/git.md`·`features/terminal.md`·`features/lsp.md` 갱신, qa6-checklist.md
              "감사 T0 24항목 재검" 절 신설(9개 실기 확인 그룹). 검증: `bun run typecheck`/
              `lint`(0 error·기존 경고 6건)/`format:check`/`test`(1189/1189, 직전 1169 대비
              +20)/`cargo fmt --check`/`cargo clippy -D warnings`/`cargo test --workspace`
              (935+6+17=958/958, 직전 938 대비 +20)/`vite build`(exit 0) 전부 그린
    - [ ] d-6. e2e 파일럿 실행 — 사용자 준비(앱 기동·REMOTE·비밀번호 export) 후 bun run e2e,
          결과 메인 분석(WebGL·포트 발견 실측). T0 와 독립 병행
    - [x] d-8. 감사 T1 정비 1차 배치(T1-E·T1-J·T1-B) — **완료**(dev `6917978`). 구현
          (wf_94e0eacb-c12, 6에이전트)+검토(wf_b31b634b-24d: major 1 confirmed — LSP kill 재사용
          PID SIGKILL 회귀(내 T1-J R7#9 도입분, is_exited 가드로 수정)·minor 6 전건 수정). 메인:
          kill 테스트 타이밍 flaky 직접 근본 수정(즉시성 단언→유한 폴링, 5회 소멸)·fanout 24→23
          문서 정정·X1#9 경합 기각(temp 파일 확인). asset scope 재등록(X1#7)은 T1-2차 이월
          (Tauri asset scope add-only+Range 재구현 보안 표면). verify·vite build exit 0
          (Rust 975·프론트 1195). 원격 라우팅 dispatch() 테스트는 REMOTE_DENIED_COMMANDS 테이블
          재구성으로 실라우팅 검증화(동어반복 해소). 사용자: T1 저위험
          묶음부터). T0 완료(main=eecb493) 위. **계약 정본:
          `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md`**. T1-E 계약 검증 테스트
          (이벤트/커맨드 파리티·Settings 필드·테마 토큰·sync 게이트)·T1-J 자원 회수(GitStore/
          TreeStore/LSP kill/restart_count/asset scope 재등록 — T0 #15 롤백분 근본안)·T1-B
          Settings 타입 union 좁히기(as 9지점 제거). 실행 R1→R2→R3 순차→F 병렬→D→E. 대형
          3건(T1-H·T1-I/C13·T2-E)은 후반 — 착수 시 위험 재고지
    - [x] d-9. 감사 T1 정비 2차 배치(T1-G·asset scope·T1-F) — **완료**(dev `acadace`). 구현
          (wf_e8320bf1-46d)+검토(wf_1500d5f6-f8f: major 1 기각(nosniff 재생 반증)·minor 11 처리).
          asset scope 재이월 대신 구현(register_uri_scheme_protocol+AppState 라이브 조회). 검토가
          중복→infra/range_file 공유 모듈로 근본 해소하며 parse_range 언더플로 패닉(serving.rs
          원격 경로 DoS 표면)·m4v MIME 동시 수정. asset:// 실 webview 는 KNOWN ISSUE(qa6). 메인:
          핵심 전건 재검증·verify·vite build exit 0(Rust 1010·프론트 1195). 상세 계약 §5. (원래
          계약 정본:
          `docs/acknowledge/2026-08-18-audit-t1-batch2-contract.md`**. T1-G 인프라·보안 하드닝
          (nonce Secure·/__taide/file CSP·nosniff·shell temp 권한·resolve_owning_project 결정성·
          http 싱글톤·manifest expect 제거·ai bytes 상한)·asset scope 재등록(X1#7 1차 이월 —
          Range 위험 재평가 후 진행/재이월)·T1-F 레이어 이동(C1 12파일). R1→R2→F→D→E. T1-C·T1-D
          는 3차. 대형(T1-H·T1-I)은 후반·위험 재고지
    - [x] d-7. mtime flaky 근본 수정 완료(dev `d2204f7`) — **실은 프로덕션 데이터 무결성 버그**.
          진단(wf_1798253b-8ad, fable+high): serde_json 기본 파서가 f64 를 ~5.5% 확률로 1 ULP
          낮게 파싱(float_roundtrip 피처 부재) → 앱 재시작 후 미러 복원 시 디스크 무변경에도
          가짜 conflict 배지. **단독 200회 중 14회 실패로 병렬성 무관 입증**(세션 내내 "병렬
          flaky·무관" 치부는 표본 착시). 메인 재검증: 실패 assert 688(conflict)·피처 미적용·
          수정 후 30회 전건 통과 확인. 수정 Cargo.toml float_roundtrip 1줄(신규 크레이트 아님).
          T0 와 무관 독립 버그라 별도 커밋. 상세 `docs/bug/2026-08-18-mirror-mtime-serde-json-
          float-roundtrip.md`. **이후 verify 게이트 안정**
    - [x] d-10. PATH env flaky 근본 수정 완료(dev `45adf9d`) — 세 번째 flaky. `lsp/service.rs`
          find_in_path 가 전역 env PATH 읽고 테스트 4곳이 전역 set_var("PATH") 조작 → cargo test
          병렬 경합(jdtls·confirms_healthy_restart 대표). 근본 수정: find_in_path_within(path_var)
          신설·detect_servers 체인 파라미터화·프로덕션 2곳(spawn_process·lsp_detect_servers)이 env
          읽어 전달·테스트 set_var 전부 삭제(0건). confirms_healthy_restart 는 비한정 sh 스폰이
          오염 PATH 검색 실패한 collateral damage 로 자연 해소. 메인: 병렬 6회 소멸 확증·verify
          그린. flaky 독립 별도 커밋. 상세 `docs/bug/2026-08-19-lsp-detect-path-env-parallel-race.md`.
          **flaky 3건 전부 근절 — verify 게이트 완전 안정**
    - [x] d-11. 세션 핸드오프(/prepare-new, 2026-08-19) — HANDOFF 재작성(2026-08-16→2026-08-19
          스냅샷: 손 QA·#12·e2e 하네스·감사 T0/T1·flaky 3건). PROCESS·HANDOFF 정합. 재개 프롬프트
          출력. **주의: PATH flaky(d-10) 진행 중·워킹트리 미커밋** — 새 세션 첫 작업
    - [x] d-0d. 착수 확인 4차(2026-08-19, 4건 전부 추천안): ① dev 선행분(T1-2차 `acadace`·문서·
          flaky#3 `45adf9d`) prod 병합 완료(main=85ea29b, branch -f + push 분리 실행) ② 다음 배치
          = 감사 T1 3차(T1-C·T1-D·T1-A 잔여) ③ tauri `test` feature 미도입 유지(REMOTE_DENIED_
          COMMANDS 테이블로 라우팅 검증 기달성) ④ e2e 파일럿·QA-W1 실기는 사용자 준비 시점 별도
    - [x] d-12. 감사 T1 정비 3차 배치(T1-C 서버상태 14건·T1-D 레지스트리 17건·T1-A 잔여 3건) —
          계약 `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md`. 착수 전 메인 실물 재확인
          13건 전건 일치. 세부 결정 3건(R7#6 제거 확정·store 이관 목적지 external-store 브리지·
          F6#5 범위 외) **전부 추천안 확정(2026-08-19)**.
          **구현 완료(wf_0874ff31-fa1)**: R(owner 스코프 4+세대 이벤트+신규 lsp_confirm_
          reinitialize)→F0(팩토리 2종+12종 마이그레이션)→F1(T1-C 13건)·F2(프로바이더 3+정리 4)·
          F3(LSP 5+shiki+재핸드셰이크, 중단 시도 1 산출물을 3차가 검증)→D(접합부 owner 6곳 봉합·
          문서). 메인 2차: 스팟 체크 전건 일치 + verify·vite build 직접 재실행 exit 0(프론트
          1283·Rust 995). 구현분 dev 커밋(`933a052`). 상세 계약 §3.5.
          **Phase E 완료(wf_b8b5d39b-595 검토 + wf_00fcdcc2-f38 수정)**: 4렌즈 47발견 → 적대적
          17건 confirmed 15·refuted 2 → 수정 3에이전트(critical: 원격 owner 위장 →
          enforce_remote_owner_label 재귀 강제 치환 / fsChanged 재설계 / claude-diff 전 창 판정 /
          LSP 6건: marker 회수·sessionsByKey 정리·root-aware API·SERVERS 제외 무효화·재핸드셰이크
          타임아웃+재시도·diagnostics client 필수화 + 부수 minor 5). 메인 2차: 수정 5축 실물
          재검증 + verify·vite build 재실행 exit 0(프론트 1304·Rust 1000·커맨드 180 파리티).
          문서 정정(ipc-contract owner 신뢰 경계·180종·data-model roots 오기). 이월 잔여 8건은
          계약 §5.1 정본(1순위: editor-pane 묶음 — F1#17 blame·F3#4 절반·F3#18 구독 2곳·
          root-aware 소비처 5곳)
    - [x] d-0e. 착수 확인 5차(2026-08-19, 2건 전부 추천안): ① T1 3차 산출물(계약·secrets·
          release.yml·구현 933a052·검토 수정 46d5504) prod 병합 완료(main=46d5504, 분리 실행)
          ② 다음 배치 = editor-pane 묶음(§5.1-1 + T2-B 분해)
    - [x] d-13. editor-pane 묶음 배치 — **완료**. 계약 `docs/acknowledge/2026-08-19-editor-pane-
          batch-contract.md`(§4 구현·§5 검토 기록). Phase A(1087→369줄·훅 6개, 동작 무변경 —
          검토가 라인 전수 대조로 충실성 확증)→B 병렬 2(blame·conflict 쿼리화(debounce 보존)·
          무효화 entities 회수·저수준 구독 표준화·root-aware 5곳)→메인 2차(useSaveFile(projectId?)
          로 F3#4 완결)→구현 커밋 `500dbce`→E 검토(4렌즈 38발견→적대적 12: confirmed 9·refuted 3)
          →수정(compareRequested 고착·waiter 세션키 큐·3중복 shared 승격·키 중앙화·
          settleAfterDiskWrite 캡슐화+minor 6)→메인 2차 스팟 7건+verify·vite 그린. T1 3차 §5.1-1
          이월(F1#17·F3#4·F3#18·root-aware 5곳) 전량 해소. qa6 실기 절 7항목 신설
    - [x] d-0f. 착수 확인 6차(2026-08-19): ① editor-pane 배치 산출물(500dbce·2f40d06) prod 병합
          완료(main=2f40d06) ② 다음 배치 = T1-K 원격 기본거부 ③ 잔여 작업 총괄 절 신설(이 문서
          상단 — 사용자 요청 "뭐가 얼마나 남았는지")
    - [x] d-14. 감사 T1-K 원격 게이팅 기본 거부 전환(C16 근본, 6건) — **완료**. 계약
          `docs/acknowledge/2026-08-19-audit-t1k-default-deny-contract.md`(§4 기록). 구현
          `4b76dda`: REMOTE_ALLOWED_COMMANDS 163·3단 게이트·RemoteDenialPolicy 8분류·완전 분할
          파리티(인위 누락 실측). 정책 무변경 전수 대조(구현·검토 독립 각 1회 — 공집합). E 검토
          (4렌즈 28발견→적대적 3: confirmed 2 minor 하향·refuted 1)→메인 직접 수정(arm↔ALLOWED
          파리티 테스트 신설·doc 상한 단서·문서 통일·agent_hooks_uninstall 허용 명시)→verify·
          vite 그린. 이제 신규 커맨드는 명시 등재 없인 원격 기본 거부(기계 강제)
    - [x] d-0g. 착수 확인 7차(2026-08-19): ① T1-K 산출물(4b76dda·19d77e6) prod 병합 완료
          (main=19d77e6) ② 다음 배치 = 저위험 청소(X-A 배선 8건+§5.1 소규모 잔여 5건)
    - [x] d-15. X-A 배선+소규모 잔여 청소 배치 — **완료**. 계약 `docs/acknowledge/2026-08-19-
          xa-wiring-cleanup-contract.md`(§4 구현·§5 F2 정정·§6 검토 종합). 구현 `8328023`(53파일):
          살리기 4(viewState·from_app·revision·cwd+터미널 파일 링크 실배선)+지우기(focus-kind·
          app:ready·중복 커맨드 5종 — 커맨드 176+raw3·이벤트 23)+§5.1 (2)(3)(4)(7)(8)+메인 접합부
          2(AppReady·remote:state-changed 소비). E 검토(4렌즈 45발견→적대적 16 **confirmed 16·
          refuted 0** — 신설 배선 실결함 전량 적중)→수정 3에이전트(viewState useLayoutEffect
          재설계·from_app temp sibling+배치 판정 실효화·guard_terminal_path 보안 가드·OSC7
          종결자·kind 한정 스킵·prune 배선+비적대 실결함 1 추가)→메인 2차 스팟 7축 전건+verify·
          vite 그린. X-A 트랙 완결(X1#2 포함). 운영 교훈: 공유 워킹트리 git stash 금지(2회 재발
          — 이후 Workflow 프롬프트 명시). 신규 이월 4건(viewState closed_tabs·useReplaceSearch
          invalidation·REMOTE 폴링 중복·predicate 한계)은 계약 §6
    - [x] d-16. 세션 핸드오프(/prepare-new, 2026-08-19) — HANDOFF 전면 재작성(이번 세션 4배치+
          Phase 8 사전준비 스냅샷·dev 17커밋·기준선 실측 프론트 1375/Rust 1030). 미응답 결정
          패키지(X-A prod 병합·다음=T1-I 추천) HANDOFF §6 이관. 재개 프롬프트 출력
    - [x] d-0h. 착수 확인 8차(2026-08-19, 새 세션 — 복원·실물 대조 후): 사용자 "전부 추천대로"
          — ① X-A 청소+핸드오프 4커밋 prod 병합 완료(main=353d590, branch -f + push 분리) ②
          다음 배치 = T1-I 도메인 경계 ③ 감사 §9 결정 10-A(코드를 architecture.md:77 규칙에)·
          12-A(Project.capabilities attach/detach 실현) 확정 + 위험 재고지 승인
    - [x] d-17. 감사 T1-I 도메인 경계 재조립(C13, 12건) — **완료**. 계약
          `docs/acknowledge/2026-08-19-audit-t1i-domain-boundary-contract.md`(§4 구현·§5 검토
          반영). 결정 10-A·12-A 실행. 구현 `1852820`(wf_e4ab6095-929, R1→R2→R3 순차 단독
          59파일): ProjectCapability 확장점 8종(open/close 순서 바이트 재현)·ide commands↔
          server 순환 절단(store 하강)·제2 진입점 service 경유·R6#2 공유 저장 경로·plugin↔
          vsix 단방향(infra/archive)·LanguageOverlay 반전·조립부 배선 3종·아키텍처 테스트
          (화이트리스트 기계 강제)·표면 무변경(커맨드 176+raw3·이벤트 23·ALLOWED 160⊎DENIED
          19). E 검토(wf_8a02c83f-46a 4렌즈 17발견→적대적 2: **confirmed 2·refuted 0** — C-1
          경계 스캔 import 우회·D1 capabilities 명목 소비, 정확성 렌즈는 동작 동등성 결함 0)
          → 수정 wf_5de1f040-63f 11항목(스캔 5형태 봉쇄+우회 4종 FAILED 실측·단일 출처화·
          save_file private·redact.rs 하강·문서 4곳) → 메인 2차(스팟 5축+verify·vite 직접
          재실행 exit 0, Rust 1061·프론트 1375). 이월: D8 AppState 공유 필드 사각지대·
          layout↔ide·window↔layout 순환·D6 배선 4기구 통합(계약 §5 기각 목록). qa6 +1절
    - [x] d-0i. 착수 확인 9차(2026-08-19, Stop hook 상시 지시 "전부 추천대로 계속 진행" 적용):
          ① T1-I 산출물 3커밋 prod 병합 완료(main=9bad3df — branch -f 분류기 차단으로
          checkout+ff-only 동등 절차, push 분리) ② 다음 배치 = T1-H 락 IO(추천안 — 위험 최고
          재고지는 T1-I 완결 보고에 명시 완료)
    - [x] d-18. 감사 T1-H 전역 락 IO 분리(C11 양축) — **완료**. 계약
          `docs/acknowledge/2026-08-19-audit-t1h-lock-io-contract.md`(§4 구현·§5 검토 반영).
          착수 전 실물 재확인: 기처리 3건(R7#3=T0#17·R8#2=T0#20·R6#2=T1-I) 제외·tree_rows
          무가드 되쓰기 잔존 실측·font_list 캐시 부재 건 정정. 원칙: **begin_mutation 입도
          재설계 기각(후속 이월)** — 락-IO 결합 국소 해소만. 구현 `100e6a4`(wf_4d766dcb-e89,
          13파일): git push/fetch 락 탈출·sync 3단계·plugin/vsix stage/commit 분리·font
          캐시·tree_rows 되쓰기 제거·update_index 제거·보류 2(git_pull CLI 융합·pty_spawn).
          E 검토(wf_6f7ce886-f9f 4렌즈 29발견→적대적 8 **confirmed 8·refuted 0** — sync
          disconnect 되살림 4렌즈 수렴·stale 적용 역행·git_pull blocking 풀 교착·commit
          §1.4 오인용) → 수정 wf_947a9c63-7b6 12파일(overlay_sync_bookkeeping·
          decide_download_apply 재검증 대칭·git_pull async 가드 동형화·git_commit/init/undo
          이관·tree in-place·최초 gist 생성 가드 유지 등) → 메인 2차(스팟 4축+verify·vite
          직접 exit 0, Rust 1076·프론트 1375). qa6 +1절(9항목). 이월: git2 in-process 13건·
          재진입 직렬화·원격 동시 상한·pty_spawn
    - [x] d-0j. 착수 확인 10차(2026-08-19, 상시 지시 적용): ① T1-H 산출물 3커밋 prod 병합
          완료(main=e554c9f) — **감사 T1 트랙 11묶음 전체 완결** ② 다음 배치 = T2 첫 배치로
          T2-I 로케일 외부화 선정(위험 최저·theme 패턴 재사용·최대 비대 파일 해소)
    - [x] d-19. T2-I 로케일 데이터 외부화 — **완료**. 계약
          `docs/acknowledge/2026-08-19-t2i-locale-externalization-contract.md`(§4 구현·§5 검토
          반영). 구현 `194a7c3`(wf_0a11c367-f88): en/ko/ja 리터럴 → resources/locales JSON,
          service.rs 4,081→1,242줄, 전수 파리티 기계 증명(diff 0)·미참조 32키 실측 제거.
          E 검토(wf_88220526-e95 4렌즈 15발견 — **이동충실성 독립 재검증 데이터 결함 0**,
          major 2 는 적대적에서 minor 강등 confirmed·refuted 0) → 수정 wf_8c67b030-da3
          (warm_builtin_catalogs 부팅 eager 검증·should_panic/중복 키 테스트·계약 기록 정합
          — 7키 정정·32키 목록·77키 분해·문서 4곳) → 메인 2차(verify·vite 직접 exit 0,
          Rust 1080·프론트 1375). qa6 +1절. **T2-F 잔여: 미판정 6키**(agent.badgeAriaLabel·
          themeEditor.preview* 5키 — 계약 §4 분해 참조)·이월 F5(경량 summaries)·D4(테이블화)
    - [x] d-20. 착수 확인 11차(2026-08-20, 새 세션 — 복원·실물 대조 후 goal "병합 계속하고
          추천안대로 배치 계속진행해"): ① 핸드오프 커밋 282b4e1 prod 병합 완료(main=282b4e1,
          checkout+ff-only 절차) ② 다음 배치 = 추천안 T2-D/F/G 통합 저위험 청소 확정.
          d-20 대표 실물 확인 기록(2026-08-20 세션 종료 시점 작성)은 본 세션 재실사로 보강:
          role='button' 은 commit-graph 조건부 포함 **7파일**·키보드 전무 4종 onKeyDown 부재
          전건 실측·Enter만 4지점 전건 실측·getLocale 소비처 0·entities/app 은 소비 체인
          실재(감사 "dead" 주장과 어긋남 — 실사 판정 대상)·레인 색 12 는 3중 정의(TS 2+Rust
          git/types.rs:4)·폰트 범위 드리프트는 T1-B 기처리 정황. 기처리 4건(app:ready=X-A·
          Project.capabilities=T1-I·resolve_terminal_path=X-A·미참조 로케일 키=T2-I) 범위 제외
    - [x] d-21. T2-D/F/G 통합 저위험 청소 배치 — **커밋 완결·prod 병합 보류**(d-22 실기 사건
          원인 확정까지). 계약 `docs/acknowledge/2026-08-20-t2dfg-cleanup-contract.md`(§4 구현·
          §5 검토 반영). 커밋 3: `fd42d59`(docs 계약)+`0b190bc`(refactor 구현 42파일 +241/−90)
          +`6e63067`(fix 검토 반영 14파일). 구현 wf_31e24384-ef3: 판정 fixed 21·already-handled
          2·unidentifiable 1(overwrite 오도 네이밍 — 단서 부재). E 검토 wf_500791bc-ff2(4렌즈
          24발견→적대적 major 5 **confirmed 5·refuted 0** — 이 중 2건은 배치 도입 실회귀:
          status-row Space 차단·color-picker 포커스 대상 변경) → 수정 wf_5fa13d64-d88(활성화
          target/repeat 가드·status-row 중첩 해소+group-focus-within·SV aria-valuetext·
          onOpenAutoFocus 복원·aria-expanded·기록 정정 — DEFAULT_FONT_SIZE 는 invalid-claim
          아닌 already-handled(T1-B) 재판정, 기각 8 사유 기록) → 메인 2차(스팟+verify·vite
          직접 exit 0, bun 1384·bindings 무변경). 로케일 776→779키×3. 이월: 계약 §5.4(레인
          토큰 열거 2곳 미보증·aria-controls 배선·APG keyup 이원화·terminal.ts 잔여 중복 등)
    - [x] d-22. **실기 사건 ① 빈 창 크래시 — 수정 완결(커밋 4건·실기 확증 대기)**. 계약
          `docs/acknowledge/2026-08-20-blank-window-hotfix-contract.md`(§1 원인·§5 구현·§7 잔존
          수정 — 전 과정 기록 정본). 원인 확정 사슬: eecb493(T0)의 [tabId, editor] 재등록
          effect 가 캐시 미스 커밋에서 dispose 된 구 에디터를 새 tabId 로 registry 등록 +
          git-gutter addAction 2 effect 가 시체의 _actions 를 무가드 재충전(monaco
          standaloneCodeEditor addAction 무가드 — 재조정 wf_fd1c5cf8-472 confirmed·메인 소스
          직독 검증) → getSupportedActions→isSupported→contextMatchesRules throw →
          ErrorBoundary 전무로 루트 언마운트. 로그 "web content process terminated" 는
          tauri-runtime-wry 2.11.4 생성 시 로그 버그(무관). 수정: `5822b85`(docs 계약)+
          `0f7b07b`(fix 렌더 중 조정 — resolveEditorStateForRender)+`2c8f0e5`(fix 잔존 경로 —
          프리뷰 분기 CodeEditor fiber 고정·사슬 고정 테스트)+`160871d`(fix 검토 반영 —
          프리뷰 off overflow 클리핑 복원(검토 confirmed major)·defaultSize 정리·기록 정직성).
          검토 2회전: 1차 17발견(major 1 refuted → 재조정으로 **전복** — addAction 재충전
          고리)·2차 17발견(major 1 confirmed — overflow 클리핑). 진단·검토 원문 스크래치패드
          7종 보존. **실기 확증(사용자) 대기**: 새 파일 연속 열기 + .md 프리뷰 on 상태 캐시 탭
          전환 + hover 위젯 브레드크럼 위 표시. T2-D/F/G 는 원인 무관 확정으로 병합 완료
          (main=4c4d6ce 시점)
    - [ ] d-23. 실기 사건 ② 초기 기동 ~5초 — 진단 완료·수정 배치 결정 대기. 광역 진단
          wf_14010876-849: dev 한정 monaco 1,331 모듈 eager 로드(2~4초) + setup() 동기 워처
          전수 워크(복원 프로젝트 RESUME 15,320파일 — 기존 코드 유래·4배치 무혐의). reveal
          게이트·T2-I warm·keyring·11111 프로브는 무혐의 판정. 결정 패키지(ErrorBoundary·부팅
          수정 배치·registry 등록 소유권 — 계약 §6) 상신 후 진행
    - [x] d-0. 착수 확인(2026-08-18) — 사용자 결정 2건 전부 추천안: ① dev 선행 문서 커밋 2건
          (c79e853·b4e7318) prod 병합 완료(main=b4e7318, branch -f + push 분리 실행) ② 전문 QA(d)
          착수. 착수 순서: 정찰/설계 Workflow → 추천안 패키지 질문(e2e 의존성 승인·감사 범위) →
          실행. Wave I 멀티 윈도우 실기 최우선. c/c-A~c-I 상위 체크박스 표기 누락 동시 정정
    - [x] d-1. 정찰/설계 Workflow(opus+high) + 추천안 패키지 확정(2026-08-18 사용자 결정 4건
          **전부 추천안**: 손 QA 수정 선행·e2e 의존성 승인(@playwright/test@1.62.1+webkit,
          *.e2e.ts·node 러너)·감사 표준 16에이전트·QA-W0~W7 편성). **설계 정본:
          `docs/acknowledge/2026-08-18-pro-qa-design.md`**(qa6 분류 a165/b51/c59·하네스 설계·
          감사 배치·티어링 요지 수록 — 스크래치패드 원문 요지 반영 완료)
        - [x] 정찰 완료(wf_7e69c774-358, opus+high 4축)·메인 재검증 16건 전건 일치 — qa6 분류
              a165/b51/c59(=275 검산), Wave I 창 생명주기 11항목 e2e 불가(window_open_auxiliary
              dispatch.rs:968 거부 확정), 원격 거부 arm 9종/11커맨드(리서치 "3종" stale 정정),
              로그 포트 파싱 불안정(tauri-plugin-log KeepOne 40KB — lsof+프로브 폴백 필요),
              monaco/xterm 전역 미노출·data-testid 0(오라클 FS/DOM/IPC 대체), WebglAddon 무보호
              (terminal-view.tsx:156), @playwright/test@1.62.1 실조회, 감사 배치 표준16/절충10안,
              심층 QA T0 6·T1 9·T2 16 티어링+QA-W0~W7 편성안. 원문은 세션 스크래치패드
              (요지는 착수 계약에 반영 예정)
        - [x] 손 QA 6건 진단 완료(wf_e1ff31e6-53b, fable+high 6축)·메인 재검증 전건 일치 —
              ①터미널 링크: WebLinksAddon 기본 핸들러 window.open 이 WKWebView 에서 null(무동작
              확정)+URL IPC 부재 ②부트 테마: reveal 게이트(테마만 대기)와 CSS 게이트(테마+로케일)
              불일치가 최유력(use-reveal-window.ts vs global.css:302)+follow_system_theme 피커
              무시 UX 결함 ③와일드카드: is_allowed_host 정확일치·sanitize 가 '*' 탈락·매처 중복
              (server.rs:98)·링크 발급 first() 함정 4지점 확정 ④파일 열기 블로킹: **lsp_stop 이
              전역 begin_mutation 락을 쥔 채 고정 2s+2s sleep**(lsp/commands.rs:484·338-355)+
              use-lsp-session 이 path 변경마다 세션 파괴 — layout/file 전 커맨드 큐잉으로 증상
              3종 전부 설명(확정) ⑤peek 겹침: tailwind preflight border-box 가 monaco twistie
              content-box 전제 파괴(오버라이드 1규칙으로 근본 해결) ⑥커밋 diff: Wave C 계약
              §3.2 원문이 원래 "파일 클릭→기존 diff 탭"(L0-2 가 범위 밖으로 미룬 TabKind::Diff
              rev 확장이 실행 수단 — 기각 재론 아님, 원계약 복귀)
    - [x] d-2. 손 QA 1차 발견 6건(2026-08-18 사용자 실기 보고) — 진단·계약·수정·**Phase E 검토·
          커밋 완료**. **수정 계약 정본: `docs/acknowledge/2026-08-18-hand-qa-fix-
          contract.md`** (사용자 승인: 전부 추천안 — 수식어 ⌘·⌥+altClickMovesCursor 해제·
          follow_system_theme 자동 해제·와일드카드 베이스 불포함·LSP Rust+프론트 병행). 실행:
          Phase R(Rust 단독)·F 병렬 4·D 통합 완료 → **E 검토(4렌즈+적대적+수정+메인 2차) → 커밋**.
          구현(wf_5a1c9ff7-dfb) 후 **메인 실물 재검증 완료**: lsp_stop diff 원분기 보존·스토어
          선제거·검증기·매칭 fn·TabKind·배선 전건 대조 일치, `bun run verify` 메인 직접 재실행
          exit 0(테스트 1127→1159 프론트·904→933 Rust). R 자진 하드닝 1건(lsp_restart 스토어
          재확인 — 가드 축소가 새로 연 stop/restart 경합 leak 차단)·D 접합부 결함 1건 발견·수정
          (flushLspSessionDisposal 미배선 → lsp-session-flush-registry). 구현분은 사용자 지시로
          검토 전 dev 선커밋(`b00c192` — verify 그린 상태). Phase E 는 자리 이동으로 일시중지
          (wf_0fefd373-dac, 완료 0건 정지) 후 2026-08-18 재개(wf_9a7edac2-576, 보존 스크립트 —
          검토 대상 b00c192 diff). **Phase E 완료**: 발견 22(major 3·minor 19) → 적대적 검증
          major 3건 전건 confirmed·반증 0 → 수정 + 잔여 minor 후속(wf_33e76fae-275). major 3:
          ①window.open noopener 항상 null(폴백 감지 사문화 — xterm 패턴으로 근본 수정)
          ②원격 shim 라벨 'main' 충돌(owner 불변식 미성립 — 'remote' 정정, 가드 축소 안전
          논거 복원) ③docs/utils .js 가 verify 를 깨는 자충(eslint ignores). minor 실질 11
          수정(stopping 배제·projectClosed 강제 dispose·레지스트리 entities/lsp 이동·배너
          공용화·set_theme apply_and_broadcast 경유·와일드카드 술어 단일 소유·URL 유니코드
          위장/userinfo 거부·ctrl 수식어 등)·기각 1(#18 폴링 이중화 — 사유 기록)·보류 1
          (#12 형제 system_* 원격 정책 — 사용자 확인 대기, QA-W2 연계). 상세 계약 §5.
          메인 2차: 수정 전건 실물 재검증 + verify 전체·vite build 직접 재실행 exit 0
          (프론트 1169·Rust 938)
        - [x] d-2-1. 터미널 링크 Option+클릭 시 외부 브라우저로 안 열림 — `system_open_external_url`
              신설(http/https 화이트리스트, 원격 거부) + `WebLinksAddon` 핸들러 주입(⌘/⌥ 겸용) +
              `window.open` 선시도→IPC 폴백 + `altClickMovesCursor: false`로 수정
        - [x] d-2-2. 초기 기동 시 테마 색상 미추종 — reveal 게이트가 로케일 `isFetched` 도 함께
              보도록 수정(`isWindowReadyToReveal`) + `follow_system_theme` 자동 해제(`set_theme`) +
              테마 로드 실패 배너
        - [x] d-2-3. remote allowed hosts 와일드카드 지원 추가 — `*.` 접두 1레이블 매칭(RFC 6125,
              베이스 도메인 불포함), `host_matches_allowed_entry` 로 `is_allowed_host`/
              `is_insecure_connection` 매칭 단일화, sanitize·링크 발급 폴백 동반
        - [x] d-2-4. 파일 열기 후 무반응·닫기 지연 — 원인 확정(`lsp_stop` 전역 락 보유 중 고정
              4초 sleep). `lsp_stop`/`lsp_restart` 가드 축소 + 프로세스 종료 폴링(`wait_for_
              process_exit`) + 프론트 세션 dispose 유예(`LSP_SESSION_DISPOSE_GRACE_MS`)로 수정.
              접합부(Phase D): 유예 세션의 프로젝트 닫기·hot-exit 확정 정리 배선 추가
        - [x] d-2-5. Peek Definitions 우측 파일 트리 chevron·파일명 겹침 — `.monaco-tl-twistie
              { box-sizing: content-box }` 1규칙으로 수정(`docs/bug/2026-08-18-peek-tree-caret-
              overlap.md`)
        - [x] d-2-6. git 커밋 상세 diff 를 에디터 탭으로 승격 — `TabKind::Diff` 에 `rev`/
              `parentRev`/`beforePath` 확장(하위 호환), 기존 `CommitFileDiff`/`git_show_file`
              재사용, 신규 커맨드 0(`docs/features/git.md` §9)
        - 실기 재검(qa6 추가 — 코드/자동테스트가 아니라 실제 기동으로만 확인 가능한 항목)은
          `docs/quality-assurance/2026-08-11-qa6-checklist.md` "손 QA 1차 수정 재검(2026-08-18)"
          절로 이월
- [ ] e. Phase 8 — 서명·공증 (d 통과 후)
    - [x] e-0. GitHub Actions secrets 사전 등록 — **완료(2026-08-19 사용자 등록 확인)**. 목록 정본
          `docs/acknowledge/2026-08-19-phase8-signing-secrets.md`. **raw-viewer release.yml 선례로
          5건 확정**(MACOS_CERTIFICATE_P12·MACOS_CERTIFICATE_PASSWORD·APPLE_ID·
          APPLE_APP_SPECIFIC_PASSWORD·APPLE_TEAM_ID — 동일 이름·값 재사용). 서명 아이덴티티는
          워크플로 자동 추출·키체인 pw 인라인이라 secret 불필요. updater 키 불필요
    - [x] e-1. 배포 사전준비 — **완료(2026-08-19)**. `.github/workflows/release.yml` 신설(148줄,
          wf_f3033c87-2f6 이식 + 메인 2차 실물 검증: 트리거 tags v*+workflow_dispatch 만·브랜치
          push 발화 없음·secret 참조 6개 정확·서명/공증 graceful skip·actionlint 재실행 PASS).
          raw-viewer 패턴 + TAIDE 적응(updater 완전 제거·`bun run tauri build` 래퍼 단일 명령이
          taide-cli 사이드카+tauri.bundle.conf.json 자동 적용·cargo test --release --workspace·
          bun 1.3.14 핀·taide-dmg 아티팩트·draft 릴리스). **실행은 Phase 8 본착수 때**(태그
          푸시 또는 수동) — 첫 실행 검증 항목(공증 로그·사이드카 서명 확인)은 그때 qa6 에 추가
- [x] f. PROCESS.md 아카이브 완료 — 문서화·Phase 0~7.10(W1~W7) 섹션 1,022줄을
      `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 이전 (1,171줄 → 151줄).
      직전 세션 4개 절(QA6 후속·기능 확장 1~3차)은 HANDOFF 참조라 유지
