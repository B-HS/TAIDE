# 실기 사건 핫픽스 — 새 파일 열기 빈 창 크래시 (2026-08-20, d-22)

> 사건: 사용자 실기 보고(2026-08-20) — `bun run tauri dev` 에서 새 파일 클릭 시 UI 전체 소멸
> (빈 창), Reload 로 복구, 다음 새 파일 클릭에서 반복(100% 재현). 콘솔 실측:
> `Error: AbstractContextKeyService has been disposed` + `<EditorArea>` 경고.
> 진단 정본: 광역 wf_14010876-849 + 표적 wf_f99903ef-499(fable+high, **confirmed**) — 원문
> 스크래치패드 `incident-broad-diagnosis.json`·`blank-window-root-cause.json`. 메인 실물
> 재검증 완료(editor-pane.tsx:171-175 blame=eecb493 전건·editor-area.tsx:302-304
> getSupportedActions·code-editor.tsx passive cleanup dispose·렌더 조정 선례 :136-150).

## 1. 확정된 근본 원인

- **기전**: 새 파일 클릭(파일 쿼리 캐시 미스) 커밋에서 EditorPane(무-key)이 placeholder 를
  렌더하며 CodeEditor 를 삭제 — 이때 `editor` useState 는 구 인스턴스를 유지(`setEditor(null)`
  은 삭제된 CodeEditor 의 passive cleanup 에서 뒤늦게 발화). 같은 커밋의 passive 단계에서
  구 에디터 `dispose()` 후, EditorPane 의 `[tabId, editor]` 재등록 effect(editor-pane.tsx:
  171-175)가 **(새 tabId, dispose 된 구 에디터)** 로 재실행되어 registry 에 시체를 등록.
  EditorArea 의 `[focusedFileTabId]` effect(editor-area.tsx:302-304)가 그 시체에
  `getSupportedActions()` 호출 → `InternalEditorAction.isSupported` → dispose 된
  ScopedContextKeyService 의 `contextMatchesRules`(monaco contextKeyService.js:245) throw.
  ErrorBoundary 부재로 React 19 가 루트를 언마운트 — 빈 창. 콘솔 스택의 layout 단계 프레임은
  throw 문맥이 아니라 React 의 에러 로깅 커밋(HostRoot updateQueue 콜백).
- **근원 커밋**: `eecb493`(감사 T0 24항목, 2026-08-18) — 등록을 handleEditorMount 에서
  `[tabId, editor]` 재키잉 effect 로 이관(T0 의 stale registry 수정 자체는 유효하나 dispose
  재등록 사각지대 도입). 소비측(getSupportedActions 경로)은 e3c13bd·Wave H 기존재.
  T1-I·T1-H·T2-I·T2-D/F/G 는 무관 확정(오늘 배치 diff 에 에디터 경로 0 실측).
- **복원·Reload 경로가 안전한 이유**: `editor` state 가 null 에서 시작해 live 인스턴스만 등록.
  캐시된 탭 간 전환도 안전(쿼리 캐시 히트 → CodeEditor 유지 → setModel 스왑 — T0 의 의도
  동작). 크래시는 "이미 에디터가 살아 있는 pane 에서 캐시 미스 신선 열기"의 결합에서만 발생.

## 2. 수정 범위 (근본·최소)

- **editor-pane.tsx 렌더 중 상태 조정**: CodeEditor 를 렌더하지 않는 분기(isPending·isError·
  refused 등 — 실제 JSX 분기 실사로 조건 확정)에서 `editor !== null` 이면 렌더 중
  `setEditor(null)` 조정(동 파일 :136-150 의 기존 adjust-state-during-render 선례와 동형).
  효과: ① 재등록 effect 가 시체를 새 tabId 로 등록하지 않음(구 tabId cleanup 만 실행) ②
  EditorArea 는 null 을 보고 `setActiveEditorActionIds(null)` 안전 종료. viewState 저장은
  무손상(use-editor-view-state 의 layout cleanup 은 이전 렌더 클로저의 live 구 에디터로 같은
  커밋 layout 단계에 실행 — 진단 검증). CodeEditor cleanup 의 뒤늦은 `onEditorMount(null)` 은
  no-op 화.
- **실패 재현 테스트 선행**(버그 수정은 재현이 먼저): "캐시 미스 전환 커밋에서 registry 가
  새 tabId 로 dispose 된 인스턴스를 갖지 않는다"를 드러내는 테스트 — EditorPane 통합이
  무거우면(monaco) 등록 effect 의 상태 패턴을 재현하는 최소 하네스로. 수정 전 FAIL 실측 →
  수정 후 PASS. 불가 판정 시 사유 보고(공허 통과 테스트 금지).
- **기각된 대안**(진단): 등록 effect 의 disposed 검사 — monaco 공개 API 에 isDisposed 부재
  (내부 접근은 우회). 재론 금지.

## 3. 범위 외 (후속 결정·배치)

- **ErrorBoundary 방어층** — 크래시 1건이 앱 전체를 지우는 구조 자체의 방어. 도입 시 에러 UI
  형태(전체 대체 화면 vs pane 단위 경계)가 제품 결정이라 **사용자 결정 패키지로 상신**(§6).
- **초기 기동 ~5초** — 광역 진단 확정분: dev 한정 monaco eager 로드(2~4초) + setup() 동기
  워처 전수 워크(기존 코드). 수정(워처 attach 후절화·React.lazy 경계)은 **별도 배치 제안**(§6).
- eecb493(T0) 계약 문서에의 소급 기록·qa6 실기 재검 항목 추가는 이 배치의 문서 몫.
- tauri-runtime-wry "web content process terminated" 로그 버그(생성 시 오출력) — 외부 의존성,
  기록만.

## 4. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독): 재현 테스트 → 수정 → verify + vite build exit 0 +
  bindings 무변경. 사용자 dev 서버 실행 중(HMR 유의 — 결과 보고에 명시).
- Phase E 검토(4렌즈 opus+xhigh): 정확성(커밋 타임라인 재검증 — 조정이 다른 커밋 경로에서
  live 에디터를 null 화하지 않는지·untitled/diff/preview 분기) / 회귀(viewState 저장·복원·
  캐시 탭 전환·멀티 pane·hot-exit 경로 무손상) / 설계(render-phase 조정 적정·선례 정합) /
  계약(표면 무변경). major 이상 적대적(opus+high) → confirmed 수정 → 메인 2차 → 커밋 →
  prod 병합.
- 실기 확증(사용자): 수정 후 새 파일 클릭 반복 — 빈 창 미재발 확인이 최종 판정.

---

## 5. 구현 완료 기록 (2026-08-20, Phase E 검토 전)

> 구현 wf_6ed8bbd4-d50(단독 sonnet+xhigh). 메인 2차: diff 실물 정독(조정 위치·가드·JSDoc 근거
> 진단 제안과 일치)·verify + vite build 직접 재실행 exit 0·재현 테스트 5 pass 직접 재실행·
> bindings 무변경.

- **수정**: `canRenderCodeEditor(isPending, isError, tier)` 순수 predicate 를
  `code-editor-visibility.ts` 로 추출(conflict-status.ts 선례 동형 — 렌더 조건과 테스트가
  단일 출처 공유). editor-pane.tsx 의 3개 early-return(isPending :266·isError :268·
  tier==='refused' :276 — JSX 실사로 미렌더 분기가 정확히 3개임을 확정, 마크다운 프리뷰 split
  포함 이후 경로는 전부 CodeEditor 렌더) 직전에 렌더 중 조정 1줄:
  `if (!canRenderCodeEditor(...) && editor !== null) setEditor(null)`. untitled-pane 은 동일
  state 패턴이나 editor-instance-registry 미참조로 무관 확인(무수정).
- **재현 테스트(수정 전 FAIL 실측)**: RTL 부재(기존 3개 테스트 파일이 동일 제약 문서화 —
  신규 패키지 금지)로 컴포넌트 렌더 재현 불가 판정 → **실제 editor-instance-registry 모듈**로
  cleanup-then-setup 커밋 시퀀스를 모델링한 데이터 레벨 재현. 수정 전: corpse 가 새 tabId 로
  등록됨(FAIL 실측 — 4 pass/1 fail). 수정 후: 5 pass. **명시된 한계**: editor-pane 내부의
  조정 1줄 배선 자체가 삭제되는 회귀는 이 테스트가 못 잡음(렌더 하네스 부재 — 기존 선례와
  동일 성격, 타입체크·리뷰로 보증).
- **부작용 분석(구현 자기 검증)**: 조정은 1회성(재렌더 후 editor=null 로 조건 해소 — 무한루프
  없음) / viewState layout cleanup 은 이전 렌더 클로저의 live 에디터로 무손상(진단 H1 기각과
  일치) / 캐시 히트 탭 전환은 predicate true 유지로 무접촉(T0 setModel 스왑 의도 보존) /
  CodeEditor cleanup 의 늦은 setEditor(null) 은 no-op 화.
- **검증**: verify exit 0(bun 1389·cargo 전량)·vite build exit 0·bindings 무변경·신규
  경고/에러 0.

## 6. 사용자 결정 패키지 (완료 보고 시 상신)

1. ErrorBoundary 방어층 도입 여부·형태 — A안(추천): pane/영역 단위 경계+전체 폴백 병행 /
   B안: 루트 1개만 / C안: 도입 보류
2. 부팅 5초 수정 배치 — A안(추천): 워처 attach 후절화(창 표시 뒤 spawn) 배치 착수 /
   B안: release 빌드 실측 후 판단 / C안: 보류
