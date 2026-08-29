import { describe, expect, test } from 'bun:test'
import { publishFileSaveSettle, subscribeFileSaveSettle } from '@entities/editor/file-save-settle-registry'

describe('file-save-settle-registry (F1 A7·B7 — 경로 단위 저장 정착 브로드캐스트)', () => {
    test('같은 경로를 보는 모든 pane 이 한 번의 저장으로 정착된다 (스플릿 동일 파일)', () => {
        const settled: string[] = []
        const unsubscribeLeft = subscribeFileSaveSettle('/repo/a.ts', (content) => settled.push(`left:${content}`))
        const unsubscribeRight = subscribeFileSaveSettle('/repo/a.ts', (content) => settled.push(`right:${content}`))

        publishFileSaveSettle('/repo/a.ts', 'saved')

        expect(settled).toEqual(['left:saved', 'right:saved'])

        unsubscribeLeft()
        unsubscribeRight()
    })

    test('다른 경로의 구독자는 호출되지 않는다', () => {
        let otherCalls = 0
        const unsubscribe = subscribeFileSaveSettle('/repo/b.ts', () => {
            otherCalls += 1
        })

        publishFileSaveSettle('/repo/a.ts', 'saved')

        expect(otherCalls).toBe(0)
        unsubscribe()
    })

    test('구독 해제된 pane 은 더 이상 정착 통지를 받지 않는다 (언마운트된 탭)', () => {
        let calls = 0
        const unsubscribe = subscribeFileSaveSettle('/repo/c.ts', () => {
            calls += 1
        })
        unsubscribe()

        publishFileSaveSettle('/repo/c.ts', 'saved')

        expect(calls).toBe(0)
    })

    test('한 pane 의 리스너가 throw 해도 나머지 pane 은 정착된다', () => {
        const settled: string[] = []
        const unsubscribeFailing = subscribeFileSaveSettle('/repo/d.ts', () => {
            throw new Error('settle 실패')
        })
        const unsubscribeOk = subscribeFileSaveSettle('/repo/d.ts', (content) => settled.push(content))

        expect(() => publishFileSaveSettle('/repo/d.ts', 'saved')).not.toThrow()
        expect(settled).toEqual(['saved'])

        unsubscribeFailing()
        unsubscribeOk()
    })

    test('통지 도중 스스로 구독 해제해도 남은 리스너 순회가 깨지지 않는다', () => {
        const settled: string[] = []
        const unsubscribeSelf = subscribeFileSaveSettle('/repo/e.ts', () => {
            settled.push('self')
            unsubscribeSelf()
        })
        const unsubscribeOther = subscribeFileSaveSettle('/repo/e.ts', () => settled.push('other'))

        publishFileSaveSettle('/repo/e.ts', 'saved')

        expect(settled).toEqual(['self', 'other'])
        unsubscribeOther()
    })

    test('구독자가 없는 경로에 발행해도 아무 일도 일어나지 않는다 (열린 pane 없는 외부 저장)', () => {
        expect(() => publishFileSaveSettle('/repo/never-open.ts', 'saved')).not.toThrow()
    })
})
