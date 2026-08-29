import { describe, expect, test } from 'bun:test'
import { IME_COMPOSITION_KEY_CODE, isImeCompositionKeydown } from '@shared/lib/ime-composition'

/**
 * Shapes taken from the measured event table in `docs/bug/2026-08-06-wkwebview-ime-composition.md`:
 * Safari reports both `isComposing: true` and `keyCode: 229`, while the app's WKWebView reports
 * `keyCode: 229` with `isComposing` stuck at `false` because no composition event ever fires.
 */
const safariComposingKeydown = { key: 'Process', isComposing: true, keyCode: IME_COMPOSITION_KEY_CODE }
const wkWebViewComposingKeydown = { key: 'Process', isComposing: false, keyCode: IME_COMPOSITION_KEY_CODE }
const plainKeydown = { key: 's', isComposing: false, keyCode: 83 }
const plainEnterKeydown = { key: 'Enter', isComposing: false, keyCode: 13 }

describe('isImeCompositionKeydown', () => {
    test('WKWebView 조합 keydown 은 isComposing 이 false 여도 keyCode 229 로 조합으로 판정한다', () => {
        expect(isImeCompositionKeydown(wkWebViewComposingKeydown)).toBe(true)
    })

    test('조합 이벤트가 정상 발생하는 환경(isComposing true)도 그대로 조합으로 판정한다', () => {
        expect(isImeCompositionKeydown(safariComposingKeydown)).toBe(true)
    })

    test('keyCode 를 보고하지 않아도 isComposing 만으로 조합으로 판정한다', () => {
        expect(isImeCompositionKeydown({ isComposing: true })).toBe(true)
    })

    test('React 합성 이벤트는 nativeEvent.isComposing 으로도 조합으로 판정한다', () => {
        expect(isImeCompositionKeydown({ keyCode: 13, nativeEvent: { isComposing: true } })).toBe(true)
    })

    test('조합이 아닌 일반 키 입력은 false 다', () => {
        expect(isImeCompositionKeydown(plainKeydown)).toBe(false)
    })

    test('조합 확정 뒤의 맨 Enter(keyCode 13)는 false 다 — 제출이 막히면 안 된다', () => {
        expect(isImeCompositionKeydown(plainEnterKeydown)).toBe(false)
    })

    test('아무 정보도 없는 이벤트는 조합으로 보지 않는다', () => {
        expect(isImeCompositionKeydown({})).toBe(false)
    })
})
