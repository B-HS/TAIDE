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
> 뒤에 선언한 등록 effect 의 cleanup(해제)이 dispose 보다 먼저 실행됨 / main.tsx 는
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

(작성 예정)
