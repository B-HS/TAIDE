import { describe, expect, test } from 'bun:test'
import { openTerminalLink, openViaBrowserWindow } from '@widgets/terminal-pane/terminal-link-opener'

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

describe('openViaBrowserWindow', () => {
    test('네이티브 open 이 창을 반환하면 opener 를 끊고 location.href 로 이동시킨다', () => {
        const fakeWindow = { opener: {}, location: { href: '' } } as unknown as Window
        const result = openViaBrowserWindow('https://example.com', () => fakeWindow)
        expect(result).toBe(fakeWindow)
        expect(fakeWindow.opener).toBeNull()
        expect(fakeWindow.location.href).toBe('https://example.com')
    })

    test('네이티브 open 이 null 을 반환하면(데스크톱 webview 가 팝업을 거부) null 을 반환한다', () => {
        const result = openViaBrowserWindow('https://example.com', () => null)
        expect(result).toBeNull()
    })

    test('opener 재할당이 예외를 던져도 location.href 이동은 계속되고 창을 반환한다', () => {
        const location = { href: '' }
        const fakeWindow = { location } as unknown as Window
        Object.defineProperty(fakeWindow, 'opener', {
            set: () => {
                throw new Error('cannot reassign opener')
            },
        })
        const result = openViaBrowserWindow('https://example.com', () => fakeWindow)
        expect(result).toBe(fakeWindow)
        expect(location.href).toBe('https://example.com')
    })
})
