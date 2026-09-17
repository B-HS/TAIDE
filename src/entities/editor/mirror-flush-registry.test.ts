import { describe, expect, test } from 'bun:test'
import {
    flushAllMirrors,
    flushProjectMirrors,
    registerMirrorFlush,
    registerViewStateFlush,
    unregisterMirrorFlush,
    unregisterViewStateFlush,
} from '@entities/editor/mirror-flush-registry'

const PROJECT_ID = 'project-1'
const OTHER_PROJECT_ID = 'project-2'

describe('flushAllMirrors', () => {
    test('등록된 모든 탭의 flush 콜백을 호출한다', async () => {
        const calledTabIds: string[] = []
        registerMirrorFlush('tab-a', PROJECT_ID, () => {
            calledTabIds.push('tab-a')
        })
        registerMirrorFlush('tab-b', PROJECT_ID, () => {
            calledTabIds.push('tab-b')
        })

        await flushAllMirrors()

        expect(calledTabIds.sort()).toEqual(['tab-a', 'tab-b'])

        unregisterMirrorFlush('tab-a')
        unregisterMirrorFlush('tab-b')
    })

    test('등록 해제된 탭은 더 이상 호출되지 않는다', async () => {
        let calls = 0
        registerMirrorFlush('tab-c', PROJECT_ID, () => {
            calls += 1
        })
        unregisterMirrorFlush('tab-c')

        await flushAllMirrors()

        expect(calls).toBe(0)
    })

    test('한 flush 가 실패해도 나머지 flush 는 모두 실행된다', async () => {
        let succeededCalls = 0
        registerMirrorFlush('tab-failing', PROJECT_ID, () => {
            throw new Error('flush 실패')
        })
        registerMirrorFlush('tab-ok', PROJECT_ID, () => {
            succeededCalls += 1
        })

        await expect(flushAllMirrors()).resolves.toBeUndefined()
        expect(succeededCalls).toBe(1)

        unregisterMirrorFlush('tab-failing')
        unregisterMirrorFlush('tab-ok')
    })

    test('비동기 flush 콜백의 완료를 기다린다', async () => {
        let resolved = false
        registerMirrorFlush('tab-async', PROJECT_ID, async () => {
            await new Promise((resolve) => setTimeout(resolve, 0))
            resolved = true
        })

        await flushAllMirrors()

        expect(resolved).toBe(true)
        unregisterMirrorFlush('tab-async')
    })

    test('같은 tabId 라도 mirror flush 와 viewState flush 는 서로 다른 슬롯이라 덮어쓰지 않는다 (X1#4 회귀)', async () => {
        const calledKinds: string[] = []
        registerMirrorFlush('tab-shared', PROJECT_ID, () => {
            calledKinds.push('mirror')
        })
        registerViewStateFlush('tab-shared', PROJECT_ID, () => {
            calledKinds.push('viewState')
        })

        await flushAllMirrors()

        expect(calledKinds.sort()).toEqual(['mirror', 'viewState'])

        unregisterMirrorFlush('tab-shared')
        unregisterViewStateFlush('tab-shared')
    })

    test('viewState flush 는 등록 해제 후 더 이상 호출되지 않는다', async () => {
        let calls = 0
        registerViewStateFlush('tab-vs', PROJECT_ID, () => {
            calls += 1
        })
        unregisterViewStateFlush('tab-vs')

        await flushAllMirrors()

        expect(calls).toBe(0)
    })
})

/**
 * d-67 #12 — closing a project asks every window to flush that project's models before its
 * `file_mirror_dirty` writes stop being accepted. A window rendering two projects must write only the
 * one that is disappearing: the other project's panes are still live, and their writes would be spent
 * inside a timeout budget nothing is waiting on.
 */
describe('flushProjectMirrors', () => {
    test('해당 프로젝트의 flush 만 호출하고 다른 프로젝트는 건드리지 않는다', async () => {
        const calledTabIds: string[] = []
        registerMirrorFlush('tab-closing', PROJECT_ID, () => {
            calledTabIds.push('tab-closing')
        })
        registerMirrorFlush('tab-staying', OTHER_PROJECT_ID, () => {
            calledTabIds.push('tab-staying')
        })

        await flushProjectMirrors(PROJECT_ID)

        expect(calledTabIds).toEqual(['tab-closing'])

        unregisterMirrorFlush('tab-closing')
        unregisterMirrorFlush('tab-staying')
    })

    test('viewState flush 도 같은 프로젝트 키로 걸러진다', async () => {
        const calledKinds: string[] = []
        registerViewStateFlush('tab-vs-closing', PROJECT_ID, () => {
            calledKinds.push('closing')
        })
        registerViewStateFlush('tab-vs-staying', OTHER_PROJECT_ID, () => {
            calledKinds.push('staying')
        })

        await flushProjectMirrors(PROJECT_ID)

        expect(calledKinds).toEqual(['closing'])

        unregisterViewStateFlush('tab-vs-closing')
        unregisterViewStateFlush('tab-vs-staying')
    })

    test('그 프로젝트의 flush 가 하나도 없으면 아무 것도 하지 않고 정상 종료한다 — 다른 프로젝트만 연 창도 확인은 보내야 한다', async () => {
        let calls = 0
        registerMirrorFlush('tab-other-only', OTHER_PROJECT_ID, () => {
            calls += 1
        })

        await expect(flushProjectMirrors(PROJECT_ID)).resolves.toBeUndefined()
        expect(calls).toBe(0)

        unregisterMirrorFlush('tab-other-only')
    })

    test('한 flush 가 실패해도 같은 프로젝트의 나머지는 실행된다', async () => {
        let succeededCalls = 0
        registerMirrorFlush('tab-project-failing', PROJECT_ID, () => {
            throw new Error('flush 실패')
        })
        registerMirrorFlush('tab-project-ok', PROJECT_ID, () => {
            succeededCalls += 1
        })

        await expect(flushProjectMirrors(PROJECT_ID)).resolves.toBeUndefined()
        expect(succeededCalls).toBe(1)

        unregisterMirrorFlush('tab-project-failing')
        unregisterMirrorFlush('tab-project-ok')
    })
})
