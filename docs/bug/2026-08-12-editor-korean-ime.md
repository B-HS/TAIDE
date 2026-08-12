# 에디터(monaco) 한글 IME 버그 2건 — 상하 떨림(수정)·자모 분리(환경층)

> QA6 실기 중 사용자 보고(2026-08-12). 발생 표면 monaco 에디터 한정(터미널·일반 input 정상 —
> 터미널은 `docs/bug/2026-08-06-wkwebview-ime-composition.md` 의 자체 어댑터가 이미 흡수).
> 디버그 fable+high(wf_830fe067-9a9) + monaco 상류 조사 opus+medium(wf_bb2f70d7-8fe),
> 핵심 인용은 메인이 monaco 0.56.0 dist 실물로 재확인.

## 버그 A — 한글 타이핑 중 텍스트 상하 떨림 (수정 완료, 커밋 `53e8969`)

- **증상**: 한글 조합 중 글자가 위아래로 흔들림(높이가 변하는 것처럼 보임). 영문 무증상.
- **원인**: monaco 는 음절 조합마다 라인 위에 visible textarea 오버레이를 토글한다
  (`textAreaEditContext.js` compositionstart→`_render`). `MONOSPACE_FONT_FALLBACK_STACK`
  (`src/shared/lib/font-stack.ts`)에 한글 글리프 폰트가 없어 한글이 UA 시스템 폴백으로 렌더되고,
  오버레이(form control)와 뷰 라인(span) 간 폴백 해석·메트릭이 비결정적으로 어긋났다.
  라인 높이 자체는 monaco 가 상수(fontSize×1.5)로 고정하므로 실제 높이 변동은 아니다.
  동일 증상의 상류 이슈: microsoft/monaco-editor#4796 (Safari 중국어, open·미수정).
- **수정**: 폴백 스택에 `"Apple SD Gothic Neo"` 를 generic `monospace` 앞에 추가 — 오버레이·뷰
  라인 양쪽에서 한글 폰트가 결정적으로 동일 매칭. 라틴 선두 폰트 불변(레이아웃·터미널 셀 무영향).
- **실기 확인 절차**: 한글 연속 타이핑(기본 스택/커스텀 폰트 각각), 터미널 한글 회귀 확인.

## 버그 B — 영→한 전환 직후 첫 조합 자모 분리 (앱 수정 없음 — 환경층)

- **증상**: 영→한 전환 직후 타이핑하면 자모가 분리 표시, 다음 글자 입력 시 해소.
- **판정**: 앱 코드 무혐의.
  - auto-tab: monaco 가 조합 중 ghost text 억제(`inlineCompletionsModel` inComposition),
    프로바이더는 모델·DOM 편집 0건.
  - 전역 단축키: 전 항목 modifier 필수 — 무보정 타이핑 매칭 불가, 미매칭 시 즉시 return.
  - 조합 중 모델 쓰기: 유일한 setValue 는 dirty 가드로 타이핑 중 실행 불가.
  - monaco 는 조합 중 textarea 쓰기를 전면 차단 — 외부 렌더가 조합을 못 끊는다.
- **원인 계층**: WKWebView↔monaco 숨김 textarea. `compositionupdate.data` 자체가 분리 자모로
  도착하며(monaco 는 매 update 마다 직전 조합 전체를 치환 — "다음 글자에 해소"와 정합),
  monaco 는 유휴 시 textarea 에 스크린리더 콘텐츠를 시드하는 유일한 표면이 에디터다.
  WebKit 은 EditContext 미구현(#269922 NEW)이라 항상 레거시 textarea 경로.
  관련 상류: WebKit #274700 "Korean Composition Events Not Triggered"(NEW),
  xterm.js PR #5704(WKWebView 한글 IME insertReplacementText, open).
- **상류 조사 결론(2026-08-12)**: monaco 0.56.0 이 최신 — 상위 버전 부재, vscode main 에도
  0.56 컷 이후 한글/조합 수정 0건. **업그레이드로 해결 불가** → backlog(릴리스 추적 트리거 +
  상류 이슈 제출 후보). 상세 출처는 조사 보고 §7(monaco releases·npm registry·WebKit 트래커).
- **미검증 실험 옵션(비-hack, 공개 API)**: TAIDE 는 `accessibilitySupport` 미지정 →
  기본 Unknown(0) → `SimplePagedScreenReaderStrategy`(페이지 단위 시드). `'off'` 지정 시
  시드가 "커서 앞 단어"로 축소된다(Disabled 분기, `textAreaEditContext.js:146-158`).
  시드가 실제 트리거라면 증상이 완화될 수 있으나 **실기 검증 전 가설** — 채택은 사용자 결정.
  부작용: 스크린리더 사용 시 접근성 저하 가능(에디터 옵션 1개 변경).

## 대상 파일

- `src/shared/lib/font-stack.ts` (+`font-stack.test.ts`) — 버그 A 수정
- `src/features/editor/code-editor.tsx` — 버그 B 실험 옵션 후보 지점(미적용)
