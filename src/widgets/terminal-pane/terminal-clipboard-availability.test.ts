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

describe('probeTerminalClipboardAvailability', () => {
    test('클립보드 API 가 없는 런타임(bun:test)에서는 예외 없이 둘 다 불가로 판정한다', () => {
        expect(probeTerminalClipboardAvailability()).toEqual({ canCopy: false, canPaste: false })
    })
})
