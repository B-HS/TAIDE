import { describe, expect, test } from 'bun:test'
import { openTerminalLink } from '@widgets/terminal-pane/terminal-link-opener'

describe('openTerminalLink', () => {
    test('window.open 이 창을 반환하면 IPC 폴백을 호출하지 않는다', async () => {
        let fallbackCalled = false
        await openTerminalLink('https://example.com', {
            windowOpen: () => ({}) as Window,
            openExternalUrl: async () => {
                fallbackCalled = true
            },
        })
        expect(fallbackCalled).toBe(false)
    })

    test('window.open 이 null 을 반환하면 같은 uri 로 IPC 폴백을 호출한다', async () => {
        const fallback: { uri: string | null } = { uri: null }
        await openTerminalLink('https://example.com', {
            windowOpen: () => null,
            openExternalUrl: async (uri) => {
                fallback.uri = uri
            },
        })
        expect(fallback.uri).toBe('https://example.com')
    })

    test('IPC 폴백이 실패하면 에러가 전파된다', async () => {
        const rejection = new Error('open failed')
        await expect(
            openTerminalLink('https://example.com', {
                windowOpen: () => null,
                openExternalUrl: async () => {
                    throw rejection
                },
            }),
        ).rejects.toThrow(rejection)
    })
})
