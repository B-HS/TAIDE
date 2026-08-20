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
  구 에디터 `dispose()` 후, EditorPane 의 `[tabId, editor]` 재등록 effect가
  **(새 tabId, dispose 된 구 에디터)** 로 재실행되어 registry 에 시체를 등록.
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
  `setEditor(null)` 조정(동 파일의 `path !== syncedPath` 렌더 중 조정 블록과 동형인 기존
  adjust-state-during-render 선례).
  효과: ① 재등록 effect 가 시체를 새 tabId 로 등록하지 않음(구 tabId cleanup 만 실행) ②
  EditorArea 는 null 을 보고 `setActiveEditorActionIds(null)` 안전 종료. viewState 저장은
  무손상(use-editor-view-state 의 layout cleanup 은 이전 렌더 클로저의 live 구 에디터로 같은
  커밋 layout 단계에 실행 — 진단 검증). CodeEditor cleanup 의 뒤늦은 `onEditorMount(null)` 은
  no-op 화.
- **실패 재현 테스트 선행**(버그 수정은 재현이 먼저): "캐시 미스 전환 커밋에서 registry 가
  새 tabId 로 dispose 된 인스턴스를 갖지 않는다"를 드러내는 테스트 — EditorPane 통합이
  무거우면(monaco) 등록 effect 의 상태 패턴을 재현하는 최소 하네스로. 수정 전 FAIL 실측 →
  수정 후 PASS. 불가 판정 시 사유 보고(공허 통과 테스트 금지).
- **기각된 대안**(진단, Phase E design-4·contract-1 정정 반영 — 2026-08-20): 등록 effect 의
  disposed 검사. `isDisposed` 프로퍼티는 monaco 공개 API 에 없으나(정정: "내부 접근 없이는
  검출 불가능"이라던 원 서술은 부정확 — `onDidDispose(listener)` 는 `IEditor` 가 노출하는 공개
  이벤트다, `editor.api.d.ts`:2676), 검출 자체는 가능하고 그 검사를 두면 §1·§7.1 이 낸 시체
  모두 registry 에 등록되는 것을 실제로 막는다(**정정, contract-1**: "분기 증상만 막고 dispose
  재등록 자체는 남긴다"던 원 기각 사유는 그 검사의 목적과 모순되어 성립하지 않았으므로 폐기).
  그래도 채택하지 않는 진짜 이유는, 그 검사가 registry 를 경유하는 소비자의 위생만 고칠 뿐
  "등록된 인스턴스 ≡ 현재 마운트된 살아있는 인스턴스" 라는 근본 불변식 자체는 강제하지 않기
  때문이다 — registry 를 거치지 않고 `editor` state 를 직접 읽는 소비자(예:
  use-editor-file-persistence.ts 의 `editor?.getAction(FORMAT_DOCUMENT_ACTION_ID)`)에는 여전히
  시체가 노출될 수 있고, 그 틈을 마저 막으려면 `onDidDispose` 구독이라는 별도 부기 상태를
  effect 에 더해야 한다. 이번 배치는 그 대신 구조적 대안(§7 — `CodeEditor` 의 fiber 위치 고정으로
  시체가 생기는 커밋 자체를 없앰)을 채택했으므로, 등록 effect 가드 대안은 재평가하지 않는다.

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
  단일 출처 공유). editor-pane.tsx 의 3개 early-return(`isPending`·`isError`·
  `tier === 'refused'` — JSX 실사로 미렌더 분기가 정확히 3개임을 확정, 마크다운 프리뷰 split
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
  반환값을 실제로 쓰는지) 자체는 여전히 렌더 하네스 부재로 **리뷰 전담**이다(**정정, Phase E
  design-3**: "타입체크·리뷰 전담"이라던 원 서술은 부정확 — 그 조정 2줄과
  `resolveEditorStateForRender` import 를 함께 지워도 남는 미사용 심볼이 없어 `tsc --noEmit`·
  eslint 모두 clean 하다. 실제 보증은 리뷰뿐이다). **명시된 한계**: 배선 삭제 회귀는 이
  테스트가 못 잡음.
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
3. registry 등록 소유권을 `CodeEditor` 자신의 생명주기로 이관할지(Phase E closure-3=design-5) —
   §7.1 의 fiber 위치 고정은 알려진 트리거 하나만 닫았을 뿐, "registry 에 등록된 인스턴스는
   살아 있다"는 불변식 자체를 강제하는 장치는 여전히 없다 — A안(추천): 등록 effect 안에서
   `editor.onDidDispose(() => unregisterEditorInstance(tabId))` 를 걸어(공개 API, §2 정정본이
   인정) 값싸게 불변식을 강제 / B안: 등록·해제 자체를 `CodeEditor` 자신의 마운트 effect 로
   이관해 부모의 `editor` state 를 거치지 않게 구조를 바꿈(더 큰 리팩터) / C안: 보류(§7.1 이
   알려진 트리거를 이미 닫았으므로 당장은 현상 유지 — 단, §7.1 의 "`CodeEditor` 를 감싸는
   조건부 렌더를 추가하지 않는다"는 불변식을 리뷰가 계속 지켜야 함)

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
  사라진다). **불변식**: 이 구조가 성립하려면 `CodeEditor` 와 `<Group>` 사이에 새 조건부
  렌더를 추가하지 않아야 한다 — 그런 분기가 새로 생기면 이 절이 닫은 크래시 클래스가 똑같이
  재개방된다(Phase E closure-3=design-5, §6 결정 패키지 3번 참고. 자동 가드는 없음 — 리뷰가
  이 불변식을 지킨다).
- **동등성 실사(react-resizable-panels 4.12.2, `node_modules/react-resizable-panels/dist/
  react-resizable-panels.js` 직접 확인, Phase E 잔존 검토 반영 — §7.5)**: 에디터 `<Panel>` 은
  `defaultSize` 를 선언하지 않는다 — 초기 레이아웃 계산 `We()`(:1340-1357)는 `defaultSize` 가
  **있는** 패널에는 그 값을 그대로 배정하고, `defaultSize` 가 **없는** 패널에는 "100 − 선언된
  합" 을 나눈다. 프리뷰가 닫혀 에디터 패널이 그룹의 유일한 패널이면 선언된 합이 0 이므로
  에디터 패널이 100% 를 그대로 받고, 프리뷰가 열리면(프리뷰 `<Panel>` 은 여전히
  `defaultSize='50%'`) 선언된 합이 50 이므로 에디터 패널이 나머지 50% 를 받는다 — 두 경우 모두
  기존 bare `<div>` 분기·기존 프리뷰-on `<Group>` 분기와 각각 시각적으로 동일하며, 합계가
  이미 100 이므로 `K()`(:837-885)의 합계-100 재정규화 단계에 결과가 의존하지 않는다(같은
  저장소 선례: editor-area.tsx 의 `<Panel id='editor-panes'>` 도 `defaultSize` 없음). 리사이즈
  핸들도 프리뷰 닫힘 상태에선 동일하게 없다(`<PaneSeparator>` 미등록 → `registerSeparator`
  호출 없음 → 드래그 히트 영역 없음). `<Group>` 자체의 `className='min-h-0 min-w-0 flex-1'`
  은 프리뷰 on 분기에서 이미 검증된 "flex-col 부모의 남는 세로 공간을 채우는" 방식이며
  (`flex-1` = flex-grow:1 로 정석적인 채움), 이전 bare `<div>` 분기가 쓰던 `h-full`(+`min-h-0`,
  flex-shrink 경유의 채움)과는 메커니즘만 다를 뿐 결과(remaining space 를 채움)는 동일하다 —
  두 메커니즘 모두 이 프로젝트에 이미 나란히 존재해 왔으므로(프리뷰 on/off 각각), 이번 변경은
  이미 검증된 한쪽 메커니즘으로 **통일**하는 것이다. `overflow` 는 프리뷰가 닫혀 있을 때만
  `<Group>`·에디터 `<Panel>` 양쪽에 `style={{ overflow: 'visible' }}` 로 되돌려 pane 경계
  클리핑을 복원한다(Phase E regression-1, §7.5 참고) — 프리뷰가 열려 있을 때는 스타일
  오버라이드 없이 라이브러리 기본값 그대로다.
- **DOM 구조 변화**: 프리뷰가 꺼진 상태에서 `codeEditorWithBlameFooter` 를 감싸는 wrapper 가
  0 개(이전, bare `<div>` 가 곧 그 자식)에서 3 개(`<Group>` div·`<Panel>` outer div·`<Panel>`
  inner div)로 늘어난다. editor-pane.tsx·editor-area.tsx·features/editor 전역 grep 확인 —
  이 DOM 위치에 의존하는 `querySelector`/`closest` 등 구조 결합 코드는 없다.

### 7.2 잔존 경로 재현 테스트

- **선택**: 컴포넌트 렌더(RTL) 를 이 저장소가 지원하지 않아(§5 와 동일 제약) editor-pane.tsx
  의 JSX 를 직접 구동하는 수정 전/후 FAIL/PASS 쌍은 불가능 — **사슬 고정(chain-lock) 테스트**로
  대체(code-editor-visibility.test.ts 확장). 실제 monaco 소스(`monaco-editor` 0.56.0,
  `node_modules` 직접 확인)에 맞춘 동형 모의로 재충전 메커니즘을 **문서화**한다(정정, Phase E
  design-1·contract-2 — 아래 한계 참고):
  `standaloneCodeEditor.js`:98-149 `addAction`(disposed 가드 없음) · `codeEditorWidget.js`:
  296-304 `dispose`(`_actions.clear()` 가 `super.dispose()` 보다 먼저) · `codeEditorWidget.js`:
  772-776 `getSupportedActions`(`isSupported()` 로 filter) · `editorAction.js`:15-17
  `InternalEditorAction.isSupported` · `contextKeyService.js`:243-245 `contextMatchesRules`
  의 disposed throw.
- **두 테스트**: ① dispose 만으로는 `getSupportedActions()` 가 `[]` 를 반환하고 throw 하지
  않음을 확인(Phase E verify 패스의 `refuted` 판정이 이 모의 위에서 성립함을 확인) → ② 같은
  커밋에서 git-gutter 의 addAction 효과 2개가 재실행되면(가드 `!editor` 만) 시체가 재충전되어
  `getSupportedActions()` 가 `AbstractContextKeyService has been disposed` 로 throw 함을
  확인(재조정이 전복한 실제 메커니즘). 두 테스트 모두 수정 전/후에 걸쳐 항상 같은 결과를
  내는 메커니즘 고정 테스트이며(editor-pane.tsx 를 참조하지 않음), 7.1 의 구조적 수정 자체에
  대한 회귀 가드는 아니다 — 그 수정은 리뷰로만 보증한다(**정정, Phase E design-3**: "타입체크·
  리뷰"라던 원 서술은 부정확 — 7.1 의 구조 변경은 삭제해도 미사용 심볼을 남기지 않으므로
  `tsc --noEmit`·eslint 어느 쪽도 잡지 못한다. §5 와 동일한 성격의 한계, 명시). **추가 한계**
  (정정, Phase E design-1·contract-2): 이 두 테스트가 실제로 검증하는 대상은 같은 파일 안에
  손으로 쓴 `createRechargeableFakeEditor` 모의뿐이다 — monaco 를 런타임으로 전혀 로드하지
  않으므로, 위에 나열한 monaco 실물 사실이 이후 버전에서 바뀌어도(예: `addAction` 에 disposed
  가드가 추가되거나 `dispose` 의 정리 순서가 바뀌어도) 이 테스트는 계속 통과한다 — "메커니즘을
  증명"하는 것이 아니라 "동형 모의로 사슬을 문서화"하는 것이며, monaco 실물이 이 문서의 전제와
  계속 일치하는지는 리뷰가 보증한다(§5 의 자기충족 시인과 같은 수위의 한계).
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

### 7.5 잔존 경로 검토 2차 반영 (Phase E, 2026-08-20 — residual-fix-review-report.json)

> §7 배치(커밋 `2c8f0e5`)에 대한 4렌즈+major 적대적 검토 17건(major 1 confirmed + minor 16,
> refuted 0)의 반영. 아래 id 는 그 보고서의 id 이며, §7.3 이 참조하는 이전 회차(0f7b07b 검토)
> id 와 이름이 겹치는 항목(design-1·design-2·contract-1·contract-2 등)이 있으나 서로 다른
> 검토·다른 지적이다 — 혼동 방지를 위해 이 절 안에서는 항상 "2차" 로 지칭한다.

- **regression-1(major, confirmed) — overflow 클리핑 경계 복원**: `<Group>`/에디터 `<Panel>`
  에 프리뷰가 닫혀 있을 때만 `style={{ overflow: 'visible' }}` 를 적용해, 프리뷰 off 경로를
  변경 전 bare `<div>` 와 overflow 의미까지 정확히 동일하게 복원했다(react-resizable-panels
  4.12.2 dist 실물 확인 — `<Group>`·`<Panel>` 모두 하드코딩된 `overflow` 값 뒤에 사용자
  `style` 을 스프레드하므로 오버라이드가 실제로 적용된다. `GroupProps.style` 의 "overflow 는
  오버라이드 불가" 라는 타입 주석은 이 설치 버전의 실제 동작과 다르다). 프리뷰 on 경로는
  건드리지 않았다 — 마크다운 프리뷰 분기는 이번 §7 배치 이전부터 항상 이 동일한
  `<Group>`+`<Panel>` 쌍을 스타일 오버라이드 없이 써 왔으므로, 그쪽은 애초에 §7 배치가 만든
  delta 가 아니었다. 즉 프리뷰 on/off 양쪽 모두 이 수정 이후 각각의 §7 이전 상태와 완전히
  동일하다 — "무조건 visible" 대안(프리뷰 on 에도 overflow 확장)은 채택하지 않았다(§7.1 에
  반영).
- **closure-1=regression-2=design-4=contract-3(2차) — JSDoc 메커니즘 정정 + defaultSize 제거
  채택**: `editorAndPreviewPanels` JSDoc 의 "단일 패널은 나눌 상대가 없어 100%" 서술을 실제
  메커니즘(`We()` 의 no-defaultSize 패널 잔여 배분)으로 정정했다. 검토가 제시한 대안도 함께
  채택 — 에디터 `<Panel>` 의 `defaultSize='50%'` 를 제거했다(§7.1 에 반영, 프리뷰 `<Panel>`
  의 `defaultSize='50%'` 는 유지). `K()` 의 합계-100 재정규화 단계에 더 이상 의존하지 않고,
  레이아웃 확정 전 첫 프레임의 인라인 스타일도 `flexBasis:'50%'` 대신 `flexGrow:1` 이 되어
  전이 상태까지 정확해진다.
- **regression-3(2차) — 프리뷰 재토글 시 분할 비율 복원(수용, 문서화)**: `<Group>` 이 더 이상
  프리뷰 토글로 언마운트되지 않으므로(§7.1 의 핵심 목적) react-resizable-panels 의 그룹
  인스턴스 수명 동안 유지되는 레이아웃 캐시(`mutableState.layouts`, 패널 id 조합별 키)도 함께
  유지된다 — 직전 드래그 비율이 재토글 시 복원된다(이전: `<Group>` 자체가 매번 언마운트되어
  항상 50/50 으로 리셋). 최초 오픈 기본값은 여전히 50/50(캐시 미스)이므로 "50% 기본" 주장은
  유지된다. 매번 50/50 으로 되돌리려면 프리뷰 토글마다 `<Group>` 을 다시 언마운트해야 하는데,
  그러면 §7.1 이 막으려는 `CodeEditor` 재마운트(와 그로 인한 크래시 잔존 경로)가 되살아난다 —
  크래시 봉쇄가 이 UX 보존보다 우선하므로, 리셋 복원은 채택하지 않고 이 동작 변화(개선)를
  그대로 수용해 여기 기록한다.
- **closure-2(2차) — 사슬 고정 테스트 순서·throw 발화 지점 정정**: `code-editor-visibility.
  test.ts` 의 두 번째 사슬 고정 테스트에서 `rerunGitGutterAddActionEffects` 호출을
  `rerunRegistrationEffect` 보다 앞으로 옮겨, 프로덕션의 실제 effect 큐 순서(git-gutter·blame
  의 `addAction` effect 가 `EditorPane` 본문에서 `[tabId, editor]` 등록 effect 보다 먼저
  호출되므로 먼저 큐잉됨)와 맞췄다. JSDoc 의 "editor-area.tsx's action-id effect then calls
  getSupportedActions()" 단정도 두 발화 경로(등록 effect 자신의 동기 `notifyTabListeners` 경유
  vs `focusedFileTabId` 도 함께 바뀐 경우의 EditorArea 자신의 재실행 effect)로 정정했다.
  메커니즘 자체(재충전 없으면 무해·재충전되면 throw)는 그대로 성립한다.
- **design-1=contract-2(2차) — "증명"·"메커니즘 자체를 고정" 서술 하향**: 사슬 고정 두 테스트는
  monaco 를 전혀 로드하지 않고 손으로 쓴 동형 모의만 검증한다 — §7.2 를 "동형 모의에 대한 사슬
  고정(자기충족 한계)" 으로 정정해, monaco 실물 동작이 바뀌어도(예: `addAction` 에 disposed
  가드 추가) 이 테스트는 계속 통과한다는 한계를 §5 의 자기충족 시인과 같은 수위로 명시했다.
- **design-2=contract-1(2차) — 기각 대안 "불충분" 논거 정정**: §2 를 다시 썼다 — 등록 가드는
  실제로 §1·§7.1 의 시체 등록 모두를 막지만(원 "분기 증상만" 서술 폐기), registry 밖 소비자에
  불변식을 강제하지 못한다는 것이 진짜 기각 사유다. 테스트 JSDoc 의 같은 문장도 동일 논리로
  교체했다.
- **design-3(2차) — "타입체크·리뷰로 보증" 정정**: 조정 2줄과 그 import 를 지워도
  `tsc --noEmit`·eslint 가 모두 clean 함을 확인 — §5·§7.2·테스트 JSDoc 세 곳 모두 "리뷰 전담
  (타입체크는 이 배선을 보증하지 않음)" 으로 정정했다. 렌더 하네스 도입 여부는 §6 결정
  패키지에 미등재 상태이며 이번 배치 범위 밖이다.
- **contract-4(2차) — 상충하는 monaco 모형 통일**: `createDisposableFakeEditor` 의
  `getSupportedActions`(dispose 후 즉시 throw 하던 구모형)가 이 테스트에서 실제로 호출된 적이
  없음을 확인하고 제거 — 등록/해제 정체성만 모델링하도록 축소해, "dispose 만으로는 무해"라는
  `createRechargeableFakeEditor` 의 모형과 더 이상 모순되지 않는다.
- **contract-5(2차) — 낡은 라인 참조 → 심볼 서술**: §1·§2·§5 의 editor-pane.tsx 라인 번호
  참조 3곳을 심볼 설명으로 치환(이번 배치의 JSX 변경으로도 다시 낙후하지 않도록).
  `code-editor-visibility.ts` 의 JSDoc 도 문서 경로가 줄바꿈으로 끊기지 않게 정리했다.
- **closure-3=design-5(2차) — 후속 결정 등재 + 불변식 문장 승격**: §6 결정 패키지에 "registry
  등록 소유권 이관"(`onDidDispose` 부기 vs `CodeEditor` 자체 이관 vs 보류) 항목을 3번으로
  추가했다. §7.1 에 "`CodeEditor` 를 감싸는 조건부 렌더를 추가하지 않는다" 를 명시적 불변식으로
  승격했다 — 채택하지 않는 한 이 문장(과 리뷰)이 유일한 안전장치임을 인정한다.
- **contract-6(2차, 기각 유지)**: `docs/PROCESS.md` d-22 갱신은 여전히 이 배치의 지정 범위
  밖이다 — 메인 세션이 완료 보고 시점에 처리한다.

### 7.6 검증 (2차 반영)

- `bun run verify` + `bunx vite build` + Tauri bindings 무변경은 이 배치의 완료 보고에 실측
  기록한다.
