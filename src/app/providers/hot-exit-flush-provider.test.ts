import { describe, expect, test } from 'bun:test'
import { HOT_EXIT_FLUSH_SAFETY_MARGIN_MS } from '@shared/constants/mirror'
import { computeFlushBudgetMs } from '@app/providers/hot-exit-flush-provider'

describe('computeFlushBudgetMs', () => {
    test('timeoutMs 에서 안전 마진을 뺀 값을 예산으로 반환한다', () => {
        expect(computeFlushBudgetMs(2_500)).toBe(2_500 - HOT_EXIT_FLUSH_SAFETY_MARGIN_MS)
    })

    test('안전 마진이 timeoutMs 보다 크면 0으로 클램프한다(음수 setTimeout 방지)', () => {
        expect(computeFlushBudgetMs(100)).toBe(0)
    })

    test('timeoutMs 가 정확히 안전 마진과 같으면 0을 반환한다', () => {
        expect(computeFlushBudgetMs(HOT_EXIT_FLUSH_SAFETY_MARGIN_MS)).toBe(0)
    })

    test('timeoutMs 가 null 이면(f64 IPC 필드의 바인딩 관례) 예산 없이 즉시 완료 보고로 처리한다', () => {
        expect(computeFlushBudgetMs(null)).toBe(0)
    })
})
