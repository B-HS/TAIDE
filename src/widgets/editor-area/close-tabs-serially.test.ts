import { describe, expect, test } from 'bun:test'
import { IpcError } from '@shared/api/unwrap-result'
import { closeTabsSerially } from '@widgets/editor-area/close-tabs-serially'

/**
 * The regression this helper exists for is a *partial* run: the loops it replaced awaited without a
 * catch, so one rejection ended the walk mid-list and left the remaining tabs open with nothing said.
 */
const notFound = () => new IpcError({ code: 'NotFound', message: 'tab is gone' })
const denied = () => new IpcError({ code: 'Forbidden', message: 'refused' })

const recordingCloser = (failBy: Record<string, () => Error>) => {
    const closed: string[] = []
    const closeTab = async (tabId: string) => {
        const failure = failBy[tabId]
        if (failure) throw failure()
        closed.push(tabId)
    }
    return { closed, closeTab }
}

describe('closeTabsSerially', () => {
    test('순서대로 하나씩 닫고, 문제가 없으면 실패 목록이 비어 있다', async () => {
        const { closed, closeTab } = recordingCloser({})

        expect(await closeTabsSerially(['a', 'b', 'c'], closeTab)).toEqual([])
        expect(closed).toEqual(['a', 'b', 'c'])
    })

    test('이미 사라진 탭(NotFound)은 조용히 건너뛰고 계속 닫는다 — 원하던 결과이기 때문', async () => {
        const { closed, closeTab } = recordingCloser({ b: notFound })

        expect(await closeTabsSerially(['a', 'b', 'c'], closeTab)).toEqual([])
        expect(closed).toEqual(['a', 'c'])
    })

    test('NotFound 가 아닌 실패는 루프를 멈추지 않고 모아 둔다 — 절반만 닫히고 조용한 상태가 사라진다', async () => {
        const { closed, closeTab } = recordingCloser({ b: denied })

        const failures = await closeTabsSerially(['a', 'b', 'c'], closeTab)

        expect(closed).toEqual(['a', 'c'])
        expect(failures).toHaveLength(1)
        expect((failures[0] as IpcError).code).toBe('Forbidden')
    })

    test('빈 목록은 아무 것도 부르지 않는다', async () => {
        const { closed, closeTab } = recordingCloser({})

        expect(await closeTabsSerially([], closeTab)).toEqual([])
        expect(closed).toEqual([])
    })
})
