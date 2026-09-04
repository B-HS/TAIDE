import { describe, expect, test } from 'bun:test'
import { createExternalUrlOpener, openViaBrowserWindow } from '@shared/lib/external-url-opener'

const TEST_URL = 'https://example.com'

describe('createExternalUrlOpener', () => {
    test('데스크톱에서는 셸(IPC)로만 열고 window.open 을 절대 호출하지 않는다', async () => {
        const calls = { shell: [] as string[], browser: 0 }
        const open = createExternalUrlOpener({
            isRemoteMirror: () => false,
            openViaShell: async (uri) => {
                calls.shell.push(uri)
            },
            openViaBrowser: () => {
                calls.browser += 1
                return {} as Window
            },
        })

        await open(TEST_URL)

        expect(calls.shell).toEqual([TEST_URL])
        expect(calls.browser).toBe(0)
    })

    test('데스크톱에서 셸 열기가 실패하면 브라우저로 폴백하지 않고 에러를 전파한다', async () => {
        const rejection = new Error('open failed')
        let browserCalled = false
        const open = createExternalUrlOpener({
            isRemoteMirror: () => false,
            openViaShell: async () => {
                throw rejection
            },
            openViaBrowser: () => {
                browserCalled = true
                return {} as Window
            },
        })

        await expect(open(TEST_URL)).rejects.toThrow(rejection)
        expect(browserCalled).toBe(false)
    })

    test('원격 미러에서는 브라우저 탭으로만 열고 셸(IPC)을 호출하지 않는다', async () => {
        const calls = { shell: 0, browser: [] as string[] }
        const open = createExternalUrlOpener({
            isRemoteMirror: () => true,
            openViaShell: async () => {
                calls.shell += 1
            },
            openViaBrowser: (uri) => {
                calls.browser.push(uri)
                return {} as Window
            },
        })

        await open(TEST_URL)

        expect(calls.browser).toEqual([TEST_URL])
        expect(calls.shell).toBe(0)
    })

    test('원격 미러에서 브라우저가 창을 거부하면 셸로 폴백하지 않고 에러를 던진다', async () => {
        let shellCalled = false
        const open = createExternalUrlOpener({
            isRemoteMirror: () => true,
            openViaShell: async () => {
                shellCalled = true
            },
            openViaBrowser: () => null,
        })

        await expect(open(TEST_URL)).rejects.toThrow(TEST_URL)
        expect(shellCalled).toBe(false)
    })

    test('런타임 판정은 호출마다 다시 평가된다 (같은 opener 가 데스크톱·미러 경로를 번갈아 탄다)', async () => {
        const calls = { shell: [] as string[], browser: [] as string[] }
        let isRemoteMirror = false
        const open = createExternalUrlOpener({
            isRemoteMirror: () => isRemoteMirror,
            openViaShell: async (uri) => {
                calls.shell.push(uri)
            },
            openViaBrowser: (uri) => {
                calls.browser.push(uri)
                return {} as Window
            },
        })

        await open(TEST_URL)
        isRemoteMirror = true
        await open(TEST_URL)

        expect(calls.shell).toEqual([TEST_URL])
        expect(calls.browser).toEqual([TEST_URL])
    })
})

describe('openViaBrowserWindow', () => {
    test('네이티브 open 이 창을 반환하면 opener 를 끊고 location.href 로 이동시킨다', () => {
        const fakeWindow = { opener: {}, location: { href: '' } } as unknown as Window
        const result = openViaBrowserWindow(TEST_URL, () => fakeWindow)
        expect(result).toBe(fakeWindow)
        expect(fakeWindow.opener).toBeNull()
        expect(fakeWindow.location.href).toBe(TEST_URL)
    })

    test('네이티브 open 이 null 을 반환하면(브라우저가 팝업을 거부) null 을 반환한다', () => {
        const result = openViaBrowserWindow(TEST_URL, () => null)
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
        const result = openViaBrowserWindow(TEST_URL, () => fakeWindow)
        expect(result).toBe(fakeWindow)
        expect(location.href).toBe(TEST_URL)
    })
})
