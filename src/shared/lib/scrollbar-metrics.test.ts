import { describe, expect, test } from 'bun:test'
import {
    computeScrollOffsetForThumbDelta,
    computeScrollOffsetForTrackClick,
    computeScrollPercent,
    computeScrollbarThumbMetrics,
} from '@shared/lib/scrollbar-metrics'

describe('computeScrollbarThumbMetrics', () => {
    test('콘텐츠가 뷰포트보다 작으면 스크롤 불가로 판정한다', () => {
        const metrics = computeScrollbarThumbMetrics({ scrollOffset: 0, scrollSize: 100, clientSize: 200, trackSize: 200 })
        expect(metrics.scrollable).toBe(false)
    })

    test('트랙 크기가 0이면 스크롤 불가로 판정한다', () => {
        const metrics = computeScrollbarThumbMetrics({ scrollOffset: 0, scrollSize: 1000, clientSize: 200, trackSize: 0 })
        expect(metrics.scrollable).toBe(false)
    })

    test('thumb 크기는 뷰포트/콘텐츠 비율에 트랙 길이를 곱한 값이다', () => {
        const metrics = computeScrollbarThumbMetrics({ scrollOffset: 0, scrollSize: 1000, clientSize: 500, trackSize: 200 })
        expect(metrics.scrollable).toBe(true)
        expect(metrics.thumbSize).toBe(100)
    })

    test('비율이 작아도 thumb 최소 크기 미만으로 줄어들지 않는다', () => {
        const metrics = computeScrollbarThumbMetrics({ scrollOffset: 0, scrollSize: 100_000, clientSize: 200, trackSize: 200 })
        expect(metrics.thumbSize).toBe(24)
    })

    test('스크롤이 끝에 도달하면 thumb 도 트랙 끝에 도달한다', () => {
        const metrics = computeScrollbarThumbMetrics({ scrollOffset: 800, scrollSize: 1000, clientSize: 500, trackSize: 200 })
        expect(metrics.thumbOffset).toBe(100)
    })

    test('스크롤이 중간이면 thumb 위치도 비례한다', () => {
        const metrics = computeScrollbarThumbMetrics({ scrollOffset: 250, scrollSize: 1000, clientSize: 500, trackSize: 200 })
        expect(metrics.thumbOffset).toBe(50)
    })
})

describe('computeScrollOffsetForThumbDelta', () => {
    test('드래그 델타에 비례해 스크롤 오프셋이 이동한다', () => {
        const next = computeScrollOffsetForThumbDelta({
            dragStartScrollOffset: 0,
            deltaPx: 50,
            scrollSize: 1000,
            clientSize: 500,
            trackSize: 200,
            thumbSize: 100,
        })
        expect(next).toBe(250)
    })

    test('스크롤 오프셋은 0 밑으로 내려가지 않는다', () => {
        const next = computeScrollOffsetForThumbDelta({
            dragStartScrollOffset: 0,
            deltaPx: -50,
            scrollSize: 1000,
            clientSize: 500,
            trackSize: 200,
            thumbSize: 100,
        })
        expect(next).toBe(0)
    })

    test('스크롤 오프셋은 최대값을 넘지 않는다', () => {
        const next = computeScrollOffsetForThumbDelta({
            dragStartScrollOffset: 400,
            deltaPx: 1000,
            scrollSize: 1000,
            clientSize: 500,
            trackSize: 200,
            thumbSize: 100,
        })
        expect(next).toBe(500)
    })

    test('트랙에 thumb 이동 여지가 없으면 시작값을 그대로 반환한다', () => {
        const next = computeScrollOffsetForThumbDelta({
            dragStartScrollOffset: 10,
            deltaPx: 50,
            scrollSize: 1000,
            clientSize: 500,
            trackSize: 100,
            thumbSize: 100,
        })
        expect(next).toBe(10)
    })
})

describe('computeScrollPercent', () => {
    test('스크롤 여지가 없으면 0을 반환한다', () => {
        expect(computeScrollPercent({ scrollOffset: 0, scrollSize: 200, clientSize: 200 })).toBe(0)
    })

    test('맨 위에서는 0을 반환한다', () => {
        expect(computeScrollPercent({ scrollOffset: 0, scrollSize: 1000, clientSize: 500 })).toBe(0)
    })

    test('맨 아래에서는 100을 반환한다', () => {
        expect(computeScrollPercent({ scrollOffset: 500, scrollSize: 1000, clientSize: 500 })).toBe(100)
    })

    test('중간 지점은 반올림된 백분율을 반환한다', () => {
        expect(computeScrollPercent({ scrollOffset: 250, scrollSize: 1000, clientSize: 500 })).toBe(50)
    })
})

describe('computeScrollOffsetForTrackClick', () => {
    test('클릭 지점이 thumb 중앙이 되도록 스크롤 오프셋을 계산한다', () => {
        const next = computeScrollOffsetForTrackClick({ clickOffset: 100, thumbSize: 100, trackSize: 200, scrollSize: 1000, clientSize: 500 })
        expect(next).toBe(250)
    })

    test('트랙 시작 부근을 클릭하면 오프셋은 0으로 클램프된다', () => {
        const next = computeScrollOffsetForTrackClick({ clickOffset: 0, thumbSize: 100, trackSize: 200, scrollSize: 1000, clientSize: 500 })
        expect(next).toBe(0)
    })

    test('트랙 끝 부근을 클릭하면 오프셋은 최대값으로 클램프된다', () => {
        const next = computeScrollOffsetForTrackClick({ clickOffset: 200, thumbSize: 100, trackSize: 200, scrollSize: 1000, clientSize: 500 })
        expect(next).toBe(500)
    })
})
