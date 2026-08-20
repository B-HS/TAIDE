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
- **addAction 무가드 재충전 고리(Phase E 재조정 확정, 2026-08-20)**: `dispose()`(monaco
  `codeEditorWidget.js`:296-304)는 `_actions.clear()` 를 `super.dispose()`(contextKeyService
  파기) **이전**에 실행하므로, dispose 직후 `getSupportedActions()` 는 빈 배열을 반환하고
  throw 하지 않는다 — Phase E 검토의 verify 패스가 correctness-1 을 `refuted` 로 판정한
  근거가 이것이다. 그러나 같은 커밋의 passive **create** 패스에서
  `useEditorGitGutterAndConflicts` 의 addAction 효과 2개
  (use-editor-git-gutter-and-conflicts.ts:266-275 "stage selection"·:279-287 "file history" —
  둘 다 가드는 `if (!editor) return` 뿐)가 재실행되면(`path` 의존성이 같은 커밋에서 `tabId` 와
  함께 바뀌는 경우), disposed 여부와 무관하게 `editor.addAction(...)`(monaco
  `standaloneCodeEditor.js`:98-149, disposed 가드 없음)을 호출해 시체의 `_actions` 를
  **재충전**한다. 재충전된 액션은 dispose 된 `_contextKeyService` 를 그대로 캡처하므로
  이후 `getSupportedActions()`(`codeEditorWidget.js`:772-776) →
  `InternalEditorAction.isSupported()`(`editorAction.js`:15-17) →
  `contextMatchesRules`(`contextKeyService.js`:243-245)에서 다시 throw 한다. 이 재충전 고리는
  재조정(wf_fd1c5cf8-472, confirmed)이 발견 — Phase E 의 correctness-1 `refuted` 판정은
  dispose 단독 효과만 반증했을 뿐 이 재충전 고리를 몰랐던 오판으로 **전복**되었다(메인 실물
  재검증 완료). §7 참고.

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
- **기각된 대안**(진단, Phase E design-4 정정 반영 — 2026-08-20): 등록 effect 의 disposed
  검사. `isDisposed` 프로퍼티는 monaco 공개 API 에 없으나(정정: "내부 접근 없이는 검출
  불가능"이라던 원 서술은 부정확 — `onDidDispose(listener)` 는 `IEditor` 가 노출하는 공개
  이벤트다, `editor.api.d.ts`:2676), 검출이 가능하더라도 그 검사는 이 배치가 고친 세
  early-return 분기의 증상만 막을 뿐 §1 의 근본 원인(dispose 재등록 자체·addAction 재충전
  고리)은 남긴다. 이번 배치는 그 대신 구조적 대안(§7 — `CodeEditor` 의 fiber 위치 고정)을
  채택했으므로, 등록 effect 가드 대안은 재평가하지 않는다.

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
  `if (!canRenderCodeEditor(...) && editor !== null) setEditor(null)`(잔존 경로 수정 배치에서
  `resolveEditorStateForRender` 로 추출·치환 — §7). untitled-pane 은 동일 state 패턴이나
  editor-instance-registry 미참조로 무관 확인(무수정).
- **재현 테스트**: RTL 부재(기존 3개 테스트 파일이 동일 제약 문서화 — 신규 패키지 금지)로
  컴포넌트 렌더 재현 불가 판정 → **실제 editor-instance-registry 모듈**로 cleanup-then-setup
  커밋 시퀀스를 모델링한 데이터 레벨 재현. **정정(Phase E design-2·contract-1 검토 반영,
  2026-08-20)**: 원 기록의 "수정 전 FAIL 실측"은 개발 중 임시로 편집한 테스트 변형본 기준
  서술이었다 — **커밋된 테스트 산출물** 자체는 조정 로직(`canRenderCodeEditor(...) ? ... :
  null`)을 테스트 본문 안에 재구현해 두었을 뿐 editor-pane.tsx 를 참조하지 않으므로, "수정
  전" 상태에서 실행해도 항상 5 pass 하는 자기충족 항진명제였다(`editor-pane.tsx` 의 조정 1줄을
  지워도 그대로 통과 — 직접 실행 확인). 잔존 경로 수정 배치(§7)에서 테스트를
  `resolveEditorStateForRender`(신규 추출, editor-pane.tsx 가 실제로 호출하는 그 함수)를
  직접 호출하도록 재작성해, 결정 로직 자체는 프로덕션과 동일 함수를 공유하게 했다 — 다만
  editor-pane.tsx 의 배선(`if (resolvedEditor !== editor) setEditor(resolvedEditor)` 이 그
  반환값을 실제로 쓰는지) 자체는 여전히 렌더 하네스 부재로 타입체크·리뷰 전담이다. **명시된
  한계**: 배선 삭제 회귀는 이 테스트가 못 잡음.
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

---

## 7. 잔존 경로 근본 수정 (2026-08-20, Phase E 검토 반영 배치)

> Phase E 4렌즈 검토(hotfix-review-report.json) 의 critical finding correctness-1(초기
> verify 패스는 `refuted` 로 판정했으나 재조정으로 전복 — §1 참고)이 지목한 잔존 크래시
> 경로를 구조적으로 닫는다. 대상: editor-pane.tsx(마크다운 프리뷰 분기)·
> use-editor-lsp-integration.ts(게이트 중복)·code-editor-visibility.test.ts(테스트 배선·
> 명칭)·이 계약 문서(§1·§2·§5 정정).

### 7.1 근본 수정 — CodeEditor 의 fiber 위치 고정

- **문제**: editor-pane.tsx 의 마지막 슬롯 삼항(`isMarkdown && showMarkdownPreview ? <Group>...
  </Group> : codeEditorWithBlameFooter`)은 프리뷰 on/off 에 따라 `<Group>` ↔ 일반 `<div>` 로
  **엘리먼트 타입이 바뀐다.** React 는 같은 슬롯의 엘리먼트 타입이 바뀌면 그 서브트리를
  삭제 후 재마운트하므로, 프리뷰 토글이나(또는 `isMarkdown` 이 바뀌는 탭 전환) `CodeEditor` 를
  매 번 삭제+재생성했다. `canRenderCodeEditor` 는 이 커밋 내내 true(파일 로드 완료·정상
  tier)이므로 `resolveEditorStateForRender` 의 render-phase 조정은 발화하지 않고, `editor`
  state 는 (이미 dispose 된) 구 인스턴스를 유지한 채 `[tabId, editor]` 등록 effect 가 새
  tabId 로 그 시체를 재등록한다 — §1 의 addAction 재충전 고리와 결합하면 원래 핫픽스가
  막으려던 것과 동일한 `AbstractContextKeyService has been disposed` throw 로 이어진다.
- **수정**: `<Group orientation='horizontal'>`+에디터 `<Panel id={\`${tabId}-editor\`}>` 를
  **항상** 렌더하고, 프리뷰 `<PaneSeparator>`+`<Panel id={\`${tabId}-preview\`}>` 만
  `isMarkdown && showMarkdownPreview` 로 조건부 추가한다(`editorAndPreviewPanels` 변수로
  추출, `return` 앞에서 조립). `<Group>` 의 첫 자식(`<Panel id="...-editor">`)은 프리뷰
  유무와 무관하게 항상 같은 위치·같은 타입이므로, React 재조정이 그 서브트리를 절대
  삭제하지 않는다 — `CodeEditor` 는 프리뷰 토글·markdown/non-markdown 탭 전환 어느
  쪽으로도 더 이상 언마운트되지 않는다(부수 효과로 monaco 재생성·viewState 왕복 비용도
  사라진다).
- **동등성 실사(react-resizable-panels 4.12.2, `node_modules/react-resizable-panels/dist/
  react-resizable-panels.js` 직접 확인)**: `<Panel>` 하나만 등록되고 `<PaneSeparator>` 가 없는
  `<Group>` 은 `defaultSize` 값과 무관하게 100% 너비로 정규화된다 — 초기 레이아웃 계산
  `We()`(:1340-1357)는 `defaultSize` 가 있는 패널의 값을 그대로 합산하지만(1개 패널·
  `defaultSize='50%'` → 합계 50), 그 뒤 `K()`(:837-885)가 `if (!k(i, 100) && o.length > 0)`
  분기(:849-855, `l = 100 / i * r`)로 합계가 100 이 아니면 100 이 되도록 **재정규화**한다 —
  1개 패널·합계 50 → `100/50*50 = 100`. 이는 프리뷰가 열려 다른 패널이 함께 등록될 때만
  `defaultSize='50%'` 가 실제 50% 로 유지되고, 프리뷰가 닫혀 패널이 하나뿐이면 자동으로
  100% 가 됨을 뜻한다 — 기존 bare `<div>` 분기와 시각적으로 동일. 리사이즈 핸들도 동일하게
  없다(`<PaneSeparator>` 미등록 → `registerSeparator` 호출 없음 → 드래그 히트 영역 없음).
  `<Group>` 자체의 `className='min-h-0 min-w-0 flex-1'` 은 프리뷰 on 분기에서 이미 검증된
  "flex-col 부모의 남는 세로 공간을 채우는" 방식이며(`flex-1` = flex-grow:1 로 정석적인 채움),
  이전 bare `<div>` 분기가 쓰던 `h-full`(+`min-h-0`, flex-shrink 경유의 채움)과는 메커니즘만
  다를 뿐 결과(remaining space 를 채움)는 동일하다 — 두 메커니즘 모두 이 프로젝트에 이미
  나란히 존재해 왔으므로(프리뷰 on/off 각각), 이번 변경은 이미 검증된 한쪽 메커니즘으로
  **통일**하는 것이다.
- **DOM 구조 변화**: 프리뷰가 꺼진 상태에서 `codeEditorWithBlameFooter` 를 감싸는 wrapper 가
  0 개(이전, bare `<div>` 가 곧 그 자식)에서 3 개(`<Group>` div·`<Panel>` outer div·`<Panel>`
  inner div)로 늘어난다. editor-pane.tsx·editor-area.tsx·features/editor 전역 grep 확인 —
  이 DOM 위치에 의존하는 `querySelector`/`closest` 등 구조 결합 코드는 없다.

### 7.2 잔존 경로 재현 테스트

- **선택**: 컴포넌트 렌더(RTL) 를 이 저장소가 지원하지 않아(§5 와 동일 제약) editor-pane.tsx
  의 JSX 를 직접 구동하는 수정 전/후 FAIL/PASS 쌍은 불가능 — **사슬 고정(chain-lock) 테스트**로
  대체(code-editor-visibility.test.ts 확장). 실제 monaco 소스(`monaco-editor` 0.56.0,
  `node_modules` 직접 확인)에 맞춘 동형 모의로 재충전 메커니즘 자체를 고정한다:
  `standaloneCodeEditor.js`:98-149 `addAction`(disposed 가드 없음) · `codeEditorWidget.js`:
  296-304 `dispose`(`_actions.clear()` 가 `super.dispose()` 보다 먼저) · `codeEditorWidget.js`:
  772-776 `getSupportedActions`(`isSupported()` 로 filter) · `editorAction.js`:15-17
  `InternalEditorAction.isSupported` · `contextKeyService.js`:243-245 `contextMatchesRules`
  의 disposed throw.
- **두 테스트**: ① dispose 만으로는 `getSupportedActions()` 가 `[]` 를 반환하고 throw 하지
  않음을 확인(Phase E verify 패스의 `refuted` 근거가 실제로 성립함을 증명) → ② 같은 커밋에서
  git-gutter 의 addAction 효과 2개가 재실행되면(가드 `!editor` 만) 시체가 재충전되어
  `getSupportedActions()` 가 `AbstractContextKeyService has been disposed` 로 throw 함을
  확인(재조정이 전복한 실제 메커니즘). 두 테스트 모두 수정 전/후에 걸쳐 항상 같은 결과를
  내는 메커니즘 고정 테스트이며(editor-pane.tsx 를 참조하지 않음), 7.1 의 구조적 수정 자체에
  대한 회귀 가드는 아니다 — 그 수정은 타입체크·리뷰·이 문서로 보증한다(§5 와 동일한 성격의
  한계, 명시).
- **결과**: `bun test src/widgets/editor-pane/code-editor-visibility.test.ts` = 11 pass / 0
  fail(기존 5 개 중 등록-effect 계약 테스트 1개는 `resolveEditorStateForRender` 직접 호출로
  갱신 · 신규 6개: `canRenderCodeEditor` tier-null 테스트 1개 + `resolveEditorStateForRender`
  단위 테스트 3개 + 사슬 고정 테스트 2개).

### 7.3 검토 minor 일괄 반영

- **correctness-2/regression-1/design-3 (LSP 게이트 중복)**: `use-editor-lsp-integration.ts`
  의 `useLspSession({ enabled: !isPending && !isError && tier !== 'refused' })` 를
  `useLspSession({ enabled: canRenderCodeEditor(isPending, isError, tier) })` 로 치환.
  `canRenderCodeEditor` 의
  `tier` 파라미터를 `FileSizeTier | undefined` → `FileSizeTier | null | undefined` 로 확장해
  `useEditorLspIntegration` 이 전달하는 `tier: FileSizeTier | null` 을 그대로 받는다(호출부
  변환 불필요, `tier !== 'refused'` 비교는 `null`/`undefined` 모두 통과이므로 동작 동일 —
  신규 단위 테스트로 고정).
- **design-1 (JSDoc 과대 서술)**: editor-pane.tsx 의 render-phase 조정 JSDoc 에서 "crashing
  past any error boundary"(어떤 경계도 통과) → "경계가 없어 React 19 가 루트를 언마운트한다
  (`createRootErrorUpdate`)"로 정정. "that stale, about-to-be-disposed instance" →
  "already-disposed instance"(재등록 시점엔 같은 destroy 패스에서 이미 dispose 완료)로 정정.
  같은 JSDoc 에 §7.1 의 잔존 경로·resolveEditorStateForRender 의 커버 범위 한계를 추가.
- **correctness-3/regression-2/design-2/contract-1 (테스트 배선·명칭)**: describe 명칭을
  "회귀"에서 "등록 effect 계약" 수준으로 낮추고, 조정 규칙을 `resolveEditorStateForRender`
  순수 함수로 추출해 editor-pane.tsx 와 테스트가 **같은 함수**를 호출하게 함(§5 정정 참고).
- **contract-2 (라인 참조 stale)**: 테스트 JSDoc 의 `editor-pane.tsx :171-175` 라인 참조를
  향후에도 밀릴 값이므로 라인 번호 대신 심볼 설명("`[tabId, editor]` 등록 effect")으로
  치환.
- **design-4**: §2 기각 사유 정정(위 참고).
- **contract-3 (PROCESS.md 상태 낙후)**: 이번 배치의 지정 범위(correctness-2/regression-1/
  design-3·design-1·correctness-3/regression-2/design-2/contract-1·contract-2·design-4)
  밖 — `docs/PROCESS.md` d-22 갱신은 미반영 상태로 남아 있음을 기록만 한다.

### 7.4 검증

- typecheck(`tsc --noEmit`) clean. `bunx eslint`(대상 4파일) clean. `bunx prettier --check`
  (대상 4파일) clean.
- `bun test src/widgets/editor-pane/ src/widgets/editor-area/ src/entities/editor/
  src/features/editor/` = 120 pass / 0 fail.
- 전체 `bun run verify`(typecheck→lint→format:check→test→rust:fmt→rust:lint→rust:test) +
  `bunx vite build` + Tauri bindings 무변경은 이 배치의 완료 보고에 실측 기록.
