import { describe, expect, spyOn, test } from 'bun:test'
import { toast } from 'sonner'
import { copyTextToClipboard } from '@shared/lib/copy-text-to-clipboard'

const TEXT = '/Users/someone/project/src/main.ts'

const COPY_FAILED_KEY = 'common.copyFailed'

/**
 * Stages a clipboard for one case. The DOM harness (`shared/testing/dom-preload.ts`) publishes a
 * working `navigator.clipboard` whose getter lives on happy-dom's `Navigator` prototype, so both
 * the "refused" and the "absent entirely" runtimes have to be shadowed with a configurable own
 * property and deleted afterwards — the same staging `terminal-clipboard-availability.test.ts` uses,
 * which restores the real one without touching any other file.
 */
const withClipboard = async (value: unknown, run: () => Promise<void>) => {
    Object.defineProperty(navigator, 'clipboard', { value, configurable: true })
    try {
        await run()
    } finally {
        delete (navigator as unknown as { clipboard?: unknown }).clipboard
    }
}

describe('copyTextToClipboard', () => {
    test('클립보드 API 가 아예 없는 런타임(원격 미러 평문 HTTP)에서도 동기 예외 없이 실패를 알리고 false 를 돌려준다', async () => {
        const toastError = spyOn(toast, 'error')

        await withClipboard(undefined, async () => {
            expect(await copyTextToClipboard(TEXT)).toBe(false)
        })

        expect(toastError).toHaveBeenCalledWith(COPY_FAILED_KEY)
    })

    test('writeText 가 거부되면 rejection 을 삼키고 실패를 알린 뒤 false 를 돌려준다', async () => {
        const toastError = spyOn(toast, 'error')

        await withClipboard({ writeText: async () => Promise.reject(new Error('The request is not allowed by the user agent')) }, async () => {
            expect(await copyTextToClipboard(TEXT)).toBe(false)
        })

        expect(toastError).toHaveBeenCalledWith(COPY_FAILED_KEY)
    })

    test('복사에 성공하면 토스트 없이 true 를 돌려주고 받은 문자열을 그대로 쓴다', async () => {
        const toastError = spyOn(toast, 'error')
        const written: string[] = []

        await withClipboard(
            {
                writeText: async (text: string) => {
                    written.push(text)
                },
            },
            async () => {
                expect(await copyTextToClipboard(TEXT)).toBe(true)
            },
        )

        expect(written).toEqual([TEXT])
        expect(toastError).not.toHaveBeenCalled()
    })
})
