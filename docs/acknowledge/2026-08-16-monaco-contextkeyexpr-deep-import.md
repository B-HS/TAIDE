# monaco `ContextKeyExpr` 딥 임포트 — 채택 사유 (정적 import)

## 대상 파일

- `src/shared/lib/keymap-when.ts` (`evaluateKeymapWhen`)
- `src/shared/lib/monaco-internal.d.ts` (앰비언트 타입 shim)

## 리포트

Wave H(`docs/acknowledge/2026-08-16-wave-h-keymap-contract.md` §3.2)의 `when` 평가기로 monaco
`ContextKeyExpr.deserialize`/`evaluate` 를 재사용한다. 공개 `monaco.d.ts` 에는 `when` 절 문자열을
`IContextKeyService` 인스턴스 없이 파싱·평가하는 API가 없다 — 딥 임포트가 유일한 경로다.

`command-relay.ts`(`2026-08-14-monaco-command-service-deep-import.md`)의 선례와 같은 매커니즘(패키지
`exports` 의 `"./*": "./esm/vs/*.js"` 와일드카드로 이미 공개 표면인 서브패스, 타입 선언만 미제공)이지만
**동적 import 가 아니라 정적 top-level import** 를 택했다는 점이 다르다.

**대안 기각**

- 동적 import(선례와 동일 패턴): `when` 평가는 keydown 핸들러 안에서 **동기적으로** 완료돼야 한다 —
  `preventDefault`/`stopPropagation` 판단이 그 평가 결과에 달려 있고, 이벤트 핸들러가 반환한 뒤에
  `preventDefault` 를 호출해도 이미 늦다. `await import(...)` 후 이어서 평가하는 구조는 keydown
  핸들러의 동기성 요건과 근본적으로 맞지 않는다.
- when 평가기 자체 구현(미니 파서): 계약 §3.2·§4 에서 "ContextKeyExpr 로드 게이트 실패 시에만" 쓰는
  조건부 폴백으로 명시돼 있다 — 게이트가 통과했으므로(아래 "로드 게이트 결과") 채택하지 않는다.

## 상세

- **로드 게이트 결과**: `bun:test` 환경(DOM 없음)에서 `monaco-editor/platform/contextkey/common/contextkey`
  를 **정적**으로 top-level import 해도 로드/`deserialize`/`evaluate` 모두 정상 동작함을 실측 확인했다
  (`keymap-when.test.ts` 의 "monaco ContextKeyExpr load gate" describe 블록이 회귀 테스트로 고정).
- **정적 import 가 안전한 이유**: 의존 체인(`platform.js`→`nls.js`→`scanner.js`→
  `instantiation.js`) 전체를 확인한 결과, `window`/`document` 참조는 전부 `typeof x !== 'undefined'`
  가드 뒤에 있다. `command-relay.ts` 가 동적 import 를 택한 이유였던 `standaloneServices.js` 체인
  (module-eval 시점에 `window` 를 무가드로 읽는 `base/browser/window.js`)과는 다른 경로라, 그 근거가
  이 모듈에는 적용되지 않는다.
- **의존 경로는 1개, 심볼 2개**로 최소화: `ContextKeyExpr`(값) · `ContextKeyExpression`(타입, deserialize
  반환 형태).
- **캐시**: `deserializeKeymapWhen` 이 문자열별 파싱 결과(유효/무효 모두)를 `Map` 에 캐시한다. 무효
  표현식(`deserialize` 가 `undefined` 반환)은 `evaluateKeymapWhen` 에서 `false` 로 처리 — 깨진 `when`
  이 "항상 매칭"으로 조용히 폴백하지 않고 해당 엔트리가 비활성화된다(계약 §3.2 "무효 표현식은 엔트리
  비활성").
- **monaco 업그레이드 시 확인 절차**: `node_modules/monaco-editor/esm/vs/platform/contextkey/common/`
  하위 `contextkey.js`(`ContextKeyExpr.deserialize`/`evaluate`)·`scanner.js` 경로가 그대로 존재하는지,
  위 의존 체인의 `window`/`document` 가드가 유지되는지 확인한다. 이 파일들이 삭제/개명되거나 가드 없이
  전역을 참조하도록 바뀌면 `keymap-when.test.ts` 의 로드 게이트 테스트가 (import 실패 또는 `bun:test`
  크래시로) 가장 먼저 깨진다 — 그 경우 계약 §3.2·§4 의 폴백(배열 AND/NOT 자체 평가기)으로 전환한다.
