import { describe, expect, test } from 'bun:test'
import {
    OPEN_WITH_OVERRIDE_MAX_ENTRIES,
    getOpenWithOverride,
    pruneOpenWithOverrides,
    setOpenWithOverride,
    subscribeOpenWithOverride,
} from '@entities/editor/open-with-registry'

describe('setOpenWithOverride / getOpenWithOverride', () => {
    test('설정한 값을 그대로 조회할 수 있다', () => {
        setOpenWithOverride('/repo/a.png', 'editor')
        expect(getOpenWithOverride('/repo/a.png')).toBe('editor')
    })

    test('null 로 설정하면 override 가 제거된다', () => {
        setOpenWithOverride('/repo/b.png', 'editor')
        setOpenWithOverride('/repo/b.png', null)
        expect(getOpenWithOverride('/repo/b.png')).toBeNull()
    })

    test('한 번도 설정하지 않은 경로는 null 이다', () => {
        expect(getOpenWithOverride('/repo/never-set.png')).toBeNull()
    })
})

describe('subscribeOpenWithOverride', () => {
    test('override 변경 시 구독자에게 알린다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenWithOverride(() => {
            calls += 1
        })
        setOpenWithOverride('/repo/c.png', 'editor')
        setOpenWithOverride('/repo/c.png', null)
        unsubscribe()
        expect(calls).toBe(2)
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenWithOverride(() => {
            calls += 1
        })
        unsubscribe()
        setOpenWithOverride('/repo/d.png', 'editor')
        expect(calls).toBe(0)
    })
})

describe('OPEN_WITH_OVERRIDE_MAX_ENTRIES / 무한 증가 방지', () => {
    test('상한을 넘기면 가장 오래 전에 쓰인 항목부터 제거된다', () => {
        const paths = Array.from({ length: OPEN_WITH_OVERRIDE_MAX_ENTRIES + 5 }, (_, index) => `/repo/eviction/${index}.png`)
        for (const path of paths) setOpenWithOverride(path, 'editor')

        expect(getOpenWithOverride(paths[0])).toBeNull()
        expect(getOpenWithOverride(paths[4])).toBeNull()
        expect(getOpenWithOverride(paths[paths.length - 1])).toBe('editor')
    })

    test('기존 경로를 다시 쓰면 최근 위치로 옮겨져 상한에 걸려도 살아남는다', () => {
        const paths = Array.from({ length: OPEN_WITH_OVERRIDE_MAX_ENTRIES }, (_, index) => `/repo/refresh/${index}.png`)
        for (const path of paths) setOpenWithOverride(path, 'editor')

        setOpenWithOverride(paths[0], 'editor')
        for (let index = 1; index <= 5; index += 1) setOpenWithOverride(`/repo/refresh/extra-${index}.png`, 'editor')

        expect(getOpenWithOverride(paths[0])).toBe('editor')
        expect(getOpenWithOverride(paths[1])).toBeNull()
    })
})

describe('pruneOpenWithOverrides — 탭/프로젝트 종료 시 정리 훅', () => {
    test('keepPaths 에 없는 경로의 override 는 제거된다', () => {
        setOpenWithOverride('/repo/prune/keep.png', 'editor')
        setOpenWithOverride('/repo/prune/drop.png', 'editor')

        pruneOpenWithOverrides(['/repo/prune/keep.png'])

        expect(getOpenWithOverride('/repo/prune/keep.png')).toBe('editor')
        expect(getOpenWithOverride('/repo/prune/drop.png')).toBeNull()
    })

    test('제거된 항목이 없으면 구독자에게 알리지 않는다', () => {
        pruneOpenWithOverrides([])
        setOpenWithOverride('/repo/prune/untouched.png', 'editor')
        let calls = 0
        const unsubscribe = subscribeOpenWithOverride(() => {
            calls += 1
        })

        pruneOpenWithOverrides(['/repo/prune/untouched.png', '/repo/prune/never-set.png'])

        unsubscribe()
        expect(calls).toBe(0)
        expect(getOpenWithOverride('/repo/prune/untouched.png')).toBe('editor')
    })

    test('제거된 항목이 있으면 구독자에게 알린다', () => {
        setOpenWithOverride('/repo/prune/notify.png', 'editor')
        let calls = 0
        const unsubscribe = subscribeOpenWithOverride(() => {
            calls += 1
        })

        pruneOpenWithOverrides([])

        unsubscribe()
        expect(calls).toBe(1)
        expect(getOpenWithOverride('/repo/prune/notify.png')).toBeNull()
    })
})
