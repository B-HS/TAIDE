# PROCESS — TAIDE 작업 상태

> 기준 문서: `~/.claude/convention/*.md`(전 컨벤션), `docs/acknowledge/`(결정), 이 문서(체크리스트).
> 구현 순서 정본은 `docs/roadmap.md`, 버전 정본은 `docs/tech-stack.md`, API 정본은 `docs/research/*.md`.

> 과거 기록(문서화·Phase 0~7.10 W1~W7)은 `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 아카이브됨 (2026-08-14).
> QA6 후속·기능 확장 1~3차(2026-08-12~14)는 `docs/history/2026-08-16-process-archive-qa6-feature-waves.md` 로 아카이브됨 (2026-08-16).

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
    - [ ] d-2. 손 QA 1차 발견 6건(2026-08-18 사용자 실기 보고) — 진단·재검증·계약 확정 완료,
          수정 착수. **수정 계약 정본: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md`**
          (사용자 승인: 전부 추천안 — 수식어 ⌘·⌥+altClickMovesCursor 해제·follow_system_theme
          자동 해제·와일드카드 베이스 불포함·LSP Rust+프론트 병행). 실행: Phase R(Rust 단독)
          → F 병렬 4 → D 통합 → E 검토(4렌즈+적대적+수정+메인 2차) → 커밋
        - [ ] d-2-1. 터미널 링크 Option+클릭 시 외부 브라우저로 안 열림
        - [ ] d-2-2. 초기 기동 시 테마 색상 미추종 의심(콜드 스타트 — reload 플래시 수정과 별개
              경로 여부 확인)
        - [ ] d-2-3. remote allowed hosts 와일드카드 미지원 → 지원 추가(패턴 의미론·보안 영향
              검토 동반)
        - [ ] d-2-4. 파일 열기 후 로딩("코드베이스 읽기") 완료 전까지 단축키·힌트 무반응, 닫기
              지연 — 원인 미상(LSP/인덱싱/프리로드 후보) 진단 필요
        - [ ] d-2-5. Peek Definitions 우측 파일 트리에서 파일명과 chevron(캐럿) 겹침(스크린샷
              확보) — monaco peek 트리 스타일 결함
        - [ ] d-2-6. git 커밋 상세: 파일 클릭 시 diff 가 커밋 파일 목록 패널 안에 렌더됨 → 에디터
              code view(디프 탭) 로 열리게 변경(Wave C L0-2 결정과의 관계 확인 동반)
- [ ] e. Phase 8 — 서명·공증 (d 통과 후)
- [x] f. PROCESS.md 아카이브 완료 — 문서화·Phase 0~7.10(W1~W7) 섹션 1,022줄을
      `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 이전 (1,171줄 → 151줄).
      직전 세션 4개 절(QA6 후속·기능 확장 1~3차)은 HANDOFF 참조라 유지
