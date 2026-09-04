import { describe, expect, test } from 'bun:test'
import { probeTerminalClipboardAvailability, resolveTerminalClipboardAvailability } from '@widgets/terminal-pane/terminal-clipboard-availability'

describe('resolveTerminalClipboardAvailability', () => {
    test('두 API 가 모두 있으면 복사·붙여넣기 둘 다 가능하다', () => {
        expect(resolveTerminalClipboardAvailability({ hasWriteText: true, hasReadText: true })).toEqual({ canCopy: true, canPaste: true })
    })

    test('클립보드 API 가 아예 없으면(원격 미러 평문 HTTP) 둘 다 불가다', () => {
        expect(resolveTerminalClipboardAvailability({ hasWriteText: false, hasReadText: false })).toEqual({ canCopy: false, canPaste: false })
    })

    test('readText 만 없으면 붙여넣기만 막힌다', () => {
        expect(resolveTerminalClipboardAvailability({ hasWriteText: true, hasReadText: false })).toEqual({ canCopy: true, canPaste: false })
    })

    test('writeText 만 없으면 복사만 막힌다', () => {
        expect(resolveTerminalClipboardAvailability({ hasWriteText: false, hasReadText: true })).toEqual({ canCopy: false, canPaste: true })
    })
})

/**
 * The DOM harness (`shared/testing/dom-preload.ts`) publishes `navigator.clipboard`, so the probe's
 * "API is there" branch is the default here and the insecure-context branch has to be staged: the
 * getter lives on happy-dom's `Navigator` prototype, so shadowing it with a configurable own
 * property and deleting it afterwards restores the real one without touching any other file.
 */
const withoutClipboardApi = (run: () => void) => {
    Object.defineProperty(navigator, 'clipboard', { value: undefined, configurable: true })
    try {
        run()
    } finally {
        delete (navigator as unknown as { clipboard?: unknown }).clipboard
    }
}

describe('probeTerminalClipboardAvailability', () => {
    test('클립보드 API 가 있는 런타임에서는 실제 navigator 를 읽어 둘 다 가능으로 판정한다', () => {
        expect(probeTerminalClipboardAvailability()).toEqual({ canCopy: true, canPaste: true })
    })

    test('클립보드 API 가 없는 런타임(원격 미러 평문 HTTP)에서는 예외 없이 둘 다 불가로 판정한다', () => {
        withoutClipboardApi(() => {
            expect(probeTerminalClipboardAvailability()).toEqual({ canCopy: false, canPaste: false })
        })
    })
})
