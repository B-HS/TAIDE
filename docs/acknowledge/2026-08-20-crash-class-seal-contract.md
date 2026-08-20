# 크래시 클래스 봉인 + ErrorBoundary 방어층 계약 (2026-08-20, d-24)

> 사용자 승인(2026-08-20): "다 추천안으로 해봐 실기는 괜찮아" — **실기 확증 완료**(빈 창
> 미재발) + 결정 3건 전부 추천안 확정: ① ErrorBoundary A안(영역 단위 경계+전체 폴백 병행)
> ③ registry 등록 소유권 A안(CodeEditor 생명주기로 이관). ② 부팅 워처 후절화 A안은 별도
> 배치(d-25 — Rust·앱 재시작 수반이라 후순).
> 선행 정본: `2026-08-20-blank-window-hotfix-contract.md`(§1 확정 사슬·§6 결정 패키지·§7 잔존
> 수정) + 검토 2회전 원문(스크래치패드 hotfix-review-report.json·residual-fix-review-report.
> json — closure-3/design-5 가 이 이관을 공통 제안).
> 착수 전 메인 실물 확인: CodeEditor 소비처 3곳(editor-pane·untitled-pane·app-file-pane) 중
> registry 사용은 editor-pane 경로뿐 / CodeEditor 는 editorRef + onEditorMountRef 구조, 생성
> effect([]) 의 cleanup 에서 dispose / 언마운트 시 effect cleanup 은 선언 역순 — 생성 effect
> 뒤에 선언한 등록 effect 의 cleanup(해제)이 dispose 보다 먼저 실행됨(**정정, §3.1 참고** —
> 설치된 react-dom@19.2.8 소스 직접 확인 결과 실제로는 선언 순서 그대로(정순) 실행된다. 구현
> 결론 자체는 바뀌지 않는다) / main.tsx 는
> StrictMode+createRoot 직행·ErrorBoundary 전무 / app-shell 주요 영역: AppSidebar·
> ExplorerContainer(사이드바 패널)·EditorArea·StatusBarContent(+auxiliary-window-shell).

## 1. 범위

### 1.1 registry 등록 소유권 이관 (결정 ③-A)

- **불변식 목표**: "registry 항목 ≡ live 인스턴스"를 구조로 강제 — 부모(EditorPane)의 stale
  `editor` state 가 시체를 등록할 창 자체를 제거(현재는 렌더 중 조정+fiber 고정+리뷰 전담
  불변식 2개가 대신 지킴).
- CodeEditor 에 **옵셔널 prop**(`registryTabId?: TabId` 계열 — 네이밍 구현 판단) 추가.
  CodeEditor 내부에서 **생성 effect 뒤에 선언**한 `[registryTabId]` effect 가
  `editorRef.current`(항상 자신의 live 인스턴스)를 등록/해제. 언마운트 cleanup 역순으로
  해제가 dispose 에 선행함을 doc 으로 근거화. tabId 변경 시 동일 live 인스턴스 재키잉(T0
  eecb493 의 의도 동작 보존).
- editor-pane 의 `[tabId, editor]` 등록 effect 제거, `registryTabId={tabId}` 전달.
  untitled-pane·app-file-pane 은 prop 미전달로 무변경.
- **동작 등가 실사 필수**: 등록 시점이 부모 effect → 자식 effect 로 이동(자식이 먼저 실행 —
  기존보다 이른 등록·notifyTabListeners 동기 통지 타이밍 변화)이 EditorArea 구독·breadcrumbs·
  status bar 등 registry 소비자에 미치는 영향 전수 실사. StrictMode 이중 effect 에서 등록/
  해제 짝 정합.
- **기존 방어 2종 유지**: `resolveEditorStateForRender` 렌더 중 조정과 프리뷰 분기 fiber
  고정은 **제거하지 않는다** — addAction 재충전 축은 registry 미경유(stale `editor` state
  직접 사용: git-gutter·blame·file-persistence)라 이관으로 봉인되지 않음. 계약·JSDoc 에 두
  장치의 잔존 이유를 명시(hotfix 계약 §2 정정본과 정합).
- 테스트: 기존 등록 계약 테스트를 새 소유권 모델로 갱신 + "tabId 전환·언마운트 시퀀스에서
  registry 가 dispose 된 인스턴스를 갖는 순간이 없다" 하네스 보강.

### 1.2 ErrorBoundary 방어층 (결정 ①-A)

- `shared/ui/error-boundary.tsx` — 재사용 class 컴포넌트(클래스 메서드 축약 문법 — 컨벤션
  허용 예외. 그 외 전부 arrow). props: 영역 라벨(로케일 키)·폴백 커스텀 여지(구현 판단 —
  과설계 금지). "다시 시도" = 경계 state 리셋으로 영역 재마운트.
- **영역 단위 경계 + 전체 폴백 병행**: 루트(App 감싸기 — 최후 폴백) + 주요 영역 경계 배치.
  배치 지점은 실사로 확정(후보: EditorArea·ExplorerContainer/사이드바 패널·StatusBarContent·
  auxiliary-window-shell — 영역 하나의 크래시가 나머지 UI 를 살려두는 구획이 기준). 근거 기록.
- 폴백 UI: 간결(영역명+오류 안내+다시 시도 버튼) — **문자열은 로케일 카탈로그 경유**(신규 키
  3언어 실번역 + MESSAGE_NAMESPACES 4곳 동기). 스택 등 상세 노출은 비프로덕션만(보안 —
  security.md §6).
- 로깅: React 19 의 기본 콘솔 로깅 유지 + 경계 catch 시 console.error 수준(신규 리포팅 체계
  도입 금지 — 최소선).
- FSD: 컴포넌트는 shared/ui, 배선은 app/widgets(위→아래 방향 준수). 1파일 1컴포넌트.

### 1.2b 사이드바 헤더 1px 정렬 (2026-08-20 사용자 실기 보고 — 배치 편입)

- 증상: 탐색기 프로젝트 헤더(MOBINOGI 행) 하단 구분선이 우측(탭바+브레드크럼) 구분선보다
  1px 낮음. 메인 기하 실측: 우측 = h-9(36)+h-8(32)=68px / 좌측 = h-9(36)+헤더(py-1 8+border 1
  +size-6 툴바 버튼 24=33)=69px — **구조적 불일치**(해당 클래스들은 금일 배치 무관 — 헤더 행은
  7.5-C 유래, 기준선 b4e7318 이후 좌우 스트립 클래스 무변경 diff 확인).
- 수정: explorer-panel.tsx 헤더 행을 `h-8` 고정+`py-1` 제거(items-center 가 24px 버튼을 31px
  내용 박스에 정중앙) → 좌우 68px 정렬. 다른 사이드바 뷰(git·search·outline 헤더가 같은 행
  구조를 공유하는지) 실사 후 동일 적용.

### 1.3 범위 외

- d-25 부팅 워처 후절화(Rust — 별도 배치). 커맨드 표면·이벤트·bindings·원격 정책 무변경.
- 에러 리포팅 인프라(Sentry 류)·hotfix 계약의 기결 사항 재론.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, TS 전용): §1.1 → §1.2 순차. git stash/commit 금지·앱 실행
  금지. 종료 전 `bun run verify` + `bunx vite build` exit 0 + bindings 무변경.
- Phase E(4렌즈 opus+xhigh): 정확성(등록 생명주기 — 마운트/언마운트/tabId 전환/StrictMode
  타이밍 전수·경계의 에러 캐치와 재시도 재마운트 동작) / 회귀(registry 소비자 전수·크래시
  방어 2종 잔존 확인·경계 추가로 인한 트리 구조 변화가 state 보존·포커스에 미치는 영향) /
  설계(경계 배치 구획 적정·과설계 여부·prop 설계) / 계약(표면 무변경·i18n 4곳 동기·컨벤션·
  문서 정합) → 적대적(opus+high, major 이상) → confirmed 수정 → 메인 2차 → 커밋(dev) →
  prod 병합.
- 실기 이월(qa6): 영역 크래시 시 해당 구획만 폴백+재시도 복구·나머지 UI 생존(인위 재현은
  실기 몫).

---

## 3. 구현 완료 기록 (Phase E 검토 전)

> 구현 Workflow(sonnet+xhigh 단독, TS 전용). §1.1 → §1.2 순차 완료. **재개 세션 메모**: §1.1
> (registry 이관)은 직전 세션이 산출물을 남기고 사망 → 이번 세션이 재검증만 수행. §1.2 는
> 이 문서의 §3.1·§3.2 를 기록한 세션과 이 세션이 동일 워킹트리에서 동시에 진행했다(작업
> 중 `git status` 가 실시간으로 겹치는 파일 변경을 드러냄 — 별도 `claude` 프로세스가 cwd 를
> 이 저장소에 두고 있었음). 두 세션이 §1.1 의 react-dom 소스 정정에 독립적으로 도달하는 등
> 결론은 일치했으므로 §3.1·§3.2 본문은 그대로 두고, §3.3(1px 정렬)과 테스트 커버리지 갱신
> (`error-boundary.test.tsx` 의 `componentDidCatch` 케이스 추가)만 이 세션이 병합했다.

### 3.1 registry 등록 소유권 이관 — 구현

- `CodeEditor`(code-editor.tsx) 에 옵셔널 prop `registryTabId?: TabId` 추가. **생성 effect
  (`editor.dispose()` 를 cleanup 으로 갖는 `[]` effect) 뒤, 그리고 그 아래 모든 옵션 동기화
  effect 보다도 뒤 — 파일의 마지막 effect**로 `[registryTabId]` effect 를 선언: setup 은
  `editorRef.current`(non-null 보장 — 아래 실사)를 `registerEditorInstance`, cleanup 은
  `unregisterEditorInstance`. `registryTabId` 미전달(untitled-pane·app-file-pane)이면 완전
  무동작.
- `editor-pane.tsx` 의 `[tabId, editor]` 등록 effect(및 그 import)를 통째로 제거하고
  `<CodeEditor ... registryTabId={tabId} />` 로 대체. `resolveEditorStateForRender` 렌더 중
  조정과 §7.1 의 fiber 위치 고정(둘 다 hotfix-contract.md)은 **그대로 유지** — 두 JSDoc 을
  "이 두 장치가 이제 보호하는 대상은 registry 가 아니라, `editor` state 를 registry 없이
  직접 소비하는 `useEditorGitGutterAndConflicts`·`useEditorBlame`·
  `useEditorFilePersistence`·`useEditorViewState`·`useEditorIdeSelection` 다"로 갱신.

**타이밍 등가 실사 (정정 포함)**

1. **선언 순서 실사(설치된 `react-dom@19.2.8` 소스 직접 확인, 추측 아님)**: 이 계약 서문과
   구현 지시 원문은 "언마운트 시 cleanup 은 선언 역순"이라 전제했다. `node_modules/react-dom/
   cjs/react-dom-client.development.js` 의 `commitHookEffectListMount`/
   `commitHookEffectListUnmount`(및 그 유일한 호출부 `commitHookPassiveMountEffects`/
   `commitHookPassiveUnmountEffects`, 전체 서브트리 삭제 경로인
   `commitPassiveUnmountEffectsInsideOfDeletedTree_begin` 포함 전 호출부)를 직접 읽은 결과,
   한 fiber 자신의 여러 `useEffect` 사이에서는 **mount·unmount 모두 선언 순서 그대로(정순)**
   순회한다 — `firstEffect` 부터 `.next` 를 forward 로 도는 하나의 순회 함수를 mount/unmount
   양쪽이 그대로 공유한다. 즉 생성 effect(먼저 선언)의 cleanup(`dispose()`)이 등록 effect
   (나중 선언)의 cleanup(`unregisterEditorInstance`)보다 **먼저** 실행된다 — 서문·구현 지시의
   전제와 정반대다. 이 정정은 §서문에도 표시했다.
2. **그런데도 구현 결론(생성 effect 뒤에 선언)은 그대로 유효**: 근거가 바뀌었을 뿐이다.
   - MOUNT: React 는 같은 커밋의 passive **create** 패스도 동일하게 정순 순회한다
     (`commitHookEffectListMount`) — 이는 애초에 LIFO/FIFO 논쟁의 대상이 아니었다(누구나
     동의하는 부분). 따라서 등록 effect 의 setup 은 항상 생성 effect 의 setup **이후** 실행되고,
     `editorRef.current` 는 등록 시점에 항상 non-null 이 보장된다 — 이 결론은 cleanup 순서
     정정과 무관하게 원래부터 참이었다.
   - FULL UNMOUNT: dispose 가 unregister 보다 먼저 실행돼도 무해하다 — 근거 둘.
     (a) `unregisterEditorInstance` 자체가 `registry.delete()` 를 `notifyTabListeners()` **호출
     전에** 실행하므로, notify 시점엔 이미 항목이 지워져 있다(순서 무관하게 항상 참).
     (b) React 의 passive effect 커밋은 전체 트리에 대해 2단계다 — `flushPassiveEffects` 는
     `commitPassiveUnmountOnFiber`(전체 트리의 모든 destroy)를 먼저 끝낸 뒤에만
     `commitPassiveMountOnFiber`(전체 트리의 모든 create)를 시작한다(같은 함수에서 직접 확인).
     즉 이 컴포넌트 자신의 dispose 와 unregister 사이의 "간극"에서 registry 를 읽는 다른
     컴포넌트의 setup 이 끼어들 수 없다 — registry 소비자의 `getEditorInstance` 호출은 전부
     각자의 effect **setup**(create 패스)에서만 일어나기 때문이다.
   - TABID 재키잉(컴포넌트 생존, `registryTabId` prop 만 변경): 이는 **같은 effect** 자신의
     cleanup-then-setup 이며(React 가 단일 effect 에 대해 항상 보장하는 순서, LIFO/FIFO 논쟁과
     무관), `editorRef.current` 는 이 시나리오에서 dispose 될 일이 없다(생성 effect deps=[]
     는 이 시점에 재실행되지 않음) — 항상 live 인스턴스만 재등록된다.
3. **registry 소비자 전수 실사**: `getEditorInstance`/`subscribeEditorInstance` 호출부는
   `breadcrumbs-bar.tsx`·`editor-area.tsx`(`[focusedFileTabId]` action-id effect, 그리고
   키맵 핸들러의 온디맨드 `getFocusedFileEditor`)·`status-bar-content.tsx` 3곳뿐(grep 전수,
   editor-instance-registry.ts 자체 제외). 셋 다 "setup 시점에 없으면 null, 나중에
   `notifyTabListeners` 로 다시 붙는다"는 패턴을 이미 전제(각 `subscribeToCursor`/
   `attachToEditor` 의 초기 호출 + 구독)하고 있어 — registry 모듈 자신의 JSDoc 도 "Consumers
   that read `getEditorInstance` at effect-setup time can miss a later mount ... subscribing
   closes that gap"라고 명시한다. 이관 후 등록이 EditorPane 의 `editor` state 왕복 커밋 하나를
   건너뛰고 CodeEditor 자신의 생성 커밋에서 즉시 일어나므로(부모 effect → 자식 effect 이동이
   아니라, "부모 state 왕복 필요 → 불필요"로 앞당겨짐), 세 소비자 모두 **더 이른** 시점에
   붙거나(같은 커밋 안에서 setup 자체가 already-registered 상태를 봄) **똑같이 늦게** 붙는다
   (그 소비자가 CodeEditor 보다 트리 상 먼저 커밋되는 형제/조상이면, 원래처럼 자신의 setup 뒤
   `notifyTabListeners` 로 붙는다) — 어느 경우든 "등록 안 된 순간을 관찰"하는 창은 좁아지거나
   그대로이며 새로 생기지 않는다. **문제로 보고할 실사 결과 없음** — 앞의 LIFO/FIFO 전제
   정정 외에는 이관을 막을 발견이 없었다.
4. **StrictMode 이중 effect**: 개발 모드 마운트→언마운트→재마운트 합성 사이클에서, 첫 번째
   가짜 언마운트가 `dispose()`+`unregisterEditorInstance`(등록 해제) 둘 다 실행하고, 두 번째
   진짜 마운트가 **새 monaco 인스턴스**를 만들어 그 인스턴스로 다시 `registerEditorInstance`
   한다 — 매 사이클이 자체적으로 register/unregister 짝을 완결하므로 dangling 등록이 남지
   않는다(2번의 두 근거가 이 사이클에도 동일하게 적용됨).
- **구조적 봉인 확인**: 등록이 이제 `CodeEditor` 자신의 `editorRef.current` 에만 의존하므로,
  disposed 인스턴스가 새 tabId 로 등록되는 코드 경로 자체가 없다(전체 언마운트는 해제만 하고
  뒤따르는 setup 이 없다 — 컴포넌트가 사라졌기 때문). `.getSupportedActions()` 호출부는
  frontend 전체에서 `editor-area.tsx` 1곳뿐이며 전적으로 registry 경유(grep 확인) — 즉 이
  이관은 hotfix-contract.md §7.1(fiber 위치 고정)과 **독립적으로** 이 크래시 클래스를 구조적으로
  닫는다(이중 봉인).
- 테스트: `code-editor-visibility.test.ts` 의 "등록 effect 계약" describe 를 자식 소유 모델로
  재작성(재키잉이 live 인스턴스만 옮긴다 / 전체 언마운트는 해제만 한다 / 재키잉→언마운트
  연쇄에서도 dispose 된 인스턴스가 어떤 tabId 로도 관찰되지 않는다 — 3개). "잔존 경로" describe
  는 삭제하지 않고, 이제 §7.1 과 이 이관 둘로 이중 봉인되었다는 사실을 JSDoc 에 반영(monaco
  자체의 addAction/dispose 재충전 메커니즘 문서로서의 가치는 유지 — git-gutter/blame/
  file-persistence 가 여전히 `editor` state 를 직접 읽으므로).

### 3.2 ErrorBoundary 방어층 — 구현

- `shared/ui/error-boundary.tsx`: class 컴포넌트(`getDerivedStateFromError`/
  `componentDidCatch`/`render` 메서드 축약, `handleRetry` 만 클래스 필드 화살표) +
  `withTranslation()` HOC — class 는 `useTranslation()` 을 쓸 수 없고, `i18next.t()` 직접
  호출은 로케일 변경 시 재렌더를 보장하지 못해 HOC 를 선택(react-i18next 공식 class 통합
  경로). `ErrorBoundaryBase`(HOC 이전 raw class)도 함께 export — `shared/ui/button.tsx` 가
  이미 한 파일에서 `Button`+`buttonVariants` 2개를 export 하는 선례를 따름(테스트 전용 목적,
  아래 3.2 테스트 항목 참고).
  - props: `labelKey`(로케일 키, 사전 해석 문자열 아님)·`children`. 커스텀 폴백 prop 은
    추가하지 않음(과설계 금지 — 이번 배치의 5개 배치 지점 모두 라벨만 다르고 구조는 동일).
  - "다시 시도" = `state.error` 를 `null` 로 리셋. React 가 에러를 캐치한 시점에 이미 던진
    서브트리의 fiber 를 완전히 삭제하므로, 이 리셋에 따른 재렌더는 진짜 재마운트다(재사용되는
    낡은 인스턴스가 없음).
  - 스택 노출은 `import.meta.env.DEV` 게이트(Vite 가 빌드 타임에 정적 치환 — `vite/client`
    타입 포함 확인). `componentDidCatch` 는 `console.error(error, errorInfo)` 만(신규 리포팅
    체계 없음 — React 19 자체 콘솔 로깅과 별개로 존재).
- **경계 배치 근거**(계약 서문이 이미 확정해 둔 후보 목록 그대로 채택 — 과설계 금지로 후보
  밖 추가 없음):
  - 루트: `main.tsx` 에서 `<StrictMode>` 는 유지한 채 그 **안쪽**에서 `<App />` 를 감쌈
    (`labelKey='errorBoundary.app'`) — provider 트리·shell 자체가 던지는 경우까지 잡는 최후
    폴백.
  - `app-shell.tsx`: `AppSidebar`(`errorBoundary.sidebar`)·`ExplorerContainer`
    (`explorer.title` — 기존 키 재사용, 신규 키 미생성)·`EditorArea`
    (`errorBoundary.editorArea`)·`StatusBarContent`(`errorBoundary.statusBar`) 4곳 각각 독립
    경계. 넷 다 "한 영역의 크래시가 나머지를 살려두는 구획"기준을 만족하는 형제 서브트리다
    — 예: 탐색기/git 패널 크래시가 에디터나 상태바를 죽이면 안 되고 역방향도 마찬가지.
    `isProblemsOpen` state 는 `AppShell` 자신이 소유하므로 이 4개 경계 중 하나가 자식을
    캐치해도 다른 경계의 상태와 얽히지 않음(확인).
  - `auxiliary-window-shell.tsx`: 그 위젯이 렌더하는 유일한 주요 영역인 `EditorArea` 에
    동일하게 `errorBoundary.editorArea` 경계.
  - 더 잘게(예: `EditorPane` 개별 탭 단위) 쪼개는 안은 채택하지 않음 — 계약이 열거한 후보
    목록 밖이고, 이번 배치의 "실효 있는 최소 구획" 기준을 넘어선다.
- **로케일 증감**: 신규 5키 — `errorBoundary.app`/`description`/`editorArea`/`sidebar`/
  `statusBar` (`errorBoundary.explorer` 는 만들지 않고 기존 `explorer.title` 재사용,
  `common.retry` 도 기존 키 재사용 — 버튼 라벨). en/ko/ja 3파일 + `MESSAGE_NAMESPACES` 4곳
  동기(알파벳 정렬 위치 — `editorArea.*` 뒤, `explorer.*` 앞), 실번역(기계 대치 아님, 문체는
  기존 `common.retry`/`explorer.title`/설명형 문장 톤에 맞춤). `cargo test` 파리티 3개
  (`내장_3종_로케일은_같은_키_집합을_가진다`·`en_메시지의_모든_키는_required_message_keys에_
  포함된다`·`내장_카탈로그_원문에_중복_키가_없다`) 전부 pass(§3.4 verify 로그).
- 테스트: `shared/ui/error-boundary.test.tsx`(5 pass) — `ErrorBoundaryBase.
  getDerivedStateFromError`(static, 인스턴스 불필요 — React 가 실제로 호출하는 그 진입점을
  직접 호출) / `render()`(에러 없으면 children 그대로, 있으면 라벨·안내·재시도 버튼 구조를
  담은 fallback 반환 — JSX 는 순수 객체 트리이므로 렌더러 없이 구조 검증 가능) /
  `handleRetry`(`setState` 를 스파이로 교체해 `{ error: null }` 호출을 검증 — 마운트되지 않은
  인스턴스에 `setState` 를 직접 걸면 React 의 기본 no-op updater 때문에 `this.state` 가 실제로
  안 바뀌므로, 상태 재확인 대신 호출 인자 검증으로 대체) / **`componentDidCatch`**(재개 세션이
  추가 — 최초 작성 시점 JSDoc 은 이 메서드가 "라이브 DOM/이벤트 시스템 필요"라 검증 불가하다고
  기록했으나, 실제로는 인자 2개를 받아 `console.error` 를 호출하는 것뿐인 순수 인스턴스 메서드라
  렌더러 없이 `new ErrorBoundaryBase(...).componentDidCatch(error, errorInfo)` 직접 호출 +
  `console.error` 스파이로 검증 가능함을 확인해 보완, 파일 상단 JSDoc 도 함께 정정). **여전히
  남은 한계**: `<ErrorBoundary>`(HOC 로 감싼 실제 export)는 hook 을 내부에서 쓰므로 렌더 하네스
  없이 마운트 불가 — 클릭 기반 "크래시 → 폴백 → 재시도 클릭 → children 재마운트" 전체 왕복은
  라이브 DOM/이벤트 시스템이 필요해 이 배치에서 검증 불가(기존 code-editor-visibility.test.ts
  의 RTL 부재 제약과 동일 수위).

### 3.3 사이드바 헤더 1px 정렬 — 구현 (§1.2b)

- `explorer-panel.tsx` 의 `view === 'files'` 헤더 행을 `border-b px-2 py-1` → `h-8 border-b
  px-2`(`py-1` 제거)로 변경. 결과: 좌측 = 상단 탭리스트 `h-9`(36) + 헤더 `h-8`(32) = 68px,
  우측 = 탭바 `h-9`(36) + 브레드크럼 `h-8`(32) = 68px — 정렬. 헤더 내용물(`size-6` 아이콘
  버튼, 24px)은 `items-center` 로 31px 내용 박스(32 − border 1)에 정중앙(맨눈 실측 아님,
  contract §1.2b 의 계산을 그대로 대입 확인).
- **다른 사이드바 뷰 실사**: `git-panel.tsx` 자체 헤더 행(`border-app-border ... border-b
  px-2 py-1.5`)은 값(`py-1.5`)·내용(BranchSwitcher+ahead/behind 뱃지)이 모두 달라 "같은 행
  구조"가 아님 — 7.5-C 레거시 계열이 아닌 git 뷰 고유 디자인이므로 손대지 않음.
  `outline-panel.tsx`·`search-panel` 계열은 grep 결과 tab strip 바로 아래에 오는 자체 헤더
  행 자체가 없음(`border-b` 매치 0건) — 적용 대상 없음. `explorer-panel.tsx` 의
  `view === 'files'` 헤더가 유일한 수정 대상.

### 3.4 검증

- `bun run verify`(typecheck → lint → format:check → test → rust:fmt → rust:lint → rust:test)
  exit 0(재개 세션이 explorer-panel.tsx 1px 수정과 error-boundary.test.tsx 의
  `componentDidCatch` 케이스 추가 이후 재실행). bun 쪽: typecheck clean, lint 0 error/6
  warning(전부 이번 배치 무관 파일 — file-tree.tsx·untitled-pane.tsx·
  use-editor-file-persistence.ts·commit-graph.tsx 의 기존 react-hooks 경고), format:check
  clean, `bun test` **1402 pass/0 fail**(137 files — 신규 `error-boundary.test.tsx` 5개 +
  `code-editor-visibility.test.ts` 갱신분 포함, explorer-panel.tsx 는 스타일 전용 변경이라
  전용 테스트 없음). cargo 쪽: fmt/clippy(`-D warnings`) clean, `cargo test --workspace`
  1080 pass/0 fail(로케일 파리티 테스트 포함).
- `bunx vite build` exit 0(청크 크기 경고는 기존에도 있던 것 — 이번 배치가 새로 만든 경고
  없음).
- `src/shared/api/bindings.ts` 무변경(`git diff` 0줄 확인 — TS 전용 배치, Rust 커맨드 표면
  무변경).
