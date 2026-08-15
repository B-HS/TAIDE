# PROCESS — TAIDE 작업 상태

> 기준 문서: `~/.claude/convention/*.md`(전 컨벤션), `docs/acknowledge/`(결정), 이 문서(체크리스트).
> 구현 순서 정본은 `docs/roadmap.md`, 버전 정본은 `docs/tech-stack.md`, API 정본은 `docs/research/*.md`.

> 과거 기록(문서화·Phase 0~7.10 W1~W7)은 `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 아카이브됨 (2026-08-14).

## 진행 중: QA6 후속 1차 — 사용자 실기 보고 5건 (2026-08-12)

> 사용자 실기(QA6 진행 중) 보고. W7 CSP 항목은 **하이라이팅 실기 동작 확인됨**(devtools 콘솔
> 무오류 확인만 잔여 — W7 재설계 불필요 방향). 역할: 3·4번 IME 버그 = **fable+high**(사용자 지정),
> 정찰·리서치 = opus+medium, 구현 = sonnet+high, 메인 2차.

- [ ] a. IME 버그 2건 — 영→한 전환 직후 자모 분리(다음 글자에 해소)·한글 타이핑 중 상하 떨림.
      발생 표면 monaco 에디터 한정(사용자 확인). fable+high 디버그·수정 (wf_830fe067-9a9 debug:ime)
      - [x] a-1. 떨림(버그 4): 원인 = 폰트 폴백 스택에 한글 글리프 폰트 부재 → 조합 오버레이
            textarea/뷰 라인 간 폴백 비결정성. 수정 = font-stack 에 "Apple SD Gothic Neo" 추가
            (monaco 소스 인용 3곳 메인 실물 대조·typecheck·393 테스트 메인 재실행 통과). 실기 확인 대기
      - [x] a-2. 자모 분리(버그 3): 판정 = WKWebView↔monaco 환경층(앱 코드 무혐의). 상류 조사
            (opus+medium) 완료 — monaco 0.56.0 이 최신·상위 버전 부재·vscode main 에 수정 0건 →
            사용자 결정 분기대로 backlog 확정. 실험 옵션(accessibilitySupport off — 미검증 가설)
            포함 상세: docs/bug/2026-08-12-editor-korean-ime.md, backlog.md
- [x] b. 단축키 설정 전면 확장 — 전 커맨드(미할당 포함) 노출 + **별도 모달**(VS Code 식 검색·
      재바인딩 강화, 사용자 결정 2026-08-12). chord/when 은 backlog 유지. 정찰 → 계약(§3.1) → 구현
      완료 — `shared/lib/keybinding-catalog.ts`(신설) + `widgets/keybindings-editor/` +
      `features/settings/keybinding-row.tsx` + `KeymapOverrideEntry.actionId: string` 확장.
      실기 확인은 e 로 이관.
- [x] c. AI provider "OMLX" 추가 — jundot/omlx 특정, FIM 우선→chat 폴백 통합(사용자: 리서치 위임)
      완료 — `src-tauri/src/domain/ai/providers/omlx.rs` + `AiProviderId::Omlx` +
      `Settings.ai_omlx_base_url` + `features/settings/ai-omlx-row.tsx`. 실기(실서버 왕복)는 e 로 이관.
- [x] d. 툴팁 전수 감사 — raw i18n 키 노출·툴팁 부재·비일관 사용 전수 조사 → 수정(locale 4곳 동기 주의)
      완료 — `shared/ui/icon-button.tsx`(신설, aria-label+Tooltip 강제) 일괄 적용 + native title= 승격
      + 재렌더 결함 수정(`bindI18nStore`). **툴팁 side 규범 문서는 계약 §3.3 이 명시한
      `docs/features` 가 아니라 `docs/memory/tooltip-conventions.md` 에 있다** — 다음 세션은 이 경로로
      찾는다. 육안 확인은 e 로 이관.
- [ ] e. QA6 잔여 — W7 콘솔 무오류 확인 + b/c/d 실기 확인(단축키 모달·OMLX 실서버 왕복·툴팁 육안) +
      체크리스트 나머지 항목 계속(사용자 실기)

## 진행 중: 기능 확장 1차 — 사용자 요구 4건 (2026-08-13)

> 사용자 보고: VS Code·Cursor 대비 기능 격차 체감. 정찰(opus) → 계약 → 구현 순.

- [x] g. blame 표시 UX 변경 — IViewZone 으로 포커스 줄 위 이동(`shared/lib/monaco/blame-zone.ts`,
      keepCursorStable 시프트 보정·StableEditorScrollState 관용구·첫 줄 afterLineNumber 0).
      ghost text 충돌 구조적 소멸. 실기 확인 대기
- [x] h. monaco 에디터 액션 노출 — 150건 전수(`shared/lib/monaco-actions.ts`), B안(실행 monaco·
      표시/재바인딩 모달, addKeybindingRules), 하이브리드 i18n(tier1 42 번역). capture
      stopPropagation 잠재결함 수정 동반. 메인 2차 추가 수정 2건: 변환 불가 키 재바인딩 시 기본
      바인딩 보존+캡처 거부, 미지정 판정을 isKeybindingRowUnassigned 로 통일(표시·필터·카운트)
- [x] i. taide CLI — install/uninstall(osascript·멱등·소유 검증·dangling 판정) + 콜드스타트 argv
      pending 큐 + AgentExternalOpen 프론트 배선(폴더/파일 분기·wait marker 해제) + externalBin
      사이드카(`scripts/tauri.ts` 진입점 — dev 를 깨지 않도록 tauri.bundle.conf.json 분리, 계약
      문언 대비 정당한 편차). 디스패치 137→140. 실기 확인 대기
- [x] j. VS Code·Cursor 갭 분석 — `docs/research/2026-08-13-vscode-cursor-gap.md`(P0 10·P1 10·
      우위 9·비추천 목록). backlog 정정 1건. P0 퀵윈 5건 동반 구현: sticky scroll 설정·커서 위치
      상태바·documentHighlight/selectionRange 어댑터·LSP initialize capabilities 확충
- 검토 웨이브: 렌즈 30건 → 검증 14건 → 확정 11건(중복 제외 9) 전부 수정(critical: sticky prop
  누락 / major: 액션 회색 고착·중복 재오픈·blame 뷰포트 이탈·wait marker 누수 등). 오버플로
  16건 중 실질 신규 2건은 메인이 직접 수정, 잔여는 중복 또는 minor(알려진 한계로 기록)

## 진행 중: 기능 확장 3차 — 사용자 요구 2건 (2026-08-14)

- [x] o. Hot Exit 구현 완료(wf_371d9fa7-504) — 미러 복원 IPC 7종+untitled 축(탭 비휘발성화)·
      baseline mtime 충돌 판정·CloseRequested 인터셉트(타임아웃 폴백)·디바운스 500ms·
      언마운트/블러 플러시·탭 활성화 lazy 복원·ConflictBanner variant·GC(탭닫기+prune).
      검토 확정 수정: **경로 탈출(root_guard)**·탭 전환 편집 소실(캐시 직접 갱신+플러시 await)·
      **⌘Q 인터셉트 우회**(메뉴 Quit 교체)·플러시 이벤트 원격 팬아웃 제거. 디스패치 145.
      실기 확인 대기(qa6-checklist "기능 확장 3차" 13항목)
- [x] p. Remote 비밀번호 구현 완료 — 2요소 게이트(미설정 시 현행 유지)·password_only 토글·
      keyring salt+sha256·세션 7일 만료·지수 백오프 잠금·로그인 페이지(JS 0·3언어)·
      remote_set/clear_password(원격 차단). 검토 확정 수정: **keyring 실패 fail-closed**·
      폴링 키링 캐시·nonce 다중 슬롯. 메인 추가 검증 후 마감 2건(wf_9814ec51-946):
      **WS 세션 epoch 즉시 단절**(revoke·비밀번호 변경 시 열린 소켓 종료)·원격 settings_update
      의 게이트 필드 스트립. 실기 확인 대기
- [x] q. IconButton 레이아웃 회귀(사용자 보고: 설정 버튼이 위로) — 툴팁 래퍼 span 이 flex 자식이
      되며 내부 버튼의 위치 클래스(mt-auto·ml-auto·mt-0.5·mr-1·hidden/group-hover)가 무효.
      전 소비처 grep 으로 동종 4곳 추가 발견, containerClassName(위치=래퍼/시각·상태=버튼) 분리
      + 래퍼 기본 shrink-0 으로 근본 수정(메인 직접, c675d6f)
- 잔여 minor 하드닝(미검증 오버플로 — backlog 등재): 개별 세션 TTL 만료의 라이브 WS 단절,
  Host 허용목록(DNS rebinding), 비밀번호 최소 길이·trim 정합, 전역 잠금 DoS 완화, stale nonce
  실패 집계, X-Forwarded-Proto 신뢰, gist 인바운드 게이트 필드 필터, 저장 직후 디바운스 부활,
  timeout_ms payload 미사용

## 진행 중: 기능 확장 2차 — 사용자 실기 보고 3건 (2026-08-13)

- [x] k. 단축키 재바인딩 버그(Trigger Suggest = Opt+Space 무동작) — 원인 확정(fable+high): macOS
      Alt 조합은 event.key 가 합성 문자(NBSP·˚)로 도착, 캡처가 event.key 기반 + 가드 무피드백 거부.
      수정 = `normalizeKeymapEventKey`(**event.key 가 깨끗한 단일 문자면 유지, 합성/데드 문자일
      때만 event.code 유도** — code 무조건 우선은 비US 레이아웃에서 monaco 의 keyCode 해석과
      어긋나 기각) + 캡처·런타임 매칭 통일 + 거부 토스트 2종(바인딩 불가·수식어 필요) + 레거시
      합성 문자 저장분 OR 매칭 하위호환. monaco 측은 keyCode 기반이라 무결(소스 논증)
- [x] l. blame footer 바 — view zone 전량 삭제, `features/editor/blame-footer-bar.tsx` 신설
      (pane 별 자기 커서 줄, 고정 높이 h-6 — 레이아웃 흔들림 방지 근거 기록, ref.textContent
      갱신은 외부 시스템 동기화 패턴 — 렌즈 지적을 사유와 함께 수용 기각)
- [x] m. 시스템 usage 상세 모달 — `system_usage_breakdown` IPC(디스패치 141), 자손 BFS + 도메인
      PID 매핑(터미널 foreground_pids·LSP server_pids 신설·에이전트 캐시) + 프로세스명 폴백,
      모달 열림 동안만 3초 폴링. LSP PID 미저장 갭도 최소 침습으로 해소
- 검토 웨이브: 렌즈 25건 → 확정 7건(중복 제외 5) 전부 수정 — 레거시 override 하위호환(OR 매칭),
  System 인스턴스 분리(상태바 CPU% 붕괴), 첫 pid 샘플 cpuPercent null 게이팅, 수식어 없는 캡처
  토스트, ipc-contract.md 정본화(1차 커맨드 3종은 메인이 보완). 기각 5건(footer 고정 높이 등은
  의도 설계로 판정)
- [x] n. suggest 위젯 화살표 누수 — 원인 확정(fable+high, monaco resolver 를 bun 으로 실로드한
      리프로 실증): unbind keybinding:0 규칙은 무혐의(커맨드 한정 제거·부작용 0). 실제 원인은
      **재바인딩 modifier(⌥)를 유지한 채 누른 ↑** 가 Alt+Up=moveLinesUp 으로 해석되는 커버리지
      공백(monaco 는 자기 기본 트리거의 modifier 만 secondary 로 커버). 수정 = 팝업 트리거
      (suggest·parameterHints) 재바인딩 시 그 modifier+Up/Down/PgUp/PgDn 을 위젯 탐색에 바인딩하는
      **동반 규칙**(when 은 monaco 기본 미러 + parameterHints 동시 열림 양보 가드). 위젯 컨텍스트
      키 전수 감사 표 작성(Enter/Tab/Esc/find/rename/hover/마커/메뉴 전부 통과 판정). 검토 확정
      4건 중 3건 수정(⌥↑ 시그니처 전환 보존·규칙 순서 결정론화·editorFocus 조건), 1건은
      "⌥ 유지 타건" 전제의 실기 미확증 지적 — **사용자 실기 확인 대기**(1건 제안 시 ↑=커서 이동은
      VS Code 동일 스펙임도 안내). 구조적 한계 2건 기록(위젯 사용 키로의 앱/모나코 재바인딩은
      컨텍스트 프리라 위젯 선취 — 의미상 불가피)
- [x] f. QA6 후속 1차 확정 결함 14건 수정 (defect-fix 에이전트, 2026-08-12) — OMLX base URL 해제 불가
      (settings-view.tsx + service.rs `merge_ai_omlx_base_url` 신설, empty-string 센티널) · 스킴만
      있는/host 없는 URL 통과 및 조용한 기존값 폐기(`sanitize_optional_url` host 검증 + 유효성 실패
      시 기존값 보존) · keybinding-row 아이콘 버튼 native title=→Tooltip+aria-label 승격 · 커맨드
      팔레트 단축키 표시가 새 바인딩 모델(override·unbind) 미반영 → `findKeybindingRowById` 로 교체 ·
      팔레트 검색이 카테고리 라벨과 불일치 → `formatCategorizedLabel` 로 통일 · `formatKeymapShortcut`
      mac 순서 역행(Apple 표기 ⌘ 항상 마지막) 수정 · 수식키 없는 캡처가 저장되어 전역 커맨드로
      실행되는 결함(keybinding-row·keybindings-editor 양쪽에 mods.length===0 가드) · `IconButton`
      disabled 상태에서 Radix 툴팁이 뜨지 않던 결함(span 트리거로 감싸 hover/focus 위임) ·
      locale-provider 마운트 게이트가 `bindI18nStore` 구독 타이밍과 충돌해 en 사용자 재렌더 0회 +
      로케일 쿼리 실패 시 영구 백지 → 게이트를 CSS(`data-locale-ready`, 기존 `data-theme-ready` 패턴
      확장)로 이전, 에러 배너+재시도 추가 · 본 항목(PROCESS.md b/c/d 미체크·문서 위치 불일치).
      검증: `bun run typecheck`·`bun run lint`·`bun test`·`cargo fmt`·`cargo test` — 상세는 세션 보고 참고.

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
- [ ] c. 잔여 기능 구현 — 갭 P0 잔여 9건 + Remote 하드닝 9 + Hot Exit 미세 3 + P1 10건.
      **웨이브 A→I 확정**(계약 §2-2): A LSP 인텔리전스 → B 하드닝 → C Git → D 탐색·검색 →
      E 터미널·태스크 → F 에디터 표현 → G AI → H 키맵 엔진 → I 셸·워크스페이스
    - [ ] c-A. Wave A — LSP 인텔리전스. 정찰 완료(wf_0d6ea4d6-7b4, 529 재시도 4차 만에 성공,
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
    - [ ] c-B. Wave B — 하드닝 마감. 정찰 완료(wf_e628c338-e87, opus+high 2축)에서 "minor" 를
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
    - [ ] c-C. Wave C — Git 확장(3-way 머지·줄 stage·커밋 상세·파일 히스토리·revert/tag·원격
          checkout·파일 blame 뷰). 정찰(wf_563a88b2-20f) 축1 Rust·축3 API 완료(git 커맨드 27종·
          git2+CLI 하이브리드 확인), 축2 프론트 UI 는 StructuredOutput 초과 실패 → resume 재실행 중.
          정찰 3축 완료(축2 resume 성공) → 계약 확정(`2026-08-15-wave-c-git-contract.md`, 결정 4건:
          3-way 인라인 데코레이터·hunk stage git2 native+선택 라인·경량 배치·revert/tag/checkout
          추천). 구현 A(백엔드 Rust 단독)→B(프론트 병렬 3)→검토 진행
    - [ ] c-D ~ c-I. 순차 진행 (탐색·검색 → 터미널·태스크 → 에디터 표현 → AI → 키맵 → 셸)
- [ ] d. 전문 QA — 기능 전수 리스트업 → 체크리스트 신설 → 기능별 심층 검토(opus+xhigh,
      심층은 opus+max) + e2e + **아키텍처·추상화 전수 감사 축**(기존 코드 포함 — 계약 §3.1)
- [ ] e. Phase 8 — 서명·공증 (d 통과 후)
- [x] f. PROCESS.md 아카이브 완료 — 문서화·Phase 0~7.10(W1~W7) 섹션 1,022줄을
      `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 이전 (1,171줄 → 151줄).
      직전 세션 4개 절(QA6 후속·기능 확장 1~3차)은 HANDOFF 참조라 유지
