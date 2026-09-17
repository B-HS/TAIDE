import { describe, expect, test } from 'bun:test'
import { HOT_EXIT_FLUSH_SAFETY_MARGIN_MS } from '@shared/constants/mirror'
import { computeFlushBudgetMs, planMirrorFlush } from '@app/providers/hot-exit-flush-provider'

const MAIN_LABEL = 'main'
const AUXILIARY_LABEL = 'editor-1'
const OTHER_AUXILIARY_LABEL = 'editor-2'
const PROJECT_ID = 'project-1'

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

/**
 * d-67 #5·#12 — the flush handshake stopped being app-exit-only. `HotExitFlushRequested` is still
 * broadcast to every window (`Event::emit` reaches them all regardless of the handle it is called
 * through), so each listener has to read the scope: answering for another window's close would let
 * that close proceed while the window that actually has unsaved models is still writing.
 */
describe('planMirrorFlush', () => {
    test('앱 종료(all)는 모든 창이 전부 flush 하고 확인한다 — 종전 동작', () => {
        expect(planMirrorFlush('all', MAIN_LABEL)).toEqual({ target: 'all' })
        expect(planMirrorFlush('all', AUXILIARY_LABEL)).toEqual({ target: 'all' })
    })

    test('창 스코프는 그 라벨의 창만 flush 한다', () => {
        expect(planMirrorFlush({ window: AUXILIARY_LABEL }, AUXILIARY_LABEL)).toEqual({ target: 'all' })
    })

    test('창 스코프를 받은 다른 창은 flush 도 확인도 하지 않는다 — 남의 창 닫기에 대신 답하지 않는다', () => {
        expect(planMirrorFlush({ window: AUXILIARY_LABEL }, MAIN_LABEL)).toEqual({ target: 'ignore' })
        expect(planMirrorFlush({ window: OTHER_AUXILIARY_LABEL }, AUXILIARY_LABEL)).toEqual({ target: 'ignore' })
    })

    test('프로젝트 스코프는 창과 무관하게 그 프로젝트만 flush 한다 — 프로젝트는 창을 가로지른다', () => {
        expect(planMirrorFlush({ project: PROJECT_ID }, MAIN_LABEL)).toEqual({ target: 'project', projectId: PROJECT_ID })
        expect(planMirrorFlush({ project: PROJECT_ID }, AUXILIARY_LABEL)).toEqual({ target: 'project', projectId: PROJECT_ID })
    })

    test('프로젝트 스코프는 그 프로젝트를 안 연 창도 ignore 가 아니다 — 모든 창이 확인해야 닫기가 진행된다', () => {
        expect(planMirrorFlush({ project: PROJECT_ID }, OTHER_AUXILIARY_LABEL).target).not.toBe('ignore')
    })
})
