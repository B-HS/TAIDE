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
    - [ ] c-D. Wave D — 탐색·검색(팔레트 @/: 모드·Workspace Symbol ⌘T·Breadcrumbs·Search Editor·
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
    - **문서 부채(캠페인 누적)**: Wave D 계약 §5 문서 갱신(features·ipc-contract·data-model·
      tech-stack `ignore`·qa6-checklist) 유예. Wave B 도 일부 유예 가능성. → 전문 QA(d) 착수 전
      문서화 워크플로로 일괄 정리 예정
    - [ ] c-E. Wave E — 터미널·태스크(OSC133 셸 통합·태스크 러너·Run Selected Text). 정찰
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
    - [ ] c-F. Wave F — 에디터 표현(Semantic Tokens·사용자 스니펫·Format on Type/Paste·Emmet).
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
    - [ ] c-G. Wave G — AI(Inline Edit ⌘I·AI 커밋 메시지). Wave F prod 병합 완료(56712b0 fast-forward).
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
    - [ ] c-H. Wave H — 키맵 엔진(chord ⌘K ⌘S·when 컨텍스트·shift+기호 캡처 — 전면 개정).
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
    - [ ] c-I. Wave I — 셸·워크스페이스(Zen/포커스·멀티 윈도우·설정 파일 탭 편집·플러그인 설치
          UI·VSIX grammar — 최대 구조 변경, 캠페인 최종).
          - [ ] 정찰(wf_c2974bc4-295, opus+high 4축: 레이아웃·윈도우 현행+Zen 재료 / 설정 탭 편집·
                root_guard / 플러그인·VSIX / Tauri 2 멀티윈도우 실태+MVP 재평가) → 하중 재검증 →
                계약(추천안 패키지) → 구현 → 4렌즈 → 적대적 → 수정 → 메인 2차 → 커밋
- [ ] d. 전문 QA — 기능 전수 리스트업 → 체크리스트 신설 → 기능별 심층 검토(opus+xhigh,
      심층은 opus+max) + e2e + **아키텍처·추상화 전수 감사 축**(기존 코드 포함 — 계약 §3.1)
- [ ] e. Phase 8 — 서명·공증 (d 통과 후)
- [x] f. PROCESS.md 아카이브 완료 — 문서화·Phase 0~7.10(W1~W7) 섹션 1,022줄을
      `docs/history/2026-08-14-process-archive-docs-to-w7.md` 로 이전 (1,171줄 → 151줄).
      직전 세션 4개 절(QA6 후속·기능 확장 1~3차)은 HANDOFF 참조라 유지
