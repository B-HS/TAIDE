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

> 구현 Workflow(sonnet+xhigh 단독, TS 전용). §1.1 → §1.2 순차 완료. **재개 세션 메모(정정,
> §4 참고)**: 애초 "직전 세션이 산출물을 남기고 사망 → 이번 세션이 재검증만 수행" 이라고
> 기록했으나, Phase E 렌즈 3(병합 정합성) 재검증 결과 이는 사실이 아니었다 — 직전 세션은
> 죽지 않고 §1.1 뿐 아니라 §1.2(ErrorBoundary 컴포넌트·배선 5곳·로케일 5키·service.rs)까지
> 전부 완주해 정상 종료했다. 실제로는 두 세션이 같은 워킹트리에서 §1.1·§1.2 를 동시에
> 진행했고(오케스트레이터가 선행 세션을 중단으로 오판해 재개 세션을 스폰한 것으로 추정 —
> 재개 세션의 프롬프트 전제가 그대로 이 문서에 굳어졌다), §3.1·§3.2·§1.2 산출물은 선행
> 세션이, §3.3(1px 정렬)과 테스트 커버리지 갱신(`error-boundary.test.tsx` 의
> `componentDidCatch` 케이스 추가)은 재개 세션이 작성했다. 두 세션이 §1.1 의 react-dom
> 소스 정정에 독립적으로 도달하는 등 결론 자체는 일치했으므로 §3.1·§3.2 본문 내용은 그대로
> 둔다.

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
  경로). `ErrorBoundaryBase`(HOC 이전 raw class)도 함께 export(테스트 전용 목적, 아래 3.2
  테스트 항목 참고) — **정정, §4 참고**: 근거로 든 `shared/ui/button.tsx` 의 `Button`+
  `buttonVariants` 선례는 무효였다(`buttonVariants` 는 컴포넌트가 아니라 cva 팩토리라
  fsd.md §3 의 constant/utils 예외에 해당하지, "한 파일 2컴포넌트" 의 선례가 아니다). 실제
  근거는 렌더 하네스(RTL/jsdom) 부재로 `withTranslation()` HOC 를 테스트에서 마운트할 수
  없어 raw class 를 테스트 전용으로 노출한다는 것뿐이며, `ErrorBoundaryBase` 의 소비처는
  테스트 파일 1곳으로 한정된다.
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
  동기(알파벳 정렬 위치 — `editorArea.*` 뒤, `explorer.*` 앞) — **정정, §4 참고**:
  `MESSAGE_NAMESPACES` 자체는 알파벳 정렬 배열이 아니라 도메인 그룹 순서다(JSON 3파일의
  키 위치만 알파벳). `errorBoundary` 튜플은 `prompts` 뒤 배열 말미에 추가됐고, 이는 기존
  append 관행과 일치해 코드 선택 자체는 옳다 — 이 문장의 "알파벳 정렬" 서술만 과대 진술.
  실번역(기계 대치 아님, 문체는
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
- **다른 사이드바 뷰 실사(정정, §4 참고)**: 최초 기록은 `git-panel.tsx` 헤더가 값·내용이
  달라 "같은 행 구조가 아니"고 `search-panel` 계열은 헤더 행이 아예 없다고 판단했으나,
  Phase E 렌즈 3·적대적 재검증 결과 둘 다 부정확했다. `git-panel.tsx` 헤더(`border-b px-2
  py-1.5`)는 값은 다르지만 좌측 합이 33px(내용 최대 20px + py-1.5 12px + border 1px)로,
  좌측 스트립 전체 = `explorer-panel.tsx` 상단 탭리스트 `h-9`(36) + 33 = 69px — 우측
  68px 보다 1px 낮아 사용자가 신고한 files 뷰와 동일한 어긋남이 있었다. `search-panel.tsx`
  는 grep 범위를 `widgets/search-panel` 로만 잡아 놓쳤을 뿐, 실제로는 `features/search/
  search-panel.tsx` 에 tab strip 바로 아래 첫 행(`border-b`)이 있다 — 다만 이 행은
  `flex-col` 로 replace 토글 시 두 번째 입력 행이 늘어나는 가변 높이 컨테이너라 애초에
  32px 고정 대상이 아니다("헤더 행 없음" 이 아니라 "가변 높이 입력 행이라 대상 아님" 이 맞는
  서술). Phase E 조치로 `git-panel.tsx` 헤더도 `h-8 border-b px-2`(`py-1.5` 제거)로
  맞춰 68px 로 정렬했다(§4). `outline-panel.tsx` 는 grep 결과 그대로 헤더 행이 없어 이
  서술이 맞다.

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

## 4. Phase E 검토 반영

> Phase E(4렌즈 opus+xhigh: lifecycle·boundary·merge-integrity·contract) 발견 26건 + 적대적
> 재검증(major 이상 4건, 전부 confirmed) 반영. major·critical 은 전건, minor 는 실질 결함만
> 수정하고 refuted/과대진술 정정은 문서만 고쳤다. 아래는 발견 id 기준 처리 결과다.

### 4.1 critical/major — 전건 수정

- **boundary-1 = contract-1(critical/major, confirmed)** — 루트 경계 폴백이 부팅 크래시에서
  비가시. 검증에서 두 기전이 갈렸다: 메인 창의 `show()` 는 `tauri-plugin-window-state` 가
  `on_window_ready` 에서 자동 수행하므로(2회차 이후 실행) 창 자체는 뜬다 — 남는 문제는
  `body{visibility:hidden}` 게이트(global.css)가 `data-theme-ready`/`data-locale-ready` 둘
  다 `<App/>` 내부(경계의 자식)에서만 세팅돼 크래시 시 영원히 안 붙는 것. 보조 창은 이
  플러그인의 복원 대상에서 아예 제외돼(`with_filter`) `useRevealWindow` 가 유일한 노출
  경로이므로 창 자체가 안 뜨는 문제가 그대로 남는다. **수정**: `ErrorBoundary` 에 옵셔널
  `onCaught` prop 을 추가해 `componentDidCatch` 에서 호출하고, `main.tsx` 의 루트 경계에만
  `document.documentElement.dataset.themeReady/localeReady` 를 세팅 + `getCurrentWindow().
  show()` 를 수행하는 콜백을 연결했다(둘 다 idempotent — 이미 세팅된 값 재대입, 이미 보이는
  창에 `show()` 재호출 모두 무해). `shared/ui` 는 Tauri API 를 직접 import 하지 않는다
  (main.tsx 가 콜백을 주입). **다른 5개 경계에는 연결하지 않음** — `app-shell.tsx`·
  `auxiliary-window-shell.tsx` 의 경계는 전부 `ThemeProvider`/`LocaleProvider` 가 이미
  settle 된 뒤에만 마운트되는 자손이라, 그 경계가 크래시를 잡는 시점엔 두 플래그·창 노출이
  이미 끝나 있다 — `onCaught` 를 연결해도 항상 무해한 재실행일 뿐이라 신호 대 잡음비만
  낮추므로 뺐다(이 판단을 `error-boundary.tsx` JSDoc 에 기록).
- **boundary-2 = merge-integrity-1(major, confirmed)** — 폴백의 `h-full w-full` 고정이
  사이드바(가로 flex, 형제 `main.flex-1` 0px 붕괴)·상태바(세로 flex, 본문 0px 붕괴) 경계에서
  살아남은 형제 영역을 화면에서 지운다. **수정**: `ErrorBoundary` 에 옵셔널
  `fallbackSizeClassName` prop 을 추가(기본값 `h-full w-full` 유지, `cn()` 으로 병합)하고,
  실측 기반으로 두 배치 지점에만 원래 슬롯 치수를 지정했다 — 사이드바 경계
  `fallbackSizeClassName='h-full w-14 shrink-0'`(AppSidebar 실제 클래스와 동일),
  상태바 경계 `fallbackSizeClassName='h-6 w-full shrink-0'`(StatusBar 실제 클래스와 동일,
  `align-items: stretch` 로 폭은 원래도 auto 지만 명시). 나머지 4곳(루트·탐색기 Panel·
  에디터 Panel·보조 창 main)은 원래도 100% 슬롯을 기대하는 자리라 기본값 유지 — Phase E
  검증이 특정한 "5개 중 2개에서만 발생" 범위와 정확히 일치한다. 폴백 좁은 폭(56px)에서
  라벨·설명·버튼 텍스트가 줄바꿈되는 시각적 열화는 남지만, 이는 "형제 영역 소실" 대신
  "폴백 자신의 레이아웃 열화" 로 급이 낮아진 것이라 이번 배치의 "확인된 major 해소 목적의
  최소 prop" 범위 안에서 수용한다 — 이 이상의 폴백 전용 컴팩트 레이아웃은 과설계로 보류.

### 4.2 minor — 실질 결함 수정

- **lifecycle-1(등록 해제 비무조건성)** — 근본 수정: `CodeEditor` 에 `registryTabIdRef` 를
  추가해 현재 등록된 tabId 를 미러링하고, 생성 effect(cleanup 순회에서 이 컴포넌트의 첫
  cleanup) 의 cleanup 최상단에서 `editor.dispose()` 등 어떤 dispose 계열 호출보다 먼저
  `unregisterEditorInstance` 를 실행하도록 재배치했다. React 가 fiber 하나의 destroy 패스
  전체를 단일 try/catch 로 묶는 한(react-dom@19.2.8 확인), 이 순서 재배치가 "선행 cleanup
  throw 시 해제 스킵" 문제를 구조적으로 닫는 유일한 근본 해법이다(대안이던 "각 dispose 호출을
  try/finally 로 감싼다" 는 N 개 호출부를 개별 방어해야 해 더 크고, 미래에 새 dispose 호출이
  추가되면 다시 뚫린다). `[registryTabId]` 효과 자신의 cleanup 은 재키잉 케이스를 위해
  `unregisterEditorInstance` 호출을 유지하되, 전체 언마운트에서는 이미 제거된 키를 다시
  지우는 무해한 중복 호출이 된다. JSDoc 을 "무조건 봉인" 대신 실제 보장 범위(등록은
  구조적으로 항상 live, 해제는 ref 기반 선행 배치로 무조건)로 정정.
- **lifecycle-2(간극 무해 논증 근거 과대)** — lifecycle-1 수정으로 "dispose 후 unregister"
  간극 자체가 사라져(이제 unregister 가 항상 dispose 보다 먼저 실행) 원래 논증이 의존하던
  "다른 fiber 의 create 가 끼어들 수 없다"(StrictMode 이중 호출에서는 거짓인 명제) 를 유지할
  필요가 없어졌다. JSDoc 에서 그 문장을 완전히 제거하고 새 순서(unregister-then-dispose)
  자체를 근거로 서술했다.
- **lifecycle-3(테스트가 불변식을 검증 못함)** — `createDisposableFakeEditor` 에 `isDisposed()`
  를 추가(값이 `{}` 라 dispose 여부가 관측 불가능했던 문제 해소)하고, 해당 테스트를 unregister
  직후·dispose 직전에 `getEditorInstance` 가 이미 `null` 이고 `isDisposed()` 가 아직
  `false` 임을 단언하도록 재작성 — 새 구현의 "unregister 가 dispose 보다 먼저" 순서를
  실제로 고정한다.
- **lifecycle-4 = merge-integrity-4 = contract-7(헬퍼 JSDoc·자기 호출부 모순)** — "잔존 경로"
  describe 의 두 corpse 재등록 호출을 `rerunChildOwnedRegistrationEffect` 대신
  `unregisterEditorInstance`/`registerEditorInstance` 직접 호출로 되돌렸다(신규 헬퍼 도입
  없이 최소 diff). 그 describe 상단 JSDoc 에 "왜 이 헬퍼를 안 쓰는지" 를 명시해 모순을 해소.
- **boundary-3(falsy throw 시 경계 스킵)** — `ErrorBoundaryState` 를 `{ hasError: boolean;
  error: unknown }` 로 분리(`error: Error | null` 대신). `getDerivedStateFromError`/
  `componentDidCatch` 파라미터도 `unknown` 으로 넓혀 `throw 0`/`throw null` 도 정확히
  캐치한다. 스택 노출은 `error instanceof Error && error.stack` 으로 좁힘. 회귀 테스트
  2개 추가(falsy 값에서도 `hasError=true`/폴백 렌더 확인).
- **boundary-5 = contract-2(pre-locale 원시 키 노출)** — theme-provider 의 기존
  `defaultValue` 패턴을 그대로 적용: `description`/`common.retry` 에 영문 기본값을 인라인,
  `labelKey` 는 호출부가 넘기는 옵셔널 `labelFallback` prop 으로 5개 배치 지점 전부에
  영문 기본값을 채웠다(en.json 값과 동일 문구 — Application/Activity Bar/Sidebar Panel/
  Editor/Status Bar).
- **contract-3 = merge-integrity-7(라벨-영역 불일치)** — `errorBoundary.sidebar` 는 실제로
  AppSidebar(액티비티 바)를 감싸므로 텍스트를 "Sidebar"→"Activity Bar" 계열로 정정(en/ko/ja,
  키는 유지 — 배선 변경 없이 표시 문구만 실제 영역에 맞춤). `ExplorerContainer` 경계는
  `explorer.title` 재사용을 그만두고 4뷰(files/search/git/outline) 를 포괄하는 신규 키
  `errorBoundary.sidebarPanel`(Sidebar Panel/사이드바 패널/サイドバーパネル)을 추가해
  라벨링했다. en/ko/ja 3파일 + `MESSAGE_NAMESPACES` 배열 동기, `cargo test` 파리티 재확인.
- **contract-4(shared/ui ESLint ignore)** — `eslint.config.js` 의 `src/shared/ui/**` 전체
  ignore 를 shadcn CLI 생성 파일(함수 선언 스타일이라 재생성 시 다시 위반하는 14개 파일)
  명시 목록으로 좁혔다. 손으로 작성한 5개 파일(error-boundary 2개 포함)이 이제 lint 대상에
  들어가며, 실행 결과 0 error(신규 위반 없음).
- **merge-integrity-2(a)(git 뷰 1px 정렬)** — `git-panel.tsx` 헤더를
  `border-b px-2 py-1.5` → `h-8 border-b px-2` 로 맞춰 좌우 68px 정렬(§3.3 정정 참고).
- **boundary-4 = merge-integrity-3 = contract-5(JSDoc 이중 블록·낡은 파일명/메서드 목록)** —
  두 블록을 하나로 병합해 실제 공개 export 인 `ErrorBoundary`(HOC) 위로 옮기고, 테스트
  전용 노출 사유만 담은 짧은 블록을 `ErrorBoundaryBase` 위에 남겼다. 파일명을
  `error-boundary.test.tsx` 로 정정하고 메서드 목록에 `componentDidCatch`/
  `componentDidMount`/`componentDidUpdate` 를 반영.
- **merge-integrity-6 = contract-6(이중 export 정당화 근거 오류)** — JSDoc·계약 §3.2 모두
  `button.tsx` 선례 인용을 제거하고, 실제 근거(렌더 하네스 부재로 HOC 를 테스트에서 마운트
  불가 → raw class 를 테스트 전용으로만 노출, 소비처 1곳 한정)로 교체했다. 1파일 1컴포넌트
  이탈은 fsd.md §3 의 명시 예외가 아니므로 별도 "허용 예외" 로 이 문서에 기록해 둔다(§4.1
  범위 밖의 관례 예외 아님 — 테스트 인프라 한계에 따른 국소 예외).
- **boundary-7(폴백 포커스 미이관)** — `fallbackRef`(`createRef`) + `tabIndex={-1}` 을
  폴백 컨테이너에 추가하고, `componentDidMount`/`componentDidUpdate` 에서 `hasError` 가
  (마운트 시점부터, 또는 방금) `true` 가 됐을 때 `fallbackRef.current?.focus()` 를 호출한다.
  `role='alert'` 는 원래도 콘텐츠와 함께 삽입되는 것이 WAI-ARIA 권장 패턴(신규 삽입 노드는
  스크린리더가 자동 announce)이라 그대로 유지 — 실제 결함은 포커스 이관 부재였다. 렌더
  하네스 부재로 `.focus()` 실호출은 테스트에서 검증 불가(호출이 throw 하지 않음만 확인).
- **boundary-6(registry 동기 통지의 EditorArea 귀속)** — 재설계 없이 근본 수정 가능한
  범위였다: `editor-instance-registry.ts` 의 `notifyTabListeners` 가 리스너 각각을
  try/catch 로 감싸도록 변경(실패한 구독자는 `console.error` 후 계속) — 한 구독자(예:
  status-bar 의 `attachToEditor` 버그)의 예외가 호출자(CodeEditor, EditorArea 경계 하위)로
  전파돼 무관한 영역이 대신 폴백되는 문제를 근본에서 막는다. 회귀 테스트 1개 추가(구독자
  하나가 던져도 나머지는 통지받고 register/unregister 밖으로 예외가 새지 않음을 확인).

### 4.3 문서 정정만 (코드 결함 아님)

- **merge-integrity-5** — §3 재개 세션 메모의 "직전 세션 사망" 을 실제 사실(동시 이중
  세션·정상 완주)로 정정(위 §3 본문에 반영).
- **contract-8** — §3.2 의 "`MESSAGE_NAMESPACES` 4곳 동기(알파벳 정렬 위치)" 서술 중
  "알파벳 정렬" 부분을 "배열 자체는 도메인 그룹 순서, 튜플은 기존 관행대로 말미 추가" 로
  정정(위 §3.2 본문에 반영). 코드·파리티 테스트는 애초부터 정상.
- **merge-integrity-2(b)** — §3.3 의 "search-panel 헤더 행 없음" 서술을 "가변 높이 입력
  행이라 32px 고정 대상이 아님" 으로 정정(위 §3.3 본문에 반영). 결론(미적용)은 유지.

### 4.4 이월/불변

- refuted 판정(전체 26건 중 major 이상 4건은 confirmed, 나머지 minor 중에서도 이번 검토가
  뒤집은 항목은 없음 — merge-integrity-2/5/8 은 "결론은 맞고 근거 서술만 틀림" 이었다)은
  전부 유지, 임의 재론 없음.
- d-25 부팅 워처 후절화(Rust, 앱 재시작 수반) 는 이번 배치 범위 밖 그대로 이월.

### 4.5 재검증

- 수정 후 `bun run verify`(typecheck → lint → format:check → test → rust:fmt → rust:lint →
  rust:test) exit 0. bun 쪽: typecheck clean, lint 0 error/6 warning(§3.4 와 동일한
  이번 배치 무관 기존 경고), format:check clean, `bun test` **1408 pass/0 fail**(137
  files — boundary-3/onCaught/componentDidMount·Update/registry 리스너 격리 관련 신규
  테스트 6개 추가). cargo 쪽: fmt/clippy(`-D warnings`) clean, `cargo test --workspace`
  **1080 pass/0 fail**(로케일 파리티 3종 포함, `sidebarPanel` 키 추가 후에도 그대로 pass).
- `bunx vite build` exit 0(청크 크기 경고는 기존과 동일, 신규 경고 없음).
- `src/shared/api/bindings.ts` 무변경(`git diff` 0줄) — 이번 반영도 TS(+로케일 JSON/Rust
  상수 1곳) 전용, Tauri 커맨드 표면 무변경.
