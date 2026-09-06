import { describe, expect, test } from 'bun:test'
import { consumeReplayBudget } from '@widgets/terminal-pane/terminal-replay-budget'

describe('consumeReplayBudget', () => {
    test('예산이 0 이면 청크 전체가 flow control 집계 대상이다', () => {
        expect(consumeReplayBudget(0, 1024)).toEqual({ countedBytes: 1024, remainingBudget: 0 })
    })

    test('예산이 청크보다 크면 아무것도 집계하지 않고 예산만 줄인다', () => {
        expect(consumeReplayBudget(4096, 1024)).toEqual({ countedBytes: 0, remainingBudget: 3072 })
    })

    test('예산을 정확히 소진하는 청크는 집계 0 · 잔여 0 이다', () => {
        expect(consumeReplayBudget(1024, 1024)).toEqual({ countedBytes: 0, remainingBudget: 0 })
    })

    test('예산 경계를 넘는 청크는 넘친 만큼만 집계한다', () => {
        expect(consumeReplayBudget(100, 250)).toEqual({ countedBytes: 150, remainingBudget: 0 })
    })

    test('0 바이트 청크는 예산을 소모하지 않는다', () => {
        expect(consumeReplayBudget(4096, 0)).toEqual({ countedBytes: 0, remainingBudget: 4096 })
    })

    test('음수 예산은 0 으로 취급해 청크 전체를 집계한다', () => {
        expect(consumeReplayBudget(-1, 512)).toEqual({ countedBytes: 512, remainingBudget: 0 })
    })

    test('연속 청크에 이어서 적용하면 리플레이 총량만큼만 제외된다', () => {
        const first = consumeReplayBudget(3000, 2048)
        const second = consumeReplayBudget(first.remainingBudget, 2048)
        expect(first.countedBytes).toBe(0)
        expect(second).toEqual({ countedBytes: 1096, remainingBudget: 0 })
    })
})
