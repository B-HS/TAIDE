import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test'
import type { NotificationDelivery } from '@shared/api/bindings'
import { NOTIFICATION_TEXT_MAX_CODE_POINTS } from '@shared/constants/notification'

/**
 * `notification.ipc.ts` is a Tauri command binding that cannot run under `bun:test`, so it is
 * stubbed before `notify.ts` is pulled in through a *dynamic* `import()`. The stub's behavior is
 * swapped per test through `sendNativeNotificationImpl`; its default rejects, which is what the
 * real binding does with no `window.__TAURI_INTERNALS__` — so a later test file that reaches
 * `notifyNative` through `git.query.ts` (`mock.module` is process-global, last-registration-wins)
 * sees the same swallowed failure it would have seen with the real module.
 *
 * The DOM harness (`shared/testing/dom-preload.ts`) gives this file a `window` with an empty
 * `location.search`, so `getWindowContext()` resolves to the main window; `isRemoteMirrorRuntime()`
 * then reads `getCurrentWindow().label`, which lives in `window.__TAURI_INTERNALS__.metadata` and is
 * the one piece happy-dom cannot supply — the main window's label is installed for this file only,
 * leaving the real gate under test. Its closed branches are covered by
 * `native-notification-gate.test.ts`.
 */
type SendNativeNotificationInput = { category: string; title: string; body: string }

const MAIN_WINDOW_INTERNALS = { metadata: { currentWindow: { label: 'main' } } }

beforeAll(() => {
    window.__TAURI_INTERNALS__ = MAIN_WINDOW_INTERNALS
})

afterAll(() => {
    delete window.__TAURI_INTERNALS__
})

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

const sendNativeNotificationImpl = { current: rejectLikeUnavailableIpc as (input: SendNativeNotificationInput) => Promise<NotificationDelivery> }

mock.module('@entities/notification/notification.ipc', () => ({
    sendNativeNotification: (input: SendNativeNotificationInput) => sendNativeNotificationImpl.current(input),
    openNotificationSystemSettings: rejectLikeUnavailableIpc,
}))

const importNotify = () => import('@entities/notification/notify')

const DELIVERED: NotificationDelivery = { outcome: 'delivered' }
const SUPPRESSED: NotificationDelivery = { outcome: 'suppressed', reason: 'windowFocused' }

const recordingSender = (delivery: NotificationDelivery) => {
    const calls: SendNativeNotificationInput[] = []
    sendNativeNotificationImpl.current = (input) => {
        calls.push(input)
        return Promise.resolve(delivery)
    }
    return calls
}

describe('notifyNative', () => {
    test('게이트 결과를 그대로 돌려주고 카테고리·제목·본문을 IPC 에 전달한다', async () => {
        const { notifyNative } = await importNotify()
        const calls = recordingSender(DELIVERED)

        const delivery = await notifyNative({ category: 'gitRemote', title: 'push ok', body: 'origin/main' })

        expect(delivery).toEqual(DELIVERED)
        expect(calls).toEqual([{ category: 'gitRemote', title: 'push ok', body: 'origin/main' }])
    })

    test('제목·본문을 코드포인트 상한으로 잘라 보낸다', async () => {
        const { notifyNative } = await importNotify()
        const calls = recordingSender(DELIVERED)
        const overlong = '가'.repeat(NOTIFICATION_TEXT_MAX_CODE_POINTS + 50)

        await notifyNative({ category: 'lspInstall', title: overlong, body: overlong })

        expect([...calls[0]!.title]).toHaveLength(NOTIFICATION_TEXT_MAX_CODE_POINTS)
        expect([...calls[0]!.body]).toHaveLength(NOTIFICATION_TEXT_MAX_CODE_POINTS)
        expect(calls[0]!.title.endsWith('…')).toBe(true)
    })

    test('상한 이하의 텍스트는 손대지 않는다', async () => {
        const { notifyNative } = await importNotify()
        const calls = recordingSender(DELIVERED)

        await notifyNative({ category: 'searchReplace', title: '치환 완료', body: '3건' })

        expect(calls[0]).toEqual({ category: 'searchReplace', title: '치환 완료', body: '3건' })
    })

    test('전달됐을 때만 첫 전달 안내 구독자에게 알린다', async () => {
        const { notifyNative, subscribeNativeNotificationDelivered } = await importNotify()
        let announced = 0
        const unsubscribe = subscribeNativeNotificationDelivered(() => {
            announced += 1
        })

        recordingSender(SUPPRESSED)
        await notifyNative({ category: 'agentCompleted', title: 't', body: 'b' })
        expect(announced).toBe(0)

        recordingSender(DELIVERED)
        await notifyNative({ category: 'agentCompleted', title: 't', body: 'b' })
        expect(announced).toBe(1)

        unsubscribe()
    })

    test('IPC 가 실패하면 예외 대신 null 을 돌려주고 안내도 하지 않는다', async () => {
        const { notifyNative, subscribeNativeNotificationDelivered } = await importNotify()
        let announced = 0
        const unsubscribe = subscribeNativeNotificationDelivered(() => {
            announced += 1
        })
        sendNativeNotificationImpl.current = rejectLikeUnavailableIpc

        await expect(notifyNative({ category: 'error', title: 't', body: 'b' })).resolves.toBeNull()
        expect(announced).toBe(0)

        unsubscribe()
    })
})
